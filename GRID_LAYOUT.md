# Room Grid Layout Implementation Guide

This document provides a complete implementation model for integrating a grid layout system with Room's runtime and Boxy rendering engine.

## Core Architecture Overview

```
Terminal Resize Event (SIGWINCH)
    ↓
Runtime::process_resize()
    ↓
Screen::handle_resize() 
    ↓
GridLayout::calculate_bounds()
    ↓
Zone::set_bounds() [marks dirty]
    ↓
Panel::update_bounds()
    ↓
Runtime::render() [only dirty zones]
```

## Boxy API Integration Primer

Room integrates with the `boxy::api` builders introduced in Boxy v0.21.0. Instead of issuing
coordinate drawing commands, you assemble a `BoxLayout` and let Boxy solve the chrome, padding,
and wrapping for you. The primary pieces you will use in the grid implementation are:

- `boxy::api::layout::{BoxBuilder, HeaderBuilder, BodyBuilder, FooterBuilder, StatusBuilder}` for
  composing the panel chrome and content.
- `BoxBuilder::with_fixed_width` / `with_fixed_height` to clamp the total box size to the rect
  supplied by the grid solver.
- `BodyBuilder::with_v_padding`, `with_wrapping`, and `set_max_height` for inner-content shaping.
- `boxy::api::room_runtime::RoomRuntimeAdapter` when you need component metadata (header/body
  spans, total height/width) to coordinate focus or cursor logic.

A minimal render helper wired for Room looks like this:

```rust
use boxy::api::layout::{BoxBuilder, HeaderBuilder, FooterBuilder};
use boxy::api::room_runtime::RoomRuntimeAdapter;

fn render_panel(title: &str, body: &str, width: usize, height: usize) -> (Vec<String>, RoomRuntimeAdapter) {
    let layout = BoxBuilder::new(body)
        .with_header(HeaderBuilder::new(title).align_center())
        .with_footer(FooterBuilder::new("Updated live").align_center())
        .with_fixed_width(width)
        .with_fixed_height(height)
        .build();

    let adapter = RoomRuntimeAdapter::new(layout.clone());
    (layout.render_lines(), adapter)
}
```

`render_lines()` returns the text you stream into the zone buffer, while the adapter exposes the
resolved dimensions and component ranges if the runtime needs them.

## File: `room/src/layout/mod.rs`

