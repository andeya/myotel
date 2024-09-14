use crate::RESOURCE;

use opentelemetry::global;
use opentelemetry_sdk::metrics::reader::{DefaultAggregationSelector, DefaultTemporalitySelector};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_stdout::MetricsExporter;

// OTEL_METRIC_EXPORT_INTERVAL
// OTEL_METRIC_EXPORT_TIMEOUT

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
    global::set_meter_provider(meter_provider);
    Ok(())
}
