//! Golden logs: the JSON log layer writes into memory, volatile fields are normalized, and the
//! result is snapshotted. A changed message or field name shows up here as a snapshot diff.
#![cfg(test)]

use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use {{crate_name}}_core::domain::ItemService;
use {{crate_name}}_core::platform::{SeededRng, TokioClock};
use {{crate_name}}_core::repo::InMemoryItemRepo;
use {{crate_name}}_core::types::Timestamp;
use serde_json::Value;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);

impl io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Buffer {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Drops fields that change from run to run.
fn normalize(line: &str) -> Value {
    let mut event: Value = serde_json::from_str(line).expect("JSON log line");
    if let Some(object) = event.as_object_mut() {
        object.remove("timestamp");
        object.remove("threadId");
        object.remove("threadName");
    }
    event
}

#[cfg_attr(
    miri,
    ignore = "the JSON log formatter reads the real-time clock, which Miri isolation forbids"
)]
#[tokio::test(start_paused = true)]
async fn domain_events_have_stable_shape() {
    let buffer = Buffer::default();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(buffer.clone())
        .finish();
    let _default = tracing::subscriber::set_default(subscriber);
    let items = ItemService::new(
        Arc::new(InMemoryItemRepo::default()),
        Arc::new(TokioClock::new(Timestamp::from_millis(1_700_000_000_000))),
        Arc::new(SeededRng::new(7)),
    );
    let item = items.create("pen").await.expect("create");
    items.delete(&item.id.to_string()).await.expect("delete");
    let text = String::from_utf8(buffer.0.lock().expect("buffer").clone()).expect("utf8");
    let events: Vec<Value> = text.lines().map(normalize).collect();
    insta::assert_json_snapshot!("domain_events", events);
}
