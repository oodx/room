//! Room Workshop: Boxy Dashboard Runtime
//!
//! This interactive example demonstrates how Room's runtime can host multiple Boxy panels
//! with shared focus and dynamic content.
//!
//! ```bash
//! bin/examples.sh run boxy_dashboard_runtime
//! ```
//!
//! Keyboard shortcuts:
//! * `Tab` / `Shift+Tab` — cycle focus across panels and the prompt
//! * `Ctrl+R` — refresh status/metrics activity data
//! * `Ctrl+Enter` — submit the prompt (regular `Enter` appends to the log)
//! * Arrow Up/Down — move panel focus when panels own input
//! * `Ctrl+C` — exit the runtime
//!
//! See `docs/ref/workshops/workshop_boxy_dashboard_runtime.md` for the full workshop flow.

use std::sync::Arc;
use std::time::Duration;

use boxy::{
    BoxColors, BoxyConfig, WidthConfig, height_plugin::get_max_safe_height, render_to_string,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use room_mvp::runtime::audit::{BootstrapAudit, NullRuntimeAudit};
use room_mvp::runtime::focus::{FocusController, ensure_focus_registry};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, LayoutError, LayoutNode, LayoutTree,
    LegacyScreenStrategy, Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext,
    RuntimeEvent, ScreenDefinition, ScreenManager, SharedStateError, Size,
};
use rsb::visual::glyphs::{glyph, glyph_enable};

const HEADER_ZONE: &str = "app:runtime.header";
const FOOTER_ZONE: &str = "app:runtime.footer";
const PANEL_STATUS_ZONE: &str = "app:runtime.panel.status";
const PANEL_METRICS_ZONE: &str = "app:runtime.panel.metrics";
const PANEL_LOG_ZONE: &str = "app:runtime.panel.log";
const PROMPT_ZONE: &str = "app:runtime.prompt";

