//! Minimal Boxy render inside the Room renderer.
//!
//! `cargo run --example workshop_boxy_minimal`

use boxy::{
    Body, BoxColors, BoxyConfig, Footer, Header, Status, WidthConfig, get_color_code,
    get_display_width, get_terminal_width,
};
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

    let rendered = render_box_to_string(&config);
    registry.apply_content(&"demo:panel".to_string(), rendered)?;

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

fn render_box_to_string(config: &BoxyConfig) -> String {
    let final_width = calculate_box_width(config);
    let inner_width = final_width.saturating_sub(2);
    let color_code = get_color_code(&config.colors.box_color);

    let text_color_code = match config.colors.text_color.as_str() {
        "auto" => get_color_code(&config.colors.box_color),
        "none" => "",
        other => get_color_code(other),
    };
    let title_color_code = config
        .colors
        .title_color
        .as_ref()
        .map(|name| get_color_code(name))
        .unwrap_or("");
    let status_color_code = config
        .colors
        .status_color
        .as_ref()
        .map(|name| get_color_code(name))
        .unwrap_or("");

    let header = Header::new(config);
    let body = Body::new(config);
    let status = Status::new(config);
    let footer = Footer::new(config);

    let mut output = String::new();
    output.push_str(&header.render(inner_width, &color_code));
    output.push('\n');

    let mut body_lines = body.render(
        inner_width,
        &color_code,
        &text_color_code,
        &title_color_code,
    );
    let status_lines = if status.should_render() {
        status.render(
            inner_width,
            &color_code,
            &text_color_code,
            &status_color_code,
        )
    } else {
        Vec::new()
    };

    if let Some(target_height) = config.fixed_height {
        let used = 1 + body_lines.len() + status_lines.len() + 1;
        if target_height > used {
            let filler = target_height - used;
            let pad = " ".repeat(config.width.h_padding);
            let available = inner_width.saturating_sub(2 * config.width.h_padding);
            let blank = format!(
                "{}{}{}{}{}{}{}",
                &color_code,
                config.style.vertical,
                boxy::RESET,
                &pad,
                " ".repeat(available),
                &pad,
                format!("{}{}{}", &color_code, config.style.vertical, boxy::RESET)
            );
            body_lines.extend(std::iter::repeat(blank).take(filler));
        }
    }

    for line in &body_lines {
        output.push_str(line);
        output.push('\n');
    }
    for line in &status_lines {
        output.push_str(line);
        output.push('\n');
    }

    output.push_str(&footer.render(inner_width, &color_code));
    output
}

fn calculate_box_width(config: &BoxyConfig) -> usize {
    let mut combined = config.text.clone();
    if let Some(title) = &config.title {
        combined.push('\n');
        combined.push_str(title);
    }
    if let Some(status) = &config.status_bar {
        combined.push('\n');
        combined.push_str(status);
    }

    let terminal_width = get_terminal_width();
    let base_width = match config.width.fixed_width {
        Some(fixed) => fixed.min(terminal_width as usize),
        None => {
            let content_max_width = combined
                .lines()
                .map(|line| get_display_width(line))
                .max()
                .unwrap_or(0);
            let ideal = content_max_width + config.width.h_padding.saturating_mul(2) + 2;
            ideal.min(terminal_width as usize)
        }
    };

    base_width.max(5)
}
