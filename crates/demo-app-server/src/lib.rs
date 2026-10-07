//! HTTP service library: [`router`], [`AppState`], middleware and handlers. `main.rs` only
//! wires configuration, telemetry and the listener, so tests and `harness/dst` reuse the
//! router without a socket.

mod error;
mod handlers;
mod middleware;
mod state;

use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, post};
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub use error::ApiError;
pub use state::AppState;

/// Default per-request timeout.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Builds the application router. Middleware, outermost first: request id (UUID v7),
/// `http_request` trace span, request-id propagation, JSON error bodies, timeout, panic catcher.
pub fn router(state: AppState) -> Router {
    let request_ids = middleware::MakeRequestIdV7::new(&state);
    let layers = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(request_ids))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(middleware::make_span)
                .on_response(middleware::on_response),
        )
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(axum::middleware::from_fn(middleware::json_errors))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            state.request_timeout(),
        ))
        .layer(CatchPanicLayer::custom(middleware::panic_response));
    Router::new()
        .route(
            "/items",
            post(handlers::create_item).get(handlers::list_items),
        )
        .route(
            "/items/{id}",
            get(handlers::get_item).delete(handlers::delete_item),
        )
        .route("/readyz", get(handlers::readyz))
        .fallback(handlers::no_route)
        .layer(layers)
        .with_state(state)
}
