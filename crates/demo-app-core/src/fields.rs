//! Telemetry field names. OpenTelemetry semantic-convention names where one exists,
//! `snake_case` domain names otherwise. See `docs/observability/fields.md`.

/// HTTP method (semconv).
pub const HTTP_REQUEST_METHOD: &str = "http.request.method";
/// Matched route template, e.g. `/items/{id}` (semconv).
pub const HTTP_ROUTE: &str = "http.route";
/// Response status code (semconv).
pub const HTTP_RESPONSE_STATUS_CODE: &str = "http.response.status_code";
/// Error class, e.g. `not_found` (semconv).
pub const ERROR_TYPE: &str = "error.type";
/// Rendered error message on a failed span.
pub const ERROR: &str = "error";
/// Span status override read by `tracing-opentelemetry`.
pub const OTEL_STATUS_CODE: &str = "otel.status_code";
/// Item identifier (`itm_` + 16 hex digits).
pub const ITEM_ID: &str = "item_id";
/// Request identifier (`x-request-id`, UUID v7).
pub const REQUEST_ID: &str = "request_id";
/// Seed of a deterministic simulation run.
pub const DST_SEED: &str = "dst_seed";

/// Metric: server request duration histogram, seconds (semconv).
pub const METRIC_HTTP_SERVER_REQUEST_DURATION: &str = "http.server.request.duration";
/// Metric: items created (counter).
pub const METRIC_ITEMS_CREATED: &str = "items.created";
/// Metric: items deleted (counter).
pub const METRIC_ITEMS_DELETED: &str = "items.deleted";
