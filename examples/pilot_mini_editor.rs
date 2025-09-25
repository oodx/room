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
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, LayoutNode, LayoutTree,
    LegacyScreenStrategy, Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext,
    RuntimeEvent, ScreenDefinition, ScreenManager, Size, SimulatedLoop, display_width,
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
    config.tick_interval = Duration::from_millis(16);

    // Support both interactive and headless testing
    let is_headless = std::env::var("CI").is_ok() || std::env::var("HEADLESS").is_ok();
    if is_headless {
        config.simulated_loop = Some(SimulatedLoop::ticks(5));
    }

    let mut runtime =
        RoomRuntime::with_config(layout.clone(), renderer, Size::new(100, 30), config)?;

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

    // Handle both interactive and headless execution
    if is_headless {
        let mut buffer = Vec::new();
        runtime.run(&mut buffer)?;
        println!("{}", String::from_utf8_lossy(&buffer));
        Ok(())
    } else {
        // Run the main event loop; the driver performs a synchronous bootstrap that
        // renders the first frame before entering the event loop.
        CliDriver::new(runtime).run()?;
        Ok(())
    }
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
    viewport_top: usize, // Top visible line for scrolling
}

impl EditorState {
    fn new() -> Self {
        Self {
            lines: {
                let mut lines = vec![
                    "Welcome to Room Mini Editor!".to_string(),
                    "".to_string(),
                    "This demonstrates Room's zone-based architecture:".to_string(),
                    "• Line numbers update independently".to_string(),
                    "• Content zone handles text rendering".to_string(),
                    "• Status bar shows editor state".to_string(),
                    "".to_string(),
                    "Type to edit, arrows/PgUp/PgDn/Home/End to navigate, Ctrl+Q to quit.".to_string(),
                    "".to_string(),
                ];

                // Add more lines to test scrolling
                for i in 10..=50 {
                    lines.push(format!("Line {} - This is a test line to demonstrate viewport scrolling functionality.", i));
                }

                lines
            },
            cursor_row: 0, // Start at line 1 (0-indexed)
            cursor_col: 0,
            version: 0,
            viewport_top: 0,
        }
    }

    fn line_char_count(&self, row: usize) -> usize {
        self.lines[row].chars().count()
    }

    fn byte_offset(line: &str, char_idx: usize) -> usize {
        if char_idx == 0 {
            return 0;
        }
        let mut iter = line.char_indices();
        iter.nth(char_idx)
            .map(|(idx, _)| idx)
            .unwrap_or_else(|| line.len())
    }

    fn cursor_display_column(&self) -> usize {
        let line = &self.lines[self.cursor_row];
        let byte_idx = Self::byte_offset(line, self.cursor_col);
        display_width(&line[..byte_idx]) as usize
    }

