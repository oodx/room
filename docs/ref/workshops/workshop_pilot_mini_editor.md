# Room Workshop: Pilot Mini Text Editor

**Demonstrates Room's zone-based architecture for building sophisticated terminal text editors**

## Overview

This workshop walks through `pilot_mini_editor.rs`, a proof-of-concept text editor that showcases Room's unique architectural advantages over traditional TUI frameworks. The example demonstrates zone-based rendering, plugin coordination, screen management, and cursor control.

## Quick Start

```bash
cargo run --example pilot_mini_editor
```

**Controls:**
- Arrow keys: Navigate cursor
- Typing: Insert characters
- Enter: New line
- Backspace: Delete character
- Ctrl+Q: Quit

## Workshop Goals

By the end of this workshop, you'll understand:

1. **Zone-Based Architecture** - Independent rendering zones vs monolithic updates
2. **Screen Management** - Proper initialization lifecycle
3. **Plugin Coordination** - Single plugin managing multiple related zones
4. **Cursor Management** - Terminal cursor visibility and positioning
5. **First-Paint Lifecycle** - Deterministic rendering without race conditions

## Architecture Deep Dive

### 1. Zone-Based Layout System

The editor uses **three independent zones**:

```rust
const LINE_NUMBERS_ZONE: &str = "editor:line_numbers";  // Fixed-width (6 chars)
const CONTENT_ZONE: &str = "editor:content";            // Flexible content area
const STATUS_ZONE: &str = "editor:status";              // Fixed-height (1 row)
```

**Layout Structure:**
```
┌──────┬─────────────────────────────────────────┐
│   1  │ Welcome to Room Mini Editor!            │  <- Line numbers + Content
│   2  │                                         │
│   3  │ This demonstrates Room's zone-based...  │
│   4  │ • Line numbers update independently     │
│   5  │ • Content zone handles text rendering   │
│   6  │ • Status bar shows editor state         │
│   7  │                                         │
│   8  │ Type to edit, arrow keys to navigate... │
│   9  │                                         │
├──────┴─────────────────────────────────────────┤
│ ──[ Line 1, Col 1 | 9 lines | v0 | Ctrl+Q ]── │  <- Status bar
└─────────────────────────────────────────────────┘
```

**Key Insight:** Each zone updates independently. When you type a character:
- Only the CONTENT_ZONE updates (text changes)
- Only the STATUS_ZONE updates (cursor position/version)
- LINE_NUMBERS_ZONE remains unchanged (no re-render needed)

This is **dramatically more efficient** than traditional TUI frameworks that redraw everything.

### 2. Screen Management (Critical!)

**Problem Discovered:** Without proper screen management, zone initialization is non-deterministic, causing race conditions where different zones appear on different runs.

**Solution:** Every Room application MUST use screen management:

```rust
// Set up screen manager - REQUIRED for proper zone initialization
let mut screen_manager = ScreenManager::new();
screen_manager.register_screen(ScreenDefinition::new(
    "editor",
    "Mini Editor",
    Arc::new(move || Box::new(LegacyScreenStrategy::new(layout.clone()))),
));
runtime.set_screen_manager(screen_manager);
runtime.activate_screen("editor")?;
```

**Why This Matters:**
- Ensures deterministic plugin initialization order
- Provides proper zone lifecycle management
- Eliminates race conditions in zone population
- Matches the pattern used by ALL working Room examples

### 3. Plugin Architecture

The editor uses a **single plugin managing multiple zones** for coordination:

```rust
impl RoomPlugin for EditorCorePlugin {
    fn init(&mut self, ctx: &mut RuntimeContext) -> Result<()> {
        // Populate ALL zones together for consistency
        self.update_all_zones(ctx);
        self.show_cursor(ctx);
        self.update_cursor_position(ctx);
        Ok(())
    }

    fn on_event(&mut self, ctx: &mut RuntimeContext, event: &RuntimeEvent) -> Result<EventFlow> {
        // Handle input and update zones together
        // ...process keyboard input...

        self.update_all_zones(ctx);      // Update content
        self.show_cursor(ctx);           // Restore cursor visibility
        self.update_cursor_position(ctx); // Position cursor
        Ok(EventFlow::Consumed)
    }
}
```

**Design Choice:** One plugin managing related zones vs multiple plugins managing individual zones.

**Pros of Single Plugin:**
- Guaranteed atomic updates across related zones
- No race conditions between zone updates
- Simpler state management
- Easier to reason about

**Pros of Multiple Plugins:**
- Better separation of concerns
- More modular/reusable components
- Easier to add/remove features

**Recommendation:** Use single plugin for tightly coupled zones (like editor components), multiple plugins for independent features (like status bars, help panels, etc.).

### 4. Cursor Management Deep Dive

**Major Discovery:** Room's CLI driver hides the terminal cursor on startup and never shows it again!

**Root Cause (in cli.rs):**
```rust
fn enter(&self, stdout: &mut impl Write) -> DriverResult<()> {
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All))?;
    //                                   ^^^^ Hides cursor forever!
    Ok(())
}
```

**Solution:** Keep the terminal cursor hidden and highlight the active cell with ANSI styling. Wrap the glyph under the caret in reverse video so the rendered width stays the same and the layout never shifts:

