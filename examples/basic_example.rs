use std::future::Future;
use std::sync::Arc;

use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, Context as OtelContext, KeyValue};
use tokio::time::{sleep, Duration};
use tracing::Instrument;
use tracing::{debug, info, instrument::WithSubscriber};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use unified_context::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    global::set_text_map_propagator(opentelemetry_jaeger_propagator::Propagator::new());
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().http())
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;
    let root_trace = tracer_provider.tracer("root_trace");
    let root_span = root_trace.in_current_span();

    let (custom_context, _context_guard1) =
        UnifiedContext::new(CurrentOtelContext, DEFAULT_INTO_TASK_CONTEXT);

    custom_context.set_span_attribute("trace_id", "root_trace");
    custom_context.set_span_attribute("span_id", "root_span");
    custom_context
        .insert_business_data("12345".to_string())
        .await;
    custom_context.insert_business_data(67890u32).await;

    debug!("Created root context");

    let span_context = custom_context.span_context();
    let (new_context, _context_guard2) = UnifiedContext::new(span_context, NONE_INTO_TASK_CONTEXT);
    debug!("Recreated context from SpanContext");

    tokio::spawn(run_async_task(new_context.clone()))
        .await
        .unwrap();

    sleep(Duration::from_secs(3)).await;

    Ok(())
}

async fn run_async_task(custom_context: UnifiedContext) {
    debug!("Running async task");

    let (child_context1, guard1) =
        custom_context.spwan_child("child_span_1", NONE_INTO_TASK_CONTEXT);

    tokio::spawn(worker(child_context1, 1)).await.unwrap();

    let (child_context2, guard2) =
        custom_context.spwan_child("child_span_2", NONE_INTO_TASK_CONTEXT);

    tokio::spawn(worker(child_context2, 2)).await.unwrap();

    sleep(Duration::from_secs(2)).await; // Simulate some work
}

fn worker(
    custom_context: UnifiedContext,
    id: usize,
    _context_guard: ContextGuard,
) -> impl Future<Output = ()> {
    let (mut worker_context, _worker_context_guard) =
        custom_context.spwan_child("worker_span", NONE_INTO_TASK_CONTEXT);
    worker_context.set_span_attribute("worker_id", id.to_string());

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    async move {
        loop {
            tokio::select! {
                _ = worker_context.done() => {
                    println!("Worker {}: Context canceled", id);
                    return;
                }
                _ = tokio::time::sleep_until(deadline) => {
                    println!("Worker {}: Reached deadline", id);
                    return;
                }
                _ = interval.tick() => {
                    let user_id: Option<Arc<String>> = worker_context.get_business_data().await;
                    let order_id: Option<Arc<u32>> = worker_context.get_business_data().await;

                    println!("Worker {}: Working... User ID: {:?}, Order ID: {:?}", id, user_id, order_id);
                }
            }
        }
    }
    // The span guard will automatically end the span here if not already ended
}
