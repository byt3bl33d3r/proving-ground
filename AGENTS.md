# {{project-name}}: agent map

{{description}}. Rust workspace: core + runtime libraries, axum server, clap CLI.
This file is a map, not a manual. Everything else (commands, conventions, testing, observability,
hardening, performance, decisions, plans) lives in docs/, the project's OKF knowledge bundle:
start at docs/index.md and recall the rest with okf.
Layout: crates/ (core, runtime, server, cli) and harness/ (checks, dst, fuzz, lints, stack);
details in docs/architecture/overview.md.

<!-- BEGIN OKF AGENT MEMORY -->
## Project memory (docs/)
- Recall before you act: search before changing code, adding a dependency or asking. Use the
  okf-memory MCP server (`okf_search`, then `okf_show` for the hits you need) or
  `okf search "<topic>" docs --scope project`. Before editing a file for the first time, search
  by its path (`for_path`, or `--for-path <path>` on the CLI).
- Write back what you learn: update the concept that covers it (`okf_update`); create one
  (`okf_create`) only when none does, and link it from a related concept.
- Multi-step work is a Plan in docs/plans/ (status draft, tag active; stable and completed when
  done). Decisions are Decision concepts in docs/decisions/.
- Never add a `verified` entry: only humans verify. `just knowledge` must pass.
- How-to and pitfalls: docs/conventions/project-memory.md.
<!-- END OKF AGENT MEMORY -->

## Commands
Run `just` recipes, not raw cargo. `just check` is tier 0 (also on commit), `just ci` is tier 1
(what CI runs), `just up` and `just down` start and stop this worktree's stack and server, and
`just --list` shows the rest. Commands, debugging and failures: docs/conventions/agent-workflow.md.
A failing check writes target/harness/<check>.json with a repro command: read it first.

## Rules
- Never bypass git hooks (--no-verify, HK=0, HK_SKIP_*). If a hook is wrong, say so and stop.
- Ask before editing protected files: clippy.toml, deny.toml, [workspace.lints] in Cargo.toml,
  .config/nextest.toml, COV_MIN_REGIONS in mise.toml, docs/observability/budgets.md,
  docs/generated/lint-exceptions.txt, docs/generated/panic-allowlist.txt, harness/lints/,
  harness/checks/tests/arch.rs. Never lower a threshold or baseline to make a check pass.
- Every bug fix adds a failing test first.
- Lints, hooks and tests enforce the code rules (tracing with constant messages, time and
  randomness through core::platform, no unwrap, typed errors, file size). Their messages name the
  rule and the fix; follow them rather than silencing them.

## Before you finish
Run `just check`, then commit through the hooks. Never push without being asked.
