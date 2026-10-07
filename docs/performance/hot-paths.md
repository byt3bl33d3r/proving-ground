---
type: Guide
title: Hot paths
description: Functions whose assembly is snapshotted and whose instruction counts are benchmarked, and how to add one.
tags: [performance, asm, benchmarks]
status: stable
code_refs: [crates/{{project-name}}-core/benches/instructions.rs, harness/checks/tests/asm.rs]
---

# Hot paths

Each function listed below is marked `#[inline(never)]`, has an instruction-count benchmark in
`crates/<core>/benches/instructions.rs`, and has its assembly (`cargo asm --simplify`) captured
as an insta snapshot by `just asm-snapshots`. An assembly diff fails until someone reviews it with
`cargo insta review`; accept it only with Gungraun numbers that justify it
(see [benchmarks](/performance/benchmarks.md)).

```text hot-paths
<{{crate_name}}_core::types::ItemId>::parse
<{{crate_name}}_core::types::Page>::bounds
```

To add a hot path: mark the function `#[inline(never)]`, add a line above (the name exactly as
`cargo asm --lib -p <core>` lists it), add a benchmark, then run `just asm-snapshots` and review the new snapshot.
