//! Server binary: configuration, telemetry, listener. Logic lives in the library.

use std::sync::Arc;

use demo_app_core::platform::{SystemClock, SystemRng};
use demo_app_server::{AppState, DEFAULT_REQUEST_TIMEOUT, router};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState::new(
        Arc::new(SystemClock::new()),
        Arc::new(SystemRng),
        DEFAULT_REQUEST_TIMEOUT,
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let app = router(state.clone());
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    state.mark_ready();
    server.await??;
    Ok(())
}
