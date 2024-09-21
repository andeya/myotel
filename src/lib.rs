/*!
# myotel

This is a foolproof best practice for initializing the integration of OpenTelemetry with the `tracing` library, providing support for logs, metrics, and trace.

## Features

-   **Logs**: Advanced logging capabilities integrated with OpenTelemetry.
-   **Metrics**: Flexible metric collection supporting various measurement types.
-   **Trace**: Rich distributed tracing tools for creating spans, adding events, and linking spans.

## Install

Run the following Cargo command in your project directory:

```sh
cargo add myotel
```

Or add the following line to your Cargo.toml:

```sh
myotel = "0.1"
```

## Examples

```rust
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
    let mut otel_span = tracer_span(SpanBuilder::from_name("example-span-1"), None);
    otel_span.set_attribute(KeyValue::new("attribute_key1", "attribute_value1"));
    otel_span.set_attribute(KeyValue::new("attribute_key2", "attribute_value2"));
    otel_span.add_event(
        "example-event-name-1",
        vec![KeyValue::new("event_attribute1", "event_value1")]
    );
    otel_span.add_link(
        SpanContext::new(
            TraceId::from_hex("58406520a006649127e371903a2de979").expect("invalid"),
            SpanId::from_hex("b6d7d7f6d7d6d7f6").expect("invalid"),
            TraceFlags::default(),
            false,
            TraceState::NONE
        ),
        vec![
            KeyValue::new("link_attribute1", "link_value1"),
            KeyValue::new("link_attribute2", "link_value2")
        ]
    );

    otel_span.add_link(
        SpanContext::new(
            TraceId::from_hex("23401120a001249127e371903f2de971").expect("invalid"),
            SpanId::from_hex("cd37d765d743d7f6").expect("invalid"),
            TraceFlags::default(),
            false,
            TraceState::NONE
        ),
        vec![
            KeyValue::new("link_attribute1", "link_value1"),
            KeyValue::new("link_attribute2", "link_value2")
        ]
    );
    (
        async {
            let _ = (
                {
                    info!("event-span-3");
                }
            ).instrument(info_span!("instrument span"));

            info!("event-name-20");
            let span2 = span!(Level::INFO, "example-span-2");
            let _enter = span2.enter();
            info!("event-name-2");
        }
    ).with_current_context_span(otel_span).await;
}

async fn emit_metrics() {
    env::set_var("OTEL_METRIC_EXPORT_INTERVAL", "1");
    env::set_var("OTEL_METRIC_EXPORT_TIMEOUT", "1");
    let meter = meter_provider().meter("stdout-example");
    // let meter = meter("stdout-example");
    let c = meter.u64_counter("example_counter").init();
    c.add(1, &[KeyValue::new("name", "apple"), KeyValue::new("color", "green")]);
    c.add(1, &[KeyValue::new("name", "apple"), KeyValue::new("color", "green")]);
    c.add(2, &[KeyValue::new("name", "apple"), KeyValue::new("color", "red")]);
    c.add(1, &[KeyValue::new("name", "banana"), KeyValue::new("color", "yellow")]);
    c.add(11, &[KeyValue::new("name", "banana"), KeyValue::new("color", "yellow")]);

    let h = meter.f64_histogram("example_histogram").init();
    h.record(1.0, &[KeyValue::new("name", "apple"), KeyValue::new("color", "green")]);
    h.record(1.0, &[KeyValue::new("name", "apple"), KeyValue::new("color", "green")]);
    h.record(2.0, &[KeyValue::new("name", "apple"), KeyValue::new("color", "red")]);
    h.record(1.0, &[KeyValue::new("name", "banana"), KeyValue::new("color", "yellow")]);
    h.record(11.0, &[KeyValue::new("name", "banana"), KeyValue::new("color", "yellow")]);
}
```
*/

#![deny(missing_docs)]

mod logs;
mod metrics;
mod trace;

use opentelemetry::global;
use opentelemetry_sdk::Resource;
use std::sync::{ Mutex, OnceLock };
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::EnvFilter;

pub use _tracing::*;
pub use logs::*;
pub use metrics::*;

