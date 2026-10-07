//! Telemetry: one `tracing` subscriber with an env filter, a JSON file layer (`APP_LOG_JSON`),
//! a compact stderr layer, and, when OTLP endpoints are set, OpenTelemetry traces, logs and
//! metrics exported over OTLP/HTTP. Every export is best effort: a missing stack costs one
//! warning, never a failure. See `docs/observability/index.md`.

use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{KeyValue, global};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{
    LogExporter, MetricExporter, Protocol, RetryPolicy, SpanExporter, WithExportConfig,
    WithHttpConfig,
};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{Sampler, SdkTracerProvider};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt};

use crate::config::env_value;

/// OpenTelemetry `service.name` unless `OTEL_SERVICE_NAME` is set.
pub const SERVICE_NAME: &str = "{{service_name}}";
/// Filter for the server when `RUST_LOG` is unset.
pub const SERVER_FILTER: &str = "info,{{crate_name}}_core=debug,{{crate_name}}_runtime=debug,{{crate_name}}_server=debug,{{crate_name}}_cli=debug,tower_http=debug,hyper=warn,h2=warn";
/// Filter for the CLI when neither `RUST_LOG` nor `-v`/`-q` is given.
pub const CLI_FILTER: &str = "warn";

/// Export timeout per batch; with retries off a down stack costs at most this per export.
const EXPORT_TIMEOUT: Duration = Duration::from_secs(2);
const TRACES_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT";
const LOGS_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_LOGS_ENDPOINT";
const METRICS_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT";

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync>;

/// Which binary is initializing telemetry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// All layers; OTLP export whenever endpoints are set.
    Server,
    /// Stderr (plus the JSON file when `APP_LOG_JSON` is set); OTLP export only with `--otel`.
    Cli {
        /// `--otel` was passed.
        otel: bool,
    },
}

/// Telemetry setup failed.
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    /// `RUST_LOG` or the override does not parse.
    #[error("invalid log filter: {0}")]
    Filter(String),
    /// The JSON log file cannot be created.
    #[error("cannot create the JSON log file {path}: {reason}")]
    LogFile {
        /// `APP_LOG_JSON`.
        path: String,
        /// Underlying error.
        reason: String,
    },
    /// An OTLP exporter could not be built.
    #[error("cannot build the OTLP exporter: {0}")]
    Exporter(String),
    /// A global subscriber was already installed.
    #[error("telemetry is already initialized: {0}")]
    AlreadyInitialized(String),
    /// An environment variable is not valid UTF-8.
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
}

/// Flushes and shuts down every provider on drop. Hold it in `main` for the whole run.
#[must_use = "dropping the guard shuts telemetry down"]
#[derive(Debug, Default)]
pub struct TelemetryGuard {
    tracer: Option<SdkTracerProvider>,
    logger: Option<SdkLoggerProvider>,
    meter: Option<SdkMeterProvider>,
    file: Option<WorkerGuard>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        let results = [
            self.tracer.take().map(|provider| provider.shutdown()),
            self.logger.take().map(|provider| provider.shutdown()),
            self.meter.take().map(|provider| provider.shutdown()),
        ];
        for err in results.into_iter().flatten().filter_map(Result::err) {
            tracing::warn!(error = %err, "telemetry_shutdown_failed");
        }
        self.file.take();
    }
}

/// Installs the global subscriber, OpenTelemetry providers and the panic hook.
/// `filter` overrides `RUST_LOG` (the CLI's `-v`/`-q`).
pub fn init(
    mode: Mode,
    version: &'static str,
    filter: Option<&str>,
) -> Result<TelemetryGuard, TelemetryError> {
    let mut guard = TelemetryGuard::default();
    let mut layers: Vec<BoxedLayer> = Vec::new();
    if let Some((layer, worker)) = json_file_layer()? {
        layers.push(layer);
        guard.file = Some(worker);
    }
    let stderr = fmt::layer()
        .compact()
        .with_writer(std::io::stderr)
        .with_target(mode == Mode::Server);
    layers.push(stderr.boxed());
    let endpoints = otlp_endpoints(mode)?;
    if !endpoints.is_empty() {
        otel_layers(&endpoints, &resource(version)?, &mut layers, &mut guard)?;
    }
    tracing_subscriber::registry()
        .with(layers.with_filter(env_filter(mode, filter)?))
        .try_init()
        .map_err(|e| TelemetryError::AlreadyInitialized(e.to_string()))?;
    install_panic_hook();
    warn_if_unreachable(endpoints);
    Ok(guard)
}

fn env_filter(mode: Mode, filter: Option<&str>) -> Result<EnvFilter, TelemetryError> {
    let directives = match filter {
        Some(directives) => directives.to_owned(),
        None => env_value("RUST_LOG")?.unwrap_or_else(|| {
            let default = if mode == Mode::Server {
                SERVER_FILTER
            } else {
                CLI_FILTER
            };
            default.to_owned()
        }),
    };
    EnvFilter::try_new(directives).map_err(|e| TelemetryError::Filter(e.to_string()))
}

