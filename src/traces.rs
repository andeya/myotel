fn init_trace() {
    let exporter = opentelemetry_stdout::SpanExporter::default();
    let provider = TracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_config(Config::default().with_resource(RESOURCE.clone()))
        .build();
    global::set_tracer_provider(provider);
}
