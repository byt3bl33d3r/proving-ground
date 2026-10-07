//! Middleware pieces used by [`crate::router`].

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{MatchedPath, Request};
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderValue, Response, StatusCode};
use axum::middleware::Next;
use demo_app_core::fields;
use demo_app_core::platform::{Clock, Rng};
use opentelemetry::global;
use opentelemetry::propagation::Extractor;
use opentelemetry::trace::TraceContextExt as _;
use serde_json::json;
use tower_http::request_id::{MakeRequestId, RequestId};
use tracing::Span;
use tracing::field::Empty;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;

use crate::AppState;
use crate::error::ErrorDetails;

const X_REQUEST_ID: &str = "x-request-id";

/// Request ids are UUID v7 built from the platform clock and RNG, so simulation runs repeat
/// them exactly. An incoming `x-request-id` header is kept as is.
#[derive(Clone, Debug)]
pub(crate) struct MakeRequestIdV7 {
    clock: Arc<dyn Clock>,
    rng: Arc<dyn Rng>,
}

impl MakeRequestIdV7 {
    pub(crate) fn new(state: &AppState) -> Self {
        Self {
            clock: state.clock(),
            rng: state.rng(),
        }
    }
}

impl MakeRequestId for MakeRequestIdV7 {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let mut random = [0_u8; 10];
        let (high, low) = random.split_at_mut(8);
        high.copy_from_slice(&self.rng.next_u64().to_le_bytes());
        low.copy_from_slice(self.rng.next_u64().to_le_bytes().get(..2)?);
        let id = uuid::Builder::from_unix_timestamp_millis(self.clock.now().as_millis(), &random);
        HeaderValue::from_str(&id.into_uuid().to_string())
            .ok()
            .map(RequestId::new)
    }
}

/// The `http_request` span: method, matched route and request id.
pub(crate) fn make_span(request: &Request<Body>) -> Span {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str);
    let request_id = request
        .headers()
        .get(X_REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let span = tracing::info_span!(
        "http_request",
        http.request.method = %request.method(),
        http.route = route,
        request_id = request_id,
        http.response.status_code = Empty,
        otel.kind = "server",
        otel.status_code = Empty,
    );
    let parent = global::get_text_map_propagator(|propagator| {
        propagator.extract(&Headers(request.headers()))
    });
    if parent.span().span_context().is_valid()
        && let Err(e) = span.set_parent(parent)
    {
        tracing::debug!(error = %e, "trace_parent_not_linked");
    }
    span
}

/// Reads W3C trace-context headers for the propagator.
struct Headers<'a>(&'a axum::http::HeaderMap);

impl Extractor for Headers<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(axum::http::HeaderName::as_str).collect()
    }
}

/// Records the response status on the `http_request` span.
pub(crate) fn on_response(response: &Response<Body>, _latency: Duration, span: &Span) {
    let status = response.status();
    span.record(fields::HTTP_RESPONSE_STATUS_CODE, status.as_u16());
    if status.is_server_error() {
        span.record(fields::OTEL_STATUS_CODE, "ERROR");
    }
}

/// Renders every 4xx/5xx response as `{ "error": { "code", "message", "request_id" } }`.
pub(crate) async fn json_errors(request: Request, next: Next) -> Response<Body> {
    let request_id = request
        .headers()
        .get(X_REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let response = next.run(request).await;
    let status = response.status();
    if !status.is_client_error() && !status.is_server_error() {
        return response;
    }
    let (mut parts, _discarded) = response.into_parts();
    let details = parts
        .extensions
        .remove::<ErrorDetails>()
        .unwrap_or_else(|| generic(status));
    let body = json!({
        "error": { "code": details.code, "message": details.message, "request_id": request_id }
    });
    parts.headers.remove(CONTENT_LENGTH);
    parts
        .headers
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Response::from_parts(parts, Body::from(body.to_string()))
}

fn generic(status: StatusCode) -> ErrorDetails {
    let code = match status {
        StatusCode::REQUEST_TIMEOUT => "timeout",
        StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
        StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported_media_type",
        _ if status.is_server_error() => "internal",
        _ => "http_error",
    };
    ErrorDetails {
        code,
        message: status.canonical_reason().unwrap_or("error").to_owned(),
    }
}

/// A handler panicked: log it and answer 500 with a JSON body.
pub(crate) fn panic_response(panic: Box<dyn Any + Send + 'static>) -> Response<Body> {
    let message = match panic.downcast::<String>() {
        Ok(text) => *text,
        Err(other) => other
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .unwrap_or_default(),
    };
    tracing::error!(panic.message = %message, "handler_panicked");
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    response.extensions_mut().insert(ErrorDetails {
        code: "internal",
        message: "internal error".to_owned(),
    });
    response
}
