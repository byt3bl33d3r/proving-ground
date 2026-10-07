# Template notes: deviations and verified values

Generated from the rust-project-template at v1.0.0.

## Deviations from the spec

- **workz dropped (human decision, SPEC §13 item 4).** workz 0.11.0 ignores `[isolation] base_port`
  (ranges start at 3000), refuses `sync --isolated` in the main checkout, and keys allocations by
  branch slug across all repos (two repos on the same branch, or any detached-HEAD worktrees,
  share a range). Ports now come from a hash of the worktree path in the `justfile`
  (Section 6's documented fallback): no `.workz.toml`, no registry, no WorktreeCreate hook.

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
- **hk hooks are git-config hooks.** `hk install --mise` on Git >= 2.54 writes `hook.hk-pre-commit`
  entries to `.git/config` instead of `.git/hooks/pre-commit`. The "hooks installed" guard checks
  `git hook list pre-commit` (and falls back to the legacy file for older Git).
- **hk builtins:** there is no `gitleaks_staged`; pre-commit uses `(Builtins.gitleaks) { scan = "staged" }`.
  The `check`/`fix` hooks call `just secrets {{files}}` because `gitleaks dir` scans a single path.
  `Builtins.taplo` only lints, so the step uses `Builtins.taplo_format` (`.taplo.toml` keeps the
  layout). Every custom step is a `CommandSpec` with an `effect`, which `hk run check --safe`
  (the agent Stop hook) requires.
- **Agent Stop hook** is hk's generated snippet (`hk agent hooks --target claude-code|codex`): it runs
  `hk agent stop-hook`, i.e. `hk run check --safe` on modified files, and blocks with a JSON
  decision. It checks but does not fix, unlike the spec's `hk fix --unstaged && hk check --unstaged`.
- **Panic audit** uses `cargo +$NIGHTLY asm --llvm` with `-Zcross-crate-inline-threshold=never`:
  since Rust 1.75 small non-generic functions are only codegen'd in their callers' crates, so on
  stable a new panicking helper is invisible. `--callers-of` matches *mangled* names, so the
  regex uses fragments (`panic_bounds_check|unwrap_failed|...|9panicking5panic`), and only
  functions owned by the core crate are kept (std generics instantiated with core types are not).
  `--llvm` mode also works on macOS, where assembly mode finds no call graph.
- **dylint** (`harness/lints`, nightly-2026-08-20 from `cargo dylint new`): `telemetry_field_style`
  is a *pre-expansion* pass, the only point where `#[instrument(fields(...))]` and the raw macro
  arguments are still visible. `cargo dylint --all` exits 0 on warnings, so `just dylint` sets
  `DYLINT_RUSTFLAGS="-D warnings"`. Both `just dylint` and the UI tests unset `RUSTC_WRAPPER`:
  under sccache, dylint_testing sees no rustc invocations. The root `clippy.toml` thresholds also
  apply to the separate workspaces (clippy searches parent directories).
- **Lint-exception ledger** (`docs/generated/lint-exceptions.txt`) lists every `#[expect]` as
  `path: lints -- reason` (no line numbers, so edits elsewhere in a file don't churn it). The
  architecture test compares it; `just docs` (`CHECKS_BLESS=1`) rewrites it and the crate graph.
- `std::env::var_os` is banned alongside `std::env::var`.
- `core -> server` as a normal dependency is a Cargo cycle, so Cargo rejects it before any test
  runs; the architecture test is demonstrated with a dev-dependency (and with `cli -> server`).
- `just check` skips the "hooks installed" guard when `CI=true` (CI runners never run `hk install`).

## Verified values

| Item | Value | How verified |
| --- | --- | --- |
| Stable toolchain | 1.98.1 | `rustup show` |
| Lint table (Section 9) | every name known to clippy 1.98.1 | `cargo clippy -- -D warnings` with `unknown_lints = "deny"` |
| clippy bans | `disallowed-*` reasons shown in errors; `allow-invalid = true` for crates not in the graph (rand, log) | planted `println!`/`Instant::now()` |
| just | 1.58.0 | `mise install` |
| hk / pkl | 2.5.0 / 0.32.1 | `hk validate`, `pkl eval hk.pkl`, real commits |
| Analysis nightly | nightly-2026-10-07 (miri, rust-src, llvm-tools on macOS arm64, Linux x86_64/aarch64) | dist manifests, rustup install |
