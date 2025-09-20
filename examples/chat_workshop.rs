use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::ExecutableCommand;
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, Clear, ClearType};
use room_mvp::{
    AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, Rect, Result, Size, ZoneRegistry,
    display_width,
};

const HEADER_ZONE: &str = "app:chat.header";
const TIMELINE_ZONE: &str = "app:chat.timeline";
const SIDEBAR_ZONE: &str = "app:chat.sidebar";
const STATUS_ZONE: &str = "app:chat.footer.status";
const INPUT_ZONE: &str = "app:chat.footer.input";
const SUMMON_LINES: &[&str] = &[
    "Checking in on the layout solver status.",
    "Diffed updates look crisp from here!",
    "Registry hashes are keeping the flicker away.",
    "Remember to sync namespaces before piping tokens.",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Chat,
    Preferences,
}

struct PreferenceItem {
    label: &'static str,
    enabled: bool,
}

struct PreferencesState {
    items: Vec<PreferenceItem>,
    selected: usize,
}

impl PreferencesState {
    fn new() -> Self {
        Self {
            items: vec![
                PreferenceItem {
                    label: "Show timestamps",
                    enabled: true,
                },
                PreferenceItem {
                    label: "Enable alerts",
                    enabled: true,
                },
                PreferenceItem {
                    label: "Wrap timeline",
                    enabled: false,
                },
                PreferenceItem {
                    label: "Compact sidebar",
                    enabled: false,
                },
            ],
            selected: 0,
        }
    }

    fn move_up(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + self.items.len() - 1) % self.items.len();
    }

    fn move_down(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.items.len();
    }

    fn toggle_selected(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected) {
            item.enabled = !item.enabled;
        }
    }

    fn summary(&self) -> String {
        let enabled = self.items.iter().filter(|item| item.enabled).count();
        let disabled = self.items.len().saturating_sub(enabled);
        format!("Preferences saved: {} on · {} off", enabled, disabled)
    }
}

