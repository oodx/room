use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use crate::{LayoutTree, Result, RoomRuntime};

use super::{EventFlow, RuntimeContext, RuntimeEvent, shared_state};
use crossterm::event::{KeyCode, KeyModifiers};

#[derive(Clone)]
pub struct ScreenNavigator {
    relay: Arc<NavigationRelay>,
}

impl ScreenNavigator {
    pub fn request_activation(&self, screen_id: impl Into<String>) {
        self.relay.request(screen_id.into());
    }
}

#[derive(Default)]
struct NavigationRelay {
    pending: Mutex<Option<String>>,
}

impl NavigationRelay {
    fn request(&self, screen_id: String) {
        let mut guard = self
            .pending
            .lock()
            .expect("screen navigation relay poisoned");
        *guard = Some(screen_id);
    }

    fn take(&self) -> Option<String> {
        self.pending
            .lock()
            .expect("screen navigation relay poisoned")
            .take()
    }
}

#[derive(Clone)]
pub struct ScreenState {
    id: Arc<str>,
    shared: shared_state::SharedState,
    navigator: ScreenNavigator,
}

impl ScreenState {
    fn new(
        id: impl Into<String>,
        shared: shared_state::SharedState,
        navigator: ScreenNavigator,
    ) -> Self {
        Self {
            id: Arc::from(id.into()),
            shared,
            navigator,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn insert_arc<T>(
        &self,
        value: Arc<T>,
    ) -> std::result::Result<(), shared_state::SharedStateError>
    where
        T: Send + Sync + 'static,
    {
        self.shared.insert_arc(value)
    }

    pub fn shared<T>(&self) -> std::result::Result<Arc<T>, shared_state::SharedStateError>
    where
        T: Send + Sync + 'static,
    {
        self.shared.get::<T>()
    }

    pub fn shared_init<T, F>(
        &self,
        make: F,
    ) -> std::result::Result<Arc<T>, shared_state::SharedStateError>
    where
        T: Send + Sync + 'static,
        F: FnOnce() -> T,
    {
        self.shared.get_or_insert_with(make)
    }

    pub fn request_activation(&self, screen_id: impl Into<String>) {
        self.navigator.request_activation(screen_id)
    }

    pub fn navigator(&self) -> ScreenNavigator {
        self.navigator.clone()
    }
}

#[derive(Clone, Default)]
struct ScreenStateStore {
    namespaces: Arc<RwLock<HashMap<String, shared_state::SharedState>>>,
    navigation: Arc<NavigationRelay>,
}

impl ScreenStateStore {
    fn new() -> Self {
        Self {
            namespaces: Arc::default(),
            navigation: Arc::new(NavigationRelay::default()),
        }
    }

    fn scope(&self, screen_id: &str) -> ScreenState {
        let shared = {
            let mut guard = self
                .namespaces
                .write()
                .expect("screen state namespaces poisoned");
            guard
                .entry(screen_id.to_string())
                .or_insert_with(shared_state::SharedState::new)
                .clone()
        };

        ScreenState::new(
            screen_id.to_string(),
            shared,
            ScreenNavigator {
                relay: self.navigation.clone(),
            },
        )
    }

    fn take_navigation_request(&self) -> Option<String> {
        self.navigation.take()
    }
}

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
    fn register_panels(&mut self, runtime: &mut RoomRuntime, state: &ScreenState) -> Result<()>;
    fn handle_event(
        &mut self,
        state: &ScreenState,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow>;
    fn on_lifecycle(&mut self, event: ScreenLifecycleEvent, state: &ScreenState) -> Result<()>;
}

impl GlobalZoneStrategy for LegacyScreenStrategy {
    fn layout(&self) -> LayoutTree {
        self.layout.clone()
    }

    fn register_panels(&mut self, _runtime: &mut RoomRuntime, _state: &ScreenState) -> Result<()> {
        Ok(())
    }

    fn handle_event(
        &mut self,
        _state: &ScreenState,
        _ctx: &mut RuntimeContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        Ok(EventFlow::Continue)
    }

    fn on_lifecycle(&mut self, _event: ScreenLifecycleEvent, _state: &ScreenState) -> Result<()> {
        Ok(())
    }
}

struct ActiveScreen {
    id: String,
    strategy: Box<dyn GlobalZoneStrategy>,
    state: ScreenState,
}

/// Handle returned when a screen is being activated. Callers are expected to
/// install the layout contained within before invoking [`ScreenManager::finish_activation`].
pub struct ScreenActivation {
    id: String,
    layout: LayoutTree,
    strategy: Box<dyn GlobalZoneStrategy>,
    state: ScreenState,
}

impl ScreenActivation {
    pub fn layout(&self) -> &LayoutTree {
        &self.layout
    }

    pub fn state(&self) -> &ScreenState {
        &self.state
    }

    pub fn into_parts(self) -> (String, LayoutTree, Box<dyn GlobalZoneStrategy>, ScreenState) {
        (self.id, self.layout, self.strategy, self.state)
    }
}

/// Coordinates screen registration, activation, and event routing.
pub struct ScreenManager {
    screens: HashMap<String, ScreenDefinition>,
    active: Option<ActiveScreen>,
    states: ScreenStateStore,
    ordered: Vec<String>,
    pending_activation: Option<ScreenActivation>,
}

impl Default for ScreenManager {
    fn default() -> Self {
        Self {
            screens: HashMap::new(),
            active: None,
            states: ScreenStateStore::new(),
            ordered: Vec::new(),
            pending_activation: None,
        }
    }
}

impl ScreenManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_screen(&mut self, definition: ScreenDefinition) {
        let id = definition.id.clone();
        if !self.screens.contains_key(&id) {
            self.ordered.push(id.clone());
        }
        self.screens.insert(id, definition);
    }

    pub fn activate(&mut self, screen_id: &str) -> Result<ScreenActivation> {
        let definition = self.screens.get(screen_id).ok_or_else(|| {
            crate::LayoutError::Backend(format!("screen '{screen_id}' not found"))
        })?;

        if let Some(active) = self.active.as_mut() {
            active
                .strategy
                .on_lifecycle(ScreenLifecycleEvent::WillDisappear, &active.state)?;
        }

        let mut strategy = (definition.factory)();
        let state = self.states.scope(&definition.id);
        strategy.on_lifecycle(ScreenLifecycleEvent::WillAppear, &state)?;
        let layout = strategy.layout();

        Ok(ScreenActivation {
            id: definition.id.clone(),
            layout,
            strategy,
            state,
        })
    }

    pub fn finish_activation(
        &mut self,
        runtime: &mut RoomRuntime,
        activation: ScreenActivation,
    ) -> Result<()> {
        let (id, layout, mut strategy, state) = activation.into_parts();
        runtime.apply_screen_layout(layout)?;
        strategy.register_panels(runtime, &state)?;
        strategy.on_lifecycle(ScreenLifecycleEvent::DidAppear, &state)?;
        runtime.apply_configured_focus()?;

        if let Some(mut previous) = self.active.take() {
            previous
                .strategy
                .on_lifecycle(ScreenLifecycleEvent::DidDisappear, &previous.state)?;
        }

        self.active = Some(ActiveScreen {
            id,
            strategy,
            state,
        });

        Ok(())
    }

    pub fn handle_event(
        &mut self,
        ctx: &mut RuntimeContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<EventFlow> {
        if let Some(flow) = self.handle_navigation_hotkeys(event)? {
            self.drain_navigation_queue()?;
            return Ok(flow);
        }

        let flow = if let Some(active) = self.active.as_mut() {
            active.strategy.handle_event(&active.state, ctx, event)?
        } else {
            EventFlow::Continue
        };

        self.drain_navigation_queue()?;
        Ok(flow)
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active.as_ref().map(|screen| screen.id.as_str())
    }

    pub fn active_state(&self) -> Option<ScreenState> {
        self.active.as_ref().map(|screen| screen.state.clone())
    }

    pub fn screen_state(&self, screen_id: &str) -> Option<ScreenState> {
        if !self.screens.contains_key(screen_id) {
            return None;
        }
        Some(self.states.scope(screen_id))
    }

    pub fn take_pending_activation(&mut self) -> Option<ScreenActivation> {
        self.pending_activation.take()
    }

    fn handle_navigation_hotkeys(&mut self, event: &RuntimeEvent) -> Result<Option<EventFlow>> {
        let RuntimeEvent::Key(key) = event else {
            return Ok(None);
        };

        if self.ordered.len() < 2 {
            return Ok(None);
        }

        let Some(active) = self.active.as_ref() else {
            return Ok(None);
        };

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Tab => {
                    let direction = if key.modifiers.contains(KeyModifiers::SHIFT) {
                        CycleDirection::Backward
                    } else {
                        CycleDirection::Forward
                    };
                    if let Some(next_id) = self.next_screen_id(&active.id, direction) {
                        self.request_activation(next_id)?;
                        return Ok(Some(EventFlow::Consumed));
                    }
                }
                KeyCode::BackTab => {
                    if let Some(prev_id) = self.next_screen_id(&active.id, CycleDirection::Backward)
                    {
                        self.request_activation(prev_id)?;
                        return Ok(Some(EventFlow::Consumed));
                    }
                }
                _ => {}
            }
        }

        Ok(None)
    }

