/*!
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
    myotel::init_otel(myotel::InitConfig::default()).await?;
    info!("This is an info log message with OpenTelemetry integration");
    warn!("This is a warning log message with OpenTelemetry integration");
    myotel::shutdown_logger_provider();
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
    std::env::set_var("OTEL_METRIC_EXPORT_INTERVAL", "1");
    std::env::set_var("OTEL_METRIC_EXPORT_TIMEOUT", "1");
    myotel::init_otel(myotel::InitConfig::default()).await?;
    let meter = global::meter("stdout-example");
    let counter = meter.u64_counter("example_counter").init();
    counter.add(1, &[KeyValue::new("name", "apple"), KeyValue::new("color", "green")]);
    Ok(())
}
```

## Trace

```rust
use opentelemetry::global;
use opentelemetry::KeyValue;
use opentelemetry::trace::{Span, Tracer};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    myotel::init_otel(myotel::InitConfig::default()).await?;
    let tracer = global::tracer("trace-example");
    let mut span = tracer.start("example-span");
    span.set_attribute(KeyValue::new("key", "value"));
    span.add_event("event-name", vec![KeyValue::new("event_key", "event_value")]);
    span.end();
    Ok(())
}
```
*/

mod logs;
mod metrics;
mod trace;

pub use logs::{logger_provider, shutdown_logger_provider};
pub use opentelemetry::global::{
    meter, meter_provider, meter_with_version, shutdown_tracer_provider, tracer, tracer_provider,
};
use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;
pub use opentelemetry_sdk::{
    logs::BatchConfig as LogsBatchConfig, trace::BatchConfig as TraceBatchConfig,
};
use std::sync::{Mutex, OnceLock};

static RESOURCE: OnceLock<Resource> = OnceLock::new();

/// OpenTelemetry initialization configuration.
#[derive(Debug, getset::WithSetters)]
#[getset(set_with = "pub")]
pub struct InitConfig {
    service_name: String,
    logs_batch_config: Option<LogsBatchConfig>,
    trace_batch_config: Option<TraceBatchConfig>,
    stdout_exporter: bool,
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            service_name: "myotel".to_owned(),
            logs_batch_config: Default::default(),
            stdout_exporter: cfg!(debug_assertions),
            trace_batch_config: Default::default(),
        }
    }
}

static INIT: Mutex<bool> = Mutex::new(false);

/// Initialize OpenTelemetry.
pub async fn init_otel(init_config: InitConfig) -> anyhow::Result<bool> {
    let mut guard = INIT.lock().unwrap();
    if *guard {
        return Ok(false);
    }
    *guard = true;

    RESOURCE
        .set(Resource::default().merge(&Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            init_config.service_name,
        )])))
        .unwrap();
    logs::init_logs(init_config.stdout_exporter, init_config.logs_batch_config)?;
    trace::init_trace(init_config.stdout_exporter, init_config.trace_batch_config)?;
    metrics::init_metrics(init_config.stdout_exporter)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::{env, time::Duration};

    use super::*;
    use opentelemetry::{
        global,
        trace::{Span, Tracer},
    };
    use tracing::{info, warn};
    #[tokio::test]
    async fn emit_log() {
        init_otel(InitConfig::default()).await.unwrap();
        info!("This is an info log message with OpenTelemetry integration");
        warn!("This is a warning log message with OpenTelemetry integration");
        shutdown_logger_provider();
    }

    #[tokio::test]
    async fn emit_metrics() {
        env::set_var("OTEL_METRIC_EXPORT_INTERVAL", "1");
        env::set_var("OTEL_METRIC_EXPORT_TIMEOUT", "1");
        init_otel(InitConfig::default()).await.unwrap();
        let meter = global::meter("stdout-example");
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
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn emit_span() {
        init_otel(InitConfig::default()).await.unwrap();
        use opentelemetry::trace::{
            SpanContext, SpanId, TraceFlags, TraceId, TraceState, TracerProvider,
        };

        let tracer = global::tracer_provider()
            .tracer_builder("trace-example")
            .with_version("v1")
            .with_schema_url("schema_url")
            .with_attributes([KeyValue::new("scope_key", "scope_value")])
            .build();
        let mut span1 = tracer.start("example-span-1");
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
        let mut span2 = tracer.start("example-span-2");
        span2.add_event(
            "example-event-name-2",
            vec![KeyValue::new("event_attribute2", "event_value2")],
        );
        span2.end();
    }
}
