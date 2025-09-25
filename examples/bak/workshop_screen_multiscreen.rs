//! Room Workshop: Multi-Screen Flow Control
//!
//! Demonstrates how multiple screens coordinate shared and screen-scoped state,
//! exercise the new navigation helpers, and keep an activity log synchronized
//! across the runtime. Use the bundled hotkeys (`Ctrl+Tab`, `Ctrl+Shift+Tab`)
//! or jump directly with `1`, `2`, and `3`.
//!
//! ```bash
//! cargo run --example workshop_screen_multiscreen
//! ```

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

use crossterm::event::{KeyCode, KeyEvent};
use room_mvp::runtime::bundles::{DefaultCliBundleConfig, default_cli_bundle};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, Result, RoomPlugin,
    RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, ScreenDefinition, ScreenManager,
    ScreenMetadata, ScreenNavigator, ScreenState, Size,
};

const HEADER_ZONE: &str = "workshop:multiscreen.header";
const BODY_ZONE: &str = "workshop:multiscreen.body";

fn main() -> Result<()> {
    println!("Room Workshop · Multi-Screen Flow Control\n");
    println!(
        "Instructions:\n  • Ctrl+Tab / Ctrl+Shift+Tab cycle screens using the default hotkeys.\n  • Press 1/2/3 to activate Dashboard, Settings, or Activity directly.\n  • Inside Settings press 't' to toggle the theme, 'n' to toggle notifications.\n  • Inside Activity press 'c' to clear the log.\n  • Press 'd' on any screen to return to the dashboard.\n"
    );

    let layout = workshop_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.default_focus_zone = Some(BODY_ZONE.to_string());

    let mut runtime =
        RoomRuntime::with_config(layout.clone(), renderer, Size::new(78, 18), config)?;

    let descriptors = vec![
        ScreenDescriptor::new(
            "dashboard",
            "Dashboard",
            "Runtime snapshot with shared configuration",
            ScreenKind::Dashboard,
        ),
        ScreenDescriptor::new(
            "settings",
            "Settings",
            "Toggle theme/notifications and trigger navigation",
            ScreenKind::Settings,
        ),
        ScreenDescriptor::new(
            "activity",
            "Activity Log",
            "Inspect history emitted from other screens",
            ScreenKind::Activity,
        ),
    ];
    let screen_ids: Arc<Vec<String>> =
        Arc::new(descriptors.iter().map(|desc| desc.id.to_string()).collect());

    let mut manager = ScreenManager::new();
    for descriptor in descriptors {
        let ids = Arc::clone(&screen_ids);
        manager.register_screen(ScreenDefinition {
            id: descriptor.id.to_string(),
            title: descriptor.title.to_string(),
            metadata: ScreenMetadata {
                description: Some(descriptor.blurb.to_string()),
                shortcuts: vec!["1/2/3 jump".into(), "Ctrl+Tab".into()],
            },
            factory: Arc::new(move || {
                Box::new(WorkshopScreenStrategy::new(
                    descriptor.clone(),
                    Arc::clone(&ids),
                    workshop_layout(),
                ))
            }),
        });
    }

    runtime.set_screen_manager(manager);
    runtime.activate_screen("dashboard")?;
    runtime.register_bundle(default_cli_bundle(DefaultCliBundleConfig::default()));

    CliDriver::new(runtime)
        .run()
        .map_err(|err| room_mvp::LayoutError::Backend(err.to_string()))
}

#[derive(Clone)]
struct ScreenDescriptor {
    id: &'static str,
    title: &'static str,
    blurb: &'static str,
    kind: ScreenKind,
}

impl ScreenDescriptor {
    const fn new(
        id: &'static str,
        title: &'static str,
        blurb: &'static str,
        kind: ScreenKind,
    ) -> Self {
        Self {
            id,
            title,
            blurb,
            kind,
        }
    }
}

#[derive(Clone, Copy)]
enum ScreenKind {
    Dashboard,
    Settings,
    Activity,
}

fn workshop_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "workshop:multiscreen.root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Fixed(2), Constraint::Flex(1)],
        children: vec![LayoutNode::leaf(HEADER_ZONE), LayoutNode::leaf(BODY_ZONE)],
        gap: 1,
        padding: 1,
    })
}

struct WorkshopScreenStrategy {
    descriptor: ScreenDescriptor,
    screen_ids: Arc<Vec<String>>,
    layout: LayoutTree,
}

impl WorkshopScreenStrategy {
    fn new(descriptor: ScreenDescriptor, screen_ids: Arc<Vec<String>>, layout: LayoutTree) -> Self {
        Self {
            descriptor,
            screen_ids,
            layout,
        }
    }
}

