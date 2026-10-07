---
type: Decision
title: Dropping workz for hashed port ranges
description: Per-worktree port ranges come from a hash of the worktree path instead of workz.
tags: [decision, ports, workz, worktree]
status: stable
code_refs: [justfile]
---

# Dropping workz for hashed port ranges

**Context.** The template spec chose workz to give each git worktree a port range. workz 0.11.0
(latest release, October 2026) ignores `base_port`, refuses the main checkout, and keys
allocations by branch name across all repositories, so two repos on the same branch, or any two
detached-HEAD worktrees, share ports. The fixes exist only on unreleased upstream main.

**Decision (human, 2026-10-07).** Use the spec's fallback: the `justfile` hashes the worktree
path to a port range. No registry, no setup hook, no `.workz.toml`.

**Consequences.** Ports are deterministic per path, so MCP configs stay static and new
worktrees need no setup. A hash collision is possible but rare; `just up` detects it and
`HARNESS_PORT` overrides it. Details: [port allocation](/architecture/port-allocation.md).
