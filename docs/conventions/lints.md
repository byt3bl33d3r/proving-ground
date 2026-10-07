---
type: Guide
title: Lint policy
description: The lint policy: denied groups, the expect escape hatch, the exception ledger, and the protected files.
tags: [conventions, lints, clippy, protected-files]
status: stable
code_refs: [Cargo.toml, clippy.toml, deny.toml, .config/nextest.toml, docs/generated/lint-exceptions.txt, docs/generated/panic-allowlist.txt, harness/lints, harness/checks/tests/arch.rs]
---

# Lint policy

- `[workspace.lints]` denies clippy's `all`, `pedantic` and `cargo` groups plus selected
  restriction and nursery lints; `cargo clippy -- -D warnings` runs in every workspace.
- The only escape hatch is `#[expect(lint, reason = "...")]` on the smallest item. `#[allow]` is
  an error, and an expectation that no longer fires is an error.
- Every `#[expect]` is listed in `docs/generated/lint-exceptions.txt`; a new one fails the
  architecture test until `just docs` records it in the same PR, so reviewers see it.
- `clippy.toml` bans with remedies: see [determinism](/testing/determinism.md) and
  [fields](/observability/fields.md). Size and complexity limits are in
  [modules](/conventions/modules.md); the panic and `process::exit` bans pair with
  [errors and exit codes](/conventions/errors.md).

Protected files (ask a human first, never weaken): `clippy.toml`, `deny.toml`,
`[workspace.lints]`, `.config/nextest.toml`, `COV_MIN_REGIONS` in mise.toml, the budgets, the
lint-exception ledger, the panic allowlist, `harness/lints/` and `harness/checks/tests/arch.rs`.
The hk `protected-files` step warns when a commit touches them.
