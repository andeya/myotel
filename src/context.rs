pub use opentelemetry::trace::{SpanContext, SpanRef};
use opentelemetry::trace::{TraceContextExt, Tracer};
pub use opentelemetry::{global, Context as OtelContext, Key, KeyValue, Value};
// use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;
pub use tokio_context::context::{Handle as TaskHandle, RefContext as TaskContext};
use tracing::{debug, span};

/// A unified context management struct that holds tracing spans, cancellation context, and business data.
#[derive(Clone)]
pub struct UnifiedContext {
    /// A map to store business-related data with flexible types.
    business_data:
        Arc<Mutex<HashMap<TypeId, Arc<dyn Any + Sync + Send>, BuildHasherDefault<IdHasher>>>>,
    /// Cancellation context shared across all contexts. Only the root context can actually cancel it.
    task_context: Option<TaskContext>,
    /// OpenTelemetry tracing context.
    trace_context: OtelContext,
}

/// With TypeIds as keys, there's no need to hash them. They are already hashes
/// themselves, coming from the compiler. The IdHasher holds the u64 of
/// the TypeId, and then returns it, instead of doing any bit fiddling.
#[derive(Clone, Default, Debug)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

impl Debug for UnifiedContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnifiedContext")
            .field("business_data", &self.business_data)
            .field(
                "cancel_context",
                &"::std::sync::<Arc<::tokio_context::context::Context>>",
            )
            .field("trace_context", &self.trace_context)
            .finish()
    }
}

impl UnifiedContext {
    /// Creates a new `UnifiedContext` instance with the current trace context
    /// but without the task context.
    #[inline]
    pub fn current() -> (Self, ContextGuard) {
        Self::new(CurrentOtelContext, NONE_INTO_TASK_CONTEXT)
    }
    /// Creates a new `UnifiedContext` instance with the given trace_context and task_context.
    pub fn new(
        trace_context: impl IntoOtelContext,
        task_context: Option<impl IntoTaskContext>,
    ) -> (Self, ContextGuard) {
        let (task_context, _task_handle) = if let Some(tc) = task_context {
            let (ctx, handle) = tc.into_task_context();
            (Some(ctx), Some(handle))
        } else {
            (None, None)
        };
        let context = UnifiedContext {
            business_data: Arc::new(Mutex::new(HashMap::default())),
            task_context,
            trace_context: trace_context.into_otel_context(),
        };

        let span_guard = ContextGuard {
            unified_context: context.clone(),
            _task_handle,
        };
        (context, span_guard)
    }

    /// Inserts a key-value pair into the business data.
    ///
    /// The key is the `TypeId` of the type of the value to be stored.
    pub async fn insert_business_data<T: Any + Sync + Send>(&self, value: T) {
        let type_id = TypeId::of::<T>();
        let mut data = self.business_data.lock().await;
        data.insert(type_id, Arc::new(value));
        debug!("Inserted business data with type_id: {:?}", type_id);
    }

