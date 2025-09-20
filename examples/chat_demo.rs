use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use crossterm::ExecutableCommand;
use crossterm::cursor;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, Clear, ClearType};
use room_mvp::{
    AnsiRenderer, Constraint, Direction, EventFlow, LayoutNode, LayoutTree, Rect, Result,
    RoomPlugin, RoomRuntime, RuntimeContext, RuntimeEvent, Size, display_width,
};

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
        .execute(cursor::Hide)?
        .execute(Clear(ClearType::All))?;

    let (width, height) = terminal::size()?;
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut runtime = RoomRuntime::new(layout, renderer, Size::new(width, height))?;
    runtime.register_plugin(ChatPlugin::new());

    let result = runtime.run(&mut stdout);

    stdout.execute(cursor::Show).ok();
    stdout.execute(terminal::LeaveAlternateScreen).ok();
    terminal::disable_raw_mode().ok();

    result
}

struct ChatPlugin {
    participants: Vec<&'static str>,
    messages: Vec<String>,
    scripted_replies: VecDeque<String>,
    input_buffer: String,
    status_message: Option<String>,
    bot_interval: Duration,
    bot_timer: Duration,
}

impl ChatPlugin {
    fn new() -> Self {
        Self {
            participants: vec!["Alice", "Bob", "You"],
            messages: vec![
                "Alice: Welcome to Room MVP!".to_string(),
                "Bob: Token streams drive the layout.".to_string(),
            ],
            scripted_replies: VecDeque::from(vec![
                "Alice: Zones stay rock solid on resize.".to_string(),
                "Bob: Footer input never jumps anymore.".to_string(),
            ]),
            input_buffer: String::new(),
            status_message: None,
            bot_interval: Duration::from_secs(6),
            bot_timer: Duration::default(),
        }
    }

    fn redraw(&mut self, ctx: &mut RuntimeContext<'_>) {
        let header_text = format!("Room Layout MVP · {} online", self.participants.len());
        let timeline_rect = ctx
            .rect(TIMELINE_ZONE)
            .copied()
            .unwrap_or(Rect::new(0, 0, 60, 20));
        let max_messages = usize::max(timeline_rect.height as usize, 1);
        let timeline_text = self
            .messages
            .iter()
            .rev()
            .take(max_messages)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");

        let sidebar_text = self
            .participants
            .iter()
            .map(|name| format!("• {}", name))
            .collect::<Vec<_>>()
            .join("\n");

        let input_rect = ctx.rect(INPUT_ZONE).copied().unwrap_or(Rect::new(
            0,
            timeline_rect.bottom().saturating_sub(1),
            40,
            1,
        ));
        let status_rect = ctx.rect(STATUS_ZONE).copied().unwrap_or(Rect::new(
            input_rect.x,
            input_rect.y.saturating_add(1),
            input_rect.width,
            4,
        ));

        let underline_len = usize::from(status_rect.width.max(1));
        let underline = "─".repeat(underline_len);
        let mut status_lines = vec![underline];
        status_lines.push("Enter to send · ESC to leave".to_string());
        status_lines.push(self.status_message.clone().unwrap_or_default());
        status_lines.push(String::new());
        status_lines.truncate(status_rect.height.max(1) as usize);
        while status_lines.len() < status_rect.height.max(1) as usize {
            status_lines.push(String::new());
        }
        let status_text = status_lines.join("\n");

        let input_display = format!(">{}", self.input_buffer);
        let typed_width = display_width(&self.input_buffer) as u16;
        let caret_base = input_rect.x.saturating_add(1);
        let caret_limit = input_rect
            .x
            .saturating_add(input_rect.width.saturating_sub(1))
            .max(caret_base);
        let caret_x = caret_base.saturating_add(typed_width).min(caret_limit);

        ctx.set_zone(HEADER_ZONE, header_text);
        ctx.set_zone(TIMELINE_ZONE, timeline_text);
        ctx.set_zone(SIDEBAR_ZONE, sidebar_text);
        ctx.set_zone(STATUS_ZONE, status_text);
        ctx.set_zone(INPUT_ZONE, input_display);
        ctx.set_cursor_hint(input_rect.y, caret_x);
    }
}

impl RoomPlugin for ChatPlugin {
    fn name(&self) -> &str {
        "chat"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.redraw(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        match event {
            RuntimeEvent::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Ok(EventFlow::Continue);
                }

                let mut flow = EventFlow::Continue;
                let mut state_changed = false;

                match key.code {
                    KeyCode::Esc => {
                        ctx.request_exit();
                        flow = EventFlow::Consumed;
                    }
                    KeyCode::Enter => {
                        let trimmed = self.input_buffer.trim();
                        if !trimmed.is_empty() {
                            let entry = format!("You: {}", trimmed);
                            self.messages.push(entry);
                            self.input_buffer.clear();
                            if let Some(reply) = self.scripted_replies.pop_front() {
                                self.messages.push(reply);
                            }
                            self.status_message = Some("Message sent".to_string());
                            state_changed = true;
                        }
                    }
                    KeyCode::Backspace => {
                        if self.input_buffer.pop().is_some() {
                            state_changed = true;
                        }
                    }
                    KeyCode::Char(ch) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.input_buffer.push(ch);
                            state_changed = true;
                        }
                    }
                    _ => {}
                }

                if state_changed || matches!(flow, EventFlow::Consumed) {
                    self.redraw(ctx);
                }

                Ok(flow)
            }
            RuntimeEvent::Paste(data) => {
                if !data.is_empty() {
                    self.input_buffer.push_str(data);
                    self.redraw(ctx);
                }
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Resize(_) => {
                self.redraw(ctx);
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Tick { elapsed } => {
                self.bot_timer += *elapsed;
                if self.bot_timer >= self.bot_interval {
                    self.bot_timer = Duration::default();
                    if let Some(reply) = self.scripted_replies.pop_front() {
                        self.messages.push(reply);
                        self.status_message = Some("Teammate replied".to_string());
                        self.redraw(ctx);
                    }
                }
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Mouse(_) | RuntimeEvent::FocusGained | RuntimeEvent::FocusLost => {
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Raw(_) => Ok(EventFlow::Continue),
        }
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
