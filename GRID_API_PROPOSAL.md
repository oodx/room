# Grid API Proposal - For Approval

## Core Types

### GridSize - Column/Row Dimensions
```rust
/// Defines how a column or row should be sized
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridSize {
    /// Fixed size in terminal cells
    Fixed(u16),

    /// Flexible size (proportional units, like CSS 'fr')
    /// Flex(2) takes 2x the space of Flex(1)
    Flex(u16),

    /// Percentage of available space (0-100)
    Percent(u8),
}

// Usage examples:
GridSize::Fixed(20)      // Sidebar: always 20 chars wide
GridSize::Flex(2)        // Main area: 2x flexible space
GridSize::Percent(30)    // Side panel: 30% of width
```

### GridArea - Zone Placement
```rust
/// Defines where a zone lives in the grid
#[derive(Debug, Clone, PartialEq)]
pub struct GridArea {
    pub rows: Range<usize>,
    pub cols: Range<usize>,
}

impl GridArea {
    /// Create area spanning specific rows and columns
    pub fn new(rows: Range<usize>, cols: Range<usize>) -> Self {
        Self { rows, cols }
    }

    /// Single cell at (row, col)
    pub fn cell(row: usize, col: usize) -> Self {
        Self {
            rows: row..(row + 1),
            cols: col..(col + 1),
        }
    }

    /// Span multiple rows in a single column
    pub fn span_rows(col: usize, rows: Range<usize>) -> Self {
        Self {
            rows,
            cols: col..(col + 1),
        }
    }

    /// Span multiple columns in a single row
    pub fn span_cols(row: usize, cols: Range<usize>) -> Self {
        Self {
            rows: row..(row + 1),
            cols,
        }
    }
}

// Usage examples:
GridArea::cell(0, 0)                     // Top-left cell
GridArea::span_cols(0, 0..3)             // Header spanning all 3 columns
GridArea::new(1..3, 0..2)                // Multi-cell area (rows 1-2, cols 0-1)
```

### GridLayout - Main API
```rust
/// Declarative 2D grid layout system
pub struct GridLayout {
    cols: Vec<GridSize>,
    rows: Vec<GridSize>,
    areas: HashMap<String, GridArea>,
    gap: u16,  // Space between cells (optional, default 0)
}

impl GridLayout {
    /// Create new empty grid
    pub fn new() -> Self {
        Self {
            cols: Vec::new(),
            rows: Vec::new(),
            areas: HashMap::new(),
            gap: 0,
        }
    }

    /// Add a column definition
    pub fn add_col(mut self, size: GridSize) -> Self {
        self.cols.push(size);
        self
    }

    /// Add a row definition
    pub fn add_row(mut self, size: GridSize) -> Self {
        self.rows.push(size);
        self
    }

    /// Place a zone at a grid area
    pub fn place(mut self, zone_id: impl Into<String>, area: GridArea) -> Self {
        self.areas.insert(zone_id.into(), area);
        self
    }

    /// Set gap between cells (in terminal cells)
    pub fn with_gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Solve the grid for a given terminal size
    /// Returns a map of zone_id → Rect
    pub fn solve(&self, size: Size) -> Result<HashMap<String, Rect>>;
}
```

---

## Usage Examples

### Example 1: Simple 3-Column Dashboard
```rust
use room_mvp::*;
use room_mvp::layout::{GridLayout, GridSize::*, GridArea};

fn build_dashboard() -> GridLayout {
    GridLayout::new()
        // 3 columns: fixed sidebar, flexible main, flexible side
        .add_col(Fixed(20))
        .add_col(Flex(2))
        .add_col(Flex(1))

        // 3 rows: fixed header, flexible body, fixed footer
        .add_row(Fixed(3))
        .add_row(Flex(1))
        .add_row(Fixed(2))

        // Place zones
        .place("header", GridArea::span_cols(0, 0..3))  // Row 0, all columns
        .place("sidebar", GridArea::cell(1, 0))         // Row 1, col 0
        .place("main", GridArea::cell(1, 1))            // Row 1, col 1
        .place("side", GridArea::cell(1, 2))            // Row 1, col 2
        .place("footer", GridArea::span_cols(2, 0..3))  // Row 2, all columns

        .with_gap(1)  // 1-cell gap between zones
}

// Visual result:
// ┌──────────────────────────────────────────────────┐
// │                    header                        │  ← 3 rows
// ├────────┬──────────────────────┬─────────────────┤
// │        │                      │                 │
// │sidebar │       main           │      side       │  ← Flex(1)
// │  20ch  │      Flex(2)         │     Flex(1)     │
// │        │                      │                 │
// ├────────┴──────────────────────┴─────────────────┤
// │                    footer                        │  ← 2 rows
// └──────────────────────────────────────────────────┘
```

### Example 2: Complex Spanning Layout
```rust
fn build_complex() -> GridLayout {
    GridLayout::new()
        .add_col(Flex(1))
        .add_col(Flex(1))
        .add_col(Flex(1))

        .add_row(Fixed(5))
        .add_row(Flex(1))
        .add_row(Flex(1))

        // Header spans all columns
        .place("header", GridArea::span_cols(0, 0..3))

        // Sidebar spans rows 1-2
        .place("sidebar", GridArea::span_rows(0, 1..3))

        // Main area is large (rows 1-2, cols 1-2)
        .place("main", GridArea::new(1..3, 1..3))
}

// Visual result:
// ┌──────────────────────────────────┐
// │           header                 │  ← Row 0
// ├─────────┬────────────────────────┤
// │         │                        │
// │ sidebar │        main            │  ← Row 1
// │         │                        │
// │         ├────────────────────────┤
// │         │                        │  ← Row 2
// └─────────┴────────────────────────┘
```

