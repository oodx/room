use std::collections::HashMap;
use std::sync::Arc;

use crate::{LayoutTree, Result, RoomRuntime};

use super::{EventFlow, RuntimeContext, RuntimeEvent};

/// Factory type responsible for creating a fresh [`GlobalZoneStrategy`] instance.
pub type ScreenFactory = Arc<dyn Fn() -> Box<dyn GlobalZoneStrategy> + Send + Sync>;

/// Declarative screen definition registered with the [`ScreenManager`].
pub struct ScreenDefinition {
    pub id: String,
    pub title: String,
    pub factory: ScreenFactory,
    pub metadata: ScreenMetadata,
}

impl ScreenDefinition {
    pub fn new(id: impl Into<String>, title: impl Into<String>, factory: ScreenFactory) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            factory,
            metadata: ScreenMetadata::default(),
        }
    }
}

/// Optional metadata carried alongside a screen definition.
#[derive(Default, Clone, Debug)]
pub struct ScreenMetadata {
    pub description: Option<String>,
    pub shortcuts: Vec<String>,
}

/// Minimal global zone strategy that preserves legacy single-screen behaviour.
///
/// This strategy simply reapplies the provided layout and forwards all events to
/// the legacy plugin pipeline, enabling existing demos to opt into the
/// `ScreenManager` without restructuring their panel registrations yet.
pub struct LegacyScreenStrategy {
    layout: LayoutTree,
}

impl LegacyScreenStrategy {
    pub fn new(layout: LayoutTree) -> Self {
        Self { layout }
    }
}

/// Lifecycle events emitted around screen activation/deactivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenLifecycleEvent {
    WillAppear,
    DidAppear,
    WillDisappear,
    DidDisappear,
}

/// Contract implemented by global zone strategies that back individual screens.
pub trait GlobalZoneStrategy: Send {
    fn layout(&self) -> LayoutTree;
    fn register_panels(&mut self, runtime: &mut RoomRuntime) -> Result<()>;
    fn handle_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow>;
    fn on_lifecycle(&mut self, event: ScreenLifecycleEvent) -> Result<()>;
}

impl GlobalZoneStrategy for LegacyScreenStrategy {
    fn layout(&self) -> LayoutTree {
        self.layout.clone()
    }

    fn register_panels(&mut self, _runtime: &mut RoomRuntime) -> Result<()> {
        Ok(())
    }

    fn handle_event(
        &mut self,
        _ctx: &mut RuntimeContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        Ok(EventFlow::Continue)
    }

    fn on_lifecycle(&mut self, _event: ScreenLifecycleEvent) -> Result<()> {
        Ok(())
    }
}

struct ActiveScreen {
    id: String,
    strategy: Box<dyn GlobalZoneStrategy>,
}

/// Handle returned when a screen is being activated. Callers are expected to
/// install the layout contained within before invoking [`ScreenManager::finish_activation`].
pub struct ScreenActivation {
    id: String,
    layout: LayoutTree,
    strategy: Box<dyn GlobalZoneStrategy>,
}

impl ScreenActivation {
    pub fn layout(&self) -> &LayoutTree {
        &self.layout
    }

    pub fn into_parts(self) -> (String, LayoutTree, Box<dyn GlobalZoneStrategy>) {
        (self.id, self.layout, self.strategy)
    }
}

/// Coordinates screen registration, activation, and event routing.
pub struct ScreenManager {
    screens: HashMap<String, ScreenDefinition>,
    active: Option<ActiveScreen>,
}

impl Default for ScreenManager {
    fn default() -> Self {
        Self {
            screens: HashMap::new(),
            active: None,
        }
    }
}

