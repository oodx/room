# Boxy Library API

## Overview

Boxy is a flexible, modular Rust library for creating Unicode-aware text boxes and layouts. Designed with layout engines like Room Runtime in mind, Boxy provides pure geometry calculations, dynamic component building, and optional theming.

Key Design Goals:
- 📐 Pure geometry calculations with precise Unicode width handling
- 🧩 Modular, decoupled components for flexible integration
- 📊 **Barmode Integration**: Document-style horizontal separator layouts
- 🎨 **Complete Box Style Set**: 10 total styles (5 classic + 5 modern character styles)
- 🌈 **Advanced Color System**: Background colors, text colors, border colors with line-by-line application
- 🔧 **Flexible Color Specification**: 5 methods (None, ANSI, RGB, Named, Hex)
- 🌟 Full Unicode and emoji support with proper width calculations
- 🎯 **ColorScheme Integration**: Complete theming system with header/footer/status colors

## Installation

Add Boxy to your `Cargo.toml`:

```toml
[dependencies]
boxy = { git = "https://github.com/your-repo/boxy" }
```

## Core Modules

### Geometry Module

Provides precise text and box dimension calculations with Unicode awareness.

Key Features:
- Emoji and CJK character width handling
- Flexible box dimension calculations
- Metrics for text display

```rust
use boxy::api::geometry;

let text = "Hello 🌟 World 中文";
let metrics = geometry::get_text_metrics(text);
let dims = geometry::calculate_box_dimensions(text, style, h_padding, v_padding);
```

### Layout Module

Create dynamic, composable box layouts without color coupling.

Components:
- `BoxBuilder`: Main layout constructor with barmode support
- `HeaderBuilder`: Configurable headers
- `FooterBuilder`: Flexible footers
- `StatusBuilder`: Status line components
- `BodyBuilder`: Content rendering with enhanced text wrapping and height constraints
- `LayoutMode`: Box or Bar rendering modes

#### Text Wrapping and Height Constraints

Boxy now provides advanced text rendering capabilities to handle complex layout requirements:

```rust
use boxy::api::layout;

let wrapped_box = layout::BoxBuilder::new(
    "This is a long paragraph that will automatically wrap to the next line when it exceeds the specified width. \
     The text will be neatly broken at word boundaries to maintain readability."
)
    .with_wrapping(true)     // Enable text wrapping
    .with_fixed_height(10)   // Limit box height, truncate if needed
    .with_fixed_width(40)    // Set maximum width for wrapping
    .build();

let ellipsis_box = layout::BoxBuilder::new(
    "Very long content with multiple paragraphs. \
     If the total content exceeds the fixed height, it will be truncated with an '... (N more lines)' indicator."
)
    .with_fixed_height(5)    // Truncate content if taller than 5 lines
    .with_wrapping(true)     // Wrap text within the height constraint
    .build();
```

**Wrapping and Height Constraint Features:**
- Automatic text wrapping at word boundaries
- Precise height limitations with intelligent truncation
- Preserves header, footer, and status components within height constraints
- Works seamlessly with all box styles and layout modes

#### Basic Box Creation
```rust
use boxy::api::layout;

let layout = layout::BoxBuilder::new(content)
    .with_header(layout::HeaderBuilder::new("Title").align_center())
    .with_footer(layout::FooterBuilder::new("Footer"))
    .build();
```

#### Barmode Integration (NEW)

Barmode transforms traditional box layouts into document-style layouts with horizontal separators only, perfect for reports and professional documents.

```rust
use boxy::api::layout::{BoxBuilder, HeaderBuilder, FooterBuilder};
use boxy::visual::DASHED;

// Traditional full box (default behavior)
let traditional_box = BoxBuilder::new("Document content here")
    .with_header(HeaderBuilder::new("Section Title"))
    .with_footer(FooterBuilder::new("Page 1"))
    .build();

// NEW: Barmode creates horizontal-line separators only
let document_style = BoxBuilder::new("Document content here")
    .with_header(HeaderBuilder::new("Section Title"))
    .with_footer(FooterBuilder::new("Page 1"))
    .with_barmode() // Enables bar mode - key difference!
    .build();

// Professional styling with barmode
let professional_doc = BoxBuilder::new("Executive summary content")
    .with_header(HeaderBuilder::new("Executive Summary").align_center())
    .with_footer(HeaderBuilder::new("Confidential - Internal Use Only").align_right())
    .with_style(DASHED)  // Professional dashed lines
    .with_barmode()      // Document-style layout
    .with_fixed_width(70)
    .build();
```

