use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::ExecutableCommand;
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, Clear, ClearType};

use boxy::{
    Body, BoxColors, BoxyConfig, Footer, Header, RESET, Status, WidthConfig, get_color_code,
    get_display_width, get_terminal_width,
};
use room_mvp::{
    AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, Rect, Result, Size, ZoneRegistry,
};
use rsb::prelude::*;
use rsb::visual::glyphs::{glyph, glyph_enable};
use std::env;

const HEADER_ZONE: &str = "app:demo.header";
const FOOTER_ZONE: &str = "app:demo.footer";
const PANEL_STATUS_ZONE: &str = "app:demo.panel.status";
const PANEL_METRICS_ZONE: &str = "app:demo.panel.metrics";
const PANEL_LOG_ZONE: &str = "app:demo.panel.log";

struct BoxyPanel {
    id: &'static str,
    config: BoxyConfig,
    cache: Option<String>,
    dirty: bool,
    base_color: &'static str,
    focus_color: &'static str,
}

impl BoxyPanel {
    fn new(
        id: &'static str,
        mut config: BoxyConfig,
        base_color: &'static str,
        focus_color: &'static str,
    ) -> Self {
        config.colors.box_color = base_color.to_string();
        Self {
            id,
            config,
            cache: None,
            dirty: true,
            base_color,
            focus_color,
        }
    }

    fn id(&self) -> &'static str {
        self.id
    }

    fn render(&mut self) -> &str {
        if self.dirty {
            let rendered = render_box_to_string(self.config.clone());
            self.cache = Some(rendered);
            self.dirty = false;
        }
        self.cache.as_deref().unwrap_or("")
    }

    fn set_focus(&mut self, focused: bool) {
        let target = if focused {
            self.focus_color
        } else {
            self.base_color
        };
        if self.config.colors.box_color != target {
            self.config.colors.box_color = target.to_string();
            self.mark_dirty();
        }
    }

    fn replace_text(&mut self, text: String) {
        if self.config.text != text {
            self.config.text = text;
            self.mark_dirty();
        }
    }

    fn append_line(&mut self, line: &str, max_lines: usize) {
        let mut lines: Vec<String> = self.config.text.lines().map(|s| s.to_string()).collect();
        lines.push(line.to_string());
        if lines.len() > max_lines {
            let excess = lines.len() - max_lines;
            lines.drain(0..excess);
        }
        let joined = lines.join("\n");
        self.replace_text(joined);
    }

    fn update_title(&mut self, title: Option<String>) {
        if self.config.title != title {
            self.config.title = title;
            self.mark_dirty();
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.cache = None;
    }
}

fn main() -> Result<()> {
    glyph_enable();

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout
        .execute(terminal::EnterAlternateScreen)?
        .execute(Clear(ClearType::All))?
        .execute(cursor::Hide)?;

    let result = run_app(&mut stdout);

    stdout.execute(cursor::Show).ok();
    stdout.execute(terminal::LeaveAlternateScreen).ok();
    terminal::disable_raw_mode().ok();

    result
}

fn run_app(stdout: &mut impl Write) -> Result<()> {
    let (width, height) = terminal::size()?;
    let layout = build_layout();
    let mut rects = layout.solve(Size::new(width, height))?;

    let mut registry = ZoneRegistry::new();
    registry.sync_layout(&rects);

    let mut renderer = AnsiRenderer::with_default();

    let mut panels = init_panels();
    let mut focused = 0usize;
    panels[focused].set_focus(true);

    let mut last_update = Instant::now();
    let mut tick = 0u32;

    render_state(
        stdout,
        &mut renderer,
        &mut registry,
        &rects,
        &mut panels,
        focused,
        tick,
    )?;

    loop {
        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Tab => {
                        panels[focused].set_focus(false);
                        focused = (focused + 1) % panels.len();
                        panels[focused].set_focus(true);
                        render_state(
                            stdout,
                            &mut renderer,
                            &mut registry,
                            &rects,
                            &mut panels,
                            focused,
                            tick,
                        )?;
                    }
                    KeyCode::BackTab => {
                        panels[focused].set_focus(false);
                        focused = if focused == 0 {
                            panels.len() - 1
                        } else {
                            focused - 1
                        };
                        panels[focused].set_focus(true);
                        render_state(
                            stdout,
                            &mut renderer,
                            &mut registry,
                            &rects,
                            &mut panels,
                            focused,
                            tick,
                        )?;
                    }
                    KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        apply_status_cycle(&mut panels, tick);
                        render_state(
                            stdout,
                            &mut renderer,
                            &mut registry,
                            &rects,
                            &mut panels,
                            focused,
                            tick,
                        )?;
                    }
                    _ => {}
                },
                Event::Resize(new_width, new_height) => {
                    rects = layout.solve(Size::new(new_width, new_height))?;
                    registry.sync_layout(&rects);
                    render_state(
                        stdout,
                        &mut renderer,
                        &mut registry,
                        &rects,
                        &mut panels,
                        focused,
                        tick,
                    )?;
                }
                _ => {}
            }
        }

        if last_update.elapsed() >= Duration::from_secs(3) {
            tick = tick.wrapping_add(1);
            apply_status_cycle(&mut panels, tick);
            render_state(
                stdout,
                &mut renderer,
                &mut registry,
                &rects,
                &mut panels,
                focused,
                tick,
            )?;
            last_update = Instant::now();
        }
    }

    Ok(())
}

