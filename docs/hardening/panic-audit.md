---
type: Guide
title: Panic audit
description: How the LLVM panic audit finds new panic paths in core's release build and how the allowlist works.
tags: [hardening, panic, asm, llvm]
status: stable
code_refs: [docs/generated/panic-allowlist.txt, justfile]
---

# Panic audit

`just panic-audit` (pre-commit when core changes, and tier 1) builds core in release with
`$NIGHTLY` and `-Zcross-crate-inline-threshold=never` (otherwise small functions are only
compiled in their callers' crates and stay invisible), then lists functions owned by core that
call a panicking function (`cargo asm --llvm --callers-of`, matched on mangled names such as
`panic_bounds_check` and `unwrap_failed`). Anything not in `docs/generated/panic-allowlist.txt`
fails with: `New panic path in <fn>. Remove it (iterators, hoisted assert, checked ops) or add it
to the allowlist with a reason in the PR.` The allowlist is protected. Part of
[tier 1](/hardening/tiers.md).
