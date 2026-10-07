//! Typed HTTP client for the server API (reqwest with rustls).

use std::time::Duration;

use demo_app_core::types::{CreateItem, Item, ItemId, ItemPage};
use opentelemetry::global;
use opentelemetry::propagation::Injector;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;

/// Request timeout for every call.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// The `error` object of an API error body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    /// Stable error code.
    pub code: String,
    /// Explanation.
    pub message: String,
    /// Server request id.
    pub request_id: Option<String>,
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ApiErrorBody,
}

/// A failed API call.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// No answer from the server.
    #[error("cannot reach {url}: {reason}")]
    Unreachable {
        /// Requested URL.
        url: String,
        /// Transport error.
        reason: String,
    },
    /// The server answered with an error status.
    #[error("HTTP {status}: {} ({})", body.message, body.code)]
    Api {
        /// HTTP status.
        status: u16,
        /// Decoded error body.
        body: ApiErrorBody,
    },
    /// The server answered with a body that does not decode.
    #[error("unexpected response from {url}: {reason}")]
    Decode {
        /// Requested URL.
        url: String,
        /// Decoding error.
        reason: String,
    },
}

/// API client bound to one server.
#[derive(Clone, Debug)]
pub struct Client {
    http: reqwest::Client,
    base: String,
}

impl Client {
    /// A client for the server at `base_url` (for example `http://127.0.0.1:20017`).
    pub fn new(base_url: &str) -> Result<Self, ClientError> {
        let http = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|e| ClientError::Unreachable {
                url: base_url.to_owned(),
                reason: e.to_string(),
            })?;
        Ok(Self {
            http,
            base: base_url.trim_end_matches('/').to_owned(),
        })
    }

    /// The server base URL.
    pub fn base_url(&self) -> &str {
        &self.base
    }

    /// `POST /items`
    pub async fn create_item(&self, name: &str) -> Result<Item, ClientError> {
        let body = CreateItem {
            name: name.to_owned(),
        };
        self.json(self.http.post(self.url("/items")).json(&body))
            .await
    }

    /// `GET /items/{id}`
    pub async fn get_item(&self, id: ItemId) -> Result<Item, ClientError> {
        self.json(self.http.get(self.url(&format!("/items/{id}"))))
            .await
    }

    /// `GET /items?offset=&limit=`
    pub async fn list_items(
        &self,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> Result<ItemPage, ClientError> {
        let query: Vec<(&str, u32)> = [("offset", offset), ("limit", limit)]
            .into_iter()
            .filter_map(|(k, v)| Some((k, v?)))
            .collect();
        self.json(self.http.get(self.url("/items")).query(&query))
            .await
    }

    /// `DELETE /items/{id}`
    pub async fn delete_item(&self, id: ItemId) -> Result<(), ClientError> {
        self.send(self.http.delete(self.url(&format!("/items/{id}"))))
            .await
            .map(drop)
    }

    /// `GET /readyz`: `Ok(true)` when ready, `Ok(false)` while starting.
    pub async fn ready(&self) -> Result<bool, ClientError> {
        let url = self.url("/readyz");
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| unreachable(&url, &e))?;
        Ok(response.status() == StatusCode::OK)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    async fn json<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T, ClientError> {
        let response = self.send(request).await?;
        let url = response.url().to_string();
        response.json().await.map_err(|e| ClientError::Decode {
            url,
            reason: e.to_string(),
        })
    }

    async fn send(&self, request: RequestBuilder) -> Result<Response, ClientError> {
        let response = request
            .headers(trace_headers())
            .send()
            .await
            .map_err(|e| unreachable(&self.base, &e))?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let url = response.url().to_string();
        let envelope: ErrorEnvelope = response.json().await.map_err(|e| ClientError::Decode {
            url,
            reason: e.to_string(),
        })?;
        Err(ClientError::Api {
            status: status.as_u16(),
            body: envelope.error,
        })
    }
}

/// W3C `traceparent` for the current span, so the server's spans join the CLI's trace when the
/// CLI runs with `--otel`. Empty when no propagator is installed.
fn trace_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    let context = Span::current().context();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut Headers(&mut headers));
    });
    headers
}

struct Headers<'a>(&'a mut HeaderMap);

impl Injector for Headers<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_str(&value),
        ) {
            self.0.insert(name, value);
        }
    }
}

fn unreachable(url: &str, err: &reqwest::Error) -> ClientError {
    let reason = if err.is_connect() {
        "connection refused".to_owned()
    } else {
        err.to_string()
    };
    ClientError::Unreachable {
        url: url.to_owned(),
        reason,
    }
}
