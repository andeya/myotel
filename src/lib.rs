use opentelemetry::KeyValue;
use opentelemetry_sdk::{logs::BatchConfig as LogsBatchConfig, Resource};
use std::sync::OnceLock;

// pub mod metrics;
// pub mod traces;
mod logs;
pub use logs::*;
pub use opentelemetry::global::shutdown_tracer_provider;
static RESOURCE: OnceLock<Resource> = OnceLock::new();

/// OpenTelemetry initialization configuration.
pub struct InitConfig {
    service_name: String,
    logs_batch_config: Option<LogsBatchConfig>,
}
impl Default for InitConfig {
    fn default() -> Self {
        Self {
            service_name: "myotel".to_owned(),
            logs_batch_config: Default::default(),
        }
    }
}

/// Initialize OpenTelemetry.
pub fn init_otel(init_config: InitConfig) -> anyhow::Result<()> {
    RESOURCE
        .set(Resource::default().merge(&Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            init_config.service_name,
        )])))
        .unwrap();
    logs::init_logs(init_config.logs_batch_config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{info, warn};
    #[test]
    fn emit_log() {
        init_otel(InitConfig::default()).unwrap();

        info!("This is an info log message with OpenTelemetry integration");
        warn!("This is a warning log message with OpenTelemetry integration");

        shutdown_logger_provider();
    }
}