**Barmode Benefits:**
- **Document Integration**: Seamlessly blends into text documents
- **Professional Appearance**: Clean horizontal separators without visual clutter
- **Style Compatibility**: Works with all box styles (NORMAL, DASHED, HEAVY, etc.)
- **Flexible Layout**: Header and footer still fully customizable

### Box Styles Module (Expanded with 10 Total Styles)

Five new box styles for enhanced visual variety and document integration.

#### Available Styles - Complete Set (10 Total)

```rust
use boxy::visual::{NORMAL, ROUNDED, DOUBLE, HEAVY, ASCII, THICKSII, COLON, DOT, STAR, DASHED};
use boxy::api::layout::BoxBuilder;

// All 10 available box styles
let normal = BoxBuilder::new("content").with_style(NORMAL).build();
let rounded = BoxBuilder::new("content").with_style(ROUNDED).build();
let double = BoxBuilder::new("content").with_style(DOUBLE).build();
let heavy = BoxBuilder::new("content").with_style(HEAVY).build();
let ascii = BoxBuilder::new("content").with_style(ASCII).build();
let thicksii = BoxBuilder::new("content").with_style(THICKSII).build();
let colon = BoxBuilder::new("content").with_style(COLON).build();
let dot = BoxBuilder::new("content").with_style(DOT).build();
let star = BoxBuilder::new("content").with_style(STAR).build();
let dashed = BoxBuilder::new("content").with_style(DASHED).build();
```

#### Complete Style Reference & Characteristics

**Classic Unicode Styles (5 styles):**
- **NORMAL**: Standard Unicode box drawing (`┌┐└┘─│`) - Clean, professional appearance for general use
- **ROUNDED**: Rounded corners (`╭╮╰╯─│`) - Soft, friendly appearance for modern UIs
- **DOUBLE**: Double-line borders (`╔╗╚╝═║`) - Formal, structured look for important content
- **HEAVY**: Bold Unicode lines (`┏┓┗┛━┃`) - Strong visual impact for emphasis
- **ASCII**: Basic ASCII characters (`+-|`) - Universal compatibility, works in any terminal

**Modern Character Styles (5 NEW styles):**
- **THICKSII**: Uses `#` for corners/verticals, `=` for horizontals - Bold, terminal-style borders perfect for system output
- **COLON**: Uses `:` for all characters - Subtle, typewriter-style formatting ideal for code blocks and technical content
- **DOT**: Uses `•` (bullet) for all characters - Modern, minimalist appearance with clean geometric feel
- **STAR**: Uses `*` for all characters - Attention-grabbing, emphasis style perfect for alerts and important notices
- **DASHED**: Uses `┄` horizontal, `┆` vertical lines - Professional document styling with subtle separation lines

#### Barmode Compatibility
All new styles work seamlessly with barmode for document-style layouts:

```rust
let dashed_bar = BoxBuilder::new("Document section")
    .with_style(DASHED)
    .with_barmode()
    .build();
```

### Theming Module

Optional color application with flexible rendering strategies and comprehensive background color support.

Features:
- Multiple color application modes
- **Enhanced background color support with 5 specification methods**
- Plain and themed renderers
- Line-by-line color application to prevent bleeding
- ColorScheme integration

#### Background Color Specification (ENHANCED)

Five flexible methods for specifying background colors with comprehensive ANSI terminal support.