fn main() -> Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout
        .execute(terminal::EnterAlternateScreen)?
        .execute(cursor::Hide)?
        .execute(Clear(ClearType::All))?;

    let result = run_app(&mut stdout);

    // Always attempt to restore terminal state even if run_app errors.
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

    let mut messages: Vec<String> = vec![
        "Alice: Welcome to Room MVP!".to_string(),
        "Bob: Token streams drive the layout.".to_string(),
    ];
    let mut scripted_replies: VecDeque<String> = VecDeque::from(vec![
        "Alice: Zones stay rock solid on resize.".to_string(),
        "Bob: Footer input never jumps anymore.".to_string(),
    ]);
    let participants = vec!["Alice", "Bob", "You"];
    let bot_names: Vec<&str> = participants
        .iter()
        .copied()
        .filter(|name| *name != "You")
        .collect();
    let mut next_bot_index = 0usize;
    let mut input_buffer = String::new();
    let mut mode = Mode::Chat;
    let mut help_visible = false;
    let mut status_message: Option<String> = None;
    let mut preferences = PreferencesState::new();

    render_state(
        stdout,
        &mut registry,
        &mut renderer,
        &rects,
        &messages,
        &participants,
        &input_buffer,
        mode,
        help_visible,
        &preferences,
        status_message.as_deref(),
    )?;

    let mut last_script_tick = Instant::now();
    loop {
        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc => match mode {
                        Mode::Chat => break,
                        Mode::Preferences => {
                            mode = Mode::Chat;
                            status_message = Some(preferences.summary());
                            render_state(
                                stdout,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                                mode,
                                help_visible,
                                &preferences,
                                status_message.as_deref(),
                            )?;
                        }
                    },
                    KeyCode::Enter => match mode {
                        Mode::Preferences => {
                            preferences.toggle_selected();
                            status_message = Some(format!(
                                "{} {}",
                                preferences.items[preferences.selected].label,
                                if preferences.items[preferences.selected].enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                }
                            ));
                            render_state(
                                stdout,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                                mode,
                                help_visible,
                                &preferences,
                                status_message.as_deref(),
                            )?;
                        }
                        Mode::Chat => {
                            let trimmed = input_buffer.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            if trimmed.starts_with('/') {
                                let handled = handle_command(
                                    trimmed,
                                    &mut help_visible,
                                    &mut mode,
                                    &mut preferences,
                                    &mut messages,
                                    &bot_names,
                                    &mut next_bot_index,
                                    &mut scripted_replies,
                                    &mut status_message,
                                );
                                if !handled {
                                    status_message = Some(format!("Unknown command: {}", trimmed));
                                }
                            } else {
                                let entry = format!("You: {}", trimmed);
                                messages.push(entry);
                                if let Some(reply) = scripted_replies.pop_front() {
                                    messages.push(reply);
                                }
                            }
                            input_buffer.clear();
                            render_state(
                                stdout,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                                mode,
                                help_visible,
                                &preferences,
                                status_message.as_deref(),
                            )?;
                        }
                    },
                    KeyCode::Backspace => {
                        if mode == Mode::Chat {
                            input_buffer.pop();
                            render_state(
                                stdout,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                                mode,
                                help_visible,
                                &preferences,
                                status_message.as_deref(),
                            )?;
                        }
                    }
                    KeyCode::Up => {
                        if mode == Mode::Preferences {
                            preferences.move_up();
                            render_state(
                                stdout,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                                mode,
                                help_visible,
                                &preferences,
                                status_message.as_deref(),
                            )?;
                        }
                    }
                    KeyCode::Down => {
                        if mode == Mode::Preferences {
                            preferences.move_down();
                            render_state(
                                stdout,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                                mode,
                                help_visible,
                                &preferences,
                                status_message.as_deref(),
                            )?;
                        }
                    }
                    KeyCode::Char(' ') => {
                        if mode == Mode::Preferences {
                            preferences.toggle_selected();
                            status_message = Some(format!(
                                "{} {}",
                                preferences.items[preferences.selected].label,
                                if preferences.items[preferences.selected].enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                }
                            ));
                            render_state(
                                stdout,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                                mode,
                                help_visible,
                                &preferences,
                                status_message.as_deref(),
                            )?;
                        } else if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            input_buffer.push(' ');
                            render_state(
                                stdout,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                                mode,
                                help_visible,
                                &preferences,
                                status_message.as_deref(),
                            )?;
                        }
                    }
                    KeyCode::Char(c) => {
                        if mode == Mode::Preferences {
                            // ignore regular text input while preferences are active
                        } else if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            input_buffer.push(c);
                            render_state(
                                stdout,
                                &mut registry,
                                &mut renderer,
                                &rects,
                                &messages,
                                &participants,
                                &input_buffer,
                                mode,
                                help_visible,
                                &preferences,
                                status_message.as_deref(),
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
                        &mut registry,
                        &mut renderer,
                        &rects,
                        &messages,
                        &participants,
                        &input_buffer,
                        mode,
                        help_visible,
                        &preferences,
                        status_message.as_deref(),
                    )?;
                }
                Event::Mouse(_) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => {
                    // Ignore for now; could surface status hints if desired.
                }
                _ => {}
            }
        }

        if mode == Mode::Chat && last_script_tick.elapsed() >= Duration::from_secs(6) {
            if let Some(reply) = scripted_replies.pop_front() {
                messages.push(reply);
                render_state(
                    stdout,
                    &mut registry,
                    &mut renderer,
                    &rects,
                    &messages,
                    &participants,
                    &input_buffer,
                    mode,
                    help_visible,
                    &preferences,
                    status_message.as_deref(),
                )?;
            }
            last_script_tick = Instant::now();
        }
    }

    Ok(())
}

