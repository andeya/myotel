extern crate myotel;
use myotel::*;
use std::env;

#[tokio::main]
async fn main() {
    init_otel(default_config!()).await.unwrap();
    emit_log().await;
    println!("===========================================================");
    emit_span().await;
    println!("===========================================================");
    emit_metrics().await;
    shutdown_all_providers();
}

async fn emit_log() {
    info!("This is an info log message with OpenTelemetry integration");
    warn!("This is a warning log message with OpenTelemetry integration");
}

async fn emit_span() {
    let mut span1 = tracer_span(SpanBuilder::from_name("example-span-1"), None);
    span1.set_attribute(KeyValue::new("attribute_key1", "attribute_value1"));
    span1.set_attribute(KeyValue::new("attribute_key2", "attribute_value2"));
    span1.add_event(
        "example-event-name-1",
        vec![KeyValue::new("event_attribute1", "event_value1")],
    );
    span1.add_link(
        SpanContext::new(
            TraceId::from_hex("58406520a006649127e371903a2de979").expect("invalid"),
            SpanId::from_hex("b6d7d7f6d7d6d7f6").expect("invalid"),
            TraceFlags::default(),
            false,
            TraceState::NONE,
        ),
        vec![
            KeyValue::new("link_attribute1", "link_value1"),
            KeyValue::new("link_attribute2", "link_value2"),
        ],
    );

    span1.add_link(
        SpanContext::new(
            TraceId::from_hex("23401120a001249127e371903f2de971").expect("invalid"),
            SpanId::from_hex("cd37d765d743d7f6").expect("invalid"),
            TraceFlags::default(),
            false,
            TraceState::NONE,
        ),
        vec![
            KeyValue::new("link_attribute1", "link_value1"),
            KeyValue::new("link_attribute2", "link_value2"),
        ],
    );
    span1.end();

    let _ = {
        info!("event-span-3");
    }
    .instrument(info_span!("instrument span"));

    info!("event-name-20");
    let span2 = span!(Level::INFO, "example-span-2");
    let _enter = span2.enter();
    info!("event-name-2");
}

async fn emit_metrics() {
    env::set_var("OTEL_METRIC_EXPORT_INTERVAL", "1");
    env::set_var("OTEL_METRIC_EXPORT_TIMEOUT", "1");
    let meter = meter_provider().meter("stdout-example");
    // let meter = meter("stdout-example");
    let c = meter.u64_counter("example_counter").init();
    c.add(
        1,
        &[
            KeyValue::new("name", "apple"),
            KeyValue::new("color", "green"),
        ],
    );
    c.add(
        1,
        &[
            KeyValue::new("name", "apple"),
            KeyValue::new("color", "green"),
        ],
    );
    c.add(
        2,
        &[
            KeyValue::new("name", "apple"),
            KeyValue::new("color", "red"),
        ],
    );
    c.add(
        1,
        &[
            KeyValue::new("name", "banana"),
            KeyValue::new("color", "yellow"),
        ],
    );
    c.add(
        11,
        &[
            KeyValue::new("name", "banana"),
            KeyValue::new("color", "yellow"),
        ],
    );

    let h = meter.f64_histogram("example_histogram").init();
    h.record(
        1.0,
        &[
            KeyValue::new("name", "apple"),
            KeyValue::new("color", "green"),
        ],
    );
    h.record(
        1.0,
        &[
            KeyValue::new("name", "apple"),
            KeyValue::new("color", "green"),
        ],
    );
    h.record(
        2.0,
        &[
            KeyValue::new("name", "apple"),
            KeyValue::new("color", "red"),
        ],
    );
    h.record(
        1.0,
        &[
            KeyValue::new("name", "banana"),
            KeyValue::new("color", "yellow"),
        ],
    );
    h.record(
        11.0,
        &[
            KeyValue::new("name", "banana"),
            KeyValue::new("color", "yellow"),
        ],
    );
}
