# Grid API Proposal - For Approval

## Core Types

### GridSize - Column/Row Dimensions
```rust
use std::num::NonZeroU16;

/// Defines how a column or row should be sized
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridSize {
    /// Fixed size in terminal cells
    Fixed(u16),

    /// Flexible size (proportional units, like CSS 'fr')
    /// Flex(2) takes 2x the space of Flex(1)
    /// Must be non-zero - zero would create a track that absorbs no space
    Flex(NonZeroU16),

    /// Percentage of available space (1-100)
    /// Note: If total percentages exceed 100%, they will be normalized
    /// If total is less than 100%, remaining space goes to Flex tracks
    Percent(NonZeroU8),
}

use std::num::NonZeroU8;

// Usage examples:
GridSize::Fixed(20)                              // Sidebar: always 20 chars wide
GridSize::Flex(NonZeroU16::new(2).unwrap())      // Main area: 2x flexible space
GridSize::Percent(NonZeroU8::new(30).unwrap())   // Side panel: 30% of width

// Convenience constructors:
impl GridSize {
    /// Create Flex variant, panics if n is zero
    pub fn flex(n: u16) -> Self {
        Self::Flex(NonZeroU16::new(n).expect("Flex size must be non-zero"))
    }

    /// Create Percent variant, panics if n is zero or > 100
    pub fn percent(n: u8) -> Self {
        assert!(n > 0 && n <= 100, "Percent must be 1-100");
        Self::Percent(NonZeroU8::new(n).unwrap())
    }
}
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

    /// Validate area is within grid bounds
    pub(crate) fn validate(self, row_count: usize, col_count: usize) -> Result<Self, GridError> {
        if self.rows.end > row_count || self.cols.end > col_count {
            return Err(GridError::OutOfBounds {
                // ... error details
            });
        }
        Ok(self)
    }

    /// Check if this area overlaps with another
    pub(crate) fn overlaps(&self, other: &GridArea) -> bool {
        let rows_overlap = self.rows.start < other.rows.end && other.rows.start < self.rows.end;
        let cols_overlap = self.cols.start < other.cols.end && other.cols.start < self.cols.end;
        rows_overlap && cols_overlap
    }
}

// Usage examples:
GridArea::cell(0, 0)                     // Top-left cell
GridArea::span_cols(0, 0..3)             // Header spanning all 3 columns
GridArea::new(1..3, 0..2)                // Multi-cell area (rows 1-2, cols 0-1)
```

### GridLayout - Main API
```rust
use std::collections::HashMap;

/// Zone identifier (avoids repeated String allocations)
pub type ZoneId = String;

/// Declarative 2D grid layout system
pub struct GridLayout {
    cols: Vec<GridSize>,
    rows: Vec<GridSize>,
    areas: HashMap<ZoneId, GridArea>,
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
    pub fn add_col(&mut self, size: GridSize) -> &mut Self {
        self.cols.push(size);
        self
    }

    /// Add a row definition
    pub fn add_row(&mut self, size: GridSize) -> &mut Self {
        self.rows.push(size);
        self
    }

    /// Place a zone at a grid area
    /// Returns error if area is out of bounds or overlaps existing zone
    pub fn place(&mut self, zone_id: impl Into<ZoneId>, area: GridArea) -> Result<&mut Self, GridError> {
        let zone_id = zone_id.into();
        let area = area.validate(self.rows.len(), self.cols.len())?;

        // Check for overlaps
        for (existing_id, existing_area) in &self.areas {
            if existing_area.overlaps(&area) {
                return Err(GridError::OverlappingAreas {
                    zone1: existing_id.clone(),
                    zone2: zone_id,
                });
            }
        }

        self.areas.insert(zone_id, area);
        Ok(self)
    }

    /// Set gap between cells (in terminal cells)
    /// Gap applies only between tracks, not at outer edges
    pub fn with_gap(&mut self, gap: u16) -> &mut Self {
        self.gap = gap;
        self
    }

    /// Solve the grid for a given terminal size
    /// Returns a map of zone_id → Rect
    pub fn solve(&self, size: Size) -> Result<HashMap<ZoneId, Rect>, GridError>;
}
```

