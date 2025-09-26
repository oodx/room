# Room Grid Layout Implementation Roadmap

## Vision
Replace Room's nested flexbox-style layout with a declarative CSS Grid-inspired system that makes layouts intuitive, predictable, and robust.

## Current State (Problems to Solve)
- ❌ Nested `LayoutNode` trees are hard to visualize
- ❌ Manual Boxy integration in every plugin (rect extraction, type conversion)
- ❌ Rows don't align across columns (each subtree solves independently)
- ❌ Resize issues: wrapping, height padding, no min-size handling
- ❌ Direction (Row/Column) confusion
- ❌ Hard to debug ("why is this zone here?")

## Target State (Goals)
- ✅ Declare grid structure: "3 columns, 3 rows, header spans all columns"
- ✅ Single solver pass calculates ALL positions (rows align correctly)
- ✅ Automatic Boxy integration (minimum sizes, collapse, wrapping)
- ✅ Predictable resize behavior
- ✅ Easy to visualize from code
- ✅ Simple mental model: spreadsheet cells

---

## Phase 1: Core Grid Foundation
**Goal:** Implement the grid solver and test it thoroughly

### Tasks
- [ ] **1.1** Design final `GridLayout` API
  - Review API with examples
  - Document design decisions
  - Get approval on API shape

- [ ] **1.2** Implement `GridSize` enum
  - `Fixed(u16)` - absolute cells
  - `Flex(u16)` - proportional units (fr)
  - `Percent(u8)` - percentage 0-100
  - Consider: `Auto`, `Min`, `Max` for v2?

- [ ] **1.3** Implement `GridArea` placement
  - `cell(row, col)` - single cell
  - `new(rows: Range<usize>, cols: Range<usize>)` - span
  - `span_rows(col, rows)` - helper
  - `span_cols(row, cols)` - helper

- [ ] **1.4** Implement grid solver
  - `solve_axis()` - calculate column offsets
  - `solve_axis()` - calculate row offsets
  - Map GridArea → Rect
  - Handle rounding errors (distribute leftovers)

- [ ] **1.5** Write comprehensive tests
  - Fixed sizes only
  - Flex distribution (1:2:1 ratios)
  - Percent calculations
  - Mixed (fixed + flex + percent)
  - Edge cases (0 size, overflow, single cell)
  - Spanning cells
  - Rounding error distribution

**Acceptance Criteria:**
- Grid solver produces correct Rects for all test cases
- Rows align perfectly across columns
- No visual gaps or overlaps
- Rounding errors distributed correctly

---

## Phase 2: Runtime Integration
**Goal:** Replace LayoutTree with GridLayout in Room runtime

### Tasks
- [ ] **2.1** Create `Layout` trait
  ```rust
  pub trait Layout {
      fn solve(&self, size: Size) -> Result<HashMap<String, Rect>>;
  }
  ```

- [ ] **2.2** Implement `Layout` for `GridLayout`
  - Use solver from Phase 1

- [ ] **2.3** Update `RoomRuntime` to accept `impl Layout`
  - Remove hard dependency on `LayoutTree`
  - Update constructor

- [ ] **2.4** Update `handle_resize()`
  - Ensure it works with trait
  - Test resize recalculation

- [ ] **2.5** Remove old layout code
  - Delete `src/layout/core.rs` (LayoutTree, LayoutNode, Constraint)
  - Clean up imports
  - Update module structure

**Acceptance Criteria:**
- Runtime works with GridLayout
- Resize events properly recalculate grid
- No references to old LayoutTree remain
- All unit tests pass

---

## Phase 3: Boxy Integration Helpers
**Goal:** Make Boxy rendering automatic and robust

### Tasks
- [ ] **3.1** Add `RuntimeContext` helper methods
  ```rust
  impl RuntimeContext {
      // Automatic Boxy rendering with sensible defaults
      pub fn render_zone(
          &self,
          zone_id: &str,
          content: impl AsRef<str>,
          config: ZoneConfig,
      ) -> Option<String>;
  }

  pub struct ZoneConfig {
      header: Option<String>,
      footer: Option<String>,
      style: BoxStyle,
      min_width: u16,
      min_height: u16,
      wrapping: bool,
  }
  ```

- [ ] **3.2** Implement minimum size handling
  - Zones below threshold show "collapsed" state
  - Automatic "..." truncation
  - Option to hide vs collapse

- [ ] **3.3** Implement automatic wrapping control
  - Detect when content exceeds width
  - Enable wrapping automatically OR truncate
  - Configurable per zone

