use crate::RESOURCE;

use opentelemetry::global;
use opentelemetry_sdk::metrics::reader::{DefaultAggregationSelector, DefaultTemporalitySelector};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_stdout::MetricsExporter;
use std::sync::OnceLock;

// OTEL_METRIC_EXPORT_INTERVAL
// OTEL_METRIC_EXPORT_TIMEOUT

/// The global `Meter` provider singleton.
static GLOBAL_MMTER_PROVIDER: OnceLock<SdkMeterProvider> = OnceLock::new();

/// Returns the global SdkMeterProvider
pub fn meter_provider() -> &'static SdkMeterProvider {
    GLOBAL_MMTER_PROVIDER.get().unwrap()
}

/// Shut down the current meter provider.
pub(crate) fn shutdown_meter_provider() {
    if let Some(meter_provider) = GLOBAL_MMTER_PROVIDER.get() {
        let _ = meter_provider.shutdown();
    }
}

pub(crate) fn init_metrics(use_stdout_exporter: bool) -> anyhow::Result<()> {
    let periodic_reader = if use_stdout_exporter {
        let exporter = MetricsExporter::default();
        PeriodicReader::builder(exporter, Tokio).build()
    } else {
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .build_metrics_exporter(
                Box::new(DefaultAggregationSelector::new()),
                Box::new(DefaultTemporalitySelector::new()),
            )?;
        PeriodicReader::builder(exporter, Tokio).build()
    };

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(RESOURCE.get().unwrap().clone())
        .with_reader(periodic_reader)
        .build();
    global::set_meter_provider(meter_provider.clone());
    GLOBAL_MMTER_PROVIDER.set(meter_provider).unwrap();
    Ok(())
}
