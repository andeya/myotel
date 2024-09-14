use std::sync::OnceLock;

use crate::RESOURCE;
use opentelemetry_appender_tracing::layer;
use opentelemetry_sdk::logs::{BatchConfig, BatchLogProcessor, LoggerProvider};
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_stdout::LogExporter;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::{prelude::*, EnvFilter};

/// The global `Logger` provider singleton.
static GLOBAL_LOGGER_PROVIDER: OnceLock<LoggerProvider> = OnceLock::new();

/// Returns the global LoggerProvider
pub fn logger_provider() -> &'static LoggerProvider {
    GLOBAL_LOGGER_PROVIDER.get().unwrap()
}

/// Shut down the current logger provider. This will invoke the shutdown method on all span processors.
/// span processors should export remaining spans before return
pub fn shutdown_logger_provider() {
    if let Some(logger_provider) = GLOBAL_LOGGER_PROVIDER.get() {
        let _ = logger_provider.shutdown();
    }
}

pub(crate) fn init_logs(
    use_stdout_exporter: bool,
    logs_batch_config: Option<BatchConfig>,
) -> anyhow::Result<()> {
    let mut logger_provider = LoggerProvider::builder();
    if use_stdout_exporter {
        let exporter = LogExporter::default();
        if let Some(logs_batch_config) = logs_batch_config {
            let batch = BatchLogProcessor::builder(exporter, Tokio)
                .with_batch_config(logs_batch_config)
                .build();
            logger_provider = logger_provider.with_log_processor(batch);
        } else {
            logger_provider = logger_provider.with_simple_exporter(exporter);
        }
    } else {
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .build_log_exporter()?;
        if let Some(logs_batch_config) = logs_batch_config {
            let batch = BatchLogProcessor::builder(exporter, Tokio)
                .with_batch_config(logs_batch_config)
                .build();
            logger_provider = logger_provider.with_log_processor(batch);
        } else {
            logger_provider = logger_provider.with_simple_exporter(exporter);
        }
    }
    let logger_provider = logger_provider
        .with_resource(RESOURCE.get().unwrap().clone())
        .build();
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap(),
        )
        .with(layer::OpenTelemetryTracingBridge::new(&logger_provider))
        .try_init()?;

    GLOBAL_LOGGER_PROVIDER.set(logger_provider).unwrap();
    Ok(())
}
