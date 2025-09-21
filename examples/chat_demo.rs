use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, DefaultCliBundleConfig, Direction, EventFlow, LayoutNode,
    LayoutTree, LegacyScreenStrategy, Rect, Result, RoomPlugin, RoomRuntime, RuntimeContext,
    RuntimeEvent, ScreenDefinition, ScreenManager, Size, default_cli_bundle, ensure_input_state,
    try_input_state,
};

const HEADER_ZONE: &str = "app:chat.header";
const TIMELINE_ZONE: &str = "app:chat.timeline";
const SIDEBAR_ZONE: &str = "app:chat.sidebar";
const STATUS_ZONE: &str = "app:chat.footer.status";
const INPUT_ZONE: &str = "app:chat.footer.input";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layout = build_layout();
    let screen_layout = layout.clone();
    let renderer = AnsiRenderer::with_default();
    let mut runtime = RoomRuntime::new(layout, renderer, Size::new(80, 24))?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "chat",
        "Chat Demo",
        Arc::new(move || Box::new(LegacyScreenStrategy::new(screen_layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("chat")?;

    let mut bundle_cfg = DefaultCliBundleConfig::default();
    bundle_cfg.input_zone = INPUT_ZONE.to_string();
    bundle_cfg.status_zone = STATUS_ZONE.to_string();
    bundle_cfg.hints_zone = None;
    runtime.register_bundle(default_cli_bundle(bundle_cfg));
    runtime.register_plugin(ChatPlugin::new());

    CliDriver::new(runtime).run()?;
    Ok(())
}

struct ChatPlugin {
    participants: Vec<&'static str>,
    messages: Vec<String>,
    scripted_replies: VecDeque<String>,
    last_seen_submission: u64,
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
            last_seen_submission: 0,
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
        ctx.set_zone(HEADER_ZONE, header_text);
        ctx.set_zone(TIMELINE_ZONE, timeline_text);
        ctx.set_zone(SIDEBAR_ZONE, sidebar_text);
    }

    fn sync_input(&mut self, ctx: &RuntimeContext<'_>) -> Result<bool> {
        if let Some(shared) = try_input_state(ctx) {
            if let Ok(state) = shared.read() {
                if state.submission_count > self.last_seen_submission {
                    if let Some(last) = state.last_submission.as_ref() {
                        self.messages.push(format!("You: {}", last));
                    }
                    self.last_seen_submission = state.submission_count;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

impl RoomPlugin for ChatPlugin {
    fn name(&self) -> &str {
        "chat"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        let _ = ensure_input_state(ctx);
        self.redraw(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        match event {
            RuntimeEvent::Key(key) if key.kind == KeyEventKind::Press => {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    ctx.request_exit();
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Resize(_) => {
                self.redraw(ctx);
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Tick { elapsed } => {
                let mut state_changed = self.sync_input(ctx)?;
                self.bot_timer += *elapsed;
                if self.bot_timer >= self.bot_interval {
                    self.bot_timer = Duration::default();
                    if let Some(reply) = self.scripted_replies.pop_front() {
                        self.messages.push(reply);
                        state_changed = true;
                    }
                }
                if state_changed {
                    self.redraw(ctx);
                }
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Key(_) => Ok(EventFlow::Continue),
            RuntimeEvent::Mouse(_)
            | RuntimeEvent::FocusGained
            | RuntimeEvent::FocusLost
            | RuntimeEvent::Paste(_)
            | RuntimeEvent::Raw(_) => Ok(EventFlow::Continue),
        }
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        if self.sync_input(ctx)? {
            self.redraw(ctx);
        }
        Ok(())
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
