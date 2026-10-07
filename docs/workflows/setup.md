---
type: Guide
title: Setup and worktrees
description: First-time setup (mise, bootstrap), the per-worktree stack and server lifecycle, adding and removing worktrees, and the shared build cache.
tags: [workflow, setup, bootstrap, worktrees, stack, mise]
status: stable
code_refs: [mise.toml, rust-toolchain.toml, justfile, harness/stack/compose.yaml, .claude/settings.json]
---

# Setup and worktrees

## First time

1. Install mise and Docker (macOS or Linux). Git hooks, agent hooks and agent MCP servers start
   their tools with `mise x --`, so `mise` must be on the PATH that git and the agent apps see.
2. `mise trust && mise install`: every tool is pinned in `mise.toml`, and a preinstall hook
   installs the toolchain from `rust-toolchain.toml` first. The analysis nightly (`NIGHTLY`)
   installs itself the first time a recipe needs it.
3. `just bootstrap`: checks the tools, warns when Docker is missing (only `just up` and
   `just e2e` need it), generates the lockfiles of the main, `harness/dst` and `harness/fuzz`
   workspaces when missing, installs the git hooks and builds everything once.
4. `just up`, then `just check`.

## The stack and server (one per worktree)

| Recipe | Does |
| --- | --- |
| `just up` | Refuses ports owned by another process, starts this worktree's Victoria stack and waits for it, writes `.harness/stack.json` and `.harness/env`, builds and starts the server in the background, waits for `/readyz`, then prints the app URL, UI URLs and ports as JSON |
| `just status` | JSON: stack health, server pid and readiness, ports |
| `just restart` | Rebuilds and restarts only the server; the stack and its data stay |
| `just down` | Stops the server, removes the stack and its volumes, deletes `.harness/` |
| `just env` | The harness variables for a shell: `eval "$(just env)"` |
| `just ui` | The Victoria web UIs, for humans |
| `just mcp <kind>` | A Victoria MCP server for this worktree (the agents' MCP configs call it) |
| `just harness-gc` | Removes stacks whose worktree directory no longer exists |

Server output goes to `.harness/logs/server.out` and the app's JSON log to
`.harness/logs/app.jsonl`. The stack itself is described in [telemetry](/observability/telemetry.md).

## Worktrees

- Add one with `git worktree add ../<dir>` or `claude --worktree`. The git hooks are shared, so
  there is nothing to reinstall: run `mise trust` there, then `just up`. It gets its own ports and
  stack ([port allocation](/architecture/port-allocation.md)).
- Each worktree keeps its own `target/`, so parallel builds never wait on Cargo's lock;
  `RUSTC_WRAPPER=sccache` (set in `mise.toml`) still shares compiled dependencies.
- Removing a Claude Code worktree runs its WorktreeRemove hook, which runs `just down` there
  first. Before a plain `git worktree remove`, run `just down` in that worktree; `just harness-gc`
  cleans up any stack left behind.

What the hooks run is in [git hooks](/workflows/git-hooks.md).
