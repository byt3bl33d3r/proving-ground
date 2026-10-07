//! FoundationDB-style fault injection (`buggify!`).
//!
//! Under feature `sim`, [`enable`] arms it for one simulation run: each call site is enabled
//! or disabled once per run by the seeded RNG, and enabled sites fire with probability
//! `fire_percent`. Without `sim`, `buggify!()` expands to the constant `false`. State is
//! per thread, because one turmoil simulation runs on one thread.

/// True when this call site should inject its fault in this run.
#[cfg(feature = "sim")]
#[macro_export]
macro_rules! buggify {
    () => {
        $crate::platform::buggify::fire(concat!(file!(), ":", line!(), ":", column!()))
    };
}

/// True when this call site should inject its fault in this run (always `false` without `sim`).
#[cfg(not(feature = "sim"))]
#[macro_export]
macro_rules! buggify {
    () => {
        false
    };
}

#[cfg(feature = "sim")]
mod sim {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    /// Share of call sites enabled per run, in percent.
    const SITE_ENABLED_PERCENT: u8 = 25;

    #[derive(Debug)]
    struct State {
        rng: fastrand::Rng,
        fire_percent: u8,
        sites: BTreeMap<&'static str, bool>,
    }

    thread_local! {
        static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    }

    /// Arms fault injection on this thread for one run.
    pub fn enable(seed: u64, fire_percent: u8) {
        let state = State {
            rng: fastrand::Rng::with_seed(seed),
            fire_percent,
            sites: BTreeMap::new(),
        };
        STATE.with(|cell| *cell.borrow_mut() = Some(state));
    }

    /// Disarms fault injection on this thread.
    pub fn disable() {
        STATE.with(|cell| *cell.borrow_mut() = None);
    }

    /// Decides whether `site` fires now. Used by `buggify!`.
    pub fn fire(site: &'static str) -> bool {
        STATE.with(|cell| {
            let mut guard = cell.borrow_mut();
            let Some(state) = guard.as_mut() else {
                return false;
            };
            let State {
                rng,
                fire_percent,
                sites,
            } = state;
            let enabled = *sites
                .entry(site)
                .or_insert_with(|| rng.u8(0..100) < SITE_ENABLED_PERCENT);
            enabled && rng.u8(0..100) < *fire_percent
        })
    }
}

#[cfg(feature = "sim")]
pub use sim::{disable, enable, fire};
