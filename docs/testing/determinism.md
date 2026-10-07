---
type: Guide
title: Determinism
description: Why time, randomness, hashing order and network go through core::platform, and how tests stay deterministic.
tags: [testing, determinism, platform, clock, rng]
status: stable
code_refs: [crates/{{project-name}}-core/src/platform.rs, crates/{{project-name}}-core/src/platform/system.rs, clippy.toml]
---

# Determinism

Every test must give the same result every run. So:

- Time comes only from `platform::Clock` (`now`, `monotonic`, `sleep`). Real: `SystemClock`.
  Tests and simulation: `TokioClock`, driven by tokio's paused or simulated time
  (`#[tokio::test(start_paused = true)]`).
- Randomness comes only from `platform::Rng`. Real: `SystemRng`. Tests: `SeededRng::new(seed)`.
  Request ids are UUID v7 built from the clock and RNG, never `Uuid::now_v7()`.
- Maps whose order could matter use `platform::HashMap`/`HashSet` (fixed hasher under `sim`);
  clippy's `iter_over_hash_type` forbids iterating them anyway.
- Sockets in library code go through `platform::net` (turmoil under `sim`).

`clippy.toml` bans `Instant::now`, `SystemTime::now`, `tokio::time::sleep`, `thread::sleep`,
`rand::rng`, std hash maps and `Uuid::now_v7`, each with this page as the remedy. The one place
allowed to call them is `core::platform::system`. The [DST](/testing/dst.md) self-check
`dst_is_deterministic` catches anything that slips through.
