//! Room Workshop: Focus Event Signals
//!
//! Demonstrates how `FocusController` drives focus ownership and how
//! `FocusChanged` events surface through the runtime lifecycle. Use Tab/Shift+Tab
//! to move focus between panes, and press Space to release focus entirely.
//!
//! ```bash
//! cargo run --example workshop_focus_events
//! ```

use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::runtime::FocusChange;
use room_mvp::runtime::focus::{FocusController, ensure_focus_registry};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, LegacyScreenStrategy,
    Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, ScreenDefinition,
    ScreenManager, Size,
};

const INSTRUCTIONS_ZONE: &str = "workshop:focus.instructions";
const PRIMARY_A_ZONE: &str = "workshop:focus.primary_a";
const SECONDARY_ZONE: &str = "workshop:focus.secondary";
const TERTIARY_ZONE: &str = "workshop:focus.tertiary";
const LOG_ZONE: &str = "workshop:focus.log";
const MAX_LOG_LINES: usize = 12;

fn main() -> Result<()> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.default_focus_zone = Some(PRIMARY_A_ZONE.to_string());

    let mut runtime =
        RoomRuntime::with_config(layout.clone(), renderer, Size::new(90, 24), config)?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "workshop-focus-events",
        "Focus Event Signals",
        std::sync::Arc::new(move || Box::new(LegacyScreenStrategy::new(layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("workshop-focus-events")?;

    runtime.register_plugin(FocusEventWorkshop::default());

    CliDriver::new(runtime)
        .run()
        .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "workshop:focus.root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(5), // instructions
            Constraint::Flex(1),  // primary/secondary panes
            Constraint::Fixed(8), // log output
        ],
        children: vec![
            LayoutNode::leaf(INSTRUCTIONS_ZONE),
            LayoutNode {
                id: "workshop:focus.split".into(),
                direction: Direction::Row,
                constraints: vec![
                    Constraint::Flex(1),
                    Constraint::Flex(1),
                    Constraint::Flex(1),
                ],
                children: vec![
                    LayoutNode::leaf(PRIMARY_A_ZONE),
                    LayoutNode::leaf(SECONDARY_ZONE),
                    LayoutNode::leaf(TERTIARY_ZONE),
                ],
                gap: 1,
                padding: 0,
            },
            LayoutNode::leaf(LOG_ZONE),
        ],
        gap: 1,
        padding: 1,
    })
}

#[derive(Default)]
struct FocusEventWorkshop {
    focus: FocusPane,
    controller: Option<FocusController>,
    log: VecDeque<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusPane {
    PrimaryA,
    Secondary,
    Tertiary,
    None,
}

impl Default for FocusPane {
    fn default() -> Self {
        FocusPane::PrimaryA
    }
}

impl FocusEventWorkshop {
    fn ensure_controller(&mut self, ctx: &RuntimeContext<'_>) -> Result<&mut FocusController> {
        if self.controller.is_none() {
            let registry = ensure_focus_registry(ctx)
                .map_err(|err| room_mvp::LayoutError::Backend(format!("focus registry: {err}")))?;
            self.controller = Some(FocusController::new("focus-workshop", registry));
        }
        Ok(self
            .controller
            .as_mut()
            .expect("focus controller initialized"))
    }

    fn render(&mut self, ctx: &mut RuntimeContext<'_>) {
        ctx.set_zone(INSTRUCTIONS_ZONE, instructions());
        ctx.set_zone(PRIMARY_A_ZONE, self.render_primary_a());
        ctx.set_zone(SECONDARY_ZONE, self.render_secondary());
        ctx.set_zone(TERTIARY_ZONE, self.render_tertiary());
        self.flush_log(ctx);
    }

    fn render_primary_a(&self) -> String {
        let header = match self.focus {
            FocusPane::PrimaryA => "[Focus] PrimaryA Pane",
            _ => "PrimaryA Pane",
        };
        let mut lines = Vec::new();
        lines.push(header.to_string());
        lines.push("--------------".into());
        lines.push("Tab -> Secondary".into());
        lines.push("Shift+Tab -> Review".into());
        while lines.len() < 6 {
            lines.push(String::new());
        }
        lines.join("\n")
    }

    fn render_secondary(&self) -> String {
        let header = match self.focus {
            FocusPane::Secondary => "[Focus] Secondary Pane",
            _ => "Secondary Pane",
        };
        let mut lines = Vec::new();
        lines.push(header.to_string());
        lines.push("------------".into());
        lines.push("Tab -> Review".into());
        lines.push("Shift+Tab -> Primary".into());
        while lines.len() < 6 {
            lines.push(String::new());
        }
        lines.join("\n")
    }

