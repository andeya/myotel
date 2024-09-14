use std::sync::OnceLock;

use opentelemetry_appender_tracing::layer;
use opentelemetry_sdk::logs::{BatchConfig, BatchLogProcessor, LoggerProvider};
use opentelemetry_sdk::runtime::Tokio;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::prelude::*;

use crate::RESOURCE;

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

pub(crate) fn init_logs(logs_batch_config: Option<BatchConfig>) -> anyhow::Result<()> {
    let mut logger_provider = LoggerProvider::builder();
    if cfg!(debug_assertions) {
        let exporter = opentelemetry_stdout::LogExporter::default();
        if let Some(logs_batch_config) = logs_batch_config {
            let batch = BatchLogProcessor::builder(exporter, Tokio)
                .with_batch_config(logs_batch_config)
                .build();
            logger_provider = logger_provider.with_log_processor(batch);
            // logger_provider = logger_provider.with_batch_exporter(exporter, Tokio);
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
            // logger_provider = logger_provider.with_batch_exporter(exporter, Tokio);
        } else {
            logger_provider = logger_provider.with_simple_exporter(exporter);
        }
    }
    let logger_provider = logger_provider
        .with_resource(RESOURCE.get().unwrap().clone())
        .build();
    tracing_subscriber::registry()
        .with(layer::OpenTelemetryTracingBridge::new(&logger_provider))
        .try_init()?;

    GLOBAL_LOGGER_PROVIDER.set(logger_provider).unwrap();
    Ok(())
}
