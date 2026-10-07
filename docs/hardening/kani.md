---
type: Guide
title: Kani proofs
description: Kani proof harnesses in core: the quick tier, the full tier, and why string-heavy proofs run nightly.
tags: [hardening, kani, proofs, formal-verification]
status: stable
code_refs: [crates/demo-app-core/src/proofs.rs]
---

# Kani proofs

Harnesses live in `crates/<project>-core/src/proofs.rs` (`#[cfg(kani)]`).

- `quick_*` (`just kani`, tier 1, seconds): id parsing never panics on short inputs, a validated
  `Page` keeps its limit in range, and pagination math never overflows.
- `full_*` (`just kani-full`, tier 3, 60-minute budget): full-length id round trips and the
  `ItemName` invariant.

CBMC is slow on symbolic strings: `str::trim` alone took over four minutes because of Unicode
tables, which is why `ItemName` trims ASCII whitespace and its proof runs in tier 3. Names are
also covered by proptest and the `decode_create_item` [fuzz target](/hardening/fuzzing.md).
`--harness` is a substring filter, not a glob.