const PANEL_OWNER: &str = "boxy::runtime";
const PANEL_MIN_HEIGHT: usize = 6;
const PANEL_MAX_HEIGHT: usize = 18;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    glyph_enable();

    let layout = build_layout();
    let screen_layout = layout.clone();
    let renderer = AnsiRenderer::with_default();

    let mut config = RuntimeConfig::default();
    config.audit = Some(BootstrapAudit::new(Arc::new(NullRuntimeAudit)));
    config.default_focus_zone = Some(PROMPT_ZONE.to_string());
    config.tick_interval = Duration::from_secs(2);

    let mut runtime = RoomRuntime::with_config(layout, renderer, Size::new(120, 34), config)?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "dashboard",
        "Boxy Dashboard",
        Arc::new(move || Box::new(LegacyScreenStrategy::new(screen_layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("dashboard")?;

    runtime.register_plugin(BoxyDashboardPlugin::new());

    CliDriver::new(runtime).run()?;
    Ok(())
}

struct BoxyPanel {
    id: &'static str,
    config: BoxyConfig,
    cache: Option<String>,
    dirty: bool,
    base_color: &'static str,
    focus_color: &'static str,
}

impl BoxyPanel {
    fn new(
        id: &'static str,
        mut config: BoxyConfig,
        base_color: &'static str,
        focus_color: &'static str,
    ) -> Self {
        config.colors.box_color = base_color.to_string();
        Self {
            id,
            config,
            cache: None,
            dirty: true,
            base_color,
            focus_color,
        }
    }

    fn id(&self) -> &'static str {
        self.id
    }

    fn render(&mut self) -> &str {
        if self.dirty {
            let rendered = render_to_string(&self.config);
            self.cache = Some(rendered);
            self.dirty = false;
        }
        self.cache.as_deref().unwrap_or("")
    }

    fn set_focus(&mut self, focused: bool) {
        let target = if focused {
            self.focus_color
        } else {
            self.base_color
        };
        if self.config.colors.box_color != target {
            self.config.colors.box_color = target.to_string();
            self.mark_dirty();
        }
    }

    fn replace_text(&mut self, text: String) {
        if self.config.text != text {
            self.config.text = text;
            self.mark_dirty();
        }
    }

    fn append_line(&mut self, line: &str, max_lines: usize) {
        let mut lines: Vec<String> = self.config.text.lines().map(|s| s.to_string()).collect();
        lines.push(line.to_string());
        if lines.len() > max_lines {
            let excess = lines.len() - max_lines;
            lines.drain(0..excess);
        }
        let joined = lines.join("\n");
        self.replace_text(joined);
    }

    fn set_fixed_height(&mut self, height: usize) {
        let sanitized = height.clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT);
        if self.config.fixed_height != Some(sanitized) {
            self.config.fixed_height = Some(sanitized);
            self.mark_dirty();
        }
    }

    fn update_title(&mut self, title: Option<String>) {
        if self.config.title != title {
            self.config.title = title;
            self.mark_dirty();
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.cache = None;
    }
}

struct BoxyPrompt {
    zone_id: &'static str,
    config: BoxyConfig,
    cache: Option<String>,
    dirty: bool,
    buffer: String,
    cursor: usize,
    focused: bool,
    base_color: &'static str,
    focus_color: &'static str,
    cursor_glyph_active: &'static str,
    cursor_glyph_inactive: &'static str,
    placeholder: &'static str,
}

impl BoxyPrompt {
    fn new(zone_id: &'static str) -> Self {
        let mut config = BoxyConfig {
            title: Some(format!("{} Prompt", glyph("chat"))),
            status_bar: Some("Ctrl+Enter to submit · Esc to clear".to_string()),
            colors: BoxColors {
                box_color: "teal".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("white".to_string()),
                status_color: Some("white".to_string()),
                header_color: None,
                footer_color: None,
            },
            width: WidthConfig {
                fixed_width: None,
                ..WidthConfig::default()
            },
            fixed_height: None,
            ..BoxyConfig::default()
        };
        config.text = String::new();
        Self {
            zone_id,
            config,
            cache: None,
            dirty: true,
            buffer: String::new(),
            cursor: 0,
            focused: false,
            base_color: "teal",
            focus_color: "yellow",
            cursor_glyph_active: "▌",
            cursor_glyph_inactive: " ",
            placeholder: "Type a message and press Enter",
        }
    }

    fn id(&self) -> &'static str {
        self.zone_id
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
        let target = if focused {
            self.focus_color
        } else {
            self.base_color
        };
        if self.config.colors.box_color != target {
            self.config.colors.box_color = target.to_string();
        }
        self.mark_dirty();
    }

    fn insert_char(&mut self, ch: char) -> bool {
        if ch == '\n' {
            return false;
        }
        let mut chars: Vec<char> = self.buffer.chars().collect();
        if self.cursor > chars.len() {
            self.cursor = chars.len();
        }
        chars.insert(self.cursor, ch);
        self.cursor += 1;
        self.buffer = chars.into_iter().collect();
        self.mark_dirty();
        true
    }

    fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let mut chars: Vec<char> = self.buffer.chars().collect();
        if self.cursor > chars.len() {
            self.cursor = chars.len();
        }
        if self.cursor == 0 {
            return false;
        }
        chars.remove(self.cursor - 1);
        self.cursor -= 1;
        self.buffer = chars.into_iter().collect();
        self.mark_dirty();
        true
    }

    fn delete(&mut self) -> bool {
        let mut chars: Vec<char> = self.buffer.chars().collect();
        if self.cursor >= chars.len() {
            return false;
        }
        chars.remove(self.cursor);
        self.buffer = chars.into_iter().collect();
        self.mark_dirty();
        true
    }

    fn move_left(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.mark_dirty();
        true
    }

    fn move_right(&mut self) -> bool {
        let len = self.buffer.chars().count();
        if self.cursor >= len {
            return false;
        }
        self.cursor += 1;
        self.mark_dirty();
        true
    }

    fn move_home(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor = 0;
        self.mark_dirty();
        true
    }

    fn move_end(&mut self) -> bool {
        let len = self.buffer.chars().count();
        if self.cursor == len {
            return false;
        }
        self.cursor = len;
        self.mark_dirty();
        true
    }

    fn clear(&mut self) -> bool {
        if self.buffer.is_empty() {
            return false;
        }
        self.buffer.clear();
        self.cursor = 0;
        self.mark_dirty();
        true
    }

    fn current_submission(&self) -> Option<String> {
        let trimmed = self.buffer.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn take_submission(&mut self) -> Option<String> {
        let trimmed = self.buffer.trim();
        if trimmed.is_empty() {
            self.clear();
            return None;
        }
        let submission = trimmed.to_string();
        self.clear();
        Some(submission)
    }

    fn render(&mut self) -> &str {
        if self.dirty {
            self.config.text = self.build_text();
            let rendered = render_to_string(&self.config);
            self.cache = Some(rendered);
            self.dirty = false;
        }
        self.cache.as_deref().unwrap_or("")
    }

    fn build_text(&self) -> String {
        let mut lines = Vec::new();
        if self.buffer.is_empty() {
            lines.push(self.placeholder.to_string());
        } else {
            lines.push(String::new());
        }
        lines.push(self.render_line());
        lines.join("\n")
    }

    fn render_line(&self) -> String {
        let mut left = String::new();
        let mut right = String::new();
        for (idx, ch) in self.buffer.chars().enumerate() {
            if idx < self.cursor {
                left.push(ch);
            } else {
                right.push(ch);
            }
        }
        let cursor_symbol = if self.focused {
            self.cursor_glyph_active
        } else {
            self.cursor_glyph_inactive
        };
        format!("> {}{}{}", left, cursor_symbol, right)
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.cache = None;
    }

    fn resize(&mut self, width: usize, height: usize) {
        let width = width.max(12);
        if self.config.width.fixed_width != Some(width) {
            self.config.width.fixed_width = Some(width);
            self.mark_dirty();
        }
        let height = height.max(5);
        if self.config.fixed_height != Some(height) {
            self.config.fixed_height = Some(height);
            self.mark_dirty();
        }
    }
}

