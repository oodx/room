use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::logging::{LogLevel, Logger};
use crate::metrics::RuntimeMetrics;
use crate::{Result, display_width};

use super::diagnostics::{LifecycleLoggerPlugin, MetricsSnapshotPlugin};
use super::focus::{FocusController, ensure_focus_registry};
use super::shared_state::SharedStateError;
use super::{EventFlow, PluginBundle, RoomPlugin, RuntimeContext, RuntimeEvent};

pub const DEFAULT_STATUS_ZONE: &str = "app:runtime.status";
pub const DEFAULT_INPUT_ZONE: &str = "app:runtime.input";
pub const DEFAULT_HINTS_ZONE: &str = "app:runtime.hints";
const DEFAULT_FOCUS_OWNER: &str = "room::default_bundle";
const DEFAULT_HINTS_TEXT: &str = "Enter to submit · Tab cycles focus · Esc clears";

#[derive(Clone)]
pub struct DefaultCliBundleConfig {
    pub status_zone: String,
    pub input_zone: String,
    pub hints_zone: Option<String>,
    pub focus_owner: String,
    pub input_priority: i32,
    pub status_priority: i32,
    pub diagnostics: Option<DiagnosticsConfig>,
}

impl Default for DefaultCliBundleConfig {
    fn default() -> Self {
        Self {
            status_zone: DEFAULT_STATUS_ZONE.to_string(),
            input_zone: DEFAULT_INPUT_ZONE.to_string(),
            hints_zone: Some(DEFAULT_HINTS_ZONE.to_string()),
            focus_owner: DEFAULT_FOCUS_OWNER.to_string(),
            input_priority: -20,
            status_priority: 80,
            diagnostics: None,
        }
    }
}

#[derive(Clone)]
pub struct DiagnosticsConfig {
    pub logger: Logger,
    pub lifecycle_priority: i32,
    pub level: LogLevel,
    pub log_keys: bool,
    pub log_mouse: bool,
    pub log_paste: bool,
    pub log_ticks: bool,
    pub log_raw: bool,
    pub metrics: Option<DiagnosticsMetricsConfig>,
}

impl DiagnosticsConfig {
    fn build_logger_plugin(&self) -> LifecycleLoggerPlugin {
        LifecycleLoggerPlugin::new(self.logger.clone())
            .with_level(self.level)
            .log_keys(self.log_keys)
            .log_mouse(self.log_mouse)
            .log_paste(self.log_paste)
            .log_ticks(self.log_ticks)
            .log_raw(self.log_raw)
    }
}

#[derive(Clone)]
pub struct DiagnosticsMetricsConfig {
    pub metrics: Arc<Mutex<RuntimeMetrics>>,
    pub interval: Duration,
    pub target: String,
    pub priority: i32,
}

pub fn default_cli_bundle(config: DefaultCliBundleConfig) -> PluginBundle {
    let mut bundle = PluginBundle::new()
        .with_plugin(
            DefaultInputPlugin::new(
                config.input_zone.clone(),
                config.hints_zone.clone(),
                config.focus_owner.clone(),
            ),
            config.input_priority,
        )
        .with_plugin(
            DefaultStatusBarPlugin::new(config.status_zone.clone()),
            config.status_priority,
        );

    if let Some(diag) = config.diagnostics.clone() {
        bundle = bundle.with_plugin(diag.build_logger_plugin(), diag.lifecycle_priority);
        if let Some(metrics_cfg) = diag.metrics {
            let plugin = MetricsSnapshotPlugin::new(diag.logger.clone(), metrics_cfg.metrics)
                .with_interval(metrics_cfg.interval)
                .with_target(metrics_cfg.target);
            bundle = bundle.with_plugin(plugin, metrics_cfg.priority);
        }
    }

    bundle
}

#[derive(Default, Debug, Clone)]
pub struct InputSharedState {
    pub last_submission: Option<String>,
    pub submission_count: u64,
}

pub type SharedInputState = Arc<RwLock<InputSharedState>>;

pub fn ensure_input_state(ctx: &RuntimeContext<'_>) -> Result<SharedInputState> {
    ctx.shared_init::<RwLock<InputSharedState>, _>(|| RwLock::new(InputSharedState::default()))
        .map_err(map_shared_err)
}

pub fn try_input_state(ctx: &RuntimeContext<'_>) -> Option<SharedInputState> {
    ctx.shared::<RwLock<InputSharedState>>().ok()
}

struct DefaultInputPlugin {
    zone_id: String,
    hints_zone: Option<String>,
    focus_owner: String,
    buffer: String,
    focus: Option<FocusController>,
}

impl DefaultInputPlugin {
    fn new(zone_id: String, hints_zone: Option<String>, focus_owner: String) -> Self {
        Self {
            zone_id,
            hints_zone,
            focus_owner,
            buffer: String::new(),
            focus: None,
        }
    }

