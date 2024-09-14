use crate::RESOURCE;

use opentelemetry::global;
use opentelemetry_sdk::metrics::reader::{DefaultAggregationSelector, DefaultTemporalitySelector};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_stdout::MetricsExporter;

pub(crate) fn init_metrics(use_stdout_exporter: bool) -> anyhow::Result<()> {
    let mut meter_provider = SdkMeterProvider::builder();
    if use_stdout_exporter {
        let exporter = MetricsExporter::default();
        let reader = PeriodicReader::builder(exporter, Tokio).build();
        meter_provider = meter_provider.with_reader(reader);
    } else {
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .build_metrics_exporter(
                Box::new(DefaultAggregationSelector::new()),
                Box::new(DefaultTemporalitySelector::new()),
            )?;
        let reader = PeriodicReader::builder(exporter, Tokio).build();
        meter_provider = meter_provider.with_reader(reader);
    }
    let meter_provider = meter_provider
        .with_resource(RESOURCE.get().unwrap().clone())
        .build();
    global::set_meter_provider(meter_provider);
    Ok(())
}
