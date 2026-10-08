---
type: Guide
title: Kani proofs
description: Kani proof harnesses in core: the quick tier, the full tier, and why string-heavy proofs run nightly.
tags: [hardening, kani, proofs, formal-verification]
status: stable
code_refs: [crates/{{project-name}}-core/src/proofs.rs]
---

# Kani proofs

Harnesses live in `crates/<project>-core/src/proofs.rs` (`#[cfg(kani)]`).

- `quick_*` (`just kani`, tier 1, seconds): id parsing never panics on short inputs, a validated
  `Page` keeps its limit in range, and pagination math never overflows.
- `full_*` (`just kani full`, tier 3): the hex-digit decoder behind `ItemId::parse` is exact for
  all 256 bytes.

Every run has a per-harness timeout (`--harness-timeout`, 2 minutes quick, 15 minutes full).
CBMC's memory grows with its formula, and harnesses over symbolic strings blow up: a full-length
id round trip ran out of memory at about 60 GB, and `str::trim` alone took over four minutes
because of Unicode tables. So proofs stay at the byte level, `ItemName` trims ASCII whitespace,
and string-level properties (id round trips, the `ItemName` invariant) are covered by proptest and
the [fuzz targets](/hardening/fuzzing.md). A harness that needs more than the timeout is a bug in
the harness: shrink its input or prove a smaller function.
`--harness` is a substring filter, not a glob.
