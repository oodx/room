use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use serde_json::json;

use crate::logging::{event_with_fields, json_kv};
use crate::{
    AnsiRenderer, LayoutTree, LogLevel, Logger, Rect, Result, RuntimeMetrics, Size, ZoneRegistry,
};

pub mod diagnostics;

/// Configuration knobs for the runtime loop.
#[derive(Clone)]
pub struct RuntimeConfig {
    /// Interval between synthetic tick events.
    pub tick_interval: Duration,
    /// Optional structured logger used by the runtime.
    pub logger: Option<Logger>,
    /// Metrics accumulator used for periodic snapshots.
    pub metrics: Option<Arc<Mutex<RuntimeMetrics>>>,
    /// Interval between metrics snapshot emissions. Zero disables snapshots.
    pub metrics_interval: Duration,
    /// Target field used when emitting metrics snapshots.
    pub metrics_target: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_millis(200),
            logger: None,
            metrics: None,
            metrics_interval: Duration::from_secs(5),
            metrics_target: "room::runtime.metrics".to_string(),
        }
    }
}

impl RuntimeConfig {
    /// Enable metrics collection if it has not already been configured.
    pub fn enable_metrics(&mut self) {
        if self.metrics.is_none() {
            self.metrics = Some(Arc::new(Mutex::new(RuntimeMetrics::new())));
        }
    }

    /// Disable metrics collection and prevent further snapshots.
    pub fn disable_metrics(&mut self) {
        self.metrics = None;
    }

    /// Access the shared metrics handle if metrics are enabled.
    pub fn metrics_handle(&self) -> Option<Arc<Mutex<RuntimeMetrics>>> {
        self.metrics.as_ref().map(Arc::clone)
    }
}

/// High-level events delivered to plugins.
#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    Tick { elapsed: Duration },
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(String),
    FocusGained,
    FocusLost,
    Resize(Size),
    Raw(CrosstermEvent),
}

/// Control the propagation of an event across plugins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventFlow {
    Continue,
    Consumed,
}

/// Context passed to plugins so they can interact with the runtime safely.
pub struct RuntimeContext<'a> {
    rects: &'a HashMap<String, Rect>,
    zone_updates: Vec<(String, String)>,
    redraw_requested: bool,
    exit_requested: bool,
    cursor_hint: Option<(u16, u16)>,
}

impl<'a> RuntimeContext<'a> {
    fn new(rects: &'a HashMap<String, Rect>) -> Self {
        Self {
            rects,
            zone_updates: Vec::new(),
            redraw_requested: false,
            exit_requested: false,
            cursor_hint: None,
        }
    }

    /// Queue new content for a zone. The update is applied after the plugin completes.
    pub fn set_zone(&mut self, zone_id: impl Into<String>, content: impl Into<String>) {
        self.zone_updates.push((zone_id.into(), content.into()));
        self.redraw_requested = true;
    }

    /// Request that the renderer runs even if no zones changed.
    pub fn request_render(&mut self) {
        self.redraw_requested = true;
    }

    /// Signal to the runtime that execution should terminate at the end of the frame.
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    /// Provide a hint for where the cursor should be restored after rendering.
    pub fn set_cursor_hint(&mut self, row: u16, col: u16) {
        self.cursor_hint = Some((row, col));
    }

    /// Fetch the solved rectangle for a zone if available.
    pub fn rect(&self, zone_id: &str) -> Option<&Rect> {
        self.rects.get(zone_id)
    }

    fn into_outcome(self) -> ContextOutcome {
        ContextOutcome {
            zone_updates: self.zone_updates,
            redraw_requested: self.redraw_requested,
            exit_requested: self.exit_requested,
            cursor_hint: self.cursor_hint,
        }
    }
}

struct ContextOutcome {
    zone_updates: Vec<(String, String)>,
    redraw_requested: bool,
    exit_requested: bool,
    cursor_hint: Option<(u16, u16)>,
}

/// Behaviour injection point for the runtime.
pub trait RoomPlugin: Send {
    fn name(&self) -> &str {
        "room_plugin"
    }

    fn init(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_event(
        &mut self,
        _ctx: &mut RuntimeContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        Ok(EventFlow::Continue)
    }

    fn before_render(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }

    fn after_render(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }
}

pub struct RoomRuntime {
    layout: LayoutTree,
    rects: HashMap<String, Rect>,
    registry: ZoneRegistry,
    renderer: AnsiRenderer,
    plugins: Vec<Box<dyn RoomPlugin>>,
    config: RuntimeConfig,
    should_exit: bool,
    redraw_requested: bool,
    start_instant: Option<Instant>,
    last_metrics_emit: Option<Instant>,
}

impl RoomRuntime {
    pub fn new(layout: LayoutTree, renderer: AnsiRenderer, initial_size: Size) -> Result<Self> {
        let mut registry = ZoneRegistry::new();
        let rects = layout.solve(initial_size)?;
        registry.sync_layout(&rects);

        Ok(Self {
            layout,
            rects,
            registry,
            renderer,
            plugins: Vec::new(),
            config: RuntimeConfig::default(),
            should_exit: false,
            redraw_requested: true,
            start_instant: None,
            last_metrics_emit: None,
        })
    }

