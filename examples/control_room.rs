use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, FocusController, LayoutError,
    LayoutNode, LayoutTree, LegacyScreenStrategy, PluginBundle, Rect, Result, RoomPlugin,
    RoomRuntime, RuntimeContext, RuntimeEvent, ScreenDefinition, ScreenManager, SharedStateError,
    Size, display_width, ensure_focus_registry,
};

const HEADER_ZONE: &str = "app:control.header";
const TIMELINE_ZONE: &str = "app:control.timeline";
const AGENTS_ZONE: &str = "app:control.agents";
const LOG_ZONE: &str = "app:control.log";
const INPUT_ZONE: &str = "app:control.input";
const HINTS_ZONE: &str = "app:control.hints";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layout = build_layout();
    let screen_layout = layout.clone();
    let renderer = AnsiRenderer::with_default();
    let mut runtime = RoomRuntime::new(layout, renderer, Size::new(100, 32))?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "control-room",
        "Control Room",
        Arc::new(move || Box::new(LegacyScreenStrategy::new(screen_layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("control-room")?;

    PluginBundle::new()
        .with_plugin(HeaderPlugin::default(), 5)
        .with_plugin(TimelinePlugin::default(), 10)
        .with_plugin(AgentPanelPlugin::default(), 20)
        .with_plugin(LogPanelPlugin::default(), 30)
        .with_plugin(CommandPlugin::default(), -10)
        .register_into(&mut runtime);

    CliDriver::new(runtime).run()?;
    Ok(())
}

#[derive(Default)]
struct DashboardState {
    version: u64,
    messages: Vec<ChatMessage>,
    agents: Vec<AgentInfo>,
    log: Vec<String>,
    pending: VecDeque<PendingReply>,
}

#[derive(Clone)]
struct ChatMessage {
    speaker: String,
    content: String,
}

#[derive(Clone)]
struct AgentInfo {
    name: String,
    status: AgentStatus,
    tasks_completed: u32,
    expertise: String,
}

#[derive(Clone)]
enum AgentStatus {
    Idle,
    Routing,
    Responding,
}

struct PendingReply {
    agent_index: usize,
    remaining: Duration,
    content: String,
}

impl DashboardState {
    fn bootstrap() -> Self {
        Self {
            version: 1,
            messages: vec![
                ChatMessage {
                    speaker: "Ada".into(),
                    content: "Planner online. Ready to route work.".into(),
                },
                ChatMessage {
                    speaker: "Mux".into(),
                    content: "Log tail streaming. Watching metrics.".into(),
                },
                ChatMessage {
                    speaker: "Scribe".into(),
                    content: "Capturing session notes.".into(),
                },
            ],
            agents: vec![
                AgentInfo {
                    name: "Ada".into(),
                    status: AgentStatus::Idle,
                    tasks_completed: 3,
                    expertise: "Planning".into(),
                },
                AgentInfo {
                    name: "Mux".into(),
                    status: AgentStatus::Idle,
                    tasks_completed: 5,
                    expertise: "Retrieval".into(),
                },
                AgentInfo {
                    name: "Scribe".into(),
                    status: AgentStatus::Idle,
                    tasks_completed: 7,
                    expertise: "Summaries".into(),
                },
            ],
            log: vec![
                "runtime booted".into(),
                "agents synchronized".into(),
                "bundle registered".into(),
            ],
            pending: VecDeque::new(),
        }
    }

    fn push_message(&mut self, speaker: impl Into<String>, content: impl Into<String>) {
        self.messages.push(ChatMessage {
            speaker: speaker.into(),
            content: content.into(),
        });
        self.bump();
    }

    fn log(&mut self, entry: impl Into<String>) {
        self.log.push(entry.into());
        if self.log.len() > 120 {
            self.log.drain(0..(self.log.len() - 120));
        }
        self.bump();
    }

    fn mark_agent(&mut self, index: usize, status: AgentStatus) {
        if let Some(agent) = self.agents.get_mut(index) {
            agent.status = status;
            if matches!(agent.status, AgentStatus::Responding) {
                agent.tasks_completed += 1;
            }
            self.bump();
        }
    }

    fn bump(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}

fn dashboard_state(
    ctx: &RuntimeContext<'_>,
) -> std::result::Result<Arc<RwLock<DashboardState>>, SharedStateError> {
    ctx.shared_init::<RwLock<DashboardState>, _>(|| RwLock::new(DashboardState::bootstrap()))
}

fn map_shared_err(err: SharedStateError) -> LayoutError {
    LayoutError::Backend(format!("shared state: {err}"))
}

enum RenderTarget {
    Timeline,
    Agents,
    Log,
    Header,
}

fn render_if_stale(
    ctx: &mut RuntimeContext<'_>,
    last_version: &mut u64,
    last_focus: &mut Option<String>,
    target: RenderTarget,
) -> Result<()> {
    let state = dashboard_state(ctx).map_err(map_shared_err)?;
    let snapshot = state
        .read()
        .map_err(|_| map_shared_err(SharedStateError::Poisoned))?;
    let focus = current_focus_zone(ctx);
    let focus_changed = last_focus
        .as_ref()
        .map(|stored| stored.as_str())
        .ne(&focus.as_deref());

    if *last_version == snapshot.version && !focus_changed {
        return Ok(());
    }
    *last_version = snapshot.version;
    *last_focus = focus.clone();

    let outcome = match target {
        RenderTarget::Timeline => render_timeline(ctx, &snapshot, focus.as_deref()),
        RenderTarget::Agents => render_agents(ctx, &snapshot, focus.as_deref()),
        RenderTarget::Log => render_log(ctx, &snapshot, focus.as_deref()),
        RenderTarget::Header => render_header(ctx, &snapshot, focus.as_deref()),
    };

    if outcome.is_ok() {
        ctx.request_render();
    }

    outcome
}

fn render_timeline(
    ctx: &mut RuntimeContext<'_>,
    snapshot: &DashboardState,
    focus: Option<&str>,
) -> Result<()> {
    let maybe_rect = ctx.rect(TIMELINE_ZONE).copied();
    let max_rows = maybe_rect.map(|r| r.height as usize).unwrap_or(12);
    let mut lines = Vec::new();
    let header = if matches!(focus, Some(TIMELINE_ZONE)) {
        "TIMELINE *"
    } else {
        "Timeline"
    };
    lines.push(header.to_string());

    for message in snapshot
        .messages
        .iter()
        .rev()
        .take(max_rows.saturating_sub(1))
        .rev()
    {
        lines.push(format!("{:>8} | {}", message.speaker, message.content));
    }

    ctx.set_zone(TIMELINE_ZONE, lines.join("\n"));
    Ok(())
}

fn render_agents(
    ctx: &mut RuntimeContext<'_>,
    snapshot: &DashboardState,
    focus: Option<&str>,
) -> Result<()> {
    let mut lines = Vec::new();
    let header = if matches!(focus, Some(AGENTS_ZONE)) {
        "AGENTS *"
    } else {
        "Agents"
    };
    lines.push(header.to_string());

    for agent in &snapshot.agents {
        let status = match agent.status {
            AgentStatus::Idle => "idle",
            AgentStatus::Routing => "routing",
            AgentStatus::Responding => "responding",
        };
        lines.push(format!(
            "{:<8} [{}] tasks:{} focus:{}",
            agent.name, status, agent.tasks_completed, agent.expertise
        ));
    }

    ctx.set_zone(AGENTS_ZONE, lines.join("\n"));
    Ok(())
}

fn render_log(
    ctx: &mut RuntimeContext<'_>,
    snapshot: &DashboardState,
    focus: Option<&str>,
) -> Result<()> {
    let maybe_rect = ctx.rect(LOG_ZONE).copied();
    let rows = maybe_rect.map(|r| r.height as usize).unwrap_or(8);
    let mut lines = Vec::new();
    let header = if matches!(focus, Some(LOG_ZONE)) {
        "LOG *"
    } else {
        "Log"
    };
    lines.push(header.to_string());

    for entry in snapshot.log.iter().rev().take(rows.saturating_sub(1)).rev() {
        lines.push(entry.clone());
    }

    ctx.set_zone(LOG_ZONE, lines.join("\n"));
    Ok(())
}

fn render_header(
    ctx: &mut RuntimeContext<'_>,
    snapshot: &DashboardState,
    focus: Option<&str>,
) -> Result<()> {
    let focus_label = focus
        .map(|zone| match zone {
            TIMELINE_ZONE => "timeline",
            AGENTS_ZONE => "agents",
            LOG_ZONE => "log",
            INPUT_ZONE => "input",
            HINTS_ZONE => "hints",
            _ => zone,
        })
        .unwrap_or("none");

    let agent_summary: Vec<String> = snapshot
        .agents
        .iter()
        .map(|agent| format!("{}:{}", agent.name, agent.tasks_completed))
        .collect();

    let line = format!(
        "Room Control · focus:{} · counters:{}",
        focus_label,
        agent_summary.join(",")
    );

    ctx.set_zone(HEADER_ZONE, line);
    Ok(())
}

fn current_focus_zone(ctx: &RuntimeContext<'_>) -> Option<String> {
    ensure_focus_registry(ctx)
        .ok()
        .and_then(|registry| registry.current())
        .map(|entry| entry.zone_id)
}

#[derive(Default)]
struct HeaderPlugin {
    last_version: u64,
    last_focus: Option<String>,
}

impl RoomPlugin for HeaderPlugin {
    fn name(&self) -> &str {
        "header"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Header,
        )
    }

    fn on_event(&mut self, ctx: &mut RuntimeContext<'_>, _: &RuntimeEvent) -> Result<EventFlow> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Header,
        )?;
        Ok(EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Header,
        )
    }
}

#[derive(Default)]
struct TimelinePlugin {
    last_version: u64,
    last_focus: Option<String>,
}

impl RoomPlugin for TimelinePlugin {
    fn name(&self) -> &str {
        "timeline"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Timeline,
        )
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Timeline,
        )?;
        Ok(EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Timeline,
        )
    }
}

