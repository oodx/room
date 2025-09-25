use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use boxy::{BoxColors, BoxyConfig, WidthConfig, render_to_string};
use crossterm::event::{KeyCode, KeyModifiers};
use room_mvp::runtime::audit::{
    BootstrapAudit, RuntimeAudit, RuntimeAuditEvent, RuntimeAuditStage,
};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, LayoutNode, LayoutTree,
    LegacyScreenStrategy, Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext,
    RuntimeEvent, ScreenDefinition, ScreenManager, Size,
};
use rsb::visual::glyphs::glyph_enable;

const INSTRUCTIONS_ZONE: &str = "app:audit.instructions";
const LIFECYCLE_ZONE: &str = "app:audit.lifecycle";
const LOOP_ZONE: &str = "app:audit.loop";
const COUNTERS_ZONE: &str = "app:audit.counters";
const MAX_ENTRIES: usize = 120;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    glyph_enable();

    let layout = LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![
            Constraint::Fixed(4),
            Constraint::Flex(3),
            Constraint::Flex(2),
        ],
        children: vec![
            LayoutNode::leaf(INSTRUCTIONS_ZONE),
            LayoutNode {
                id: "app:row".into(),
                direction: Direction::Row,
                constraints: vec![Constraint::Flex(1), Constraint::Flex(1)],
                children: vec![
                    LayoutNode::leaf(LIFECYCLE_ZONE),
                    LayoutNode::leaf(LOOP_ZONE),
                ],
                gap: 1,
                padding: 0,
            },
            LayoutNode::leaf(COUNTERS_ZONE),
        ],
        gap: 1,
        padding: 0,
    });
    let screen_layout = layout.clone();

    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    let audit_events = Arc::new(Mutex::new(AuditQueues::default()));
    let audit = BootstrapAudit::new(Arc::new(BufferAudit::new(audit_events.clone())));
    config.audit = Some(audit);
    config.tick_interval = std::time::Duration::from_millis(600);

    let mut runtime = RoomRuntime::with_config(layout, renderer, Size::new(100, 24), config)?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "audit",
        "Runtime Audit",
        Arc::new(move || Box::new(LegacyScreenStrategy::new(screen_layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("audit")?;
    runtime.register_plugin(AuditViewer::new(audit_events));

    CliDriver::new(runtime).run()?;
    Ok(())
}

struct AuditViewer {
    events: Arc<Mutex<AuditQueues>>,
    lifecycle_box: BoxyConfig,
    loop_box: BoxyConfig,
    counters_box: BoxyConfig,
}

impl AuditViewer {
    fn new(events: Arc<Mutex<AuditQueues>>) -> Self {
        let lifecycle_box = BoxyConfig {
            title: Some("Lifecycle Timeline".to_string()),
            status_bar: Some("Warm-up stages · Esc to exit".to_string()),
            colors: BoxColors {
                box_color: "blue".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("white".to_string()),
                status_color: Some("white".to_string()),
                header_color: None,
                footer_color: None,
            },
            width: WidthConfig {
                fixed_width: None,
                ..WidthConfig::default()
            },
            fixed_height: None,
            ..BoxyConfig::default()
        };
        let loop_box = BoxyConfig {
            title: Some("Event Loop Audit".to_string()),
            status_bar: Some("LoopIn / LoopOut / Render / Tick".to_string()),
            colors: BoxColors {
                box_color: "magenta".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("white".to_string()),
                status_color: Some("white".to_string()),
                header_color: None,
                footer_color: None,
            },
            width: WidthConfig {
                fixed_width: None,
                ..WidthConfig::default()
            },
            fixed_height: None,
            ..BoxyConfig::default()
        };
        let counters_box = BoxyConfig {
            title: Some("Stage Counters".to_string()),
            status_bar: Some("Total occurrences by stage".to_string()),
            colors: BoxColors {
                box_color: "green".to_string(),
                text_color: "auto".to_string(),
                title_color: Some("white".to_string()),
                status_color: Some("white".to_string()),
                header_color: None,
                footer_color: None,
            },
            width: WidthConfig {
                fixed_width: None,
                ..WidthConfig::default()
            },
            fixed_height: None,
            ..BoxyConfig::default()
        };
        Self {
            events,
            lifecycle_box,
            loop_box,
            counters_box,
        }
    }

    fn render_lifecycle(&mut self, ctx: &RuntimeContext<'_>) -> String {
        let body = {
            let guard = self.events.lock().unwrap();
            render_lines(&guard.lifecycle, "Waiting for lifecycle events...")
        };
        render_panel(ctx, LIFECYCLE_ZONE, &mut self.lifecycle_box, body)
    }

    fn render_loop(&mut self, ctx: &RuntimeContext<'_>) -> String {
        let body = {
            let guard = self.events.lock().unwrap();
            render_lines(
                &guard.loop_events,
                "Loop idle – waiting for next activity...",
            )
        };
        render_panel(ctx, LOOP_ZONE, &mut self.loop_box, body)
    }

    fn render_counters(&mut self, ctx: &RuntimeContext<'_>) -> String {
        let body = {
            let guard = self.events.lock().unwrap();
            render_counters(&guard.counters)
        };
        render_panel(ctx, COUNTERS_ZONE, &mut self.counters_box, body)
    }
}

impl RoomPlugin for AuditViewer {
    fn name(&self) -> &str {
        "audit_viewer"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(INSTRUCTIONS_ZONE, instructions_text());
        ctx.set_zone_pre_rendered(LIFECYCLE_ZONE, self.render_lifecycle(ctx));
        ctx.set_zone_pre_rendered(LOOP_ZONE, self.render_loop(ctx));
        ctx.set_zone_pre_rendered(COUNTERS_ZONE, self.render_counters(ctx));
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        if let RuntimeEvent::Key(key) = event {
            if key.code == KeyCode::Esc
                || key.code == KeyCode::Char('q')
                || (key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q')))
            {
                ctx.request_exit();
                return Ok(EventFlow::Consumed);
            }
        }
        Ok(EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone_pre_rendered(LIFECYCLE_ZONE, self.render_lifecycle(ctx));
        ctx.set_zone_pre_rendered(LOOP_ZONE, self.render_loop(ctx));
        ctx.set_zone_pre_rendered(COUNTERS_ZONE, self.render_counters(ctx));
        Ok(())
    }
}

#[derive(Clone)]
struct AuditRecord {
    stage: String,
    summary: String,
}

impl std::fmt::Display for AuditRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:<16} {}", self.stage, self.summary)
    }
}

#[derive(Default)]
struct AuditQueues {
    lifecycle: VecDeque<AuditRecord>,
    loop_events: VecDeque<AuditRecord>,
    counters: HashMap<String, u64>,
}

struct BufferAudit {
    events: Arc<Mutex<AuditQueues>>,
}

impl BufferAudit {
    fn new(events: Arc<Mutex<AuditQueues>>) -> Self {
        Self { events }
    }
}

impl RuntimeAudit for BufferAudit {
    fn record(&self, event: RuntimeAuditEvent) {
        if matches!(event.stage, RuntimeAuditStage::RenderSkipped) {
            return;
        }

        let mut summary = event
            .details
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        if summary.is_empty() {
            summary = "(no details)".to_string();
        }

        if let Ok(mut guard) = self.events.lock() {
            let target = match event.stage {
                RuntimeAuditStage::LoopIn
                | RuntimeAuditStage::LoopOut
                | RuntimeAuditStage::EventDispatched
                | RuntimeAuditStage::TickDispatched
                | RuntimeAuditStage::RenderCommitted => &mut guard.loop_events,
                _ => &mut guard.lifecycle,
            };

            let stage_label = format_stage(event.stage);
            target.push_back(AuditRecord {
                stage: stage_label.clone(),
                summary,
            });
            while target.len() > MAX_ENTRIES {
                target.pop_front();
            }
            *guard.counters.entry(stage_label).or_insert(0) += 1;
        }
    }
}

fn format_stage(stage: RuntimeAuditStage) -> String {
    use RuntimeAuditStage as S;
    match stage {
        S::RuntimeConstructed => "constructed".into(),
        S::Open => "open".into(),
        S::Boot => "boot".into(),
        S::Setup => "setup".into(),
        S::UserReady => "user_ready".into(),
        S::LoopIn => "loop_in".into(),
        S::LoopOut => "loop_out".into(),
        S::UserEnd => "user_end".into(),
        S::Cleanup => "cleanup".into(),
        S::End => "end".into(),
        S::Close => "close".into(),
        S::Error => "error".into(),
        S::RecoverOrFatal => "recover_or_fatal".into(),
        S::Fatal => "fatal".into(),
        S::FatalCleanup => "fatal_cleanup".into(),
        S::FatalClose => "fatal_close".into(),
        S::CursorMoved => "cursor_moved".into(),
        S::CursorShown => "cursor_shown".into(),
        S::CursorHidden => "cursor_hidden".into(),
        S::FocusChanged => "focus_changed".into(),
        S::BootstrapStarted => "bootstrap".into(),
        S::PluginRegistered => "plugin_registered".into(),
        S::PluginInitialized => "plugin_init".into(),
        S::EventDispatched => "event".into(),
        S::TickDispatched => "tick".into(),
        S::RenderCommitted => "render".into(),
        S::RuntimeStopped => "stopped".into(),
        other => format!("{other:?}"),
    }
}

fn render_panel(
    ctx: &RuntimeContext<'_>,
    zone: &str,
    boxy: &mut BoxyConfig,
    body: String,
) -> String {
    let rect = ctx
        .rect(zone)
        .copied()
        .unwrap_or(room_mvp::Rect::new(0, 0, 100, 24));
    boxy.width.fixed_width = Some(rect.width.max(40) as usize);
    boxy.fixed_height = Some(rect.height.max(8) as usize);
    boxy.text = body;
    render_to_string(boxy)
}

fn render_lines(records: &VecDeque<AuditRecord>, empty: &str) -> String {
    if records.is_empty() {
        return empty.to_string();
    }
    let mut items = records
        .iter()
        .rev()
        .take(18)
        .cloned()
        .collect::<Vec<AuditRecord>>();
    items.reverse();
    items
        .into_iter()
        .map(|record| record.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_counters(counters: &HashMap<String, u64>) -> String {
    if counters.is_empty() {
        return "No events recorded yet.".to_string();
    }
    let mut entries = counters.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    entries
        .into_iter()
        .take(18)
        .map(|(stage, count)| format!("{stage:<16} {count:>6}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn instructions_text() -> &'static str {
    "Audit Demo\n\
    ----------\n\
    - Lifecycle pane lists non-loop stages in order.\n\
    - Event Loop pane shows LoopIn / LoopOut / Tick / Render.\n\
    - Counts pane aggregates total events.\n\
    - Press Esc or q (Ctrl+C/Ctrl+Q also work) to exit."
}
