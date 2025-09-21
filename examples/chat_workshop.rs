use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, LayoutNode, LayoutTree,
    LegacyScreenStrategy, Rect, Result, RoomPlugin, RoomRuntime, RuntimeContext, RuntimeEvent,
    ScreenDefinition, ScreenManager, Size, display_width,
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

const SCRIPT_REPLY_INTERVAL: Duration = Duration::from_secs(6);

fn main() -> Result<()> {
    println!("Room Workshop · Chat Experience\n");
    println!("Commands available: /help, /summon, /pref. Press Esc to exit.\n");

    let layout = build_layout();
    let screen_layout = layout.clone();
    let renderer = AnsiRenderer::with_default();
    let mut runtime = RoomRuntime::new(layout, renderer, Size::new(100, 32))?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "chat-workshop",
        "Chat Workshop",
        Arc::new(move || Box::new(LegacyScreenStrategy::new(screen_layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("chat-workshop")?;

    runtime.config_mut().tick_interval = Duration::from_millis(120);
    runtime.register_plugin(ChatWorkshopPlugin::default());

    CliDriver::new(runtime)
        .run()
        .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))?;
    Ok(())
}

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

struct ChatWorkshopPlugin {
    participants: Vec<&'static str>,
    messages: Vec<String>,
    scripted_replies: VecDeque<String>,
    input_buffer: String,
    mode: Mode,
    help_visible: bool,
    status_message: Option<String>,
    preferences: PreferencesState,
    bot_names: Vec<&'static str>,
    next_bot_index: usize,
    script_timer: Duration,
}

impl Default for ChatWorkshopPlugin {
    fn default() -> Self {
        let participants = vec!["Alice", "Bob", "You"];
        let messages = vec![
            "Alice: Welcome to Room MVP!".to_string(),
            "Bob: Token streams drive the layout.".to_string(),
        ];
        let scripted_replies = VecDeque::from(vec![
            "Alice: Zones stay rock solid on resize.".to_string(),
            "Bob: Footer input never jumps anymore.".to_string(),
        ]);
        let bot_names = participants
            .iter()
            .copied()
            .filter(|name| *name != "You")
            .collect();

        Self {
            participants,
            messages,
            scripted_replies,
            input_buffer: String::new(),
            mode: Mode::Chat,
            help_visible: false,
            status_message: None,
            preferences: PreferencesState::new(),
            bot_names,
            next_bot_index: 0,
            script_timer: Duration::default(),
        }
    }
}

impl ChatWorkshopPlugin {
    fn render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        let header_text = format!("Room Layout MVP · {} online", self.participants.len());
        let input_rect = ctx
            .rect(INPUT_ZONE)
            .copied()
            .unwrap_or(Rect::new(0, 0, 20, 1));
        let timeline_rect = ctx.rect(TIMELINE_ZONE).copied();

        let timeline_text = match self.mode {
            Mode::Chat => {
                let max_messages = timeline_rect
                    .as_ref()
                    .map(|rect| rect.height as usize)
                    .unwrap_or(20)
                    .max(1);
                self.messages
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
                for (idx, item) in self.preferences.items.iter().enumerate() {
                    let selector = if idx == self.preferences.selected {
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

        let sidebar_text = self
            .participants
            .iter()
            .map(|name| format!("• {}", name))
            .collect::<Vec<_>>()
            .join("\n");

        let status_rect = ctx
            .rect(STATUS_ZONE)
            .copied()
            .or_else(|| ctx.rect(INPUT_ZONE).copied())
            .unwrap_or(Rect::new(
                input_rect.x,
                input_rect.y.saturating_add(1),
                input_rect.width,
                4,
            ));

        let underline = "─".repeat(status_rect.width.max(2) as usize);
        let input_display = match self.mode {
            Mode::Chat => format!(">{}", self.input_buffer),
            Mode::Preferences => "> Preferences active (Esc to confirm)".to_string(),
        };

        let fallback_cursor = (input_rect.y, input_rect.x.saturating_add(1));
        let caret_position = match self.mode {
            Mode::Chat => {
                let typed_width = display_width(&self.input_buffer) as u16;
                let base_x = input_rect.x.saturating_add(1);
                let limit = input_rect
                    .x
                    .saturating_add(input_rect.width.saturating_sub(1))
                    .max(base_x);
                let caret_x = base_x.saturating_add(typed_width).min(limit);
                (input_rect.y, caret_x)
            }
            Mode::Preferences => timeline_rect
                .map(|rect| {
                    let base_y = rect.y.saturating_add(2);
                    let target_y = base_y.saturating_add(self.preferences.selected as u16);
                    let target_x = rect.x.saturating_add(3);
                    (target_y, target_x)
                })
                .unwrap_or(fallback_cursor),
        };

        let mut status_lines = vec![underline.clone()];
        if self.help_visible {
            status_lines.push("/help — toggle this menu".to_string());
            status_lines.push("/summon — prompt a teammate".to_string());
            status_lines.push("/pref — open preferences".to_string());
        } else {
            let default_line = match self.mode {
                Mode::Chat => "Enter to send · ESC to leave · /help for commands".to_string(),
                Mode::Preferences => {
                    "Arrow keys to move · Space/Enter to toggle · Esc to confirm".to_string()
                }
            };
            status_lines.push(default_line);
            status_lines.push(self.status_message.as_deref().unwrap_or("").to_string());
            status_lines.push(String::new());
        }
        status_lines.resize(4, String::new());
        let status_text = status_lines.join("\n");

        ctx.set_zone(HEADER_ZONE, header_text);
        ctx.set_zone(TIMELINE_ZONE, timeline_text);
        ctx.set_zone(SIDEBAR_ZONE, sidebar_text);
        ctx.set_zone(STATUS_ZONE, status_text);
        ctx.set_zone(INPUT_ZONE, input_display);
        ctx.set_cursor_hint(caret_position.0, caret_position.1);
        Ok(())
    }

    fn handle_command(&mut self, command: &str) -> bool {
        let mut parts = command.split_whitespace();
        let keyword = parts.next().unwrap_or("");

        match keyword {
            "/help" => {
                self.help_visible = !self.help_visible;
                self.status_message = Some(if self.help_visible {
                    "Help menu opened".to_string()
                } else {
                    "Help menu hidden".to_string()
                });
                true
            }
            "/summon" => {
                if self.bot_names.is_empty() {
                    self.status_message = Some("No teammates available to summon".to_string());
                    return true;
                }
                let index = self.next_bot_index;
                self.next_bot_index = self.next_bot_index.saturating_add(1);
                let name = self.bot_names[index % self.bot_names.len()];
                let quote = SUMMON_LINES[index % SUMMON_LINES.len()];
                let reply = format!("{}: {}", name, quote);
                self.messages.push(reply.clone());
                self.scripted_replies.retain(|queued| queued != &reply);
                self.status_message = Some(format!("Summoned {}", name));
                true
            }
            "/pref" | "/prefs" => {
                self.mode = Mode::Preferences;
                self.help_visible = false;
                if self.preferences.selected >= self.preferences.items.len() {
                    self.preferences.selected = self.preferences.items.len().saturating_sub(1);
                }
                self.status_message = Some(
                    "Preferences mode: arrow keys to move, space/enter to toggle, Esc to confirm"
                        .to_string(),
                );
                true
            }
            _ => false,
        }
    }
}

impl RoomPlugin for ChatWorkshopPlugin {
    fn name(&self) -> &str {
        "chat_workshop"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.script_timer = Duration::default();
        self.render(ctx)
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        let mut render_needed = false;

        match event {
            RuntimeEvent::Tick { elapsed } => {
                self.script_timer += *elapsed;
                if self.mode == Mode::Chat && self.script_timer >= SCRIPT_REPLY_INTERVAL {
                    self.script_timer = Duration::default();
                    if let Some(reply) = self.scripted_replies.pop_front() {
                        self.messages.push(reply);
                        render_needed = true;
                    }
                }
            }
            RuntimeEvent::Resize(_) => {
                render_needed = true;
            }
            RuntimeEvent::Paste(data) => {
                if self.mode == Mode::Chat && !data.is_empty() {
                    self.input_buffer.push_str(data);
                    render_needed = true;
                }
            }
            RuntimeEvent::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Ok(EventFlow::Continue);
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    ctx.request_exit();
                    return Ok(EventFlow::Consumed);
                }
                match key.code {
                    KeyCode::Esc => match self.mode {
                        Mode::Chat => {
                            ctx.request_exit();
                            return Ok(EventFlow::Consumed);
                        }
                        Mode::Preferences => {
                            self.mode = Mode::Chat;
                            self.status_message = Some(self.preferences.summary());
                            render_needed = true;
                        }
                    },
                    KeyCode::Enter => match self.mode {
                        Mode::Preferences => {
                            self.preferences.toggle_selected();
                            self.status_message = Some(format!(
                                "{} {}",
                                self.preferences.items[self.preferences.selected].label,
                                if self.preferences.items[self.preferences.selected].enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                }
                            ));
                            render_needed = true;
                        }
                        Mode::Chat => {
                            let trimmed = self.input_buffer.trim();
                            if !trimmed.is_empty() {
                                let owned = trimmed.to_string();
                                if owned.starts_with('/') {
                                    if !self.handle_command(&owned) {
                                        self.status_message =
                                            Some(format!("Unknown command: {}", owned));
                                    }
                                } else {
                                    let entry = format!("You: {}", owned);
                                    self.messages.push(entry);
                                    if let Some(reply) = self.scripted_replies.pop_front() {
                                        self.messages.push(reply);
                                    }
                                    self.status_message = None;
                                }
                                self.input_buffer.clear();
                                render_needed = true;
                            }
                        }
                    },
                    KeyCode::Backspace => {
                        if self.mode == Mode::Chat {
                            self.input_buffer.pop();
                            render_needed = true;
                        }
                    }
                    KeyCode::Up => {
                        if self.mode == Mode::Preferences {
                            self.preferences.move_up();
                            render_needed = true;
                        }
                    }
                    KeyCode::Down => {
                        if self.mode == Mode::Preferences {
                            self.preferences.move_down();
                            render_needed = true;
                        }
                    }
                    KeyCode::Char(' ') => {
                        if self.mode == Mode::Preferences {
                            self.preferences.toggle_selected();
                            self.status_message = Some(format!(
                                "{} {}",
                                self.preferences.items[self.preferences.selected].label,
                                if self.preferences.items[self.preferences.selected].enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                }
                            ));
                            render_needed = true;
                        } else if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.input_buffer.push(' ');
                            render_needed = true;
                        }
                    }
                    KeyCode::Char(c) => {
                        if self.mode == Mode::Chat && !key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            self.input_buffer.push(c);
                            render_needed = true;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if render_needed {
            self.render(ctx)?;
        }

        Ok(EventFlow::Continue)
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
