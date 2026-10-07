//! Server binary: telemetry, configuration, listener, graceful shutdown. Logic lives in the
//! library; this file only wires it (keep it under 80 lines).

use std::sync::Arc;

use {{crate_name}}_core::platform::{Clock, SystemClock, SystemRng};
use {{crate_name}}_runtime::config::{AppInfo, ServerConfig};
use {{crate_name}}_runtime::telemetry::{self, Mode};
use {{crate_name}}_server::{AppState, router};
use tokio::net::TcpListener;
use tracing::{Instrument as _, info, info_span};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _telemetry = telemetry::init(Mode::Server, env!("CARGO_PKG_VERSION"), None)?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let (listener, state, config) = start(Arc::clone(&clock))
        .instrument(info_span!("startup"))
        .await?;
    let server = tokio::spawn(
        axum::serve(listener, router(state.clone()))
            .with_graceful_shutdown(shutdown())
            .into_future(),
    );
    state.mark_ready();
    let served = server.await;
    AppInfo::remove(&config.harness_dir)?;
    info!("server_stopped");
    Ok(served??)
}

/// Config load to listening: the `startup` span the 800 ms budget measures.
async fn start(clock: Arc<dyn Clock>) -> anyhow::Result<(TcpListener, AppState, ServerConfig)> {
    let config = ServerConfig::load()?;
    let state = AppState::new(
        Arc::clone(&clock),
        Arc::new(SystemRng),
        config.request_timeout(),
    );
    let listener = TcpListener::bind((config.host, config.port)).await?;
    let addr = listener.local_addr()?;
    let info = AppInfo {
        url: format!("http://{addr}"),
        pid: std::process::id(),
        started_at: clock.now().as_millis(),
    };
    info.write(&config.harness_dir)?;
    info!(listen_addr = %addr, url = %info.url, "server_listening");
    Ok((listener, state, config))
}

/// Resolves on Ctrl-C or SIGTERM (`just down`).
async fn shutdown() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => signal.recv().await,
            Err(_unsupported) => std::future::pending().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<Option<()>>();
    tokio::select! {
        result = ctrl_c => if let Err(e) = result { tracing::warn!(error = %e, "ctrl_c_handler_failed") },
        _signal = terminate => {}
    }
    info!("shutdown_requested");
}