fn init_panels() -> Vec<BoxyPanel> {
    let status_panel = BoxyPanel::new(
        PANEL_STATUS_ZONE,
        BoxyConfig {
            text: "Status: BOOTING\nCPU: --\nRAM: --".to_string(),
            title: Some(format!("{} Status", glyph("gear"))),
            colors: BoxColors {
                box_color: "blue".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("white".to_string()),
                ..Default::default()
            },
            width: WidthConfig {
                fixed_width: Some(26),
                ..Default::default()
            },
            ..Default::default()
        },
        "blue",
        "yellow",
    );

    let metrics_panel = BoxyPanel::new(
        PANEL_METRICS_ZONE,
        BoxyConfig {
            text: metrics_text(0),
            title: Some(format!("{} Metrics", glyph("bolt"))),
            colors: BoxColors {
                box_color: "purple".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("white".to_string()),
                ..Default::default()
            },
            width: WidthConfig {
                fixed_width: Some(26),
                ..Default::default()
            },
            ..Default::default()
        },
        "purple",
        "magenta",
    );

    let mut activity_panel = BoxyPanel::new(
        PANEL_LOG_ZONE,
        BoxyConfig {
            text: "Activity log ready.".to_string(),
            title: Some(format!("{} Activity", glyph("star"))),
            colors: BoxColors {
                box_color: "cyan".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("black".to_string()),
                ..Default::default()
            },
            width: WidthConfig {
                fixed_width: Some(26),
                ..Default::default()
            },
            ..Default::default()
        },
        "cyan",
        "green",
    );
    activity_panel.append_line("Awaiting updates...", 6);

    vec![status_panel, metrics_panel, activity_panel]
}

fn apply_status_cycle(panels: &mut [BoxyPanel], tick: u32) {
    if let Some(status_panel) = panels.iter_mut().find(|p| p.id() == PANEL_STATUS_ZONE) {
        let (status_text, color, title) = match tick % 4 {
            0 => ("Status: STABLE\nCPU: 37%\nRAM: 1.1GB", "blue", "Status"),
            1 => ("Status: BUSY\nCPU: 63%\nRAM: 1.6GB", "yellow", "Status"),
            2 => ("Status: HOT\nCPU: 82%\nRAM: 2.2GB", "red", "Status"),
            _ => ("Status: COOL\nCPU: 28%\nRAM: 0.9GB", "green", "Status"),
        };
        status_panel.replace_text(format!("{}\n{}", status_text, tick_line(tick)));
        status_panel.config.colors.box_color = color.to_string();
        status_panel.update_title(Some(format!("{} {}", glyph("gear"), title)));
        status_panel.mark_dirty();
    }

    if let Some(metrics_panel) = panels.iter_mut().find(|p| p.id() == PANEL_METRICS_ZONE) {
        metrics_panel.replace_text(metrics_text(tick));
    }

    if let Some(activity_panel) = panels.iter_mut().find(|p| p.id() == PANEL_LOG_ZONE) {
        let message = match tick % 4 {
            0 => "System heartbeat OK",
            1 => "Workload batch submitted",
            2 => "Autoscaler responding",
            _ => "Cleanup cycle complete",
        };
        activity_panel.append_line(message, 6);
    }
}

