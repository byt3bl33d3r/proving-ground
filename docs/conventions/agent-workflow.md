---
type: Guide
title: Agent workflow
description: The commands an agent runs, how to see what the app does, how to read a failing check, and what to do before finishing.
tags: [agents, workflow, commands, just, debugging]
status: stable
code_refs: [justfile, hk.pkl, .claude/settings.json, .codex/config.toml]
---

# Agent workflow

Run `just` recipes, not raw cargo: git hooks, CI and agents call the same recipes, so they never
disagree. Recall and record knowledge as described in [project memory](/conventions/project-memory.md).

## Commands

| Need | Run |
| --- | --- |
| Tier 0: fmt, clippy, tests, machete, docs check, justfile lint (also on commit) | `just check` |
| Tier 1: everything CI runs on a PR (pre-push runs `just ci-fast`) | `just ci` |
| Per-worktree stack and server; URLs in `.harness/app.json` | `just up`, `just down`, `just status` |
| End-to-end tests and latency budgets (needs `just up`) | `just e2e` |
| See the app | `just logs`, `just logs <request_id>`, `just query <backend> '<query>'`, `just trace <id>` |
| Deterministic simulation | `just dst` or `just dst SEEDS=1000` |
| Performance signals (never gate) | `just perf` |
| Fuzzing, sanitizers, full proofs, mutants | `just harden` |
| Regenerate `docs/generated/` after review | `just docs` |
| Validate project memory | `just knowledge` |

`just --list` shows the menu by group; hooks and CI also call private recipes, which each check's
`repro` names. What each tier runs and where: [check tiers](/hardening/tiers.md).
Each worktree gets its own ports and stack: [port allocation](/architecture/port-allocation.md).
Setup and the stack lifecycle: [setup and worktrees](/workflows/setup.md); what runs on commit and
push: [git hooks](/workflows/git-hooks.md); CI: [CI workflows](/workflows/ci.md).

## Seeing what the app does

1. `just up`, reproduce, then `just logs` (fast, the local JSON log).
2. Need history or cross-request data: the Victoria MCP servers (victoriametrics, victorialogs,
   victoriatraces) or `just query logs|metrics|traces '<query>'`.
3. Budgets: `just budgets`. Ready-made queries: [canned queries](/observability/queries.md).

Query and aggregate; never dump raw logs into context. API error bodies carry a `request_id`:
pass it to `just logs`.

## When something fails

Read `target/harness/<check>.json` first: it has `ok`, `summary`, `details_path` and a `repro`
command. Lint, hook and test failures name the rule and the fix; follow it rather than silencing it.

- DST failures print `just dst SEED=<n> TEST=<name>`; rerun exactly that ([DST](/testing/dst.md)).
- Coverage misses are listed in `target/harness/cov-missing.txt`.
- Assembly snapshot diffs need review with `cargo insta review`, backed by Gungraun numbers
  ([hot paths](/performance/hot-paths.md)).
- A new `#[expect]` needs `just docs` so the exception ledger records it ([lint policy](/conventions/lints.md)).

## Before you finish

Run `just check`. The agent Stop hook runs hk's lint steps on your changes and blocks until they
pass. Commit through the hooks, never with `--no-verify`, `HK=0` or `HK_SKIP_*`; if a hook is
wrong, say so and stop. Never push without being asked.