```rust
use std::collections::HashMap;
use crate::zone::ZoneId;

/// Represents a rectangular area
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }
    
    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.x + self.width <= other.x || 
          other.x + other.width <= self.x ||
          self.y + self.height <= other.y ||
          other.y + other.height <= self.y)
    }
}

/// Defines how a track (row/column) should be sized
#[derive(Debug, Clone)]
pub enum GridTrack {
    Fixed(u16),        // Fixed size in cells
    Fraction(u16),     // Relative fraction (like CSS fr unit)
    Percent(f32),      // Percentage of available space
    Auto,              // Size to content
}

/// Defines where a zone lives in the grid
#[derive(Debug, Clone)]
pub struct GridArea {
    pub col_start: usize,
    pub row_start: usize,
    pub col_end: usize,
    pub row_end: usize,
}

impl GridArea {
    pub fn new(col_start: usize, row_start: usize, col_end: usize, row_end: usize) -> Self {
        Self { col_start, row_start, col_end, row_end }
    }
    
    /// Single cell
    pub fn cell(col: usize, row: usize) -> Self {
        Self { col_start: col, row_start: row, col_end: col + 1, row_end: row + 1 }
    }
    
    /// Span across columns
    pub fn span_cols(row: usize, col_start: usize, col_end: usize) -> Self {
        Self { col_start, row_start: row, col_end, row_end: row + 1 }
    }
}

/// Update information for a zone after layout calculation
#[derive(Debug)]
pub struct BoundsUpdate {
    pub zone_id: ZoneId,
    pub old_bounds: Rect,
    pub new_bounds: Rect,
    pub should_collapse: bool,
}

/// Main grid layout manager
pub struct GridLayout {
    cols: Vec<GridTrack>,
    rows: Vec<GridTrack>,
    zones: HashMap<ZoneId, GridArea>,
    zone_bounds_cache: HashMap<ZoneId, Rect>,
    last_size: (u16, u16),
    collapse_threshold: (u16, u16), // (min_width, min_height)
}

impl GridLayout {
    pub fn new() -> Self {
        Self {
            cols: Vec::new(),
            rows: Vec::new(),
            zones: HashMap::new(),
            zone_bounds_cache: HashMap::new(),
            last_size: (0, 0),
            collapse_threshold: (10, 3),
        }
    }
    
    pub fn add_column(mut self, track: GridTrack) -> Self {
        self.cols.push(track);
        self
    }
    
    pub fn add_row(mut self, track: GridTrack) -> Self {
        self.rows.push(track);
        self
    }
    
    pub fn place_zone(&mut self, zone_id: ZoneId, area: GridArea) {
        self.zones.insert(zone_id, area);
    }
    
    pub fn set_collapse_threshold(&mut self, width: u16, height: u16) {
        self.collapse_threshold = (width, height);
    }
    
    /// Main layout calculation - returns zones that need updating
    pub fn calculate_bounds(&mut self, term_width: u16, term_height: u16) -> Vec<BoundsUpdate> {
        let mut updates = Vec::new();
        
        // Skip if size hasn't changed
        if (term_width, term_height) == self.last_size {
            return updates;
        }
        
        // Calculate column positions
        let col_positions = self.calculate_track_positions(term_width, &self.cols, true);
        
        // Calculate row positions  
        let row_positions = self.calculate_track_positions(term_height, &self.rows, false);
        
        // Calculate bounds for each zone
        for (zone_id, grid_area) in &self.zones {
            let new_bounds = Rect {
                x: col_positions[grid_area.col_start],
                y: row_positions[grid_area.row_start],
                width: col_positions[grid_area.col_end].saturating_sub(col_positions[grid_area.col_start]),
                height: row_positions[grid_area.row_end].saturating_sub(row_positions[grid_area.row_start]),
            };
            
            // Check if zone should collapse
            let should_collapse = new_bounds.width < self.collapse_threshold.0 
                || new_bounds.height < self.collapse_threshold.1;
            
            // Get old bounds for comparison
            let old_bounds = self.zone_bounds_cache.get(zone_id).copied()
                .unwrap_or(Rect { x: 0, y: 0, width: 0, height: 0 });
            
            // Only create update if bounds changed
            if old_bounds != new_bounds {
                updates.push(BoundsUpdate {
                    zone_id: *zone_id,
                    old_bounds,
                    new_bounds,
                    should_collapse,
                });
                
                // Update cache
                self.zone_bounds_cache.insert(*zone_id, new_bounds);
            }
        }
        
        self.last_size = (term_width, term_height);
        updates
    }
    
    /// Calculate actual positions for tracks (columns or rows)
    fn calculate_track_positions(&self, total_size: u16, tracks: &[GridTrack], is_column: bool) -> Vec<u16> {
        if tracks.is_empty() {
            return vec![0];
        }
        
        let mut positions = Vec::with_capacity(tracks.len() + 1);
        positions.push(0);
        
        // First pass: calculate fixed and percent sizes
        let mut remaining = total_size;
        let mut sizes = Vec::with_capacity(tracks.len());
        let mut total_fractions = 0u16;
        
        for track in tracks {
            match track {
                GridTrack::Fixed(size) => {
                    sizes.push(Some(*size));
                    remaining = remaining.saturating_sub(*size);
                }
                GridTrack::Percent(pct) => {
                    let size = (total_size as f32 * pct).round() as u16;
                    sizes.push(Some(size));
                    remaining = remaining.saturating_sub(size);
                }
                GridTrack::Fraction(fr) => {
                    sizes.push(None);
                    total_fractions += fr;
                }
                GridTrack::Auto => {
                    // For now, treat Auto as Fraction(1)
                    sizes.push(None);
                    total_fractions += 1;
                }
            }
        }
        
        // Second pass: distribute remaining space to fractions/autos
        let mut base_sizes = Vec::with_capacity(tracks.len());
        for (i, track) in tracks.iter().enumerate() {
            let size = if let Some(s) = sizes[i] {
                s
            } else {
                match track {
                    GridTrack::Fraction(fr) if total_fractions > 0 =>
                        ((remaining as u32 * (*fr as u32)) / total_fractions as u32) as u16,
                    GridTrack::Fraction(_) => remaining / tracks.len() as u16,
                    GridTrack::Auto if total_fractions > 0 => remaining / total_fractions,
                    GridTrack::Auto => remaining / tracks.len() as u16,
                    _ => unreachable!(),
                }
            };
            base_sizes.push(size);
        }

        // Deal with rounding leftovers so the grid spans the full axis
        let assigned: u16 = base_sizes.iter().copied().sum();
        let mut leftover = total_size.saturating_sub(assigned);
        while leftover > 0 {
            let mut redistributed = false;
            for (i, track) in tracks.iter().enumerate() {
                if leftover == 0 {
                    break;
                }
                match track {
                    GridTrack::Fraction(_) | GridTrack::Auto => {
                        base_sizes[i] = base_sizes[i].saturating_add(1);
                        leftover = leftover.saturating_sub(1);
                        redistributed = true;
                    }
                    _ => {}
                }
            }
            if !redistributed {
                break; // nothing eligible to receive leftover space
            }
        }

        let mut current_pos = 0u16;
        for size in base_sizes {
            current_pos = current_pos.saturating_add(size);
            positions.push(current_pos);
        }
        
        positions
    }
}
```