```rust
use boxy::api::theming::{BackgroundColor, ColorScheme, apply_background_color};

// Method 1: No background color (transparent)
let bg_none = BackgroundColor::None;
let transparent_text = apply_background_color("Hello World", &bg_none);

// Method 2: ANSI color codes (0-255) - Full 256-color palette support
let bg_ansi_dark = BackgroundColor::Ansi(234);   // Dark gray
let bg_ansi_blue = BackgroundColor::Ansi(21);    // Blue
let bg_ansi_green = BackgroundColor::Ansi(46);   // Green
let ansi_styled = apply_background_color("ANSI Colors", &bg_ansi_dark);

// Method 3: RGB values (0-255 each) - True color support
let bg_rgb_dark = BackgroundColor::Rgb(40, 44, 52);      // Dark blue-gray
let bg_rgb_gold = BackgroundColor::Rgb(255, 215, 0);     // Gold
let bg_rgb_teal = BackgroundColor::Rgb(56, 178, 172);    // Teal
let rgb_styled = apply_background_color("RGB Colors", &bg_rgb_dark);

// Method 4: Named colors (maps to ANSI background codes)
let bg_named_std = BackgroundColor::Named("blue".to_string());
let bg_named_bright = BackgroundColor::Named("bright_cyan".to_string());
// Supported names: black, red, green, yellow, blue, magenta, cyan, white
//                  bright_black, bright_red, bright_green, bright_yellow,
//                  bright_blue, bright_magenta, bright_cyan, bright_white
let named_styled = apply_background_color("Named Colors", &bg_named_std);

// Method 5: Hex color codes (#RRGGBB) - Web-style colors
let bg_hex_slate = BackgroundColor::Hex("#2d3748".to_string());    // Slate gray
let bg_hex_emerald = BackgroundColor::Hex("#48bb78".to_string());  // Emerald green
let bg_hex_violet = BackgroundColor::Hex("#805ad5".to_string());   // Violet
let hex_styled = apply_background_color("Hex Colors", &bg_hex_slate);
```

**Key Features:**
- **Line-by-line application**: Prevents color bleeding across terminal lines
- **Automatic reset handling**: Each line gets proper ANSI reset sequences
- **Terminal compatibility**: Graceful fallback for unsupported colors
- **ColorScheme integration**: Works seamlessly with existing theming system

#### Complete Named Color Reference (112 Total Colors)

Boxy includes an extensive palette of **112 named colors** organized by category. All colors are accessible by name in string format.

**Legacy Colors (v0.5.0 Compatibility):**
`red`, `red2`, `deep`, `deep_green`, `orange`, `yellow`, `green`, `green2`, `blue`, `blue2`, `cyan`, `magenta`, `purple`, `purple2`, `white`, `white2`, `grey`, `grey2`, `grey3`

**Extended Color Spectrum:**
- **Reds**: `crimson`, `ruby`, `coral`, `salmon`, `rose`, `brick`
- **Oranges**: `amber`, `tangerine`, `peach`, `rust`, `bronze`, `gold`
- **Yellows**: `lemon`, `mustard`, `sand`, `cream`, `khaki`
- **Greens**: `lime`, `emerald`, `forest`, `mint`, `sage`, `jade`, `olive`
- **Blues**: `azure`, `navy`, `royal`, `ice`, `steel`, `teal`, `indigo`
- **Purples**: `violet`, `plum`, `lavender`, `orchid`, `mauve`, `amethyst`
- **Cyans**: `aqua`, `turquoise`, `sky`, `ocean`
- **Monochrome**: `black`, `charcoal`, `slate`, `silver`, `pearl`, `snow`

**Semantic Colors (Context-Aware):**
- **Alerts**: `error`, `warning`, `danger`, `alert`
- **Success**: `success`, `complete`, `verified`, `approved`
- **Info**: `info`, `note`, `hint`, `debug`
- **States**: `pending`, `progress`, `blocked`, `queued`, `active`, `inactive`
- **Priority**: `critical`, `high`, `medium`, `low`, `trivial`

**Debug/Status Colors (Jynx Integration):**
- `silly` - Bright magenta for ridiculous debugging/invalid conditions
- `magic` - Lighter purple for "how did this even work?" moments
- `trace` - Medium grey for tracing state progression
- `think` - Bright white for tracing function calls

**Brightness Variants:**
- **Bright**: `bright_red`, `bright_green`, `bright_yellow`, `bright_blue`, `bright_magenta`, `bright_cyan`
- **Dim**: `dim_red`, `dim_green`, `dim_yellow`, `dim_blue`, `dim_magenta`, `dim_cyan`
- **Pastel**: `pastel_red`, `pastel_green`, `pastel_yellow`, `pastel_blue`, `pastel_purple`, `pastel_orange`

