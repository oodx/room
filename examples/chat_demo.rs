//! Room Chat Demo
//!
//! This example recreates the original Room MVP chat walkthrough using the
//! latest runtime lifecycle APIs. The footer input is wired into the shared
//! CLI bundle, scripted bot replies arrive on a timer, and status updates show
//! focus changes alongside message hints.
//!
//! ```bash
//! cargo run --example chat_demo
//! ```

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::{
    default_cli_bundle, ensure_input_state, try_input_state, AnsiRenderer, CliDriver,
    CliDriverError, Constraint, DefaultCliBundleConfig, Direction, EventFlow, LayoutError,
    LayoutNode, LayoutTree, LegacyScreenStrategy, Rect, Result, RoomPlugin, RoomRuntime,
    RuntimeConfig, RuntimeContext, RuntimeEvent, ScreenDefinition, ScreenManager, Size,
};

const HEADER_ZONE: &str = "app:chat.header";
const TIMELINE_ZONE: &str = "app:chat.timeline";
const SIDEBAR_ZONE: &str = "app:chat.sidebar";
const STATUS_ZONE: &str = "app:chat.footer.status";
const INPUT_ZONE: &str = "app:chat.footer.input";
const TICK_INTERVAL: Duration = Duration::from_millis(250);
const STATUS_OVERLAY_TIMEOUT: Duration = Duration::from_secs(4);

