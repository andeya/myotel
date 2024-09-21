pub use opentelemetry_sdk::logs::BatchConfig as BatchLogConfig;

use std::sync::OnceLock;
use crate::RESOURCE;
use opentelemetry_appender_tracing::layer;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::{ logs::BatchLogProcessor, logs::Logger, logs::LoggerProvider };
use opentelemetry_stdout::LogExporter;

/// The global `Logger` provider singleton.
static GLOBAL_LOGGER_PROVIDER: OnceLock<LoggerProvider> = OnceLock::new();

/// Returns the global LoggerProvider
pub fn logger_provider() -> &'static LoggerProvider {
    GLOBAL_LOGGER_PROVIDER.get().unwrap()
}

/// Shut down the current logger provider.
/// This will invoke the shutdown method on all log processors.
/// log processors should export remaining logs before return.
pub(crate) fn shutdown_logger_provider() {
    if let Some(logger_provider) = GLOBAL_LOGGER_PROVIDER.get() {
        let _ = logger_provider.shutdown();
    }
}

pub(crate) fn init_logs(
    use_stdout_exporter: bool,
    batch_log_config: Option<BatchLogConfig>
) -> anyhow::Result<layer::OpenTelemetryTracingBridge<LoggerProvider, Logger>> {
    let mut logger_provider = LoggerProvider::builder();
    if use_stdout_exporter {
        let log_exporter = LogExporter::default();
        if let Some(logs_batch_config) = batch_log_config {
            let batch = BatchLogProcessor::builder(log_exporter, Tokio)
                .with_batch_config(logs_batch_config)
                .build();
            logger_provider = logger_provider.with_log_processor(batch);
        } else {
            logger_provider = logger_provider.with_simple_exporter(log_exporter);
        }
    } else {
        let log_exporter = opentelemetry_otlp::new_exporter().tonic().build_log_exporter()?;
        if let Some(logs_batch_config) = batch_log_config {
            let batch = BatchLogProcessor::builder(log_exporter, Tokio)
                .with_batch_config(logs_batch_config)
                .build();
            logger_provider = logger_provider.with_log_processor(batch);
        } else {
            logger_provider = logger_provider.with_simple_exporter(log_exporter);
        }
    }
    let logger_provider = logger_provider.with_resource(RESOURCE.get().unwrap().clone()).build();

    let logger_layer: layer::OpenTelemetryTracingBridge<
        LoggerProvider,
        opentelemetry_sdk::logs::Logger
    > = layer::OpenTelemetryTracingBridge::new(&logger_provider);

    GLOBAL_LOGGER_PROVIDER.set(logger_provider).unwrap();
    Ok(logger_layer)
}