---

## Usage Examples

### Example 1: Simple 3-Column Dashboard
```rust
use room_mvp::*;
use room_mvp::layout::{GridLayout, GridSize, GridArea};

fn build_dashboard() -> Result<GridLayout, GridError> {
    let mut grid = GridLayout::new();

    // 3 columns: fixed sidebar, flexible main, flexible side
    grid.add_col(GridSize::Fixed(20))
        .add_col(GridSize::flex(2))
        .add_col(GridSize::flex(1));

    // 3 rows: fixed header, flexible body, fixed footer
    grid.add_row(GridSize::Fixed(3))
        .add_row(GridSize::flex(1))
        .add_row(GridSize::Fixed(2));

    // Place zones (returns Result for validation)
    grid.place("header", GridArea::span_cols(0, 0..3))?;  // Row 0, all columns
    grid.place("sidebar", GridArea::cell(1, 0))?;         // Row 1, col 0
    grid.place("main", GridArea::cell(1, 1))?;            // Row 1, col 1
    grid.place("side", GridArea::cell(1, 2))?;            // Row 1, col 2
    grid.place("footer", GridArea::span_cols(2, 0..3))?;  // Row 2, all columns

    grid.with_gap(1);  // 1-cell gap between zones

    Ok(grid)
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
fn build_complex() -> Result<GridLayout, GridError> {
    let mut grid = GridLayout::new();

    grid.add_col(GridSize::flex(1))
        .add_col(GridSize::flex(1))
        .add_col(GridSize::flex(1));

    grid.add_row(GridSize::Fixed(5))
        .add_row(GridSize::flex(1))
        .add_row(GridSize::flex(1));

    // Header spans all columns
    grid.place("header", GridArea::span_cols(0, 0..3))?;

    // Sidebar spans rows 1-2
    grid.place("sidebar", GridArea::span_rows(0, 1..3))?;

    // Main area is large (rows 1-2, cols 1-2)
    grid.place("main", GridArea::new(1..3, 1..3))?;

    Ok(grid)
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
fn build_percentage() -> Result<GridLayout, GridError> {
    let mut grid = GridLayout::new();

    grid.add_col(GridSize::percent(25))   // Left: 25% of width
        .add_col(GridSize::percent(50))   // Center: 50% of width
        .add_col(GridSize::percent(25));  // Right: 25% of width

    grid.add_row(GridSize::flex(1));

    grid.place("left", GridArea::cell(0, 0))?;
    grid.place("center", GridArea::cell(0, 1))?;
    grid.place("right", GridArea::cell(0, 2))?;

    Ok(grid)
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

    /// Hide a zone (won't be rendered, rect still calculated)
    pub fn hide_zone(&mut self, zone_id: &str) {
        self.hidden_zones.insert(zone_id.to_string());
        self.request_render();
    }

    /// Show a previously hidden zone
    pub fn show_zone(&mut self, zone_id: &str) {
        self.hidden_zones.remove(zone_id);
        self.request_render();
    }

    /// Check if zone is hidden
    pub fn is_zone_hidden(&self, zone_id: &str) -> bool {
        self.hidden_zones.contains(zone_id)
    }

    /// Toggle zone visibility
    pub fn toggle_zone(&mut self, zone_id: &str) {
        if self.is_zone_hidden(zone_id) {
            self.show_zone(zone_id);
        } else {
            self.hide_zone(zone_id);
        }
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

// Dynamic visibility control:
ctx.hide_zone("sidebar");           // Hide sidebar
ctx.show_zone("sidebar");           // Show sidebar
ctx.toggle_zone("debug_panel");     // Toggle debug panel

// Check visibility before rendering:
if !ctx.is_zone_hidden("main") {
    ctx.render_zone("main", content, config)?;
}
```

### Visibility Behavior

