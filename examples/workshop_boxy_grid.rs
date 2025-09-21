//! Room Workshop: Boxy Grid Layouts
//!
//! Walk through Boxy-powered grid compositions alongside Room's layout solver.
//!
//! ```bash
//! bin/examples.sh run workshop_boxy_grid             # default (2x2 grid)
//! bin/examples.sh run workshop_boxy_grid -- wide     # 1x3 metric strip
//! ```
//!
//! Each scenario prints the resolved rectangles from `LayoutTree` and renders
//! sample Boxy panels so you can visualise the output. Extend the scenarios as
//! directed in `docs/ref/workshops/workshop_boxy_grid.md`.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, Write};

use boxy::{BoxColors, BoxyConfig, WidthConfig, render_to_string};
use room_mvp::{
    AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, Result, Size, ZoneRegistry,
};

fn main() -> Result<()> {
    let scenario = std::env::args().nth(1).unwrap_or_else(|| "2x2".to_string());
    match scenario.as_str() {
        "wide" => run_wide_strip()?,
        "2x2" | _ => run_two_by_two()?,
    }
    Ok(())
}

fn run_two_by_two() -> Result<()> {
    println!("Room + Boxy Workshop — Scenario: 2x2 grid\n");

    let layout = LayoutTree::new(LayoutNode {
        id: "workshop:grid".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1), Constraint::Flex(1)],
        children: vec![
            LayoutNode {
                id: "workshop:grid.row1".into(),
                direction: Direction::Row,
                constraints: vec![Constraint::Flex(1), Constraint::Flex(1)],
                children: vec![
                    LayoutNode::leaf("workshop:grid.row1.left"),
                    LayoutNode::leaf("workshop:grid.row1.right"),
                ],
                gap: 1,
                padding: 0,
            },
            LayoutNode {
                id: "workshop:grid.row2".into(),
                direction: Direction::Row,
                constraints: vec![Constraint::Flex(1), Constraint::Flex(1)],
                children: vec![
                    LayoutNode::leaf("workshop:grid.row2.left"),
                    LayoutNode::leaf("workshop:grid.row2.right"),
                ],
                gap: 1,
                padding: 0,
            },
        ],
        gap: 1,
        padding: 1,
    });

    let canvas_size = Size::new(100, 28);
    let rects = layout.solve(canvas_size)?;

    let panels = vec![
        panel(
            "workshop:grid.row1.left",
            "Status",
            "Status: OK\nUptime: 4h",
            "cyan",
        ),
        panel(
            "workshop:grid.row1.right",
            "Timeline",
            "Events\n- build\n- deploy",
            "blue",
        ),
        panel(
            "workshop:grid.row2.left",
            "Metrics",
            "CPU: 42%\nRAM: 1.3GB",
            "magenta",
        ),
        panel(
            "workshop:grid.row2.right",
            "Notes",
            "Add TODOs here",
            "green",
        ),
    ];
    render_grid_preview("2x2", canvas_size, &rects, &panels)?;
    log_rects(&rects);

    println!("Next steps: modify constraints to mix FIXED/PERCENT values and re-run.");
    Ok(())
}

fn run_wide_strip() -> Result<()> {
    println!("Room + Boxy Workshop — Scenario: Wide metric strip\n");

    let layout = LayoutTree::new(LayoutNode {
        id: "workshop:strip".into(),
        direction: Direction::Row,
        constraints: vec![
            Constraint::Flex(2),
            Constraint::Flex(1),
            Constraint::Flex(1),
        ],
        children: vec![
            LayoutNode::leaf("workshop:strip.main"),
            LayoutNode::leaf("workshop:strip.metric"),
            LayoutNode::leaf("workshop:strip.activity"),
        ],
        gap: 1,
        padding: 1,
    });

    let canvas_size = Size::new(120, 20);
    let rects = layout.solve(canvas_size)?;

    let panels = vec![
        panel(
            "workshop:strip.main",
            "Overview",
            "Main panel with detailed text...",
            "teal",
        ),
        panel("workshop:strip.metric", "Metric", "Latency\n38ms", "yellow"),
        panel(
            "workshop:strip.activity",
            "Activity",
            "Ops\ndeploy-42",
            "red",
        ),
    ];
    render_grid_preview("wide", canvas_size, &rects, &panels)?;
    log_rects(&rects);

    println!("Next steps: clamp widths with WidthConfig::fixed_width and compare outcomes.");
    Ok(())
}

fn log_rects(rects: &HashMap<String, room_mvp::Rect>) {
    let ordered: BTreeMap<_, _> = rects.iter().collect();
    println!("Resolved Rectangles (id → Rect):\n--------------------------------");
    for (id, rect) in ordered {
        println!("{id:<35} -> {:?}", rect);
    }
    println!();
}

#[derive(Clone, Copy)]
struct PanelSpec<'a> {
    zone_id: &'a str,
    title: &'a str,
    body: &'a str,
    color: &'a str,
}

fn panel(
    zone_id: &'static str,
    title: &'static str,
    body: &'static str,
    color: &'static str,
) -> PanelSpec<'static> {
    PanelSpec {
        zone_id,
        title,
        body,
        color,
    }
}

fn render_grid_preview(
    scenario: &str,
    canvas_size: Size,
    rects: &HashMap<String, room_mvp::Rect>,
    panels: &[PanelSpec<'_>],
) -> Result<()> {
    println!("Rendered Grid Preview ({scenario}):\n------------------------------");

    let mut registry = ZoneRegistry::new();
    registry.sync_layout(rects);
    registry.take_dirty();

    for panel in panels {
        if let Some(rect) = rects.get(panel.zone_id) {
            let mut config = sample_boxy_config(panel.title, panel.body, panel.color);
            config.width = WidthConfig {
                fixed_width: Some(rect.width.max(4) as usize),
                enable_wrapping: true,
                ..WidthConfig::default()
            };
            config.fixed_height = Some(rect.height.max(4) as usize);
            registry.apply_pre_rendered(&panel.zone_id.to_string(), render_to_string(&config))?;
        } else {
            println!("⚠️  missing rect for zone `{}`; skipping", panel.zone_id);
        }
    }

    let dirty = registry.take_dirty();
    if dirty.is_empty() {
        println!("(no panels to render)\n");
        return Ok(());
    }

    let mut renderer = AnsiRenderer::with_default();
    let mut buffer = Vec::new();
    renderer.render(&mut buffer, &dirty)?;
    let canvas = String::from_utf8(buffer).unwrap_or_default();

    let mut stdout = io::stdout();
    write!(stdout, "\x1b[2J\x1b[H")?;
    stdout.write_all(canvas.as_bytes())?;
    let esc = 27u8 as char;
    write!(stdout, "{esc}[{};1H", canvas_size.height.saturating_add(2))?;
    writeln!(stdout)?;
    stdout.flush()?;

    Ok(())
}

fn sample_boxy_config(title: &str, body: &str, color: &str) -> BoxyConfig {
    BoxyConfig {
        title: Some(title.to_string()),
        status_bar: None,
        colors: BoxColors {
            box_color: color.to_string(),
            text_color: "auto".to_string(),
            title_color: Some("white".to_string()),
            status_color: Some("white".to_string()),
            header_color: None,
            footer_color: None,
        },
        width: WidthConfig::default(),
        fixed_height: None,
        text: format!("{body}\n\nExperiment with panel constraints"),
        ..BoxyConfig::default()
    }
}