#[derive(Default)]
struct AgentPanelPlugin {
    last_version: u64,
    last_focus: Option<String>,
}

impl RoomPlugin for AgentPanelPlugin {
    fn name(&self) -> &str {
        "agents"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Agents,
        )
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Agents,
        )?;
        Ok(EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Agents,
        )
    }
}

#[derive(Default)]
struct LogPanelPlugin {
    last_version: u64,
    last_focus: Option<String>,
}

impl RoomPlugin for LogPanelPlugin {
    fn name(&self) -> &str {
        "log"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Log,
        )
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Log,
        )?;
        Ok(EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        render_if_stale(
            ctx,
            &mut self.last_version,
            &mut self.last_focus,
            RenderTarget::Log,
        )
    }
}

#[derive(Default)]
struct CommandPlugin {
    input: String,
    focus: Option<FocusController>,
    zones: Vec<&'static str>,
    current_zone: usize,
    reply_scripts: VecDeque<String>,
}

impl CommandPlugin {
    fn ensure_focus(
        &mut self,
        ctx: &RuntimeContext<'_>,
    ) -> std::result::Result<&mut FocusController, SharedStateError> {
        if self.focus.is_none() {
            let controller = FocusController::new("command", ensure_focus_registry(ctx)?);
            self.focus = Some(controller);
        }
        Ok(self.focus.as_mut().unwrap())
    }

