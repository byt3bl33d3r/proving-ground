---
type: Guide
title: Test strategy
description: The kinds of tests (unit, property, snapshot, golden logs, transcripts, architecture, e2e, DST) and where each one lives.
tags: [testing, nextest, insta, proptest, trycmd]
status: stable
code_refs: [.config/nextest.toml]
---

# Test strategy

| Kind | Tool | Location | Recipe |
| --- | --- | --- | --- |
| Unit | nextest | `#[cfg(test)]` modules | `just test` |
| Property | proptest (regressions committed) | core | `just test` |
| API snapshots | insta JSON, ids redacted | `crates/<project>-server/tests/api.rs` (router via `oneshot`) | `just test` |
| Golden logs | JSON layer into memory | `crates/<project>-core/tests/golden_logs.rs` | `just test` |
| CLI transcripts | trycmd | `crates/<project>-cli/tests/cmd/*.toml` | `just test` |
| Architecture | `cargo metadata` + source scans | `harness/checks/tests/arch.rs` | `just test` |
| End to end | CLI client against `just up` | `harness/checks/tests/e2e.rs` | `just e2e` |
| DST | turmoil + mad-turmoil + buggify | `harness/dst` | `just dst` |

Integration test files start with `#![cfg(test)]` (clippy's `tests_outside_test_module`). Every
assertion carries a message. Time and randomness come from `core::platform`
([determinism](/testing/determinism.md)); the simulation is described in [DST](/testing/dst.md).
When a database is added, integration tests use testcontainers (random ports, automatic cleanup)
and the DST model gains a storage fault layer behind the `ItemRepo` trait.