## File: `room/src/zone.rs` (modifications)

```rust
use boxy::api::layout::{BoxBuilder, FooterBuilder, HeaderBuilder};
use boxy::visual::BoxStyle;
use crate::layout::Rect;
use crate::buffer::Buffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZoneId(pub usize);

#[derive(Debug, PartialEq)]
pub enum ZoneState {
    Normal,
    Collapsed,
    Hidden,
}

pub struct Zone {
    pub id: ZoneId,
    bounds: Rect,
    panels: Vec<Panel>,
    state: ZoneState,
    collapsed_style: BoxStyle,
    dirty: bool,
    title: String,
}

impl Zone {
    pub fn new(id: ZoneId, title: String) -> Self {
        Self {
            id,
            bounds: Rect { x: 0, y: 0, width: 0, height: 0 },
            panels: Vec::new(),
            state: ZoneState::Normal,
            collapsed_style: BoxStyle::Single,
            dirty: true,
            title,
        }
    }

    pub fn set_bounds(&mut self, new_bounds: Rect) {
        if self.bounds != new_bounds {
            self.bounds = new_bounds;
            self.dirty = true;
            self.update_panel_bounds();
        }
    }

    fn render_collapsed(&self) -> Vec<String> {
        let (width, height) = ensure_min_size(self.bounds.width, self.bounds.height);
        BoxBuilder::new("… collapsed …")
            .with_header(HeaderBuilder::new(&self.title).align_center())
            .with_footer(FooterBuilder::new("Expand to view").align_center())
            .with_style(self.collapsed_style)
            .with_fixed_width(width)
            .with_fixed_height(height)
            .build()
            .render_lines()
    }

    pub fn render(&mut self, buffer: &mut Buffer) {
        if !self.dirty {
            return;
        }

        match self.state {
            ZoneState::Collapsed => {
                for (row, line) in self.render_collapsed().into_iter().enumerate() {
                    if row as u16 >= self.bounds.height {
                        break;
                    }
                    buffer.set_line(self.bounds.x, self.bounds.y + row as u16, &line);
                }
            }
            ZoneState::Normal => {
                for panel in &mut self.panels {
                    panel.render(buffer);
                }
            }
            ZoneState::Hidden => {}
        }

        self.dirty = false;
    }

    fn update_panel_bounds(&mut self) {
        for panel in &mut self.panels {
            panel.set_bounds(self.bounds);
        }
    }
}

fn ensure_min_size(width: u16, height: u16) -> (usize, usize) {
    (
        width.max(3) as usize,
        height.max(3) as usize,
    )
}
```

