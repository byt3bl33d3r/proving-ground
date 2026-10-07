//! A minimal HTTP client over turmoil's network (reqwest cannot run inside turmoil).

use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt as _, Full};
use hyper::{Request, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::Value;

/// Per-request deadline: past it the outcome is unknown (the request may still apply).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

/// What one request produced.
#[derive(Debug)]
pub enum Reply {
    /// The server answered.
    Answer(StatusCode, Value),
    /// No answer: connection failed or the deadline passed. The effect is unknown.
    Unknown,
}

/// Sends one request to `host:8080` on a fresh connection.
pub async fn send(host: &str, method: &str, path: &str, body: Option<Value>) -> Reply {
    match tokio::time::timeout(REQUEST_TIMEOUT, exchange(host, method, path, body)).await {
        Ok(Ok((status, json))) => Reply::Answer(status, json),
        Ok(Err(_)) | Err(_) => Reply::Unknown,
    }
}

async fn exchange(
    host: &str,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
    let stream = turmoil::net::TcpStream::connect((host, 8080)).await?;
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(stream)).await?;
    tokio::spawn(connection);
    let bytes = body.map(|json| json.to_string()).unwrap_or_default();
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", host)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(bytes)))?;
    let response = sender.send_request(request).await?;
    let status = response.status();
    let collected = response.into_body().collect().await?.to_bytes();
    let json = serde_json::from_slice(&collected).unwrap_or(Value::Null);
    Ok((status, json))
}
