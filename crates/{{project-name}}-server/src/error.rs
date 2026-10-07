//! API errors. Every error response body is
//! `{ "error": { "code", "message", "request_id" } }`; `request_id` is filled in by the
//! `json_errors` middleware so handlers never need it.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use {{crate_name}}_core::types::ItemError;

/// Error code and message carried in response extensions until `json_errors` renders them.
#[derive(Clone, Debug)]
pub(crate) struct ErrorDetails {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

/// An error returned by a handler.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    details: ErrorDetails,
}

impl ApiError {
    /// An error with an explicit status, stable code and message.
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            details: ErrorDetails {
                code,
                message: message.into(),
            },
        }
    }

    pub(crate) fn from_json(rejection: &JsonRejection) -> Self {
        Self::new(rejection.status(), "invalid_body", rejection.body_text())
    }

    pub(crate) fn from_query(rejection: &QueryRejection) -> Self {
        Self::new(rejection.status(), "invalid_query", rejection.body_text())
    }
}

impl From<ItemError> for ApiError {
    fn from(err: ItemError) -> Self {
        let status = match &err {
            ItemError::Validation(_) => StatusCode::BAD_REQUEST,
            ItemError::NotFound(_) => StatusCode::NOT_FOUND,
            ItemError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        };
        Self::new(status, err.code(), err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = self.status.into_response();
        response.extensions_mut().insert(self.details);
        response
    }
}