    pub fn config_mut(&mut self) -> &mut RuntimeConfig {
        &mut self.config
    }

    pub fn register_plugin<P>(&mut self, plugin: P)
    where
        P: RoomPlugin + 'static,
    {
        self.plugins.push(Box::new(plugin));
    }

    pub fn run(&mut self, stdout: &mut impl Write) -> Result<()> {
        self.bootstrap(stdout)?;
        let mut last_tick = Instant::now();

        while !self.should_exit {
            let timeout = self
                .config
                .tick_interval
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_millis(0));

            if event::poll(timeout)? {
                let crossterm_event = event::read()?;
                let runtime_event = self.map_event(crossterm_event)?;
                self.dispatch_event(runtime_event)?;
                self.render_if_needed(stdout)?;
                if self.should_exit {
                    break;
                }
            }

            if last_tick.elapsed() >= self.config.tick_interval {
                let now = Instant::now();
                let elapsed = now.duration_since(last_tick);
                last_tick = now;
                self.dispatch_event(RuntimeEvent::Tick { elapsed })?;
                self.render_if_needed(stdout)?;
            }

            self.maybe_emit_metrics();
        }

        self.finalize();
        Ok(())
    }

    pub fn run_scripted<I>(&mut self, stdout: &mut impl Write, events: I) -> Result<()>
    where
        I: IntoIterator<Item = RuntimeEvent>,
    {
        self.bootstrap(stdout)?;
        for event in events.into_iter() {
            let event = match event {
                RuntimeEvent::Resize(size) => {
                    self.handle_resize(size)?;
                    RuntimeEvent::Resize(size)
                }
                other => other,
            };
            self.dispatch_event(event)?;
            self.render_if_needed(stdout)?;
            if self.should_exit {
                break;
            }
        }
        self.finalize();
        Ok(())
    }

    fn dispatch_event(&mut self, event: RuntimeEvent) -> Result<()> {
        let mut consumed = false;
        for idx in 0..self.plugins.len() {
            let (flow, outcome) = {
                let plugin = &mut self.plugins[idx];
                let mut ctx = RuntimeContext::new(&self.rects);
                let flow = plugin.on_event(&mut ctx, &event)?;
                (flow, ctx.into_outcome())
            };
            self.apply_outcome(outcome)?;
            if matches!(flow, EventFlow::Consumed) {
                consumed = true;
                break;
            }
        }
        self.record_event_metric();
        self.log_runtime_event(
            LogLevel::Debug,
            "event_dispatched",
            [
                json_kv("event", json!(Self::describe_event(&event))),
                json_kv("consumed", json!(consumed)),
            ],
        );
        self.maybe_emit_metrics();
        Ok(())
    }

    fn render_if_needed(&mut self, stdout: &mut impl Write) -> Result<()> {
        if !self.redraw_requested {
            return Ok(());
        }

        self.redraw_requested = false;

        for idx in 0..self.plugins.len() {
            let outcome = {
                let plugin = &mut self.plugins[idx];
                let mut ctx = RuntimeContext::new(&self.rects);
                plugin.before_render(&mut ctx)?;
                ctx.into_outcome()
            };
            self.apply_outcome(outcome)?;
        }

        let dirty = self.registry.take_dirty();
        if !dirty.is_empty() {
            self.renderer.render(stdout, &dirty)?;
            self.record_render_metric(dirty.len());
            self.log_runtime_event(
                LogLevel::Debug,
                "render_completed",
                [json_kv("dirty_zones", json!(dirty.len()))],
            );
        }

        for idx in 0..self.plugins.len() {
            let outcome = {
                let plugin = &mut self.plugins[idx];
                let mut ctx = RuntimeContext::new(&self.rects);
                plugin.after_render(&mut ctx)?;
                ctx.into_outcome()
            };
            self.apply_outcome(outcome)?;
        }

        if self.registry.has_dirty() {
            self.redraw_requested = true;
        }

        Ok(())
    }

    fn apply_outcome(&mut self, outcome: ContextOutcome) -> Result<()> {
        let ContextOutcome {
            zone_updates,
            redraw_requested,
            exit_requested,
            cursor_hint,
        } = outcome;

        let update_count = zone_updates.len();
        if update_count > 0 {
            for (zone, content) in zone_updates {
                self.registry.apply_content(&zone, content)?;
            }
            self.record_zone_updates_metric(update_count);
            self.redraw_requested = true;
        }

        if redraw_requested {
            self.redraw_requested = true;
        }

        if let Some(cursor) = cursor_hint {
            self.renderer.settings_mut().restore_cursor = Some(cursor);
        }

        if exit_requested {
            self.should_exit = true;
            self.log_runtime_event(LogLevel::Info, "exit_requested", std::iter::empty());
        }

        Ok(())
    }

