use std::time::Duration;

use boxy::{BoxColors, BoxyConfig, WidthConfig, render_to_string};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, LayoutNode, LayoutTree,
    LegacyScreenStrategy, Rect, Result, RoomPlugin, RoomRuntime, RuntimeContext, RuntimeEvent,
    ScreenDefinition, ScreenManager, Size,
};
use rsb::visual::glyphs::{glyph, glyph_enable};

const HEADER_ZONE: &str = "app:demo.header";
const FOOTER_ZONE: &str = "app:demo.footer";
const PANEL_STATUS_ZONE: &str = "app:demo.panel.status";
const PANEL_METRICS_ZONE: &str = "app:demo.panel.metrics";
const PANEL_LOG_ZONE: &str = "app:demo.panel.log";
const TICK_INTERVAL: Duration = Duration::from_millis(200);
const STATUS_CYCLE: Duration = Duration::from_secs(3);

fn main() -> Result<()> {
    glyph_enable();

    let layout = build_layout();
    let screen_layout = layout.clone();
    let renderer = AnsiRenderer::with_default();
    let mut runtime = RoomRuntime::new(layout, renderer, Size::new(120, 32))?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "boxy-dashboard",
        "Boxy Dashboard",
        std::sync::Arc::new(move || Box::new(LegacyScreenStrategy::new(screen_layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("boxy-dashboard")?;

    runtime.config_mut().tick_interval = TICK_INTERVAL;
    runtime.register_plugin(BoxyDashboardPlugin::default());

    CliDriver::new(runtime)
        .run()
        .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))?;
    Ok(())
}

struct BoxyPanel {
    id: &'static str,
    config: BoxyConfig,
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
            base_color,
            focus_color,
        }
    }

    fn id(&self) -> &'static str {
        self.id
    }

    fn set_focus(&mut self, focused: bool) {
        let target = if focused {
            self.focus_color
        } else {
            self.base_color
        };
        if self.config.colors.box_color != target {
            self.config.colors.box_color = target.to_string();
        }
    }

    fn replace_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.config.text != text {
            self.config.text = text;
        }
    }

    fn append_line(&mut self, line: &str, max_lines: usize) {
        let mut lines: Vec<String> = self.config.text.lines().map(|s| s.to_string()).collect();
        lines.push(line.to_string());
        if lines.len() > max_lines {
            let excess = lines.len() - max_lines;
            lines.drain(0..excess);
        }
        self.config.text = lines.join("\n");
    }

    fn update_title(&mut self, title: Option<String>) {
        if self.config.title != title {
            self.config.title = title;
        }
    }

    fn render_in_rect(&mut self, rect: Rect) -> String {
        self.config.width.fixed_width = Some(rect.width.max(6) as usize);
        self.config.fixed_height = Some(rect.height.max(3) as usize);
        render_to_string(&self.config)
    }
}

struct BoxyDashboardPlugin {
    panels: Vec<BoxyPanel>,
    focused: usize,
    tick_counter: u32,
    elapsed_since_update: Duration,
}

impl BoxyDashboardPlugin {
    fn new_panels() -> Vec<BoxyPanel> {
        let status_panel = BoxyPanel::new(
            PANEL_STATUS_ZONE,
            BoxyConfig {
                text: "Status: BOOTING\nCPU: --\nRAM: --".to_string(),
                title: Some(format!("{} Status", glyph("gear"))),
                colors: BoxColors {
                    box_color: "blue".to_string(),
                    text_color: "auto".to_string(),
                    title_color: Some("white".to_string()),
                    ..Default::default()
                },
                width: WidthConfig::default(),
                ..Default::default()
            },
            "blue",
            "yellow",
        );

        let metrics_panel = BoxyPanel::new(
            PANEL_METRICS_ZONE,
            BoxyConfig {
                text: metrics_text(0),
                title: Some(format!("{} Metrics", glyph("bolt"))),
                colors: BoxColors {
                    box_color: "purple".to_string(),
                    text_color: "auto".to_string(),
                    title_color: Some("white".to_string()),
                    ..Default::default()
                },
                width: WidthConfig::default(),
                ..Default::default()
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
                    ..Default::default()
                },
                width: WidthConfig::default(),
                ..Default::default()
            },
            "cyan",
            "green",
        );
        activity_panel.append_line("Awaiting updates...", 6);

