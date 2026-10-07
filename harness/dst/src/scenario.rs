//! One simulation run: topology, workload, network faults and the final oracle check.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use demo_app_core::platform::{Clock as _, SeededRng, TokioClock, buggify};
use demo_app_core::types::Timestamp;
use demo_app_server::{AppState, router};
use hyper::StatusCode;
use serde_json::json;

use crate::client::{Reply, send};
use crate::listener::TurmoilListener;
use crate::model::Model;

/// Workload shape for one run.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Client hosts driving load.
    pub clients: u32,
    /// Operations per client.
    pub ops_per_client: u32,
    /// Percent of enabled `buggify!` sites that fire per call.
    pub buggify_percent: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            clients: 3,
            ops_per_client: 30,
            buggify_percent: 20,
        }
    }
}

/// A run that broke an invariant, with the captured JSON log.
#[derive(Debug)]
pub struct Failure {
    /// What went wrong.
    pub reason: String,
    /// JSON log lines of the run.
    pub log: String,
}

type Shared = Arc<Mutex<Model>>;

fn with_model<T>(model: &Shared, op: impl FnOnce(&mut Model) -> T) -> T {
    op(&mut model.lock().unwrap_or_else(PoisonError::into_inner))
}

/// Runs one seed. Returns the run's JSON log on success.
pub fn run(seed: u64, config: &Config) -> Result<String, Failure> {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = Arc::clone(&buffer);
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_writer(move || LogWriter(Arc::clone(&writer)))
        .with_max_level(tracing::Level::INFO)
        .finish();
    let result = tracing::subscriber::with_default(subscriber, || {
        let _root = tracing::info_span!("dst_run", dst_seed = seed).entered();
        buggify::enable(seed, config.buggify_percent);
        let outcome = simulate(seed, config);
        buggify::disable();
        outcome
    });
    let log = String::from_utf8_lossy(&buffer.lock().unwrap_or_else(PoisonError::into_inner))
        .into_owned();
    result
        .map(|()| log.clone())
        .map_err(|reason| Failure { reason, log })
}

fn simulate(seed: u64, config: &Config) -> Result<(), String> {
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .simulation_duration(Duration::from_secs(3600))
        .min_message_latency(Duration::from_millis(1))
        .max_message_latency(Duration::from_millis(30))
        .enable_random_order()
        .build();
    sim.host("server", move || async move {
        let clock = Arc::new(TokioClock::new(Timestamp::from_millis(1_700_000_000_000)));
        let state = AppState::new(
            clock,
            Arc::new(SeededRng::new(seed)),
            Duration::from_secs(5),
        );
        state.mark_ready();
        let listener = TurmoilListener(turmoil::net::TcpListener::bind(("0.0.0.0", 8080)).await?);
        axum::serve(listener, router(state)).await?;
        Ok(())
    });
    let model: Shared = Arc::default();
    let done = Arc::new(Mutex::new(0_u32));
    for index in 0..config.clients {
        let (model, done) = (Arc::clone(&model), Arc::clone(&done));
        let mut rng = fastrand::Rng::with_seed(seed ^ u64::from(index).wrapping_mul(0x9e37_79b9));
        let ops = config.ops_per_client;
        sim.client(format!("client{index}"), async move {
            let outcome = workload(&format!("c{index}"), &mut rng, ops, &model).await;
            *done.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            outcome.map_err(Into::into)
        });
    }
    let (verify_model, verify_done, clients) =
        (Arc::clone(&model), Arc::clone(&done), config.clients);
    sim.client("verifier", async move {
        verify(&verify_model, &verify_done, clients)
            .await
            .map_err(Into::into)
    });
    drive(&mut sim, seed, &done, config.clients).map_err(|e| e.to_string())
}

/// Steps the simulation, partitioning and holding links at random until the workload is done,
/// then heals the network so the verifier sees a quiet system.
fn drive(
    sim: &mut turmoil::Sim<'_>,
    seed: u64,
    done: &Arc<Mutex<u32>>,
    clients: u32,
) -> turmoil::Result {
    let mut faults = fastrand::Rng::with_seed(seed.rotate_left(17));
    let mut healed = false;
    while !sim.step()? {
        let finished = *done.lock().unwrap_or_else(PoisonError::into_inner) >= clients;
        if finished && !healed {
            for index in 0..clients {
                sim.repair("server", format!("client{index}"));
                sim.release("server", format!("client{index}"));
            }
            healed = true;
        }
        if !finished && faults.u16(0..300) == 0 {
            let link = format!("client{}", faults.u32(0..clients));
            match faults.u8(0..4) {
                0 => sim.partition("server", link),
                1 => sim.repair("server", link),
                2 => sim.hold("server", link),
                _ => sim.release("server", link),
            }
        }
    }
    Ok(())
}

