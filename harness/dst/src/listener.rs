//! Lets `axum::serve` accept connections on a turmoil socket.

use std::net::SocketAddr;
use std::time::Duration;

use demo_app_core::platform::{Clock as _, TokioClock};
use demo_app_core::types::Timestamp;

/// A turmoil listener as an axum `Listener`.
pub struct TurmoilListener(pub turmoil::net::TcpListener);

impl axum::serve::Listener for TurmoilListener {
    type Io = turmoil::net::TcpStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        let clock = TokioClock::new(Timestamp::from_millis(0));
        loop {
            match self.0.accept().await {
                Ok(pair) => return pair,
                Err(e) => {
                    tracing::warn!(error = %e, "accept_failed");
                    clock.sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.0.local_addr()
    }
}
