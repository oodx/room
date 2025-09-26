//! Boxy API Demo - Showcasing the new Boxy library API integration with Room
//!
//! This example demonstrates the new `boxy::api` module which provides:
//! - Pure geometry calculations via `boxy::api::geometry`
//! - Component-based layout building via `boxy::api::layout`
//! - Room Runtime integration via `boxy::api::room_runtime::RoomRuntimeAdapter`
//! - Advanced features: 10 box styles, barmode, text wrapping, height constraints
//!
//! Compare this to `boxy_grid_test.rs` which uses the old `BoxyConfig` API.
//!
//! ```bash
//! cargo run --example boxy_api_demo
//! ```

use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, RoomPlugin,
    RoomRuntime, RuntimeConfig, RuntimeContext, Size, SimulatedLoop, EventFlow, RuntimeEvent,
};
use room_mvp::runtime::audit::{BootstrapAudit, NullRuntimeAudit};
use std::sync::Arc;
use boxy::api::{
    layout::{BoxBuilder, HeaderBuilder, FooterBuilder, StatusBuilder},
    geometry,
    room_runtime::{RoomRuntimeAdapter, ComponentType},
};
use boxy::visual::{NORMAL, ROUNDED, DOUBLE, HEAVY, DOT, STAR, DASHED};
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal;

const PANEL_INFO: &str = "app:demo.info";
const PANEL_GEOMETRY: &str = "app:demo.geometry";
const PANEL_COMPONENTS: &str = "app:demo.components";
const PANEL_STYLES: &str = "app:demo.styles";
const PANEL_FEATURES: &str = "app:demo.features";
const STATUS_ZONE: &str = "app:demo.status";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();

    // Bootstrap configuration for clean first paint
    config.audit = Some(BootstrapAudit::new(Arc::new(NullRuntimeAudit)));

    let is_headless = std::env::var("HEADLESS").is_ok();
    if is_headless {
        config.simulated_loop = Some(SimulatedLoop::ticks(3));
    }

    // Get actual terminal size or use reasonable defaults
    let (term_width, term_height) = if is_headless {
        (120, 35)  // Reasonable default for headless
    } else {
        terminal::size().unwrap_or((100, 30))
    };

    let mut runtime = RoomRuntime::with_config(
        layout,
        renderer,
        Size::new(term_width, term_height),
        config,
    )?;

    runtime.register_plugin(BoxyApiDemoPlugin::new());

    if is_headless {
        let mut buffer = Vec::new();
        runtime.run(&mut buffer)?;
        println!("{}", String::from_utf8_lossy(&buffer));
        Ok(())
    } else {
        CliDriver::new(runtime).run()?;
        Ok(())
    }
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "demo:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Flex(2),     // Top row (info + geometry)
            Constraint::Flex(2),     // Middle row (components)
            Constraint::Flex(2),     // Styles row
            Constraint::Flex(2),     // Features row
            Constraint::Fixed(3),    // Status bar
        ],
        children: vec![
            LayoutNode {
                id: "demo:top_row".into(),
                direction: Direction::Row,
                constraints: vec![Constraint::Flex(1), Constraint::Flex(1)],
                children: vec![
                    LayoutNode::leaf(PANEL_INFO),
                    LayoutNode::leaf(PANEL_GEOMETRY),
                ],
                gap: 1,
                padding: 0,
            },
            LayoutNode::leaf(PANEL_COMPONENTS),
            LayoutNode::leaf(PANEL_STYLES),
            LayoutNode::leaf(PANEL_FEATURES),
            LayoutNode::leaf(STATUS_ZONE),
        ],
        gap: 1,
        padding: 1,
    })
}

struct ColorTheme {
    name: &'static str,
    border_code: &'static str,
    text_code: &'static str,
}

const THEMES: &[ColorTheme] = &[
    ColorTheme {
        name: "Default",
        border_code: "\x1b[0m",
        text_code: "\x1b[0m",
    },
    ColorTheme {
        name: "Cyan",
        border_code: "\x1b[36m",
        text_code: "\x1b[96m",
    },
    ColorTheme {
        name: "Green",
        border_code: "\x1b[32m",
        text_code: "\x1b[92m",
    },
    ColorTheme {
        name: "Magenta",
        border_code: "\x1b[35m",
        text_code: "\x1b[95m",
    },
    ColorTheme {
        name: "Yellow",
        border_code: "\x1b[33m",
        text_code: "\x1b[93m",
    },
];

struct BoxyApiDemoPlugin {
    theme_index: usize,
}

impl BoxyApiDemoPlugin {
    fn new() -> Self {
        Self { theme_index: 0 }
    }

