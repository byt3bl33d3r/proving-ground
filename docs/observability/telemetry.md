---
type: Concept
title: Telemetry pipeline
description: How the binaries emit traces, logs and metrics: tracing subscriber layers, OTLP export to the Victoria stack, and the local JSON log.
tags: [observability, otel, victoria, logs, traces, metrics]
status: stable
code_refs: [crates/demo-app-runtime/src/telemetry.rs, harness/stack/compose.yaml]
---

# Telemetry pipeline

`runtime::telemetry::init` installs one `tracing` subscriber: an `EnvFilter` (`RUST_LOG`), a JSON
layer writing `.harness/logs/app.jsonl` when `APP_LOG_JSON` is set, a compact stderr layer, the
`tracing-opentelemetry` span layer and the log bridge. Metrics use an OTel `MeterProvider` with a
periodic exporter (`OTEL_METRIC_EXPORT_INTERVAL=5000` under `just up`).

Each signal goes over OTLP/HTTP straight to its backend; the endpoints come from
`OTEL_EXPORTER_OTLP_{TRACES,LOGS,METRICS}_ENDPOINT` (set by the `justfile`). With none set, export
is off and only the JSON file and stderr remain, so tests never need the stack. Export is best
effort: retries are off and an unreachable endpoint costs one warning.

| Signal | Backend | OTLP path | Query |
| --- | --- | --- | --- |
| Metrics | VictoriaMetrics | `/opentelemetry/v1/metrics` | PromQL, `just q-metrics` |
| Logs | VictoriaLogs | `/insert/opentelemetry/v1/logs` | LogsQL, `just q-logs` |
| Traces | VictoriaTraces | `/insert/opentelemetry/v1/traces` | LogsQL `just q-traces`, Jaeger `just trace` |

The UIs are at `/vmui/` (metrics) and `/select/vmui/` (logs, traces); `just ui` prints them. The
CLI logs to stderr (default level `warn`, `-v`/`-q` adjust it) and exports only with `--otel`.
Field names follow [fields](/observability/fields.md); ready-made queries are in
[queries](/observability/queries.md).
