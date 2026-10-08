---
type: Guide
title: Canned queries
description: Ready-to-run LogsQL and PromQL queries for this service's logs, traces and metrics, with the real field names.
tags: [observability, queries, logsql, promql]
status: stable
code_refs: [justfile]
---

# Canned queries

Run them with `just query logs|traces|metrics '<query>'`, or through the Victoria MCP
servers. Every recipe caps its output (200 lines for logs and traces); aggregate with
`| stats ...` instead of paging through raw rows. Field names below were read from each
backend's `field_names` endpoint; they are listed in [fields](/observability/fields.md).

## Logs (VictoriaLogs, LogsQL)

```text
# Errors in the last 15 minutes for this service
_time:15m service.name:="{{service_name}}" severity_text:=ERROR

# Same, summarized by message instead of rows
_time:15m service.name:="{{service_name}}" severity_text:in(ERROR,WARN) | stats by (severity_text, _msg) count() hits | sort by (hits desc) | limit 20

# Everything one request logged (request_id from an error body)
_time:1h request_id:="<request_id>"

# Everything in one trace
_time:1h trace_id:="<trace_id>"
```

Fields: `_msg` (message), `_time`, `severity_text` (`ERROR`, `WARN`, `INFO`, ...),
`severity_number` (17 = ERROR), `trace_id`, `span_id`, resource attributes as plain fields
(`service.name`, `service.version`, `deployment.environment.name`, `worktree`), and event fields
such as `item_id` and `error.type`.

## Traces (VictoriaTraces, LogsQL)

```text
# Startup span over the 800 ms budget (duration is in nanoseconds; units convert)
"resource_attr:service.name":="{{service_name}}" name:="startup" duration:>800ms

# Slowest requests in the last 15 minutes
_time:15m name:="http_request" | sort by (duration desc) | limit 10 | fields trace_id, span_attr:http.route, duration

# Failed spans
_time:15m status_code:=2
```

Use `name:="x"` (exact) rather than `name:"x"` (substring). Span attributes are
`span_attr:<key>`, resource attributes `resource_attr:<key>`. For a whole trace use
`just trace <trace_id>` (Jaeger API, durations in milliseconds).

## Metrics (VictoriaMetrics, PromQL/MetricsQL)

```text
# p95 latency by route
histogram_quantile(0.95, sum by (le, http_route) (rate(http_server_request_duration_seconds_bucket{service_name="{{service_name}}"}[5m])))

# Request rate by route and status
sum by (http_route, http_response_status_code) (rate(http_server_request_duration_seconds_count[5m]))

# Items created
items_created_total
```

OTLP names become Prometheus names (`-opentelemetry.usePrometheusNaming`):
`http.server.request.duration` (unit `s`) is `http_server_request_duration_seconds_*`, `items.created`
is `items_created_total`, and attribute dots become underscores (`http_route`, `service_name`).
The [budgets](/observability/budgets.md) reuse these queries.
