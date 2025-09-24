//! Room Workshop: Debug Zone Explorer
//!
//! Demonstrates how to surface runtime dirty-zone activity by logging every
//! `set_zone` call into a dedicated debug panel. Use the editor to type content,
//! tab between focus targets, and watch the debug log update as zones change.
//!
//! ```bash
//! cargo run --example workshop_debug_zone
//! ```

use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::runtime::focus::{FocusController, ensure_focus_registry};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, LegacyScreenStrategy,
    Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, ScreenDefinition,
    ScreenManager, Size,
};

const INSTRUCTION_ZONE: &str = "workshop:debug.instructions";
const EDITOR_ZONE: &str = "workshop:debug.editor";
const STATUS_ZONE: &str = "workshop:debug.status";
const DEBUG_ZONE: &str = "workshop:debug.log";
const MAX_LOG_LINES: usize = 12;
const MAX_EVENT_COUNTER: usize = 999;

fn main() -> Result<()> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.default_focus_zone = Some(EDITOR_ZONE.to_string());

    let mut runtime =
        RoomRuntime::with_config(layout.clone(), renderer, Size::new(100, 28), config)?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "workshop-debug-Z",
        "Debug Zone Workshop",
        std::sync::Arc::new(move || Box::new(LegacyScreenStrategy::new(layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("workshop-debug-Z")?;

    runtime.register_plugin(DebugZoneWorkshop::default());

    CliDriver::new(runtime)
        .run()
        .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "workshop:debug.root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(4), // instructions
            Constraint::Flex(1),  // editor
            Constraint::Fixed(3), // status
            Constraint::Fixed(8), // debug log
        ],
        children: vec![
            LayoutNode::leaf(INSTRUCTION_ZONE),
            LayoutNode::leaf(EDITOR_ZONE),
            LayoutNode::leaf(STATUS_ZONE),
            LayoutNode::leaf(DEBUG_ZONE),
        ],
        gap: 1,
        padding: 1,
    })
}

#[derive(Default)]
struct DebugZoneWorkshop {
    editor_text: String,
    cursor_col: usize,
    status: String,
    log: VecDeque<String>,
    focus: FocusState,
    focus_controller: Option<FocusController>,
    event_counter: usize,
}

enum FocusState {
    Editor,
    Status,
}

impl Default for FocusState {
    fn default() -> Self {
        FocusState::Editor
    }
}

impl DebugZoneWorkshop {
    fn ensure_focus_controller(
        &mut self,
        ctx: &RuntimeContext<'_>,
    ) -> Result<&mut FocusController> {
        if self.focus_controller.is_none() {
            let registry = ensure_focus_registry(ctx)
                .map_err(|err| room_mvp::LayoutError::Backend(format!("focus registry: {err}")))?;
            self.focus_controller = Some(FocusController::new("debug-workshop", registry));
        }
        Ok(self
            .focus_controller
            .as_mut()
            .expect("focus controller present"))
    }

    fn log_dirty(&mut self, ctx: &mut RuntimeContext<'_>, zone: &str, reason: &str) {
        self.event_counter = if self.event_counter >= MAX_EVENT_COUNTER {
            1
        } else {
            self.event_counter + 1
        };

        let entry = format!("{:03} | {} -> {}", self.event_counter, zone, reason);
        self.log.push_back(entry);
        while self.log.len() > MAX_LOG_LINES {
            self.log.pop_front();
        }
        let mut joined = String::from("Dirty Zone Log\n================\n");
        for line in &self.log {
            joined.push_str(line);
            joined.push('\n');
        }
        ctx.set_zone(DEBUG_ZONE, joined);
    }

    fn refresh_instructions(&mut self, ctx: &mut RuntimeContext<'_>) {
        let instructions = "Debug Zone Workshop\n\
            ---------------------\n\
            • Type to update the editor zone.\n\
            • Press Tab to toggle focus between the editor and status footer.\n\
            • Scroll the mouse wheel to log mouse events.\n\
            • Press Ctrl+L to clear the debug log.\n\
            • Esc or Ctrl+Q to exit.\n";
        ctx.set_zone(INSTRUCTION_ZONE, instructions);
        self.log_dirty(ctx, INSTRUCTION_ZONE, "Initial render");
    }

    fn refresh_editor(&mut self, ctx: &mut RuntimeContext<'_>) {
        let mut content = self.editor_text.clone();
        content.push_str("\n\n[Type here]");
        ctx.set_zone(EDITOR_ZONE, content);
        self.log_dirty(ctx, EDITOR_ZONE, "Editor updated");
    }