## File: `room/src/panel.rs` (new/modified)

```rust
use boxy::api::layout::{BoxBuilder, FooterBuilder, HeaderBuilder};
use boxy::visual::BoxStyle;
use crate::layout::Rect;
use crate::buffer::Buffer;

pub struct Panel {
    bounds: Rect,
    content: PanelContent,
    style: BoxStyle,
    dirty: bool,
}

pub enum PanelContent {
    Text {
        lines: Vec<String>,
        header: Option<String>,
        footer: Option<String>,
    },
    Custom {
        render_fn: Box<dyn FnMut(&mut Buffer, Rect)>,
    },
}

impl Panel {
    pub fn new_text(lines: Vec<String>) -> Self {
        Self {
            bounds: Rect::new(0, 0, 0, 0),
            content: PanelContent::Text {
                lines,
                header: None,
                footer: None,
            },
            style: BoxStyle::Single,
            dirty: true,
        }
    }

    pub fn set_bounds(&mut self, new_bounds: Rect) {
        if self.bounds != new_bounds {
            self.bounds = new_bounds;
            self.dirty = true;
        }
    }

    pub fn render(&mut self, buffer: &mut Buffer) {
        if !self.dirty {
            return;
        }

        match &mut self.content {
            PanelContent::Text { lines, header, footer } => {
                let (width, height) = ensure_min_size(self.bounds.width, self.bounds.height);
                let body = lines.join("\n");
                let mut builder = BoxBuilder::new(&body)
                    .with_style(self.style)
                    .with_fixed_width(width)
                    .with_fixed_height(height);

                if let Some(title) = header.as_deref() {
                    builder = builder.with_header(HeaderBuilder::new(title));
                }
                if let Some(status) = footer.as_deref() {
                    builder = builder.with_footer(FooterBuilder::new(status));
                }

                let layout = builder.build();
                for (row, line) in layout.render_lines().into_iter().enumerate() {
                    if row as u16 >= self.bounds.height {
                        break;
                    }
                    buffer.set_line(self.bounds.x, self.bounds.y + row as u16, &line);
                }
            }
            PanelContent::Custom { render_fn } => {
                render_fn(buffer, self.bounds);
            }
        }

        self.dirty = false;
    }

    pub fn set_header(&mut self, text: Option<String>) {
        if let PanelContent::Text { header, .. } = &mut self.content {
            *header = text;
            self.dirty = true;
        }
    }

    pub fn set_footer(&mut self, text: Option<String>) {
        if let PanelContent::Text { footer, .. } = &mut self.content {
            *footer = text;
            self.dirty = true;
        }
    }
}
```

## File: `room/src/screen.rs` (modifications)

