# Grid API Changes - Codex Review Summary

## Original Feedback Points Addressed

### 1. ✅ GridSize Zero Values
**Feedback:** `Flex(u16)` and `Percent(u8)` should reject zero

**Changes Made:**
```rust
// BEFORE
pub enum GridSize {
    Fixed(u16),
    Flex(u16),        // Could be zero!
    Percent(u8),      // Could be zero!
}

// AFTER
pub enum GridSize {
    Fixed(u16),
    Flex(NonZeroU16),    // Type-safe: cannot be zero
    Percent(NonZeroU8),  // Type-safe: cannot be zero (also 1-100)
}

// Added convenience constructors that panic on invalid input
impl GridSize {
    pub fn flex(n: u16) -> Self {
        Self::Flex(NonZeroU16::new(n).expect("Flex size must be non-zero"))
    }

    pub fn percent(n: u8) -> Self {
        assert!(n > 0 && n <= 100, "Percent must be 1-100");
        Self::Percent(NonZeroU8::new(n).unwrap())
    }
}
```

### 2. ✅ Builder Mutability
**Feedback:** `place()` taking ownership makes incremental edits awkward

**Changes Made:**
```rust
// BEFORE (consuming self)
pub fn add_col(mut self, size: GridSize) -> Self { ... }
pub fn place(mut self, zone_id: impl Into<String>, area: GridArea) -> Self { ... }

// Usage required reassignment:
let grid = GridLayout::new()
    .add_col(Fixed(20))
    .place("zone", area);  // Takes ownership each time

// AFTER (&mut self with &mut Self return)
pub fn add_col(&mut self, size: GridSize) -> &mut Self { ... }
pub fn place(&mut self, zone_id: impl Into<ZoneId>, area: GridArea) -> Result<&mut Self, GridError> { ... }

// Usage allows mutation:
let mut grid = GridLayout::new();
grid.add_col(Fixed(20))
    .add_row(flex(1));
grid.place("zone", area)?;  // Can call separately, modify later
```

### 3. ✅ String Key Type
**Feedback:** Internal map keyed by `String` causes repeated heap allocations

**Changes Made:**
```rust
// BEFORE
pub struct GridLayout {
    areas: HashMap<String, GridArea>,  // String allocations on every solve()
}

// AFTER
pub type ZoneId = String;  // Type alias for clarity

pub struct GridLayout {
    areas: HashMap<ZoneId, GridArea>,  // Same storage, clearer intent
}

// Future optimization path (if needed):
// pub struct ZoneId(Arc<str>);  // or SmallString, etc.
```

**Note:** Currently just a type alias. Real optimization (Arc<str>, interning) can be added later without API changes.

### 4. ✅ Concrete Error Type
**Feedback:** `solve()` needs concrete error type for validation

**Changes Made:**
```rust
// BEFORE
pub fn solve(&self, size: Size) -> Result<HashMap<String, Rect>>;  // Generic Result

// AFTER
pub fn solve(&self, size: Size) -> Result<HashMap<ZoneId, Rect>, GridError>;

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

## Open Questions Resolved

### Gap Behavior ✅
**Decision:** Apply gap **only between tracks**, not at outer edges
- Example: 3 columns with `gap=1` → 2 gaps total (between cols 0-1 and 1-2)
- Outer margins handled separately via layout padding

### Validation ✅
**Decision:** **Fail fast** during `place()`, not during `solve()`
```rust
impl GridLayout {
    pub fn place(&mut self, zone_id: impl Into<ZoneId>, area: GridArea) -> Result<&mut Self, GridError> {
        let area = area.validate(self.rows.len(), self.cols.len())?;  // Immediate validation

        // Check for overlaps with existing zones
        for (existing_id, existing_area) in &self.areas {
            if existing_area.overlaps(&area) {
                return Err(GridError::OverlappingAreas { ... });
            }
        }

        self.areas.insert(zone_id.into(), area);
        Ok(self)
    }
}

impl GridArea {
    pub(crate) fn validate(self, row_count: usize, col_count: usize) -> Result<Self, GridError> {
        if self.rows.end > row_count || self.cols.end > col_count {
            return Err(GridError::OutOfBounds { ... });
        }
        Ok(self)
    }

    pub(crate) fn overlaps(&self, other: &GridArea) -> bool {
        let rows_overlap = self.rows.start < other.rows.end && other.rows.start < self.rows.end;
        let cols_overlap = self.cols.start < other.cols.end && other.cols.start < self.cols.end;
        rows_overlap && cols_overlap
    }
}
```

### Percent Semantics ✅
**Decision:** Normalize percentages, redistribute rounding errors
- If sum > 100%: Normalize proportionally (e.g., [50%, 50%, 50%] → [33%, 33%, 34%])
- If sum < 100%: Remaining space goes to Flex tracks
- Rounding leftovers: Distribute to Flex tracks (leftmost first)
- Example: `[Percent(33), Percent(33), Percent(33)]` with width 100 → [33, 33, 34]

## Additional Improvements

### Zone Visibility Control (User Request)
Added runtime-level visibility helpers (Phase 3):

```rust
impl RuntimeContext {
    /// Hide zone (still occupies grid space, just not rendered)
    pub fn hide_zone(&mut self, zone_id: &str);

    /// Show previously hidden zone
    pub fn show_zone(&mut self, zone_id: &str);

    /// Toggle visibility
    pub fn toggle_zone(&mut self, zone_id: &str);

