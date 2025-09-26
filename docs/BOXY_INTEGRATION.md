# Boxy API Integration with Room

## Overview

Room now integrates with Boxy's new API layer (`boxy::api`) which provides powerful layout capabilities and component-level control.

## Migration: Old vs New API

### Old API (Pre-0.21.0)
```rust
use boxy::{BoxyConfig, BoxColors, render_to_string};

let config = BoxyConfig {
    text: "Content".to_string(),
    colors: BoxColors {
        box_color: "blue".to_string(),
        text_color: "white".to_string(),
        ..Default::default()
    },
    ..Default::default()
};

let rendered = render_to_string(&config);
ctx.set_zone_pre_rendered("zone_id", rendered);
```

**Limitations:**
- Opaque string output
- No component-level metadata
- Cannot query positions or dimensions
- Limited to pre-rendering entire box

### New API (0.21.0+)
```rust
use boxy::api::{
    layout::{BoxBuilder, HeaderBuilder, FooterBuilder},
    geometry,
    room_runtime::RoomRuntimeAdapter,
};
use boxy::visual::ROUNDED;

let layout = BoxBuilder::new("Content")
    .with_header(HeaderBuilder::new("Title").align_center())
    .with_footer(FooterBuilder::new("Status"))
    .with_style(ROUNDED)
    .build();

let adapter = RoomRuntimeAdapter::new(layout.clone());

// Access component metadata
for pos in adapter.positions() {
    println!("{:?} at lines {}-{}", pos.component_type, pos.start_line, pos.end_line);
}

// Render for Room
let rendered = layout.render();
ctx.set_zone_pre_rendered("zone_id", rendered);
```

**Advantages:**
- Component-level positioning data
- Query capabilities (which component is at line X?)
- Geometry calculations without rendering
- Builder pattern for composability
- 10 box styles + advanced features

## New Capabilities

### 1. Pure Geometry Calculations
```rust
use boxy::api::geometry;

// Unicode-aware width (handles emoji, CJK)
let width = geometry::get_text_width("Hello 🌟 World 中文");
// Returns: 19 (correctly accounts for wide characters)

// Detailed metrics
let metrics = geometry::get_text_metrics("Hello");
// metrics.display_width, metrics.char_count, metrics.byte_length

// Box dimension calculation
let dims = geometry::calculate_box_dimensions(
    "Content",
    NORMAL,     // style
    2,          // h_padding
    1,          // v_padding
    Some(40),   // fixed_width
);
// dims.total_width, dims.inner_width, dims.total_height, etc.
```

### 2. Component-Based Layout
```rust
use boxy::api::layout::{BoxBuilder, HeaderBuilder, FooterBuilder, StatusBuilder};

let layout = BoxBuilder::new("Main content here")
    .with_header(HeaderBuilder::new("Document Title").align_center())
    .with_status(StatusBuilder::new("Status: Active"))
    .with_footer(FooterBuilder::new("Page 1").align_right())
    .with_wrapping(true)          // NEW: Text wrapping
    .with_fixed_height(20)        // NEW: Height constraints
    .with_barmode()               // NEW: Document-style separators
    .build();
```

### 3. Room Runtime Integration
```rust
use boxy::api::room_runtime::{RoomRuntimeAdapter, ComponentType};

let adapter = RoomRuntimeAdapter::new(layout);

// Get all component positions
let positions = adapter.positions();

// Query component at specific line
if let Some((pos, comp_type)) = adapter.component_at_line(5) {
    println!("Line 5 is in {:?}", comp_type);
}

// Extract lines for specific component
if let Some(lines) = adapter.component_lines(ComponentType::Header) {
    for line in lines {
        println!("{}", line);
    }
}

// Get dimensions
let width = adapter.total_width();
let height = adapter.total_height();
```

### 4. Advanced Features

#### 10 Box Styles
```rust
use boxy::visual::{NORMAL, ROUNDED, DOUBLE, HEAVY, ASCII, THICKSII, COLON, DOT, STAR, DASHED};

// Classic Unicode styles
NORMAL    // ┌─┐ Standard box drawing
ROUNDED   // ╭─╮ Rounded corners
DOUBLE    // ╔═╗ Double-line borders
HEAVY     // ┏━┓ Bold Unicode lines
ASCII     // +-+ Universal compatibility

// Modern character styles (NEW)
THICKSII  // #=# Bold terminal-style
COLON     // ::: Typewriter-style
DOT       // ••• Minimalist bullets
STAR      // *** Attention-grabbing
DASHED    // ┄┆┄ Professional document styling
```

#### Barmode (Document Style)
```rust
// Traditional full box
let box_layout = BoxBuilder::new("Content")
    .with_header(HeaderBuilder::new("Title"))
    .build();
// Renders: ┌─────┐
//          │Title│
//          ├─────┤
//          │Content│
//          └─────┘

// NEW: Barmode (horizontal separators only)
let barmode_layout = BoxBuilder::new("Content")
    .with_header(HeaderBuilder::new("Title"))
    .with_barmode()
    .build();
// Renders: ──────
//          Title
//          ──────
//          Content
//          ──────
```

#### Text Wrapping
```rust
let wrapped = BoxBuilder::new(
    "This is a very long paragraph that will automatically wrap to the next line..."
)
    .with_wrapping(true)
    .with_fixed_width(40)
    .build();
```

#### Height Constraints
```rust
let truncated = BoxBuilder::new("Line 1\nLine 2\n...Line 20")
    .with_fixed_height(10)  // Truncate if > 10 lines
    .build();
// Automatically adds "... (N more lines)" indicator
```

## Example: boxy_api_demo.rs

See `examples/boxy_api_demo.rs` for a comprehensive demonstration showing:
- Pure geometry calculations
- Component tracking with RoomRuntimeAdapter
- All 10 box styles
- Advanced features (wrapping, barmode, height constraints)

Run with:
```bash
cargo run --example boxy_api_demo
```

## Migration Strategy

1. **Keep old API for simple cases** - If you just need a quick box, `BoxyConfig` still works
2. **Use new API for component control** - When you need positioning data or dynamic updates
3. **Use geometry module for measurements** - Pure calculations without rendering
4. **Use RoomRuntimeAdapter for integration** - Full metadata and component queries

## Updated Dependency

Room now uses Boxy v0.21.0:
```toml
[dev-dependencies]
boxy = { git = "https://github.com/oodx/boxy.git" }
```

## See Also

- `docs/BOXY_API_README.md` - Complete Boxy API documentation
- `examples/boxy_api_demo.rs` - Room integration showcase
- `examples/boxy_grid_test.rs` - Old API example (still works)
- `examples/boxy_dashboard_runtime.rs` - Hybrid approach