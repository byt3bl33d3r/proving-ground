# {{project-name}}: agent map

{{description}}. Rust workspace: core + runtime libraries, axum server, clap CLI.
This file is a map, not a manual: details live in docs/ (an OKF knowledge bundle). Keep it short.

## Commands (run these, not raw cargo)
| Need | Run |
| --- | --- |
| Tier 0: fmt, clippy, tests, machete, docs check, justfile lint (also on commit) | `just check` |
| Tier 1: everything CI runs on a PR (pre-push runs `just ci-fast`) | `just ci` |
| Per-worktree stack + server; URLs in .harness/app.json | `just up` / `just down` / `just status` |
| End-to-end tests + latency budgets (needs `just up`) | `just e2e` |
| See the app | `just logs-errors`, `just logs-request <id>`, `just q-logs`, `just q-metrics`, `just q-traces`, `just trace <id>` |
| Deterministic simulation | `just dst` or `just dst SEEDS=1000` |
| Performance signals (never gate) | `just perf` |
| Fuzzing, sanitizers, full proofs, mutants | `just harden` |
| Regenerate docs/generated/ after review | `just docs` |
| Validate project memory | `just knowledge` |

Every check writes target/harness/<check>.json; `just --list` shows all recipes.

## Layout
crates/: core (types, domain, repo, platform seams), runtime (config, telemetry), server (axum), cli (clap).
harness/: checks (architecture + e2e tests), dst (simulation), fuzz, lints (dylint), stack (local Victoria compose).
Allowed dependencies: docs/architecture/layers.md (enforced by harness/checks/tests/arch.rs).
Ports and stacks per worktree: docs/architecture/port-allocation.md.

## Rules
- Logging: tracing only, constant messages, data in fields (docs/observability/fields.md).
- Time, randomness, HashMap: only via core::platform (DST depends on it; docs/testing/determinism.md).
- Environment and config only through runtime::config; user-facing CLI output only through cli::output.
- No unwrap, expect or panic outside tests. Exceptions: #[expect(lint, reason = "...")] on the smallest
  item, never #[allow]; then run `just docs` so docs/generated/lint-exceptions.txt records it.
- Errors are typed; CLI exit codes are fixed (docs/conventions/errors.md).
- Every bug fix adds a failing test first (unit, snapshot, DST seed, or fuzz regression).
- Never bypass git hooks (--no-verify, HK=0, HK_SKIP_*). If a hook is wrong, say so and stop.
- Ask before editing protected files: clippy.toml, deny.toml, [workspace.lints] in Cargo.toml,
  .config/nextest.toml, COV_MIN_REGIONS in mise.toml, docs/observability/budgets.md,
  docs/generated/lint-exceptions.txt, docs/generated/panic-allowlist.txt, harness/lints/,
  harness/checks/tests/arch.rs. Never lower a threshold or baseline to make a check pass.
- Perf claims need Gungraun numbers, not wall-clock alone (docs/performance/benchmarks.md).
- A new hot function goes in docs/performance/hot-paths.md (asm snapshot + benchmark).
- Files stay under 500 lines and main.rs under 80 (docs/conventions/modules.md).

## Where tests go
- Unit and property tests: `#[cfg(test)]` modules (proptest in core). API: insta snapshots in
  crates/{{project-name}}-server/tests/api.rs. CLI: trycmd transcripts in crates/{{project-name}}-cli/tests/cmd/.
- Integration test files start with `#![cfg(test)]`; every assert has a message.
- Async tests use `#[tokio::test(start_paused = true)]` with `TokioClock` and `SeededRng`.
- New failure modes in the domain: add a `buggify!` site and let `just dst` find the bugs.
- Strategy and the full table: docs/testing/strategy.md; simulation: docs/testing/dst.md.

## Hardening at a glance
Tiers and what gates: docs/hardening/tiers.md. Panic audit (pre-commit when core changes):
docs/hardening/panic-audit.md. Kani, fuzzing, sanitizers: docs/hardening/.

## Seeing what the app does
1. `just up`, reproduce, then `just logs-errors` (fast, local JSON log).
2. Need history or cross-request data: the Victoria MCP servers (victoriametrics, victorialogs,
   victoriatraces) or `just q-logs` / `just q-metrics` / `just q-traces`.
3. Budgets: `just budgets`. Canned queries: docs/observability/queries.md.
Query and aggregate; never dump raw logs into context. API error bodies carry a request_id:
pass it to `just logs-request`.

## When something fails
Read target/harness/<check>.json first: it has ok, summary, details_path and a repro command.
Lint, hook and test failures name the rule and the fix; follow it rather than silencing it.
DST failures print `just dst SEED=<n> TEST=<name>`; rerun exactly that.
Coverage misses are listed in target/harness/cov-missing.txt.
Assembly snapshot diffs need review: `cargo insta review`, with Gungraun numbers to justify them.

## Before you finish
Run `just check`. Agent Stop hooks run hk's lint steps on your changes and block until they pass.
Commit through the hooks; never push without being asked.

## Plans and docs
Multi-step work: create a Plan concept in docs/plans/ (`okf create plans/<slug> docs --type Plan
--status draft --tags plan,active ...`); when done, `okf update` it to status stable, tags plan,completed.
Knowledge: docs/ is an OKF bundle. Search before you write or ask: the okf-memory MCP server or
`okf search "..." docs`. Update an existing concept instead of adding a near-duplicate.
Record decisions in docs/decisions/ (type Decision). Never add a `verified` entry: only humans verify.
Template deviations and tool versions: docs/template/notes.md.
