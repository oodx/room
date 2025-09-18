# Session Continuation - Terminal Layout Engine Research

## Latest Session (2025-09-18)
- Scaffolded `pilot/room_mvp` with solver, registry, renderer, token router, and a chat demo example showcasing selective redraw and fixed footer input.
- Constraint solver now honors fixed/percent/min/max/flex constraints with padding/gap; registry hashing prevents redundant renders; renderer leverages Boxy width helpers for Unicode safety.
- Verification: `cargo fmt`, `cargo test`, and `cargo run --example chat_demo`.
- Remaining follow-ups: ROOM-501 (wire pilot into RSB `bin/test.sh`) and ROOM-502 (write developer guide + troubleshooting notes).

## Session Context
**Date**: 2025-09-18
**Primary Focus**: Terminal layout engine design and token stream integration
**Repositories**:
- RSB (current location)
- Boxy (rendering library)
- XStream (token stream library)
- Concepts/Room (new layout engine location)

## Completed Tasks

### 1. Fixed test.sh Newline Rendering Bug
**Problem**: The boxy orchestrator in `bin/test.sh` was displaying literal `\n` characters instead of actual newlines.

**Solution**: Changed `printf '%s\n'` to `printf '%b\n'` in two locations:
- `_emit_boxy()` function at line 155
- `_boxy_fallback()` function at line 173

**Result**: Proper multi-line rendering in boxy output boxes.

## Conversation Progression

### Phase 1: Terminal Layout Fundamentals
Started with exploring how to create column-based layouts in terminals. Key findings:
- Terminals are fundamentally line-based, sequential output devices
- Complex layouts require "fancy algorithms" and position tracking
- Existing tools (paste, column, pr) provide basic functionality
- Full solutions need libraries like ncurses or TUI frameworks

### Phase 2: TUI Component Research
Discovered `whiptail`/`dialog` as "Bootstrap for terminals":
- Pre-built UI component libraries for shell scripts
- Handle complex drawing, input, focus management
- Used by system tools (dpkg-reconfigure, raspi-config)
- Limited to modal dialogs, no custom layouts

### Phase 3: TUI Framework Architecture
Explored why TUI frameworks need event loops despite being single applications:
- Terminal input is blocking by default
- Need to handle multiple simultaneous events (keyboard, mouse, timers, data updates)
- Enables real-time updates while waiting for input
- Required for any interactive dashboard or editor

### Phase 4: Layout Engine Deep Dive
Analyzed how modern TUI libraries handle layout:

**Ratatui Pattern** (Terminal-optimized):
```rust
Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(30),
        Constraint::Min(20),
        Constraint::Length(15),
    ])
```

**Taffy/Yoga Pattern** (CSS Flexbox):
- Full flexbox/grid implementation
- Overkill for terminals (subpixel precision, transforms, etc.)
- Used by Bevy, Zed for real GUI applications

### Phase 5: Terminal-Specific Layout Requirements
Identified what makes terminal layout unique:
- Integer-only dimensions (character cells, not pixels)
- ANSI escape sequences for positioning
- Box drawing characters
- No overlapping regions (except modals)
- Text wrapping at character boundaries

### Phase 6: Token Stream Integration
Explored how RSB's token stream format (`k=v;prefix:k=v`) could power layouts:

**Current Design**: Pure data streams
```
"status=OK;error_count=5;user=alice"
```

**Potential Extensions**:
1. **Bracket notation for indexing**: `users[0]=alice`, `grid[2,3]=red`
2. **Multi-prefix for hierarchy**: `form:login:field:username=alice`
3. **Event routing**: Tokens as dirty region signals

**Key Decision**: Keep token streams as pure data, let consumers handle layout interpretation.

## Key Insights

### 1. Two-Phase Rendering Pattern
All robust TUI libraries separate:
- **Phase 1**: Layout calculation (positions/sizes)
- **Phase 2**: Content rendering (into calculated areas)

This separation enables resize handling, theme switching, and testing.

### 2. Position Tracking is Mandatory
For interactive terminals with selective updates:
- Must track absolute positions of all widgets
- Required for focus management and partial redraws
- No way around it for efficient updates

### 3. Token Streams as Event Bus
Token streams can signal update events:
```
"field:username:dirty=true"
"table:row[3]:updated=true"
```
Layout engine subscribes and performs minimal repaints.

### 4. Editor Optimization Strategies
Vim/nano don't repaint on every keystroke:
- Use differential updates (only changed regions)
- Leverage terminal capabilities (insert/delete line, scroll regions)
- Maintain dirty region tracking
- Only full repaint on resize

## Technical Decisions

### 1. Ratatui > Taffy for Terminals
- Ratatui's constraint system is terminal-native
- Taffy is overkill (CSS compliance unnecessary)
- Integer-based math sufficient for character grids

### 2. Token Stream API Philosophy
**Keep it tight** - Don't allow every variation:
- Current: `key__ext` for extensions
- Proposed: `key[ext]` for clarity
- Decision: Need to review TokenBucket prefix folding behavior before adding features

### 3. Layout Engine Architecture
```
Token Stream → Parser → Layout Engine → Renderer (Boxy)
     ↓           ↓            ↓            ↓
  (k=v;k=v)  HashMap     Rectangles    ANSI output
```

## Current State

### Repository Structure
- **RSB**: Main project with test.sh (fixed)
- **Boxy**: Rendering library with width calculation solved
- **XStream**: Token stream implementation
- **Concepts/Room**: New location for layout engine research

### Open Questions
1. Should bracket notation `key[index]` replace or complement `key__ext`?
2. How does TokenBucket's automatic prefix folding work with nested prefixes?
3. Should layout definitions be declarative (config) or programmatic (API)?
4. How to handle terminal resize events efficiently?

## Next Steps

### Immediate
1. Review TokenBucket implementation in xstream/rsb
2. Decide on token stream extension syntax
3. Create minimal proof-of-concept layout engine

### Future Work
1. Build terminal-optimized layout library ("termflex"/"boxflow")
2. Integrate with boxy for rendering
3. Add interactive field support with focus management
4. Implement efficient dirty region tracking

## Technical Context for Next Session

### Key Files Modified
- `/bin/test.sh` - Fixed newline rendering in `_emit_boxy()` and `_boxy_fallback()`

### Created Documentation
- `/concepts/room/RESEARCH.md` - Comprehensive layout engine research (moved to new repo)

### Important Code Patterns

**Token Stream Routing** (proposed):
```rust
// Stream describes widget updates
"box1:content=Hello;box2:title=Settings"

// Layout engine routes to correct positions
fn process_token(&mut self, token: &str) {
    let (widget_id, property, value) = parse_token(token);
    let pos = self.positions[&widget_id];
    // Update only affected region
}
```

**Interactive Fields** (concept):
```rust
struct Form {
    fields: Vec<Field>,
    active: usize,
    positions: HashMap<String, Rect>,
}

// Tab navigation with position tracking
fn handle_tab(&mut self) {
    self.fields[self.active].unfocus();  // Clear at known position
    self.active = (self.active + 1) % self.fields.len();
    self.fields[self.active].focus();    // Highlight at new position
}
```

## Session Handoff Notes

The conversation evolved from a simple test.sh fix to deep exploration of terminal layout engines. We identified that a lightweight, terminal-specific layout library would fill a gap between simple tools (whiptail) and heavy frameworks (full TUI libraries). The key insight is using token streams as an event bus for efficient partial updates, combined with Ratatui-style constraints for layout calculation. The next session should focus on reviewing the actual TokenBucket implementation to make informed decisions about API extensions.