**Usage Example:**
```rust
// Use any named color in schemes
scheme.text_color = "emerald".to_string();
scheme.border_color = "silly".to_string();
scheme.header_color = Some("magic".to_string());

// Or in background colors
let bg = BackgroundColor::Named("trace".to_string());
```

**Pro Tip:** Use `boxy --colors` (if available) to see all colors with visual previews in your terminal.

#### Basic Usage

```rust
use boxy::api::theming;

// Create renderers
let plain_renderer = theming::create_plain_renderer();
let themed_renderer = theming::create_themed_renderer();

// Apply background colors to content
let bg_color = theming::BackgroundColor::Rgb(40, 44, 52);
let styled_text = theming::apply_background_color("Hello World", &bg_color);
```

## Usage Examples

### Room Runtime (Pure Geometry)

```rust
use boxy::api::{geometry, layout, room_runtime};

// Calculate dimensions without colors
let dims = geometry::calculate_box_dimensions(content, style);
let layout = layout::BoxBuilder::new(content).build();
let adapter = room_runtime::RoomRuntimeAdapter::new(layout);

// Get line-by-line positioning
let positions = adapter.positions();
let header_component = adapter.component_at_line(0);
```

### Convenience Box Rendering

```rust
use boxy::api::layout;

// Quick box creation with options
let output = layout::render_box("Hello World!",
    layout::BoxOptions {
        header: Some("Welcome".to_string()),
        width: Some(40),
        ..Default::default()
    }
);

// Line-by-line rendering for precise positioning
let lines = layout::render_box_lines("Content",
    layout::BoxOptions {
        footer: Some("v1.0".to_string()),
        ..Default::default()
    }
);
```

### ANSI Size Analysis

```rust
use boxy::api::geometry;

let plain_text = "Hello, World!";
let colored_text = "\x1b[32mHello, World!\x1b[0m";

let size_comparison = geometry::compare_ansi_sizes(plain_text, colored_text);
println!("Color Overhead: {}%", size_comparison.overhead_percentage);
```

### Traditional Usage with Theming

```rust
use boxy::api::{layout, theming};

let layout = layout::BoxBuilder::new(content)
    .with_header(layout::HeaderBuilder::new("Title"))
    .build();

let scheme = theming::ColorScheme::default();
let styled_layout = theming::apply_colors(&layout.render(), &scheme);
```

### Comprehensive Feature Examples

Combining all the new features for powerful layouts.

#### Example 1: Document Layout with Custom Styling

```rust
use boxy::api::{layout, theming};
use boxy::visual::DASHED;

// Create a document-style layout with dashed borders and barmode
let document = layout::BoxBuilder::new(
    "This is a document section with professional styling.\n\n\
     Perfect for reports, documentation, and structured content."
)
    .with_header(layout::HeaderBuilder::new("Executive Summary").align_center())
    .with_footer(layout::FooterBuilder::new("Page 1 of 3").align_right())
    .with_style(DASHED)  // Professional dashed lines
    .with_barmode()      // Document-style horizontal separators
    .with_fixed_width(60)
    .build();

// Apply background color
let bg_color = theming::BackgroundColor::Hex("#f8f9fa".to_string());
let final_output = theming::apply_background_color(&document.render(), &bg_color);
println!("{}", final_output);
```

#### Example 2: Modern UI Components

```rust
use boxy::api::{layout, theming};
use boxy::visual::{DOT, STAR};

// Create a modern notification box
let notification = layout::BoxBuilder::new("✅ Task completed successfully!")
    .with_header(layout::HeaderBuilder::new("🔔 Notification"))
    .with_style(DOT)  // Modern bullet styling
    .build();

// Create an attention-grabbing alert
let alert = layout::BoxBuilder::new("⚠️ System maintenance in 10 minutes")
    .with_header(layout::HeaderBuilder::new("🚨 Alert"))
    .with_style(STAR)  // Eye-catching star styling
    .build();

// Apply different background colors
let green_bg = theming::BackgroundColor::Named("bright_green".to_string());
let yellow_bg = theming::BackgroundColor::Rgb(255, 255, 0);

let styled_notification = theming::apply_background_color(&notification.render(), &green_bg);
let styled_alert = theming::apply_background_color(&alert.render(), &yellow_bg);
```

