use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use room_mvp::logging::{LogEvent, LogSink};
use room_mvp::runtime::diagnostics::{LifecycleLoggerPlugin, MetricsSnapshotPlugin};
use room_mvp::{
    AnsiRenderer, Constraint, Direction, EventFlow, FocusRegistry, LayoutNode, LayoutTree, Logger,
    LoggingResult, PluginBundle, Rect, Result, RoomPlugin, RoomRuntime, RuntimeContext,
    RuntimeEvent, Size, display_width, ensure_focus_registry,
};

#[derive(Clone, Default)]
struct NullSink;

impl LogSink for NullSink {
    fn log(&self, _event: &LogEvent) -> LoggingResult<()> {
        Ok(())
    }
}

const HEADER_ZONE: &str = "app:chat.header";
const TIMELINE_ZONE: &str = "app:chat.timeline";
const SIDEBAR_ZONE: &str = "app:chat.sidebar";
const STATUS_ZONE: &str = "app:chat.footer.status";
const INPUT_ZONE: &str = "app:chat.footer.input";

fn runtime_chat_script(c: &mut Criterion) {
    let script = scripted_events();
    c.bench_function("runtime_chat_script", |b| {
        b.iter(|| {
            let mut runtime = build_runtime().expect("runtime");
            let mut sink = io::sink();
            runtime
                .run_scripted(&mut sink, black_box(script.clone()))
                .expect("scripted run");
        });
    });
}

fn runtime_focus_script(c: &mut Criterion) {
    let script = focus_events();
    c.bench_function("runtime_focus_script", |b| {
        b.iter(|| {
            let mut runtime = build_focus_runtime().expect("runtime");
            let mut sink = io::sink();
            runtime
                .run_scripted(&mut sink, black_box(script.clone()))
                .expect("scripted run");
        });
    });
}

fn build_runtime() -> Result<RoomRuntime> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut runtime = RoomRuntime::new(layout, renderer, Size::new(100, 30))?;

    let logger = Logger::new(NullSink::default());
    let metrics_handle = {
        let config = runtime.config_mut();
        config.logger = Some(logger.clone());
        config.metrics_interval = Duration::from_millis(0);
        config.enable_metrics();
        config.metrics_handle().expect("metrics handle")
    };

    PluginBundle::new()
        .with_plugin(
            LifecycleLoggerPlugin::new(logger.clone())
                .log_mouse(false)
                .log_ticks(false)
                .log_raw(false)
                .log_keys(false),
            -100,
        )
        .with_plugin(
            MetricsSnapshotPlugin::new(logger.clone(), metrics_handle.clone())
                .with_interval(Duration::from_millis(250)),
            100,
        )
        .with_plugin(BenchChatPlugin::new(), 0)
        .register_into(&mut runtime);

    Ok(runtime)
}

fn build_focus_runtime() -> Result<RoomRuntime> {
    let layout = build_layout();
    let renderer = AnsiRenderer::with_default();
    let mut runtime = RoomRuntime::new(layout, renderer, Size::new(100, 30))?;

    let logger = Logger::new(NullSink::default());
    let metrics_handle = {
        let config = runtime.config_mut();
        config.logger = Some(logger.clone());
        config.metrics_interval = Duration::from_millis(0);
        config.enable_metrics();
        config.metrics_handle().expect("metrics handle")
    };

    PluginBundle::new()
        .with_plugin(
            LifecycleLoggerPlugin::new(logger.clone())
                .log_mouse(false)
                .log_ticks(false)
                .log_raw(false)
                .log_keys(false),
            -100,
        )
        .with_plugin(
            MetricsSnapshotPlugin::new(logger.clone(), metrics_handle.clone())
                .with_interval(Duration::from_millis(250)),
            100,
        )
        .with_plugin(FocusStressPlugin::new(), 0)
        .register_into(&mut runtime);

    Ok(runtime)
}

fn scripted_events() -> Vec<RuntimeEvent> {
    vec![
        RuntimeEvent::Resize(Size::new(100, 30)),
        RuntimeEvent::Key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE)),
        RuntimeEvent::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        RuntimeEvent::Key(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE)),
        RuntimeEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        RuntimeEvent::Tick {
            elapsed: Duration::from_millis(500),
        },
        RuntimeEvent::Key(KeyEvent::new(KeyCode::Char('O'), KeyModifiers::NONE)),
        RuntimeEvent::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        RuntimeEvent::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
        RuntimeEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        RuntimeEvent::Tick {
            elapsed: Duration::from_millis(600),
        },
    ]
}

fn focus_events() -> Vec<RuntimeEvent> {
    let mut events = Vec::with_capacity(200);
    events.push(RuntimeEvent::Resize(Size::new(100, 30)));
    for _ in 0..50 {
        events.push(RuntimeEvent::Key(KeyEvent::new(
            KeyCode::Char('1'),
            KeyModifiers::NONE,
        )));
        events.push(RuntimeEvent::Key(KeyEvent::new(
            KeyCode::Char('2'),
            KeyModifiers::NONE,
        )));
        events.push(RuntimeEvent::Key(KeyEvent::new(
            KeyCode::Char('3'),
            KeyModifiers::NONE,
        )));
    }
    events.push(RuntimeEvent::Tick {
        elapsed: Duration::from_millis(16),
    });
    events
}

