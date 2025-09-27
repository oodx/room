use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use boxy::api::layout::BoxBuilder;
use boxy::visual::{BoxStyle, NORMAL};
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use serde_json::json;

use self::audit::{NullRuntimeAudit, RuntimeAudit, RuntimeAuditEventBuilder, RuntimeAuditStage};
use self::focus::{FocusController, FocusEntry, ensure_focus_registry};
use self::screens::{ScreenActivation, ScreenManager};
use crate::logging::{event_with_fields, json_kv};
use crate::{
    AnsiRenderer, Layout, LayoutError, LogLevel, Logger, Rect, Result, RuntimeMetrics, Size,
    ZoneRegistry,
};
pub mod audit;
pub mod bundles;
pub mod diagnostics;
pub mod driver;
pub mod focus;
pub mod screens;
pub mod shared_state;

const RUNTIME_FOCUS_OWNER: &str = "room::runtime";

pub struct PluginBundle {
    entries: Vec<PluginEntry>,
}

impl PluginBundle {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn with_plugin<P>(mut self, plugin: P, priority: i32) -> Self
    where
        P: RoomPlugin + 'static,
    {
        let name = plugin.name().to_string();
        self.entries.push(PluginEntry {
            name,
            priority,
            plugin: Box::new(plugin),
        });
        self
    }

    pub fn register_into(self, runtime: &mut RoomRuntime) {
        for entry in self.entries {
            runtime.register_plugin_with_entry(entry);
        }
    }
}

struct PluginEntry {
    name: String,
    priority: i32,
    plugin: Box<dyn RoomPlugin>,
}

/// Configuration for a bounded simulated loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulatedLoop {
    /// Maximum number of iterations to execute before the runtime tears down.
    pub max_iterations: usize,
    /// Whether to emit synthetic tick events each iteration.
    pub dispatch_ticks: bool,
}

impl SimulatedLoop {
    /// Create a loop that runs for `max_iterations` iterations emitting ticks.
    pub fn ticks(max_iterations: usize) -> Self {
        Self {
            max_iterations,
            dispatch_ticks: true,
        }
    }