    fn move_cursor(&mut self, direction: CursorDirection) {
        match direction {
            CursorDirection::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    let max_col = self.line_char_count(self.cursor_row);
                    self.cursor_col = self.cursor_col.min(max_col);
                }
            }
            CursorDirection::Down => {
                if self.cursor_row < self.lines.len() - 1 {
                    self.cursor_row += 1;
                    let max_col = self.line_char_count(self.cursor_row);
                    self.cursor_col = self.cursor_col.min(max_col);
                }
            }
            CursorDirection::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.line_char_count(self.cursor_row);
                }
            }
            CursorDirection::Right => {
                let line_len = self.line_char_count(self.cursor_row);
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_row < self.lines.len() - 1 {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
            }
            CursorDirection::PageUp => {
                const PAGE_SIZE: usize = 20; // Move by ~page size
                if self.cursor_row >= PAGE_SIZE {
                    self.cursor_row -= PAGE_SIZE;
                } else {
                    self.cursor_row = 0;
                }
                // Keep cursor within line bounds
                let max_col = self.line_char_count(self.cursor_row);
                self.cursor_col = self.cursor_col.min(max_col);
            }
            CursorDirection::PageDown => {
                const PAGE_SIZE: usize = 20; // Move by ~page size
                if self.cursor_row + PAGE_SIZE < self.lines.len() {
                    self.cursor_row += PAGE_SIZE;
                } else {
                    self.cursor_row = self.lines.len() - 1;
                }
                // Keep cursor within line bounds
                let max_col = self.line_char_count(self.cursor_row);
                self.cursor_col = self.cursor_col.min(max_col);
            }
            CursorDirection::Home => {
                self.cursor_col = 0; // Move to start of current line
            }
            CursorDirection::End => {
                self.cursor_col = self.line_char_count(self.cursor_row); // Move to end of current line
            }
        }
        self.version += 1;
        // Update viewport to keep cursor visible
        self.update_viewport_to_follow_cursor();
    }

    /// Update viewport to keep cursor visible
    fn update_viewport_to_follow_cursor(&mut self) {
        const VIEWPORT_HEIGHT: usize = 25; // Approximate visible lines (30 total - status - line numbers)

        // Scroll up if cursor is above viewport
        if self.cursor_row < self.viewport_top {
            self.viewport_top = self.cursor_row;
        }
        // Scroll down if cursor is below viewport
        else if self.cursor_row >= self.viewport_top + VIEWPORT_HEIGHT {
            self.viewport_top = self.cursor_row - VIEWPORT_HEIGHT + 1;
        }

        // Ensure viewport_top doesn't go below 0 or beyond document
        self.viewport_top = self.viewport_top.min(self.lines.len().saturating_sub(VIEWPORT_HEIGHT));
    }

    fn insert_char(&mut self, ch: char) {
        let line = &mut self.lines[self.cursor_row];
        let byte_idx = Self::byte_offset(line, self.cursor_col);
        line.insert(byte_idx, ch);
        self.cursor_col += 1;
        self.version += 1;
        self.update_viewport_to_follow_cursor();
    }

    fn insert_newline(&mut self) {
        let current_line = self.lines[self.cursor_row].clone();
        let byte_idx = Self::byte_offset(&current_line, self.cursor_col);
        let left = current_line[..byte_idx].to_string();
        let right = current_line[byte_idx..].to_string();

        self.lines[self.cursor_row] = left;
        self.lines.insert(self.cursor_row + 1, right);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.version += 1;
        self.update_viewport_to_follow_cursor();
    }

    fn delete_char(&mut self) {
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];
            let start = Self::byte_offset(line, self.cursor_col - 1);
            let end = Self::byte_offset(line, self.cursor_col);
            line.replace_range(start..end, "");
            self.cursor_col -= 1;
            self.version += 1;
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            let previous_len = self.line_char_count(self.cursor_row);
            self.cursor_col = previous_len;
            self.lines[self.cursor_row].push_str(&current_line);
            self.version += 1;
        }
    }

    /// Calculate cursor position relative to content zone (viewport)
    fn cursor_position(&self) -> (u16, u16) {
        let viewport_row = self.cursor_row.saturating_sub(self.viewport_top);
        (viewport_row as u16, self.cursor_display_column() as u16)
    }

    /// Produce display content with an ANSI-highlighted caret that preserves cell width.
    fn render_content_with_highlight(&self) -> String {
        const VIEWPORT_HEIGHT: usize = 25;
        let mut buffer = String::new();

        // Only render visible viewport slice
        let viewport_end = (self.viewport_top + VIEWPORT_HEIGHT).min(self.lines.len());
        let visible_lines = &self.lines[self.viewport_top..viewport_end];

        for (viewport_idx, line) in visible_lines.iter().enumerate() {
            let actual_line_idx = self.viewport_top + viewport_idx;

            if viewport_idx > 0 {
                buffer.push('\n');
            }

            if actual_line_idx == self.cursor_row {
                let byte_idx = Self::byte_offset(line, self.cursor_col);
                let (left, right) = line.split_at(byte_idx);
                buffer.push_str(left);

                // Highlight the glyph under the caret using reverse video.
                buffer.push_str("\x1b[7m");
                if let Some(ch) = right.chars().next() {
                    buffer.push(ch);
                    buffer.push_str("\x1b[0m");
                    buffer.push_str(&right[ch.len_utf8()..]);
                } else {
                    buffer.push(' ');
                    buffer.push_str("\x1b[0m");
                }
            } else {
                buffer.push_str(line);
            }
        }
        buffer
    }

    /// Generate line numbers - demonstrates independent zone updates
    fn render_line_numbers(&self) -> String {
        const VIEWPORT_HEIGHT: usize = 25;
        let viewport_end = (self.viewport_top + VIEWPORT_HEIGHT).min(self.lines.len());

        // Show line numbers only for visible viewport
        (self.viewport_top + 1..=viewport_end)
            .map(|n| format!("{:>4} ", n))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate status line - shows Room's real-time updates
    fn render_status(&self) -> String {
        let display_col = self.cursor_display_column();
        format!(
            "──[ Line {}, Col {} | {} lines | v{} | Ctrl+Q to quit ]──",
            self.cursor_row + 1,
            display_col + 1,
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
    PageUp,
    PageDown,
    Home,
    End,
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

            // Update content using ANSI highlighting without altering layout width.
            ctx.set_zone_pre_rendered(CONTENT_ZONE, state.render_content_with_highlight());

            // Update status zone text
            ctx.set_zone(STATUS_ZONE, state.render_status());

            // Don't request render here - let the caller decide when to render
        }
    }

    /// Set cursor position after content is rendered
    fn update_cursor_position(&self, ctx: &mut RuntimeContext) {
        if let Ok(state) = self.state.lock() {
            // Get the content zone rect to calculate absolute screen coordinates
            if let Some(content_rect) = ctx.rect(CONTENT_ZONE) {
                let (cursor_row, cursor_col) = state.cursor_position();
                // Convert zone-relative coordinates to absolute screen coordinates
                let max_row = content_rect
                    .y
                    .saturating_add(content_rect.height.saturating_sub(1));
                let max_col = content_rect
                    .x
                    .saturating_add(content_rect.width.saturating_sub(1));
                let absolute_row = content_rect.y.saturating_add(cursor_row).min(max_row);
                let absolute_col = content_rect.x.saturating_add(cursor_col).min(max_col);
                ctx.set_cursor_hint(absolute_row, absolute_col);
            }
        }
    }
}