    fn current_theme(&self) -> &'static ColorTheme {
        &THEMES[self.theme_index]
    }

    fn cycle_theme(&mut self) {
        self.theme_index = (self.theme_index + 1) % THEMES.len();
    }

    fn apply_color(&self, text: &str) -> String {
        let theme = self.current_theme();
        let mut result = String::new();

        for line in text.lines() {
            if line.is_empty() {
                result.push('\n');
                continue;
            }

            let first_char = line.chars().next().unwrap_or(' ');
            let is_border_line = matches!(first_char, '┌' | '┐' | '└' | '┘' | '│' | '─' | '├' | '┤' | '╭' | '╮' | '╰' | '╯' | '╔' | '╗' | '╚' | '╝' | '║' | '═' | '┏' | '┓' | '┗' | '┛' | '┃' | '━' | '•' | '*' | '┄' | '┆');

            if is_border_line {
                result.push_str(theme.border_code);
                result.push_str(line);
                result.push_str("\x1b[0m");
            } else {
                result.push_str(theme.text_code);
                result.push_str(line);
                result.push_str("\x1b[0m");
            }
            result.push('\n');
        }

        result
    }

    fn render_info_panel(&self) -> String {
        let layout = BoxBuilder::new(
            "The new Boxy API provides:\n\n\
             • Pure geometry calculations\n\
             • Component-based layouts\n\
             • RoomRuntimeAdapter for positioning\n\
             • 10 box styles (5 classic + 5 modern)\n\
             • Barmode for document layouts\n\
             • Text wrapping & height constraints\n\
             • 112 named colors + RGB/Hex/ANSI"
        )
        .with_header(HeaderBuilder::new("🎨 New Boxy API").align_center())
        .with_style(ROUNDED)
        .build();

        self.apply_color(&layout.render())
    }

    fn render_geometry_panel(&self) -> String {
        let content = "Hello 🌟 World 中文";

        let width = geometry::get_text_width(content);
        let metrics = geometry::get_text_metrics(content);

        let dims = geometry::calculate_box_dimensions(
            content,
            NORMAL,
            2,  // h_padding
            1,  // v_padding
            None,
        );

        let info = format!(
            "Text: \"{}\"\n\n\
             Unicode Width: {} cols\n\
             Char Count: {}\n\
             Byte Size: {} bytes\n\n\
             Box Dimensions:\n\
             • Total: {}x{}\n\
             • Inner: {}x{}\n\
             • Padding: H={}, V={}",
            content,
            width,
            metrics.char_count,
            metrics.byte_length,
            dims.total_width, dims.total_height,
            dims.inner_width, dims.inner_height,
            dims.h_padding, dims.v_padding,
        );

        let layout = BoxBuilder::new(&info)
            .with_header(HeaderBuilder::new("📐 Geometry API").align_center())
            .with_style(DOUBLE)
            .build();

        self.apply_color(&layout.render())
    }

    fn render_components_panel(&self) -> String {
        let layout = BoxBuilder::new(
            "This box has multiple components:\n\
             • Header (you see above)\n\
             • Body (this content)\n\
             • Status (below)\n\
             • Footer (bottom line)"
        )
        .with_header(HeaderBuilder::new("Component-Based Layout"))
        .with_status(StatusBuilder::new("Status: Active"))
        .with_footer(FooterBuilder::new("RoomRuntimeAdapter tracks each component").align_center())
        .with_style(HEAVY)
        .build();

        let adapter = RoomRuntimeAdapter::new(layout.clone());

        let mut analysis = format!(
            "RoomRuntimeAdapter Analysis:\n\n\
             Total Size: {}x{}\n\
             Components: {}\n\n",
            adapter.total_width(),
            adapter.total_height(),
            adapter.positions().len(),
        );

        for pos in adapter.positions() {
            let comp_name = match pos.component_type {
                ComponentType::Header => "Header",
                ComponentType::Body => "Body",
                ComponentType::Status => "Status",
                ComponentType::Footer => "Footer",
            };
            analysis.push_str(&format!(
                "• {}: lines {}-{} ({}x{})\n",
                comp_name,
                pos.start_line,
                pos.end_line,
                pos.width,
                pos.height,
            ));
        }

        let result = BoxBuilder::new(&analysis)
            .with_header(HeaderBuilder::new("🔍 Component Tracking").align_center())
            .with_style(NORMAL)
            .build();

        self.apply_color(&result.render())
    }

    fn render_styles_panel(&self) -> String {
        let styles = [
            ("NORMAL", NORMAL),
            ("ROUNDED", ROUNDED),
            ("DOUBLE", DOUBLE),
            ("HEAVY", HEAVY),
            ("DOT", DOT),
            ("STAR", STAR),
            ("DASHED", DASHED),
        ];

        let mut output = String::new();

        for (name, style) in styles.iter() {
            let box_content = BoxBuilder::new(name)
                .with_style(*style)
                .build();

            for line in box_content.render().lines() {
                output.push_str(line);
                output.push_str("  ");
            }
            output.push('\n');
        }

        let layout = BoxBuilder::new(&output)
            .with_header(HeaderBuilder::new("🎭 Box Styles (7 of 10 shown)").align_center())
            .with_footer(FooterBuilder::new("Also available: ASCII, THICKSII, COLON"))
            .with_style(ROUNDED)
            .build();

        self.apply_color(&layout.render())
    }

    fn render_features_panel(&self) -> String {
        let wrapped_text = BoxBuilder::new(
            "Text wrapping breaks long paragraphs at word boundaries to fit the specified width."
        )
        .with_header(HeaderBuilder::new("Text Wrapping"))
        .with_wrapping(true)
        .with_fixed_width(30)
        .with_fixed_height(8)
        .with_style(DOT)
        .build();

        let barmode = BoxBuilder::new(
            "Horizontal\nseparators\nonly.\n\nNo vertical\nborders!"
        )
        .with_header(HeaderBuilder::new("Barmode"))
        .with_barmode()
        .with_fixed_width(26)
        .with_fixed_height(8)
        .with_style(DASHED)
        .build();

        let height_constrained = BoxBuilder::new(
            "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10"
        )
        .with_header(HeaderBuilder::new("Height Limit"))
        .with_fixed_height(8)
        .with_fixed_width(18)
        .with_style(HEAVY)
        .build();

        let mut combined = String::new();

        let wrapped_render = wrapped_text.render();
        let wrapped_lines: Vec<_> = wrapped_render.lines().collect();
        let barmode_render = barmode.render();
        let barmode_lines: Vec<_> = barmode_render.lines().collect();
        let constrained_render = height_constrained.render();
        let constrained_lines: Vec<_> = constrained_render.lines().collect();

        let max_lines = wrapped_lines.len().max(barmode_lines.len()).max(constrained_lines.len());

        for i in 0..max_lines {
            let w = wrapped_lines.get(i).unwrap_or(&"");
            let b = barmode_lines.get(i).unwrap_or(&"");
            let c = constrained_lines.get(i).unwrap_or(&"");
            combined.push_str(&format!("{}  {}  {}\n", w, b, c));
        }

        let layout = BoxBuilder::new(&combined)
            .with_header(HeaderBuilder::new("✨ Advanced Features").align_center())
            .with_style(ROUNDED)
            .build();

        self.apply_color(&layout.render())
    }

    fn render_status(&self) -> String {
        let theme = self.current_theme();
        let status_text = format!(
            "Press [Q] or [Esc] to exit  •  Press [C] to cycle color themes  •  Theme: {}",
            theme.name
        );

        let layout = BoxBuilder::new(&status_text)
            .with_barmode()
            .with_style(DASHED)
            .build();

        self.apply_color(&layout.render())
    }
}

