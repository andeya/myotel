use crate::RESOURCE;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::runtime::Tokio;
pub use opentelemetry_sdk::{trace::BatchConfig as BatchTraceConfig, trace::Tracer};
use opentelemetry_sdk::{
    trace::BatchSpanProcessor, trace::Config as TraceConfig, trace::TracerProvider,
};
use opentelemetry_stdout::SpanExporter;

const INSTRUMENTATION_LIBRARY_NAME: &str = "opentelemetry-appender-tracing";

pub(crate) fn init_trace(
    use_stdout_exporter: bool,
    batch_trace_config: Option<BatchTraceConfig>,
) -> anyhow::Result<Tracer> {
    let mut tracer_provider = TracerProvider::builder();
    if use_stdout_exporter {
        let span_exporter = SpanExporter::default();
        if let Some(batch_trace_config) = batch_trace_config {
            let batch = BatchSpanProcessor::builder(span_exporter, Tokio)
                .with_batch_config(batch_trace_config)
                .build();
            tracer_provider = tracer_provider.with_span_processor(batch);
        } else {
            tracer_provider = tracer_provider.with_simple_exporter(span_exporter);
        }
    } else {
        let span_exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .build_span_exporter()?;
        if let Some(batch_trace_config) = batch_trace_config {
            let batch = BatchSpanProcessor::builder(span_exporter, Tokio)
                .with_batch_config(batch_trace_config)
                .build();
            tracer_provider = tracer_provider.with_span_processor(batch);
        } else {
            tracer_provider = tracer_provider.with_simple_exporter(span_exporter);
        }
    }

    let tracer_provider: TracerProvider = tracer_provider
        .with_config(TraceConfig::default().with_resource(RESOURCE.get().unwrap().clone()))
        .build();

    let tracer = tracer_provider.tracer(INSTRUMENTATION_LIBRARY_NAME);

    global::set_tracer_provider(tracer_provider);

    Ok(tracer)
}
