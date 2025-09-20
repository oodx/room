use std::collections::HashMap;
use std::io::Write;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};

use crate::{AnsiRenderer, LayoutTree, Rect, Result, Size, ZoneRegistry};

/// Configuration knobs for the runtime loop.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Interval between synthetic tick events.
    pub tick_interval: Duration,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_millis(200),
        }
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
        for idx in 0..self.plugins.len() {
            let outcome = {
                let plugin = &mut self.plugins[idx];
                let mut ctx = RuntimeContext::new(&self.rects);
                plugin.init(&mut ctx)?;
                ctx.into_outcome()
            };
            self.apply_outcome(outcome)?;
        }

        self.render_if_needed(stdout)?;

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
        }

        Ok(())
    }

    fn dispatch_event(&mut self, event: RuntimeEvent) -> Result<()> {
        for idx in 0..self.plugins.len() {
            let (flow, outcome) = {
                let plugin = &mut self.plugins[idx];
                let mut ctx = RuntimeContext::new(&self.rects);
                let flow = plugin.on_event(&mut ctx, &event)?;
                (flow, ctx.into_outcome())
            };
            self.apply_outcome(outcome)?;
            if matches!(flow, EventFlow::Consumed) {
                break;
            }
        }
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
        if !outcome.zone_updates.is_empty() {
            for (zone, content) in outcome.zone_updates {
                self.registry.apply_content(&zone, content)?;
            }
            self.redraw_requested = true;
        }

        if outcome.redraw_requested {
            self.redraw_requested = true;
        }

        if let Some(cursor) = outcome.cursor_hint {
            self.renderer.settings_mut().restore_cursor = Some(cursor);
        }

        if outcome.exit_requested {
            self.should_exit = true;
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
        Ok(())
    }
}
