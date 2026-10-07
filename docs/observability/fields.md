---
type: Guide
title: Telemetry fields and naming
description: Telemetry naming rules: OpenTelemetry semantic-convention names, snake_case domain fields, verb_noun span names, constant messages.
tags: [observability, fields, naming, tracing, lint]
status: stable
code_refs: [crates/{{project-name}}-core/src/fields.rs, harness/lints/src/field_style.rs]
---

# Telemetry fields and naming

## Naming

- Use OpenTelemetry semantic-convention names where one exists: `http.request.method`,
  `http.route`, `http.response.status_code`, `error.type`.
- Domain fields are `snake_case`: `item_id`, `request_id`, `dst_seed`.
- Span names are `verb_noun`: `startup`, `load_config`, `create_item`, `http_request`.
- Failures record `error = %e` and set `otel.status_code = "ERROR"` on the span.
- Messages are constant strings and variable data goes in fields:
  `info!(item_id = %id, "item_created")`, never `info!("created item {id}")`.
- Metrics follow semconv: `http.server.request.duration` (histogram, seconds) and counters such
  as `items.created`. VictoriaMetrics shows them as `http_server_request_duration_seconds_*`
  and `items_created_total`.

The constants live in `core::fields`. Two dylint lints enforce the rules:
`tracing_message_interpolation` (format arguments in a message) and `telemetry_field_style`
(keys that are not `snake_case` or lowercase dotted). Run them with `just dylint`. How the
fields reach the backends is in the [telemetry pipeline](/observability/telemetry.md).