    /// Create a loop that runs for `max_iterations` iterations without ticks.
    pub fn silent(max_iterations: usize) -> Self {
        Self {
            max_iterations,
            dispatch_ticks: false,
        }
    }
}

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
    /// Optional audit sink for lifecycle instrumentation.
    pub audit: Option<Arc<dyn RuntimeAudit>>,
    /// Zone that should receive focus automatically once bootstrap completes.
    pub default_focus_zone: Option<String>,
    /// Optional loop guard for the driver event loop.
    pub loop_iteration_limit: Option<usize>,
    /// When present, bypasses the driver loop and uses a bounded simulated loop.
    pub simulated_loop: Option<SimulatedLoop>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_millis(200),
            logger: None,
            metrics: None,
            metrics_interval: Duration::from_secs(5),
            metrics_target: "room::runtime.metrics".to_string(),
            audit: None,
            default_focus_zone: None,
            loop_iteration_limit: None,
            simulated_loop: None,
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
    Open,
    Boot,
    Setup,
    UserReady,
    LoopIn { kind: LoopEventKind },
    LoopOut { kind: LoopEventKind, consumed: bool },
    UserEnd,
    Cleanup,
    End,
    Close,
    Error(RuntimeError),
    RecoverOrFatal { recovered: bool },
    Fatal,
    FatalCleanup,
    FatalClose,
    CursorMoved(Cursor),
    CursorShown(Cursor),
    CursorHidden(Cursor),
    FocusChanged(FocusChange),
    Tick { elapsed: Duration },
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(String),
    FocusGained,
    FocusLost,
    Resize(Size),
    Raw(CrosstermEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopEventKind {
    Tick,
    Key,
    Mouse,
    Paste,
    FocusGained,
    FocusLost,
    Resize,
    Raw,
}

impl LoopEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoopEventKind::Tick => "tick",
            LoopEventKind::Key => "key",
            LoopEventKind::Mouse => "mouse",
            LoopEventKind::Paste => "paste",
            LoopEventKind::FocusGained => "focus_gained",
            LoopEventKind::FocusLost => "focus_lost",
            LoopEventKind::Resize => "resize",
            LoopEventKind::Raw => "raw",
        }
    }

    pub fn from_runtime_event(event: &RuntimeEvent) -> Option<Self> {
        match event {
            RuntimeEvent::Tick { .. } => Some(LoopEventKind::Tick),
            RuntimeEvent::Key(_) => Some(LoopEventKind::Key),
            RuntimeEvent::Mouse(_) => Some(LoopEventKind::Mouse),
            RuntimeEvent::Paste(_) => Some(LoopEventKind::Paste),
            RuntimeEvent::FocusGained => Some(LoopEventKind::FocusGained),
            RuntimeEvent::FocusLost => Some(LoopEventKind::FocusLost),
            RuntimeEvent::Resize(_) => Some(LoopEventKind::Resize),
            RuntimeEvent::Raw(_) => Some(LoopEventKind::Raw),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub category: String,
    pub source: Option<String>,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CursorStyle {
    pub fg_color: Option<String>,
    pub bg_color: Option<String>,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub position: (u16, u16),
    pub visible: bool,
    pub char: Option<char>,
    pub style: Option<CursorStyle>,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            position: (0, 0),
            visible: true,
            char: None,
            style: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CursorEvent {
    Moved(Cursor),
    Shown(Cursor),
    Hidden(Cursor),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusTarget {
    pub owner: Option<String>,
    pub zone: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusChange {
    pub from: Option<FocusTarget>,
    pub to: Option<FocusTarget>,
}

/// Control the propagation of an event across plugins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventFlow {
    Continue,
    Consumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollapseMode {
    Hide,
    Show,
}

#[derive(Clone, Copy)]
pub struct BoxConfig {
    pub style: &'static BoxStyle,
    pub min_width: u16,
    pub min_height: u16,
    pub collapse_mode: CollapseMode,
}

impl BoxConfig {
    pub fn new(style: &'static BoxStyle) -> Self {
        Self {
            style,
            min_width: 10,
            min_height: 3,
            collapse_mode: CollapseMode::Show,
        }
    }

    pub fn with_style(mut self, style: &'static BoxStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_min_width(mut self, min_width: u16) -> Self {
        self.min_width = min_width;
        self
    }

    pub fn with_min_height(mut self, min_height: u16) -> Self {
        self.min_height = min_height;
        self
    }

    pub fn with_collapse_mode(mut self, mode: CollapseMode) -> Self {
        self.collapse_mode = mode;
        self
    }
}

impl Default for BoxConfig {
    fn default() -> Self {
        Self::new(&NORMAL)
    }
}

/// Context passed to plugins so they can interact with the runtime safely.
pub struct RuntimeContext<'a> {
    rects: &'a HashMap<String, Rect>,
    shared_state: &'a shared_state::SharedState,
    zone_updates: Vec<ZoneUpdate>,
    redraw_requested: bool,
    exit_requested: bool,
    cursor_hint: Option<(u16, u16)>,
    cursor_update: CursorUpdate,
    reported_error: Option<RuntimeError>,
}

impl<'a> RuntimeContext<'a> {
    fn new(rects: &'a HashMap<String, Rect>, shared_state: &'a shared_state::SharedState) -> Self {
        Self {
            rects,
            shared_state,
            zone_updates: Vec::new(),
            redraw_requested: false,
            exit_requested: false,
            cursor_hint: None,
            cursor_update: CursorUpdate::default(),
            reported_error: None,
        }
    }

    /// Queue new content for a zone. The update is applied after the plugin completes.
    pub fn set_zone(&mut self, zone_id: impl Into<String>, content: impl Into<String>) {
        self.zone_updates.push(ZoneUpdate {
            zone: zone_id.into(),
            content: content.into(),
            pre_rendered: false,
        });
        self.redraw_requested = true;
    }

    /// Queue pre-rendered content for a zone. The renderer will blit it verbatim.
    pub fn set_zone_pre_rendered(
        &mut self,
        zone_id: impl Into<String>,
        content: impl Into<String>,
    ) {
        self.zone_updates.push(ZoneUpdate {
            zone: zone_id.into(),
            content: content.into(),
            pre_rendered: true,
        });
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
        self.cursor_update.position = Some((row, col));
    }

    /// Position the cursor at the provided absolute coordinates.
    pub fn set_cursor_position(&mut self, row: u16, col: u16) {
        self.set_cursor_hint(row, col);
    }

    /// Position the cursor relative to a zone (row/col offsets within the zone).
    pub fn set_cursor_in_zone(&mut self, zone_id: &str, row_offset: i32, col_offset: i32) {
        if let Some(rect) = self.rect(zone_id) {
            let row_min = rect.y as i32;
            let row_max = row_min + rect.height as i32 - 1;
            let col_min = rect.x as i32;
            let col_max = col_min + rect.width as i32 - 1;
            if row_max >= row_min && col_max >= col_min {
                let row = (row_min + row_offset).clamp(row_min, row_max);
                let col = (col_min + col_offset).clamp(col_min, col_max);
                if row >= 0 && col >= 0 {
                    self.set_cursor_hint(row as u16, col as u16);
                }
            }
        }
    }

    /// Hide the terminal cursor after the next render.
    pub fn hide_cursor(&mut self) {
        self.cursor_update.visible = Some(false);
    }

    /// Show the terminal cursor after the next render.
    pub fn show_cursor(&mut self) {
        self.cursor_update.visible = Some(true);
    }

    /// Override the glyph (caret) rendered for the cursor.
    pub fn set_cursor_char(&mut self, ch: Option<char>) {
        self.cursor_update.char = Some(ch);
    }

    /// Apply a style to the cursor.
    pub fn set_cursor_style(&mut self, style: Option<CursorStyle>) {
        self.cursor_update.style = Some(style);
    }

    /// Report a runtime error. The runtime will attempt recovery before escalating.
    pub fn report_error(&mut self, error: RuntimeError) {
        self.reported_error = Some(error);
    }

    /// Fetch the solved rectangle for a zone if available.
    pub fn rect(&self, zone_id: &str) -> Option<&Rect> {
        self.rects.get(zone_id)
    }

    pub fn render_zone_with_box(
        &self,
        zone_id: &str,
        content: impl AsRef<str>,
        config: BoxConfig,
    ) -> Option<String> {
        let rect = self.rect(zone_id)?;

        if rect.width < config.min_width || rect.height < config.min_height {
            match config.collapse_mode {
                CollapseMode::Hide => return None,
                CollapseMode::Show => {
                    if rect.width < 5 || rect.height < 3 {
                        return None;
                    }

                    let collapsed_content = "...";
                    let box_layout = BoxBuilder::new(collapsed_content)
                        .with_fixed_width((rect.width as usize).max(1))
                        .with_fixed_height((rect.height as usize).max(1))
                        .with_style(*config.style)
                        .build();

                    return Some(box_layout.render());
                }
            }
        }

        let box_layout = BoxBuilder::new(content.as_ref())
            .with_fixed_width((rect.width as usize).max(1))
            .with_fixed_height((rect.height as usize).max(1))
            .with_style(*config.style)
            .build();

        Some(box_layout.render())
    }

    /// Access shared state resources by type.
    pub fn shared<T>(&self) -> std::result::Result<Arc<T>, shared_state::SharedStateError>
    where
        T: Send + Sync + 'static,
    {
        self.shared_state.get::<T>()
    }

    /// Lazily initialize a shared resource.
    pub fn shared_init<T, F>(
        &self,
        make: F,
    ) -> std::result::Result<Arc<T>, shared_state::SharedStateError>
    where
        T: Send + Sync + 'static,
        F: FnOnce() -> T,
    {
        self.shared_state.get_or_insert_with(make)
    }

    fn into_outcome(self) -> ContextOutcome {
        ContextOutcome {
            zone_updates: self.zone_updates,
            redraw_requested: self.redraw_requested,
            exit_requested: self.exit_requested,
            cursor_hint: self.cursor_hint,
            cursor_update: self.cursor_update,
            error: self.reported_error,
        }
    }
}

struct ContextOutcome {
    zone_updates: Vec<ZoneUpdate>,
    redraw_requested: bool,
    exit_requested: bool,
    cursor_hint: Option<(u16, u16)>,
    cursor_update: CursorUpdate,
    error: Option<RuntimeError>,
}

struct ZoneUpdate {
    zone: String,
    content: String,
    pre_rendered: bool,
}

#[derive(Default)]
struct CursorUpdate {
    position: Option<(u16, u16)>,
    visible: Option<bool>,
    char: Option<Option<char>>,
    style: Option<Option<CursorStyle>>,
}

impl CursorUpdate {
    fn is_empty(&self) -> bool {
        self.position.is_none()
            && self.visible.is_none()
            && self.char.is_none()
            && self.style.is_none()
    }
}

#[derive(Default)]
struct CursorManager {
    current: Cursor,
}

impl CursorManager {
    fn new() -> Self {
        Self {
            current: Cursor::default(),
        }
    }

    fn apply_update(&mut self, update: &CursorUpdate) -> Vec<CursorEvent> {
        if update.is_empty() {
            return Vec::new();
        }

        let mut next = self.current.clone();

        if let Some(position) = update.position {
            next.position = position;
        }

        if let Some(visible) = update.visible {
            next.visible = visible;
        }

        if let Some(char_update) = update.char.clone() {
            next.char = char_update;
        }

        if let Some(style_update) = update.style.clone() {
            next.style = style_update;
        }

        let mut events = Vec::new();

        if next.position != self.current.position || update.char.is_some() || update.style.is_some()
        {
            events.push(CursorEvent::Moved(next.clone()));
        }

        if !self.current.visible && next.visible {
            events.push(CursorEvent::Shown(next.clone()));
        } else if self.current.visible && !next.visible {
            events.push(CursorEvent::Hidden(next.clone()));
        }

        self.current = next;
        events
    }
}

/// Behaviour injection point for the runtime.
pub trait RoomPlugin: Send {
    fn name(&self) -> &str {
        "room_plugin"
    }

    fn on_boot(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_setup(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_user_ready(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_user_end(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_cleanup(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_close(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_error(&mut self, _ctx: &mut RuntimeContext<'_>, _error: &mut RuntimeError) -> Result<()> {
        Ok(())
    }

    fn on_recover_or_fatal(
        &mut self,
        _ctx: &mut RuntimeContext<'_>,
        _error: &RuntimeError,
        _recovered: bool,
    ) -> Result<()> {
        Ok(())
    }

    fn on_fatal(&mut self, _ctx: &mut RuntimeContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_cursor_event(
        &mut self,
        _ctx: &mut RuntimeContext<'_>,
        _event: &CursorEvent,
    ) -> Result<()> {
        Ok(())
    }

    fn on_focus_change(
        &mut self,
        _ctx: &mut RuntimeContext<'_>,
        _change: &FocusChange,
    ) -> Result<()> {
        Ok(())
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
    layout: Box<dyn Layout>,
    rects: HashMap<String, Rect>,
    registry: ZoneRegistry,
    renderer: AnsiRenderer,
    plugins: Vec<PluginEntry>,
    config: RuntimeConfig,
    should_exit: bool,
    redraw_requested: bool,
    start_instant: Option<Instant>,
    last_metrics_emit: Option<Instant>,
    shared_state: shared_state::SharedState,
    audit: Arc<dyn RuntimeAudit>,
    current_size: Size,
    screen_manager: Option<ScreenManager>,
    user_ready_emitted: bool,
    user_end_emitted: bool,
    cursor_manager: CursorManager,
    pending_cursor_events: Vec<CursorEvent>,
    pending_focus_changes: Vec<FocusChange>,
    pending_errors: Vec<RuntimeError>,
    last_focus_entry: Option<FocusEntry>,
    fatal_active: bool,
}

impl RoomRuntime {
    pub fn new(layout: impl Layout + 'static, renderer: AnsiRenderer, initial_size: Size) -> Result<Self> {
        Self::with_config(layout, renderer, initial_size, RuntimeConfig::default())
    }

    pub fn with_config(
        layout: impl Layout + 'static,
        renderer: AnsiRenderer,
        initial_size: Size,
        config: RuntimeConfig,
    ) -> Result<Self> {
        let mut registry = ZoneRegistry::new();
        let rects = layout.solve(initial_size)?;
        registry.sync_layout(&rects);

        let audit = config
            .audit
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Arc::new(NullRuntimeAudit));

        let runtime = Self {
            layout: Box::new(layout),
            rects,
            registry,
            renderer,
            plugins: Vec::new(),
            config,
            should_exit: false,
            redraw_requested: true,
            start_instant: None,
            last_metrics_emit: None,
            shared_state: shared_state::SharedState::new(),
            audit,
            current_size: initial_size,
            screen_manager: None,
            user_ready_emitted: false,
            user_end_emitted: false,
            cursor_manager: CursorManager::new(),
            pending_cursor_events: Vec::new(),
            pending_focus_changes: Vec::new(),
            pending_errors: Vec::new(),
            last_focus_entry: None,
            fatal_active: false,
        };
        runtime.audit_record(RuntimeAuditStage::RuntimeConstructed, []);
        Ok(runtime)
    }

    pub fn config_mut(&mut self) -> &mut RuntimeConfig {
        &mut self.config
    }

    /// Attach a screen manager so callers can orchestrate multi-screen flows.
    pub fn set_screen_manager(&mut self, manager: ScreenManager) {
        self.screen_manager = Some(manager);
    }

    /// Access the installed screen manager (mutable), if present.
    pub fn screen_manager_mut(&mut self) -> Option<&mut ScreenManager> {
        self.screen_manager.as_mut()
    }

    /// Activate a screen by id using the installed screen manager.
    pub fn activate_screen(&mut self, screen_id: &str) -> Result<()> {
        let mut manager = self
            .screen_manager
            .take()
            .ok_or_else(|| LayoutError::Backend("screen manager not installed".to_string()))?;
        let activation = manager.activate(screen_id)?;
        let result = manager.finish_activation(self, activation);
        self.screen_manager = Some(manager);
        result
    }

    pub fn register_plugin<P>(&mut self, plugin: P)
    where
        P: RoomPlugin + 'static,
    {
        self.register_plugin_with_priority(plugin, 0);
    }

    pub fn register_plugin_with_priority<P>(&mut self, plugin: P, priority: i32)
    where
        P: RoomPlugin + 'static,
    {
        let plugin_name = plugin.name().to_string();
        self.register_plugin_with_entry(PluginEntry {
            name: plugin_name.clone(),
            priority,
            plugin: Box::new(plugin),
        });
        let mut builder = RuntimeAuditEventBuilder::new(RuntimeAuditStage::PluginRegistered);
        builder
            .detail("plugin", json!(plugin_name))
            .detail("priority", json!(priority));
        self.audit_record_event(builder.finish());
    }

    pub fn register_bundle(&mut self, bundle: PluginBundle) {
        bundle.register_into(self);
    }

    pub fn resize(&mut self, size: Size) -> Result<()> {
        self.handle_resize(size)
    }

    pub fn signal_open(&mut self) {
        self.audit_record(RuntimeAuditStage::Open, []);
        self.log_lifecycle_stage("open");
    }

    pub fn signal_end(&mut self) {
        self.audit_record(RuntimeAuditStage::End, []);
        self.log_lifecycle_stage("end");
    }

    pub fn signal_close(&mut self) -> Result<()> {
        if self.fatal_active {
            self.audit_record(RuntimeAuditStage::FatalClose, []);
            self.log_lifecycle_stage("fatal_close");
        }
        self.audit_record(RuntimeAuditStage::Close, []);
        self.log_lifecycle_stage("close");
        self.notify_plugins(|plugin, ctx| plugin.on_close(ctx))
    }

    /// Obtain a handle to the shared state map managed by the runtime.
    pub fn shared_state_handle(&self) -> shared_state::SharedState {
        self.shared_state.clone()
    }

    fn register_plugin_with_entry(&mut self, entry: PluginEntry) {
        if self
            .plugins
            .iter()
            .any(|existing| existing.name == entry.name)
        {
            panic!("plugin '{}' already registered", entry.name);
        }
        self.plugins.push(entry);
        self.sort_plugins();
    }

    fn sort_plugins(&mut self) {
        self.plugins.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.name.cmp(&b.name))
        });
    }

    pub fn run(&mut self, stdout: &mut impl Write) -> Result<()> {
        // Branch early: if simulated_loop is configured, use bounded execution
        if let Some(sim_config) = self.config.simulated_loop {
            return self.run_simulated_internal(stdout, sim_config);
        }

        self.bootstrap(stdout)?;
        let mut last_tick = Instant::now();
        let mut loop_iterations = 0;

        while !self.should_exit {
            // Check loop iteration limit for safety guard
            if let Some(limit) = self.config.loop_iteration_limit {
                if loop_iterations >= limit {
                    self.audit_record(RuntimeAuditStage::LoopGuardTriggered, []);
                    self.log_lifecycle_stage("loop_guard_triggered");
                    self.audit_record(RuntimeAuditStage::LoopAborted, []);
                    self.log_lifecycle_stage("loop_aborted");
                    self.should_exit = true;
                    break;
                }
            }
            loop_iterations += 1;

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
                self.audit_record(RuntimeAuditStage::TickDispatched, []);
                self.render_if_needed(stdout)?;
            }

            self.maybe_emit_metrics();
        }

        self.finalize()
    }

    /// Internal helper for simulated loop execution - performs bootstrap + bounded for-loop
    fn run_simulated_internal(&mut self, stdout: &mut impl Write, sim_config: SimulatedLoop) -> Result<()> {
        self.bootstrap(stdout)?;
        let mut last_tick = Instant::now();

        // Emit audit stage at start of bounded run
        self.audit_record(RuntimeAuditStage::LoopSimulated, []);

        for _iteration in 0..sim_config.max_iterations {
            if self.should_exit {
                break;
            }

            // Optionally emit synthetic tick events
            if sim_config.dispatch_ticks {
                let now = Instant::now();
                let elapsed = now.duration_since(last_tick);
                last_tick = now;
                self.dispatch_event(RuntimeEvent::Tick { elapsed })?;
                self.audit_record(RuntimeAuditStage::TickDispatched, []);
            }

            // Apply pending renders
            self.render_if_needed(stdout)?;

            // Check if we should exit early
            if self.should_exit {
                break;
            }
        }

        // Emit appropriate completion stage based on exit reason
        if self.fatal_active {
            self.audit_record(RuntimeAuditStage::LoopSimulatedAborted, []);
            self.log_lifecycle_stage("loop_simulated_aborted");
        } else {
            self.audit_record(RuntimeAuditStage::LoopSimulatedComplete, []);
            self.log_lifecycle_stage("loop_simulated_complete");
        }

        self.finalize()
    }

    /// Convenience wrapper for running simulated execution - unwraps config option
    pub fn run_simulated(&mut self, stdout: &mut impl Write) -> Result<()> {
        match self.config.simulated_loop {
            Some(sim_config) => self.run_simulated_internal(stdout, sim_config),
            None => Err(LayoutError::Backend("No simulated_loop configuration found".to_string())),
        }
    }

    pub fn run_scripted<I>(&mut self, stdout: &mut impl Write, events: I) -> Result<()>
    where
        I: IntoIterator<Item = RuntimeEvent>,
    {
        self.bootstrap(stdout)?;
        let mut loop_iterations = 0;

        for event in events.into_iter() {
            // Check loop iteration limit for safety guard
            if let Some(limit) = self.config.loop_iteration_limit {
                if loop_iterations >= limit {
                    self.audit_record(RuntimeAuditStage::LoopGuardTriggered, []);
                    self.log_lifecycle_stage("loop_guard_triggered");
                    self.audit_record(RuntimeAuditStage::LoopAborted, []);
                    self.log_lifecycle_stage("loop_aborted");
                    self.should_exit = true;
                    break;
                }
            }
            loop_iterations += 1;
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
        self.finalize()
    }

    /// Obtain fine-grained control over the bootstrap phase without automatically forcing
    /// the first render. The returned handle exposes helpers to present the initial frame,
    /// pump synthetic ticks, or gate startup on high-level events before handing execution
    /// back to a driver.
    pub fn bootstrap_controls<'a, W: Write>(
        &'a mut self,
        stdout: &'a mut W,
    ) -> Result<BootstrapControls<'a, W>> {
        self.bootstrap_prepare()?;
        Ok(BootstrapControls::new(self, stdout))
    }

    fn dispatch_event(&mut self, event: RuntimeEvent) -> Result<()> {
        let mut consumed = false;
        let mut consumed_by: Option<String> = None;
        let loop_kind = LoopEventKind::from_runtime_event(&event);

        if let Some(kind) = loop_kind {
            self.audit_record(
                RuntimeAuditStage::LoopIn,
                [json_kv("kind", json!(kind.as_str()))],
            );
            self.log_runtime_event(
                LogLevel::Debug,
                "loop_in",
                [json_kv("kind", json!(kind.as_str()))],
            );
        }

        if self.screen_manager.is_some() {
            let mut manager = self
                .screen_manager
                .take()
                .expect("screen manager missing after presence check");

            let result: Result<(EventFlow, Vec<ScreenActivation>)> = (|| {
                let mut ctx = RuntimeContext::new(&self.rects, &self.shared_state);
                let flow = manager.handle_event(&mut ctx, &event)?;
                let outcome = ctx.into_outcome();
                self.apply_outcome(outcome)?;
                let mut pending = Vec::new();
                while let Some(activation) = manager.take_pending_activation() {
                    pending.push(activation);
                }
                Ok((flow, pending))
            })();

            match result {
                Ok((flow, pending)) => {
                    for activation in pending {
                        if let Err(err) = manager.finish_activation(self, activation) {
                            self.screen_manager = Some(manager);
                            return Err(err);
                        }
                    }
                    if matches!(flow, EventFlow::Consumed) {
                        consumed = true;
                        consumed_by = Some("screen_manager".to_string());
                    }
                }
                Err(err) => {
                    self.screen_manager = Some(manager);
                    return Err(err);
                }
            }

            self.screen_manager = Some(manager);
        }

        if consumed {
            self.record_event_metric();
            self.log_runtime_event(
                LogLevel::Debug,
                "event_dispatched",
                [
                    json_kv("event", json!(Self::describe_event(&event))),
                    json_kv("consumed", json!(true)),
                    json_kv("consumed_by", json!("screen_manager")),
                ],
            );
            let mut builder = RuntimeAuditEventBuilder::new(RuntimeAuditStage::EventDispatched);
            builder.detail("event", json!(Self::describe_event(&event)));
            builder.detail("consumed", json!(true));
            builder.detail("consumed_by", json!("screen_manager"));
            self.audit_record_event(builder.finish());
            self.maybe_emit_metrics();
            return Ok(());
        }
        for idx in 0..self.plugins.len() {
            let (flow, outcome, plugin_name) = {
                let entry = &mut self.plugins[idx];
                let mut ctx = RuntimeContext::new(&self.rects, &self.shared_state);
                let flow = entry.plugin.on_event(&mut ctx, &event)?;
                let name = entry.name.clone();
                (flow, ctx.into_outcome(), name)
            };
            self.apply_outcome(outcome)?;
            if matches!(flow, EventFlow::Consumed) {
                consumed = true;
                consumed_by = Some(plugin_name);
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
                json_kv(
                    "consumed_by",
                    consumed_by
                        .as_ref()
                        .map(|name| json!(name))
                        .unwrap_or(serde_json::Value::Null),
                ),
            ],
        );
        let mut builder = RuntimeAuditEventBuilder::new(RuntimeAuditStage::EventDispatched);
        builder.detail("event", json!(Self::describe_event(&event)));
        builder.detail("consumed", json!(consumed));
        if let Some(name) = consumed_by {
            builder.detail("consumed_by", json!(name));
        }
        self.audit_record_event(builder.finish());
        self.maybe_emit_metrics();
        if let Some(kind) = loop_kind {
            self.audit_record(
                RuntimeAuditStage::LoopOut,
                [
                    json_kv("kind", json!(kind.as_str())),
                    json_kv("consumed", json!(consumed)),
                ],
            );
            self.log_runtime_event(
                LogLevel::Debug,
                "loop_out",
                [
                    json_kv("kind", json!(kind.as_str())),
                    json_kv("consumed", json!(consumed)),
                ],
            );
        }
        self.flush_notifications()?;
        self.process_pending_errors()?;
        Ok(())
    }

    fn render_if_needed(&mut self, stdout: &mut impl Write) -> Result<()> {
        if !self.redraw_requested {
            self.audit_record(RuntimeAuditStage::RenderSkipped, []);
            return Ok(());
        }

        self.redraw_requested = false;

        for idx in 0..self.plugins.len() {
            let outcome = {
                let entry = &mut self.plugins[idx];
                let mut ctx = RuntimeContext::new(&self.rects, &self.shared_state);
                entry.plugin.before_render(&mut ctx)?;
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
            let mut builder = RuntimeAuditEventBuilder::new(RuntimeAuditStage::RenderCommitted);
            builder.detail("dirty_zones", json!(dirty.len()));
            self.audit_record_event(builder.finish());
        }

        // Emit UserReady after bootstrap completion, regardless of dirty zones
        if !self.user_ready_emitted {
            self.user_ready_emitted = true;
            self.audit_record(RuntimeAuditStage::UserReady, []);
            self.log_lifecycle_stage("user_ready");
            self.notify_plugins(|plugin, ctx| plugin.on_user_ready(ctx))?;
        }

        for idx in 0..self.plugins.len() {
            let outcome = {
                let entry = &mut self.plugins[idx];
                let mut ctx = RuntimeContext::new(&self.rects, &self.shared_state);
                entry.plugin.after_render(&mut ctx)?;
                ctx.into_outcome()
            };
            self.apply_outcome(outcome)?;
        }

        if self.registry.has_dirty() {
            self.redraw_requested = true;
        }

        self.flush_notifications()?;
        self.process_pending_errors()?;

        Ok(())
    }

    fn apply_outcome(&mut self, outcome: ContextOutcome) -> Result<()> {
        let ContextOutcome {
            zone_updates,
            redraw_requested,
            exit_requested,
            cursor_hint,
            cursor_update,
            error,
        } = outcome;

        let update_count = zone_updates.len();
        if update_count > 0 {
            for update in zone_updates {
                let ZoneUpdate {
                    zone,
                    content,
                    pre_rendered,
                } = update;
                if pre_rendered {
                    self.registry.apply_pre_rendered(&zone, content)?;
                } else {
                    self.registry.apply_content(&zone, content)?;
                }
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

        if !cursor_update.is_empty() {
            if let Some(position) = cursor_update.position {
                self.renderer.settings_mut().restore_cursor = Some(position);
            }
            if let Some(visible) = cursor_update.visible {
                self.renderer.settings_mut().cursor_visible = Some(visible);
            }
            let events = self.cursor_manager.apply_update(&cursor_update);
            if !events.is_empty() {
                self.pending_cursor_events.extend(events);
            }
        }

        if exit_requested {
            if !self.user_end_emitted {
                self.user_end_emitted = true;
                self.audit_record(RuntimeAuditStage::UserEnd, []);
                self.log_lifecycle_stage("user_end");
                self.notify_plugins(|plugin, ctx| plugin.on_user_end(ctx))?;
            }
            self.should_exit = true;
            self.log_runtime_event(LogLevel::Info, "exit_requested", std::iter::empty());
        }

        if let Some(error) = error {
            self.pending_errors.push(error);
        }

        self.detect_focus_change()?;

        Ok(())
    }

    fn detect_focus_change(&mut self) -> Result<()> {
        let ctx = RuntimeContext::new(&self.rects, &self.shared_state);
        if let Ok(registry) = ensure_focus_registry(&ctx) {
            let current = registry.current();
            if current != self.last_focus_entry {
                let change = FocusChange {
                    from: self.last_focus_entry.as_ref().map(|entry| FocusTarget {
                        owner: Some(entry.owner.clone()),
                        zone: entry.zone_id.clone(),
                    }),
                    to: current.as_ref().map(|entry| FocusTarget {
                        owner: Some(entry.owner.clone()),
                        zone: entry.zone_id.clone(),
                    }),
                };
                self.pending_focus_changes.push(change);
                self.last_focus_entry = current;
            }
        }
        Ok(())
    }

    fn flush_notifications(&mut self) -> Result<()> {
        let cursor_events = std::mem::take(&mut self.pending_cursor_events);
        for event in cursor_events {
            match &event {
                CursorEvent::Moved(cursor) => {
                    self.audit_record(
                        RuntimeAuditStage::CursorMoved,
                        [
                            json_kv("row", json!(cursor.position.0)),
                            json_kv("col", json!(cursor.position.1)),
                        ],
                    );
                    self.log_runtime_event(
                        LogLevel::Debug,
                        "cursor_moved",
                        [
                            json_kv("row", json!(cursor.position.0)),
                            json_kv("col", json!(cursor.position.1)),
                            json_kv("visible", json!(cursor.visible)),
                        ],
                    );
                }
                CursorEvent::Shown(_) => {
                    self.audit_record(RuntimeAuditStage::CursorShown, []);
                    self.log_runtime_event(LogLevel::Debug, "cursor_shown", std::iter::empty());
                }
                CursorEvent::Hidden(_) => {
                    self.audit_record(RuntimeAuditStage::CursorHidden, []);
                    self.log_runtime_event(LogLevel::Debug, "cursor_hidden", std::iter::empty());
                }
            }
            let event_clone = event.clone();
            self.notify_plugins(|plugin, ctx| plugin.on_cursor_event(ctx, &event_clone))?;
        }

        let focus_changes = std::mem::take(&mut self.pending_focus_changes);
        for change in focus_changes {
            self.audit_record(
                RuntimeAuditStage::FocusChanged,
                [
                    json_kv(
                        "from",
                        json!(
                            change
                                .from
                                .as_ref()
                                .map(|target| target.zone.clone())
                                .unwrap_or_else(|| "".to_string())
                        ),
                    ),
                    json_kv(
                        "to",
                        json!(
                            change
                                .to
                                .as_ref()
                                .map(|target| target.zone.clone())
                                .unwrap_or_else(|| "".to_string())
                        ),
                    ),
                ],
            );
            self.log_runtime_event(
                LogLevel::Debug,
                "focus_changed",
                [
                    json_kv(
                        "from",
                        json!(
                            change
                                .from
                                .as_ref()
                                .map(|target| target.zone.clone())
                                .unwrap_or_else(|| "".to_string())
                        ),
                    ),
                    json_kv(
                        "to",
                        json!(
                            change
                                .to
                                .as_ref()
                                .map(|target| target.zone.clone())
                                .unwrap_or_else(|| "".to_string())
                        ),
                    ),
                ],
            );
            let change_clone = change.clone();
            self.notify_plugins(|plugin, ctx| plugin.on_focus_change(ctx, &change_clone))?;
        }

        Ok(())
    }

    fn process_pending_errors(&mut self) -> Result<()> {
        let errors = std::mem::take(&mut self.pending_errors);
        for mut error in errors {
            self.audit_record(
                RuntimeAuditStage::Error,
                [
                    json_kv("category", json!(error.category.clone())),
                    json_kv("source", json!(error.source.clone())),
                    json_kv("recoverable", json!(error.recoverable)),
                ],
            );
            self.log_runtime_event(
                LogLevel::Error,
                "runtime_error",
                [
                    json_kv("category", json!(error.category.clone())),
                    json_kv("message", json!(error.message.clone())),
                ],
            );

            self.notify_plugins(|plugin, ctx| plugin.on_error(ctx, &mut error))?;

            let recovered = error.recoverable;

            self.audit_record(
                RuntimeAuditStage::RecoverOrFatal,
                [json_kv("recovered", json!(recovered))],
            );
            self.notify_plugins(|plugin, ctx| plugin.on_recover_or_fatal(ctx, &error, recovered))?;

            if recovered {
                self.log_runtime_event(
                    LogLevel::Info,
                    "runtime_error_recovered",
                    [json_kv("category", json!(error.category.clone()))],
                );
            } else {
                self.audit_record(RuntimeAuditStage::Fatal, []);
                self.log_lifecycle_stage("fatal");
                self.notify_plugins(|plugin, ctx| plugin.on_fatal(ctx))?;
                self.fatal_active = true;
                self.should_exit = true;
            }
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
        self.current_size = size;
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

    pub(crate) fn apply_screen_layout(&mut self, layout: impl Layout + 'static) -> Result<()> {
        let rects = layout.solve(self.current_size)?;
        self.layout = Box::new(layout);
        self.rects = rects;
        self.registry.sync_layout(&self.rects);
        self.redraw_requested = true;
        Ok(())
    }

    fn bootstrap(&mut self, stdout: &mut impl Write) -> Result<()> {
        self.bootstrap_prepare()?;
        self.render_if_needed(stdout)
    }

    fn bootstrap_prepare(&mut self) -> Result<()> {
        self.should_exit = false;
        self.redraw_requested = true;
        self.user_ready_emitted = false;
        self.user_end_emitted = false;
        self.fatal_active = false;
        self.pending_cursor_events.clear();
        self.pending_focus_changes.clear();
        self.pending_errors.clear();
        self.last_focus_entry = None;
        self.ensure_metrics_initialized();
        let now = Instant::now();
        self.start_instant = Some(now);
        self.last_metrics_emit = Some(now);
        self.audit_record(RuntimeAuditStage::BootstrapStarted, []);
        self.log_runtime_event(
            LogLevel::Info,
            "runtime_started",
            [
                json_kv("plugins", json!(self.plugins.len())),
                json_kv("zones", json!(self.rects.len())),
            ],
        );

        self.audit_record(RuntimeAuditStage::Boot, []);
        self.log_lifecycle_stage("boot");
        self.notify_plugins(|plugin, ctx| plugin.on_boot(ctx))?;

        for idx in 0..self.plugins.len() {
            let (plugin_name, priority, outcome) = {
                let entry = &mut self.plugins[idx];
                let name = entry.name.clone();
                let priority = entry.priority;
                let mut ctx = RuntimeContext::new(&self.rects, &self.shared_state);
                entry.plugin.init(&mut ctx)?;
                (name, priority, ctx.into_outcome())
            };
            self.log_runtime_event(
                LogLevel::Debug,
                "plugin_initialized",
                [
                    json_kv("plugin", json!(plugin_name)),
                    json_kv("priority", json!(priority)),
                ],
            );
            self.apply_outcome(outcome)?;
            let mut builder = RuntimeAuditEventBuilder::new(RuntimeAuditStage::PluginInitialized);
            builder
                .detail("plugin", json!(plugin_name))
                .detail("priority", json!(priority));
            self.audit_record_event(builder.finish());
        }
        self.apply_configured_focus()?;
        self.audit_record(RuntimeAuditStage::Setup, []);
        self.log_lifecycle_stage("setup");
        self.notify_plugins(|plugin, ctx| plugin.on_setup(ctx))?;
        Ok(())
    }

    pub(crate) fn apply_configured_focus(&mut self) -> Result<()> {
        let Some(zone) = self.config.default_focus_zone.clone() else {
            return Ok(());
        };

        let ctx = RuntimeContext::new(&self.rects, &self.shared_state);
        let registry = ensure_focus_registry(&ctx)
            .map_err(|err| LayoutError::Backend(format!("focus registry: {err}")))?;
        let mut controller = FocusController::new(RUNTIME_FOCUS_OWNER, registry);
        controller.focus(zone);
        let outcome = ctx.into_outcome();
        self.apply_outcome(outcome)?;
        Ok(())
    }

    fn finalize(&mut self) -> Result<()> {
        if self.fatal_active {
            self.audit_record(RuntimeAuditStage::FatalCleanup, []);
            self.log_lifecycle_stage("fatal_cleanup");
        }
        self.audit_record(RuntimeAuditStage::Cleanup, []);
        self.log_lifecycle_stage("cleanup");
        self.notify_plugins(|plugin, ctx| plugin.on_cleanup(ctx))?;
        let uptime_ms = self
            .start_instant
            .map(|start| start.elapsed().as_millis())
            .unwrap_or(0);
        self.log_runtime_event(
            LogLevel::Info,
            "runtime_stopped",
            [json_kv("uptime_ms", json!(uptime_ms))],
        );
        let mut builder = RuntimeAuditEventBuilder::new(RuntimeAuditStage::RuntimeStopped);
        builder.detail("uptime_ms", json!(uptime_ms));
        self.audit_record_event(builder.finish());
        self.flush_notifications()?;
        self.process_pending_errors()?;
        Ok(())
    }

    fn ensure_metrics_initialized(&mut self) {
        if self.config.metrics.is_none() && self.config.metrics_interval > Duration::from_millis(0)
        {
            self.config.metrics = Some(Arc::new(Mutex::new(RuntimeMetrics::new())));
        }
    }

    fn audit_record<I>(&self, stage: RuntimeAuditStage, details: I)
    where
        I: IntoIterator<Item = (String, serde_json::Value)>,
    {
        let mut builder = RuntimeAuditEventBuilder::new(stage);
        for (key, value) in details {
            builder.detail(key, value);
        }
        self.audit.record(builder.finish());
    }

    fn audit_record_event(&self, event: audit::RuntimeAuditEvent) {
        self.audit.record(event);
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

    fn log_lifecycle_stage(&self, stage: &str) {
        self.log_runtime_event(
            LogLevel::Debug,
            "lifecycle",
            [json_kv("stage", json!(stage))],
        );
    }

    fn notify_plugins<F>(&mut self, mut hook: F) -> Result<()>
    where
        F: for<'a> FnMut(&mut dyn RoomPlugin, &mut RuntimeContext<'a>) -> Result<()>,
    {
        for idx in 0..self.plugins.len() {
            let outcome = {
                let entry = &mut self.plugins[idx];
                let mut ctx = RuntimeContext::new(&self.rects, &self.shared_state);
                hook(entry.plugin.as_mut(), &mut ctx)?;
                ctx.into_outcome()
            };
            self.apply_outcome(outcome)?;
        }
        Ok(())
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
            RuntimeEvent::Open => "open",
            RuntimeEvent::Boot => "boot",
            RuntimeEvent::Setup => "setup",
            RuntimeEvent::UserReady => "user_ready",
            RuntimeEvent::LoopIn { .. } => "loop_in",
            RuntimeEvent::LoopOut { .. } => "loop_out",
            RuntimeEvent::UserEnd => "user_end",
            RuntimeEvent::Cleanup => "cleanup",
            RuntimeEvent::End => "end",
            RuntimeEvent::Close => "close",
            RuntimeEvent::Error(_) => "error",
            RuntimeEvent::RecoverOrFatal { .. } => "recover_or_fatal",
            RuntimeEvent::Fatal => "fatal",
            RuntimeEvent::FatalCleanup => "fatal_cleanup",
            RuntimeEvent::FatalClose => "fatal_close",
            RuntimeEvent::CursorMoved(_) => "cursor_moved",
            RuntimeEvent::CursorShown(_) => "cursor_shown",
            RuntimeEvent::CursorHidden(_) => "cursor_hidden",
            RuntimeEvent::FocusChanged(_) => "focus_changed",
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

/// Controller returned by `RoomRuntime::bootstrap_controls` so callers can synchronously
/// drive the runtime through bootstrap experiments before handing execution to a driver.
pub struct BootstrapControls<'a, W: Write> {
    runtime: &'a mut RoomRuntime,
    stdout: &'a mut W,
    first_frame_presented: bool,
}

impl<'a, W: Write> BootstrapControls<'a, W> {
    fn new(runtime: &'a mut RoomRuntime, stdout: &'a mut W) -> Self {
        Self {
            runtime,
            stdout,
            first_frame_presented: false,
        }
    }

    /// Present the first frame if it has not yet been rendered.
    pub fn present_first_frame(&mut self) -> Result<()> {
        if !self.first_frame_presented {
            self.runtime.render_if_needed(self.stdout)?;
            self.first_frame_presented = true;
        }
        Ok(())
    }

    /// Ensure at least one render has occurred, calling `present_first_frame` if necessary.
    pub fn ensure_first_frame(&mut self) -> Result<()> {
        if !self.first_frame_presented {
            self.present_first_frame()?;
        }
        Ok(())
    }

    /// Dispatch a runtime event and render any resulting updates.
    pub fn dispatch_event(&mut self, event: RuntimeEvent) -> Result<()> {
        let event = match event {
            RuntimeEvent::Resize(size) => {
                self.runtime.handle_resize(size)?;
                RuntimeEvent::Resize(size)
            }
            other => other,
        };
        self.runtime.dispatch_event(event)?;
        self.runtime.render_if_needed(self.stdout)?;
        self.first_frame_presented = true;
        Ok(())
    }

    /// Dispatch a synthetic tick event and render any resulting updates.
    pub fn dispatch_tick(&mut self, elapsed: Duration) -> Result<()> {
        self.runtime
            .dispatch_event(RuntimeEvent::Tick { elapsed })?;
        self.runtime
            .audit_record(RuntimeAuditStage::TickDispatched, []);
        self.runtime.render_if_needed(self.stdout)?;
        self.first_frame_presented = true;
        Ok(())
    }

    /// Pump the runtime for a fixed number of synthetic ticks.
    pub fn run_ticks(&mut self, count: usize, interval: Duration) -> Result<()> {
        for _ in 0..count {
            self.dispatch_tick(interval)?;
        }
        Ok(())
    }

    /// Gate bootstrap on the first key event returned by the supplied provider. The
    /// provider is typically a thin wrapper over `crossterm::event::poll` + `read` that
    /// yields high-level runtime events.
    pub fn gate_on_first_key_event<F>(&mut self, mut next_event: F) -> Result<()>
    where
        F: FnMut() -> Result<Option<RuntimeEvent>>,
    {
        loop {
            let maybe_event = next_event()?;
            let Some(event) = maybe_event else {
                continue;
            };
            let is_key = matches!(event, RuntimeEvent::Key(_));
            self.dispatch_event(event)?;
            if is_key {
                break;
            }
        }
        Ok(())
    }

    /// Finalise bootstrap, ensuring the first frame is presented before returning the
    /// underlying runtime and writer handles.
    pub fn finish_with_handles(mut self) -> Result<(&'a mut RoomRuntime, &'a mut W)> {
        self.ensure_first_frame()?;
        let runtime = self.runtime;
        let stdout = self.stdout;
        Ok((runtime, stdout))
    }

    /// Finalise bootstrap, ensuring the first frame is presented before dropping the
    /// controller.
    pub fn finish(mut self) -> Result<()> {
        self.ensure_first_frame()
    }
}

#[cfg(test)]
mod bootstrap_tests {
    use super::*;
    use crate::{
        AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, RoomPlugin, RuntimeConfig,
        Size,
    };
    use std::time::Duration;

    const TEST_ZONE: &str = "app:test";

    #[derive(Default)]
    struct TestPlugin {
        ticks: usize,
    }

    impl RoomPlugin for TestPlugin {
        fn name(&self) -> &str {
            "test_plugin"
        }

        fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
            ctx.set_zone(TEST_ZONE, "Bootstrap starting");
            Ok(())
        }

        fn on_event(
            &mut self,
            ctx: &mut RuntimeContext<'_>,
            event: &RuntimeEvent,
        ) -> Result<EventFlow> {
            if matches!(event, RuntimeEvent::Tick { .. }) {
                self.ticks += 1;
                ctx.set_zone(TEST_ZONE, format!("Ticks observed: {}", self.ticks));
            }
            if matches!(event, RuntimeEvent::Key(_)) {
                ctx.set_zone(TEST_ZONE, "Key received");
            }
            Ok(EventFlow::Continue)
        }
    }

    fn build_runtime() -> RoomRuntime {
        let layout = LayoutTree::new(LayoutNode {
            id: "app:root".into(),
            direction: Direction::Column,
            constraints: vec![Constraint::Flex(1)],
            children: vec![LayoutNode::leaf(TEST_ZONE)],
            gap: 0,
            padding: 0,
        });
        let renderer = AnsiRenderer::with_default();
        let config = RuntimeConfig::default();
        let mut runtime =
            RoomRuntime::with_config(layout, renderer, Size::new(40, 4), config).expect("runtime");
        runtime.register_plugin(TestPlugin::default());
        runtime
    }

    #[test]
    fn present_first_frame_renders_content() {
        let mut runtime = build_runtime();
        let mut buffer = Vec::new();
        {
            let mut controls = runtime.bootstrap_controls(&mut buffer).expect("controls");
            controls.present_first_frame().expect("render");
            controls.finish().expect("finish");
        }
        assert!(!buffer.is_empty(), "bootstrap render should write output");
    }

    #[test]
    fn run_ticks_updates_plugin_state() {
        let mut runtime = build_runtime();
        let mut buffer = Vec::new();
        {
            let mut controls = runtime.bootstrap_controls(&mut buffer).expect("controls");
            controls
                .run_ticks(3, Duration::from_millis(10))
                .expect("ticks");
            controls.finish().expect("finish");
        }
        let output = String::from_utf8(buffer).expect("utf8");
        assert!(output.contains("Ticks observed: 3"));
    }
}
