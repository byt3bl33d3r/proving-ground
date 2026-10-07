---
name: harness
description: Reproduce, observe, fix and verify a bug or behaviour change in this service using the per-worktree harness (just up, logs, traces, metrics, budgets, DST). Use when debugging the server or CLI, investigating an error or latency, or proving a fix.
---

# Reproduce, observe, fix, verify

1. **Reproduce.** `just up` (prints the app URL). Trigger the behaviour with the CLI
   (`cargo run -p {{project-name}}-cli -- items ...`) or a failing test. Error bodies carry a `request_id`.
2. **Observe** (aggregate, never dump):
   - `just logs-errors`: recent WARN/ERROR from the local JSON log.
   - `just logs-request <request_id>`: everything one request logged.
   - `just q-logs '<LogsQL>'`, `just q-traces '<LogsQL>'`, `just trace <trace_id>`,
     `just q-metrics '<PromQL>'`, or the Victoria MCP servers. Canned queries:
     docs/observability/queries.md.
3. **Write the failing test first**: unit/snapshot test, a trycmd transcript, an e2e test in
   harness/checks/tests/e2e.rs, a DST seed (`just dst SEED=<n> TEST=<name>`) or a fuzz regression.
4. **Fix** within the rules in AGENTS.md (core::platform for time and randomness, typed errors,
   tracing with constant messages).
5. **Verify**: `just check`, then `just e2e` (tests plus budgets) and `just dst`. For performance
   changes, Gungraun numbers (`just gungraun`) and `just perf`. Finish with `just down` if you
   started the stack.
