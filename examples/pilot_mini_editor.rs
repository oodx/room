//! Room Pilot: Mini Text Editor
//!
//! Demonstrates Room's novel zone-based architecture for building a terminal text editor.
//! This example showcases:
//!
//! * **Zone-based layout**: Line numbers, editor content, status bar as separate zones
//! * **Token-driven updates**: Uses Room's zone token system for content updates
//! * **Plugin-based features**: Modular editor features as Room plugins
//! * **Focus management**: Zone navigation and cursor control
//! * **Novel Room patterns**: Leverages Room's unique architecture
//!
//! ```bash
//! cargo run --example pilot_mini_editor
//! ```
//!
//! Controls:
//! - Arrow keys: Navigate cursor
//! - Typing: Insert characters
//! - Enter: New line
//! - Backspace: Delete character
//! - Ctrl+Q: Quit

use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, LayoutNode, LayoutTree,
    LegacyScreenStrategy, Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext,
    RuntimeEvent, ScreenDefinition, ScreenManager, Size, cursor,
};

// Editor zone definitions - showcase Room's zone-based architecture
const LINE_NUMBERS_ZONE: &str = "editor:line_numbers";
const CONTENT_ZONE: &str = "editor:content";
const STATUS_ZONE: &str = "editor:status";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Room Pilot · Mini Text Editor");
    println!("Demonstrating zone-based editor architecture\n");

    let layout = build_editor_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.default_focus_zone = Some(CONTENT_ZONE.to_string());

    let mut runtime = RoomRuntime::with_config(layout.clone(), renderer, Size::new(100, 30), config)?;

    // Set up screen manager - REQUIRED for proper zone initialization
    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "editor",
        "Mini Editor",
        Arc::new(move || Box::new(LegacyScreenStrategy::new(layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("editor")?;

    // Register editor plugin to demonstrate Room's plugin system
    // A single plugin handles all zones to ensure coordinated updates
    runtime.register_plugin(EditorCorePlugin::new());

    // Don't use CLI bundle since we're handling everything ourselves
    // The CLI bundle would conflict with our custom input handling

    // Run the main event loop with the driver
    // The CliDriver will handle the first paint automatically
    CliDriver::new(runtime).run()?;
    Ok(())
}

/// Novel Room layout: demonstrates multi-zone editor architecture
fn build_editor_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "editor:root".into(),
        direction: Direction::Row,
        constraints: vec![Constraint::Fixed(6), Constraint::Flex(1)],
        children: vec![
            // Line numbers zone - fixed width
            LayoutNode::leaf(LINE_NUMBERS_ZONE),
            // Main editor area - flexible column
            LayoutNode {
                id: "editor:main".into(),
                direction: Direction::Column,
                constraints: vec![Constraint::Flex(1), Constraint::Fixed(1)],
                children: vec![
                    LayoutNode::leaf(CONTENT_ZONE),
                    LayoutNode::leaf(STATUS_ZONE),
                ],
                gap: 0,
                padding: 0,
            },
        ],
        gap: 0,
        padding: 0,
    })
}

/// Simple editor state - demonstrates Room's plugin state management
#[derive(Debug)]
struct EditorState {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    version: u64, // For tracking changes
}

impl EditorState {
    fn new() -> Self {
        Self {
            lines: vec![
                "Welcome to Room Mini Editor!".to_string(),
                "".to_string(),
                "This demonstrates Room's zone-based architecture:".to_string(),
                "• Line numbers update independently".to_string(),
                "• Content zone handles text rendering".to_string(),
                "• Status bar shows editor state".to_string(),
                "".to_string(),
                "Type to edit, arrow keys to navigate, Ctrl+Q to quit.".to_string(),
                "".to_string(),
            ],
            cursor_row: 0,  // Start at line 1 (0-indexed)
            cursor_col: 0,
            version: 0,
        }
    }

