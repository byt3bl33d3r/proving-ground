# Observability

Telemetry pipeline, field naming, canned queries and budgets.

* [Budgets](budgets.md) - The latency and error budgets that just budgets checks after just e2e, each a query plus a threshold.
* [Telemetry fields and naming](fields.md) - Telemetry naming rules: OpenTelemetry semantic-convention names, snake_case domain fields, verb_noun span names, constant messages.
* [Canned queries](queries.md) - Ready-to-run LogsQL and PromQL queries for this service's logs, traces and metrics, with the real field names.
* [Telemetry pipeline](telemetry.md) - How the binaries emit traces, logs and metrics: tracing subscriber layers, OTLP export to the Victoria stack, and the local JSON log.