impl RoomPlugin for BoxyApiDemoPlugin {
    fn name(&self) -> &str {
        "boxy_api_demo"
    }

    fn init(&mut self, ctx: &mut RuntimeContext) -> room_mvp::Result<()> {
        ctx.set_zone_pre_rendered(PANEL_INFO, self.render_info_panel());
        ctx.set_zone_pre_rendered(PANEL_GEOMETRY, self.render_geometry_panel());
        ctx.set_zone_pre_rendered(PANEL_COMPONENTS, self.render_components_panel());
        ctx.set_zone_pre_rendered(PANEL_STYLES, self.render_styles_panel());
        ctx.set_zone_pre_rendered(PANEL_FEATURES, self.render_features_panel());
        ctx.set_zone_pre_rendered(STATUS_ZONE, self.render_status());
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext,
        event: &RuntimeEvent,
    ) -> room_mvp::Result<EventFlow> {
        if let RuntimeEvent::Key(key_event) = event {
            match key_event.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    ctx.request_exit();
                    Ok(EventFlow::Consumed)
                }
                KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    ctx.request_exit();
                    Ok(EventFlow::Consumed)
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.cycle_theme();
                    ctx.set_zone_pre_rendered(PANEL_INFO, self.render_info_panel());
                    ctx.set_zone_pre_rendered(PANEL_GEOMETRY, self.render_geometry_panel());
                    ctx.set_zone_pre_rendered(PANEL_COMPONENTS, self.render_components_panel());
                    ctx.set_zone_pre_rendered(PANEL_STYLES, self.render_styles_panel());
                    ctx.set_zone_pre_rendered(PANEL_FEATURES, self.render_features_panel());
                    ctx.set_zone_pre_rendered(STATUS_ZONE, self.render_status());
                    Ok(EventFlow::Consumed)
                }
                _ => Ok(EventFlow::Continue),
            }
        } else {
            Ok(EventFlow::Continue)
        }
    }
}