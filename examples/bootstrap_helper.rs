use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use room_mvp::cursor;
use room_mvp::runtime::audit::{BootstrapAudit, RuntimeAudit, RuntimeAuditEvent};
use room_mvp::{
    AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, LegacyScreenStrategy, Result,
    RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, ScreenDefinition,
    ScreenManager, Size,
};

const STATUS_ZONE: &str = "app:bootstrap.status";
const BOOTSTRAP_HEIGHT: usize = 4;

fn main() -> Result<()> {
    let layout = LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1)],
        children: vec![LayoutNode::leaf(STATUS_ZONE)],
        gap: 0,
        padding: 0,
    });
    let screen_layout = layout.clone();

    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.audit = Some(BootstrapAudit::new(Arc::new(PrintAudit)));

    let mut runtime = RoomRuntime::with_config(layout, renderer, Size::new(60, 4), config)?;

    let mut screen_manager = ScreenManager::new();
    screen_manager.register_screen(ScreenDefinition::new(
        "bootstrap-helper",
        "Bootstrap Helper",
        Arc::new(move || Box::new(LegacyScreenStrategy::new(screen_layout.clone()))),
    ));
    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("bootstrap-helper")?;
    runtime.register_plugin(Ticker::default());

    let mut buffer = Vec::new();
    {
        let mut controls = runtime.bootstrap_controls(&mut buffer)?;
        controls.present_first_frame()?;
        controls.run_ticks(2, Duration::from_millis(16))?;

        let mut scripted = vec![
            RuntimeEvent::Tick {
                elapsed: Duration::from_millis(16),
            },
            RuntimeEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ]
        .into_iter();

        controls.gate_on_first_key_event(|| Ok(scripted.next()))?;
        controls.finish()?;
    }

    println!(
        "Captured bootstrap frame:\n{}",
        String::from_utf8_lossy(&buffer)
    );

    // Reposition the cursor below the captured frame so subsequent output resumes cleanly.
    print!("{}", cursor::move_down_lines((BOOTSTRAP_HEIGHT + 1) as u16));
    println!();
    Ok(())
}

#[derive(Default)]
struct Ticker {
    ticks: usize,
}

impl RoomPlugin for Ticker {
    fn name(&self) -> &str {
        "ticker"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(STATUS_ZONE, "Bootstrapping...");
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if matches!(event, RuntimeEvent::Tick { .. }) {
            self.ticks += 1;
            ctx.set_zone(
                STATUS_ZONE,
                format!("Bootstrap ticks observed: {}", self.ticks),
            );
        }
        if matches!(event, RuntimeEvent::Key(_)) {
            ctx.set_zone(STATUS_ZONE, "Bootstrap complete!");
        }
        Ok(room_mvp::EventFlow::Continue)
    }
}

struct PrintAudit;

impl RuntimeAudit for PrintAudit {
    fn record(&self, event: RuntimeAuditEvent) {
        println!("[AUDIT] {:?}", event.stage);
    }
}