fn metrics_text(tick: u32) -> String {
    let latency = 18 + ((tick as i32) % 7);
    let throughput = 240 + ((tick as i32) % 50);
    format!(
        "Req/s: {}\nLatency: {} ms\nErrors: {}\nUptime: {}h",
        throughput,
        latency,
        tick % 3,
        12 + (tick % 9)
    )
}

fn tick_line(tick: u32) -> String {
    format!("Update: #{}", tick)
}

fn render_state(
    stdout: &mut impl Write,
    renderer: &mut AnsiRenderer,
    registry: &mut ZoneRegistry,
    rects: &std::collections::HashMap<String, Rect>,
    panels: &mut [BoxyPanel],
    focused: usize,
    tick: u32,
) -> Result<()> {
    let header_text = format!(
        "{} Room + Boxy Integration · Focus with Tab / Ctrl+R to force refresh",
        glyph("light"),
    );
    let footer_text = format!(
        "Focused panel: {} · Tick {} · Esc to exit",
        panels.get(focused).map(|p| p.id).unwrap_or("(none)"),
        tick
    );

    registry.apply_content(&HEADER_ZONE.to_string(), header_text.clone())?;

    let rendered_panels: Vec<String> = panels
        .iter_mut()
        .map(|panel| panel.render().to_string())
        .collect();
    for (panel, rendered) in panels.iter().zip(rendered_panels.iter()) {
        registry.apply_content(&panel.id().to_string(), rendered.clone())?;
    }

    registry.apply_content(&FOOTER_ZONE.to_string(), footer_text.clone())?;

    if let Some(rect) = registry
        .rect_of(&panels[focused].id().to_string())
        .or_else(|| rects.get(panels[focused].id()).cloned())
    {
        renderer.settings_mut().restore_cursor = Some((rect.y + 1, rect.x + 2));
    }

    let dirty = registry.take_dirty();
    if !dirty.is_empty() {
        renderer.render(stdout, &dirty)?;
    }

    stdout.flush()?;
    Ok(())
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(1),
            Constraint::Flex(1),
            Constraint::Fixed(1),
        ],
        children: vec![
            LayoutNode::leaf(HEADER_ZONE),
            LayoutNode {
                id: "app:demo.body".into(),
                direction: Direction::Row,
                constraints: vec![
                    Constraint::Flex(1),
                    Constraint::Flex(1),
                    Constraint::Flex(1),
                ],
                children: vec![
                    LayoutNode::leaf(PANEL_STATUS_ZONE),
                    LayoutNode::leaf(PANEL_METRICS_ZONE),
                    LayoutNode::leaf(PANEL_LOG_ZONE),
                ],
                gap: 1,
                padding: 1,
            },
            LayoutNode::leaf(FOOTER_ZONE),
        ],
        gap: 1,
        padding: 0,
    })
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
            let ideal = content_max_width + config.width.h_padding.saturating_mul(2) + 2; // borders
            ideal.min(terminal_width as usize)
        }
    };

    let min_width = param!("BOXY_MIN_WIDTH", default: "5")
        .parse::<usize>()
        .unwrap_or(5);

    base_width.max(min_width)
}

fn render_box_to_string(config: BoxyConfig) -> String {
    let final_width = calculate_box_width(&config);
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

    let header = Header::new(&config);
    let body = Body::new(&config);
    let status = Status::new(&config);
    let footer = Footer::new(&config);

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

    // Optional multiplex padding
    let mut multiplex_mode = param!("BOXY_MULTIPLEX_MODE", default: "");
    if multiplex_mode.is_empty() {
        multiplex_mode = env::var("BOXY_MULTIPLEX_MODE").unwrap_or_default();
    }
    let multiplex_active = !multiplex_mode.is_empty()
        && multiplex_mode != "0"
        && multiplex_mode.to_lowercase() != "false";

    if multiplex_active {
        if let Some(target_height) = config.fixed_height {
            let current_total = 1 + body_lines.len() + status_lines.len() + 1;
            if target_height > current_total {
                let filler = target_height - current_total;
                let available_width = inner_width.saturating_sub(2 * config.width.h_padding);
                let pad = " ".repeat(config.width.h_padding);
                let blank_line = format!(
                    "{}{}{}{}{}{}{}",
                    &color_code,
                    config.style.vertical,
                    RESET,
                    &pad,
                    " ".repeat(available_width),
                    &pad,
                    format!("{}{}{}", &color_code, config.style.vertical, RESET)
                );
                for _ in 0..filler {
                    body_lines.push(blank_line.clone());
                }
            }
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
