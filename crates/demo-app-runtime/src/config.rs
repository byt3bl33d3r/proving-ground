//! Configuration: built-in defaults, then an optional TOML file (`APP_CONFIG`, default
//! `app.toml`), then `APP_*` environment variables. This is the only module that reads the
//! environment; everything else receives typed values.

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::Duration;

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// A configuration value is missing, malformed or out of range.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A source could not be parsed or a value has the wrong type.
    #[error("invalid configuration: {0}")]
    Invalid(#[from] Box<figment::Error>),
    /// A value parsed but breaks a rule.
    #[error("invalid configuration: {field} {reason}")]
    OutOfRange {
        /// Offending key.
        field: &'static str,
        /// What is wrong with it.
        reason: &'static str,
    },
}

/// Server settings. Env: `APP_HOST`, `APP_PORT`, `APP_REQUEST_TIMEOUT_MS`, `APP_HARNESS_DIR`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// Interface to bind.
    pub host: IpAddr,
    /// Port to bind; `0` lets the OS choose (the harness default).
    pub port: u16,
    /// Per-request timeout in milliseconds.
    pub request_timeout_ms: u64,
    /// Directory for `app.json` and other per-worktree state.
    pub harness_dir: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 0,
            request_timeout_ms: 10_000,
            harness_dir: PathBuf::from(".harness"),
        }
    }
}

impl ServerConfig {
    /// Loads and validates the server configuration.
    #[instrument(name = "load_config", skip_all, err)]
    pub fn load() -> Result<Self, ConfigError> {
        let file = env_value("APP_CONFIG")?.unwrap_or_else(|| "app.toml".to_owned());
        Self::from_figment(
            &Figment::from(Serialized::defaults(Self::default()))
                .merge(Toml::file(file))
                .merge(Env::prefixed("APP_").only(&[
                    "host",
                    "port",
                    "request_timeout_ms",
                    "harness_dir",
                ])),
        )
    }

    fn from_figment(figment: &Figment) -> Result<Self, ConfigError> {
        let config: Self = figment.extract().map_err(Box::new)?;
        if config.request_timeout_ms == 0 {
            return Err(ConfigError::OutOfRange {
                field: "request_timeout_ms",
                reason: "must be > 0",
            });
        }
        Ok(config)
    }

    /// The per-request timeout as a [`Duration`].
    pub const fn request_timeout(&self) -> Duration {
        Duration::from_millis(self.request_timeout_ms)
    }
}

/// Reads one environment variable through figment (`std::env::var` is banned elsewhere).
pub fn env_value(name: &str) -> Result<Option<String>, ConfigError> {
    let key = name.to_ascii_lowercase();
    let figment = Figment::from(Env::raw().only(&[name]));
    if !figment.contains(&key) {
        return Ok(None);
    }
    figment
        .extract_inner::<String>(&key)
        .map(Some)
        .map_err(|e| ConfigError::Invalid(Box::new(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layered(file: &str, overrides: &[(&str, u64)]) -> Figment {
        let mut figment =
            Figment::from(Serialized::defaults(ServerConfig::default())).merge(Toml::string(file));
        for (key, value) in overrides {
            figment = figment.merge(Serialized::default(key, value));
        }
        figment
    }

    #[test]
    fn later_sources_override_earlier_ones() {
        let figment = layered("port = 8080\nrequest_timeout_ms = 500", &[("port", 9090)]);
        let config = ServerConfig::from_figment(&figment).expect("valid");
        assert_eq!(config.port, 9090, "override wins over file");
        assert_eq!(config.request_timeout_ms, 500, "file wins over defaults");
    }

    #[test]
    fn rejects_zero_timeout_and_unknown_keys() {
        assert!(
            ServerConfig::from_figment(&layered("request_timeout_ms = 0", &[])).is_err(),
            "zero timeout"
        );
        assert!(
            ServerConfig::from_figment(&layered("prot = 1", &[])).is_err(),
            "unknown key"
        );
    }

    #[test]
    fn env_value_reads_one_variable() {
        let name = env_value("CARGO_PKG_NAME").expect("readable");
        assert_eq!(
            name.as_deref(),
            Some(env!("CARGO_PKG_NAME")),
            "cargo sets it for tests"
        );
        assert_eq!(
            env_value("APP_UNSET_FOR_TEST").expect("readable"),
            None,
            "unset"
        );
    }
}