    fn map_event(&mut self, event: CrosstermEvent) -> Result<RuntimeEvent> {
        match event {
            CrosstermEvent::Key(key) => Ok(RuntimeEvent::Key(key)),
            CrosstermEvent::Mouse(mouse) => Ok(RuntimeEvent::Mouse(mouse)),
            CrosstermEvent::Paste(data) => Ok(RuntimeEvent::Paste(data)),
            CrosstermEvent::FocusGained => Ok(RuntimeEvent::FocusGained),
            CrosstermEvent::FocusLost => Ok(RuntimeEvent::FocusLost),
            CrosstermEvent::Resize(width, height) => {
                let size = Size::new(width, height);
                self.handle_resize(size)?;
                Ok(RuntimeEvent::Resize(size))
            }
        }
    }

    fn handle_resize(&mut self, size: Size) -> Result<()> {
        let rects = self.layout.solve(size)?;
        self.rects = rects;
        self.registry.sync_layout(&self.rects);
        self.redraw_requested = true;
        self.log_runtime_event(
            LogLevel::Info,
            "resized",
            [
                json_kv("width", json!(size.width)),
                json_kv("height", json!(size.height)),
            ],
        );
        Ok(())
    }

    fn bootstrap(&mut self, stdout: &mut impl Write) -> Result<()> {
        self.should_exit = false;
        self.redraw_requested = true;
        self.ensure_metrics_initialized();
        let now = Instant::now();
        self.start_instant = Some(now);
        self.last_metrics_emit = Some(now);
        self.log_runtime_event(
            LogLevel::Info,
            "runtime_started",
            [
                json_kv("plugins", json!(self.plugins.len())),
                json_kv("zones", json!(self.rects.len())),
            ],
        );

        for idx in 0..self.plugins.len() {
            let outcome = {
                let plugin = &mut self.plugins[idx];
                let plugin_name = plugin.name().to_string();
                let mut ctx = RuntimeContext::new(&self.rects);
                plugin.init(&mut ctx)?;
                self.log_runtime_event(
                    LogLevel::Debug,
                    "plugin_initialized",
                    [json_kv("plugin", json!(plugin_name))],
                );
                ctx.into_outcome()
            };
            self.apply_outcome(outcome)?;
        }

        self.render_if_needed(stdout)
    }

    fn finalize(&mut self) {
        let uptime_ms = self
            .start_instant
            .map(|start| start.elapsed().as_millis())
            .unwrap_or(0);
        self.log_runtime_event(
            LogLevel::Info,
            "runtime_stopped",
            [json_kv("uptime_ms", json!(uptime_ms))],
        );
    }

    fn ensure_metrics_initialized(&mut self) {
        if self.config.metrics.is_none() && self.config.metrics_interval > Duration::from_millis(0)
        {
            self.config.metrics = Some(Arc::new(Mutex::new(RuntimeMetrics::new())));
        }
    }

    fn log_runtime_event<I>(&self, level: LogLevel, message: &str, fields: I)
    where
        I: IntoIterator<Item = (String, serde_json::Value)>,
    {
        if let Some(logger) = self.config.logger.as_ref() {
            let event = event_with_fields(level, "room::runtime", message, fields);
            let _ = logger.log_event(event);
        }
    }

    fn record_event_metric(&mut self) {
        if let Some(metrics) = self.config.metrics.as_ref() {
            if let Ok(mut guard) = metrics.lock() {
                guard.record_event();
            }
        }
    }

    fn record_render_metric(&mut self, dirty_count: usize) {
        if let Some(metrics) = self.config.metrics.as_ref() {
            if let Ok(mut guard) = metrics.lock() {
                guard.record_render(dirty_count);
            }
        }
    }

    fn record_zone_updates_metric(&mut self, count: usize) {
        if let Some(metrics) = self.config.metrics.as_ref() {
            if let Ok(mut guard) = metrics.lock() {
                guard.record_zone_updates(count);
            }
        }
    }

    fn maybe_emit_metrics(&mut self) {
        if self.config.metrics.is_none() {
            return;
        }

        if self.config.metrics_interval == Duration::from_millis(0) {
            return;
        }

        let now = Instant::now();
        match self.last_metrics_emit {
            Some(last) if now.duration_since(last) < self.config.metrics_interval => {
                return;
            }
            _ => {
                self.last_metrics_emit = Some(now);
            }
        }

        let uptime = self
            .start_instant
            .map(|start| now.duration_since(start))
            .unwrap_or_default();

        if let (Some(logger), Some(metrics)) =
            (self.config.logger.as_ref(), self.config.metrics.as_ref())
        {
            if let Ok(guard) = metrics.lock() {
                let target = self.config.metrics_target.as_str();
                let snapshot_event = guard.snapshot(uptime).to_log_event(target);
                let _ = logger.log_event(snapshot_event);
            }
        }
    }

    fn describe_event(event: &RuntimeEvent) -> &'static str {
        match event {
            RuntimeEvent::Tick { .. } => "tick",
            RuntimeEvent::Key(_) => "key",
            RuntimeEvent::Mouse(_) => "mouse",
            RuntimeEvent::Paste(_) => "paste",
            RuntimeEvent::FocusGained => "focus_gained",
            RuntimeEvent::FocusLost => "focus_lost",
            RuntimeEvent::Resize(_) => "resize",
            RuntimeEvent::Raw(_) => "raw",
        }
    }
}