```rust
use std::collections::HashSet;
use crate::layout::{GridLayout, BoundsUpdate};
use crate::zone::{Zone, ZoneId, ZoneState};
use crate::runtime::Runtime;

pub struct Screen {
    zones: Vec<Zone>,
    layout: GridLayout,
    dirty_zones: HashSet<ZoneId>,
    size: (u16, u16),
}

impl Screen {
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
            layout: GridLayout::new(),
            dirty_zones: HashSet::new(),
            size: (0, 0),
        }
    }
    
    pub fn set_layout(&mut self, layout: GridLayout) {
        self.layout = layout;
    }
    
    pub fn add_zone(&mut self, zone: Zone) {
        self.dirty_zones.insert(zone.id);
        self.zones.push(zone);
    }
    
    /// Handle terminal resize event
    pub fn handle_resize(&mut self, width: u16, height: u16) {
        // Get layout updates
        let updates = self.layout.calculate_bounds(width, height);
        
        // Apply updates to zones
        for update in updates {
            if let Some(zone) = self.zones.iter_mut().find(|z| z.id == update.zone_id) {
                // Handle collapsing
                if update.should_collapse {
                    zone.collapse();
                } else if zone.state == ZoneState::Collapsed {
                    zone.expand();
                }
                
                // Update bounds
                zone.set_bounds(update.new_bounds);
                
                // Mark as dirty
                self.dirty_zones.insert(zone.id);
            }
        }
        
        self.size = (width, height);
    }
    
    /// Update only dirty zones
    pub fn update(&mut self, runtime: &mut Runtime) {
        // Process each dirty zone
        let dirty_zones: Vec<ZoneId> = self.dirty_zones.drain().collect();
        
        for zone_id in dirty_zones {
            if let Some(zone) = self.zones.iter_mut().find(|z| z.id == zone_id) {
                if zone.is_dirty() {
                    zone.render(runtime.get_buffer());
                    zone.mark_clean();
                }
            }
        }
    }
    
    /// Force all zones to redraw
    pub fn invalidate_all(&mut self) {
        for zone in &self.zones {
            self.dirty_zones.insert(zone.id);
        }
    }
}
```

## File: `room/src/runtime.rs` (modifications)

```rust
use crate::screen::Screen;
use crate::buffer::Buffer;

pub struct Runtime {
    current_screen: Option<Screen>,
    term_width: u16,
    term_height: u16,
    buffer: Buffer,
    needs_render: bool,
}

impl Runtime {
    pub fn new() -> Self {
        let (width, height) = terminal::size().unwrap_or((80, 24));
        Self {
            current_screen: None,
            term_width: width,
            term_height: height,
            buffer: Buffer::new(width, height),
            needs_render: false,
        }
    }
    
    pub fn set_screen(&mut self, screen: Screen) {
        self.current_screen = Some(screen);
        self.needs_render = true;
    }
    
    /// Process terminal resize event
    pub fn process_resize(&mut self, width: u16, height: u16) {
        self.term_width = width;
        self.term_height = height;
        
        // Resize buffer
        self.buffer.resize(width, height);
        
        // Cascade to current screen
        if let Some(screen) = &mut self.current_screen {
            screen.handle_resize(width, height);
        }
        
        self.needs_render = true;
    }
    
    /// Main update loop
    pub fn update(&mut self) {
        if !self.needs_render {
            return;
        }
        
        // Let screen update its dirty zones
        if let Some(screen) = &mut self.current_screen {
            screen.update(self);
        }
        
        // Flush buffer to terminal
        self.buffer.flush();
        self.needs_render = false;
    }
    
    pub fn get_buffer(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
    
    /// Handle SIGWINCH signal
    pub fn on_sigwinch(&mut self) {
        let (width, height) = self.get_terminal_size();
        self.process_resize(width, height);
    }
    
    fn get_terminal_size(&self) -> (u16, u16) {
        // Use termion or crossterm to get actual size
        terminal::size().unwrap_or((80, 24))
    }
}
```

## Usage Example