    /// Retrieves the value associated with the given type from the business data.
    ///
    /// The key is the `TypeId` of the type of the value to be retrieved.
    pub async fn get_business_data<T: Any + Sync + Send>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();
        let data = self.business_data.lock().await;
        data.get(&type_id)
            .and_then(|arc_any| arc_any.clone().downcast::<T>().ok())
    }

    /// An object that is passed around to asynchronous functions that may be used to check if the
    /// function it was passed into should perform a graceful termination.
    pub fn task_context(&self) -> Option<TaskContext> {
        self.task_context.clone()
    }

    /// Return true if the tast_context has timed out or been canceled,
    /// otherwise return false if no tast_context is set.
    pub async fn done(&mut self) -> bool {
        if let Some(task_context) = &mut self.task_context {
            task_context.done().await;
            true
        } else {
            false
        }
    }

    /// Returns the current span.
    pub fn ref_span(&self) -> SpanRef<'_> {
        self.trace_context.span()
    }

    /// Whether the current span is active.
    pub fn has_active_span(&self) -> bool {
        self.trace_context.has_active_span()
    }

    /// Get the SpanContext clone, which is equivalent to `self.ref_span().span_context().clone()`.
    pub fn span_context(&self) -> SpanContext {
        self.ref_span().span_context().clone()
    }

    /// Starts a child context and returns a `SpanGuard`.
    ///
    /// The child context inherits the cancel context from its parent but does not generate a new cancel handle.
    pub fn spwan_child(
        &self,
        span_name: impl Into<Cow<'static, str>>,
        task_context: Option<impl IntoTaskContext>,
    ) -> (Self, ContextGuard) {
        let name = span_name.into();
        let tracer = global::tracer("unified_context_tracer");
        debug!("Started child context: span_name={}", name);
        let child_span = tracer.start_with_context(name, &self.trace_context);
        let (task_context, _task_handle) = if let Some(tc) = task_context {
            let (ctx, handle) = tc.into_task_context();
            (Some(ctx), Some(handle))
        } else {
            // Child contexts inherit the cancel context
            // Child spans do not have a cancel handle
            (self.task_context.clone(), None)
        };

        let child_context = UnifiedContext {
            business_data: self.business_data.clone(),
            task_context,
            trace_context: OtelContext::current_with_span(child_span),
        };

        let span_guard = ContextGuard {
            unified_context: child_context.clone(),
            _task_handle,
        };

        (child_context, span_guard)
    }

    /// Ends the current span.
    fn end_span(&self) {
        let span = self.ref_span();
        let ctx = span.span_context();
        debug!(
            "Ending span with trace_id: {:?}, span_id: {:?}",
            ctx.trace_id(),
            ctx.span_id()
        );
        span.end();
    }

    /// Sets an attribute for the current span.
    pub fn set_span_attribute(&self, key: impl Into<Key>, value: impl Into<Value>) {
        let kv = KeyValue::new(key, value);
        debug!("Set span attribute: {kv:?}");
        self.ref_span().set_attribute(kv);
    }
}

/// A guard that ends a span when it is dropped.
pub struct ContextGuard {
    unified_context: UnifiedContext,
    /// TaskHandle for cancellation.
    _task_handle: Option<TaskHandle>,
}

impl ContextGuard {
    /// 1. Signals that the operation described by this span has now ended.
    /// 2. Cancels the Context, ensuring that done() returns immediately.
    pub fn end(self) {
        // drop.
    }
}

impl Drop for ContextGuard {
    /// 1. Signals that the operation described by this span has now ended.
    /// 2. Cancels the Context, ensuring that done() returns immediately.
    fn drop(&mut self) {
        self.unified_context.end_span();
    }
}

/// IntoOtelContext covert to OtelContext
pub trait IntoOtelContext {
    fn into_otel_context(self) -> OtelContext;
}

impl IntoOtelContext for OtelContext {
    #[inline(always)]
    fn into_otel_context(self) -> OtelContext {
        self
    }
}

impl IntoOtelContext for SpanContext {
    #[inline(always)]
    fn into_otel_context(self) -> OtelContext {
        opentelemetry::Context::new().with_remote_span_context(self)
    }
}

/// Current OtelContext Creater
pub struct CurrentOtelContext;
impl IntoOtelContext for CurrentOtelContext {
    #[inline]
    fn into_otel_context(self) -> OtelContext {
        OtelContext::current()
    }
}

/// Convert to TaskContext
pub trait IntoTaskContext {
    /// Converts to TaskContext
    fn into_task_context(self) -> (TaskContext, TaskHandle);
}

/// NONE_INTO_TASK_CONTEXT is the abbreviation for an empty `IntoTaskContext`.
pub const NONE_INTO_TASK_CONTEXT: Option<(TaskContext, TaskHandle)> = None;

/// DEFAULT_INTO_TASK_CONTEXT is the abbreviation for the default `TaskContext` creater.
pub const DEFAULT_INTO_TASK_CONTEXT: Option<fn() -> (TaskContext, TaskHandle)> =
    Some(TaskContext::new);

impl<T: FnOnce() -> (TaskContext, TaskHandle)> IntoTaskContext for T {
    #[inline(always)]
    fn into_task_context(self) -> (TaskContext, TaskHandle) {
        self()
    }
}

impl IntoTaskContext for (TaskContext, TaskHandle) {
    #[inline(always)]
    fn into_task_context(self) -> (TaskContext, TaskHandle) {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    #[test]
    fn main() {
        assert_send::<UnifiedContext>();
        assert_sync::<UnifiedContext>();
        assert_send::<ContextGuard>();
        assert_sync::<ContextGuard>();
    }
}
