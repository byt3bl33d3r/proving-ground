---
okf_version: "0.2"
---

# Knowledge Base

Project memory for this repository, an OKF v0.2 bundle. Search before you write or ask:
`okf search "..." docs` or the okf-memory MCP server. Validate with `just knowledge`.
Agents: [agent workflow](conventions/agent-workflow.md) for commands and failures,
[project memory](conventions/project-memory.md) for recalling and recording knowledge.

# Sections

* [Architecture](architecture/index.md) - Crate layout, layering rules and per-worktree port allocation.
* [Observability](observability/index.md) - Telemetry pipeline, field naming, canned queries and budgets.
* [Testing](testing/index.md) - Test kinds, determinism rules and deterministic simulation.
* [Hardening](hardening/index.md) - Check tiers, panic audit, Kani, Miri, fuzzing and sanitizers.
* [Performance](performance/index.md) - Hot paths and benchmarks; Gungraun gates, Criterion informs.
* [Conventions](conventions/index.md) - How agents work here (commands, project memory), errors and exit codes, module size, lint policy.
* [Workflows](workflows/index.md) - Setup and worktrees, git hooks, CI workflows, and upgrading from the template.
* [Decisions](decisions/index.md) - Architecture decision records.
* [Plans](plans/index.md) - Multi-step work: active plans are status draft with tag active; finished ones are stable with tag completed.
* [Template](template/index.md) - Template deviations and verified tool versions.

# Machine output (not concepts)

* `generated/` holds the crate graph, the lint-exception ledger and the panic allowlist; okf ignores non-Markdown files.