- **Hidden zones**: Still occupy grid space (Rect calculated), just not rendered
- **No reflow**: Hiding a zone doesn't trigger grid recalculation
- **Use case**: Toggle debug panels, sidebars, status bars without layout shift
- **Alternative**: To actually remove from layout, rebuild grid without that zone

---

## Design Decisions (Resolved)

1. **Gap behavior**: ✅ **RESOLVED**
   - Gap applies **only between tracks**, not at outer edges
   - Outer margins handled by layout padding (separate concern)
   - Example: 3 columns with gap=1 → total gaps = 2 (between cols 0-1 and 1-2)

2. **GridArea validation**: ✅ **RESOLVED**
   - **Fail fast** with `GridError::OutOfBounds` during `place()`
   - Validation happens when area is added, not during solve
   - Error message includes which zone and which dimension is invalid

3. **Empty cells**: ✅ **RESOLVED**
   - Empty grid cells are **perfectly fine** - just empty space
   - No requirement to fill every cell
   - Useful for visual spacing in layouts

4. **Overlapping areas**: ✅ **RESOLVED**
   - **Detect and error** during `place()` with `GridError::OverlappingAreas`
   - Overlap detection prevents hard-to-debug rendering issues
   - Small performance cost at layout construction (one-time, not per-frame)

5. **Percent overflow**: ✅ **RESOLVED**
   - If percentages sum > 100%: **Normalize to 100%** proportionally
   - If percentages sum < 100%: Remaining space goes to Flex tracks
   - Rounding errors: Distribute leftovers to Flex tracks (leftmost first)
   - Example: `[Percent(33), Percent(33), Percent(33)]` with width 100 → [33, 33, 34]

6. **GridSize::Auto**: ✅ **RESOLVED**
   - **Defer to Phase 5** (requires content introspection)
   - MVP includes: Fixed, Flex, Percent (sufficient for most layouts)
   - Auto can be added later without breaking existing code

---

## Error Handling

```rust
/// Errors that can occur during grid layout operations
#[derive(Debug, thiserror::Error)]
pub enum GridError {
    #[error("GridArea for zone '{zone}' is out of bounds: rows {row_start}..{row_end} (grid has {row_count} rows), cols {col_start}..{col_end} (grid has {col_count} cols)")]
    OutOfBounds {
        zone: ZoneId,
        row_start: usize,
        row_end: usize,
        row_count: usize,
        col_start: usize,
        col_end: usize,
        col_count: usize,
    },

    #[error("GridArea for zones '{zone1}' and '{zone2}' overlap")]
    OverlappingAreas {
        zone1: ZoneId,
        zone2: ZoneId,
    },

    #[error("Grid has no columns defined")]
    NoColumns,

    #[error("Grid has no rows defined")]
    NoRows,

    #[error("Zone '{zone}' not found in grid")]
    ZoneNotFound { zone: ZoneId },
}
```

## Breaking Changes

The following will be **removed**:
- `src/layout/core.rs` - entire file
- `pub struct LayoutTree`
- `pub struct LayoutNode`
- `pub enum Constraint`
- `pub enum Direction`

These are replaced by:
- `GridLayout`
- `GridSize` (similar to Constraint but type-safe with NonZeroU16)
- `GridError` (concrete error type with helpful messages)
- No Direction needed (2D grid, not 1D axis)

All examples will be updated in Phase 4.

---

## Approval Checklist

Before implementation, confirm:
- [x] API shape looks correct
- [x] Usage examples are clear and intuitive
- [x] GridSize enum has right variants (Fixed, Flex, Percent) with NonZeroU16
- [x] GridArea helpers cover common cases
- [x] Builder methods use `&mut self` for mutability
- [x] ZoneId type used instead of String for efficiency
- [x] GridError concrete type with helpful messages
- [x] Overlap detection enabled (fail fast)
- [x] Percent normalization defined (proportional when >100%)
- [x] Gap behavior defined (between tracks only)
- [x] Breaking changes are acceptable
- [x] All design decisions resolved
- [x] Ready to proceed with Phase 1 implementation