fn render_state(
    stdout: &mut impl Write,
    registry: &mut ZoneRegistry,
    renderer: &mut AnsiRenderer,
    rects: &std::collections::HashMap<String, Rect>,
    messages: &[String],
    participants: &[&str],
    input_buffer: &str,
    mode: Mode,
    help_visible: bool,
    preferences: &PreferencesState,
    status_message: Option<&str>,
) -> Result<()> {
    let input_rect = registry
        .rect_of(&INPUT_ZONE.to_string())
        .or_else(|| rects.get(INPUT_ZONE).cloned())
        .unwrap_or(Rect::new(0, 0, 20, 1));

    let header_text = format!("Room Layout MVP · {} online", participants.len());
    let timeline_rect = rects.get(TIMELINE_ZONE).cloned();
    let timeline_text = match mode {
        Mode::Chat => {
            let max_messages = (timeline_rect
                .as_ref()
                .map(|r| r.height as usize)
                .unwrap_or(20))
            .max(1);
            messages
                .iter()
                .rev()
                .take(max_messages)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n")
        }
        Mode::Preferences => {
            let mut lines = Vec::new();
            lines.push("Preferences".to_string());
            lines.push("───────────".to_string());
            for (idx, item) in preferences.items.iter().enumerate() {
                let selector = if idx == preferences.selected {
                    '>'
                } else {
                    ' '
                };
                let state = if item.enabled { "[x]" } else { "[ ]" };
                lines.push(format!("{} {} {}", selector, state, item.label));
            }
            lines.join("\n")
        }
    };

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
    let input_display = match mode {
        Mode::Chat => format!(">{}", input_buffer),
        Mode::Preferences => "> Preferences active (Esc to confirm)".to_string(),
    };

    let fallback_cursor = (input_rect.y, input_rect.x.saturating_add(1));
    let caret_position = match mode {
        Mode::Chat => {
            let typed_width = display_width(input_buffer) as u16;
            let caret_base = input_rect.x.saturating_add(1);
            let caret_limit = input_rect
                .x
                .saturating_add(input_rect.width.saturating_sub(1))
                .max(caret_base);
            let caret_x = caret_base.saturating_add(typed_width).min(caret_limit);
            (input_rect.y, caret_x)
        }
        Mode::Preferences => timeline_rect
            .map(|rect| {
                let base_y = rect.y.saturating_add(2);
                let target_y = base_y.saturating_add(preferences.selected as u16);
                let target_x = rect.x.saturating_add(3);
                (target_y, target_x)
            })
            .unwrap_or(fallback_cursor),
    };
    renderer.settings_mut().restore_cursor = Some(caret_position);

    let mut status_lines = vec![underline.clone()];
    if help_visible {
        status_lines.push("/help — toggle this menu".to_string());
        status_lines.push("/summon — prompt a teammate".to_string());
        status_lines.push("/pref — open preferences".to_string());
    } else {
        let default_line = match mode {
            Mode::Chat => "Enter to send · ESC to leave · /help for commands".to_string(),
            Mode::Preferences => {
                "Arrow keys to move · Space/Enter to toggle · Esc to confirm".to_string()
            }
        };
        status_lines.push(default_line);
        status_lines.push(status_message.unwrap_or("").to_string());
        status_lines.push(String::new());
    }
    status_lines.resize(4, String::new());
    let status_text = status_lines.join("\n");

    let updates = [
        (HEADER_ZONE, header_text),
        (TIMELINE_ZONE, timeline_text),
        (SIDEBAR_ZONE, sidebar_text),
        (STATUS_ZONE, status_text),
        (INPUT_ZONE, input_display),
    ];

    for (zone, content) in updates {
        registry.apply_content(&zone.to_string(), content)?;
    }

    let dirty = registry.take_dirty();
    if !dirty.is_empty() {
        renderer.render(stdout, &dirty)?;
    }

    stdout.flush()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_command(
    command: &str,
    help_visible: &mut bool,
    mode: &mut Mode,
    preferences: &mut PreferencesState,
    messages: &mut Vec<String>,
    bot_names: &[&str],
    next_bot_index: &mut usize,
    scripted_replies: &mut VecDeque<String>,
    status_message: &mut Option<String>,
) -> bool {
    let mut parts = command.split_whitespace();
    let keyword = parts.next().unwrap_or("");

    match keyword {
        "/help" => {
            *help_visible = !*help_visible;
            *status_message = Some(if *help_visible {
                "Help menu opened".to_string()
            } else {
                "Help menu hidden".to_string()
            });
            true
        }
        "/summon" => {
            if bot_names.is_empty() {
                *status_message = Some("No teammates available to summon".to_string());
                return true;
            }
            let index = *next_bot_index;
            *next_bot_index = next_bot_index.saturating_add(1);
            let name = bot_names[index % bot_names.len()];
            let quote = SUMMON_LINES[index % SUMMON_LINES.len()];
            let reply = format!("{}: {}", name, quote);
            messages.push(reply.clone());
            // Prevent the scripted ticker from serving the same line immediately again.
            scripted_replies.retain(|queued| queued != &reply);
            *status_message = Some(format!("Summoned {}", name));
            true
        }
        "/pref" | "/prefs" => {
            *mode = Mode::Preferences;
            *help_visible = false;
            if preferences.selected >= preferences.items.len() {
                preferences.selected = preferences.items.len().saturating_sub(1);
            }
            *status_message = Some(
                "Preferences mode: arrow keys to move, space/enter to toggle, Esc to confirm"
                    .to_string(),
            );
            true
        }
        _ => false,
    }
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