#### Example 3: All Box Styles Comparison

```rust
use boxy::api::layout::{BoxBuilder, HeaderBuilder};
use boxy::visual::{NORMAL, ROUNDED, DOUBLE, HEAVY, ASCII, THICKSII, COLON, DOT, STAR, DASHED};

let content = "Sample content for style comparison";
let styles = [
    ("Normal", NORMAL),
    ("Rounded", ROUNDED),
    ("Double", DOUBLE),
    ("Heavy", HEAVY),
    ("ASCII", ASCII),
    ("ThickSII", THICKSII),
    ("Colon", COLON),
    ("Dot", DOT),
    ("Star", STAR),
    ("Dashed", DASHED),
];

for (name, style) in styles {
    let box_display = BoxBuilder::new(content)
        .with_header(HeaderBuilder::new(name))
        .with_style(style)
        .build();

    println!("{}", box_display.render());
    println!(); // Add spacing
}
```

#### Example 4: Barmode vs Box Mode Comparison

```rust
use boxy::api::layout::{BoxBuilder, HeaderBuilder, FooterBuilder};
use boxy::visual::DASHED;

let content = "Compare rendering modes";

// Traditional box mode
let box_mode = BoxBuilder::new(content)
    .with_header(HeaderBuilder::new("Box Mode"))
    .with_footer(FooterBuilder::new("Complete borders"))
    .with_style(DASHED)
    .build();

// Document-style barmode
let bar_mode = BoxBuilder::new(content)
    .with_header(HeaderBuilder::new("Bar Mode"))
    .with_footer(FooterBuilder::new("Horizontal separators only"))
    .with_style(DASHED)
    .with_barmode()  // Key difference
    .build();

println!("Box Mode:");
println!("{}", box_mode.render());
println!("\nBar Mode:");
println!("{}", bar_mode.render());
```

#### Example 5: Complete Color Scheme Integration

```rust
use boxy::api::{layout, theming};
use boxy::visual::DOT;

// Create a layout
let layout = layout::BoxBuilder::new("System Status: All services operational")
    .with_header(layout::HeaderBuilder::new("Dashboard"))
    .with_footer(layout::FooterBuilder::new("Live"))
    .with_style(DOT)
    .with_fixed_width(50)
    .build();

// Create a complete color scheme
let mut scheme = theming::ColorScheme::default();
scheme.background_color = theming::BackgroundColor::Rgb(30, 30, 40);  // Dark background
scheme.text_color = "bright_white".to_string();                        // Light text
scheme.border_color = "cyan".to_string();                              // Cyan borders
scheme.header_color = Some("bright_yellow".to_string());               // Yellow header
scheme.footer_color = Some("green".to_string());                       // Green footer

// Apply the complete scheme
let styled_output = theming::apply_colors(&layout.render(), &scheme);
println!("{}", styled_output);
```

#### Example 6: Production Dashboard Layout