    fn refresh_status(&mut self, ctx: &mut RuntimeContext<'_>, reason: &str) {
        let focus_label = match self.focus {
            FocusState::Editor => "Focus: Editor",
            FocusState::Status => "Focus: Status Bar",
        };
        let footer = format!("{focus_label}\nStatus: {}", self.status);
        ctx.set_zone(STATUS_ZONE, footer);
        self.log_dirty(ctx, STATUS_ZONE, reason);
    }

    fn toggle_focus(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.focus = match self.focus {
            FocusState::Editor => FocusState::Status,
            FocusState::Status => FocusState::Editor,
        };

        let target = match self.focus {
            FocusState::Editor => EDITOR_ZONE,
            FocusState::Status => STATUS_ZONE,
        };
        let controller = self.ensure_focus_controller(ctx)?;
        controller.focus(target);
        self.status = format!("[Focus] {} now has keyboard focus", target);
        let status_msg = self.status.clone();
        self.refresh_status(ctx, status_msg.as_str());
        Ok(())
    }

    fn handle_char(&mut self, ch: char) {
        self.editor_text.push(ch);
        self.cursor_col += 1;
        self.status = format!("[Key] Inserted '{ch}' at column {}", self.cursor_col);
    }

    fn handle_backspace(&mut self) {
        if !self.editor_text.is_empty() {
            self.editor_text.pop();
            if self.cursor_col > 0 {
                self.cursor_col -= 1;
            }
            self.status = "[Key] Deleted last character".into();
        } else {
            self.status = "[Key] Nothing left to delete".into();
        }
    }

    fn clear_log(&mut self, ctx: &mut RuntimeContext<'_>) {
        self.log.clear();
        self.event_counter = 0;
        self.status = "[Action] Cleared debug log".into();
        ctx.set_zone(DEBUG_ZONE, "Dirty Zone Log\n================\n<empty>\n");
        // Do not record this reset as a dirty event to avoid recursion into log.
    }

    fn update_all(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.refresh_instructions(ctx);
        self.refresh_editor(ctx);
        self.refresh_status(ctx, "Initial status");
        Ok(())
    }
}

impl RoomPlugin for DebugZoneWorkshop {
    fn name(&self) -> &str {
        "workshop_debug_zone"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.update_all(ctx)
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if let RuntimeEvent::Key(key_event) = event {
            if key_event.kind != KeyEventKind::Press {
                return Ok(room_mvp::EventFlow::Continue);
            }

            if key_event.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key_event.code, KeyCode::Char('q') | KeyCode::Char('c'))
            {
                ctx.request_exit();
                return Ok(room_mvp::EventFlow::Consumed);
            }

            if key_event.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key_event.code, KeyCode::Char('l'))
            {
                self.clear_log(ctx);
                let status_msg = self.status.clone();
                self.refresh_status(ctx, status_msg.as_str());
                return Ok(room_mvp::EventFlow::Consumed);
            }

            let mut touched_editor = false;
            let mut handled_status = false;

            match key_event.code {
                KeyCode::Tab | KeyCode::BackTab => {
                    self.toggle_focus(ctx)?;
                    handled_status = true;
                }
                KeyCode::Backspace => {
                    self.handle_backspace();
                    touched_editor = true;
                }
                KeyCode::Enter => {
                    self.editor_text.push('\n');
                    self.cursor_col = 0;
                    self.status = "[Key] Inserted newline".into();
                    touched_editor = true;
                }
                KeyCode::Char(ch) => {
                    self.handle_char(ch);
                    touched_editor = true;
                }
                KeyCode::Esc => {
                    ctx.request_exit();
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                _ => {}
            }

            if touched_editor {
                self.refresh_editor(ctx);
            }
            if !handled_status {
                let status_msg = self.status.clone();
                self.refresh_status(ctx, status_msg.as_str());
            }
            return Ok(room_mvp::EventFlow::Consumed);
        }

        if let RuntimeEvent::Mouse(mouse_event) = event {
            self.status = format!("[Mouse] {:?}", mouse_event);
            let status_msg = self.status.clone();
            self.refresh_status(ctx, status_msg.as_str());
            return Ok(room_mvp::EventFlow::Consumed);
        }

        Ok(room_mvp::EventFlow::Continue)
    }
}
