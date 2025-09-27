//! Grid Layout Showcase - Comprehensive demonstration
//!
//! This example showcases all GridLayout features in a beautiful dashboard:
//! - Mixed sizing: Fixed (20 cols), Flex (2 units), Percent (20%)
//! - Spanning zones: header and footer span all columns
//! - Gap handling: 1-cell gap between zones
//! - Multiple box styles: ROUNDED, DOUBLE, HEAVY, NORMAL
//! - Resize handling: all zones adapt dynamically
//! - Helper usage: render_zone_with_box() for one-call rendering
//! - Collapse modes: zones gracefully handle small sizes
//!
//! Layout:
//! ```
//! ┌───────────────────────────────────────────┐
//! │          HEADER (spans all)               │  <- Fixed 5 rows
//! ├────────┬──────────────────┬───────────────┤
//! │        │                  │               │
//! │ SIDE   │   MAIN CONTENT   │    ASIDE      │  <- Flex 1 (fills space)
//! │ BAR    │   (Flex 2x)      │  (Percent 20%)│
//! │        │                  │               │
//! ├────────┴──────────────────┴───────────────┤
//! │          FOOTER (spans all)               │  <- Fixed 3 rows
//! └───────────────────────────────────────────┘
//! ```
//!
//! Run with:
//! ```bash
//! cargo run --example grid_showcase
//! ```
//!
//! Try resizing your terminal to see the grid adapt!

use boxy::visual::{DOUBLE, HEAVY, NORMAL, ROUNDED};
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal;
use room_mvp::runtime::audit::{BootstrapAudit, NullRuntimeAudit};
use room_mvp::{
    AnsiRenderer, BoxConfig, CliDriver, CollapseMode, EventFlow, GridArea, GridLayout, GridSize,
    Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, Size,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

const HEADER_ZONE: &str = "header";
const SIDEBAR_ZONE: &str = "sidebar";
const MAIN_ZONE: &str = "main";
const ASIDE_ZONE: &str = "aside";
const FOOTER_ZONE: &str = "footer";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layout = build_showcase_layout()?;
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();

    config.audit = Some(BootstrapAudit::new(Arc::new(NullRuntimeAudit)));

    let (term_width, term_height) = terminal::size().unwrap_or((120, 40));

    let mut runtime = RoomRuntime::with_config(
        layout,
        renderer,
        Size::new(term_width, term_height),
        config,
    )?;

    runtime.register_plugin(ShowcasePlugin::new(term_width, term_height));

    CliDriver::new(runtime).run()?;
    Ok(())
}

fn build_showcase_layout() -> Result<GridLayout> {
    let mut grid = GridLayout::new();

    grid.add_col(GridSize::Fixed(20))
        .add_col(GridSize::flex(2))
        .add_col(GridSize::percent(20));

    grid.add_row(GridSize::Fixed(5))
        .add_row(GridSize::flex(1))
        .add_row(GridSize::Fixed(3));

    grid.with_gap(1);

    grid.place(HEADER_ZONE, GridArea::span_cols(0, 0..3))?;
    grid.place(SIDEBAR_ZONE, GridArea::cell(1, 0))?;
    grid.place(MAIN_ZONE, GridArea::cell(1, 1))?;
    grid.place(ASIDE_ZONE, GridArea::cell(1, 2))?;
    grid.place(FOOTER_ZONE, GridArea::span_cols(2, 0..3))?;

    Ok(grid)
}

struct ShowcasePlugin {
    term_width: u16,
    term_height: u16,
    resize_count: usize,
    update_count: usize,
    start_time: Instant,
}

impl ShowcasePlugin {
    fn new(width: u16, height: u16) -> Self {
        Self {
            term_width: width,
            term_height: height,
            resize_count: 0,
            update_count: 0,
            start_time: Instant::now(),
        }
    }

    fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    fn update_zones(&mut self, ctx: &mut RuntimeContext) {
        self.update_count += 1;

        let uptime = self.uptime();
        let uptime_str = format!("{}m {:02}s", uptime.as_secs() / 60, uptime.as_secs() % 60);

        let header_rect = ctx.rect(HEADER_ZONE);
        let header_content = format!(
            "🎯 GRID LAYOUT SHOWCASE\n\n\
            Terminal: {}×{} • Uptime: {} • Updates: {}\n\
            Zone: {}×{} • Press 'q' to quit, 'r' to refresh",
            self.term_width,
            self.term_height,
            uptime_str,
            self.update_count,
            header_rect.map(|r| r.width).unwrap_or(0),
            header_rect.map(|r| r.height).unwrap_or(0)
        );

        let header_config = BoxConfig::new(&DOUBLE)
            .with_min_width(30)
            .with_min_height(5);

        if let Some(rendered) = ctx.render_zone_with_box(HEADER_ZONE, &header_content, header_config)
        {
            ctx.set_zone(HEADER_ZONE, rendered);
        }

        let sidebar_rect = ctx.rect(SIDEBAR_ZONE);
        let sidebar_content = format!(
            "📊 SIDEBAR\n\n\
            Type: Fixed\n\
            Width: 20 cells\n\
            Always constant\n\n\
            Zone: {}×{}\n\
            Resizes: {}",
            sidebar_rect.map(|r| r.width).unwrap_or(0),
            sidebar_rect.map(|r| r.height).unwrap_or(0),
            self.resize_count
        );

        let sidebar_config = BoxConfig::new(&ROUNDED)
            .with_min_width(15)
            .with_min_height(8)
            .with_collapse_mode(CollapseMode::Hide);

        if let Some(rendered) =
            ctx.render_zone_with_box(SIDEBAR_ZONE, &sidebar_content, sidebar_config)
        {
            ctx.set_zone(SIDEBAR_ZONE, rendered);
        }

        let main_rect = ctx.rect(MAIN_ZONE);
        let main_content = format!(
            "📝 MAIN CONTENT AREA\n\n\
            Type: Flex (2 units)\n\
            Takes 2/3 of flexible space\n\
            Adapts to terminal size\n\n\
            This is where your primary content goes.\n\
            GridLayout automatically calculates the\n\
            optimal dimensions based on your sizing\n\
            constraints.\n\n\
            Current zone: {}×{}\n\
            Aspect ratio: {:.2}\n\
            Total resizes: {}",
            main_rect.map(|r| r.width).unwrap_or(0),
            main_rect.map(|r| r.height).unwrap_or(0),
            main_rect
                .map(|r| r.width as f64 / r.height.max(1) as f64)
                .unwrap_or(0.0),
            self.resize_count
        );

        let main_config = BoxConfig::new(&HEAVY).with_min_width(20).with_min_height(10);

        if let Some(rendered) = ctx.render_zone_with_box(MAIN_ZONE, &main_content, main_config) {
            ctx.set_zone(MAIN_ZONE, rendered);
        }

        let aside_rect = ctx.rect(ASIDE_ZONE);
        let aside_content = format!(
            "💡 ASIDE\n\n\
            Type: Percent\n\
            Width: 20%\n\
            Proportional\n\n\
            Zone: {}×{}\n\
            Gap: 1 cell\n\n\
            Collapse: {}\n\
            Updates: {}",
            aside_rect.map(|r| r.width).unwrap_or(0),
            aside_rect.map(|r| r.height).unwrap_or(0),
            if aside_rect.map(|r| r.width).unwrap_or(0) < 15 {
                "⚠️  Warning"
            } else {
                "✅ Normal"
            },
            self.update_count
        );

        let aside_config = BoxConfig::new(&NORMAL)
            .with_min_width(12)
            .with_min_height(8)
            .with_collapse_mode(CollapseMode::Show);

        if let Some(rendered) = ctx.render_zone_with_box(ASIDE_ZONE, &aside_content, aside_config)
        {
            ctx.set_zone(ASIDE_ZONE, rendered);
        }

        let footer_rect = ctx.rect(FOOTER_ZONE);
        let footer_content = format!(
            "⚡ Status: Ready • Grid: 3 cols × 3 rows • Gap: 1 • Zone: {}×{} • Phase 3 Complete ✅",
            footer_rect.map(|r| r.width).unwrap_or(0),
            footer_rect.map(|r| r.height).unwrap_or(0)
        );

        let footer_config = BoxConfig::new(&ROUNDED).with_min_width(40).with_min_height(3);

        if let Some(rendered) = ctx.render_zone_with_box(FOOTER_ZONE, &footer_content, footer_config)
        {
            ctx.set_zone(FOOTER_ZONE, rendered);
        }
    }
}

impl RoomPlugin for ShowcasePlugin {
    fn name(&self) -> &str {
        "grid_showcase"
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
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    ctx.request_exit();
                    Ok(EventFlow::Consumed)
                } else if key.code == KeyCode::Char('r') {
                    self.update_zones(ctx);
                    ctx.request_render();
                    Ok(EventFlow::Consumed)
                } else {
                    Ok(EventFlow::Continue)
                }
            }
            _ => Ok(EventFlow::Continue),
        }
    }
}