```rust
use boxy::api::{layout, theming};
use boxy::visual::{DOT, HEAVY, DASHED};

// System status panel with dot styling and background color
let system_status = layout::BoxBuilder::new(
    "🟢 All Services Operational\n\n\
     CPU: 23% | Memory: 4.2/16GB | Disk: 156GB free\n\
     Network: ↑2.3MB/s ↓847KB/s | Uptime: 5d 14h"
)
    .with_header(layout::HeaderBuilder::new("🖥️  System Status").align_center())
    .with_style(DOT)
    .with_fixed_width(60)
    .build();

// Service list with heavy borders and warning background
let services = layout::BoxBuilder::new(
    "✅ nginx        │ Running  │ 45MB\n\
     ✅ postgresql   │ Running  │ 128MB\n\
     ✅ redis        │ Running  │ 23MB\n\
     ⚠️  app-server   │ High CPU │ 189MB"
)
    .with_header(layout::HeaderBuilder::new("⚙️  Services").align_center())
    .with_style(HEAVY)
    .with_fixed_width(60)
    .build();

// Log panel with barmode and dark background
let recent_logs = layout::BoxBuilder::new(
    "[INFO]  Application started successfully\n\
     [WARN]  High memory usage detected (85%)\n\
     [ERROR] Authentication failed: user@192.168.1.100\n\
     [INFO]  Cache cleared, performance improved"
)
    .with_header(layout::HeaderBuilder::new("📋 Recent Logs").align_center())
    .with_footer(layout::FooterBuilder::new("Press 'L' for detailed logs").align_center())
    .with_style(DASHED)
    .with_barmode()  // Clean document-style presentation
    .with_fixed_width(60)
    .build();

// Apply different background colors for visual distinction
let normal_bg = theming::BackgroundColor::None;
let warning_bg = theming::BackgroundColor::Rgb(60, 40, 20);  // Dark orange
let info_bg = theming::BackgroundColor::Ansi(234);           // Dark gray

let status_display = theming::apply_background_color(&system_status.render(), &normal_bg);
let service_display = theming::apply_background_color(&services.render(), &warning_bg);
let log_display = theming::apply_background_color(&recent_logs.render(), &info_bg);

println!("{}", status_display);
println!("{}", service_display);
println!("{}", log_display);
```\n\n## Key Features

### Core Functionality
- 📏 **Precise Unicode Width Calculations**: Proper handling of emojis, CJK characters, and complex Unicode
- 🔧 **Dynamic Component Updates**: Real-time layout modifications and responsive sizing
- 🔒 **Protected Calculation Macros**: Width calculation macros prevent layout corruption
- 🌐 **Multi-language Support**: Full Unicode support including right-to-left languages

### Visual & Layout System
- 🎨 **Complete Box Style Collection**: 10 total styles (5 classic Unicode + 5 modern character styles)
- 📊 **Barmode Integration**: Document-style horizontal separator layouts for professional documents
- 🎯 **Flexible Alignment**: Left, center, right alignment for headers, footers, and content
- 🔄 **Responsive Layouts**: Automatic width adjustment and content wrapping

### Advanced Color System
- 🌈 **Comprehensive Background Colors**: 5 specification methods (None, ANSI, RGB, Named, Hex)
- 🎨 **Complete Color Schemes**: Background, text, border, header, footer, and status colors
- 🛡️ **Line-by-line Color Application**: Prevents color bleeding across terminal lines
- ⚡ **Performance Optimized**: Efficient ANSI escape sequence generation
- 🔧 **Graceful Degradation**: Automatic fallback for unsupported colors

### Integration & Compatibility
- 🏗️ **Room Runtime Compatible**: Pure geometry calculations for layout engine integration
- 🧩 **Modular Architecture**: Use only the components you need (geometry, layout, theming)
- 📱 **Terminal Agnostic**: Works across different terminal emulators and environments
- 🔌 **Builder Pattern API**: Intuitive, chainable method calls for easy layout construction

## API Reference

### Main Types
- `geometry::BoxDimensions`
- `geometry::TextMetrics`
- `geometry::AnsiSizeComparison`
- `layout::ComponentLayout`
- `layout::BoxOptions`
- `layout::LayoutMode` *(NEW)* - Box or Bar rendering modes
- `layout::HorizontalAlign` - Text alignment options
- `layout::VerticalAlign` - Vertical positioning options
- `layout::BodyBuilder` *(ENHANCED)* - Content rendering with wrapping and height control
- `room_runtime::RoomRuntimeAdapter`
- `room_runtime::ComponentPosition`
- `room_runtime::LayoutMetadata`
- `theming::ColorScheme`
- `theming::BackgroundColor` *(ENHANCED)* - 5 color specification methods
- `visual::BoxStyle` - Box drawing styles including 5 new styles

#### Enhanced BodyBuilder

The `BodyBuilder` now supports advanced text rendering:

```rust
use boxy::api::layout;

let body_builder = layout::BodyBuilder::new()
    .with_content("Dynamic content")
    .enable_wrapping(true)       // Word boundary text wrapping
    .set_max_height(10)           // Truncate if content exceeds 10 lines
    .set_max_width(50);           // Wrap text within 50 characters
```