impl ScreenManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_screen(&mut self, definition: ScreenDefinition) {
        self.screens.insert(definition.id.clone(), definition);
    }

    pub fn activate(&mut self, screen_id: &str) -> Result<ScreenActivation> {
        let definition = self.screens.get(screen_id).ok_or_else(|| {
            crate::LayoutError::Backend(format!("screen '{screen_id}' not found"))
        })?;

        if let Some(active) = self.active.as_mut() {
            active
                .strategy
                .on_lifecycle(ScreenLifecycleEvent::WillDisappear)?;
        }

        let mut strategy = (definition.factory)();
        strategy.on_lifecycle(ScreenLifecycleEvent::WillAppear)?;
        let layout = strategy.layout();

        Ok(ScreenActivation {
            id: definition.id.clone(),
            layout,
            strategy,
        })
    }

    pub fn finish_activation(
        &mut self,
        runtime: &mut RoomRuntime,
        activation: ScreenActivation,
    ) -> Result<()> {
        let (id, layout, mut strategy) = activation.into_parts();
        runtime.apply_screen_layout(layout)?;
        strategy.register_panels(runtime)?;
        strategy.on_lifecycle(ScreenLifecycleEvent::DidAppear)?;

        if let Some(mut previous) = self.active.take() {
            previous
                .strategy
                .on_lifecycle(ScreenLifecycleEvent::DidDisappear)?;
        }

        self.active = Some(ActiveScreen { id, strategy });

        Ok(())
    }

    pub fn handle_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        if let Some(active) = self.active.as_mut() {
            active.strategy.handle_event(ctx, event)
        } else {
            Ok(EventFlow::Continue)
        }
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active.as_ref().map(|screen| screen.id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, RuntimeConfig, Size};
    use std::sync::Mutex;

    #[test]
    fn legacy_strategy_is_passthrough() {
        let layout = LayoutTree::new(LayoutNode {
            id: "root".into(),
            direction: Direction::Column,
            constraints: vec![Constraint::Flex(1)],
            children: vec![LayoutNode::leaf("zone")],
            gap: 0,
            padding: 0,
        });

        let mut strategy = LegacyScreenStrategy::new(layout.clone());
        let cloned = strategy.layout();
        assert_eq!(cloned.root.id, layout.root.id);
        let mut runtime = RoomRuntime::with_config(
            layout,
            AnsiRenderer::with_default(),
            Size::new(4, 4),
            RuntimeConfig::default(),
        )
        .expect("runtime");

        strategy
            .register_panels(&mut runtime)
            .expect("register panels");
        strategy
            .on_lifecycle(ScreenLifecycleEvent::WillAppear)
            .expect("will appear");
        strategy
            .on_lifecycle(ScreenLifecycleEvent::DidAppear)
            .expect("did appear");
    }

    #[derive(Default)]
    struct TestState {
        lifecycles: Vec<ScreenLifecycleEvent>,
        panels_registered: bool,
    }

    struct TestStrategy {
        state: Arc<Mutex<TestState>>,
    }

    impl TestStrategy {
        fn new(state: Arc<Mutex<TestState>>) -> Self {
            Self { state }
        }
    }

    impl GlobalZoneStrategy for TestStrategy {
        fn layout(&self) -> LayoutTree {
            LayoutTree::new(LayoutNode {
                id: "root".into(),
                direction: Direction::Column,
                constraints: vec![Constraint::Flex(1)],
                children: vec![LayoutNode::leaf("status")],
                gap: 0,
                padding: 0,
            })
        }

        fn register_panels(&mut self, _runtime: &mut RoomRuntime) -> Result<()> {
            let mut state = self.state.lock().unwrap();
            state.panels_registered = true;
            Ok(())
        }

        fn handle_event(
            &mut self,
            _ctx: &mut RuntimeContext<'_>,
            _event: &RuntimeEvent,
        ) -> Result<EventFlow> {
            Ok(EventFlow::Continue)
        }

        fn on_lifecycle(&mut self, event: ScreenLifecycleEvent) -> Result<()> {
            self.state.lock().unwrap().lifecycles.push(event);
            Ok(())
        }
    }

    #[test]
    fn activate_and_finish_records_lifecycle() {
        let mut manager = ScreenManager::new();
        let state = Arc::new(Mutex::new(TestState::default()));
        let state_clone = state.clone();
        manager.register_screen(ScreenDefinition {
            id: "main".into(),
            title: "Main".into(),
            factory: Arc::new(move || Box::new(TestStrategy::new(state_clone.clone()))),
            metadata: ScreenMetadata::default(),
        });

        let activation = manager.activate("main").expect("activation");
        // Layout should be the one provided by the strategy; we expect a single-root tree.
        let _ = activation.layout();

        let base_layout = LayoutTree::new(LayoutNode {
            id: "root".into(),
            direction: Direction::Column,
            constraints: vec![Constraint::Flex(1)],
            children: vec![LayoutNode::leaf("status")],
            gap: 0,
            padding: 0,
        });
        let renderer = AnsiRenderer::with_default();
        let mut runtime = RoomRuntime::with_config(
            base_layout,
            renderer,
            Size::new(10, 4),
            RuntimeConfig::default(),
        )
        .expect("runtime");

        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish activation");

        let state = state.lock().unwrap();
        assert_eq!(
            state.lifecycles,
            vec![
                ScreenLifecycleEvent::WillAppear,
                ScreenLifecycleEvent::DidAppear
            ]
        );
        assert!(state.panels_registered);
        assert_eq!(manager.active_id(), Some("main"));
    }
}
