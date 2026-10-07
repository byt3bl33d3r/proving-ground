---
type: Guide
title: Fuzzing
description: Fuzz targets, corpora and the crash workflow: reproduce, minimize, add a regression test, fix.
tags: [hardening, fuzzing, cargo-fuzz, libfuzzer]
status: stable
code_refs: [harness/fuzz/Cargo.toml, harness/fuzz/corpus]
---

# Fuzzing

`harness/fuzz` is a separate cargo-fuzz workspace; every recipe passes `--fuzz-dir harness/fuzz`.

| Target | Checks |
| --- | --- |
| `parse_item_id` | parsing never panics; accepted ids round-trip |
| `decode_create_item` | JSON request bodies decode safely; accepted names uphold the invariant |
| `differential_core` | random operation sequences agree between `ItemService` and a `BTreeMap` model |

`just fuzz <target> <secs>` (nightly runs each for 10 minutes). On a crash the recipe prints the
repro and minimize commands (`just fuzz-repro`, `just fuzz-tmin`). Save the minimized input as a
regression test and fix it in the same PR. Corpora live in `harness/fuzz/corpus/`; run
`cargo fuzz cmin` before committing them. Part of [tier 3](/hardening/tiers.md).
