//! HTTP server metrics: semconv `http.server.request.duration` (histogram, seconds).

use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use demo_app_core::fields;
use opentelemetry::metrics::Histogram;
use opentelemetry::{KeyValue, global};

use crate::AppState;

/// Bucket boundaries in seconds (OpenTelemetry semantic-convention advice). The SDK default
/// buckets assume milliseconds, which would make every p95 query meaningless.
const DURATION_BUCKETS: [f64; 14] = [
    0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
];

/// Instruments created from the global meter provider (set by telemetry init, so build the
/// state after initializing telemetry).
#[derive(Clone)]
pub(crate) struct HttpMetrics {
    duration: Histogram<f64>,
}

impl std::fmt::Debug for HttpMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpMetrics").finish_non_exhaustive()
    }
}

impl HttpMetrics {
    pub(crate) fn new() -> Self {
        let duration = global::meter(env!("CARGO_PKG_NAME"))
            .f64_histogram(fields::METRIC_HTTP_SERVER_REQUEST_DURATION)
            .with_unit("s")
            .with_description("Duration of HTTP server requests")
            .with_boundaries(DURATION_BUCKETS.to_vec())
            .build();
        Self { duration }
    }
}

/// Records one histogram sample per request, labelled with method, route and status.
pub(crate) async fn record_duration(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let clock = state.clock();
    let start = clock.monotonic();
    let method = request.method().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str)
        .to_owned();
    let response = next.run(request).await;
    let seconds = clock.monotonic().saturating_sub(start).as_secs_f64();
    let attributes = [
        KeyValue::new(fields::HTTP_REQUEST_METHOD, method),
        KeyValue::new(fields::HTTP_ROUTE, route),
        KeyValue::new(
            fields::HTTP_RESPONSE_STATUS_CODE,
            i64::from(response.status().as_u16()),
        ),
    ];
    state.http_metrics().duration.record(seconds, &attributes);
    response
}
