use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::json;

use crate::Result;
use crate::logging::{LogLevel, Logger, event_with_fields, json_kv};
use crate::metrics::RuntimeMetrics;

use super::{EventFlow, RoomPlugin, RuntimeContext, RuntimeEvent};

/// Logs high-level runtime lifecycle events for observability/debugging.
pub struct LifecycleLoggerPlugin {
    logger: Logger,
    level: LogLevel,
    log_keys: bool,
    log_mouse: bool,
    log_paste: bool,
    log_ticks: bool,
    log_raw: bool,
}

impl LifecycleLoggerPlugin {
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            level: LogLevel::Debug,
            log_keys: true,
            log_mouse: false,
            log_paste: true,
            log_ticks: false,
            log_raw: false,
        }
    }

    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    pub fn log_keys(mut self, enabled: bool) -> Self {
        self.log_keys = enabled;
        self
    }

    pub fn log_mouse(mut self, enabled: bool) -> Self {
        self.log_mouse = enabled;
        self
    }

    pub fn log_paste(mut self, enabled: bool) -> Self {
        self.log_paste = enabled;
        self
    }

    pub fn log_ticks(mut self, enabled: bool) -> Self {
        self.log_ticks = enabled;
        self
    }

    pub fn log_raw(mut self, enabled: bool) -> Self {
        self.log_raw = enabled;
        self
    }

    fn emit(&self, message: &str, fields: impl IntoIterator<Item = (String, serde_json::Value)>) {
        let event = event_with_fields(self.level, "room::runtime.lifecycle", message, fields);
        let _ = self.logger.log_event(event);
    }
}

impl RoomPlugin for LifecycleLoggerPlugin {
    fn name(&self) -> &str {
        "diagnostics.lifecycle_logger"
    }

    fn init(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.emit(
            "plugin_initialized",
            [json_kv("logger_level", json!(format!("{:?}", self.level)))],
        );
        Ok(())
    }

    fn on_event(
        &mut self,
        _ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        match event {
            RuntimeEvent::Key(key) if self.log_keys => {
                self.emit(
                    "event.key",
                    [
                        json_kv("code", json!(format!("{:?}", key.code))),
                        json_kv("modifiers", json!(format!("{:?}", key.modifiers))),
                        json_kv("kind", json!(format!("{:?}", key.kind))),
                    ],
                );
            }
            RuntimeEvent::Mouse(mouse_event) if self.log_mouse => {
                self.emit(
                    "event.mouse",
                    [json_kv("event", json!(format!("{:?}", mouse_event)))],
                );
            }
            RuntimeEvent::Paste(data) if self.log_paste => {
                self.emit(
                    "event.paste",
                    [json_kv("chars", json!(data.chars().count()))],
                );
            }
            RuntimeEvent::Tick { elapsed } if self.log_ticks => {
                self.emit(
                    "event.tick",
                    [json_kv("elapsed_ms", json!(elapsed.as_millis()))],
                );
            }
            RuntimeEvent::Raw(raw) if self.log_raw => {
                self.emit("event.raw", [json_kv("event", json!(format!("{:?}", raw)))])
            }
            RuntimeEvent::FocusGained => {
                self.emit("event.focus_gained", std::iter::empty());
            }
            RuntimeEvent::FocusLost => {
                self.emit("event.focus_lost", std::iter::empty());
            }
            RuntimeEvent::Resize(size) => {
                self.emit(
                    "event.resize",
                    [
                        json_kv("width", json!(size.width)),
                        json_kv("height", json!(size.height)),
                    ],
                );
            }
            _ => {}
        }

        Ok(EventFlow::Continue)
    }
}

/// Periodically emits runtime metrics snapshots through the provided logger.
pub struct MetricsSnapshotPlugin {
    logger: Logger,
    metrics: Arc<Mutex<RuntimeMetrics>>,
    target: String,
    interval: Duration,
    last_emit: Option<Instant>,
    started_at: Instant,
}

impl MetricsSnapshotPlugin {
    pub fn new(logger: Logger, metrics: Arc<Mutex<RuntimeMetrics>>) -> Self {
        Self {
            logger,
            metrics,
            target: "room::runtime.metrics".to_string(),
            interval: Duration::from_secs(5),
            last_emit: None,
            started_at: Instant::now(),
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    fn emit_snapshot(&mut self) {
        if self.interval == Duration::from_millis(0) {
            return;
        }

        let now = Instant::now();
        if let Some(last) = self.last_emit {
            if now.duration_since(last) < self.interval {
                return;
            }
        }

        self.last_emit = Some(now);
        let uptime = now.duration_since(self.started_at);

        if let Ok(guard) = self.metrics.lock() {
            let event = guard.snapshot(uptime).to_log_event(&self.target);
            let _ = self.logger.log_event(event);
        }
    }
}

impl RoomPlugin for MetricsSnapshotPlugin {
    fn name(&self) -> &str {
        "diagnostics.metrics_snapshot"
    }

    fn init(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.started_at = Instant::now();
        self.last_emit = None;
        Ok(())
    }

    fn before_render(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.emit_snapshot();
        Ok(())
    }

    fn on_event(
        &mut self,
        _ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        if matches!(event, RuntimeEvent::Tick { .. }) {
            self.emit_snapshot();
        }
        Ok(EventFlow::Continue)
    }
}
