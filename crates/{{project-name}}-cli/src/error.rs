//! CLI errors and the single error-to-exit-code mapping (see `docs/conventions/errors.md`).

use std::process::ExitCode;

use crate::client::ClientError;

/// Why a command failed.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// Bad arguments or input the server rejected (4xx other than 404).
    #[error("{message}")]
    Usage {
        /// Stable error code.
        code: String,
        /// Explanation.
        message: String,
        /// Server request id, when the server answered.
        request_id: Option<String>,
    },
    /// The item does not exist (404).
    #[error("{message}")]
    NotFound {
        /// Explanation.
        message: String,
        /// Server request id.
        request_id: Option<String>,
    },
    /// The server failed (5xx) or answered something unexpected.
    #[error("{message}")]
    Server {
        /// Stable error code.
        code: String,
        /// Explanation.
        message: String,
        /// Server request id, when the server answered.
        request_id: Option<String>,
    },
    /// No server answered.
    #[error("{message}")]
    Unreachable {
        /// Explanation, including the fix.
        message: String,
    },
}

impl CliError {
    /// A usage error raised before contacting the server.
    pub fn usage(code: &str, message: impl Into<String>) -> Self {
        Self::Usage {
            code: code.to_owned(),
            message: message.into(),
            request_id: None,
        }
    }

    /// The exit code for this error: 1 usage, 2 not found, 3 server error, 4 unreachable.
    /// (0 is success.) This is the only place exit codes are decided.
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage { .. } => 1,
            Self::NotFound { .. } => 2,
            Self::Server { .. } => 3,
            Self::Unreachable { .. } => 4,
        }
    }

    /// Stable machine-readable code.
    pub fn code(&self) -> &str {
        match self {
            Self::Usage { code, .. } | Self::Server { code, .. } => code,
            Self::NotFound { .. } => "not_found",
            Self::Unreachable { .. } => "unreachable",
        }
    }

    /// The server's request id, for `just logs-request <id>`.
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::Usage { request_id, .. }
            | Self::NotFound { request_id, .. }
            | Self::Server { request_id, .. } => request_id.as_deref(),
            Self::Unreachable { .. } => None,
        }
    }
}

impl From<CliError> for ExitCode {
    fn from(err: CliError) -> Self {
        Self::from(err.exit_code())
    }
}

impl From<ClientError> for CliError {
    fn from(err: ClientError) -> Self {
        match err {
            ClientError::Unreachable { url, reason } => Self::Unreachable {
                message: format!(
                    "cannot reach {url}: {reason}. Run: just up (or pass --server-url)"
                ),
            },
            ClientError::Api { status, body } => {
                let request_id = body.request_id;
                match status {
                    404 => Self::NotFound {
                        message: body.message,
                        request_id,
                    },
                    400..=499 => Self::Usage {
                        code: body.code,
                        message: body.message,
                        request_id,
                    },
                    _ => Self::Server {
                        code: body.code,
                        message: body.message,
                        request_id,
                    },
                }
            }
            ClientError::Decode { url, reason } => Self::Server {
                code: "bad_response".to_owned(),
                message: format!("unexpected response from {url}: {reason}"),
                request_id: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ApiErrorBody;

    fn api(status: u16, code: &str) -> ClientError {
        let body = ApiErrorBody {
            code: code.to_owned(),
            message: "m".to_owned(),
            request_id: Some("r".to_owned()),
        };
        ClientError::Api { status, body }
    }

    #[test]
    fn client_errors_map_to_exit_codes() {
        let cases = [
            (api(400, "validation_error"), 1, "validation_error"),
            (api(404, "not_found"), 2, "not_found"),
            (api(503, "unavailable"), 3, "unavailable"),
            (
                ClientError::Unreachable {
                    url: "u".to_owned(),
                    reason: "r".to_owned(),
                },
                4,
                "unreachable",
            ),
            (
                ClientError::Decode {
                    url: "u".to_owned(),
                    reason: "r".to_owned(),
                },
                3,
                "bad_response",
            ),
        ];
        for (client_error, exit, code) in cases {
            let err = CliError::from(client_error);
            assert_eq!(
                (err.exit_code(), err.code()),
                (exit, code),
                "mapping for {err:?}"
            );
        }
    }

    #[test]
    fn request_ids_survive_the_mapping() {
        assert_eq!(
            CliError::from(api(404, "not_found")).request_id(),
            Some("r"),
            "not found keeps it"
        );
        assert_eq!(
            CliError::from(api(500, "internal")).request_id(),
            Some("r"),
            "server error keeps it"
        );
        assert_eq!(
            CliError::usage("x", "y").request_id(),
            None,
            "usage errors have none"
        );
    }
}
