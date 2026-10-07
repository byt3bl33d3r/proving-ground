---
type: Guide
title: Check tiers
description: The five check tiers (check, ci, perf, harden, release), what each runs and where it runs.
tags: [hardening, ci, tiers, coverage]
status: stable
code_refs: [justfile]
---

# Check tiers

| Tier | Recipe | Runs on | Adds | Gate |
| --- | --- | --- | --- | --- |
| 0 | `just check` | pre-commit (plus affected tests and the panic audit); the agent Stop hook runs the lint steps | fmt, clippy `-D warnings` in every workspace, nextest, machete, `just knowledge`, `just lint-recipes` (shellcheck on the justfile) | hard |
| 1 | `just ci` | every PR; pre-push runs `just ci-fast` | `hk check --all`, cargo-deny, dylint, coverage gate, panic audit, asm snapshots, Gungraun, Miri, Kani quick, DST 200 seeds | hard |
| 2 | `just perf` | manual | optimization remarks, llvm-mca, Criterion, llvm-lines, build timings | informational |
| 3 | `just harden` | nightly | fuzzing, ASan, TSan, Kani full, DST 100k seeds, cargo-mutants | hard, opens an issue |
| 4 | `just release` | tags | public API diff | hard |

Every check writes `target/harness/<check>.json` with `check`, `ok`, `summary`, `details_path`
and `repro`. Coverage: region coverage must stay at least `COV_MIN_REGIONS` (mise.toml);
`just cov-ratchet` raises it, nothing lowers it.

- **Mutation testing** (`just mutants`, tier 3) runs cargo-mutants on core with nextest and lists
  surviving mutants in `target/harness/mutants.json`; a survivor is a missing test.
- **Release** (`just release`, tier 4) diffs each library crate's public API against the latest
  tag (`BASE=<ref>` to change it). Removed or changed items fail unless `ALLOW_BREAKING=1`, which
  the PR must explain.

Details: [panic audit](/hardening/panic-audit.md), [Kani](/hardening/kani.md),
[Miri](/hardening/miri.md), [fuzzing](/hardening/fuzzing.md), [sanitizers](/hardening/sanitizers.md),
[benchmarks](/performance/benchmarks.md). Where each tier runs: [CI workflows](/workflows/ci.md)
and [git hooks](/workflows/git-hooks.md).
