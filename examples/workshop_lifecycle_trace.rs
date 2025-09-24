use std::sync::{Arc, Mutex};
use std::time::Duration;

use room_mvp::runtime::RuntimeError;
use room_mvp::runtime::audit::{RuntimeAudit, RuntimeAuditEvent, RuntimeAuditStage};
use room_mvp::{
    AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, Result, RoomPlugin, RoomRuntime,
    RuntimeConfig, RuntimeContext, SimulatedLoop, Size,
};

const TRACE_ZONE: &str = "app:lifecycle.baseline";

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Lifecycle trace · WKSP-00 baseline");
    println!("Simulated loops ensure bounded execution while showing each teardown path.\n");

    for (idx, mode) in [
        TraceMode::Graceful,
        TraceMode::Recoverable,
        TraceMode::Fatal,
    ]
    .into_iter()
    .enumerate()
    {
        if idx > 0 {
            println!();
        }
        run_trace(mode)?;
    }

    println!("\nLifecycle trace complete.");
    Ok(())
}

fn run_trace(mode: TraceMode) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("== {} ==", mode.label());

    let layout = LayoutTree::new(LayoutNode {
        id: "app:root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1)],
        children: vec![LayoutNode::leaf(TRACE_ZONE)],
        gap: 0,
        padding: 0,
    });

    let renderer = AnsiRenderer::with_default();

    let mut config = RuntimeConfig::default();
    config.tick_interval = Duration::from_millis(8);
    config.simulated_loop = Some(SimulatedLoop::ticks(6));

    let audit = LifecycleTraceAudit::new();
    config.audit = Some(audit.clone());

    let mut runtime = RoomRuntime::with_config(layout, renderer, Size::new(80, 5), config)?;
    runtime.register_plugin(TracePlugin::new(mode));

    let mut sink = std::io::sink();
    runtime.signal_open();
    runtime.run(&mut sink)?;
    runtime.signal_end();
    runtime.signal_close()?;

    audit.print_summary();

    Ok(())
}

#[derive(Clone, Copy)]
enum TraceMode {
    Graceful,
    Recoverable,
    Fatal,
}

impl TraceMode {
    fn label(self) -> &'static str {
        match self {
            TraceMode::Graceful => "Graceful Exit",
            TraceMode::Recoverable => "Recoverable Error",
            TraceMode::Fatal => "Fatal Error",
        }
    }
}

struct TracePlugin {
    mode: TraceMode,
    step: u8,
    rendered: bool,
}

impl TracePlugin {
    fn new(mode: TraceMode) -> Self {
        Self {
            mode,
            step: 0,
            rendered: false,
        }
    }
}

impl RoomPlugin for TracePlugin {
    fn name(&self) -> &str {
        "lifecycle_trace_plugin"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        let instructions = match self.mode {
            TraceMode::Graceful => "Path: graceful teardown via request_exit()",
            TraceMode::Recoverable => "Path: recoverable error then graceful exit",
            TraceMode::Fatal => "Path: fatal error escalation (no manual exit)",
        };
        ctx.set_zone(TRACE_ZONE, instructions);
        ctx.request_render();
        Ok(())
    }

    fn after_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        if !self.rendered {
            self.rendered = true;
            ctx.request_render();
        }
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &room_mvp::RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if let room_mvp::RuntimeEvent::Tick { .. } = event {
            match (self.mode, self.step) {
                (TraceMode::Graceful, 0) => {
                    ctx.request_exit();
                    ctx.request_render();
                    self.step = 1;
                }
                (TraceMode::Recoverable, 0) => {
                    let error = RuntimeError {
                        category: "workshop".into(),
                        source: Some("baseline".into()),
                        message: "Simulated recoverable error".into(),
                        recoverable: true,
                    };
                    ctx.report_error(error);
                    ctx.request_render();
                    self.step = 1;
                }
                (TraceMode::Recoverable, 1) => {
                    ctx.request_exit();
                    ctx.request_render();
                    self.step = 2;
                }
                (TraceMode::Fatal, 0) => {
                    let error = RuntimeError {
                        category: "workshop".into(),
                        source: Some("baseline".into()),
                        message: "Simulated fatal error".into(),
                        recoverable: false,
                    };
                    ctx.report_error(error);
                    ctx.request_render();
                    self.step = 1;
                }
                _ => {}
            }
        }

        Ok(room_mvp::EventFlow::Continue)
    }
}

struct LifecycleTraceAudit {
    state: Mutex<TraceState>,
}