    fn render_input(&self, ctx: &mut RuntimeContext<'_>) {
        let cursor_base = ctx
            .rect(INPUT_ZONE)
            .copied()
            .unwrap_or(Rect::new(0, 0, 60, 1));
        let caret_glyph = '█';
        let caret_x = cursor_base.x + display_width(&self.input) as u16 + 2;
        let mut display = String::from("> ");
        display.push_str(&self.input);
        display.push(caret_glyph);
        ctx.set_zone(INPUT_ZONE, display);
        ctx.set_cursor_hint(
            cursor_base.y,
            caret_x.min(cursor_base.x + cursor_base.width),
        );

        let hints = "Enter to dispatch · Tab cycles focus · Esc exits";
        ctx.set_zone(HINTS_ZONE, hints);
    }

    fn cycle_focus(&mut self, forward: bool, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        if self.zones.is_empty() {
            self.zones = vec![INPUT_ZONE, TIMELINE_ZONE, AGENTS_ZONE, LOG_ZONE];
        }
        if self.zones.is_empty() {
            return Ok(());
        }
        if forward {
            self.current_zone = (self.current_zone + 1) % self.zones.len();
        } else {
            if self.current_zone == 0 {
                self.current_zone = self.zones.len() - 1;
            } else {
                self.current_zone -= 1;
            }
        }
        let zone_id = self.zones[self.current_zone];
        if let Ok(controller) = self.ensure_focus(ctx) {
            controller.focus(zone_id);
        }
        ctx.request_render();
        Ok(())
    }

    fn handle_submit(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        if self.input.trim().is_empty() {
            return Ok(());
        }
        let text = self.input.trim().to_string();
        self.input.clear();
        self.current_zone = 0;
        if let Ok(controller) = self.ensure_focus(ctx) {
            controller.focus(INPUT_ZONE);
        }
        ctx.request_render();

        let shared = dashboard_state(ctx).map_err(map_shared_err)?;
        {
            let mut state = shared
                .write()
                .map_err(|_| map_shared_err(SharedStateError::Poisoned))?;
            state.push_message("You", text.clone());
            state.log(format!("user dispatched: {}", text));

            let agent_index = (state.version as usize) % state.agents.len();
            state.mark_agent(agent_index, AgentStatus::Routing);
            let script = self
                .reply_scripts
                .pop_front()
                .unwrap_or_else(|| "Agent ack: queued task.".to_string());
            state.pending.push_back(PendingReply {
                agent_index,
                remaining: Duration::from_millis(900),
                content: script.clone(),
            });
            if !script.is_empty() {
                self.reply_scripts.push_back(script);
            }
        }

        self.render_input(ctx);
        Ok(())
    }

