//! Domain types, services, repository and platform seams.
//!
//! Module layering (enforced by `harness/checks/tests/arch.rs`): `types` uses nothing,
//! `platform` uses `types`, `repo` uses `types` and `platform`, `domain` uses all three.
//! Every module may use `fields`. No axum, no OpenTelemetry SDK, no socket code.
#![deny(clippy::arithmetic_side_effects)]

pub mod domain;
pub mod fields;
pub mod platform;
pub mod repo;
pub mod types;
