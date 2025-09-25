//! Minimal Boxy render inside the Room renderer.
//!
//! `cargo run --example workshop_boxy_minimal`

use boxy::{BoxColors, BoxyConfig, WidthConfig, render_to_string};
use room_mvp::{
    AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, Result, Size, ZoneRegistry,
};

fn main() -> Result<()> {
    let layout = LayoutTree::new(LayoutNode {
        id: "demo:root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1)],
        children: vec![LayoutNode::leaf("demo:panel")],
        gap: 0,
        padding: 1,
    });

    let size = Size::new(40, 12);
    let rects = layout.solve(size)?;

    let mut registry = ZoneRegistry::new();
    registry.sync_layout(&rects);
    registry.take_dirty();

    let rect = rects.get("demo:panel").copied().unwrap();
    let config = BoxyConfig {
        title: Some("Minimal Box".into()),
        status_bar: None,
        colors: BoxColors {
            box_color: "cyan".into(),
            text_color: "auto".into(),
            title_color: Some("white".into()),
            status_color: None,
            header_color: None,
            footer_color: None,
        },
        width: WidthConfig {
            fixed_width: Some(rect.width.max(4) as usize),
            enable_wrapping: true,
            ..WidthConfig::default()
        },
        fixed_height: Some(rect.height.max(4) as usize),
        text: "This is a single Boxy panel rendered via Room.".into(),
        ..BoxyConfig::default()
    };

    let rendered = render_to_string(&config);
    registry.apply_pre_rendered(&"demo:panel".to_string(), rendered)?;

    let dirty = registry.take_dirty();
    let mut renderer = AnsiRenderer::with_default();
    let mut buffer = Vec::new();
    renderer.render(&mut buffer, &dirty)?;

    use std::io::Write;
    let mut stdout = std::io::stdout();
    write!(stdout, "\x1b[2J\x1b[H")?;
    stdout.write_all(&buffer)?;
    writeln!(stdout)?;
    stdout.flush()?;

    println!("Rect: {:?}", rect);
    Ok(())
}
