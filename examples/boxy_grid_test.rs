//! Boxy Grid Test - Testing grid of colored 1-char Boxy panels

use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, RoomPlugin,
    RoomRuntime, RuntimeConfig, RuntimeContext, Size, SimulatedLoop,
};
use boxy::{BoxyConfig, BoxColors, render_to_string};

const COLORS: &[&str] = &["red", "green", "blue", "yellow", "cyan", "magenta"];

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Testing Boxy Grid - Colored 1-char panels");

    // Calculate grid based on terminal size
    let term_width = 80;  // Standard terminal width
    let term_height = 24; // Standard terminal height

    // Each boxy is 3x3, calculate how many fit
    let cols = term_width / 3;
    let rows = term_height / 3;

    println!("Terminal: {}x{}, Grid: {}x{} boxes", term_width, term_height, cols, rows);

    let layout = build_grid_layout(rows, cols);
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();

    // Check headless mode early
    let is_headless = std::env::var("HEADLESS").is_ok();
    if is_headless {
        config.simulated_loop = Some(SimulatedLoop::ticks(3));
    }

    let mut runtime = RoomRuntime::with_config(layout, renderer, Size::new(term_width as u16, term_height as u16), config)?;
    runtime.register_plugin(BoxyGridPlugin::new(rows, cols));

    // Handle both modes
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

fn build_grid_layout(rows: usize, cols: usize) -> LayoutTree {
    let mut row_children = Vec::new();

    for row in 0..rows {
        let mut col_children = Vec::new();
        for col in 0..cols {
            col_children.push(LayoutNode::leaf(format!("box_{}_{}", row, col)));
        }

        row_children.push(LayoutNode {
            id: format!("row_{}", row),
            direction: Direction::Row,
            constraints: vec![Constraint::Flex(1); cols],
            children: col_children,
            gap: 0,
            padding: 0,
        });
    }

    LayoutTree::new(LayoutNode {
        id: "grid_root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1); rows],
        children: row_children,
        gap: 0,
        padding: 0,
    })
}

struct BoxyGridPlugin {
    rows: usize,
    cols: usize,
}

impl BoxyGridPlugin {
    fn new(rows: usize, cols: usize) -> Self {
        Self { rows, cols }
    }
}

impl RoomPlugin for BoxyGridPlugin {
    fn name(&self) -> &str {
        "boxy_grid"
    }

    fn init(&mut self, ctx: &mut RuntimeContext) -> room_mvp::Result<()> {
        for row in 0..self.rows {
            for col in 0..self.cols {
                let zone_id = format!("box_{}_{}", row, col);
                let color = COLORS[(row + col) % COLORS.len()];
                let char = ((row + col) % 26) as u8 + b'A';

                let config = BoxyConfig {
                    text: (char as char).to_string(),
                    colors: BoxColors {
                        box_color: color.to_string(),
                        text_color: "white".to_string(),
                        ..BoxColors::default()
                    },
                    ..BoxyConfig::default()
                };

                let rendered = render_to_string(&config);
                ctx.set_zone_pre_rendered(&zone_id, rendered);
            }
        }
        Ok(())
    }
}