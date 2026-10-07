---
type: Guide
title: Modules and file size
description: File and function size limits, and how to split code by responsibility when a limit is hit.
tags: [conventions, modules, size, clippy]
status: stable
code_refs: [clippy.toml, harness/checks/tests/arch.rs]
---

# Modules and file size

- A `.rs` file over 500 lines fails the architecture test; a binary's `main.rs` must stay under
  80 lines (logic belongs in the library).
- Clippy limits functions to 80 lines, cognitive complexity 15, nesting depth 4, 5 arguments and
  one boolean parameter.
- Modules use `name.rs` plus `name/`, never `mod.rs` (`mod_module_files`).

When a limit is hit, split by responsibility, not by size: move a cohesive group of types or
functions into a submodule and keep the [layering](/architecture/layers.md) intact.