pub use opentelemetry::{
    Array,
    InstrumentationLibrary,
    InstrumentationLibraryBuilder,
    Key,
    KeyValue,
    Value,
};
pub use trace::*;
mod _tracing {
    pub use tracing;
    // Attribute Macros
    pub use tracing::instrument;
    // Macros
    pub use tracing::{
        debug,
        debug_span,
        enabled,
        error,
        error_span,
        event,
        event_enabled,
        info,
        info_span,
        span,
        span_enabled,
        trace,
        trace_span,
        warn,
        warn_span,
    };
    pub use tracing::{ Instrument, Level };
}

static RESOURCE: OnceLock<Resource> = OnceLock::new();

/// OpenTelemetry initialization configuration.
#[derive(Debug, getset2::WithSetters)]
#[getset(set_with = "pub")]
pub struct InitConfig {
    /// Service name
    service_name: String,
    /// Service version
    service_version: String,
    /// Whether to use the standard output.
    /// The standard output is used by default in debug mode,
    /// and OTLP is used in release mode.
    stdout_exporter: bool,
    /// If the batch log configuration is configured, batch reporting will be enabled.
    batch_log_config: Option<BatchLogConfig>,
    /// If the batch trace configuration is configured, batch reporting will be enabled.
    batch_trace_config: Option<BatchTraceConfig>,
    /// Tracer Provider Config.
    tracer_provider_config: TracerProviderConfig,
}

impl InitConfig {
    /// Create a new InitConfig.
    pub fn new() -> Self {
        Self {
            service_name: Default::default(),
            service_version: Default::default(),
            stdout_exporter: cfg!(debug_assertions),
            batch_log_config: Default::default(),
            batch_trace_config: Default::default(),
            tracer_provider_config: Default::default(),
        }
    }
}

/// Create the default InitConfig.
#[macro_export]
macro_rules! default_config {
    () => {
        InitConfig::new()
            .with_service_name(env!("CARGO_PKG_NAME").to_owned())
            .with_service_version(env!("CARGO_PKG_VERSION").to_owned())
    };
}

static INIT: Mutex<bool> = Mutex::new(false);

/// Initialize OpenTelemetry.
pub async fn init_otel(init_config: InitConfig) -> anyhow::Result<bool> {
    let mut guard = INIT.lock().unwrap();
    if *guard {
        return Ok(false);
    }
    *guard = true;

    let mut kvs = vec![];
    if !init_config.service_name.is_empty() {
        kvs.push(
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                init_config.service_name
            )
        );
    }
    if !init_config.service_version.is_empty() {
        kvs.push(
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                init_config.service_version
            )
        );
    }
    RESOURCE.set(Resource::default().merge(&Resource::new(kvs))).unwrap();

    init_logs_and_trace(
        init_config.stdout_exporter,
        init_config.batch_log_config,
        init_config.batch_trace_config,
        init_config.tracer_provider_config.with_resource(RESOURCE.get().unwrap().clone())
    )?;
    metrics::init_metrics(init_config.stdout_exporter)?;

    Ok(true)
}

fn init_logs_and_trace(
    use_stdout_exporter: bool,
    batch_log_config: Option<BatchLogConfig>,
    batch_trace_config: Option<BatchTraceConfig>,
    tracer_provider_config: TracerProviderConfig
) -> anyhow::Result<()> {
    let env_filter_layer = EnvFilter::try_from_default_env().or_else(|_|
        EnvFilter::try_new("info")
    )?;

    let tracer = trace::init_trace(
        use_stdout_exporter,
        batch_trace_config,
        tracer_provider_config
    )?;
    let tracer_layer = OpenTelemetryLayer::new(tracer);

    let subscriber = tracing_subscriber::registry().with(env_filter_layer).with(tracer_layer);

    if use_stdout_exporter {
        let fmt_layer = tracing_subscriber::fmt
            ::layer()
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .pretty();
        tracing::subscriber::set_global_default(subscriber.with(fmt_layer))?;
    } else {
        let logger_layer = logs::init_logs(use_stdout_exporter, batch_log_config)?;
        tracing::subscriber::set_global_default(subscriber.with(logger_layer))?;
    }

    Ok(())
}

/// Shut down the current logger, tracer and meter providers.
pub fn shutdown_all_providers() {
    logs::shutdown_logger_provider();
    global::shutdown_tracer_provider();
    metrics::shutdown_meter_provider();
}