        vec![status_panel, metrics_panel, activity_panel]
    }

    fn render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(
            HEADER_ZONE,
            format!(
                "{} Room + Boxy Integration · Focus with Tab / Ctrl+R to force refresh",
                glyph("light")
            ),
        );

        for panel in &mut self.panels {
            if let Some(rect) = ctx.rect(panel.id()) {
                let rendered = panel.render_in_rect(*rect);
                ctx.set_zone_pre_rendered(panel.id(), rendered);
            }
        }

        ctx.set_zone(
            FOOTER_ZONE,
            format!(
                "Focused panel: {} · Tick {} · Esc to exit",
                self.panels
                    .get(self.focused)
                    .map(|panel| panel.id())
                    .unwrap_or("(none)"),
                self.tick_counter
            ),
        );

        if let Some(rect) = ctx
            .rect(self.panels[self.focused].id())
            .copied()
            .map(|rect| (rect.y.saturating_add(1), rect.x.saturating_add(2)))
        {
            ctx.set_cursor_hint(rect.0, rect.1);
        }

        Ok(())
    }

    fn rotate_focus(&mut self, direction: i32) {
        if self.panels.is_empty() {
            return;
        }
        self.panels[self.focused].set_focus(false);
        if direction.is_negative() {
            if self.focused == 0 {
                self.focused = self.panels.len() - 1;
            } else {
                self.focused -= 1;
            }
        } else {
            self.focused = (self.focused + 1) % self.panels.len();
        }
        self.panels[self.focused].set_focus(true);
    }
}

impl Default for BoxyDashboardPlugin {
    fn default() -> Self {
        let mut panels = Self::new_panels();
        if let Some(first) = panels.first_mut() {
            first.set_focus(true);
        }
        Self {
            panels,
            focused: 0,
            tick_counter: 0,
            elapsed_since_update: Duration::default(),
        }
    }
}

impl RoomPlugin for BoxyDashboardPlugin {
    fn name(&self) -> &str {
        "boxy_dashboard"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.render(ctx)
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        let mut needs_render = false;

        match event {
            RuntimeEvent::Tick { elapsed } => {
                self.elapsed_since_update += *elapsed;
                if self.elapsed_since_update >= STATUS_CYCLE {
                    self.elapsed_since_update = Duration::default();
                    self.tick_counter = self.tick_counter.wrapping_add(1);
                    apply_status_cycle(&mut self.panels, self.tick_counter);
                    needs_render = true;
                }
            }
            RuntimeEvent::Resize(_) => needs_render = true,
            RuntimeEvent::Key(key) if key.kind == KeyEventKind::Press => {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    ctx.request_exit();
                    return Ok(EventFlow::Consumed);
                }
                match key.code {
                    KeyCode::Esc => {
                        ctx.request_exit();
                        return Ok(EventFlow::Consumed);
                    }
                    KeyCode::Tab => {
                        self.rotate_focus(1);
                        needs_render = true;
                    }
                    KeyCode::BackTab => {
                        self.rotate_focus(-1);
                        needs_render = true;
                    }
                    KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.tick_counter = self.tick_counter.wrapping_add(1);
                        apply_status_cycle(&mut self.panels, self.tick_counter);
                        needs_render = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if needs_render {
            self.render(ctx)?;
        }

        Ok(EventFlow::Continue)
    }
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

fn apply_status_cycle(panels: &mut [BoxyPanel], tick: u32) {
    if let Some(status_panel) = panels.iter_mut().find(|p| p.id() == PANEL_STATUS_ZONE) {
        let (status_text, color, title) = match tick % 4 {
            0 => ("Status: STABLE\nCPU: 37%\nRAM: 1.1GB", "blue", "Status"),
            1 => ("Status: BUSY\nCPU: 63%\nRAM: 1.6GB", "yellow", "Status"),
            2 => ("Status: HOT\nCPU: 82%\nRAM: 2.2GB", "red", "Status"),
            _ => ("Status: COOL\nCPU: 28%\nRAM: 0.9GB", "green", "Status"),
        };
        status_panel.replace_text(format!("{}\n{}", status_text, tick_line(tick)));
        status_panel.config.colors.box_color = color.to_string();
        status_panel.update_title(Some(format!("{} {}", glyph("gear"), title)));
    }

    if let Some(metrics_panel) = panels.iter_mut().find(|p| p.id() == PANEL_METRICS_ZONE) {
        metrics_panel.replace_text(metrics_text(tick));
    }

    if let Some(activity_panel) = panels.iter_mut().find(|p| p.id() == PANEL_LOG_ZONE) {
        let message = match tick % 4 {
            0 => "System heartbeat OK",
            1 => "Workload batch submitted",
            2 => "Autoscaler responding",
            _ => "Cleanup cycle complete",
        };
        activity_panel.append_line(message, 6);
    }
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(1),
            Constraint::Flex(1),
            Constraint::Fixed(1),
        ],
        children: vec![
            LayoutNode::leaf(HEADER_ZONE),
            LayoutNode {
                id: "app:demo.body".into(),
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
            LayoutNode::leaf(FOOTER_ZONE),
        ],
        gap: 1,
        padding: 0,
    })
}