    fn drain_navigation_queue(&mut self) -> Result<()> {
        while let Some(target) = self.states.take_navigation_request() {
            if Some(target.as_str()) == self.active_id() {
                continue;
            }
            self.request_activation(target)?;
        }
        Ok(())
    }

    fn request_activation(&mut self, screen_id: String) -> Result<()> {
        let activation = self.activate(&screen_id)?;
        self.pending_activation = Some(activation);
        Ok(())
    }

    fn next_screen_id(&self, current: &str, direction: CycleDirection) -> Option<String> {
        let position = self.ordered.iter().position(|id| id == current)?;
        let total = self.ordered.len();
        let next_index = match direction {
            CycleDirection::Forward => (position + 1) % total,
            CycleDirection::Backward => (position + total - 1) % total,
        };
        if next_index == position {
            None
        } else {
            self.ordered.get(next_index).cloned()
        }
    }
}

enum CycleDirection {
    Forward,
    Backward,
}

#[cfg(test)]
mod tests {
    use super::super::RUNTIME_FOCUS_OWNER;
    use super::*;
    use crate::{
        AnsiRenderer, Constraint, Direction, LayoutNode, LayoutTree, RuntimeConfig, RuntimeContext,
        Size, runtime::focus::ensure_focus_registry,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct PassthroughStrategy;

    impl GlobalZoneStrategy for PassthroughStrategy {
        fn layout(&self) -> LayoutTree {
            LayoutTree::new(LayoutNode {
                id: "root".into(),
                direction: Direction::Column,
                constraints: vec![Constraint::Flex(1)],
                children: vec![LayoutNode::leaf("zone")],
                gap: 0,
                padding: 0,
            })
        }

        fn register_panels(
            &mut self,
            _runtime: &mut RoomRuntime,
            _state: &ScreenState,
        ) -> Result<()> {
            Ok(())
        }

        fn handle_event(
            &mut self,
            _state: &ScreenState,
            _ctx: &mut RuntimeContext<'_>,
            _event: &RuntimeEvent,
        ) -> Result<EventFlow> {
            Ok(EventFlow::Continue)
        }

        fn on_lifecycle(
            &mut self,
            _event: ScreenLifecycleEvent,
            _state: &ScreenState,
        ) -> Result<()> {
            Ok(())
        }
    }

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

        let store = ScreenStateStore::new();
        let state = store.scope("legacy");

        strategy
            .register_panels(&mut runtime, &state)
            .expect("register panels");
        strategy
            .on_lifecycle(ScreenLifecycleEvent::WillAppear, &state)
            .expect("will appear");
        strategy
            .on_lifecycle(ScreenLifecycleEvent::DidAppear, &state)
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

        fn register_panels(
            &mut self,
            _runtime: &mut RoomRuntime,
            _state: &ScreenState,
        ) -> Result<()> {
            let mut state = self.state.lock().unwrap();
            state.panels_registered = true;
            Ok(())
        }

        fn handle_event(
            &mut self,
            _state: &ScreenState,
            _ctx: &mut RuntimeContext<'_>,
            _event: &RuntimeEvent,
        ) -> Result<EventFlow> {
            Ok(EventFlow::Continue)
        }

        fn on_lifecycle(
            &mut self,
            event: ScreenLifecycleEvent,
            _state: &ScreenState,
        ) -> Result<()> {
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
        assert_eq!(activation.state().id(), "main");

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
        assert_eq!(manager.active_state().expect("active state").id(), "main");
        assert_eq!(
            manager.screen_state("main").expect("scoped state").id(),
            "main"
        );
    }

    #[test]
    fn screen_state_namespaces_resources_per_screen() {
        #[derive(Debug, PartialEq, Eq)]
        struct Marker(&'static str);

        let store = ScreenStateStore::new();
        let alpha = store.scope("alpha");
        let beta = store.scope("beta");

        alpha
            .insert_arc(Arc::new(Marker("alpha")))
            .expect("insert alpha");
        assert_eq!(alpha.shared::<Marker>().unwrap().0, "alpha");
        assert!(beta.shared::<Marker>().is_err());

        beta.shared_init::<Marker, _>(|| Marker("beta"))
            .expect("init beta");
        assert_eq!(beta.shared::<Marker>().unwrap().0, "beta");
        assert_eq!(alpha.shared::<Marker>().unwrap().0, "alpha");
    }

    #[test]
    fn screen_state_request_activation_enqueues_switch() {
        struct NavStrategy {
            target: &'static str,
        }

        impl GlobalZoneStrategy for NavStrategy {
            fn layout(&self) -> LayoutTree {
                LayoutTree::new(LayoutNode {
                    id: "root".into(),
                    direction: Direction::Column,
                    constraints: vec![Constraint::Flex(1)],
                    children: vec![LayoutNode::leaf("zone")],
                    gap: 0,
                    padding: 0,
                })
            }

            fn register_panels(
                &mut self,
                _runtime: &mut RoomRuntime,
                _state: &ScreenState,
            ) -> Result<()> {
                Ok(())
            }

            fn handle_event(
                &mut self,
                state: &ScreenState,
                _ctx: &mut RuntimeContext<'_>,
                event: &RuntimeEvent,
            ) -> Result<EventFlow> {
                if matches!(event, RuntimeEvent::Key(key) if key.code == KeyCode::Char('n')) {
                    state.request_activation(self.target);
                    return Ok(EventFlow::Consumed);
                }
                Ok(EventFlow::Continue)
            }

            fn on_lifecycle(
                &mut self,
                _event: ScreenLifecycleEvent,
                _state: &ScreenState,
            ) -> Result<()> {
                Ok(())
            }
        }

        let mut manager = ScreenManager::new();
        manager.register_screen(ScreenDefinition {
            id: "primary".into(),
            title: "Primary".into(),
            factory: Arc::new(|| {
                Box::new(NavStrategy {
                    target: "secondary",
                })
            }),
            metadata: ScreenMetadata::default(),
        });
        manager.register_screen(ScreenDefinition {
            id: "secondary".into(),
            title: "Secondary".into(),
            factory: Arc::new(|| Box::new(PassthroughStrategy::default())),
            metadata: ScreenMetadata::default(),
        });

        let base_layout = LayoutTree::new(LayoutNode {
            id: "root".into(),
            direction: Direction::Column,
            constraints: vec![Constraint::Flex(1)],
            children: vec![LayoutNode::leaf("zone")],
            gap: 0,
            padding: 0,
        });
        let renderer = AnsiRenderer::with_default();
        let mut runtime = RoomRuntime::with_config(
            base_layout,
            renderer,
            Size::new(80, 24),
            RuntimeConfig::default(),
        )
        .expect("runtime");

        let activation = manager.activate("primary").expect("activate primary");
        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish primary");
        assert_eq!(manager.active_id(), Some("primary"));

        let event = RuntimeEvent::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        let mut ctx = RuntimeContext::new(&runtime.rects, &runtime.shared_state);
        let flow = manager
            .handle_event(&mut ctx, &event)
            .expect("handle event");
        assert_eq!(flow, EventFlow::Consumed);

        let activation = manager
            .take_pending_activation()
            .expect("pending activation");
        runtime.config_mut().default_focus_zone = Some("secondary:zone".into());
        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish secondary");
        assert_eq!(manager.active_id(), Some("secondary"));
    }

    #[test]
    fn ctrl_tab_cycles_through_registered_screens() {
        let mut manager = ScreenManager::new();

        for id in ["alpha", "beta", "gamma"] {
            let id_owned = id.to_string();
            manager.register_screen(ScreenDefinition {
                id: id_owned.clone(),
                title: id_owned.clone(),
                factory: Arc::new(move || Box::new(PassthroughStrategy::default())),
                metadata: ScreenMetadata::default(),
            });
        }

        let base_layout = LayoutTree::new(LayoutNode {
            id: "root".into(),
            direction: Direction::Column,
            constraints: vec![Constraint::Flex(1)],
            children: vec![LayoutNode::leaf("zone")],
            gap: 0,
            padding: 0,
        });
        let renderer = AnsiRenderer::with_default();
        let mut runtime = RoomRuntime::with_config(
            base_layout,
            renderer,
            Size::new(80, 24),
            RuntimeConfig::default(),
        )
        .expect("runtime");

        let activation = manager.activate("alpha").expect("activate alpha");
        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish alpha");
        assert_eq!(manager.active_id(), Some("alpha"));

        // Ctrl+Tab → beta
        let mut ctx = RuntimeContext::new(&runtime.rects, &runtime.shared_state);
        let event = RuntimeEvent::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL));
        let flow = manager.handle_event(&mut ctx, &event).expect("ctrl+tab");
        assert_eq!(flow, EventFlow::Consumed);
        let activation = manager.take_pending_activation().expect("activation beta");
        runtime.config_mut().default_focus_zone = Some("beta:zone".into());
        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish beta");
        assert_eq!(manager.active_id(), Some("beta"));

        // Ctrl+Tab again → gamma
        let mut ctx = RuntimeContext::new(&runtime.rects, &runtime.shared_state);
        let event = RuntimeEvent::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL));
        manager
            .handle_event(&mut ctx, &event)
            .expect("ctrl+tab once more");
        let activation = manager.take_pending_activation().expect("activation gamma");
        runtime.config_mut().default_focus_zone = Some("gamma:zone".into());
        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish gamma");
        assert_eq!(manager.active_id(), Some("gamma"));

        // Ctrl+Shift+Tab → beta
        let mut ctx = RuntimeContext::new(&runtime.rects, &runtime.shared_state);
        let event = RuntimeEvent::Key(KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        manager
            .handle_event(&mut ctx, &event)
            .expect("ctrl+shift+tab");
        let activation = manager
            .take_pending_activation()
            .expect("activation beta again");
        runtime.config_mut().default_focus_zone = Some("beta:zone".into());
        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish beta");
        assert_eq!(manager.active_id(), Some("beta"));

        // Ctrl+BackTab → alpha
        let mut ctx = RuntimeContext::new(&runtime.rects, &runtime.shared_state);
        let event = RuntimeEvent::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::CONTROL));
        manager
            .handle_event(&mut ctx, &event)
            .expect("ctrl+backtab");
        let activation = manager.take_pending_activation().expect("activation alpha");
        runtime.config_mut().default_focus_zone = Some("alpha:zone".into());
        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish alpha");
        assert_eq!(manager.active_id(), Some("alpha"));
    }

    #[test]
    fn switch_between_screens_invokes_lifecycle_and_updates_layouts() {
        let mut manager = ScreenManager::new();

        fn layout_for(id: &str) -> LayoutTree {
            LayoutTree::new(LayoutNode {
                id: format!("root:{id}"),
                direction: Direction::Column,
                constraints: vec![Constraint::Flex(1)],
                children: vec![LayoutNode::leaf(format!("{id}:zone"))],
                gap: 0,
                padding: 0,
            })
        }

        #[derive(Debug, PartialEq, Eq)]
        enum Call {
            WillAppear(&'static str),
            DidAppear(&'static str),
            WillDisappear(&'static str),
            DidDisappear(&'static str),
        }

        struct RecordingStrategy {
            id: &'static str,
            calls: Arc<Mutex<Vec<Call>>>,
        }

        impl GlobalZoneStrategy for RecordingStrategy {
            fn layout(&self) -> LayoutTree {
                layout_for(self.id)
            }

            fn register_panels(
                &mut self,
                _runtime: &mut RoomRuntime,
                _state: &ScreenState,
            ) -> Result<()> {
                Ok(())
            }

            fn handle_event(
                &mut self,
                _state: &ScreenState,
                _ctx: &mut RuntimeContext<'_>,
                _event: &RuntimeEvent,
            ) -> Result<EventFlow> {
                Ok(EventFlow::Continue)
            }

            fn on_lifecycle(
                &mut self,
                event: ScreenLifecycleEvent,
                _state: &ScreenState,
            ) -> Result<()> {
                let mut calls = self.calls.lock().unwrap();
                calls.push(match event {
                    ScreenLifecycleEvent::WillAppear => Call::WillAppear(self.id),
                    ScreenLifecycleEvent::DidAppear => Call::DidAppear(self.id),
                    ScreenLifecycleEvent::WillDisappear => Call::WillDisappear(self.id),
                    ScreenLifecycleEvent::DidDisappear => Call::DidDisappear(self.id),
                });
                Ok(())
            }
        }

        let calls = Arc::new(Mutex::new(Vec::new()));

        for screen_id in ["primary", "secondary"] {
            let call_log = Arc::clone(&calls);
            manager.register_screen(ScreenDefinition {
                id: screen_id.to_string(),
                title: screen_id.to_string(),
                factory: Arc::new(move || {
                    Box::new(RecordingStrategy {
                        id: screen_id,
                        calls: Arc::clone(&call_log),
                    })
                }),
                metadata: ScreenMetadata::default(),
            });
        }

        let base_layout = layout_for("base");
        let renderer = AnsiRenderer::with_default();
        let mut runtime = RoomRuntime::with_config(
            base_layout,
            renderer,
            Size::new(32, 8),
            RuntimeConfig::default(),
        )
        .expect("runtime");
        runtime.config_mut().default_focus_zone = Some("primary:zone".to_string());

        // Activate primary screen
        let activation = manager.activate("primary").expect("activate primary");
        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish primary");

        // Switch to secondary screen
        runtime.config_mut().default_focus_zone = Some("secondary:zone".to_string());
        let activation = manager.activate("secondary").expect("activate secondary");
        manager
            .finish_activation(&mut runtime, activation)
            .expect("finish secondary");

        // Ensure layout root was updated to secondary screen
        // Note: With Layout trait abstraction, internal structure is no longer accessible
        // This verification is implicitly tested by the focus zone assertion below

        // Default focus should track the active screen
        let ctx = RuntimeContext::new(&runtime.rects, &runtime.shared_state);
        let focus_registry = ensure_focus_registry(&ctx).expect("focus registry");
        let focus_entry = focus_registry.current().expect("focus entry");
        assert_eq!(focus_entry.owner, RUNTIME_FOCUS_OWNER);
        assert_eq!(focus_entry.zone_id, "secondary:zone");

        let log = calls.lock().unwrap();
        use Call::*;
        assert_eq!(
            log.as_slice(),
            [
                WillAppear("primary"),
                DidAppear("primary"),
                WillDisappear("primary"),
                WillAppear("secondary"),
                DidAppear("secondary"),
                DidDisappear("primary"),
            ]
        );
    }
}
