//! Room Workshop: Cursor Event Signals
//!
//! Showcases how cursor visibility and movement events travel through the runtime.
//! Move the cursor inside the field with the arrow keys, hit Space to toggle
//! visibility, and press `r` to reset the caret position.
//!
//! ```bash
//! cargo run --example workshop_cursor_events
//! ```

use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::runtime::CursorEvent;
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, LegacyScreenStrategy,
    Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, ScreenDefinition,
    ScreenManager, Size,
};

const INSTRUCTIONS_ZONE: &str = "workshop:cursor.instructions";
const FIELD_ZONE: &str = "workshop:cursor.field";
const LOG_ZONE: &str = "workshop:cursor.log";
const MAX_LOG_LINES: usize = 12;
const FIELD_ROWS: i32 = 6;
const FIELD_COLS: i32 = 30;
const FIELD_HEADER_LINES: i32 = 4; // lines before the drawable grid
const FIELD_LEFT_MARGIN: i32 = 2; // spaces before the grid in the field zone

fn main() -> Result<()> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.default_focus_zone = Some(FIELD_ZONE.to_string());

    let mut runtime =
        RoomRuntime::with_config(layout.clone(), renderer, Size::new(100, 26), config)?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "workshop-cursor-events",
        "Cursor Event Signals",
        std::sync::Arc::new(move || Box::new(LegacyScreenStrategy::new(layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("workshop-cursor-events")?;

    runtime.register_plugin(CursorEventWorkshop::default());

    CliDriver::new(runtime)
        .run()
        .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "workshop:cursor.root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(5), // instructions
            Constraint::Flex(1),  // cursor field
            Constraint::Fixed(8), // log output
        ],
        children: vec![
            LayoutNode::leaf(INSTRUCTIONS_ZONE),
            LayoutNode::leaf(FIELD_ZONE),
            LayoutNode::leaf(LOG_ZONE),
        ],
        gap: 1,
        padding: 1,
    })
}

#[derive(Default)]
struct CursorEventWorkshop {
    cursor_row: i32,
    cursor_col: i32,
    cursor_visible: bool,
    log: VecDeque<String>,
}

impl CursorEventWorkshop {
    fn render(&self, ctx: &mut RuntimeContext<'_>) {
        ctx.set_zone(INSTRUCTIONS_ZONE, instructions());
        ctx.set_zone(FIELD_ZONE, self.render_field());
        self.flush_log(ctx);
    }

    fn render_field(&self) -> String {
        let mut output = String::from("Cursor Field\n------------\n");
        output.push_str("Use arrows to move, Space to toggle visibility, r to reset.\n\n");
        output.push_str("  +------------------------------+\n");
        for row in 0..FIELD_ROWS {
            output.push_str("  |");
            for col in 0..FIELD_COLS {
                if row == self.cursor_row && col == self.cursor_col {
                    output.push('X');
                } else {
                    output.push('.');
                }
            }
            output.push_str("|\n");
        }
        output.push_str("  +------------------------------+\n");
        output
    }

    fn log_event(&mut self, message: impl Into<String>) {
        if self.log.len() == MAX_LOG_LINES {
            self.log.pop_front();
        }
        self.log.push_back(message.into());
    }

    fn flush_log(&self, ctx: &mut RuntimeContext<'_>) {
        let mut output = String::from("Cursor Event Log\n================\n");
        for line in &self.log {
            output.push_str(line);
            output.push('\n');
        }
        ctx.set_zone(LOG_ZONE, output);
    }

    fn move_cursor(&mut self, ctx: &mut RuntimeContext<'_>, row_delta: i32, col_delta: i32) {
        self.cursor_row = (self.cursor_row + row_delta).clamp(0, FIELD_ROWS - 1);
        self.cursor_col = (self.cursor_col + col_delta).clamp(0, FIELD_COLS - 1);
        ctx.set_zone(FIELD_ZONE, self.render_field());
        ctx.show_cursor();
        ctx.set_cursor_in_zone(
            FIELD_ZONE,
            FIELD_HEADER_LINES + self.cursor_row,
            FIELD_LEFT_MARGIN + 1 + self.cursor_col,
        );
    }

    fn reset_cursor(&mut self, ctx: &mut RuntimeContext<'_>) {
        self.cursor_row = 0;
        self.cursor_col = 0;
        ctx.set_zone(FIELD_ZONE, self.render_field());
        ctx.show_cursor();
        ctx.set_cursor_in_zone(
            FIELD_ZONE,
            FIELD_HEADER_LINES + self.cursor_row,
            FIELD_LEFT_MARGIN + 1 + self.cursor_col,
        );
    }

    fn toggle_visibility(&mut self, ctx: &mut RuntimeContext<'_>) {
        self.cursor_visible = !self.cursor_visible;
        if self.cursor_visible {
            ctx.show_cursor();
            ctx.set_cursor_in_zone(
                FIELD_ZONE,
                FIELD_HEADER_LINES + self.cursor_row,
                FIELD_LEFT_MARGIN + 1 + self.cursor_col,
            );
        } else {
            ctx.hide_cursor();
        }
    }
}

impl RoomPlugin for CursorEventWorkshop {
    fn name(&self) -> &str {
        "cursor_event_workshop"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.cursor_visible = true;
        self.render(ctx);
        ctx.show_cursor();
        ctx.set_cursor_in_zone(
            FIELD_ZONE,
            FIELD_HEADER_LINES + self.cursor_row,
            FIELD_LEFT_MARGIN + 1 + self.cursor_col,
        );
        self.log_event("[Init] Cursor positioned at origin");
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
                KeyCode::Up => {
                    self.move_cursor(ctx, -1, 0);
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Down => {
                    self.move_cursor(ctx, 1, 0);
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Left => {
                    self.move_cursor(ctx, 0, -1);
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Right => {
                    self.move_cursor(ctx, 0, 1);
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Char('r') => {
                    self.reset_cursor(ctx);
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Char(' ') => {
                    self.toggle_visibility(ctx);
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                _ => {}
            }
        }

        Ok(room_mvp::EventFlow::Continue)
    }

    fn on_cursor_event(&mut self, ctx: &mut RuntimeContext<'_>, event: &CursorEvent) -> Result<()> {
        match event {
            CursorEvent::Moved(cursor) => {
                self.cursor_visible = cursor.visible;
                self.log_event(format!(
                    "[CursorMoved] row={}, col={} visible={}",
                    cursor.position.0, cursor.position.1, cursor.visible
                ));
            }
            CursorEvent::Shown(_) => {
                self.cursor_visible = true;
                self.log_event("[CursorShown]");
            }
            CursorEvent::Hidden(_) => {
                self.cursor_visible = false;
                self.log_event("[CursorHidden]");
            }
        }
        self.flush_log(ctx);
        Ok(())
    }
}

fn instructions() -> &'static str {
    "Cursor Event Workshop\n\
    ----------------------\n\
    - Arrow keys move the caret.\n\
    - Space toggles visibility.\n\
    - r resets to the origin.\n\
    - Esc or Ctrl+Q exits.\n"
}
