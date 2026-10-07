---
type: Guide
title: Deterministic simulation testing
description: Deterministic simulation testing with turmoil, mad-turmoil and buggify: topology, faults, oracle, seeds and reproduction.
tags: [testing, dst, simulation, turmoil, buggify, seeds]
status: stable
code_refs: [harness/dst/src/scenario.rs, crates/demo-app-core/src/platform/buggify.rs]
---

# Deterministic simulation testing

`harness/dst` is a separate workspace that builds core and server with feature `sim`, so the
main workspace never compiles the simulated platform. mad-turmoil replaces the process clock,
so it is only ever a dependency there.

- **Topology:** one turmoil host serves `server::router(state)` through a `Listener` adapter;
  four client hosts send requests with hyper (reqwest cannot run inside turmoil).
- **Faults:** random partitions and repairs, message hold and release, 1-30 ms latency, and the
  `buggify!` sites in core (slow `get`, spurious `Unavailable` on create, delayed write).
  `buggify!` is a hand-rolled FoundationDB pattern (each site enabled once per run by the seed,
  enabled sites fire with a configured probability); it is `false` without `sim`.
- **Oracle:** each client only touches items it created. A shadow model records acknowledged
  creates and deletes, plus operations whose outcome is unknown (timeouts, partitions). After
  the run the full listing must contain every acknowledged item, no acknowledged delete, and
  nothing that was never created.
- **Seeds:** `just dst` runs 200 seeds (about 25 s); nightly runs 100,000 in shards;
  `just dst SEEDS=<n> START=<k>` on demand. Every failure prints
  `DST failure. Reproduce: just dst SEED=<n> TEST=<test>` and saves the run's log.
- **Self-check:** `dst_is_deterministic` runs one seed in two fresh processes and compares the
  normalized log hashes. If it fails, a nondeterminism source leaked past
  [core::platform](/testing/determinism.md); fix that first.