```rust
use room::{Screen, Runtime, Zone, ZoneId, Panel};
use room::layout::{GridLayout, GridTrack, GridArea};

fn main() {
    let mut runtime = Runtime::new();
    let mut screen = Screen::new();
    
    // Setup grid layout
    let mut layout = GridLayout::new()
        .add_column(GridTrack::Fixed(20))      // Sidebar
        .add_column(GridTrack::Fraction(2))    // Main content
        .add_column(GridTrack::Fraction(1));   // Right panel
    
    let layout = layout
        .add_row(GridTrack::Fixed(3))          // Header
        .add_row(GridTrack::Fraction(1))       // Body
        .add_row(GridTrack::Fixed(1));         // Status
    
    // Create zones with panels
    let mut header_zone = Zone::new(ZoneId(0), "Header".to_string());
    header_zone.add_panel(Panel::new_text(vec!["Application Title".to_string()]));
    
    let mut sidebar_zone = Zone::new(ZoneId(1), "Navigation".to_string());
    sidebar_zone.add_panel(Panel::new_text(vec![
        "File".to_string(),
        "Edit".to_string(),
        "View".to_string(),
        "Help".to_string(),
    ]));
    
    let mut main_zone = Zone::new(ZoneId(2), "Content".to_string());
    main_zone.add_panel(Panel::new_text(vec![
        "Main content goes here...".to_string(),
    ]));
    
    let mut status_zone = Zone::new(ZoneId(3), "Status".to_string());
    status_zone.add_panel(Panel::new_text(vec!["Ready".to_string()]));
    
    // Place zones in grid
    layout.place_zone(ZoneId(0), GridArea::span_cols(0, 0, 3)); // Header spans all columns
    layout.place_zone(ZoneId(1), GridArea::cell(0, 1));          // Sidebar
    layout.place_zone(ZoneId(2), GridArea::new(1, 1, 3, 2));     // Main spans 2 cols
    layout.place_zone(ZoneId(3), GridArea::span_cols(2, 0, 3));  // Status spans all columns
    
    // Configure collapse thresholds
    layout.set_collapse_threshold(15, 5);
    
    // Add zones to screen
    screen.add_zone(header_zone);
    screen.add_zone(sidebar_zone);
    screen.add_zone(main_zone);
    screen.add_zone(status_zone);
    screen.set_layout(layout);
    
    // Set screen in runtime
    runtime.set_screen(screen);
    
    // Initial render
    runtime.process_resize(runtime.term_width, runtime.term_height);
    
    // Main loop
    loop {
        runtime.update();
        
        // Handle events...
        // if terminal_resized() {
        //     runtime.on_sigwinch();
        // }
    }
}
```

## Key Design Decisions

1. **Boxy Builder Pipeline**
   - Every zone/panel builds a `BoxLayout` via `BoxBuilder` + component builders.
   - Rendering uses `layout.render_lines()`; no direct buffer poking or deprecated helpers.
   - `RoomRuntimeAdapter` is available when callers need per-component spans.

2. **Separation of Concerns**
   - `GridLayout` owns track definitions, reflow, and dirty-zone detection.
   - `Zone` stores grid placement, collapse rules, and delegates rendering to panels.
   - `Panel` encapsulates Boxy construction per cell and can expose knobs (header/footer).
   - `Screen` orchestrates zones and dirty lists; `Runtime` handles resize + selective redraws.

3. **Efficient Updates**
   - Recompute track offsets only when the terminal size changes.
   - Compare previous vs. new bounds to mark just the affected zones dirty.
   - Allow the runtime to continue doing selective zone redraws rather than a full-screen repaint.

4. **Collapse + Minimum Size**
   - Zones collapse below configurable width/height thresholds and render a small Boxy stub.
   - `ensure_min_size` guards Boxy against zero/negative dims while preserving chrome.
   - Zones re-expand automatically when the solver grants them enough area again.

5. **Panel System**
   - Text panels feed `lines.join("\n")` into `BoxBuilder`, applying headers/footers as needed.
   - Custom panels receive the solved rect if they need bespoke rendering.
   - Future extensions (wrapping, scrolling) can be layered on top of the Boxy body builders.

## Additional Grid Considerations (from `GRID_NOTES.md`)

- Track sizing should follow a two-pass strategy: allocate fixed/percent tracks first, then
  distribute the remainder across `Fraction` / `Auto` tracks and spread rounding leftovers so the
  grid spans the full axis.
- Integrate resize handling with the runtime’s existing SIGWINCH path: capture the new terminal
  size, call `GridLayout::calculate_bounds`, and let the zone dirty flags drive selective redraws.
- Maintain per-zone state (scroll offsets, focus, etc.) when bounds shrink; clamp positions based
  on the new rect so overlays do not jump unexpectedly.
- Debounce rapid resize events if needed—Room’s runtime loop can coalesce multiple SIGWINCH signals
  before triggering a single grid reflow.

These notes keep the grid system aligned with the runtime architecture the notes file outlines.
