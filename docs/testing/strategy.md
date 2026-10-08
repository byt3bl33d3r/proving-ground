---
type: Guide
title: Test strategy
description: The kinds of tests (unit, property, snapshot, golden logs, transcripts, architecture, e2e, DST) and where each one lives.
tags: [testing, nextest, insta, proptest, trycmd]
status: stable
code_refs: [.config/nextest.toml, crates/{{project-name}}-server/tests/api.rs, crates/{{project-name}}-core/tests/golden_logs.rs, crates/{{project-name}}-cli/tests/cmd, harness/checks/tests/arch.rs, harness/checks/tests/recipes.rs, harness/checks/tests/nextest.rs, harness/checks/tests/e2e.rs, harness/dst, harness/dst/.config/nextest.toml]
---

# Test strategy

| Kind | Tool | Location | Recipe |
| --- | --- | --- | --- |
| Unit | nextest | `#[cfg(test)]` modules | `just test` |
| Property | proptest (regressions committed) | core | `just test` |
| API snapshots | insta JSON, ids redacted | `crates/<project>-server/tests/api.rs` (router via `oneshot`) | `just test` |
| Golden logs | JSON layer into memory | `crates/<project>-core/tests/golden_logs.rs` | `just test` |
| CLI transcripts | trycmd | `crates/<project>-cli/tests/cmd/*.toml` | `just test` |
| Architecture | `cargo metadata` + source scans; every `just <recipe>` in docs/ exists; every `NEXTEST_PROFILE` a workflow sets exists in each nextest workspace | `harness/checks/tests/arch.rs`, `recipes.rs`, `nextest.rs` | `just test` |
| End to end | CLI client against `just up` | `harness/checks/tests/e2e.rs` | `just e2e` |
| DST | turmoil + mad-turmoil + buggify | `harness/dst` | `just dst` |

Every bug fix adds a failing test first: a unit or snapshot test, a trycmd transcript, an e2e
test, a DST seed or a fuzz regression. A new failure mode in the domain gets a `buggify!` site, so
`just dst` explores it. Integration test files start with `#![cfg(test)]` (clippy's `tests_outside_test_module`). Every
assertion carries a message. Time and randomness come from `core::platform`
([determinism](/testing/determinism.md)); the simulation is described in [DST](/testing/dst.md).
There are no browser or UI tests: the template has no UI. The plan for one (Playwright, WebDriver
or `egui_kittest`, plus before-and-after recordings) is the [Adding a UI](/decisions/adding-a-ui.md)
decision; [UI evidence](/testing/evidence.md) is the matching how-to.
When a database is added, integration tests use testcontainers (random ports, automatic cleanup)
and the DST model gains a storage fault layer behind the `ItemRepo` trait.

`harness/dst` is a separate workspace with its own `.config/nextest.toml`. CI sets
`NEXTEST_PROFILE=ci` for every recipe, so a profile added to the root config must be added there too,
or `just dst` fails with "profile not found" before running a test (`nextest.rs` checks this).