fn json_file_layer() -> Result<Option<(BoxedLayer, WorkerGuard)>, TelemetryError> {
    let Some(path) = env_value("APP_LOG_JSON")? else {
        return Ok(None);
    };
    let file_error = |reason: String| TelemetryError::LogFile {
        path: path.clone(),
        reason,
    };
    let target = Path::new(&path);
    let dir = target
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .ok_or_else(|| file_error("no file name".to_owned()))?;
    std::fs::create_dir_all(dir).map_err(|e| file_error(e.to_string()))?;
    let appender = tracing_appender::rolling::never(dir, name);
    let (writer, worker) = tracing_appender::non_blocking(appender);
    let layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(writer);
    Ok(Some((layer.boxed(), worker)))
}

/// The OTLP endpoint variables that are set, or none when export is off for this mode.
fn otlp_endpoints(mode: Mode) -> Result<Vec<(&'static str, String)>, TelemetryError> {
    if mode == (Mode::Cli { otel: false }) {
        return Ok(Vec::new());
    }
    let mut endpoints = Vec::new();
    for var in [TRACES_ENDPOINT, LOGS_ENDPOINT, METRICS_ENDPOINT] {
        if let Some(url) = env_value(var)? {
            endpoints.push((var, url));
        }
    }
    Ok(endpoints)
}

fn resource(version: &'static str) -> Result<Resource, TelemetryError> {
    let mut builder = Resource::builder().with_attributes([
        KeyValue::new("service.version", version),
        KeyValue::new("deployment.environment.name", "dev"),
    ]);
    if env_value("OTEL_SERVICE_NAME")?.is_none() {
        builder = builder.with_service_name(SERVICE_NAME);
    }
    Ok(builder.build())
}

fn otel_layers(
    endpoints: &[(&'static str, String)],
    resource: &Resource,
    layers: &mut Vec<BoxedLayer>,
    guard: &mut TelemetryGuard,
) -> Result<(), TelemetryError> {
    let exporter_error =
        |e: opentelemetry_otlp::ExporterBuildError| TelemetryError::Exporter(e.to_string());
    let has = |var: &str| endpoints.iter().any(|(name, _)| *name == var);
    global::set_text_map_propagator(TraceContextPropagator::new());
    if has(TRACES_ENDPOINT) {
        let exporter = SpanExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_timeout(EXPORT_TIMEOUT)
            .with_retry_policy(RetryPolicy::disabled())
            .build()
            .map_err(exporter_error)?;
        let mut builder = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource.clone());
        if env_value("OTEL_TRACES_SAMPLER")?.is_none() {
            builder = builder.with_sampler(Sampler::AlwaysOn);
        }
        let provider = builder.build();
        layers.push(
            tracing_opentelemetry::layer()
                .with_tracer(provider.tracer(SERVICE_NAME))
                .boxed(),
        );
        global::set_tracer_provider(provider.clone());
        guard.tracer = Some(provider);
    }
    if has(LOGS_ENDPOINT) {
        let exporter = LogExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_timeout(EXPORT_TIMEOUT)
            .with_retry_policy(RetryPolicy::disabled())
            .build()
            .map_err(exporter_error)?;
        let provider = SdkLoggerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource.clone())
            .build();
        layers.push(OpenTelemetryTracingBridge::new(&provider).boxed());
        guard.logger = Some(provider);
    }
    if has(METRICS_ENDPOINT) {
        let exporter = MetricExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_timeout(EXPORT_TIMEOUT)
            .with_retry_policy(RetryPolicy::disabled())
            .build()
            .map_err(exporter_error)?;
        let provider = SdkMeterProvider::builder()
            .with_periodic_exporter(exporter)
            .with_resource(resource.clone())
            .build();
        global::set_meter_provider(provider.clone());
        guard.meter = Some(provider);
    }
    Ok(())
}

/// Logs panics as `process_panicked` events (JSON file and the logs backend), then runs the
/// default hook.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map(ToString::to_string).unwrap_or_default();
        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!(
            panic.message = info.payload_as_str().unwrap_or("non-string panic payload"),
            panic.location = %location,
            backtrace = %backtrace,
            "process_panicked"
        );
        default_hook(info);
    }));
}

/// One warning if no OTLP endpoint accepts TCP connections (checked off the startup path).
fn warn_if_unreachable(endpoints: Vec<(&'static str, String)>) {
    if endpoints.is_empty() {
        return;
    }
    let probe = move || {
        let down: Vec<String> = endpoints
            .into_iter()
            .filter(|(_, url)| !accepts(url))
            .map(|(_, url)| url)
            .collect();
        if !down.is_empty() {
            tracing::warn!(endpoints = ?down, "otlp_endpoint_unreachable");
        }
    };
    if let Err(e) = std::thread::Builder::new()
        .name("otlp-probe".to_owned())
        .spawn(probe)
    {
        tracing::warn!(error = %e, "otlp_probe_not_started");
    }
}

fn accepts(url: &str) -> bool {
    let authority = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or_default();
    authority.to_socket_addrs().is_ok_and(|mut addrs| {
        addrs.any(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_filters_parse() {
        for directives in [SERVER_FILTER, CLI_FILTER] {
            assert!(
                EnvFilter::try_new(directives).is_ok(),
                "{directives} parses"
            );
        }
    }

    #[test]
    fn cli_without_otel_exports_nothing() {
        assert_eq!(
            otlp_endpoints(Mode::Cli { otel: false }).ok(),
            Some(Vec::new()),
            "no endpoints"
        );
    }

    #[test]
    fn accepts_rejects_closed_ports() {
        assert!(!accepts("http://127.0.0.1:9/v1/traces"), "port 9 is closed");
    }
}