impl LifecycleTraceAudit {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(TraceState::default()),
        })
    }

    fn print_summary(&self) {
        let mut state = self.state.lock().unwrap();
        let summary = state.build_summary();
        drop(state);

        if summary.stages.is_empty() {
            println!("  Stages: (none)\n");
            return;
        }

        println!("  Stages:");
        for (idx, stage) in summary.stages.iter().enumerate() {
            println!("    {:>2}. {}", idx + 1, stage_label(*stage));
            if *stage == RuntimeAuditStage::LoopSimulated && !summary.loop_entries.is_empty() {
                for entry in &summary.loop_entries {
                    println!("          {entry}");
                }
            }
        }

        if !summary.notes.is_empty() {
            println!("  Notes:");
            for note in summary.notes {
                println!("    - {note}");
            }
        }
    }
}

impl RuntimeAudit for LifecycleTraceAudit {
    fn record(&self, event: RuntimeAuditEvent) {
        let mut state = self.state.lock().unwrap();
        state.ingest(event);
    }
}

struct TraceState {
    stages: Vec<RuntimeAuditStage>,
    loop_iterations: usize,
    loop_max: Option<usize>,
    dispatch_ticks: bool,
    user_end: bool,
    error_recoverable: Option<bool>,
    error_message: Option<String>,
    error_category: Option<String>,
    fatal_triggered: bool,
    fatal_cleanup: bool,
    fatal_close: bool,
    recovered: Option<bool>,
}

impl Default for TraceState {
    fn default() -> Self {
        Self {
            stages: Vec::new(),
            loop_iterations: 0,
            loop_max: None,
            dispatch_ticks: false,
            user_end: false,
            error_recoverable: None,
            error_message: None,
            error_category: None,
            fatal_triggered: false,
            fatal_cleanup: false,
            fatal_close: false,
            recovered: None,
        }
    }
}

impl TraceState {
    fn ingest(&mut self, event: RuntimeAuditEvent) {
        if stage_visible(event.stage) && !self.stages.contains(&event.stage) {
            self.stages.push(event.stage);
        }

        match event.stage {
            RuntimeAuditStage::LoopSimulated => {
                self.loop_max = detail_usize(&event, "max_iterations");
                self.dispatch_ticks = detail_bool(&event, "dispatch_ticks").unwrap_or(false);
            }
            RuntimeAuditStage::LoopIn => {
                if matches!(detail_str(&event, "kind"), Some(kind) if kind == "tick") {
                    self.loop_iterations += 1;
                }
            }
            RuntimeAuditStage::UserEnd => {
                self.user_end = true;
            }
            RuntimeAuditStage::Error => {
                self.error_recoverable = detail_bool(&event, "recoverable");
                self.error_category = detail_str(&event, "category");
                self.error_message = detail_str(&event, "message");
            }
            RuntimeAuditStage::RecoverOrFatal => {
                self.recovered = detail_bool(&event, "recovered");
            }
            RuntimeAuditStage::Fatal => {
                self.fatal_triggered = true;
            }
            RuntimeAuditStage::FatalCleanup => {
                self.fatal_cleanup = true;
            }
            RuntimeAuditStage::FatalClose => {
                self.fatal_close = true;
            }
            _ => {}
        }
    }

    fn build_summary(&mut self) -> TraceSummary {
        let mut ordered = Vec::new();
        for stage in STAGE_DISPLAY_ORDER {
            if self.stages.contains(stage) {
                ordered.push(*stage);
            }
        }
        for stage in &self.stages {
            if !ordered.contains(stage) {
                ordered.push(*stage);
            }
        }

        let loop_entries = if self.loop_iterations > 0 {
            let mut lines = (1..=self.loop_iterations)
                .map(|idx| format!("EventLoop[{idx}]"))
                .collect::<Vec<_>>();

            if let Some(max) = self.loop_max {
                if max > self.loop_iterations {
                    lines.push(format!("(stopped early out of {max})"));
                }
            }
            if !self.dispatch_ticks {
                lines.push("(ticks disabled)".to_string());
            }
            lines
        } else if let Some(max) = self.loop_max {
            let mut lines = vec![format!("(no iterations; configured max {max})")];
            if !self.dispatch_ticks {
                lines.push("(ticks disabled)".to_string());
            }
            lines
        } else {
            Vec::new()
        };

        let mut notes = Vec::new();

        if let Some(recoverable) = self.error_recoverable {
            let label = if recoverable { "recoverable" } else { "fatal" };
            let mut details = format!("Encountered {label} error");
            if let Some(category) = &self.error_category {
                details.push_str(&format!(" in category '{category}'"));
            }
            if let Some(message) = &self.error_message {
                details.push_str(&format!(": {message}"));
            }
            details.push('.');
            notes.push(details);
        }

        if let Some(recovered) = self.recovered {
            if recovered {
                notes.push("Error recovered; resumed graceful teardown.".to_string());
            } else {
                notes.push("Recovery failed; fatal teardown initiated.".to_string());
            }
        }

        if self.user_end {
            notes.push("Exit requested via UserEnd.".to_string());
        }

        if self.fatal_triggered {
            let mut stages = vec!["Fatal"]; // always includes Fatal when triggered
            if self.fatal_cleanup {
                stages.push("FatalCleanup");
            }
            if self.fatal_close {
                stages.push("FatalClose");
            }
            notes.push(format!("Fatal path executed ({}).", stages.join(" → ")));
        }

        TraceSummary {
            stages: ordered,
            loop_entries,
            notes,
        }
    }
}

