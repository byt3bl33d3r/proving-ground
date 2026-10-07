---
type: Concept
title: Port allocation
description: How each git worktree gets its own range of local ports for the telemetry stack, derived from a hash of its path.
tags: [ports, worktree, harness, isolation]
status: stable
code_refs: [justfile, harness/stack/compose.yaml]
---

# Port allocation

Every git worktree (including agent worktrees under `.claude/worktrees/`) gets its own 10-port
range, its own Victoria stack (compose project `<project>-<dir>-<hash>`), its own server and its
own `.harness/` directory, so parallel agents never collide.

- The `justfile` hashes the worktree path (`cksum`) to a slot: base port `20000 + (crc % 1000) * 10`.
  VictoriaMetrics uses the base, VictoriaLogs base+1, VictoriaTraces base+2; the server binds
  port 0 and reports its URL in `.harness/app.json`.
- The ports are deterministic, so static MCP configs work: `just mcp <kind>` computes the same
  ports `just up` uses, with no registry and no setup step.
- Collisions are rare; `just up` refuses ports owned by another process and says how to pick a
  different base with `HARNESS_PORT=<base>` in `.env.local` (never committed).
- `eval "$(just env)"` gives a shell the same variables; `just harness-gc` removes stacks whose
  worktree is gone.

This replaces workz (see [dropping workz](/decisions/dropping-workz.md)). The services that bind
these ports are described in [telemetry](/observability/telemetry.md).
