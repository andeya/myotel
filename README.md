# myotel

This is a foolproof best practice for initializing the integration of OpenTelemetry with the `tracing` library, providing support for logs, metrics, and trace.

## Features

-   **Logs**: Advanced logging capabilities integrated with OpenTelemetry.
-   **Metrics**: Flexible metric collection supporting various measurement types.
-   **Trace**: Rich distributed tracing tools for creating spans, adding events, and linking spans.

## Examples

### Logs

```rust
use tracing::{info, warn};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_otel(InitConfig::default()).await?;
    info!("This is an info log message with OpenTelemetry integration");
    warn!("This is a warning log message with OpenTelemetry integration");
    shutdown_logger_provider();
    Ok(())
}
```

## Metrics

```rust
use std::env;
use opentelemetry::global;
use opentelemetry::KeyValue;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env::set_var("OTEL_METRIC_EXPORT_INTERVAL", "1");
    env::set_var("OTEL_METRIC_EXPORT_TIMEOUT", "1");
    init_otel(InitConfig::default()).await?;
    let meter = global::meter("stdout-example");
    let counter = meter.u64_counter("example_counter").init();
    counter.add(1, &[KeyValue::new("name", "apple"), KeyValue::new("color", "green")]);
    Ok(())
}
```

## Trace

```rust
use opentelemetry::global;
use opentelemetry::trace::{Span, Tracer};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_otel(InitConfig::default()).await?;
    let tracer = global::tracer("trace-example");
    let mut span = tracer.start("example-span");
    span.set_attribute(KeyValue::new("key", "value"));
    span.add_event("event-name", vec![KeyValue::new("event_key", "event_value")]);
    span.end();
    Ok(())
}
```
