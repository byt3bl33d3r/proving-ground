# Template notes: deviations and verified values

Generated from the rust-project-template at v1.0.0.

## Deviations from the spec

- Integration-test files start with `#![cfg(test)]`: clippy's `tests_outside_test_module` (1.98.1)
  fires on `#[test]` fns at the root of `tests/*.rs`; the crate-level attribute satisfies it.
- Every crate sets `publish = false` (inherited from `[workspace.package]`), so clippy's
  `cargo_common_metadata` does not require `readme`/`keywords`/`categories`.
- `just check` skips the "hooks installed" guard when `CI=true` (CI runners never run `hk install`).

## Verified values

| Item | Value | How verified |
| --- | --- | --- |
| Stable toolchain | 1.98.1 | `rustup show` |
| Lint table (Section 9) | every name known to clippy 1.98.1 | `cargo clippy -- -D warnings` with `unknown_lints = "deny"` |
| clippy bans | `disallowed-*` reasons shown in errors; `allow-invalid = true` for crates not in the graph (rand, log) | planted `println!`/`Instant::now()` |
| just | 1.58.0 | `mise install` |
