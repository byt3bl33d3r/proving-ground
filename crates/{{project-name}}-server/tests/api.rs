//! API snapshot tests: the router through `oneshot`, no socket. Ids, timestamps and request
//! ids are redacted; review changes with `cargo insta review`.
#![cfg(test)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use {{crate_name}}_core::platform::{SeededRng, TokioClock};
use {{crate_name}}_core::types::Timestamp;
use {{crate_name}}_server::{AppState, router};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

fn app() -> Router {
    let clock = Arc::new(TokioClock::new(Timestamp::from_millis(1_700_000_000_000)));
    let state = AppState::new(clock, Arc::new(SeededRng::new(1)), Duration::from_secs(5));
    state.mark_ready();
    router(state)
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<&str>) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |text| Body::from(text.to_owned())))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json")
    };
    (status, json)
}

macro_rules! snapshot {
    ($name:expr, $value:expr) => {
        insta::assert_json_snapshot!($name, $value, {
            ".id" => "[id]",
            ".created_at_ms" => "[timestamp]",
            ".items[].id" => "[id]",
            ".items[].created_at_ms" => "[timestamp]",
            ".error.request_id" => "[request_id]",
        })
    };
}

#[tokio::test(start_paused = true)]
async fn item_lifecycle() {
    let app = app();
    let (created_status, created) =
        call(&app, Method::POST, "/items", Some(r#"{"name":"pen"}"#)).await;
    assert_eq!(created_status, StatusCode::CREATED, "create status");
    snapshot!("create", &created);
    let id = created["id"].as_str().expect("id");
    let (fetched_status, fetched) = call(&app, Method::GET, &format!("/items/{id}"), None).await;
    assert_eq!(
        (fetched_status, &fetched),
        (StatusCode::OK, &created),
        "get returns the created item"
    );
    let (list_status, page) = call(&app, Method::GET, "/items?limit=10", None).await;
    assert_eq!(list_status, StatusCode::OK, "list status");
    snapshot!("list", &page);
    let (deleted_status, _) = call(&app, Method::DELETE, &format!("/items/{id}"), None).await;
    assert_eq!(deleted_status, StatusCode::NO_CONTENT, "delete status");
    let (missing_status, missing) = call(&app, Method::GET, &format!("/items/{id}"), None).await;
    assert_eq!(missing_status, StatusCode::NOT_FOUND, "gone after delete");
    insta::assert_json_snapshot!("not_found", &missing, {
        ".error.request_id" => "[request_id]",
        ".error.message" => "[message with id]",
    });
}

#[tokio::test(start_paused = true)]
async fn errors_are_json_with_request_id() {
    let app = app();
    let cases = [
        (
            "empty_name",
            Method::POST,
            "/items",
            Some(r#"{"name":"  "}"#),
        ),
        ("bad_body", Method::POST, "/items", Some(r#"{"nom":"pen"}"#)),
        ("bad_id", Method::GET, "/items/nope", None),
        ("bad_limit", Method::GET, "/items?limit=0", None),
        ("no_route", Method::GET, "/nowhere", None),
        ("wrong_method", Method::PUT, "/items", None),
    ];
    for (name, method, uri, body) in cases {
        let (status, json) = call(&app, method, uri, body).await;
        assert!(
            status.is_client_error(),
            "{name}: expected 4xx, got {status}"
        );
        assert!(
            json["error"]["request_id"].is_string(),
            "{name}: request_id present"
        );
        snapshot!(name, &json);
    }
}

#[tokio::test(start_paused = true)]
async fn readyz_follows_the_ready_flag() {
    let clock = Arc::new(TokioClock::new(Timestamp::from_millis(0)));
    let state = AppState::new(clock, Arc::new(SeededRng::new(1)), Duration::from_secs(5));
    let app = router(state.clone());
    let (before, _) = call(&app, Method::GET, "/readyz", None).await;
    state.mark_ready();
    let (after, _) = call(&app, Method::GET, "/readyz", None).await;
    assert_eq!(
        (before, after),
        (StatusCode::SERVICE_UNAVAILABLE, StatusCode::OK),
        "readyz"
    );
}