#[derive(Clone)]
struct BenchChatPlugin {
    participants: Vec<&'static str>,
    messages: Vec<String>,
    scripted_replies: VecDeque<String>,
    input_buffer: String,
}

impl BenchChatPlugin {
    fn new() -> Self {
        Self {
            participants: vec!["Alice", "Bob", "You"],
            messages: vec!["Alice: Welcome back to Room.".to_string()],
            scripted_replies: VecDeque::from(vec![
                "Bob: Rendering stays minimal.".to_string(),
                "Alice: Plugins keep logic tidy.".to_string(),
            ]),
            input_buffer: String::new(),
        }
    }

    fn redraw(&mut self, ctx: &mut RuntimeContext<'_>) {
        let header_text = format!("Room Runtime Bench · {} online", self.participants.len());
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

        let input_rect = ctx.rect(INPUT_ZONE).copied().unwrap_or(Rect::new(
            timeline_rect.x,
            timeline_rect.bottom(),
            60,
            1,
        ));
        let status_rect = ctx.rect(STATUS_ZONE).copied().unwrap_or(Rect::new(
            input_rect.x,
            input_rect.y.saturating_add(1),
            input_rect.width,
            3,
        ));

        let status_text = format!(
            "{} messages · pending replies {}",
            self.messages.len(),
            self.scripted_replies.len()
        );

        let input_display = format!(">{}", self.input_buffer);
        let caret_base = input_rect.x.saturating_add(1);
        let typed_width = display_width(&self.input_buffer) as u16;
        let caret_limit = input_rect
            .x
            .saturating_add(input_rect.width.saturating_sub(1))
            .max(caret_base);
        let caret_x = caret_base.saturating_add(typed_width).min(caret_limit);

        ctx.set_zone(HEADER_ZONE, header_text);
        ctx.set_zone(TIMELINE_ZONE, timeline_text);
        ctx.set_zone(SIDEBAR_ZONE, sidebar_text);
        ctx.set_zone(STATUS_ZONE, status_text);
        ctx.set_zone(INPUT_ZONE, input_display);
        ctx.set_cursor_hint(input_rect.y, caret_x);
    }
}

impl RoomPlugin for BenchChatPlugin {
    fn name(&self) -> &str {
        "bench.chat"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.redraw(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        match event {
            RuntimeEvent::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Ok(EventFlow::Continue);
                }

                match key.code {
                    KeyCode::Esc => {
                        ctx.request_exit();
                        return Ok(EventFlow::Consumed);
                    }
                    KeyCode::Enter => {
                        let trimmed = self.input_buffer.trim();
                        if !trimmed.is_empty() {
                            self.messages.push(format!("You: {}", trimmed));
                            self.input_buffer.clear();
                            if let Some(reply) = self.scripted_replies.pop_front() {
                                self.messages.push(reply);
                            }
                            self.redraw(ctx);
                        }
                    }
                    KeyCode::Backspace => {
                        if self.input_buffer.pop().is_some() {
                            self.redraw(ctx);
                        }
                    }
                    KeyCode::Char(ch) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.input_buffer.push(ch);
                            self.redraw(ctx);
                        }
                    }
                    _ => {}
                }
            }
            RuntimeEvent::Paste(data) => {
                if !data.is_empty() {
                    self.input_buffer.push_str(data);
                    self.redraw(ctx);
                }
            }
            RuntimeEvent::Resize(_) => {
                self.redraw(ctx);
            }
            RuntimeEvent::Tick { .. } => {
                if let Some(reply) = self.scripted_replies.pop_front() {
                    self.messages.push(reply);
                    self.redraw(ctx);
                }
            }
            _ => {}
        }

        Ok(EventFlow::Continue)
    }
}

struct FocusStressPlugin {
    focus: Option<Arc<FocusRegistry>>,
}

impl FocusStressPlugin {
    fn new() -> Self {
        Self { focus: None }
    }

    fn update_focus(&mut self, ctx: &mut RuntimeContext<'_>, zone: &str) {
        if let Some(registry) = &self.focus {
            registry.set_focus("bench.focus", zone);
        }
        ctx.set_zone(STATUS_ZONE, format!("focus: {zone}"));
        ctx.set_zone(TIMELINE_ZONE, format!("active zone -> {zone}"));
    }
}

impl RoomPlugin for FocusStressPlugin {
    fn name(&self) -> &str {
        "bench.focus"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        let focus = ensure_focus_registry(ctx).expect("focus registry available");
        self.focus = Some(focus);
        ctx.set_zone(HEADER_ZONE, "Room Runtime Focus Bench".to_string());
        ctx.set_zone(SIDEBAR_ZONE, "zones: timeline/input/status".to_string());
        ctx.set_zone(STATUS_ZONE, "focus: none".to_string());
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        if let RuntimeEvent::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(EventFlow::Continue);
            }
            match key.code {
                KeyCode::Char('1') => self.update_focus(ctx, TIMELINE_ZONE),
                KeyCode::Char('2') => self.update_focus(ctx, INPUT_ZONE),
                KeyCode::Char('3') => self.update_focus(ctx, STATUS_ZONE),
                _ => {}
            }
        }
        Ok(EventFlow::Continue)
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

criterion_group!(benches, runtime_chat_script, runtime_focus_script);
criterion_main!(benches);
