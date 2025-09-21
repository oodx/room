use std::sync::Arc;
use std::time::Duration;

use room_mvp::runtime::audit::{BootstrapAudit, RuntimeAudit, RuntimeAuditEvent};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, Result, RoomPlugin,
    RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, Size,
};
const MESSAGE_ZONE: &str = "app:first_paint.message";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layout = LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1)],
        children: vec![LayoutNode::leaf(MESSAGE_ZONE)],
        gap: 0,
        padding: 0,
    });

    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.tick_interval = Duration::from_secs(10);
    let audit = BootstrapAudit::new(Arc::new(PrintAudit));
    config.audit = Some(audit);

    let mut runtime = RoomRuntime::with_config(layout, renderer, Size::new(80, 10), config)?;
    runtime.register_plugin(StaticBanner::default());

    CliDriver::new(runtime).run()?;
    Ok(())
}

#[derive(Default)]
struct StaticBanner;

impl RoomPlugin for StaticBanner {
    fn name(&self) -> &str {
        "static_banner"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(
            MESSAGE_ZONE,
            "First paint ready! Press any key to exit the demo.",
        );
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if matches!(event, RuntimeEvent::Key(_)) {
            ctx.request_exit();
        }
        Ok(room_mvp::EventFlow::Consumed)
    }
}

struct PrintAudit;

impl RuntimeAudit for PrintAudit {
    fn record(&self, event: RuntimeAuditEvent) {
        let details = event
            .details
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("[AUDIT] {:?} {details}", event.stage);
    }
}