    fn render_tertiary(&self) -> String {
        let header = match self.focus {
            FocusPane::Tertiary => "[Focus] Review Pane",
            _ => "Review Pane",
        };
        let mut lines = Vec::new();
        lines.push(header.to_string());
        lines.push("-----------".into());
        lines.push("Tab -> Primary".into());
        lines.push("Shift+Tab -> Secondary".into());
        while lines.len() < 6 {
            lines.push(String::new());
        }
        lines.join("\n")
    }

    fn log_event(&mut self, message: impl Into<String>) {
        if self.log.len() == MAX_LOG_LINES {
            self.log.pop_front();
        }
        self.log.push_back(message.into());
    }

    fn flush_log(&self, ctx: &mut RuntimeContext<'_>) {
        let mut output = String::from("Focus Event Log\n================\n");
        for line in &self.log {
            output.push_str(line);
            output.push('\n');
        }
        ctx.set_zone(LOG_ZONE, output);
    }

    fn focus_next(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        match self.focus {
            FocusPane::PrimaryA => self.set_focus(ctx, FocusPane::Secondary),
            FocusPane::Secondary => self.set_focus(ctx, FocusPane::Tertiary),
            FocusPane::Tertiary | FocusPane::None => self.set_focus(ctx, FocusPane::PrimaryA),
        }
    }

    fn focus_prev(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        match self.focus {
            FocusPane::Tertiary => self.set_focus(ctx, FocusPane::Secondary),
            FocusPane::Secondary => self.set_focus(ctx, FocusPane::PrimaryA),
            FocusPane::PrimaryA | FocusPane::None => self.set_focus(ctx, FocusPane::Tertiary),
        }
    }

    fn set_focus(&mut self, ctx: &mut RuntimeContext<'_>, pane: FocusPane) -> Result<()> {
        let controller = self.ensure_controller(ctx)?;
        match pane {
            FocusPane::PrimaryA => controller.focus(PRIMARY_A_ZONE),
            FocusPane::Secondary => controller.focus(SECONDARY_ZONE),
            FocusPane::Tertiary => controller.focus(TERTIARY_ZONE),
            FocusPane::None => controller.release(),
        }
        Ok(())
    }

    fn release_focus(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        let controller = self.ensure_controller(ctx)?;
        controller.release();
        Ok(())
    }
}

impl RoomPlugin for FocusEventWorkshop {
    fn name(&self) -> &str {
        "focus_event_workshop"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.log_event("[Init] Rendering focus panes");
        self.render(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if let RuntimeEvent::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(room_mvp::EventFlow::Continue);
            }

            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
                ctx.request_exit();
                return Ok(room_mvp::EventFlow::Consumed);
            }

            match key.code {
                KeyCode::Esc => {
                    ctx.request_exit();
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Tab => {
                    self.focus_next(ctx)?;
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::BackTab => {
                    self.focus_prev(ctx)?;
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Char(' ') => {
                    self.release_focus(ctx)?;
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                _ => {}
            }
        }

        Ok(room_mvp::EventFlow::Continue)
    }

    fn on_focus_change(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        change: &FocusChange,
    ) -> Result<()> {
        let from = change
            .from
            .as_ref()
            .map(|target| target.zone.clone())
            .unwrap_or_else(|| "<none>".to_string());
        let to = change
            .to
            .as_ref()
            .map(|target| target.zone.clone())
            .unwrap_or_else(|| "<none>".to_string());
        self.log_event(format!("[FocusChanged] {from} -> {to}"));

        self.focus = match change.to.as_ref().map(|target| target.zone.as_str()) {
            Some(PRIMARY_A_ZONE) => FocusPane::PrimaryA,
            Some(SECONDARY_ZONE) => FocusPane::Secondary,
            Some(TERTIARY_ZONE) => FocusPane::Tertiary,
            _ => FocusPane::None,
        };

        self.render(ctx);
        Ok(())
    }
}

fn instructions() -> &'static str {
    "Focus Event Workshop\n\
    ---------------------\n\
    - Tab cycles focus forward across PrimaryA → Secondary → Review.\n\
    - Shift+Tab cycles focus backward.\n\
    - Space releases focus (FocusChanged to <none>).\n\
    - Esc or Ctrl+Q exits.\n"
}