**Enhanced BodyBuilder Features:**
- `enable_wrapping(bool)`: Toggle text wrapping
- `set_max_height(usize)`: Limit content height
- `set_max_width(usize)`: Control text wrapping width
- Intelligent truncation with ellipsis for overflow
- Maintains component integrity during resizing

### Key Functions
- `geometry::get_text_width()`
- `geometry::calculate_box_dimensions()`
- `geometry::calculate_ansi_overhead()`
- `geometry::compare_ansi_sizes()`
- `layout::BoxBuilder::new()`
- `layout::BoxBuilder::with_barmode()` *(NEW)* - Enable barmode layout
- `layout::BoxBuilder::with_style()` - Apply box styles
- `layout::BoxBuilder::with_wrapping(bool)` *(NEW)* - Enable text wrapping at word boundaries
- `layout::BoxBuilder::with_fixed_height(usize)` *(NEW)* - Set maximum box height, truncate if needed
- `layout::BoxBuilder::with_fixed_width(usize)` - Set maximum box width
- `layout::render_box()`
- `layout::render_box_lines()`
- `room_runtime::RoomRuntimeAdapter::new()`
- `theming::apply_colors()`
- `theming::apply_background_color()` *(ENHANCED)* - Line-by-line color application
- `theming::create_plain_renderer()` - Color-free rendering
- `theming::create_themed_renderer()` - Full theme application

### Box Styles - Complete Set (10 Total)

**Classic Unicode Styles:**
- `visual::NORMAL` - Standard Unicode box drawing (`┌┐└┘─│`)
- `visual::ROUNDED` - Rounded corners (`╭╮╰╯─│`)
- `visual::DOUBLE` - Double-line borders (`╔╗╚╝═║`)
- `visual::HEAVY` - Bold Unicode lines (`┏┓┗┛━┃`)
- `visual::ASCII` - Basic ASCII characters (`+-|`)

**Modern Character Styles:**
- `visual::THICKSII` - Bold `#` and `=` characters
- `visual::COLON` - Uniform `:` characters
- `visual::DOT` - Bullet `•` characters
- `visual::STAR` - Asterisk `*` characters
- `visual::DASHED` - Unicode dashed lines `┄┆`

### Color System Functions

**Background Colors:**
- `theming::apply_background_color()` - Line-by-line background color application (prevents bleeding)
- `theming::BackgroundColor::None` - No background (transparent)
- `theming::BackgroundColor::Ansi(u8)` - ANSI 256-color codes (0-255)
- `theming::BackgroundColor::Rgb(u8, u8, u8)` - True color RGB values
- `theming::BackgroundColor::Named(String)` - Named color mapping
- `theming::BackgroundColor::Hex(String)` - Hex color codes (#RRGGBB)

**Complete Color Schemes:**
- `theming::ColorScheme::default()` - Default color scheme
- `theming::ColorScheme::plain()` - No colors applied
- `theming::ColorScheme::with_background()` - Add background color to scheme
- `theming::apply_colors()` - Apply complete color scheme to content

**Individual Color Components:**
- `scheme.background_color` - Background color specification
- `scheme.text_color` - Main text color (String)
- `scheme.border_color` - Box border color (String)
- `scheme.header_color` - Header text color (Option<String>)
- `scheme.footer_color` - Footer text color (Option<String>)
- `scheme.status_color` - Status line color (Option<String>)

### Barmode Integration

**Layout Modes:**
- `layout::LayoutMode::Box` - Traditional full box rendering (default)
- `layout::LayoutMode::Bar` - Document-style horizontal separators only
- `layout::BoxBuilder::with_barmode()` - Enable barmode for BoxBuilder

## Migration Guide

### From CLI to Library

1. Replace direct terminal printing with `layout` and `geometry` module calls
2. Use `theming` module for optional color application
3. Leverage `BoxBuilder` for dynamic layouts
4. Use `get_text_metrics()` instead of manual width calculations

## Limitations & Considerations

- Requires Rust 1.70+ for full Unicode support
- Performance may vary with complex Unicode strings
- Background color support is optional

## Contributing

Contributions welcome! Please check our GitHub repository for guidelines.

## License

[Insert your project's license here]