---
type: Guide
title: Layers
description: Allowed dependencies between crates and between modules inside core, and how the architecture tests enforce them.
tags: [architecture, layering, dependencies]
status: stable
code_refs: [harness/checks/tests/arch.rs]
---

# Layers

| Crate | May depend on (internal) |
| --- | --- |
| core | nothing |
| runtime | core |
| server | core, runtime |
| cli | core, runtime |
| checks (test-only) | cli, core |
| dst (separate workspace) | core and server, both with feature `sim` |

Inside core: `types` uses nothing; `platform` uses `types`; `repo` uses `types` and `platform`;
`domain` uses all three; every module may use `fields`. `harness/checks/tests/arch.rs` reads
`cargo metadata` and scans `use crate::` paths; a violation fails with
`Forbidden dependency: <from> -> <to>. Allowed for <from>: [...]`. Move the shared code down a
layer instead of adding an edge. If core passes about 3,000 lines, split it along these module
lines into crates. The overall picture is in the [architecture overview](/architecture/overview.md).
