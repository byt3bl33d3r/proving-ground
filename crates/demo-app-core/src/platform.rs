//! Platform seams: time, randomness, hash-map order, networking and fault injection.
//!
//! Domain code gets time only from [`Clock`] and randomness only from [`Rng`], both passed in
//! through the server's `AppState`. Under feature `sim` (harness/dst only) the network types
//! come from turmoil, hash maps use a fixed hasher and [`buggify!`](crate::buggify) is live.

pub mod buggify;
mod system;

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

pub use system::{HashMap, HashSet, SystemClock, SystemRng, TokioClock};

use crate::types::Timestamp;

/// A boxed, sendable future (keeps the platform traits dyn-compatible).
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Source of time. Real: [`SystemClock`]. Tests and simulation: [`TokioClock`] under paused
/// or simulated tokio time.
pub trait Clock: Send + Sync + fmt::Debug {
    /// Wall-clock time, for timestamps stored in data.
    fn now(&self) -> Timestamp;
    /// Monotonic time since an arbitrary origin, for measuring durations.
    fn monotonic(&self) -> Duration;
    /// Waits for `duration`.
    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()>;
}

/// Source of randomness. Real: [`SystemRng`]. Tests and simulation: [`SeededRng`].
pub trait Rng: Send + Sync + fmt::Debug {
    /// The next random 64 bits.
    fn next_u64(&self) -> u64;
}

/// Deterministic RNG: the same seed yields the same sequence.
#[derive(Debug)]
pub struct SeededRng(Mutex<fastrand::Rng>);

impl SeededRng {
    /// Creates an RNG from `seed`.
    pub fn new(seed: u64) -> Self {
        Self(Mutex::new(fastrand::Rng::with_seed(seed)))
    }
}

impl Rng for SeededRng {
    fn next_u64(&self) -> u64 {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .u64(..)
    }
}

/// Sockets. Real code binds through these so simulation can swap in turmoil's network.
pub mod net {
    #[cfg(not(feature = "sim"))]
    pub use tokio::net::{TcpListener, TcpStream};
    #[cfg(feature = "sim")]
    pub use turmoil::net::{TcpListener, TcpStream};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_rng_is_reproducible() {
        let first = SeededRng::new(7);
        let second = SeededRng::new(7);
        assert_eq!(
            first.next_u64(),
            second.next_u64(),
            "same seed, same sequence"
        );
    }

    #[cfg_attr(
        all(miri, not(target_os = "linux")),
        ignore = "tokio needs kqueue, which Miri lacks off Linux"
    )]
    #[tokio::test(start_paused = true)]
    async fn tokio_clock_follows_paused_time() {
        let clock = TokioClock::new(Timestamp::from_millis(1_000));
        clock.sleep(Duration::from_millis(250)).await;
        assert_eq!(
            clock.now(),
            Timestamp::from_millis(1_250),
            "virtual time advanced"
        );
        assert_eq!(
            clock.monotonic(),
            Duration::from_millis(250),
            "monotonic matches"
        );
    }
}