### Example 3: Percentage-Based Layout
```rust
fn build_percentage() -> GridLayout {
    GridLayout::new()
        .add_col(Percent(25))   // Left: 25% of width
        .add_col(Percent(50))   // Center: 50% of width
        .add_col(Percent(25))   // Right: 25% of width

        .add_row(Flex(1))

        .place("left", GridArea::cell(0, 0))
        .place("center", GridArea::cell(0, 1))
        .place("right", GridArea::cell(0, 2))
}
```

---

## Runtime Integration

### Creating Runtime with Grid
```rust
let grid = GridLayout::new()
    .add_col(Fixed(20))
    .add_col(Flex(1))
    .add_row(Flex(1))
    .place("sidebar", GridArea::cell(0, 0))
    .place("main", GridArea::cell(0, 1));

let mut runtime = RoomRuntime::new(
    grid,
    AnsiRenderer::with_default(),
    Size::new(80, 24),
)?;
```

### Plugin Usage
```rust
impl RoomPlugin for MyPlugin {
    fn init(&mut self, ctx: &mut RuntimeContext) -> Result<()> {
        // Get zone rect (calculated by Grid)
        let rect = ctx.rect("main").unwrap();

        // Render content with Boxy (manually for now, Phase 3 adds helpers)
        let content = BoxBuilder::new("Hello Grid!")
            .with_header(HeaderBuilder::new("Main Panel"))
            .with_fixed_width(rect.width as usize)
            .with_fixed_height(rect.height as usize)
            .build()
            .render();

        ctx.set_zone_pre_rendered("main", content);
        Ok(())
    }

    fn on_event(&mut self, ctx: &mut RuntimeContext, event: &RuntimeEvent) -> Result<EventFlow> {
        match event {
            RuntimeEvent::Resize(_) => {
                // Grid automatically recalculates!
                // Just re-render with new rect
                let rect = ctx.rect("main").unwrap();
                // ... render content ...
                Ok(EventFlow::Continue)
            }
            _ => Ok(EventFlow::Continue)
        }
    }
}
```

---

## Phase 3: Boxy Integration Helpers (Future)

This will be added in Phase 3 to simplify plugin code:

```rust
// Instead of manual rect extraction + Boxy building:
impl RuntimeContext {
    /// Helper: Automatically render zone with Boxy
    pub fn render_zone(
        &mut self,
        zone_id: &str,
        content: impl AsRef<str>,
        config: ZoneConfig,
    ) -> Result<()> {
        let rect = self.rect(zone_id).ok_or(...)?;

        let rendered = BoxBuilder::new(content.as_ref())
            .with_header(config.header.map(HeaderBuilder::new))
            .with_footer(config.footer.map(FooterBuilder::new))
            .with_style(config.style)
            .with_fixed_width(rect.width.max(config.min_width) as usize)
            .with_fixed_height(rect.height.max(config.min_height) as usize)
            .with_wrapping(config.wrapping)
            .build()
            .render();

        self.set_zone_pre_rendered(zone_id, rendered);
        Ok(())
    }
}

pub struct ZoneConfig {
    pub header: Option<String>,
    pub footer: Option<String>,
    pub style: BoxStyle,
    pub min_width: u16,
    pub min_height: u16,
    pub wrapping: bool,
}

// Usage in plugin:
ctx.render_zone("main", "Hello!", ZoneConfig {
    header: Some("Main Panel".into()),
    style: ROUNDED,
    min_width: 20,
    min_height: 5,
    ..Default::default()
})?;
```

---

## Open Questions

1. **Gap behavior**: Should gap apply to outer edges too, or just between cells?
   - Proposal: Only between cells (use padding for outer margins)

2. **GridArea validation**: What if area references out-of-bounds rows/cols?
   - Proposal: Return error from `solve()` with helpful message

3. **Empty cells**: What if no zone is placed at a grid area?
   - Proposal: That's fine, just empty space

4. **Overlapping areas**: What if two zones have overlapping GridAreas?
   - Proposal: Undefined behavior (don't detect for performance), or error?

5. **GridSize::Auto**: Include in MVP or defer?
   - Proposal: Defer to Phase 5 (requires content introspection)

---

## Breaking Changes

The following will be **removed**:
- `src/layout/core.rs` - entire file
- `pub struct LayoutTree`
- `pub struct LayoutNode`
- `pub enum Constraint`
- `pub enum Direction`

These are replaced by:
- `GridLayout`
- `GridSize` (similar to Constraint but clearer)
- No Direction needed (2D grid, not 1D axis)

All examples will be updated in Phase 4.

---

## Approval Checklist

Before implementation, please confirm:
- [ ] API shape looks correct
- [ ] Usage examples are clear and intuitive
- [ ] GridSize enum has right variants (Fixed, Flex, Percent)
- [ ] GridArea helpers cover common cases
- [ ] Breaking changes are acceptable
- [ ] Open questions are resolved
- [ ] Ready to proceed with Phase 1 implementation