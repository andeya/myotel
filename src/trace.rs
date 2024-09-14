use crate::RESOURCE;

use opentelemetry::global;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::trace::{BatchConfig, BatchSpanProcessor, Config, TracerProvider};
use opentelemetry_stdout::SpanExporter;

pub(crate) fn init_trace(
    use_stdout_exporter: bool,
    trace_batch_config: Option<BatchConfig>,
) -> anyhow::Result<()> {
    let mut tracer_provider = TracerProvider::builder();
    if use_stdout_exporter {
        let exporter = SpanExporter::default();
        if let Some(trace_batch_config) = trace_batch_config {
            let batch = BatchSpanProcessor::builder(exporter, Tokio)
                .with_batch_config(trace_batch_config)
                .build();
            tracer_provider = tracer_provider.with_span_processor(batch);
        } else {
            tracer_provider = tracer_provider.with_simple_exporter(exporter);
        }
    } else {
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .build_span_exporter()?;
        if let Some(trace_batch_config) = trace_batch_config {
            let batch = BatchSpanProcessor::builder(exporter, Tokio)
                .with_batch_config(trace_batch_config)
                .build();
            tracer_provider = tracer_provider.with_span_processor(batch);
        } else {
            tracer_provider = tracer_provider.with_simple_exporter(exporter);
        }
    }
    let tracer_provider = tracer_provider
        .with_config(Config::default().with_resource(RESOURCE.get().unwrap().clone()))
        .build();
    global::set_tracer_provider(tracer_provider);
    Ok(())
}
