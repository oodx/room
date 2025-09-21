//! Room Workshop: Runtime Bootstrap & Cleanup
//!
//! This walkthrough highlights the expected UX flow for bootstrapping the Room runtime,
//! presenting the first frame, and exiting cleanly so the shell prompt returns to a sane
//! position.
//!
//! ```bash
//! cargo run --example workshop_room_bootstrap              # interactive flow
//! cargo run --example workshop_room_bootstrap -- capture   # capture walkthrough only
//! ```
//!
//! The `capture` flag skips launching the interactive runtime—useful for automated checks.

use std::io::{self, Read, Write};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent};
use room_mvp::cursor;
use room_mvp::runtime::audit::{BootstrapAudit, RuntimeAudit, RuntimeAuditEvent};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, Result, RoomPlugin,
    RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, Size,
};

const STATUS_ZONE: &str = "workshop:bootstrap.status";
const CAPTURE_HEIGHT: usize = 6;

fn main() -> Result<()> {
    let capture_only = std::env::args().any(|arg| arg == "capture");

    println!("Room Workshop · Runtime Bootstrap & Cleanup\n");
    println!("Step 1: Build a minimal runtime with a single status zone and audit helper.");

    let layout = LayoutTree::new(LayoutNode {
        id: "workshop:root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1)],
        children: vec![LayoutNode::leaf(STATUS_ZONE)],
        gap: 0,
        padding: 0,
    });

    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.audit = Some(BootstrapAudit::new(Arc::new(PrintAudit)));

    let mut runtime = RoomRuntime::with_config(layout, renderer, Size::new(60, 6), config)?;
    runtime.register_plugin(BootstrapWorkshop::default());

    println!(
        "\nStep 2: Use bootstrap controls to stage the first frame without touching the terminal yet.\n"
    );

    let mut capture = Vec::new();
    {
        let mut controls = runtime.bootstrap_controls(&mut capture)?;
        controls.present_first_frame()?;
        controls.run_ticks(2, Duration::from_millis(16))?;
        controls.finish()?;
    }

    println!("Captured first-frame snapshot (audit gated):\n");
    print!("{}", String::from_utf8_lossy(&capture));
    // Advance the terminal cursor beyond the captured frame so following prose stays lined up.
    print!("{}", cursor::move_down_lines((CAPTURE_HEIGHT + 1) as u16));
    println!("The runtime status panel is ready but has not yet entered raw-mode execution.\n");

    if capture_only {
        println!("Capture mode enabled – skipping interactive run.\n");
        return Ok(());
    }

    println!(
        "Step 3: Launch the runtime via `CliDriver`.\n  • Observe how Room enters raw mode and keeps the status zone updated.\n  • Press `Esc` or `q` when you're ready to exit.\n"
    );
    wait_for_enter()?;

    CliDriver::new(runtime)
        .run()
        .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))?;

    println!(
        "{}{}",
        cursor::move_down_lines(2),
        cursor::move_to_column(1)
    );
    println!(
        "Step 4: Runtime exited cleanly. Raw mode disabled, alternate screen released, and the cursor\nresumes below this message."
    );
    println!("\nWorkshop complete – the prompt below should be exactly where you expect it.");
    Ok(())
}

fn wait_for_enter() -> Result<()> {
    print!("Press Enter to continue...");
    io::stdout().flush()?;
    let mut buf = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    while handle.read(&mut buf)? == 1 {
        if buf[0] == b'\n' {
            break;
        }
    }
    Ok(())
}

#[derive(Default)]
struct BootstrapWorkshop {
    tick_count: usize,
}

impl RoomPlugin for BootstrapWorkshop {
    fn name(&self) -> &str {
        "workshop_bootstrap"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(STATUS_ZONE, render_status("Bootstrapping runtime", 0));
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if let RuntimeEvent::Tick { .. } = event {
            self.tick_count += 1;
            ctx.set_zone(
                STATUS_ZONE,
                render_status("Processing bootstrap tick", self.tick_count),
            );
        }

        if let RuntimeEvent::Key(KeyEvent {
            code: KeyCode::Esc, ..
        }) = event
        {
            ctx.set_zone(
                STATUS_ZONE,
                render_status("Exit requested", self.tick_count),
            );
            ctx.request_exit();
            return Ok(room_mvp::EventFlow::Consumed);
        }

        if let RuntimeEvent::Key(KeyEvent {
            code: KeyCode::Char('q'),
            ..
        }) = event
        {
            ctx.set_zone(
                STATUS_ZONE,
                render_status("Exit requested", self.tick_count),
            );
            ctx.request_exit();
            return Ok(room_mvp::EventFlow::Consumed);
        }

        Ok(room_mvp::EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(
            STATUS_ZONE,
            render_status("Runtime active", self.tick_count),
        );
        Ok(())
    }

    fn after_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(
            STATUS_ZONE,
            render_status("Runtime active", self.tick_count),
        );
        Ok(())
    }
}

fn render_status(stage: &str, ticks: usize) -> String {
    format!(
        "Room Bootstrap Workshop\n=========================\nStatus : {stage}\nTicks  : {ticks}\nKeys   : Esc or q to exit\n\n"
    )
}

struct PrintAudit;

impl RuntimeAudit for PrintAudit {
    fn record(&self, event: RuntimeAuditEvent) {
        println!("[AUDIT] {:?}", event.stage);
    }
}