impl room_mvp::GlobalZoneStrategy for WorkshopScreenStrategy {
    fn layout(&self) -> LayoutTree {
        self.layout.clone()
    }

    fn register_panels(&mut self, runtime: &mut RoomRuntime, state: &ScreenState) -> Result<()> {
        let visits = state
            .shared_init::<Mutex<usize>, _>(|| Mutex::new(0))
            .map_err(|err| room_mvp::LayoutError::Backend(format!("screen state: {err}")))?;

        runtime.register_plugin_with_priority(
            WorkshopPanel::new(
                self.descriptor.clone(),
                state.navigator(),
                Arc::clone(&self.screen_ids),
                visits,
            ),
            10,
        );
        Ok(())
    }

    fn handle_event(
        &mut self,
        _state: &ScreenState,
        _ctx: &mut RuntimeContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        Ok(room_mvp::EventFlow::Continue)
    }

    fn on_lifecycle(
        &mut self,
        _event: room_mvp::ScreenLifecycleEvent,
        _state: &ScreenState,
    ) -> Result<()> {
        Ok(())
    }
}

struct WorkshopPanel {
    descriptor: ScreenDescriptor,
    navigator: ScreenNavigator,
    screen_ids: Arc<Vec<String>>,
    visits: Arc<Mutex<usize>>,
    shared: Option<Arc<SharedAppState>>,
}

impl WorkshopPanel {
    fn new(
        descriptor: ScreenDescriptor,
        navigator: ScreenNavigator,
        screen_ids: Arc<Vec<String>>,
        visits: Arc<Mutex<usize>>,
    ) -> Self {
        Self {
            descriptor,
            navigator,
            screen_ids,
            visits,
            shared: None,
        }
    }

    fn shared(&self) -> &Arc<SharedAppState> {
        self.shared
            .as_ref()
            .expect("shared app state should be initialised in init")
    }

    fn render(&self, ctx: &mut RuntimeContext<'_>) {
        let visits = *self
            .visits
            .lock()
            .expect("visit counter should remain lockable");
        let shortcuts = self
            .screen_ids
            .iter()
            .enumerate()
            .map(|(idx, id)| format!("{} → {}", idx + 1, id))
            .collect::<Vec<_>>()
            .join(", ");

        let header = format!(
            "Screen: {} — Ctrl+Tab cycles, digits jump ({})",
            self.descriptor.title, shortcuts
        );
        ctx.set_zone(HEADER_ZONE, header);

        let body = match self.descriptor.kind {
            ScreenKind::Dashboard => self.render_dashboard(visits),
            ScreenKind::Settings => self.render_settings(visits),
            ScreenKind::Activity => self.render_activity(visits),
        };
        ctx.set_zone(BODY_ZONE, body);
    }