    fn move_cursor(&mut self, direction: CursorDirection) {
        match direction {
            CursorDirection::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                }
            }
            CursorDirection::Down => {
                if self.cursor_row < self.lines.len() - 1 {
                    self.cursor_row += 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                }
            }
            CursorDirection::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                }
            }
            CursorDirection::Right => {
                if self.cursor_col < self.lines[self.cursor_row].len() {
                    self.cursor_col += 1;
                } else if self.cursor_row < self.lines.len() - 1 {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
            }
        }
        self.version += 1;
    }

    fn insert_char(&mut self, ch: char) {
        self.lines[self.cursor_row].insert(self.cursor_col, ch);
        self.cursor_col += 1;
        self.version += 1;
    }

    fn insert_newline(&mut self) {
        let current_line = self.lines[self.cursor_row].clone();
        let (left, right) = current_line.split_at(self.cursor_col);

        self.lines[self.cursor_row] = left.to_string();
        self.lines.insert(self.cursor_row + 1, right.to_string());
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.version += 1;
    }

    fn delete_char(&mut self) {
        if self.cursor_col > 0 {
            self.lines[self.cursor_row].remove(self.cursor_col - 1);
            self.cursor_col -= 1;
            self.version += 1;
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.version += 1;
        }
    }

    /// Generate content without cursor markers - let Room handle cursor positioning
    fn render_content(&self) -> String {
        self.lines.join("\n")
    }

    /// Calculate cursor position relative to content zone
    fn cursor_position(&self) -> (u16, u16) {
        (self.cursor_row as u16, self.cursor_col as u16)
    }

    /// Generate line numbers - demonstrates independent zone updates
    fn render_line_numbers(&self) -> String {
        (1..=self.lines.len())
            .map(|n| format!("{:>4} ", n))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate status line - shows Room's real-time updates
    fn render_status(&self) -> String {
        format!(
            "──[ Line {}, Col {} | {} lines | v{} | Ctrl+Q to quit ]──",
            self.cursor_row + 1,
            self.cursor_col + 1,
            self.lines.len(),
            self.version
        )
    }
}

#[derive(Debug, Clone)]
enum CursorDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Core editor plugin - demonstrates Room's plugin architecture
struct EditorCorePlugin {
    state: Arc<Mutex<EditorState>>,
}

impl EditorCorePlugin {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(EditorState::new())),
        }
    }

    /// Update all editor zones - showcases Room's zone coordination
    fn update_all_zones(&self, ctx: &mut RuntimeContext) {
        if let Ok(state) = self.state.lock() {
            // Update line numbers zone
            ctx.set_zone(LINE_NUMBERS_ZONE, state.render_line_numbers());

            // Update content zone without cursor markers
            ctx.set_zone(CONTENT_ZONE, state.render_content());

            // Update status zone
            ctx.set_zone(STATUS_ZONE, state.render_status());

            ctx.request_render();
        }
    }

    /// Set cursor position after content is rendered
    fn update_cursor_position(&self, ctx: &mut RuntimeContext) {
        if let Ok(state) = self.state.lock() {
            // Get the content zone rect to calculate absolute screen coordinates
            if let Some(content_rect) = ctx.rect(CONTENT_ZONE) {
                let (cursor_row, cursor_col) = state.cursor_position();
                // Convert zone-relative coordinates to absolute screen coordinates
                let absolute_row = content_rect.y + cursor_row;
                let absolute_col = content_rect.x + cursor_col;
                ctx.set_cursor_hint(absolute_row, absolute_col);
            }
        }
    }

    /// Show the cursor once - CLI driver hides it by default
    fn show_cursor_once(&self, ctx: &mut RuntimeContext) {
        // Use a hidden zone just to inject the cursor show command
        // This ensures it only happens once at startup
        ctx.set_zone("editor:cursor_show", cursor::show());
    }
}

impl RoomPlugin for EditorCorePlugin {
    fn name(&self) -> &str {
        "editor_core"
    }

    fn init(&mut self, ctx: &mut RuntimeContext) -> Result<()> {
        // Initial zone population - Room's startup pattern
        self.update_all_zones(ctx);

        // Show cursor and set position after content is rendered
        self.show_cursor_once(ctx);
        self.update_cursor_position(ctx);
        Ok(())
    }

    fn on_event(&mut self, ctx: &mut RuntimeContext, event: &RuntimeEvent) -> Result<EventFlow> {
        if let RuntimeEvent::Key(key_event) = event {
            if key_event.kind != KeyEventKind::Press {
                return Ok(EventFlow::Continue);
            }

            // Handle quit command
            if key_event.modifiers.contains(KeyModifiers::CONTROL) && key_event.code == KeyCode::Char('q') {
                ctx.request_exit();
                return Ok(EventFlow::Consumed);
            }

            if let Ok(mut state) = self.state.lock() {
                match key_event.code {
                    // Cursor movement - Room's input handling pattern
                    KeyCode::Up => state.move_cursor(CursorDirection::Up),
                    KeyCode::Down => state.move_cursor(CursorDirection::Down),
                    KeyCode::Left => state.move_cursor(CursorDirection::Left),
                    KeyCode::Right => state.move_cursor(CursorDirection::Right),

                    // Text input - Room's character processing
                    KeyCode::Char(ch) => state.insert_char(ch),
                    KeyCode::Enter => state.insert_newline(),
                    KeyCode::Backspace => state.delete_char(),

                    _ => return Ok(EventFlow::Continue),
                }

                // Drop the lock before calling update
                drop(state);

                // Update zones after state change - Room's reactive pattern
                self.update_all_zones(ctx);
                // Don't call show_cursor() on every keystroke - cursor is already shown
                self.update_cursor_position(ctx);
                return Ok(EventFlow::Consumed);
            }
        }

        Ok(EventFlow::Continue)
    }
}

