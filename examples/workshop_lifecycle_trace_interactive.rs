use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use room_mvp::runtime::RuntimeError;
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, EventFlow, LayoutNode, LayoutTree, Result,
    RoomPlugin, RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, Size,
};

const INSTRUCTIONS_ZONE: &str = "app:lifecycle.instructions";
const TIMELINE_ZONE: &str = "app:lifecycle.timeline";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let layout = LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Fixed(7), Constraint::Flex(1)],
        children: vec![
            LayoutNode::leaf(INSTRUCTIONS_ZONE),
            LayoutNode::leaf(TIMELINE_ZONE),
        ],
        gap: 1,
        padding: 1,
    });

    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.loop_iteration_limit = Some(1000); // Safety guard to prevent infinite loops

    let mut runtime = RoomRuntime::with_config(layout, renderer, Size::new(100, 20), config)?;
    runtime.register_plugin(LifecycleTimelinePlugin::default());

    println!("🎯 Interactive Lifecycle Trace - UserReady Fix Test");
    println!("Press keys to see events, Enter to exit gracefully, Ctrl+C for emergency exit");
    println!("This will test that UserReady fires and keyboard input works!");
    println!("");

    CliDriver::new(runtime).run()?;
    Ok(())
}

#[derive(Default)]
struct LifecycleTimelinePlugin {
    entries: Vec<StageEntry>,
    started_at: Option<Instant>,
    needs_redraw: bool,
    first_render_logged: bool,
    fatal_logged: bool,
}

struct StageEntry {
    label: &'static str,
    elapsed: Duration,
}

impl LifecycleTimelinePlugin {
    fn record_stage(&mut self, label: &'static str) {
        let now = Instant::now();
        let start = self.started_at.get_or_insert(now);
        let elapsed = now.saturating_duration_since(*start);

        if self.entries.iter().any(|entry| entry.label == label) {
            return;
        }

        self.entries.push(StageEntry { label, elapsed });
        self.needs_redraw = true;
    }

    fn render_timeline(&self) -> String {
        if self.entries.is_empty() {
            return "Lifecycle checkpoints will appear once bootstrap begins.".to_string();
        }

        let mut lines = vec![
            "Lifecycle checkpoints (non-loop)".to_string(),
            "".to_string(),
        ];
        for (idx, entry) in self.entries.iter().enumerate() {
            let duration = format_duration(entry.elapsed);
            lines.push(format!("{:>2}. [{duration}] {}", idx + 1, entry.label));
        }
        lines.join("\n")
    }

    fn refresh_view(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        if !self.needs_redraw {
            return Ok(());
        }

        ctx.set_zone(TIMELINE_ZONE, self.render_timeline());
        self.needs_redraw = false;
        Ok(())
    }

    fn handle_key(&mut self, ctx: &mut RuntimeContext<'_>, key: &KeyEvent) -> Result<EventFlow> {
        match key.code {
            KeyCode::Enter => {
                ctx.request_exit();
                ctx.request_render();
                Ok(EventFlow::Consumed)
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                let error = RuntimeError {
                    category: "workshop".to_string(),
                    source: Some("hotkey".to_string()),
                    message: "Recoverable lifecycle probe".to_string(),
                    recoverable: true,
                };
                ctx.report_error(error);
                ctx.request_render();
                Ok(EventFlow::Consumed)
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                let error = RuntimeError {
                    category: "workshop".to_string(),
                    source: Some("hotkey".to_string()),
                    message: "Fatal lifecycle probe".to_string(),
                    recoverable: false,
                };
                ctx.report_error(error);
                ctx.request_render();
                Ok(EventFlow::Consumed)
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                ctx.request_exit();
                ctx.request_render();
                Ok(EventFlow::Consumed)
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ctx.request_exit();
                ctx.request_render();
                Ok(EventFlow::Consumed)
            }
            _ => Ok(EventFlow::Continue),
        }
    }
}

impl RoomPlugin for LifecycleTimelinePlugin {
    fn name(&self) -> &str {
        "lifecycle_timeline"
    }

    fn on_boot(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.record_stage("Boot");
        ctx.request_render();
        Ok(())
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(INSTRUCTIONS_ZONE, INSTRUCTIONS_TEXT.trim());
        ctx.set_zone(TIMELINE_ZONE, self.render_timeline());
        ctx.request_render();
        Ok(())
    }

    fn on_setup(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.record_stage("Setup");
        ctx.request_render();
        Ok(())
    }

    fn on_user_ready(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.record_stage("UserReady");
        ctx.request_render();
        Ok(())
    }

    fn on_user_end(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.record_stage("UserEnd");
        ctx.request_render();
        Ok(())
    }

    fn on_cleanup(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.record_stage("Cleanup");
        ctx.request_render();
        Ok(())
    }

    fn on_close(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.record_stage("Close");
        ctx.request_render();
        Ok(())
    }

    fn on_error(&mut self, ctx: &mut RuntimeContext<'_>, error: &mut RuntimeError) -> Result<()> {
        if error.recoverable {
            self.record_stage("RecoverableError");
        } else {
            self.record_stage("FatalError");
        }
        ctx.request_render();
        Ok(())
    }

    fn on_recover_or_fatal(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        _error: &RuntimeError,
        recovered: bool,
    ) -> Result<()> {
        if recovered {
            self.record_stage("Recovered");
        } else {
            self.record_stage("FatalEscalation");
            ctx.request_render();
        }
        Ok(())
    }

    fn on_fatal(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        if !self.fatal_logged {
            self.record_stage("Fatal");
            self.fatal_logged = true;
            ctx.request_render();
        }
        Ok(())
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.refresh_view(ctx)
    }

    fn after_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        if !self.first_render_logged {
            self.first_render_logged = true;
            self.record_stage("RenderCommitted");
            ctx.request_render();
        }
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        if let RuntimeEvent::Key(key) = event {
            return self.handle_key(ctx, key);
        }
        Ok(EventFlow::Continue)
    }
}

static INSTRUCTIONS_TEXT: &str = r#"
Lifecycle Trace · WKSP-00 baseline

Hotkeys:
  Enter → request graceful exit
  e     → emit recoverable error (stays alive)
  f     → emit fatal error (forces teardown)
  Esc/q/Ctrl+C → emergency exit

The timeline below lists non-loop lifecycle stages in order with
relative timings so you can confirm the runtime contract.
"#;

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    format!("{:02}.{millis:03}s", secs)
}
