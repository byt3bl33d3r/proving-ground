# Template notes: deviations and verified values

Generated from the rust-project-template at v1.0.0.

## Deviations from the spec

- Integration-test files start with `#![cfg(test)]`: clippy's `tests_outside_test_module` (1.98.1)
  fires on `#[test]` fns at the root of `tests/*.rs`; the crate-level attribute satisfies it.
- Every crate sets `publish = false` (inherited from `[workspace.package]`), so clippy's
  `cargo_common_metadata` does not require `readme`/`keywords`/`categories`.
- `min-ident-chars-threshold = 1`, not 2: clippy lints names whose length is *at most* the
  threshold, so 2 would ban `id`, `ok` and even `'_`. 1 matches the spec's intent (single-letter
  names banned except the allowlist).
- `TokioClock` (time from tokio's timer) is always compiled, not only under `sim`: paused-time
  unit tests in the main workspace need a deterministic clock, and enabling `sim` there is
  forbidden. `SeededRng` likewise. Only the hash-map hasher, `net` and live `buggify!` switch on `sim`.
- `cargo clippy --all-features` compiles `core` with `sim` (linting only); `just test` never passes
  `--all-features`. Because of that the server binary binds with `tokio::net::TcpListener`
  directly; `platform::net` is for library code that opens sockets.
- Starting size: non-test code is core 537, runtime 76, server 340, cli 569 lines, above the
  "about 200" guideline. The extra lines are the surface the spec requires (platform seams,
  buggify, typed errors, the exit-code mapping), not padding.
- `just check` skips the "hooks installed" guard when `CI=true` (CI runners never run `hk install`).

## Verified values

| Item | Value | How verified |
| --- | --- | --- |
| Stable toolchain | 1.98.1 | `rustup show` |
| Lint table (Section 9) | every name known to clippy 1.98.1 | `cargo clippy -- -D warnings` with `unknown_lints = "deny"` |
| clippy bans | `disallowed-*` reasons shown in errors; `allow-invalid = true` for crates not in the graph (rand, log) | planted `println!`/`Instant::now()` |
| just | 1.58.0 | `mise install` |
