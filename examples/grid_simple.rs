//! Simple Grid Layout Demo
//!
//! Demonstrates the new GridLayout system integrated with RoomRuntime.
//! Shows a 3-column, 3-row grid with header, sidebar, main content, and footer.
//!
//! Features demonstrated:
//! - GridLayout creation with fixed, flex, and percent sizing
//! - Zone placement using GridArea helpers
//! - Integration with RoomRuntime (Phase 2 complete)
//! - Dynamic resize handling with grid recalculation
//! - Boxy integration for zone rendering
//!
//! ```bash
//! cargo run --example grid_simple
//! ```
//!
//! Try resizing your terminal to see the grid adapt!

use room_mvp::{
    AnsiRenderer, BoxConfig, CliDriver, EventFlow, GridArea, GridLayout, GridSize, Result,
    RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, Size,
};
use room_mvp::runtime::audit::{BootstrapAudit, NullRuntimeAudit};
use boxy::visual::ROUNDED;
use std::sync::Arc;
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal;

const HEADER_ZONE: &str = "header";
const SIDEBAR_ZONE: &str = "sidebar";
const MAIN_ZONE: &str = "main";
const ASIDE_ZONE: &str = "aside";
const FOOTER_ZONE: &str = "footer";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layout = build_grid_layout()?;
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();

    config.audit = Some(BootstrapAudit::new(Arc::new(NullRuntimeAudit)));

    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    let mut runtime = RoomRuntime::with_config(
        layout,
        renderer,
        Size::new(term_width, term_height),
        config,
    )?;

    runtime.register_plugin(GridDemoPlugin::new(term_width, term_height));

    CliDriver::new(runtime).run()?;
    Ok(())
}

fn build_grid_layout() -> Result<GridLayout> {
    let mut grid = GridLayout::new();

    // Define columns: 20 chars sidebar | flex 2x main | flex 1x aside
    grid.add_col(GridSize::Fixed(20))
        .add_col(GridSize::flex(2))
        .add_col(GridSize::flex(1));

    // Define rows: 3 lines header | flex body | 2 lines footer
    grid.add_row(GridSize::Fixed(3))
        .add_row(GridSize::flex(1))
        .add_row(GridSize::Fixed(2));

    // Optional: add gap between cells
    grid.with_gap(1);

    // Place zones
    grid.place(HEADER_ZONE, GridArea::span_cols(0, 0..3))?;  // Header spans all columns
    grid.place(SIDEBAR_ZONE, GridArea::cell(1, 0))?;         // Sidebar: row 1, col 0
    grid.place(MAIN_ZONE, GridArea::cell(1, 1))?;            // Main: row 1, col 1
    grid.place(ASIDE_ZONE, GridArea::cell(1, 2))?;           // Aside: row 1, col 2
    grid.place(FOOTER_ZONE, GridArea::span_cols(2, 0..3))?;  // Footer spans all columns

    Ok(grid)
}

struct GridDemoPlugin {
    term_width: u16,
    term_height: u16,
    resize_count: usize,
}

impl GridDemoPlugin {
    fn new(width: u16, height: u16) -> Self {
        Self {
            term_width: width,
            term_height: height,
            resize_count: 0,
        }
    }

    fn update_zones(&self, ctx: &mut RuntimeContext) {
        let config = BoxConfig::new(&ROUNDED);

        let header_content = format!(
            "Grid Layout Demo - Phase 2 Complete\n\nHeader spans all 3 columns • Press 'q' to quit\n\nSize: {}x{}\nResizes: {}",
            ctx.rect(HEADER_ZONE).map(|r| r.width).unwrap_or(0),
            ctx.rect(HEADER_ZONE).map(|r| r.height).unwrap_or(0),
            self.resize_count
        );
        match ctx.render_zone_with_box(HEADER_ZONE, &header_content, config) {
            Some(rendered) => ctx.set_zone_pre_rendered(HEADER_ZONE, rendered),
            None => ctx.set_zone_pre_rendered(HEADER_ZONE, String::new()),
        }

        let sidebar_content = format!(
            "Sidebar\n\nFixed width: 20 chars\n\nThis zone maintains a constant width.\n\nSize: {}x{}\nResizes: {}",
            ctx.rect(SIDEBAR_ZONE).map(|r| r.width).unwrap_or(0),
            ctx.rect(SIDEBAR_ZONE).map(|r| r.height).unwrap_or(0),
            self.resize_count
        );
        match ctx.render_zone_with_box(SIDEBAR_ZONE, &sidebar_content, config) {
            Some(rendered) => ctx.set_zone_pre_rendered(SIDEBAR_ZONE, rendered),
            None => ctx.set_zone_pre_rendered(SIDEBAR_ZONE, String::new()),
        }

        let main_content = format!(
            "Main Content\n\nFlex: 2x\n\nThis zone gets 2/3 of the flexible space (2 flex units).\n\nSize: {}x{}\nResizes: {}",
            ctx.rect(MAIN_ZONE).map(|r| r.width).unwrap_or(0),
            ctx.rect(MAIN_ZONE).map(|r| r.height).unwrap_or(0),
            self.resize_count
        );
        match ctx.render_zone_with_box(MAIN_ZONE, &main_content, config) {
            Some(rendered) => ctx.set_zone_pre_rendered(MAIN_ZONE, rendered),
            None => ctx.set_zone_pre_rendered(MAIN_ZONE, String::new()),
        }

        let aside_content = format!(
            "Aside\n\nFlex: 1x\n\nThis zone gets 1/3 of the flexible space (1 flex unit).\n\nSize: {}x{}\nResizes: {}",
            ctx.rect(ASIDE_ZONE).map(|r| r.width).unwrap_or(0),
            ctx.rect(ASIDE_ZONE).map(|r| r.height).unwrap_or(0),
            self.resize_count
        );
        match ctx.render_zone_with_box(ASIDE_ZONE, &aside_content, config) {
            Some(rendered) => ctx.set_zone_pre_rendered(ASIDE_ZONE, rendered),
            None => ctx.set_zone_pre_rendered(ASIDE_ZONE, String::new()),
        }

        let footer_content = format!(
            "Footer\n\nTerminal: {}x{} • Spans all columns • Gap: 1\n\nSize: {}x{}\nResizes: {}",
            self.term_width,
            self.term_height,
            ctx.rect(FOOTER_ZONE).map(|r| r.width).unwrap_or(0),
            ctx.rect(FOOTER_ZONE).map(|r| r.height).unwrap_or(0),
            self.resize_count
        );
        match ctx.render_zone_with_box(FOOTER_ZONE, &footer_content, config) {
            Some(rendered) => ctx.set_zone_pre_rendered(FOOTER_ZONE, rendered),
            None => ctx.set_zone_pre_rendered(FOOTER_ZONE, String::new()),
        }
    }
}

impl RoomPlugin for GridDemoPlugin {
    fn name(&self) -> &str {
        "grid_demo"
    }

    fn init(&mut self, ctx: &mut RuntimeContext) -> Result<()> {
        self.update_zones(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        match event {
            RuntimeEvent::Resize(size) => {
                self.term_width = size.width;
                self.term_height = size.height;
                self.resize_count += 1;
                self.update_zones(ctx);
                ctx.request_render();
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Key(key) => {
                if key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    ctx.request_exit();
                    Ok(EventFlow::Consumed)
                } else {
                    Ok(EventFlow::Continue)
                }
            }
            _ => Ok(EventFlow::Continue),
        }
    }
}
