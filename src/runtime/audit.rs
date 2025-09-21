//! Runtime lifecycle audit utilities (RSB MODULE_SPEC compliant).
//!
//! This module provides lightweight instrumentation hooks so callers can observe
//! the major lifecycle transitions of `RoomRuntime`. Records capture a stage
//! identifier plus structured metadata so downstream code can log, buffer, or
//! visualize the runtime’s progression without contorting the core loop.

use std::time::SystemTime;

use serde_json::Value;

/// Distinct lifecycle checkpoints emitted by `RoomRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAuditStage {
    /// A new runtime instance was constructed.
    RuntimeConstructed,
    /// Bootstrap has started (plugins running their `init` hooks).
    BootstrapStarted,
    /// A plugin was registered with the runtime.
    PluginRegistered,
    /// A plugin finished initialising.
    PluginInitialized,
    /// A runtime event finished propagating.
    EventDispatched,
    /// Synthetic tick dispatched.
    TickDispatched,
    /// Rendering flushed dirty zones.
    RenderCommitted,
    /// Render attempt skipped because nothing was dirty.
    RenderSkipped,
    /// Runtime stopped (typically after exit request).
    RuntimeStopped,
}

/// Structured audit entry.
#[derive(Debug, Clone)]
pub struct RuntimeAuditEvent {
    pub timestamp: SystemTime,
    pub stage: RuntimeAuditStage,
    pub details: Vec<(String, Value)>,
}

impl RuntimeAuditEvent {
    fn new(stage: RuntimeAuditStage) -> Self {
        Self {
            timestamp: SystemTime::now(),
            stage,
            details: Vec::new(),
        }
    }
}

/// Builder helper to append fields ergonomically.
pub struct RuntimeAuditEventBuilder {
    event: RuntimeAuditEvent,
}

impl RuntimeAuditEventBuilder {
    pub fn new(stage: RuntimeAuditStage) -> Self {
        Self {
            event: RuntimeAuditEvent::new(stage),
        }
    }

    pub fn detail(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        self.event.details.push((key.into(), value));
        self
    }

    pub fn finish(self) -> RuntimeAuditEvent {
        self.event
    }
}

/// Trait implemented by any audit sink.
pub trait RuntimeAudit: Send + Sync {
    fn record(&self, event: RuntimeAuditEvent);
}

/// Default no-op implementation used when auditing is disabled.
#[derive(Debug, Default)]
pub struct NullRuntimeAudit;

impl RuntimeAudit for NullRuntimeAudit {
    fn record(&self, _event: RuntimeAuditEvent) {}
}