    fn render_dashboard(&self, visits: usize) -> String {
        let snapshot = self.shared().snapshot();
        let history = snapshot
            .history
            .iter()
            .take(5)
            .enumerate()
            .map(|(idx, line)| format!("  {}. {}", idx + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "Dashboard\n==========\nTheme: {}\nNotifications: {}\nVisits: {}\n\nRecent activity:\n{}\n\nKeys:\n  • 's' to open Settings\n  • 'a' to open Activity\n  • Ctrl+Tab / digits to navigate\n",
            snapshot.theme,
            yes_no(snapshot.notifications),
            visits,
            if history.is_empty() {
                "  (no events yet)".to_string()
            } else {
                history
            }
        )
    }

    fn render_settings(&self, visits: usize) -> String {
        let snapshot = self.shared().snapshot();
        format!(
            "Settings\n========\nTheme: {}\nNotifications: {}\nVisits: {}\n\nKeys:\n  • 't' toggle theme\n  • 'n' toggle notifications\n  • 'd' return to Dashboard\n  • 'a' open Activity\n  • Ctrl+Tab / digits to navigate\n",
            snapshot.theme,
            yes_no(snapshot.notifications),
            visits
        )
    }

    fn render_activity(&self, visits: usize) -> String {
        let snapshot = self.shared().snapshot();
        let history = snapshot
            .history
            .iter()
            .enumerate()
            .map(|(idx, line)| format!("  {}. {}", idx + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "Activity Log\n============\nEntries: {}\nVisits: {}\n\n{}\n\nKeys:\n  • 'c' clear log\n  • 'd' return to Dashboard\n  • Ctrl+Tab / digits to navigate\n",
            snapshot.history.len(),
            visits,
            if history.is_empty() {
                "  (log empty — toggle settings to add entries)".to_string()
            } else {
                history
            }
        )
    }

    fn navigate_if_missing(&self, target: &str) {
        if self.descriptor.id != target {
            self.navigator.request_activation(target.to_string());
        }
    }
}

impl RoomPlugin for WorkshopPanel {
    fn name(&self) -> &str {
        "workshop_multiscreen_panel"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        let shared = ctx
            .shared_init::<SharedAppState, _>(SharedAppState::default)
            .map_err(|err| room_mvp::LayoutError::Backend(format!("shared state: {err}")))?;
        if let Ok(mut visits) = self.visits.lock() {
            *visits += 1;
        }
        shared.record(format!("Screen '{}' activated", self.descriptor.title));
        self.shared = Some(shared);
        self.render(ctx);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if let RuntimeEvent::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Char('1') => {
                    self.navigator.request_activation("dashboard".to_string());
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Char('2') => {
                    self.navigator.request_activation("settings".to_string());
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                KeyCode::Char('3') => {
                    self.navigator.request_activation("activity".to_string());
                    return Ok(room_mvp::EventFlow::Consumed);
                }
                _ => {}
            }
        }

        match event {
            RuntimeEvent::Key(KeyEvent {
                code: KeyCode::Char('d'),
                ..
            }) => {
                self.navigate_if_missing("dashboard");
                return Ok(room_mvp::EventFlow::Consumed);
            }
            RuntimeEvent::Key(KeyEvent {
                code: KeyCode::Char('a'),
                ..
            }) => {
                self.navigate_if_missing("activity");
                return Ok(room_mvp::EventFlow::Consumed);
            }
            RuntimeEvent::Key(KeyEvent {
                code: KeyCode::Char('s'),
                ..
            }) if matches!(self.descriptor.kind, ScreenKind::Dashboard) => {
                self.navigator.request_activation("settings".to_string());
                return Ok(room_mvp::EventFlow::Consumed);
            }
            RuntimeEvent::Key(KeyEvent {
                code: KeyCode::Char('t'),
                ..
            }) if matches!(self.descriptor.kind, ScreenKind::Settings) => {
                self.shared().toggle_theme();
                self.render(ctx);
                return Ok(room_mvp::EventFlow::Consumed);
            }
            RuntimeEvent::Key(KeyEvent {
                code: KeyCode::Char('n'),
                ..
            }) if matches!(self.descriptor.kind, ScreenKind::Settings) => {
                self.shared().toggle_notifications();
                self.render(ctx);
                return Ok(room_mvp::EventFlow::Consumed);
            }
            RuntimeEvent::Key(KeyEvent {
                code: KeyCode::Char('c'),
                ..
            }) if matches!(self.descriptor.kind, ScreenKind::Activity) => {
                self.shared().clear_history();
                self.shared().record("Activity log cleared".to_string());
                self.render(ctx);
                return Ok(room_mvp::EventFlow::Consumed);
            }
            _ => {}
        }

        Ok(room_mvp::EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        self.render(ctx);
        Ok(())
    }
}

#[derive(Default)]
struct SharedAppState {
    config: RwLock<AppConfig>,
    history: Mutex<VecDeque<String>>,
}

impl SharedAppState {
    fn snapshot(&self) -> AppSnapshot {
        let config = self.config.read().expect("config poisoned");
        let theme = config.theme;
        let notifications = config.notifications;
        drop(config);
        let history = self.history.lock().expect("history poisoned");
        AppSnapshot {
            theme,
            notifications,
            history: history.iter().cloned().collect(),
        }
    }

    fn toggle_theme(&self) -> Theme {
        let mut config = self.config.write().expect("config poisoned");
        config.theme = config.theme.toggle();
        let theme = config.theme;
        drop(config);
        self.record(format!("Theme toggled — now {theme}"));
        theme
    }

    fn toggle_notifications(&self) -> bool {
        let mut config = self.config.write().expect("config poisoned");
        config.notifications = !config.notifications;
        let notifications = config.notifications;
        drop(config);
        self.record(format!(
            "Notifications {}",
            if notifications { "enabled" } else { "disabled" }
        ));
        notifications
    }

    fn record(&self, message: impl Into<String>) {
        let mut history = self.history.lock().expect("history poisoned");
        history.push_front(message.into());
        while history.len() > 12 {
            history.pop_back();
        }
    }

    fn clear_history(&self) {
        let mut history = self.history.lock().expect("history poisoned");
        history.clear();
    }
}

struct AppSnapshot {
    theme: Theme,
    notifications: bool,
    history: Vec<String>,
}

#[derive(Clone, Copy)]
enum Theme {
    Light,
    Dark,
}

impl Theme {
    fn toggle(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Light
    }
}

struct AppConfig {
    theme: Theme,
    notifications: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            notifications: true,
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "enabled" } else { "disabled" }
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Theme::Light => write!(f, "light"),
            Theme::Dark => write!(f, "dark"),
        }
    }
}
