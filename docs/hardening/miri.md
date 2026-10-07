---
type: Guide
title: Miri
description: How Miri checks core's tests for undefined behaviour in tier 1, and which tests it skips.
tags: [hardening, miri, undefined-behaviour, nightly]
status: stable
code_refs: [justfile, crates/{{project-name}}-core/tests/golden_logs.rs]
---

# Miri

`just miri` runs core's tests under Miri (`cargo +$NIGHTLY miri nextest run`) with
`-Zmiri-strict-provenance`, as part of [tier 1](/hardening/tiers.md). Miri interprets every test,
so property tests run 8 cases each (`PROPTEST_CASES=8`).

Miri isolates tests from the host, so a test that reads the real clock, the network or files is
skipped with `#[cfg_attr(miri, ignore = "<reason>")]`. The golden-log test is one: the JSON log
formatter reads the real-time clock. Code under test gets time and randomness from
`core::platform` ([determinism](/testing/determinism.md)), so most tests need no exception.

A Miri failure means undefined behaviour (or a new host dependency in a test); reproduce it with
`just miri` and fix the code rather than ignoring the test.