    fn process_pending(&mut self, ctx: &RuntimeContext<'_>, elapsed: Duration) -> Result<()> {
        let shared = dashboard_state(ctx).map_err(map_shared_err)?;
        let mut state = shared
            .write()
            .map_err(|_| map_shared_err(SharedStateError::Poisoned))?;
        for reply in state.pending.iter_mut() {
            if reply.remaining > elapsed {
                reply.remaining -= elapsed;
            } else {
                reply.remaining = Duration::from_millis(0);
            }
        }

        let mut delivered = Vec::new();
        while let Some(front) = state.pending.front() {
            if front.remaining > Duration::from_millis(0) {
                break;
            }
            let reply = state.pending.pop_front().unwrap();
            delivered.push(reply);
        }

        for reply in delivered {
            state.mark_agent(reply.agent_index, AgentStatus::Responding);
            let agent_name = state
                .agents
                .get(reply.agent_index)
                .map(|agent| agent.name.clone())
                .unwrap_or_else(|| "Agent".to_string());
            state.push_message(agent_name.clone(), reply.content);
            state.log(format!("{} responded", agent_name));
            state.mark_agent(reply.agent_index, AgentStatus::Idle);
        }
        Ok(())
    }
}

impl RoomPlugin for CommandPlugin {
    fn name(&self) -> &str {
        "command"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.zones = vec![INPUT_ZONE, TIMELINE_ZONE, AGENTS_ZONE, LOG_ZONE];
        self.reply_scripts = VecDeque::from(vec![
            "Ada: Route prepared. Expect analysis soon.".to_string(),
            "Mux: Pulled fresh telemetry for the request.".to_string(),
            "Scribe: Drafted a summary playbook.".to_string(),
        ]);
        self.current_zone = 0;
        if let Ok(controller) = self.ensure_focus(ctx) {
            controller.focus(INPUT_ZONE);
        }
        self.render_input(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        match event {
            RuntimeEvent::Key(key) if key.kind == KeyEventKind::Press => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('d') {
                    ctx.request_exit();
                    return Ok(EventFlow::Consumed);
                }

                match key.code {
                    KeyCode::Esc => {
                        ctx.request_exit();
                        return Ok(EventFlow::Consumed);
                    }
                    KeyCode::Tab => {
                        let forward = !key.modifiers.contains(KeyModifiers::SHIFT);
                        self.cycle_focus(forward, ctx)?;
                        return Ok(EventFlow::Consumed);
                    }
                    KeyCode::Enter => {
                        if matches!(self.ensure_focus(ctx).ok().and_then(|c| c.last_zone()), Some(zone) if zone == INPUT_ZONE)
                        {
                            self.handle_submit(ctx)?;
                            return Ok(EventFlow::Consumed);
                        }
                    }
                    KeyCode::Backspace => {
                        if matches!(self.ensure_focus(ctx).ok().and_then(|c| c.last_zone()), Some(zone) if zone == INPUT_ZONE)
                        {
                            self.input.pop();
                            self.render_input(ctx);
                            return Ok(EventFlow::Consumed);
                        }
                    }
                    KeyCode::Char(ch) => {
                        if matches!(self.ensure_focus(ctx).ok().and_then(|c| c.last_zone()), Some(zone) if zone == INPUT_ZONE)
                        {
                            let mut buf = [0u8; 4];
                            self.input.push_str(ch.encode_utf8(&mut buf));
                            self.render_input(ctx);
                            return Ok(EventFlow::Consumed);
                        }
                    }
                    _ => {}
                }
            }
            RuntimeEvent::Tick { elapsed } => {
                self.process_pending(ctx, *elapsed)?;
            }
            _ => {}
        }

        Ok(EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.render_input(ctx);
        Ok(())
    }
}

fn build_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "app:control.root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(1),
            Constraint::Flex(1),
            Constraint::Fixed(3),
        ],
        children: vec![
            LayoutNode::leaf(HEADER_ZONE),
            LayoutNode {
                id: "app:control.body".into(),
                direction: Direction::Row,
                constraints: vec![Constraint::Flex(3), Constraint::Fixed(28)],
                children: vec![
                    LayoutNode::leaf(TIMELINE_ZONE),
                    LayoutNode {
                        id: "app:control.sidebar".into(),
                        direction: Direction::Column,
                        constraints: vec![Constraint::Flex(1), Constraint::Fixed(8)],
                        children: vec![LayoutNode::leaf(AGENTS_ZONE), LayoutNode::leaf(LOG_ZONE)],
                        gap: 1,
                        padding: 0,
                    },
                ],
                gap: 1,
                padding: 1,
            },
            LayoutNode {
                id: "app:control.footer".into(),
                direction: Direction::Column,
                constraints: vec![Constraint::Fixed(1), Constraint::Fixed(2)],
                children: vec![LayoutNode::leaf(INPUT_ZONE), LayoutNode::leaf(HINTS_ZONE)],
                gap: 0,
                padding: 0,
            },
        ],
        gap: 1,
        padding: 0,
    })
}