- [ ] **3.4** Add resize-aware rendering
  - Zones re-render when Rect changes
  - Content adapts to new dimensions
  - Scroll position preservation (future)

**Acceptance Criteria:**
- Plugins don't manually call `ctx.rect()` anymore
- Minimum sizes handled gracefully
- No unexpected wrapping
- Content fills available space (padding works)

---

## Phase 4: Examples & Documentation
**Goal:** Update all examples and create migration guide

### Tasks
- [ ] **4.1** Create showcase example
  - `examples/grid_showcase.rs`
  - Demonstrates: fixed, flex, percent, spanning, resize
  - Beautiful multi-panel dashboard layout

- [ ] **4.2** Update existing examples
  - `examples/boxy_api_demo.rs` → use Grid
  - `examples/boxy_dynamic_resize.rs` → use Grid
  - `examples/boxy_grid_dynamic.rs` → simplify with new helpers
  - Any workshop examples

- [ ] **4.3** Write migration guide
  - `docs/GRID_MIGRATION.md`
  - Before/after comparisons
  - Common patterns
  - API reference

- [ ] **4.4** Update architecture docs
  - `GRID_LAYOUT.md` - mark as IMPLEMENTED
  - Update `README.md` with Grid API
  - Update `QUICK_REF.md`

- [ ] **4.5** Add Grid workshop
  - `workshop_grid_basics.md`
  - How to think in Grid
  - Common layouts (sidebar, dashboard, editor)

**Acceptance Criteria:**
- All examples compile and run
- Examples demonstrate Grid features clearly
- Documentation is complete and accurate
- Migration path is clear

---

## Phase 5: Polish & Edge Cases
**Goal:** Handle edge cases and add quality-of-life features

### Tasks
- [ ] **5.1** Error handling
  - What if zone_id doesn't exist in grid?
  - What if GridArea is out of bounds?
  - Helpful error messages

- [ ] **5.2** Debug helpers
  - Visual grid overlay (show rows/cols)
  - Print zone placements
  - Validate GridArea references

- [ ] **5.3** Performance
  - Benchmark grid solver vs old LayoutTree
  - Optimize hot paths
  - Cache calculations where possible

- [ ] **5.4** Advanced features (consider for v2)
  - Gap/gutter between cells
  - Grid padding (margins around entire grid)
  - `Auto` size (fit content)
  - `Min`/`Max` constraints
  - Nested grids

**Acceptance Criteria:**
- Comprehensive error messages
- Debug tools aid development
- Performance is acceptable
- Advanced features documented as "future work"

---

## Success Metrics
1. **Code Reduction**: Plugin code is simpler (less manual Boxy integration)
2. **No Layout Bugs**: Resize works predictably, no clipping/overflow
3. **Easy to Learn**: New users can create layouts in <10 minutes
4. **Correct by Default**: Common layouts "just work" without tweaking

---

## Timeline Estimate
- Phase 1: 2-3 days (core solver + tests)
- Phase 2: 1-2 days (runtime integration)
- Phase 3: 2-3 days (Boxy helpers)
- Phase 4: 1-2 days (examples/docs)
- Phase 5: 1-2 days (polish)

**Total: ~1-2 weeks** for complete, production-ready Grid system

---

## Risk Mitigation
- **Risk**: Grid solver has bugs
  - *Mitigation*: Comprehensive test suite in Phase 1

- **Risk**: Breaking changes frustrate users
  - *Mitigation*: No users yet, clear migration guide

- **Risk**: Performance regression
  - *Mitigation*: Benchmark in Phase 5, Grid should be faster (fewer tree walks)

- **Risk**: Boxy integration still has issues
  - *Mitigation*: Phase 3 focused entirely on this, extensive testing

---

## Decision Log

### Decision 1: Delete LayoutTree or Keep?
**Decision**: DELETE (no backwards compat)
**Rationale**: No users yet, clean slate is better than maintaining two systems

### Decision 2: GridSize enum - include Auto/Min/Max?
**Decision**: START with Fixed/Flex/Percent, defer Auto/Min/Max to Phase 5
**Rationale**: YAGNI, can add later without breaking changes

### Decision 3: Boxy integration - automatic or helpers?
**Decision**: Helper methods in RuntimeContext (Phase 3)
**Rationale**: Balance of convenience and control, plugins can still customize

### Decision 4: Use Range<usize> or separate start/end?
**Decision**: Use `Range<usize>` for GridArea (more Rust-idiomatic)
**Rationale**: Familiar syntax, works with `0..3` notation