```rust
fn render_content_with_highlight(&self) -> String {
    let mut buffer = String::new();
    for (idx, line) in self.lines.iter().enumerate() {
        if idx > 0 {
            buffer.push('\n');
        }

        if idx == self.cursor_row {
            let split_at = self.cursor_col.min(line.len());
            let (left, right) = line.split_at(split_at);
            buffer.push_str(left);
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

fn update_all_zones(&self, ctx: &mut RuntimeContext) {
    if let Ok(state) = self.state.lock() {
        ctx.set_zone(LINE_NUMBERS_ZONE, state.render_line_numbers());
        ctx.set_zone_pre_rendered(CONTENT_ZONE, state.render_content_with_highlight());
        ctx.set_zone(STATUS_ZONE, state.render_status());
        ctx.request_render();
    }
}
```

**Cursor Positioning:** Room's `set_cursor_hint()` expects **absolute screen coordinates**, not zone-relative:

```rust
fn update_cursor_position(&self, ctx: &mut RuntimeContext) {
    if let Ok(state) = self.state.lock() {
        if let Some(content_rect) = ctx.rect(CONTENT_ZONE) {
            let (cursor_row, cursor_col) = state.cursor_position();
            // Convert zone-relative to absolute screen coordinates
            let max_row = content_rect
                .y
                .saturating_add(content_rect.height.saturating_sub(1));
            let max_col = content_rect
                .x
                .saturating_add(content_rect.width.saturating_sub(1));
            let absolute_row = content_rect
                .y
                .saturating_add(cursor_row)
                .min(max_row);
            let absolute_col = content_rect
                .x
                .saturating_add(cursor_col)
                .min(max_col);
            ctx.set_cursor_hint(absolute_row, absolute_col);
        }
    }
}
```

Clamping the hint ensures the (hidden) terminal cursor never escapes the solved rectangle, so the renderer restores it directly over the block caret.

### 5. First-Paint Lifecycle

**Problem:** Traditional approach leads to blank screens while waiting for input.

**Room's Solution:** Plugins populate zones during `init()`, then `CliDriver` renders everything before entering the event loop.

**Correct Sequence:**
1. Register plugins
2. Plugins' `init()` methods populate zones
3. `CliDriver::run()` renders first frame
4. Enter event loop for user interaction

Set `RuntimeConfig::tick_interval` to a predictable value (16 ms in the demo) and stage a couple of bootstrap ticks via `runtime.bootstrap_controls` before handing the runtime to `CliDriver`. That warm-up guarantees the first render commits with all zones populated before raw mode kicks in.

## Exercise Ideas

### Exercise 1: Add Syntax Highlighting
Create a `SyntaxHighlightPlugin` that processes the content zone and adds ANSI color codes for keywords.

**Hint:** Override content in your plugin's `on_event()` after the core plugin updates.

### Exercise 2: Add Line Wrapping
Modify the content rendering to handle lines longer than the zone width.

**Challenge:** Keep cursor positioning accurate when lines wrap.

### Exercise 3: Multi-Buffer Support
Extend the editor to support multiple files with tab switching.

**Architecture:** Consider whether to use multiple screens or multiple content zones.

### Exercise 4: Search Functionality
Add a search mode that highlights matches and provides navigation.

**UI Design:** Add a search zone that appears/disappears, or use the status zone for search input.

## Key Learnings

### Room's Unique Advantages

1. **Zone Independence** - Update only what changed, not everything
2. **Layout Flexibility** - Constraint-based layouts adapt to terminal size
3. **Plugin Architecture** - Modular, testable, reusable components
4. **Screen Management** - Proper application lifecycle and state management
5. **Efficient Rendering** - Minimal diff updates, no full-screen redraws

### Common Pitfalls

1. **Forgetting Screen Management** - Leads to race conditions and non-deterministic rendering
2. **Using Relative Coordinates** - `set_cursor_hint()` needs absolute screen coordinates
3. **Cursor Visibility** - CLI driver hides cursor; must explicitly restore with ANSI sequences
4. **Plugin Coordination** - Multiple plugins updating same zones can conflict

### Architecture Patterns

1. **Single Plugin for Related Zones** - Editor components that need coordination
2. **Multiple Plugins for Independent Features** - Separable functionality
3. **Zone-Relative Rendering** - Let Room handle layout, focus on content
4. **State Sharing** - Use `Arc<Mutex<T>>` for state shared across plugins

## Performance Notes

The pilot mini editor demonstrates Room's performance advantages:

- **Line numbers update:** O(1) - just re-render the fixed-width zone
- **Content update:** O(1) - just the changed content zone
- **Status update:** O(1) - just the status zone
- **Cursor movement:** O(1) - just cursor positioning, no content re-render

Compare this to traditional TUI frameworks that redraw the entire screen on any change.

## Next Steps

1. Try the exercises above
2. Explore other workshop examples for different patterns
3. Build your own Room application using these architectural principles
4. Consider contributing your innovations back to the Room project

The pilot mini editor proves Room's viability for building sophisticated terminal applications with clean architecture and excellent performance. The zone-based approach scales from simple utilities to complex IDEs while maintaining efficient rendering and clean code organization.

## Troubleshooting

**Cursor not visible?** Check that `show_cursor()` is being called after zone updates.

**Zones appearing inconsistently?** Ensure screen manager is set up before registering plugins.

**Cursor in wrong position?** Verify you're using absolute screen coordinates, not zone-relative.

**Race conditions?** Make sure all related zones are updated by the same plugin in the same event handler.

---

*This workshop demonstrates Room's potential for building the next generation of terminal applications with unprecedented efficiency and clean architecture.*