fn main() -> Result<()> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();

    let mut config = RuntimeConfig::default();
    config.tick_interval = TICK_INTERVAL;
    config.default_focus_zone = Some(INPUT_ZONE.to_string());

    let mut runtime =
        RoomRuntime::with_config(layout.clone(), renderer, Size::new(90, 28), config)?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "chat-demo",
        "Room Chat Demo",
        Arc::new(move || Box::new(LegacyScreenStrategy::new(layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("chat-demo")?;

    let mut bundle_cfg = DefaultCliBundleConfig::default();
    bundle_cfg.input_zone = INPUT_ZONE.to_string();
    bundle_cfg.status_zone = STATUS_ZONE.to_string();
    bundle_cfg.hints_zone = None;
    runtime.register_bundle(default_cli_bundle(bundle_cfg));

    runtime.register_plugin(ChatPlugin::default());

    CliDriver::new(runtime).run().map_err(|err| match err {
        CliDriverError::Runtime(layout_err) => layout_err,
        other => LayoutError::Backend(other.to_string()),
    })
}

struct ChatPlugin {
    participants: Vec<&'static str>,
    messages: Vec<String>,
    scripted_replies: VecDeque<String>,
    last_seen_submission: u64,
    bot_interval: Duration,
    bot_timer: Duration,
    status_overlay: Option<String>,
    overlay_timer: Duration,
}

impl ChatPlugin {
    fn base_status(&self) -> String {
        format!(
            "Ctrl+C exits · Tab cycles focus · {} participants · {} messages",
            self.participants.len(),
            self.messages.len()
        )
    }

    fn new_state() -> Self {
        Self {
            participants: vec!["Alice", "Bob", "You"],
            messages: vec![
                "Alice: Welcome to the Room MVP chat demo!".to_string(),
                "Bob: Layout zones keep the UI steady on resize.".to_string(),
            ],
            scripted_replies: VecDeque::from(vec![
                "Alice: Focus changes now show up in the status footer.".to_string(),
                "Bob: Timers and input sync run off the runtime tick.".to_string(),
            ]),
            last_seen_submission: 0,
            bot_interval: Duration::from_secs(6),
            bot_timer: Duration::default(),
            status_overlay: None,
            overlay_timer: Duration::default(),
        }
    }

    fn ensure_initialized(&mut self) {
        if self.participants.is_empty() {
            *self = Self::new_state();
        }
    }

    fn redraw(&mut self, ctx: &mut RuntimeContext<'_>) {
        let header_text = format!("Room Layout MVP · {} online", self.participants.len());
        let timeline_rect = ctx
            .rect(TIMELINE_ZONE)
            .copied()
            .unwrap_or(Rect::new(0, 0, 70, 18));
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
        self.render_status(ctx);
    }

    fn render_status(&self, ctx: &mut RuntimeContext<'_>) {
        let mut lines = vec![self.base_status()];
        if let Some(ref overlay) = self.status_overlay {
            lines.push(format!("› {}", overlay));
        }
        ctx.set_zone(STATUS_ZONE, lines.join("\n"));
    }

    fn set_overlay(&mut self, message: impl Into<String>) {
        self.status_overlay = Some(message.into());
        self.overlay_timer = Duration::default();
    }

    fn clear_overlay_if_needed(&mut self) -> bool {
        if self.status_overlay.is_some() && self.overlay_timer >= STATUS_OVERLAY_TIMEOUT {
            self.status_overlay = None;
            return true;
        }
        false
    }

    fn process_input(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<bool> {
        if let Some(shared) = try_input_state(ctx) {
            if let Ok(state) = shared.read() {
                if state.submission_count > self.last_seen_submission {
                    self.last_seen_submission = state.submission_count;
                    if let Some(last) = state.last_submission.as_ref() {
                        let trimmed = last.trim();
                        if matches!(trimmed, "/exit" | "/quit") {
                            self.set_overlay("Exit requested");
                            ctx.request_exit();
                            return Ok(true);
                        }
                        if !trimmed.is_empty() {
                            self.messages.push(format!("You: {}", last));
                            self.set_overlay("Message sent");
                            return Ok(true);
                        }
                    }
                }
            }
        }
        Ok(false)
    }
}

impl Default for ChatPlugin {
    fn default() -> Self {
        Self::new_state()
    }
}

impl RoomPlugin for ChatPlugin {
    fn name(&self) -> &str {
        "chat"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.ensure_initialized();
        let _ = ensure_input_state(ctx);
        self.redraw(ctx);
        Ok(())
    }

    fn on_user_ready(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.set_overlay("Ready. Type a message and press Enter.");
        self.render_status(ctx);
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
            RuntimeEvent::FocusChanged(change) => {
                if let Some(target) = change.to.as_ref() {
                    self.set_overlay(format!("Focus moved to {}", target.zone));
                    ctx.request_render();
                }
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Resize(_) => {
                self.redraw(ctx);
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Tick { elapsed } => {
                let mut state_changed = self.process_input(ctx)?;
                self.bot_timer += *elapsed;
                self.overlay_timer += *elapsed;

                if self.bot_timer >= self.bot_interval {
                    self.bot_timer = Duration::default();
                    if let Some(reply) = self.scripted_replies.pop_front() {
                        self.messages.push(reply);
                        self.set_overlay("Bot replied");
                        state_changed = true;
                    }
                }

                if self.clear_overlay_if_needed() {
                    state_changed = true;
                }

                if state_changed {
                    self.redraw(ctx);
                }
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::Key(_) | RuntimeEvent::Mouse(_) | RuntimeEvent::Paste(_) => {
                Ok(EventFlow::Continue)
            }
            RuntimeEvent::FocusGained
            | RuntimeEvent::FocusLost
            | RuntimeEvent::Raw(_)
            | RuntimeEvent::Open
            | RuntimeEvent::Boot
            | RuntimeEvent::Setup
            | RuntimeEvent::UserReady
            | RuntimeEvent::LoopIn { .. }
            | RuntimeEvent::LoopOut { .. }
            | RuntimeEvent::UserEnd
            | RuntimeEvent::Cleanup
            | RuntimeEvent::End
            | RuntimeEvent::Close
            | RuntimeEvent::Error(_)
            | RuntimeEvent::RecoverOrFatal { .. }
            | RuntimeEvent::Fatal
            | RuntimeEvent::FatalCleanup
            | RuntimeEvent::FatalClose
            | RuntimeEvent::CursorMoved(_)
            | RuntimeEvent::CursorShown(_)
            | RuntimeEvent::CursorHidden(_) => Ok(EventFlow::Continue),
        }
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.render_status(ctx);
        Ok(())
    }
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "app:chat.root".into(),
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