    fn focus_controller<'a>(
        &'a mut self,
        ctx: &RuntimeContext<'a>,
    ) -> std::result::Result<&'a mut FocusController, SharedStateError> {
        if self.focus.is_none() {
            let registry = ensure_focus_registry(ctx)?;
            self.focus = Some(FocusController::new(self.focus_owner.clone(), registry));
        }
        Ok(self.focus.as_mut().expect("focus controller present"))
    }

    fn render(&self, ctx: &mut RuntimeContext<'_>) {
        let caret_position = display_width(&self.buffer) as u16 + 2;
        let mut content = String::from("> ");
        content.push_str(&self.buffer);
        content.push('█');
        ctx.set_zone(&self.zone_id, content);
        if let Some(rect) = ctx.rect(&self.zone_id).copied() {
            let col = rect.x + caret_position;
            ctx.set_cursor_hint(rect.y, col.min(rect.x + rect.width.saturating_sub(1)));
        }
        if let Some(zone) = &self.hints_zone {
            ctx.set_zone(zone, DEFAULT_HINTS_TEXT);
        }
    }

    fn submit(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        let text = self.buffer.trim();
        if text.is_empty() {
            self.buffer.clear();
            self.render(ctx);
            return Ok(());
        }

        let shared = ensure_input_state(ctx)?;
        if let Ok(mut guard) = shared.write() {
            guard.submission_count += 1;
            guard.last_submission = Some(text.to_string());
        }

        self.buffer.clear();
        ctx.request_render();
        self.render(ctx);
        Ok(())
    }

    fn handle_key(&mut self, ctx: &mut RuntimeContext<'_>, key: &KeyEvent) -> Result<EventFlow> {
        if key.kind != KeyEventKind::Press {
            return Ok(EventFlow::Continue);
        }

        match key.code {
            KeyCode::Backspace => {
                self.buffer.pop();
                ctx.request_render();
                self.render(ctx);
                Ok(EventFlow::Consumed)
            }
            KeyCode::Enter => {
                self.submit(ctx)?;
                Ok(EventFlow::Consumed)
            }
            KeyCode::Esc => {
                self.buffer.clear();
                ctx.request_render();
                self.render(ctx);
                Ok(EventFlow::Consumed)
            }
            KeyCode::Char(ch) => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::ALT)
                {
                    return Ok(EventFlow::Continue);
                }
                self.buffer.push(ch);
                ctx.request_render();
                self.render(ctx);
                Ok(EventFlow::Consumed)
            }
            _ => Ok(EventFlow::Continue),
        }
    }
}

impl RoomPlugin for DefaultInputPlugin {
    fn name(&self) -> &str {
        "room::bundle.input"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        let zone = self.zone_id.clone();
        self.focus_controller(ctx)
            .map_err(map_shared_err)?
            .focus(zone);
        ensure_input_state(ctx)?;
        self.render(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        match event {
            RuntimeEvent::Key(key) => self.handle_key(ctx, key),
            RuntimeEvent::Paste(data) => {
                if !data.is_empty() {
                    self.buffer.push_str(data);
                    ctx.request_render();
                    self.render(ctx);
                }
                Ok(EventFlow::Consumed)
            }
            RuntimeEvent::FocusGained => {
                let zone = self.zone_id.clone();
                self.focus_controller(ctx)
                    .map_err(map_shared_err)?
                    .focus(zone);
                ctx.request_render();
                self.render(ctx);
                Ok(EventFlow::Continue)
            }
            _ => Ok(EventFlow::Continue),
        }
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.render(ctx);
        Ok(())
    }
}

struct DefaultStatusBarPlugin {
    zone_id: String,
}

impl DefaultStatusBarPlugin {
    fn new(zone_id: String) -> Self {
        Self { zone_id }
    }

    fn status_line(&self, ctx: &RuntimeContext<'_>) -> String {
        let focus_label = ensure_focus_registry(ctx)
            .ok()
            .and_then(|registry| registry.current())
            .map(|entry| friendly_zone_name(&entry.zone_id).to_string())
            .unwrap_or_else(|| "none".to_string());

        let (submissions, last) = try_input_state(ctx)
            .and_then(|shared| shared.read().ok().map(|guard| guard.clone()))
            .map(|data| (data.submission_count, data.last_submission))
            .unwrap_or((0, None));

        let mut line = format!(
            "Status · focus:{} · submissions:{}",
            focus_label, submissions
        );
        if let Some(last_submission) = last {
            if !last_submission.is_empty() {
                line.push_str(" · last:");
                line.push_str(&truncate_display(&last_submission, 32));
            }
        }
        line
    }
}

impl RoomPlugin for DefaultStatusBarPlugin {
    fn name(&self) -> &str {
        "room::bundle.status"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(&self.zone_id, self.status_line(ctx));
        Ok(())
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(&self.zone_id, self.status_line(ctx));
        Ok(())
    }
}

fn friendly_zone_name(zone: &str) -> &str {
    zone.rsplit(':').next().unwrap_or(zone)
}

fn truncate_display(text: &str, max_width: usize) -> String {
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let mut result = String::new();
    let mut width = 0usize;
    for ch in text.chars() {
        let w = display_width(&ch.to_string());
        if width + w >= max_width {
            if width < max_width {
                result.push('…');
            }
            break;
        }
        width += w;
        result.push(ch);
    }
    result
}

fn map_shared_err(err: SharedStateError) -> crate::LayoutError {
    crate::LayoutError::Backend(format!("shared state error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, RoomRuntime, Size};

    #[test]
    fn bundle_compiles_and_initializes() {
        let layout = LayoutTree::new(LayoutNode {
            id: "root".to_string(),
            direction: Direction::Column,
            constraints: vec![
                Constraint::Fixed(1),
                Constraint::Fixed(1),
                Constraint::Flex(1),
            ],
            children: vec![
                LayoutNode::leaf(DEFAULT_STATUS_ZONE),
                LayoutNode::leaf(DEFAULT_HINTS_ZONE),
                LayoutNode::leaf(DEFAULT_INPUT_ZONE),
            ],
            gap: 0,
            padding: 0,
        });
        let renderer = AnsiRenderer::with_default();
        let mut runtime = RoomRuntime::new(layout, renderer, Size::new(80, 24)).unwrap();
        let bundle = default_cli_bundle(DefaultCliBundleConfig::default());
        runtime.register_bundle(bundle);
        // run scripted with no events to ensure bootstrap renders
        let mut output = Vec::new();
        runtime
            .run_scripted(&mut output, std::iter::empty())
            .unwrap();
        assert!(!output.is_empty());
    }
}
