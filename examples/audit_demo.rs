use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use boxy::{BoxColors, BoxyConfig, WidthConfig, render_to_string};
use crossterm::event::KeyCode;
use room_mvp::runtime::audit::{
    BootstrapAudit, RuntimeAudit, RuntimeAuditEvent, RuntimeAuditStage,
};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, LayoutNode, LayoutTree,
    LegacyScreenStrategy, Result, RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext,
    RuntimeEvent, ScreenDefinition, ScreenManager, Size,
};
use rsb::visual::glyphs::glyph_enable;

const AUDIT_ZONE: &str = "app:audit.log";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    glyph_enable();

    let layout = LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1)],
        children: vec![LayoutNode::leaf(AUDIT_ZONE)],
        gap: 0,
        padding: 0,
    });
    let screen_layout = layout.clone();

    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    let audit_events = Arc::new(Mutex::new(VecDeque::<AuditRecord>::new()));
    let audit = BootstrapAudit::new(Arc::new(BufferAudit::new(audit_events.clone())));
    config.audit = Some(audit);

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
    runtime.config_mut().tick_interval = std::time::Duration::from_millis(50);

    CliDriver::new(runtime).run()?;
    Ok(())
}

struct AuditViewer {
    events: Arc<Mutex<VecDeque<AuditRecord>>>,
    boxy: BoxyConfig,
    stream_started: bool,
}

impl AuditViewer {
    fn new(events: Arc<Mutex<VecDeque<AuditRecord>>>) -> Self {
        let boxy = BoxyConfig {
            title: Some("Runtime Audit Trail".to_string()),
            status_bar: Some("Esc to exit · Audit feed updates live".to_string()),
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
        Self {
            events,
            boxy,
            stream_started: false,
        }
    }

    fn render(&mut self, ctx: &RuntimeContext<'_>) -> String {
        let rect = ctx
            .rect(AUDIT_ZONE)
            .copied()
            .unwrap_or(room_mvp::Rect::new(0, 0, 100, 24));
        self.boxy.width.fixed_width = Some(rect.width.max(40) as usize);
        self.boxy.fixed_height = Some(rect.height.max(10) as usize);
        self.boxy.text = self.render_body();
        render_to_string(&self.boxy)
    }

    fn render_placeholder(&mut self, ctx: &RuntimeContext<'_>) -> String {
        let rect = ctx
            .rect(AUDIT_ZONE)
            .copied()
            .unwrap_or(room_mvp::Rect::new(0, 0, 100, 24));
        self.boxy.width.fixed_width = Some(rect.width.max(40) as usize);
        self.boxy.fixed_height = Some(rect.height.max(10) as usize);
        self.boxy.text = "Initializing audit stream...".to_string();
        render_to_string(&self.boxy)
    }

    fn render_body(&self) -> String {
        let guard = self.events.lock().unwrap();
        if guard.is_empty() {
            return "Waiting for audit events...".to_string();
        }
        let mut items = guard.iter().rev().take(18).cloned().collect::<Vec<_>>();
        items.reverse();
        items
            .into_iter()
            .map(|record| record.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl RoomPlugin for AuditViewer {
    fn name(&self) -> &str {
        "audit_viewer"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone_pre_rendered(AUDIT_ZONE, self.render(ctx));
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        if matches!(event, RuntimeEvent::Key(key) if key.code == KeyCode::Esc) {
            ctx.request_exit();
            return Ok(EventFlow::Consumed);
        }
        Ok(EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        if !self.stream_started {
            self.stream_started = true;
            ctx.set_zone_pre_rendered(AUDIT_ZONE, self.render_placeholder(ctx));
            return Ok(());
        }
        ctx.set_zone_pre_rendered(AUDIT_ZONE, self.render(ctx));
        Ok(())
    }
}

#[derive(Clone)]
struct AuditRecord {
    stage: AuditStageLabel,
    summary: String,
}

#[derive(Clone)]
enum AuditStageLabel {
    Constructed,
    Bootstrap,
    PluginRegistered,
    PluginInitialized,
    Event,
    Tick,
    Render,
    RenderSkip,
    Stopped,
}

impl std::fmt::Display for AuditStageLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Constructed => "constructed",
            Self::Bootstrap => "bootstrap",
            Self::PluginRegistered => "plugin_registered",
            Self::PluginInitialized => "plugin_init",
            Self::Event => "event",
            Self::Tick => "tick",
            Self::Render => "render",
            Self::RenderSkip => "render_skip",
            Self::Stopped => "stopped",
        };
        write!(f, "{text:>12}")
    }
}

impl std::fmt::Display for AuditRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.stage, self.summary)
    }
}

struct BufferAudit {
    events: Arc<Mutex<VecDeque<AuditRecord>>>,
}

impl BufferAudit {
    fn new(events: Arc<Mutex<VecDeque<AuditRecord>>>) -> Self {
        Self { events }
    }
}

impl RuntimeAudit for BufferAudit {
    fn record(&self, event: RuntimeAuditEvent) {
        let mut summary = event
            .details
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        if summary.is_empty() {
            summary = "(no details)".to_string();
        }

        let label = match event.stage {
            RuntimeAuditStage::RuntimeConstructed => AuditStageLabel::Constructed,
            RuntimeAuditStage::BootstrapStarted => AuditStageLabel::Bootstrap,
            RuntimeAuditStage::PluginRegistered => AuditStageLabel::PluginRegistered,
            RuntimeAuditStage::PluginInitialized => AuditStageLabel::PluginInitialized,
            RuntimeAuditStage::EventDispatched => AuditStageLabel::Event,
            RuntimeAuditStage::TickDispatched => AuditStageLabel::Tick,
            RuntimeAuditStage::RenderCommitted => AuditStageLabel::Render,
            RuntimeAuditStage::RenderSkipped => AuditStageLabel::RenderSkip,
            RuntimeAuditStage::RuntimeStopped => AuditStageLabel::Stopped,
        };

        if let Ok(mut guard) = self.events.lock() {
            guard.push_back(AuditRecord {
                stage: label,
                summary,
            });
            while guard.len() > 256 {
                guard.pop_front();
            }
        }
    }
}
