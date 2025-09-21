# Terminal Layout Engine Research

## Problem Statement

Terminal applications need robust layout systems for complex UIs, but current solutions are either:
- Too heavy (full CSS engines like Taffy)
- Too simple (basic column splitting)
- Coupled to specific TUI frameworks (Ratatui's layout)

## Current Landscape

### Terminal UI Challenges

1. **Fixed Character Grid**: Terminals work in discrete character cells, not pixels
2. **Sequential Rendering**: Traditional terminals print line-by-line
3. **Dynamic Content**: Need to handle streaming data, resizing, and reflow
4. **Limited Styling**: Only ANSI escape codes for positioning and colors

### Existing Solutions

#### Simple Tools
- **column**: Basic column formatting
- **paste**: Merge files side-by-side
- **pr**: Print in columns
- **whiptail/dialog**: Pre-built UI components

#### TUI Frameworks
- **Ratatui/tui-rs**: Constraint-based layout system
- **Blessed (Node.js)**: CSS-like positioning
- **FTXUI (C++)**: Functional composition
- **Textual (Python)**: Full CSS support

#### Pure Layout Engines
- **Taffy** (formerly Stretch): Rust flexbox/CSS Grid implementation
- **Yoga**: Facebook's C++ flexbox (React Native)

## Architecture Patterns

### Two-Phase Rendering

All robust TUI libraries separate layout from content:

```rust
// Phase 1: Calculate layout (no content)
let rectangles = layout_tree.calculate(terminal_size);

// Phase 2: Render content into calculated spaces
for (widget, rect) in widgets.zip(rectangles) {
    widget.render(rect);
}
```

### Constraint Systems

#### Ratatui Pattern
```rust
Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(30),
        Constraint::Min(20),
        Constraint::Length(15),
    ])
```

#### Taffy/Flexbox Pattern
```rust
Style {
    display: Display::Flex,
    flex_direction: FlexDirection::Row,
    flex_grow: 1.0,
}
```

## Terminal-Specific Requirements

### What Terminals Need
- Integer-only dimensions (character cells)
- Box drawing characters
- Text wrapping at character boundaries
- Overflow handling (scroll/truncate)
- ANSI escape sequence awareness

### What Terminals Don't Need
- Subpixel precision
- Float positioning
- Transforms/rotations
- Z-index layering
- Complex text metrics

## Integration with Token Streams

Our RSB/xstream token format (`k=v;prefix:k=v`) enables clean separation:

```
Token Stream → Parser → Layout Engine → Renderer
     ↓           ↓            ↓            ↓
  (k=v;k=v)  HashMap<>    Rectangles    Boxy calls
```

Benefits:
- Pure data streams remain layout-agnostic
- Multiple layouts from same data
- Testable at each layer

## Proposed Solution: Terminal Layout Engine

### Core Features
```rust
enum Constraint {
    Fixed(u16),      // Exact character count
    Percent(u8),     // Percentage of parent
    Min(u16),        // Minimum characters
    Max(u16),        // Maximum characters
    Flex(f32),       // Flex weight
}

struct Layout {
    direction: Direction,
    constraints: Vec<Constraint>,
    gap: u16,
    padding: u16,
}
```

### Key Algorithms Needed

1. **Constraint Solver**: Distribute space according to constraints
2. **Text Wrapper**: Break text at word boundaries
3. **Box Drawing**: Unicode/ASCII border rendering
4. **Width Calculator**: Handle wide chars, emoji, ANSI codes
5. **Overflow Handler**: Scroll, truncate, or wrap

### Integration Points

- **Input**: Token streams, direct API, or config files
- **Output**: Render instructions for boxy or direct ANSI output
- **Interop**: Can be used standalone or with existing TUI frameworks

## References

### Libraries to Study
- [Taffy](https://github.com/DioxusLabs/taffy) - Pure layout, flexbox/grid
- [Ratatui Layout](https://github.com/ratatui-org/ratatui/blob/main/src/layout.rs) - Terminal-specific constraints
- [Yoga](https://github.com/facebook/yoga) - Battle-tested flexbox

### Existing Terminal Apps with Complex Layouts
- [Zellij](https://github.com/zellij-org/zellij) - Terminal multiplexer
- [Helix](https://github.com/helix-editor/helix) - Modal editor
- [Lazygit](https://github.com/jesseduffield/lazygit) - Git TUI
- [K9s](https://github.com/derailed/k9s) - Kubernetes TUI

## Next Steps

1. Build minimal constraint solver (50-100 lines)
2. Create terminal-specific layout primitives
3. Integrate with boxy for rendering
4. Add token stream routing for dynamic layouts
5. Benchmark against existing solutions

## Open Questions

- Should layouts be declarative (config) or programmatic (API)?
- How to handle overlapping regions (modal dialogs)?
- Should we support nested layouts (layouts within layouts)?
- What's the migration path for existing Ratatui users?