impl RoomPlugin for EditorCorePlugin {
    fn name(&self) -> &str {
        "editor_core"
    }

    fn init(&mut self, _ctx: &mut RuntimeContext) -> Result<()> {
        // Don't populate zones during init - wait for on_user_ready
        // This prevents race conditions during bootstrap
        Ok(())
    }

    fn on_user_ready(&mut self, ctx: &mut RuntimeContext) -> Result<()> {
        // Initial zone population after runtime is ready
        self.update_all_zones(ctx);
        // Set cursor position after content is rendered
        self.update_cursor_position(ctx);
        Ok(())
    }

    fn on_event(&mut self, ctx: &mut RuntimeContext, event: &RuntimeEvent) -> Result<EventFlow> {
        if let RuntimeEvent::Key(key_event) = event {
            if key_event.kind != KeyEventKind::Press {
                return Ok(EventFlow::Continue);
            }

            // Handle quit command
            if key_event.modifiers.contains(KeyModifiers::CONTROL)
                && key_event.code == KeyCode::Char('q')
            {
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
                    KeyCode::PageUp => state.move_cursor(CursorDirection::PageUp),
                    KeyCode::PageDown => state.move_cursor(CursorDirection::PageDown),
                    KeyCode::Home => state.move_cursor(CursorDirection::Home),
                    KeyCode::End => state.move_cursor(CursorDirection::End),

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
                self.update_cursor_position(ctx);
                ctx.request_render(); // Request render after all zones updated
                return Ok(EventFlow::Consumed);
            }
        }

        Ok(EventFlow::Continue)
    }
}