struct TraceSummary {
    stages: Vec<RuntimeAuditStage>,
    loop_entries: Vec<String>,
    notes: Vec<String>,
}

const STAGE_DISPLAY_ORDER: &[RuntimeAuditStage] = &[
    RuntimeAuditStage::Open,
    RuntimeAuditStage::BootstrapStarted,
    RuntimeAuditStage::Boot,
    RuntimeAuditStage::PluginInitialized,
    RuntimeAuditStage::Setup,
    RuntimeAuditStage::RenderCommitted,
    RuntimeAuditStage::UserReady,
    RuntimeAuditStage::LoopSimulated,
    RuntimeAuditStage::Error,
    RuntimeAuditStage::RecoverOrFatal,
    RuntimeAuditStage::Fatal,
    RuntimeAuditStage::LoopSimulatedComplete,
    RuntimeAuditStage::UserEnd,
    RuntimeAuditStage::FatalCleanup,
    RuntimeAuditStage::Cleanup,
    RuntimeAuditStage::RuntimeStopped,
    RuntimeAuditStage::End,
    RuntimeAuditStage::FatalClose,
    RuntimeAuditStage::Close,
];

fn stage_label(stage: RuntimeAuditStage) -> &'static str {
    match stage {
        RuntimeAuditStage::Open => "Open",
        RuntimeAuditStage::BootstrapStarted => "BootstrapStarted",
        RuntimeAuditStage::Boot => "Boot",
        RuntimeAuditStage::PluginInitialized => "PluginInitialized",
        RuntimeAuditStage::Setup => "Setup",
        RuntimeAuditStage::RenderCommitted => "RenderCommitted",
        RuntimeAuditStage::UserReady => "UserReady",
        RuntimeAuditStage::LoopSimulated => "LoopSimulated",
        RuntimeAuditStage::Error => "Error",
        RuntimeAuditStage::RecoverOrFatal => "RecoverOrFatal",
        RuntimeAuditStage::Fatal => "Fatal",
        RuntimeAuditStage::LoopSimulatedComplete => "LoopSimulatedComplete",
        RuntimeAuditStage::UserEnd => "UserEnd",
        RuntimeAuditStage::FatalCleanup => "FatalCleanup",
        RuntimeAuditStage::Cleanup => "Cleanup",
        RuntimeAuditStage::RuntimeStopped => "RuntimeStopped",
        RuntimeAuditStage::End => "End",
        RuntimeAuditStage::FatalClose => "FatalClose",
        RuntimeAuditStage::Close => "Close",
        _ => "Other",
    }
}

fn stage_visible(stage: RuntimeAuditStage) -> bool {
    matches!(
        stage,
        RuntimeAuditStage::Open
            | RuntimeAuditStage::BootstrapStarted
            | RuntimeAuditStage::Boot
            | RuntimeAuditStage::PluginInitialized
            | RuntimeAuditStage::Setup
            | RuntimeAuditStage::RenderCommitted
            | RuntimeAuditStage::UserReady
            | RuntimeAuditStage::Error
            | RuntimeAuditStage::RecoverOrFatal
            | RuntimeAuditStage::Fatal
            | RuntimeAuditStage::FatalCleanup
            | RuntimeAuditStage::FatalClose
            | RuntimeAuditStage::UserEnd
            | RuntimeAuditStage::LoopSimulated
            | RuntimeAuditStage::LoopSimulatedComplete
            | RuntimeAuditStage::Cleanup
            | RuntimeAuditStage::RuntimeStopped
            | RuntimeAuditStage::End
            | RuntimeAuditStage::Close
    )
}

fn detail_bool(event: &RuntimeAuditEvent, key: &str) -> Option<bool> {
    event
        .details
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, value)| value.as_bool())
}

fn detail_usize(event: &RuntimeAuditEvent, key: &str) -> Option<usize> {
    event
        .details
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, value)| value.as_u64())
        .map(|value| value as usize)
}

fn detail_str(event: &RuntimeAuditEvent, key: &str) -> Option<String> {
    event
        .details
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, value)| value.as_str())
        .map(|value| value.to_string())
}
