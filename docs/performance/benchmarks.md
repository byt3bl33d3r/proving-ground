---
type: Guide
title: Benchmarks
description: Instruction-count (Gungraun) and wall-clock (Criterion) benchmarks: which one gates and how baselines work.
tags: [performance, benchmarks, gungraun, criterion]
status: stable
code_refs: [crates/{{project-name}}-core/benches/instructions.rs, crates/{{project-name}}-core/benches/wall_clock.rs]
---

# Benchmarks

- **Gungraun** (`just gungraun`, tier 1, Linux with Valgrind): instruction counts are
  deterministic, so they gate. A regression of more than 2% in instructions exits with code 3.
  Baselines are not committed files: CI benchmarks the base commit with `just gungraun base`
  and the PR head with `just gungraun "" base`. On macOS the recipe reports
  `skipped: gungraun needs Valgrind (Linux)` with `ok: true`. `gungraun-runner` in mise.toml must
  equal the library version exactly.
- **Criterion** (`just criterion`, part of `just perf`): wall-clock confirmation only, never gates.

Performance claims need Gungraun numbers, not wall-clock alone. Accept a performance change when Gungraun improves, the tests still pass and Criterion agrees.
The functions measured are the [hot paths](/performance/hot-paths.md).
