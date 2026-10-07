//! Deterministic simulation of the items service: one turmoil host serves
//! `server::router(state)`, client hosts drive load over hyper, the network partitions and
//! holds messages, and `buggify!` sites in core inject slow paths and spurious errors. A shadow
//! model checks the final state. See docs/testing/dst.md.

mod client;
mod listener;
mod model;
mod scenario;

pub use scenario::{Config, Failure, run};

/// Seeds every randomness source the simulation can reach. Safe to call once per seed.
pub fn seed_all(seed: u64) {
    let rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(seed);
    match mad_turmoil::rand::try_rng() {
        // set_rng panics on a second call, so later seeds replace the value in place.
        Some(mut current) => *current = rng,
        None => mad_turmoil::rand::set_rng(rng),
    }
    fastrand::seed(seed);
}