async fn workload(
    name: &str,
    rng: &mut fastrand::Rng,
    ops: u32,
    model: &Shared,
) -> Result<(), String> {
    for n in 0..ops {
        // Each client reads and deletes only its own items, so concurrent clients never race on
        // one id and every answer has a single correct value.
        let prefix = format!("{name}-");
        let known: Vec<String> = with_model(model, |m| {
            m.live
                .iter()
                .filter(|(_, owned)| owned.starts_with(&prefix))
                .map(|(id, _)| id.clone())
                .collect()
        });
        let target = (!known.is_empty()).then(|| known[rng.usize(0..known.len())].clone());
        match (rng.u8(0..100), target) {
            (0..60, _) | (_, None) => create(&format!("{name}-{n}"), model).await?,
            (60..75, Some(id)) => get(&id, model).await?,
            (75..90, Some(id)) => delete(&id, model).await?,
            (_, Some(_)) => drop(send("server", "GET", "/items?limit=100", None).await),
        }
    }
    Ok(())
}

async fn create(name: &str, model: &Shared) -> Result<(), String> {
    for _attempt in 0..3 {
        match send("server", "POST", "/items", Some(json!({ "name": name }))).await {
            Reply::Answer(StatusCode::CREATED, item) => {
                let id = item["id"]
                    .as_str()
                    .ok_or("create returned no id")?
                    .to_owned();
                with_model(model, |m| m.live.insert(id, name.to_owned()));
                return Ok(());
            }
            Reply::Answer(StatusCode::SERVICE_UNAVAILABLE, _) => {}
            Reply::Answer(status, body) => {
                return Err(format!("create {name}: unexpected {status} {body}"));
            }
            Reply::Unknown => {
                with_model(model, |m| m.maybe_created.insert(name.to_owned()));
                return Ok(());
            }
        }
    }
    Ok(())
}

async fn get(id: &str, model: &Shared) -> Result<(), String> {
    let expected = with_model(model, |m| {
        (m.live.get(id).cloned(), m.maybe_deleted.contains(id))
    });
    match (
        send("server", "GET", &format!("/items/{id}"), None).await,
        expected,
    ) {
        (Reply::Answer(StatusCode::OK, item), (Some(name), _))
            if item["name"].as_str() == Some(name.as_str()) =>
        {
            Ok(())
        }
        (Reply::Answer(StatusCode::NOT_FOUND, _), (_, true)) | (Reply::Unknown, _) => Ok(()),
        (Reply::Answer(status, body), _) => Err(format!(
            "get {id}: acknowledged item answered {status} {body}"
        )),
    }
}

async fn delete(id: &str, model: &Shared) -> Result<(), String> {
    match send("server", "DELETE", &format!("/items/{id}"), None).await {
        Reply::Answer(StatusCode::NO_CONTENT, _) => {
            with_model(model, |m| {
                m.live.remove(id);
                m.deleted.insert(id.to_owned());
            });
            Ok(())
        }
        Reply::Answer(StatusCode::NOT_FOUND, _)
            if with_model(model, |m| m.maybe_deleted.contains(id)) =>
        {
            Ok(())
        }
        Reply::Answer(status, body) => Err(format!("delete {id}: unexpected {status} {body}")),
        Reply::Unknown => {
            with_model(model, |m| m.maybe_deleted.insert(id.to_owned()));
            Ok(())
        }
    }
}

/// Waits for the workload, then lists everything and checks it against the model.
async fn verify(model: &Shared, done: &Arc<Mutex<u32>>, clients: u32) -> Result<(), String> {
    let clock = TokioClock::new(Timestamp::from_millis(0));
    while *done.lock().unwrap_or_else(PoisonError::into_inner) < clients {
        clock.sleep(Duration::from_millis(50)).await;
    }
    let mut listed = BTreeMap::new();
    let mut offset = Some(0_u64);
    while let Some(start) = offset {
        let Reply::Answer(StatusCode::OK, page) = send(
            "server",
            "GET",
            &format!("/items?offset={start}&limit=100"),
            None,
        )
        .await
        else {
            return Err("final list failed".to_owned());
        };
        for item in page["items"].as_array().into_iter().flatten() {
            listed.insert(
                item["id"].as_str().unwrap_or_default().to_owned(),
                item["name"].as_str().unwrap_or_default().to_owned(),
            );
        }
        offset = page["next_offset"].as_u64();
    }
    with_model(model, |m| m.check(&listed))
}

/// Collects the run's JSON log lines.
struct LogWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
