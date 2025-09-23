//! Room Workshop: Multi-Screen Navigation
//!
//! Explore the ScreenManager by switching between three screens using both the
//! built-in hotkeys (`Ctrl+Tab`, `Ctrl+Shift+Tab`) and explicit navigation
//! commands (`1`, `2`, `3`). Each screen keeps its own visit counter via
//! `ScreenState`, demonstrating the per-screen shared state namespaces.
//!
//! ```bash
//! cargo run --example workshop_screen_navigation
//! ```

use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use room_mvp::runtime::bundles::{DefaultCliBundleConfig, default_cli_bundle};
use room_mvp::{
    AnsiRenderer, CliDriver, Constraint, Direction, LayoutNode, LayoutTree, Result, RoomPlugin,
    RoomRuntime, RuntimeConfig, RuntimeContext, RuntimeEvent, ScreenDefinition, ScreenManager,
    ScreenMetadata, ScreenNavigator, ScreenState, Size,
};

const HEADER_ZONE: &str = "workshop:navigation.header";
const BODY_ZONE: &str = "workshop:navigation.body";

fn main() -> Result<()> {
    println!("Room Workshop · Multi-Screen Navigation\n");
    println!(
        "Instructions:\n  • Ctrl+Tab / Ctrl+Shift+Tab cycle screens using the default hotkeys.\n  • Press 1/2/3 to jump directly to a screen via the new navigator helper.\n"
    );

    let runtime_layout = screen_layout();
    let renderer = AnsiRenderer::with_default();
    let mut config = RuntimeConfig::default();
    config.default_focus_zone = Some(BODY_ZONE.to_string());

    let mut runtime =
        RoomRuntime::with_config(runtime_layout, renderer, Size::new(72, 12), config)?;

    let descriptors = [
        ScreenDescriptor::new("overview", "Overview", "Screen manager lifecycle & hotkeys"),
        ScreenDescriptor::new(
            "state",
            "State Isolation",
            "Each screen keeps its own visit count",
        ),
        ScreenDescriptor::new(
            "commands",
            "Command Routing",
            "Digits trigger ScreenState navigation",
        ),
    ];
    let screen_ids: Arc<Vec<String>> =
        Arc::new(descriptors.iter().map(|desc| desc.id.to_string()).collect());

    let mut screen_manager = ScreenManager::new();
    for descriptor in descriptors {
        let layout = screen_layout();
        let ids = Arc::clone(&screen_ids);
        screen_manager.register_screen(ScreenDefinition {
            id: descriptor.id.to_string(),
            title: descriptor.title.to_string(),
            factory: Arc::new(move || {
                Box::new(WorkshopScreenStrategy::new(
                    descriptor.id.to_string(),
                    descriptor.title.to_string(),
                    descriptor.blurb.to_string(),
                    layout.clone(),
                    Arc::clone(&ids),
                ))
            }),
            metadata: ScreenMetadata::default(),
        });
    }

    runtime.set_screen_manager(screen_manager);
    runtime.activate_screen("overview")?;
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
}

impl ScreenDescriptor {
    const fn new(id: &'static str, title: &'static str, blurb: &'static str) -> Self {
        Self { id, title, blurb }
    }
}

fn screen_layout() -> LayoutTree {
    LayoutTree::new(LayoutNode {
        id: "workshop:navigation.root".into(),
        direction: Direction::Column,
        constraints: vec![Constraint::Flex(1)],
        children: vec![LayoutNode::leaf(HEADER_ZONE), LayoutNode::leaf(BODY_ZONE)],
        gap: 1,
        padding: 1,
    })
}

struct WorkshopScreenStrategy {
    screen_id: String,
    title: String,
    blurb: String,
    layout: LayoutTree,
    screens: Arc<Vec<String>>,
}

