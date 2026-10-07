---
type: Guide
title: Template notes
description: Deviations from the template spec and the tool versions verified while building the template.
tags: [template, deviations, versions]
status: stable
code_refs: [mise.toml]
---

# Template notes: deviations and verified values

Generated from the {{template_repo}} at {{template_version}}.

## Deviations from the spec

- **AGENTS.md is a map into the okf bundle (human decision).** It keeps the project-memory
  instructions (in okf's managed block), a commands pointer, and every rule as a one-liner with a
  link to the concept that holds the detail. Commands, debugging and failure handling moved to
  conventions/agent-workflow and the okf how-to to conventions/project-memory.

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
  The `check`/`fix` hooks call `just secrets {% raw %}{{files}}{% endraw %}` because `gitleaks dir` scans a single path.
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
- **okf (v0.6.0):** `okf validate --strict --drift` does not fail on expired `stale_after` (needs
  `--stale`) and never fails on drift, so `just knowledge` runs `--strict --drift --stale --json`
  and fails on any warning. Plans cannot use `status: active|completed` (only draft, stable,
  deprecated): active plans are `status: draft` with tag `active`, finished ones `status: stable`
  with tag `completed`. There is no `trust` field: trust comes from `verified` entries, which only
  humans add. Shipped concepts carry no `stale_after` (a fixed date would expire in every
  generated project). `docs/generated/` stays: okf ignores non-Markdown files. okf checks only
  `.md` links to concepts (a gate under `--strict`) and `code_refs` paths (a `--drift` warning,
  which `just knowledge` fails on); it never sees paths in inline code or links to non-`.md`
  files. So concepts link other concepts with Markdown links and list the repo paths they govern
  in `code_refs`. `just knowledge` also runs `okf agents lint --strict AGENTS.md`
  (AAG rules, 400-token budget for the okf memory block between its BEGIN and END markers);
  the architecture test still enforces the 120-line limit and that every path in AGENTS.md exists.
- **Agents start tools through `mise x --`.** The MCP servers (`mise x -- just mcp ...`,
  `mise x -- okf mcp docs`, `mise x -- hk mcp`) and the Stop and WorktreeRemove hooks run through
  mise, as the git hooks do, so desktop agent apps that do not load the shell profile only need
  `mise` on their PATH, not mise activation or shims. SPEC lists `okf` as the bare command.
- **Agents:** Claude's Stop hook is hk's generated snippet; a `WorktreeRemove` hook runs
  `just down` in the worktree before removing it (replacing workz's `pre_done`). Codex reads
  `.codex/config.toml` and `.codex/hooks.json` only for trusted projects, and asks to trust each
  hook once. hk's own MCP server is added to both agents. A rust-analyzer MCP server and a Codex
  skill are not added (no verified project-level skills directory for Codex 0.157).
- **Kani:** the quick tier proves id parsing, the `Page` invariant and pagination math in seconds;
  the full tier proves the byte-level hex decoder. Harnesses over symbolic strings are not used:
  a full-length id round trip made CBMC run out of memory (~60 GB) and `str::trim` alone took
  254 s, so the `ItemName` invariant and id round trips are covered by proptest and fuzzing, and
  `ItemName` trims ASCII whitespace only. Kani runs with `-Z unstable-options --harness-timeout`
  (2 min quick, 15 min full) so a runaway harness is killed rather than exhausting memory.
- **Miri** runs on core; tests that need tokio's runtime are ignored off Linux (Miri has no
  kqueue), and proptests and real-clock tests are ignored under Miri (file and clock access).
- **Fuzz corpora:** the template ships a seed corpus only; generated projects commit their own
  minimized corpora (`cargo fuzz cmin`).
- **cargo-mutants** exit code 3 (some mutants timed out) counts as a pass: a timeout means the
  tests caught the mutant.
- **cargo-generate (0.25.0):** placeholder defaults are not rendered, so `pre.rhai` replaces the
  literal `service_name` default with the project name; `template_version` and `template_repo`
  are set by `init.rhai` (there is no template metadata field, and a placeholder without a prompt
  is an error); `post.rhai` deletes the leftover empty `template-hooks/` directory. `ignore` takes
  literal paths only. `mise.toml` and `README.md` come from `.liquid` twins, because mise renders
  its own double-brace templates and so cannot read a file with Liquid placeholders. The floor is
  `cargo_generate_version = ">=0.25.0"` (the version tested).
- Generated projects get a short `README.md`; `CODEOWNERS` names `@<gh_owner>/maintainers`.
  Template-only recipes live in `template.just`, imported optionally by the `justfile` and
  dropped on generation.
- The `template-ci` leftover check cannot catch {% raw %}`${{ x }}`{% endraw %} collapsing to `$` (it leaves
  nothing to grep), so workflows are excluded from Liquid entirely and templated files avoid
  double-brace escapes.
- **Coverage baseline** is measured on a clean checkout (`just coverage` now runs
  `cargo llvm-cov clean` first): an earlier 71.0% reading included stale profiles; a fresh
  generated project measured 67.7%. Unit tests for the CLI's error mapping and text output brought
  it to 71.7%, so `COV_MIN_REGIONS` is 71.2.
- **Fresh-machine installs (found in a clean Linux container):** `aqua:mitsuhiko/insta` has no
  linux/arm64 binary, so cargo-insta comes from the `cargo:` backend; and parallel `cargo install`s
  each triggered rustup to install the pinned toolchain and raced on its components, so
  `mise.toml` has `[hooks] preinstall = "rustup toolchain install"` (a no-op once installed).
- **Offline cargo-deny** (`just deny-offline`, a pre-commit step) runs `cargo fetch --locked`
  first: `cargo metadata --offline` needs every platform's crates (for example `windows-sys`),
  and a Linux build only downloads the host's.
- **Gungraun in containers:** Valgrind runs under `setarch -R` (ASLR off, for reproducible
  counts), which Docker's default seccomp profile blocks (`failed to set personality`). GitHub's
  Ubuntu runners are VMs and allow it; inside Docker use `--security-opt seccomp=unconfined`.
- `just bootstrap` only warns when Docker is missing: it is needed for `just up` and `just e2e`,
  not for building, hooks or the CI tiers.
- **Local CI with act** (`just ci-local <job>`): caching behaves differently from GitHub,
  artifacts go to local storage, and steps that call the GitHub API need
  `-s GITHUB_TOKEN=...`. Logic stays in recipes, so a fix never goes in workflow YAML.
- `just check` skips the "hooks installed" guard when `CI=true` (CI runners never run `hk install`).
- **Justfile lint (added at the maintainer's request; SPEC says no shell lint).** Recipe bodies are
  bash, so `just lint-recipes` checks `just --fmt` layout and runs shellcheck (pinned in
  `mise.toml`) on every recipe body taken from `just --dump --dump-format json`. Interpolations
  become `${JUST_EXPR}`, and warnings and errors fail while notes do not: an unquoted interpolation
  of a variadic parameter is word-split on purpose. It runs in `just check` and as the hk `just-recipes` step.
- **The hk `knowledge` step has no glob** (SPEC: `docs/**`): concepts list governed code in
  `code_refs`, so moving or deleting a file outside docs/ can break the bundle. It takes well
  under a second.
- **Template repository hooks.** cargo-generate's placeholders make hk.pkl's Rust steps fail in
  the template repository itself, so its `mise.toml` (not `mise.toml.liquid`) sets
  `HK_FILE=template.hk.pkl`: secrets, actionlint, the justfile lint and `just knowledge` run there,
  and the Rust steps are skipped. hk.pkl is exercised by `just template-ci`.

## Verified values

| Item | Value | How verified |
| --- | --- | --- |
| Stable toolchain | 1.98.1 | `rustup show` |
| Lint table (Section 9) | every name known to clippy 1.98.1 | `cargo clippy -- -D warnings` with `unknown_lints = "deny"` |
| clippy bans | `disallowed-*` reasons shown in errors; `allow-invalid = true` for crates not in the graph (rand, log) | planted `println!`/`Instant::now()` |
| just | 1.58.0 | `mise install` |
| hk / pkl | 2.5.0 / 0.32.1 | `hk validate`, `pkl eval hk.pkl`, real commits |
| Analysis nightly | nightly-2026-10-07 (miri, rust-src, llvm-tools on macOS arm64, Linux x86_64/aarch64) | dist manifests, rustup install |