    /// Check if zone is hidden
    pub fn is_zone_hidden(&self, zone_id: &str) -> bool;
}
```

**Behavior:**
- Hidden zones still occupy grid space (no layout reflow)
- Useful for debug panels, collapsible sidebars
- To actually remove from layout: rebuild grid without that zone

## API Usage Examples

### Before (Consuming Builder)
```rust
// Had to chain everything
let grid = GridLayout::new()
    .add_col(Fixed(20))
    .add_col(Flex(2))
    .place("sidebar", GridArea::cell(0, 0))
    .place("main", GridArea::cell(0, 1));

// Couldn't modify later without rebuilding
```

### After (Mutable Builder)
```rust
// Can build incrementally
let mut grid = GridLayout::new();

grid.add_col(GridSize::Fixed(20))
    .add_col(GridSize::flex(2));

grid.add_row(GridSize::flex(1));

// Add zones with validation
grid.place("sidebar", GridArea::cell(0, 0))?;
grid.place("main", GridArea::cell(0, 1))?;

// Can modify later
grid.place("debug", GridArea::cell(0, 2))?;  // Add new zone

// Solve when ready
let rects = grid.solve(Size::new(80, 24))?;
```

### Error Handling Example
```rust
// Out of bounds
grid.place("invalid", GridArea::cell(10, 5))?;
// Error: GridArea for zone 'invalid' is out of bounds:
//        rows 10..11 (grid has 3 rows), cols 5..6 (grid has 3 cols)

// Overlapping zones
grid.place("zone1", GridArea::new(0..2, 0..2))?;
grid.place("zone2", GridArea::new(1..3, 1..3))?;
// Error: GridArea for zones 'zone1' and 'zone2' overlap
```

## Breaking Changes Summary

**Removed** (no users yet, acceptable):
- `src/layout/core.rs` - entire file
- `LayoutTree`, `LayoutNode`, `Constraint`, `Direction`

**Added**:
- `GridLayout` with `GridSize` (type-safe)
- `GridArea` with validation and helpers
- `GridError` with detailed messages
- Zone visibility control (Phase 3)

## Implementation Status

- ✅ API design complete
- ✅ All feedback addressed
- ✅ Design decisions resolved
- ✅ Error handling defined
- ✅ Committed to `main` branch (commits `ef204ff`, `3dcb96e`)
- ⏳ Phase 1 implementation: Ready to begin

## Codex Feedback & Resolutions

### Round 2 Feedback (All Addressed ✅)

**1. GridArea::validate placeholder** ✅
- **Issue:** Error payload was incomplete
- **Fixed:** Filled in all GridError::OutOfBounds fields with actual values
```rust
pub(crate) fn validate(self, zone: &ZoneId, row_count: usize, col_count: usize) -> Result<Self, GridError> {
    if self.rows.end > row_count || self.cols.end > col_count {
        return Err(GridError::OutOfBounds {
            zone: zone.clone(),
            row_start: self.rows.start,
            row_end: self.rows.end,
            row_count,
            col_start: self.cols.start,
            col_end: self.cols.end,
            col_count,
        });
    }
    Ok(self)
}
```

**2. GridLayout::place move issue** ✅
- **Issue:** zone_id moved into error then used again (compile error)
- **Fixed:** Clone zone_id for error, validate with reference
- **Bonus:** Added duplicate zone detection before overlap check

```rust
pub fn place(&mut self, zone_id: impl Into<ZoneId>, area: GridArea) -> Result<&mut Self, GridError> {
    let zone_id = zone_id.into();

    // Check for duplicate zone first
    if self.areas.contains_key(&zone_id) {
        return Err(GridError::DuplicateZone { zone: zone_id });
    }

    // Validate with reference (no move)
    let area = area.validate(&zone_id, self.rows.len(), self.cols.len())?;

    // Check overlaps (zone_id cloned for error if needed)
    for (existing_id, existing_area) in &self.areas {
        if existing_area.overlaps(&area) {
            return Err(GridError::OverlappingAreas {
                zone1: existing_id.clone(),
                zone2: zone_id,  // Can move here now
            });
        }
    }

    self.areas.insert(zone_id, area);
    Ok(self)
}
```

**3. Add GridError::DuplicateZone** ✅
- **Added:** New error variant distinguishes double-placement from overlap
```rust
#[error("Zone '{zone}' already exists in grid")]
DuplicateZone {
    zone: ZoneId,
},
```

**4. Document rounding algorithm** ✅
- **Added:** Detailed 4-step algorithm with code snippets
  - Step a: Calculate Fixed tracks (reserve space)
  - Step b: Calculate Percent tracks (normalize if > 100%)
  - Step c: Calculate Flex tracks (distribute remaining)
  - Step d: Redistribute leftovers (leftmost Flex tracks first)
- **Added:** 4 worked examples showing different scenarios
- **Added:** u32 conversion note for precise arithmetic

### Original Questions - Codex Answers

**Q1: ZoneId optimization?**
- **A:** Type alias is fine for initial cut. Profile later, optimize if needed.
- **Decision:** Keep `type ZoneId = String` for MVP

**Q2: Overlap detection optional?**
- **A:** Keep on by default. Cheap relative to build, saves debugging.
- **Decision:** Always-on validation, can add `unchecked()` later if needed

**Q3: Percent normalization concerns?**
- **A:** Algorithm looks right, just spell out the conversion/rounding.
- **Decision:** Documented full algorithm with u32 arithmetic

**Q4: More GridError variants?**
- **A:** Besides DuplicateZone, existing set covers major failures.
- **Decision:** Current set sufficient (OutOfBounds, DuplicateZone, OverlappingAreas, NoColumns, NoRows, ZoneNotFound)

## Files Changed

- `GRID_API_PROPOSAL.md` - Complete API specification with all changes
- `GRID_ROADMAP.md` - 5-phase implementation plan (~1-2 weeks)
- No code implementation yet (Phase 1 starts next)