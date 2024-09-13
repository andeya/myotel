/*!
# unified_context

A unified context management crate for handling OpenTelemetry spans, tokio-context, and business data.

This crate provides an abstraction for managing tracing spans and business contexts in a unified way. It combines the functionality of OpenTelemetry for distributed tracing and tokio-context for task cancellation, along with a mechanism to store and retrieve business-related data of various types efficiently.

## Features

- Create and manage tracing spans using OpenTelemetry.
- Gracefully cancel tasks using tokio-context.
- Store and retrieve business-related data of various types.
- Serialize and deserialize lightweight network contexts for distributed tracing.

## Usage

Add `unified_context` to your `Cargo.toml`:

```toml
[dependencies]
unified_context = "0.1.0"
```
Here's an example of how to use `unified_context`:

```rust
use tokio::time::{sleep, Duration};
use tracing::{info, debug};
use tracing_subscriber::layer::SubscriberExt;
use opentelemetry::{global, Context as OtelContext, KeyValue};
use opentelemetry_jaeger::PipelineBuilder;
use unified_context::{UnifiedContext, NetContext};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("example-app")
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("Failed to install OpenTelemetry tracer.");

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(telemetry_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (custom_context, handle) = UnifiedContext::current();
    let root_span = global::tracer("example-tracer").start("root_span");
    let (custom_context, root_span_guard) = UnifiedContext::with_span(OtelContext::current_with_span(root_span.clone()));
    custom_context.set_span_attribute("trace_id", "root_trace");
    custom_context.set_span_attribute("span_id", "root_span");
    custom_context.insert_business_data("12345".to_string()).await;
    custom_context.insert_business_data(67890u32).await;

    debug!("Created root context");

    let net_context = custom_context.to_net_context();
    let (new_context, _) = UnifiedContext::from_net_context(net_context, Some("example-tracer"));
    debug!("Recreated context from NetContext");

    tokio::spawn(run_async_task(new_context.clone())).await.unwrap();

    sleep(Duration::from_secs(3)).await;
    if let Some(handle) = handle {
        handle.cancel();
    }
    if let Some(handle) = root_span_guard.cancel_handle() {
        handle.cancel();
    }

    Ok(())
}

async fn run_async_task(custom_context: UnifiedContext) {
    debug!("Running async task");

    let (child_context1, guard1) = custom_context.start_child_span("child_span_1");
    let handle1 = guard1.cancel_handle().cloned();

    tokio::spawn(worker(child_context1, 1, guard1)).await.unwrap();

    let (child_context2, guard2) = custom_context.start_child_span("child_span_2");
    let handle2 = guard2.cancel_handle().cloned();

    tokio::spawn(worker(child_context2, 2, guard2)).await.unwrap();

    sleep(Duration::from_secs(2)).await; // Simulate some work

    if let Some(handle) = handle1 {
        handle.cancel();
    }
    if let Some(handle) = handle2 {
        handle.cancel();
    }
}

async fn worker(custom_context: UnifiedContext, id: usize, span_guard: SpanGuard) {
    let (worker_context, worker_span_guard) = custom_context.start_child_span("worker_span");
    worker_span_guard.set_span_attribute("worker_id", &id.to_string());

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let ctx = span_guard.cancel_handle();
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = ctx.clone() => {
                println!("Worker {}: Context canceled", id);
                span_guard.end_span();  // End the span explicitly if needed
                return;
            }
            _ = tokio::time::sleep_until(deadline) => {
                println!("Worker {}: Reached deadline", id);
                span_guard.end_span();  // End the span explicitly if needed
                return;
            }
            _ = interval.tick() => {
                let user_id: Option<Arc<String>> = worker_context.get_business_data().await;
                let order_id: Option<Arc<u32>> = worker_context.get_business_data().await;

                println!("Worker {}: Working... User ID: {:?}, Order ID: {:?}", id, user_id, order_id);
            }
        }
    }

    // The span guard will automatically end the span here if not already ended
}
```

## License
This project is licensed under the MIT license : LICENSE.

For more detailed examples, refer to the examples directory.
*/

pub use context::*;
mod context;