#[derive(Clone, Copy)]
enum FocusSlot {
    Panel(usize),
    Prompt,
}

struct BoxyDashboardPlugin {
    panels: Vec<BoxyPanel>,
    prompt: BoxyPrompt,
    focus_index: usize,
    cycle_tick: u32,
    cycle_interval: Duration,
    elapsed: Duration,
    focus: Option<FocusController>,
}

impl BoxyDashboardPlugin {
    fn new() -> Self {
        let panel_height = compute_panel_height();
        let panels = init_panels(panel_height);
        let prompt = BoxyPrompt::new(PROMPT_ZONE);
        let focus_index = panels.len();
        Self {
            panels,
            prompt,
            focus_index,
            cycle_tick: 0,
            cycle_interval: Duration::from_secs(3),
            elapsed: Duration::default(),
            focus: None,
        }
    }

    fn focus_controller<'a>(
        &'a mut self,
        ctx: &RuntimeContext<'a>,
    ) -> std::result::Result<&'a mut FocusController, SharedStateError> {
        if self.focus.is_none() {
            let registry = ensure_focus_registry(ctx)?;
            self.focus = Some(FocusController::new(PANEL_OWNER, registry));
        }
        Ok(self.focus.as_mut().expect("focus controller present"))
    }

    fn focus_slots_len(&self) -> usize {
        self.panels.len() + 1
    }

    fn slot_for(&self, index: usize) -> FocusSlot {
        if index < self.panels.len() {
            FocusSlot::Panel(index)
        } else {
            FocusSlot::Prompt
        }
    }

    fn current_slot(&self) -> FocusSlot {
        self.slot_for(self.focus_index)
    }

    fn apply_focus(&mut self, ctx: &mut RuntimeContext<'_>, index: usize) -> Result<()> {
        if index >= self.focus_slots_len() {
            return Ok(());
        }

        match self.current_slot() {
            FocusSlot::Panel(i) => self.panels[i].set_focus(false),
            FocusSlot::Prompt => self.prompt.set_focus(false),
        }

        self.focus_index = index;

        let zone = match self.current_slot() {
            FocusSlot::Panel(i) => {
                self.panels[i].set_focus(true);
                self.panels[i].id().to_string()
            }
            FocusSlot::Prompt => {
                self.prompt.set_focus(true);
                self.prompt.id().to_string()
            }
        };

        self.focus_controller(ctx)
            .map_err(map_shared_err)?
            .focus(zone);
        Ok(())
    }

    fn rotate_focus(&mut self, ctx: &mut RuntimeContext<'_>, delta: isize) -> Result<()> {
        let len = self.focus_slots_len() as isize;
        if len == 0 {
            return Ok(());
        }
        let next = (self.focus_index as isize + delta).rem_euclid(len) as usize;
        if next != self.focus_index {
            self.apply_focus(ctx, next)?;
        }
        Ok(())
    }

    fn render_dashboard(&mut self, ctx: &mut RuntimeContext<'_>) {
        if let Some(rect) = ctx.rect(PROMPT_ZONE) {
            self.prompt
                .resize(rect.width as usize, rect.height as usize);
        }

        let header_text = format!(
            "{} Room + Boxy Runtime · Tab cycles focus · Ctrl+R refresh",
            glyph("light")
        );
        ctx.set_zone(HEADER_ZONE, header_text);

        for panel in &mut self.panels {
            let rendered = panel.render().to_string();
            ctx.set_zone_pre_rendered(panel.id(), rendered);
        }

        ctx.set_zone_pre_rendered(PROMPT_ZONE, self.prompt.render());

        let focus_label = match self.current_slot() {
            FocusSlot::Panel(i) => self.panels[i].id(),
            FocusSlot::Prompt => self.prompt.id(),
        };
        let footer_text = format!(
            "Focus: {} · Tick {} · Ctrl+C to exit",
            focus_label, self.cycle_tick
        );
        ctx.set_zone(FOOTER_ZONE, footer_text);
    }

    fn cycle_panels(&mut self) {
        self.cycle_tick = self.cycle_tick.wrapping_add(1);
        let tick = self.cycle_tick;
        if let Some(status_panel) = self
            .panels
            .iter_mut()
            .find(|panel| panel.id() == PANEL_STATUS_ZONE)
        {
            let (status_text, color, title) = match tick % 4 {
                0 => ("Status: STABLE\nCPU: 37%\nRAM: 1.1GB", "blue", "Status"),
                1 => ("Status: BUSY\nCPU: 63%\nRAM: 1.6GB", "yellow", "Status"),
                2 => ("Status: HOT\nCPU: 82%\nRAM: 2.2GB", "red", "Status"),
                _ => ("Status: COOL\nCPU: 28%\nRAM: 0.9GB", "green", "Status"),
            };
            status_panel.replace_text(format!("{}\n{}", status_text, tick_line(tick)));
            status_panel.config.colors.box_color = color.to_string();
            status_panel.update_title(Some(format!("{} {}", glyph("gear"), title)));
            status_panel.mark_dirty();
        }

        if let Some(metrics_panel) = self
            .panels
            .iter_mut()
            .find(|panel| panel.id() == PANEL_METRICS_ZONE)
        {
            metrics_panel.replace_text(metrics_text(tick));
        }

        if let Some(activity_panel) = self
            .panels
            .iter_mut()
            .find(|panel| panel.id() == PANEL_LOG_ZONE)
        {
            let message = match tick % 4 {
                0 => "System heartbeat OK",
                1 => "Workload batch submitted",
                2 => "Autoscaler responding",
                _ => "Cleanup cycle complete",
            };
            activity_panel.append_line(message, 8);
        }
    }

    fn record_submission(&mut self, ctx: &mut RuntimeContext<'_>, submission: &str) {
        self.log_activity(ctx, format!("You: {}", submission));
    }

    fn handle_prompt_key(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        key: &KeyEvent,
    ) -> Result<EventFlow> {
        match key.code {
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let Some(submission) = self.prompt.take_submission() {
                        self.record_submission(ctx, &submission);
                    }
                    ctx.request_render();
                    return Ok(EventFlow::Consumed);
                }
                if let Some(submission) = self.prompt.current_submission() {
                    self.record_submission(ctx, &submission);
                    self.prompt.mark_dirty();
                }
                ctx.request_render();
                Ok(EventFlow::Consumed)
            }
            KeyCode::Backspace => {
                if self.prompt.backspace() {
                    ctx.request_render();
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }
            KeyCode::Delete => {
                if self.prompt.delete() {
                    ctx.request_render();
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }
            KeyCode::Left => {
                if self.prompt.move_left() {
                    ctx.request_render();
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }
            KeyCode::Right => {
                if self.prompt.move_right() {
                    ctx.request_render();
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }
            KeyCode::Home => {
                if self.prompt.move_home() {
                    ctx.request_render();
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }
            KeyCode::End => {
                if self.prompt.move_end() {
                    ctx.request_render();
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }
            KeyCode::Esc => {
                if self.prompt.clear() {
                    ctx.request_render();
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }
            KeyCode::Char(ch) => {
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    return Ok(EventFlow::Continue);
                }
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                {
                    return Ok(EventFlow::Continue);
                }
                if self.prompt.insert_char(ch) {
                    ctx.request_render();
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }
            _ => Ok(EventFlow::Continue),
        }
    }

    fn handle_panel_key(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        key: &KeyEvent,
    ) -> Result<EventFlow> {
        match key.code {
            KeyCode::Up => {
                self.rotate_focus(ctx, -1)?;
                ctx.request_render();
                Ok(EventFlow::Consumed)
            }
            KeyCode::Down => {
                self.rotate_focus(ctx, 1)?;
                ctx.request_render();
                Ok(EventFlow::Consumed)
            }
            _ => Ok(EventFlow::Continue),
        }
    }

    fn handle_key(&mut self, ctx: &mut RuntimeContext<'_>, key: &KeyEvent) -> Result<EventFlow> {
        match key.kind {
            KeyEventKind::Press | KeyEventKind::Repeat => {}
            _ => return Ok(EventFlow::Continue),
        }

        self.log_activity(
            ctx,
            format!(
                "Key event: {:?} mods {:?} kind {:?}",
                key.code, key.modifiers, key.kind
            ),
        );

        match key.code {
            KeyCode::Tab => {
                self.rotate_focus(ctx, 1)?;
                ctx.request_render();
                return Ok(EventFlow::Consumed);
            }
            KeyCode::BackTab => {
                self.rotate_focus(ctx, -1)?;
                ctx.request_render();
                return Ok(EventFlow::Consumed);
            }
            _ => {}
        }

        if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.cycle_panels();
            self.force_refresh();
            ctx.request_render();
            return Ok(EventFlow::Consumed);
        }

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            ctx.request_exit();
            return Ok(EventFlow::Consumed);
        }

        match self.current_slot() {
            FocusSlot::Prompt => self.handle_prompt_key(ctx, key),
            FocusSlot::Panel(_) => self.handle_panel_key(ctx, key),
        }
    }

    fn log_activity(&mut self, ctx: &mut RuntimeContext<'_>, line: impl Into<String>) {
        if let Some(activity_panel) = self
            .panels
            .iter_mut()
            .find(|panel| panel.id() == PANEL_LOG_ZONE)
        {
            activity_panel.append_line(&line.into(), 10);
            ctx.request_render();
        }
    }

    fn force_refresh(&mut self) {
        for panel in &mut self.panels {
            panel.mark_dirty();
        }
        self.prompt.mark_dirty();
    }

    fn recompute_heights(&mut self) {
        let panel_height = compute_panel_height();
        for panel in &mut self.panels {
            panel.set_fixed_height(panel_height);
        }
        self.prompt.mark_dirty();
    }
}

impl RoomPlugin for BoxyDashboardPlugin {
    fn name(&self) -> &str {
        "boxy_dashboard_runtime"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.focus = None;
        self.force_refresh();
        self.apply_focus(ctx, self.panels.len())?;
        self.render_dashboard(ctx);
        ctx.request_render();
        Ok(())
    }

    fn on_user_ready(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.log_activity(ctx, "[lifecycle] UserReady emitted");
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        self.log_activity(ctx, format!("RuntimeEvent observed: {:?}", event));
        match event {
            RuntimeEvent::Key(key) => self.handle_key(ctx, key),
            RuntimeEvent::Resize(_) => {
                self.recompute_heights();
                self.force_refresh();
                ctx.request_render();
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Tick { elapsed } => {
                self.elapsed += *elapsed;
                if self.elapsed >= self.cycle_interval {
                    self.elapsed = Duration::default();
                    self.cycle_panels();
                    ctx.request_render();
                }
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::FocusGained
            | RuntimeEvent::FocusLost
            | RuntimeEvent::Mouse(_)
            | RuntimeEvent::Paste(_)
            | RuntimeEvent::Raw(_) => Ok(EventFlow::Continue),
            _ => Ok(EventFlow::Continue),
        }
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.render_dashboard(ctx);
        Ok(())
    }
}

fn map_shared_err(err: SharedStateError) -> LayoutError {
    LayoutError::Backend(format!("shared state error: {err}"))
}

fn init_panels(panel_height: usize) -> Vec<BoxyPanel> {
    let mut status_panel = BoxyPanel::new(
        PANEL_STATUS_ZONE,
        BoxyConfig {
            text: "Status: BOOTING\nCPU: --\nRAM: --".to_string(),
            title: Some(format!("{} Status", glyph("gear"))),
            colors: BoxColors {
                box_color: "blue".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("white".to_string()),
                status_color: Some("white".to_string()),
                header_color: None,
                footer_color: None,
            },
            width: WidthConfig {
                fixed_width: Some(28),
                ..WidthConfig::default()
            },
            fixed_height: Some(panel_height),
            ..BoxyConfig::default()
        },
        "blue",
        "yellow",
    );
    status_panel.append_line("Awaiting updates...", 6);

    let metrics_panel = BoxyPanel::new(
        PANEL_METRICS_ZONE,
        BoxyConfig {
            text: metrics_text(0),
            title: Some(format!("{} Metrics", glyph("bolt"))),
            colors: BoxColors {
                box_color: "purple".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("white".to_string()),
                status_color: None,
                header_color: None,
                footer_color: None,
            },
            width: WidthConfig {
                fixed_width: Some(28),
                ..WidthConfig::default()
            },
            fixed_height: Some(panel_height),
            ..BoxyConfig::default()
        },
        "purple",
        "magenta",
    );

    let mut activity_panel = BoxyPanel::new(
        PANEL_LOG_ZONE,
        BoxyConfig {
            text: "Activity log ready.".to_string(),
            title: Some(format!("{} Activity", glyph("star"))),
            colors: BoxColors {
                box_color: "cyan".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("black".to_string()),
                status_color: None,
                header_color: None,
                footer_color: None,
            },
            width: WidthConfig {
                fixed_width: Some(28),
                ..WidthConfig::default()
            },
            fixed_height: Some(panel_height),
            ..BoxyConfig::default()
        },
        "cyan",
        "green",
    );
    activity_panel.append_line("Awaiting updates...", 8);

    vec![status_panel, metrics_panel, activity_panel]
}

fn compute_panel_height() -> usize {
    let safe = get_max_safe_height();
    let reserved = 8usize; // header, footer, prompt, breathing room
    let available = safe.saturating_sub(reserved);
    let per_panel = available / 3;
    per_panel.clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT)
}

fn metrics_text(tick: u32) -> String {
    let latency = 18 + ((tick as i32) % 7);
    let throughput = 240 + ((tick as i32) % 50);
    format!(
        "Req/s: {}\nLatency: {} ms\nErrors: {}\nUptime: {}h",
        throughput,
        latency,
        tick % 3,
        12 + (tick % 9)
    )
}

fn tick_line(tick: u32) -> String {
    format!("Update: #{}", tick)
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(1),
            Constraint::Flex(1),
            Constraint::Fixed(7),
            Constraint::Fixed(1),
        ],
        children: vec![
            LayoutNode::leaf(HEADER_ZONE),
            LayoutNode {
                id: "app:runtime.body".into(),
                direction: Direction::Row,
                constraints: vec![
                    Constraint::Flex(1),
                    Constraint::Flex(1),
                    Constraint::Flex(1),
                ],
                children: vec![
                    LayoutNode::leaf(PANEL_STATUS_ZONE),
                    LayoutNode::leaf(PANEL_METRICS_ZONE),
                    LayoutNode::leaf(PANEL_LOG_ZONE),
                ],
                gap: 1,
                padding: 1,
            },
            LayoutNode::leaf(PROMPT_ZONE),
            LayoutNode::leaf(FOOTER_ZONE),
        ],
        gap: 1,
        padding: 0,
    })
}
