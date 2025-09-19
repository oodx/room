use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use boxy::width_plugin::get_display_width;
use crossterm::ExecutableCommand;
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, Clear, ClearType};
use room_mvp::{
    AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, Rect, Result, Size, ZoneRegistry,
    ZoneTokenRouter,
};
use rsb::token::format::{escape_token, quote_token};

const HEADER_ZONE: &str = "app:chat.header";
const TIMELINE_ZONE: &str = "app:chat.timeline";
const SIDEBAR_ZONE: &str = "app:chat.sidebar";
const STATUS_ZONE: &str = "app:chat.footer.status";
const INPUT_ZONE: &str = "app:chat.footer.input";

fn main() -> Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout
        .execute(terminal::EnterAlternateScreen)?
        .execute(Clear(ClearType::All))?;

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
    let input_rect = registry
        .rect_of(&INPUT_ZONE.to_string())
        .unwrap_or(Rect::new(0, height.saturating_sub(5), width, 1));

    let mut renderer = AnsiRenderer::with_default();
    renderer.settings_mut().restore_cursor = Some((input_rect.y, input_rect.x + 1));

    let initial_dirty = registry.take_dirty();
    if !initial_dirty.is_empty() {
        renderer.render(stdout, &initial_dirty)?;
    }

    let router = ZoneTokenRouter::new();
    let mut messages: Vec<String> = vec![
        "Alice: Welcome to Room MVP!".to_string(),
        "Bob: Token streams drive the layout.".to_string(),
    ];
    let mut scripted_replies: VecDeque<String> = VecDeque::from(vec![
        "Alice: Zones stay rock solid on resize.".to_string(),
        "Bob: Footer input never jumps anymore.".to_string(),
    ]);
    let participants = vec!["Alice", "Bob", "You"];
    let mut input_buffer = String::new();

    render_state(
        stdout,
        &router,
        &mut registry,
        &mut renderer,
        &rects,
        &messages,
        &participants,
        &input_buffer,
    )?;

    let mut last_script_tick = Instant::now();
    loop {
        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if !input_buffer.trim().is_empty() {
                            let entry = format!("You: {}", input_buffer.trim());
                            messages.push(entry);
                            if let Some(reply) = scripted_replies.pop_front() {
                                messages.push(reply);
                            }
                            input_buffer.clear();
                            render_state(
                                stdout,
                                &router,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                            )?;
                        }
                    }
                    KeyCode::Backspace => {
                        input_buffer.pop();
                        render_state(
                            stdout,
                            &router,
                            &mut registry,
                            &mut renderer,
                            &rects,
                            &messages,
                            &participants,
                            &input_buffer,
                        )?;
                    }
                    KeyCode::Char(c) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            input_buffer.push(c);
                            render_state(
                                stdout,
                                &router,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                            )?;
                        }
                    }
                    _ => {}
                },
                Event::Resize(new_width, new_height) => {
                    rects = layout.solve(Size::new(new_width, new_height))?;
                    registry.sync_layout(&rects);
                    render_state(
                        stdout,
                        &router,
                        &mut registry,
                        &mut renderer,
                        &rects,
                        &messages,
                        &participants,
                        &input_buffer,
                    )?;
                }
                _ => {}
            }
        }

        if last_script_tick.elapsed() >= Duration::from_secs(6) {
            if let Some(reply) = scripted_replies.pop_front() {
                messages.push(reply);
                render_state(
                    stdout,
                    &router,
                    &mut registry,
                    &mut renderer,
                    &rects,
                    &messages,
                    &participants,
                    &input_buffer,
                )?;
            }
            last_script_tick = Instant::now();
        }
    }

    Ok(())
}

fn render_state(
    stdout: &mut impl Write,
    router: &ZoneTokenRouter,
    registry: &mut ZoneRegistry,
    renderer: &mut AnsiRenderer,
    rects: &std::collections::HashMap<String, Rect>,
    messages: &[String],
    participants: &[&str],
    input_buffer: &str,
) -> Result<()> {
    let input_rect = registry
        .rect_of(&INPUT_ZONE.to_string())
        .or_else(|| rects.get(INPUT_ZONE).cloned())
        .unwrap_or(Rect::new(0, 0, 20, 1));

    let header_text = format!("Room Layout MVP · {} online", participants.len());
    let max_messages = (rects
        .get(TIMELINE_ZONE)
        .map(|r| r.height as usize)
        .unwrap_or(20))
    .max(1);
    let timeline_text = messages
        .iter()
        .rev()
        .take(max_messages)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    let sidebar_text = participants
        .iter()
        .map(|name| format!("• {}", name))
        .collect::<Vec<_>>()
        .join("\n");

    let status_rect = registry
        .rect_of(&STATUS_ZONE.to_string())
        .or_else(|| rects.get(STATUS_ZONE).cloned())
        .unwrap_or(Rect::new(
            input_rect.x,
            input_rect.y.saturating_add(1),
            input_rect.width,
            4,
        ));

    let underline = "─".repeat(status_rect.width.max(2) as usize);
    let input_display = format!(">{}", input_buffer);
    let typed_width = get_display_width(input_buffer) as u16;
    let caret_base = input_rect.x.saturating_add(1);
    let caret_limit = input_rect
        .x
        .saturating_add(input_rect.width.saturating_sub(1))
        .max(caret_base);
    let caret_x = caret_base.saturating_add(typed_width).min(caret_limit);
    renderer.settings_mut().restore_cursor = Some((input_rect.y, caret_x));

    let status_body = "Enter to send · ESC to leave";
    let status_lines = vec![
        underline.clone(),
        status_body.to_string(),
        String::new(),
        String::new(),
    ];
    let status_text = status_lines.join("\n");

    let stream = format!(
        "ctx=app; ns=chat.header; content={}; ns=chat.timeline; content={}; ns=chat.sidebar; content={}; ns=chat.footer.status; content={}; ns=chat.footer.input; content={};",
        encode(&header_text),
        encode(&timeline_text),
        encode(&sidebar_text),
        encode(&status_text),
        encode(&input_display),
    );

    for update in router.route(&stream)? {
        registry.apply_content(&update.zone_id, update.content)?;
    }

    let dirty = registry.take_dirty();
    if !dirty.is_empty() {
        renderer.render(stdout, &dirty)?;
    }

    stdout.flush()?;
    Ok(())
}

fn encode(value: &str) -> String {
    quote_token(&escape_token(value))
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(1),
            Constraint::Flex(1),
            Constraint::Fixed(5),
        ],
        children: vec![
            LayoutNode::leaf(HEADER_ZONE),
            LayoutNode {
                id: "app:chat.body".into(),
                direction: Direction::Row,
                constraints: vec![Constraint::Flex(3), Constraint::Fixed(24)],
                children: vec![
                    LayoutNode::leaf(TIMELINE_ZONE),
                    LayoutNode::leaf(SIDEBAR_ZONE),
                ],
                gap: 1,
                padding: 1,
            },
            LayoutNode {
                id: "app:chat.footer".into(),
                direction: Direction::Column,
                constraints: vec![Constraint::Fixed(1), Constraint::Fixed(4)],
                children: vec![LayoutNode::leaf(INPUT_ZONE), LayoutNode::leaf(STATUS_ZONE)],
                gap: 0,
                padding: 0,
            },
        ],
        gap: 1,
        padding: 0,
    })
}
