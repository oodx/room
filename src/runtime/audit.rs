//! Runtime lifecycle audit utilities (RSB MODULE_SPEC compliant).
//!
//! This module provides lightweight instrumentation hooks so callers can observe
//! the major lifecycle transitions of `RoomRuntime`. Records capture a stage
//! identifier plus structured metadata so downstream code can log, buffer, or
//! visualize the runtime’s progression without contorting the core loop.

use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use serde_json::Value;

/// Distinct lifecycle checkpoints emitted by `RoomRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAuditStage {
    /// A new runtime instance was constructed.
    RuntimeConstructed,
    /// Driver is preparing terminal (raw mode, alternate screen).
    Open,
    /// Runtime boot sequence starting.
    Boot,
    /// Runtime setup finished pre-first render.
    Setup,
    /// First frame committed; user interaction can begin.
    UserReady,
    /// Event dispatch about to run.
    LoopIn,
    /// Event dispatch finished.
    LoopOut,
    /// User-requested session end in progress.
    UserEnd,
    /// Runtime cleanup prior to driver teardown.
    Cleanup,
    /// Driver exiting alternate screen / raw mode.
    End,
    /// Session fully closed.
    Close,
    /// Recoverable or fatal-error candidate detected.
    Error,
    /// Recovery attempted; payload indicates outcome.
    RecoverOrFatal,
    /// Fatal teardown initiated.
    Fatal,
    /// Fatal cleanup in progress.
    FatalCleanup,
    /// Fatal teardown completed.
    FatalClose,
    /// Cursor moved or changed characteristics.
    CursorMoved,
    /// Cursor became visible.
    CursorShown,
    /// Cursor became hidden.
    CursorHidden,
    /// Focus changed between zones/components.
    FocusChanged,
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

struct BootstrapAuditState {
    buffering: bool,
    buffer: Vec<RuntimeAuditEvent>,
}

impl BootstrapAuditState {
    fn new() -> Self {
        Self {
            buffering: true,
            buffer: Vec::new(),
        }
    }
}

/// Audit helper that buffers lifecycle events until bootstrap finishes the first render.
///
/// Certain demos (notably ones that print audit traces to stdout) can overwhelm the
/// terminal with bootstrap noise before any UI paints. This wrapper holds on to events
/// until the runtime emits `RuntimeAuditStage::RenderCommitted`, then flushes the buffer
/// so observers see a coherent first frame before the stream starts. If the runtime
/// exits before a render occurs, the buffer is flushed on `RuntimeAuditStage::RuntimeStopped`
/// so no events are lost.
pub struct BootstrapAudit {
    inner: Arc<dyn RuntimeAudit>,
    release_stage: RuntimeAuditStage,
    state: Mutex<BootstrapAuditState>,
}

impl BootstrapAudit {
    /// Create a new bootstrap audit helper that releases once the first render commits.
    pub fn new(inner: Arc<dyn RuntimeAudit>) -> Arc<Self> {
        Self::with_release_stage(inner, RuntimeAuditStage::RenderCommitted)
    }

    /// Create a bootstrap helper that flushes once the specified stage occurs.
    pub fn with_release_stage(
        inner: Arc<dyn RuntimeAudit>,
        release_stage: RuntimeAuditStage,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner,
            release_stage,
            state: Mutex::new(BootstrapAuditState::new()),
        })
    }

    /// Manually flush buffered events even if the release stage has not been seen.
    pub fn release(&self) {
        let buffered = {
            let mut state = self.state.lock().unwrap();
            if !state.buffering {
                return;
            }
            state.buffering = false;
            std::mem::take(&mut state.buffer)
        };

        for event in buffered {
            self.inner.record(event);
        }
    }
}

impl RuntimeAudit for BootstrapAudit {
    fn record(&self, event: RuntimeAuditEvent) {
        let mut state = self.state.lock().unwrap();
        if state.buffering {
            let stage = event.stage;
            state.buffer.push(event);
            if stage == self.release_stage || stage == RuntimeAuditStage::RuntimeStopped {
                state.buffering = false;
                let buffered = std::mem::take(&mut state.buffer);
                drop(state);
                for event in buffered {
                    self.inner.record(event);
                }
            }
            return;
        }

        drop(state);
        self.inner.record(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingAudit {
        stages: Mutex<Vec<RuntimeAuditStage>>,
    }

    impl RecordingAudit {
        fn snapshot(&self) -> Vec<RuntimeAuditStage> {
            self.stages.lock().unwrap().clone()
        }
    }

    impl RuntimeAudit for RecordingAudit {
        fn record(&self, event: RuntimeAuditEvent) {
            self.stages.lock().unwrap().push(event.stage);
        }
    }

    fn event(stage: RuntimeAuditStage) -> RuntimeAuditEvent {
        RuntimeAuditEventBuilder::new(stage).finish()
    }

    #[test]
    fn buffers_until_first_render_commits() {
        let sink = Arc::new(RecordingAudit::default());
        let audit = BootstrapAudit::new(sink.clone());
        audit.record(event(RuntimeAuditStage::BootstrapStarted));
        assert_eq!(sink.snapshot(), Vec::<RuntimeAuditStage>::new());

        audit.record(event(RuntimeAuditStage::RenderCommitted));
        assert_eq!(
            sink.snapshot(),
            vec![
                RuntimeAuditStage::BootstrapStarted,
                RuntimeAuditStage::RenderCommitted
            ]
        );
    }

    #[test]
    fn flushes_on_manual_release() {
        let sink = Arc::new(RecordingAudit::default());
        let audit = BootstrapAudit::new(sink.clone());
        audit.record(event(RuntimeAuditStage::BootstrapStarted));
        assert!(sink.snapshot().is_empty());

        audit.release();
        assert_eq!(sink.snapshot(), vec![RuntimeAuditStage::BootstrapStarted]);
    }

    #[test]
    fn release_stage_short_circuits_after_flush() {
        let sink = Arc::new(RecordingAudit::default());
        let audit = BootstrapAudit::new(sink.clone());
        audit.record(event(RuntimeAuditStage::BootstrapStarted));
        audit.record(event(RuntimeAuditStage::RenderCommitted));
        assert_eq!(sink.snapshot().len(), 2);

        audit.record(event(RuntimeAuditStage::TickDispatched));
        assert_eq!(
            sink.snapshot(),
            vec![
                RuntimeAuditStage::BootstrapStarted,
                RuntimeAuditStage::RenderCommitted,
                RuntimeAuditStage::TickDispatched,
            ]
        );
    }
}
