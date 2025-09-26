//! Dynamic Grid Layout with Boxy - Complete Integration
//!
//! Shows how Room's existing layout system provides:
//! - Dynamic resize handling (already built-in)
//! - Percentage-based splits (Constraint::Percent)
//! - Proportional layouts (Constraint::Flex)
//! - Integration with Boxy for box rendering
//!
//! This demonstrates that you DON'T need to build a new Grid system -
//! Room's constraint layout already does what you need!

use room_mvp::*;
use room_mvp::runtime::audit::{BootstrapAudit, NullRuntimeAudit};
use std::sync::Arc;
use boxy::api::layout::{BoxBuilder, HeaderBuilder, FooterBuilder};
use boxy::visual::{ROUNDED, DOUBLE, HEAVY};
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal;

// Zone IDs for our grid
const ZONE_HEADER: &str = "grid:header";
const ZONE_SIDEBAR: &str = "grid:sidebar";
const ZONE_MAIN: &str = "grid:main";
const ZONE_SIDE_PANEL: &str = "grid:side";
const ZONE_FOOTER: &str = "grid:footer";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layout = build_responsive_grid();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.audit = Some(BootstrapAudit::new(Arc::new(NullRuntimeAudit)));

    let (term_width, term_height) = terminal::size().unwrap_or((120, 30));

    let mut runtime = RoomRuntime::with_config(
        layout,
        renderer,
        Size::new(term_width, term_height),
        config,
    )?;

    runtime.register_plugin(DynamicGridPlugin::new());

    CliDriver::new(runtime).run()?;
    Ok(())
}

/// Room's constraint-based layout - percentage and flex splits
fn build_responsive_grid() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(3),    // Header - fixed 3 rows
            Constraint::Flex(1),     // Body - grows to fill
            Constraint::Fixed(2),    // Footer - fixed 2 rows
        ],
        children: vec![
            LayoutNode::leaf(ZONE_HEADER),
            // Body row split into 3 columns
            LayoutNode {
                id: "body".into(),
                direction: Direction::Row,
                constraints: vec![
                    Constraint::Percent(20),  // Sidebar - 20% width
                    Constraint::Flex(2),      // Main - 2x proportional
                    Constraint::Flex(1),      // Side panel - 1x proportional
                ],
                children: vec![
                    LayoutNode::leaf(ZONE_SIDEBAR),
                    LayoutNode::leaf(ZONE_MAIN),
                    LayoutNode::leaf(ZONE_SIDE_PANEL),
                ],
                gap: 1,
                padding: 0,
            },
            LayoutNode::leaf(ZONE_FOOTER),
        ],
        gap: 0,
        padding: 1,
    })
}

struct DynamicGridPlugin {
    resize_count: usize,
    term_width: u16,
    term_height: u16,
}

impl DynamicGridPlugin {
    fn new() -> Self {
        let (width, height) = terminal::size().unwrap_or((120, 30));
        Self {
            resize_count: 0,
            term_width: width,
            term_height: height,
        }
    }

    /// Render a zone using its Room-calculated Rect
    fn render_zone(&self, ctx: &RuntimeContext, zone_id: &str, title: &str, style: boxy::visual::BoxStyle) -> Option<String> {
        // Get the Rect that Room calculated for this zone
        let rect = ctx.rect(zone_id)?;

        let content = format!(
            "Zone: {}\n\
             Position: ({}, {})\n\
             Size: {}x{}\n\
             \n\
             This zone automatically resizes\n\
             when the terminal changes!\n\
             \n\
             Terminal resizes: {}",
            zone_id,
            rect.x, rect.y,
            rect.width, rect.height,
            self.resize_count
        );

        // Pass Room's calculated dimensions to Boxy
        Some(BoxBuilder::new(&content)
            .with_header(HeaderBuilder::new(title).align_center())
            .with_footer(FooterBuilder::new(&format!("{}x{}", rect.width, rect.height)).align_center())
            .with_style(style)
            .with_fixed_width(rect.width as usize)
            .with_fixed_height(rect.height as usize)
            .build()
            .render())
    }

    fn update_all_zones(&self, ctx: &mut RuntimeContext) {
        if let Some(rendered) = self.render_zone(ctx, ZONE_HEADER, "🌟 Dynamic Grid Demo", ROUNDED) {
            ctx.set_zone_pre_rendered(ZONE_HEADER, rendered);
        }

        if let Some(rendered) = self.render_zone(ctx, ZONE_SIDEBAR, "📁 Sidebar (20%)", DOUBLE) {
            ctx.set_zone_pre_rendered(ZONE_SIDEBAR, rendered);
        }

        if let Some(rendered) = self.render_zone(ctx, ZONE_MAIN, "📄 Main Content (2x flex)", HEAVY) {
            ctx.set_zone_pre_rendered(ZONE_MAIN, rendered);
        }

        if let Some(rendered) = self.render_zone(ctx, ZONE_SIDE_PANEL, "🔧 Side Panel (1x flex)", DOUBLE) {
            ctx.set_zone_pre_rendered(ZONE_SIDE_PANEL, rendered);
        }

        // Footer shows terminal info
        if let Some(footer_rect) = ctx.rect(ZONE_FOOTER) {
            let footer_content = format!(
                "Terminal: {}x{} | Resize Count: {} | Press [Q] to exit",
                self.term_width, self.term_height, self.resize_count
            );

            let footer = BoxBuilder::new(&footer_content)
                .with_style(ROUNDED)
                .with_fixed_width(footer_rect.width as usize)
                .with_fixed_height(footer_rect.height as usize)
                .build()
                .render();

            ctx.set_zone_pre_rendered(ZONE_FOOTER, footer);
        }
    }
}

impl RoomPlugin for DynamicGridPlugin {
    fn name(&self) -> &str {
        "dynamic_grid"
    }

    fn init(&mut self, ctx: &mut RuntimeContext) -> room_mvp::Result<()> {
        self.update_all_zones(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext,
        event: &RuntimeEvent,
    ) -> room_mvp::Result<EventFlow> {
        match event {
            RuntimeEvent::Key(key_event) => {
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                        ctx.request_exit();
                        Ok(EventFlow::Consumed)
                    }
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        ctx.request_exit();
                        Ok(EventFlow::Consumed)
                    }
                    _ => Ok(EventFlow::Continue),
                }
            }
            RuntimeEvent::Resize(new_size) => {
                // Room AUTOMATICALLY recalculates all zone Rects!
                // We just need to re-render with the new dimensions
                self.term_width = new_size.width;
                self.term_height = new_size.height;
                self.resize_count += 1;
                self.update_all_zones(ctx);
                ctx.request_render();
                Ok(EventFlow::Continue)
            }
            _ => Ok(EventFlow::Continue),
        }
    }
}