impl WorkshopScreenStrategy {
    fn new(
        screen_id: String,
        title: String,
        blurb: String,
        layout: LayoutTree,
        screens: Arc<Vec<String>>,
    ) -> Self {
        Self {
            screen_id,
            title,
            blurb,
            layout,
            screens,
        }
    }
}

impl room_mvp::GlobalZoneStrategy for WorkshopScreenStrategy {
    fn layout(&self) -> LayoutTree {
        self.layout.clone()
    }

    fn register_panels(&mut self, runtime: &mut RoomRuntime, state: &ScreenState) -> Result<()> {
        let counter = state
            .shared_init::<Mutex<usize>, _>(|| Mutex::new(0))
            .map_err(|err| room_mvp::LayoutError::Backend(format!("screen state: {err}")))?;

        runtime.register_plugin_with_priority(
            WorkshopPanel::new(
                self.screen_id.clone(),
                self.title.clone(),
                self.blurb.clone(),
                state.navigator(),
                Arc::clone(&self.screens),
                counter,
            ),
            10,
        );
        Ok(())
    }

    fn handle_event(
        &mut self,
        state: &ScreenState,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if let RuntimeEvent::Key(KeyEvent {
            code: KeyCode::Char('!'),
            ..
        }) = event
        {
            ctx.set_zone(
                BODY_ZONE,
                format!(
                    "{}\n• Screen ID: {}\n• State handle ID: {}",
                    self.blurb,
                    self.screen_id,
                    state.id()
                ),
            );
            return Ok(room_mvp::EventFlow::Consumed);
        }
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
    screen_id: String,
    title: String,
    blurb: String,
    navigator: ScreenNavigator,
    screens: Arc<Vec<String>>,
    counter: Arc<Mutex<usize>>,
}

impl WorkshopPanel {
    fn new(
        screen_id: String,
        title: String,
        blurb: String,
        navigator: ScreenNavigator,
        screens: Arc<Vec<String>>,
        counter: Arc<Mutex<usize>>,
    ) -> Self {
        Self {
            screen_id,
            title,
            blurb,
            navigator,
            screens,
            counter,
        }
    }

    fn render_body(&self) -> String {
        let visit_count = self.counter.lock().expect("visit counter");
        let shortcut_help = self
            .screens
            .iter()
            .enumerate()
            .map(|(idx, id)| format!("{} → {}", idx + 1, id))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{}\n\nThis screen has been activated {} time(s).\nDigits: {}\nCtrl+Tab cycles forward, Ctrl+Shift+Tab / Ctrl+BackTab cycles backward.",
            self.blurb, *visit_count, shortcut_help
        )
    }
}

impl RoomPlugin for WorkshopPanel {
    fn name(&self) -> &str {
        "workshop_navigation_panel"
    }

    fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        if let Ok(mut visits) = self.counter.lock() {
            *visits += 1;
        }
        ctx.set_zone(
            HEADER_ZONE,
            format!(
                "Screen: {} — press Ctrl+Tab to advance, digits to jump.",
                self.title
            ),
        );
        ctx.set_zone(BODY_ZONE, self.render_body());
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<room_mvp::EventFlow> {
        if let RuntimeEvent::Key(key) = event {
            if let KeyCode::Char(digit @ '1'..='9') = key.code {
                let index = (digit as usize) - ('1' as usize);
                if let Some(target) = self.screens.get(index) {
                    if target != &self.screen_id {
                        self.navigator.request_activation(target.clone());
                        return Ok(room_mvp::EventFlow::Consumed);
                    }
                }
            }
        }

        if let RuntimeEvent::Key(KeyEvent {
            code: KeyCode::Char('i'),
            ..
        }) = event
        {
            ctx.set_zone(BODY_ZONE, self.render_body());
            return Ok(room_mvp::EventFlow::Consumed);
        }

        Ok(room_mvp::EventFlow::Continue)
    }

    fn before_render(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
        ctx.set_zone(BODY_ZONE, self.render_body());
        Ok(())
    }
}
