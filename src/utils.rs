// src/context_utils.rs
use opentelemetry::baggage::{Baggage, BaggageExt};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::{
    global,
    trace::{Span, Tracer},
};
use opentelemetry::{Context as OtelContext, KeyValue};
use std::collections::HashMap;
use tracing::info;

pub fn create_context() -> OtelContext {
    let tracer = global::tracer("example-tracer");
    let root_span = tracer.start("root_operation");
    // 创建 Baggage
    let baggage_data = vec![
        KeyValue::new("session_id", "session123"),
        KeyValue::new("user_id", "user456"),
        KeyValue::new("role", "admin"),
    ];
    // 绑定 Baggage 到 Context
    OtelContext::current_with_span(root_span).with_baggage(baggage_data)
}

pub fn create_child_span(
    parent_context: &OtelContext,
    operation_name: &str,
) -> (Span, OtelContext) {
    let tracer = global::tracer("example-tracer");
    let child_span = tracer.start_with_context(operation_name, parent_context);

    // 将子Span绑定到新的Context中
    let child_context = OtelContext::current_with_span(child_span.clone())
        .with_baggage(parent_context.baggage().clone());
    (child_span, child_context)
}

pub fn extract_baggage(context: &OtelContext) -> HashMap<String, String> {
    let mut baggage_data = HashMap::new();
    let baggage = context.baggage();

    if let Some(session_id) = baggage.get("session_id") {
        baggage_data.insert("session_id".to_string(), session_id.to_string());
    }
    if let Some(user_id) = baggage.get("user_id") {
        baggage_data.insert("user_id".to_string(), user_id.to_string());
    }
    if let Some(role) = baggage.get("role") {
        baggage_data.insert("role".to_string(), role.to_string());
    }

    baggage_data
}

pub fn update_baggage(context: &OtelContext, key: &str, value: &str) -> OtelContext {
    let mut baggage = context.baggage().clone();
    baggage.insert(key, value.into());
    context.with_baggage(baggage)
}

pub fn update_span_attribute(span: &Span, key: &str, value: &str) {
    span.set_attribute(opentelemetry::KeyValue::new(key, value.to_string()));
}
