//! The only module allowed to touch real time, sleeping and std hash maps (clippy.toml bans
//! them everywhere else). Keep it small.
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "core::platform is the single place that wraps real time, sleep and std hash maps"
)]

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{BoxFuture, Clock, Rng};
use crate::types::Timestamp;

/// Hash map with deterministic iteration under `sim` (fixed hasher). Still never iterate it
/// where order matters: clippy's `iter_over_hash_type` is denied.
#[cfg(feature = "sim")]
pub type HashMap<K, V> =
    std::collections::HashMap<K, V, std::hash::BuildHasherDefault<std::hash::DefaultHasher>>;
/// Hash map with the std hasher (outside simulation).
#[cfg(not(feature = "sim"))]
pub type HashMap<K, V> = std::collections::HashMap<K, V>;
/// Hash set with deterministic iteration under `sim` (fixed hasher).
#[cfg(feature = "sim")]
pub type HashSet<T> =
    std::collections::HashSet<T, std::hash::BuildHasherDefault<std::hash::DefaultHasher>>;
/// Hash set with the std hasher (outside simulation).
#[cfg(not(feature = "sim"))]
pub type HashSet<T> = std::collections::HashSet<T>;

/// The real clock: system wall time and tokio timers.
#[derive(Debug)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    /// Creates a clock whose monotonic origin is now.
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Timestamp::from_millis(u64::try_from(since_epoch.as_millis()).unwrap_or(u64::MAX))
    }

    fn monotonic(&self) -> Duration {
        self.origin.elapsed()
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
        Box::pin(tokio::time::sleep(duration))
    }
}

/// A clock driven entirely by tokio's timer: deterministic under `start_paused` tests and
/// under turmoil's simulated time. Wall time is `epoch` plus elapsed tokio time.
#[derive(Debug)]
pub struct TokioClock {
    epoch: Timestamp,
    origin: tokio::time::Instant,
}

impl TokioClock {
    /// Creates a clock that reads `epoch` now.
    pub fn new(epoch: Timestamp) -> Self {
        Self {
            epoch,
            origin: tokio::time::Instant::now(),
        }
    }
}

impl Clock for TokioClock {
    fn now(&self) -> Timestamp {
        let elapsed = u64::try_from(self.monotonic().as_millis()).unwrap_or(u64::MAX);
        Timestamp::from_millis(self.epoch.as_millis().saturating_add(elapsed))
    }

    fn monotonic(&self) -> Duration {
        self.origin.elapsed()
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
        Box::pin(tokio::time::sleep(duration))
    }
}

/// The real RNG: fastrand's entropy-seeded thread-local generator.
#[derive(Debug, Default)]
pub struct SystemRng;

impl Rng for SystemRng {
    fn next_u64(&self) -> u64 {
        fastrand::u64(..)
    }
}
