---
type: Guide
title: Budgets
description: The latency and error budgets that just budgets checks after just e2e, each a query plus a threshold.
tags: [observability, budgets, performance]
status: stable
code_refs: [justfile]
---

# Budgets

`just budgets` (run at the end of `just e2e`) executes every line of the block below against
this worktree's Victoria stack and writes `target/harness/budgets.json`. Each line is one
budget: `backend` is `logs`, `traces` or `metrics`; the query must return a single number
(`value` for LogsQL, one series for PromQL); the budget passes when `value <= max`.
`$SERVICE`, `$START` and `$END` are replaced with the service name and the e2e time window.
A budget with no data fails: missing telemetry is a bug too.

```jsonl budgets
{"name": "startup_under_800ms", "backend": "traces", "unit": "ns", "max": 800000000, "query": "_time:1h \"resource_attr:service.name\":=\"$SERVICE\" name:=\"startup\" | stats max(duration) as value"}
{"name": "no_error_logs_during_e2e", "backend": "logs", "unit": "lines", "max": 0, "query": "_time:[$START, $END] service.name:=\"$SERVICE\" severity_text:=ERROR | stats count() as value"}
{"name": "items_p95_under_50ms", "backend": "metrics", "unit": "s", "max": 0.05, "query": "histogram_quantile(0.95, sum by (le) (last_over_time(http_server_request_duration_seconds_bucket{service_name=\"$SERVICE\", http_route=~\"/items.*\"}[15m])))"}
```

Thresholds are protected: lower them freely, but raising one needs a human's approval
(see the Rules in AGENTS.md). Add a budget by adding a line; the canned queries in
[queries](/observability/queries.md) are a good starting point.
