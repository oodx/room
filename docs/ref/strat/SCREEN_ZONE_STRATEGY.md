# Screen & Global Zone Strategy

This note captures the first-pass architecture for introducing **Screens** and
**Global Zones** into the Room runtime so applications can present multiple
full-screen layouts that share the same runtime instance.

## Core Concepts

| Concept      | Responsibilities |
|--------------|------------------|
| **Screen**   | Declarative description of a full-screen experience. Identifies the global zone strategy to apply, provides a layout config, and exposes optional metadata (title, navigation hints, etc.). Screens do not own event logic directly. |
| **Global Zone** | A runtime plugin that acts as a composition root for the screen. It owns the layout tree, registers child panels, and mediates events between the runtime and panels. Different strategies (chat, browser, dashboard) implement this behaviour. |
| **Panel**    | Existing building block (zone + plugin) that renders a portion of the UI. Panels remain unaware of screens; the global zone decides which panels to instantiate and where they live. |
| **Screen Manager** | Runtime extension that tracks the active screen, handles navigation (switching), and orchestrates lifecycle events (`ScreenWillAppear`, `ScreenDidDisappear`). Responsible for swapping the renderer/registry state when screens change. |

## Flow Overview

### Activation & Event Lifecycle

```
┌─────────────────────────────┐
│  Application / App Builder  │
└──────────────┬──────────────┘
               │ register screens
┌──────────────▼──────────────┐
│       ScreenManager         │
└──────────────┬──────────────┘
               │ activate(initial)
┌──────────────▼──────────────┐     bootstrap()      ┌──────────────────────┐
│    GlobalZoneStrategy       │◄────────────────────►│     RoomRuntime      │
└──────────────┬──────────────┘                      └──────────┬───────────┘
               │ register_panels()                                │ render()
               ▼                                                 ▼
        Panels / Plugins  ◄────────────── events ────────────── Runtime events

Legend: solid lines = control flow, dashed = event propagation.
```

1. Application registers one or more `ScreenDefinition`s with the `ScreenManager`.
2. At runtime start, the manager activates the initial screen:
   - Instantiates the configured `GlobalZoneStrategy`.
   - Hands its layout tree to the runtime (swapping registry layout + renderer buffers).
   - Registers child panels/plugins via the global zone.
   - Emits lifecycle callbacks in order: `WillAppear` → `DidAppear`.
3. Runtime events flow:
   - `RoomRuntime` → `ScreenManager` → active `GlobalZone`.
   - The global zone may handle events, translate them, or delegate to panels.
   - Panels can bubble responses back up (navigation, notifications) via a shared protocol.
4. Switching screens:
   - Manager emits `WillDisappear` to current screen, flushes dirty state, and performs panel teardown if required.
   - Activates the new screen (steps from #2) and emits `DidDisappear` for the old screen once swap completes.
   - Renderer performs a single render batch for the new layout to maintain first-frame smoothness.

## Traits & APIs (MVP)

```rust
pub struct ScreenDefinition {
    pub id: String,
    pub title: String,
    pub strategy: Box<dyn GlobalZoneStrategy>,
    pub metadata: ScreenMetadata,
}

pub trait GlobalZoneStrategy {
    fn layout(&self) -> LayoutTree;
    fn register_panels(
        &mut self,
        runtime: &mut RoomRuntime,
        state: &ScreenState,
    ) -> Result<()>;
    fn handle_event(
        &mut self,
        state: &ScreenState,
        ctx: &mut RuntimeContext,
        event: &RuntimeEvent,
    ) -> Result<EventFlow>;
    fn on_lifecycle(
        &mut self,
        event: ScreenLifecycleEvent,
        state: &ScreenState,
    ) -> Result<()>;
}

pub struct ScreenManager {
    pub fn register_screen(&mut self, definition: ScreenDefinition);
    pub fn activate(&mut self, screen_id: &str) -> Result<ScreenActivation>;
    pub fn finish_activation(
        &mut self,
        runtime: &mut RoomRuntime,
        activation: ScreenActivation,
    ) -> Result<()>;
    pub fn handle_event(&mut self, ctx: &mut RuntimeContext, event: &RuntimeEvent) -> Result<EventFlow>;
    pub fn active_state(&self) -> Option<ScreenState>;
    pub fn screen_state(&self, screen_id: &str) -> Option<ScreenState>;
    pub fn take_pending_activation(&mut self) -> Option<ScreenActivation>;
}
```

Notes:
- `ScreenManager` integrates with `RoomRuntime` (either via a helper plugin or by extending the runtime API) so the runtime loop calls `handle_event` after it constructs the per-frame `RuntimeContext`.
- `ScreenLifecycleEvent` covers `WillAppear`, `DidAppear`, `WillDisappear`, `DidDisappear`.
- `ScreenMetadata` can store navigation ordering, hotkeys, or app-defined data.

## State & Shared Data

- Reuse `SharedState` for cross-screen sharing via the existing runtime APIs.
- Provide a thin adapter `ScreenState` that namespaces keys per screen so switching does not clobber data. Each
  screen receives the same `SharedState` instance across activations via the screen manager.
- Global zone strategies can opt into cross-screen state by using a shared key prefix (e.g., `app:global`).
- Migration note: existing plugins can continue using `SharedState`; adopting `ScreenState` is opt-in. Legacy screens can attach to a default namespace to avoid behavioral changes.

Access patterns:

- `ScreenManager::active_state` gives callers a clone of the currently active `ScreenState` (useful for navigation helpers).
- `ScreenManager::screen_state(<id>)` retrieves a scoped state handle for any registered screen, allowing shared
  initialization before the screen first activates.
- `ScreenState` mirrors the `SharedState` helpers: `insert_arc`, `shared`, and `shared_init`. Strategies can pass
  clones of the handle to panels or helper plugins if they want zone-specific storage.
- `ScreenState::navigator()` returns a `ScreenNavigator` handle that queues activation requests processed after the
  current event completes.

## Navigation Defaults

- `ScreenNavigator::request_activation(<id>)` schedules a screen swap without touching `RoomRuntime` directly; the
  runtime drains pending requests after the active screen finishes handling the current event.
- Built-in hotkeys now ship with the manager: `Ctrl+Tab` cycles forward, `Ctrl+Shift+Tab`/`Ctrl+BackTab` cycle backward.
  Hotkeys are processed before delegating to the active screen, so strategies can override behaviour by returning
  `EventFlow::Consumed` first if needed.
- Strategies can mix custom navigation (e.g., digit shortcuts) with the defaults—the workshop example demonstrates both
  approaches.

## Extensibility Hooks

- **Global Zone Formats**: implement the strategy trait for patterns such as chat, file browser, multi-pane dashboards.
- **Navigation**: expose a `ScreenNavigator` interface so global zones or panels can request screen switches (eg. `navigator.activate("settings")`).
- **Future Work**: stack-based navigation, transitions/animations, nested screens, panel layout nesting.

## Migration & Rollout Plan

1. **Passive Introduction**
   - Gate the `ScreenManager` behind a feature flag or runtime config so existing single-screen apps run unchanged.
   - Provide a `LegacyScreenStrategy` that simply mounts the current layout/panels to ease migration.
2. **Incremental Adoption**
   - Update core examples (`chat_demo`, Boxy workshops) to use the manager once the legacy strategy is stable.
   - Introduce screen-aware audit/log messages to confirm switching behaviour.
   - Current status: `chat_demo`, `audit_demo`, `boxy_dashboard_runtime`, `boxy_dashboard`, `control_room`, `runtime_first_paint`, `bootstrap_helper`, `workshop_room_bootstrap`, and `chat_workshop` already activate the manager via the legacy strategy.
3. **Documentation & Workshops**
   - Multi-screen workshop (SCREEN-106) now lives in `examples/workshop_screen_multiscreen.rs` with the companion guide in `docs/ref/workshops/workshop_screen_multiscreen.md`.
   - Capture lessons learned back into this strategy doc (navigation patterns, shared state usage) and the QUICK_REF short list as new findings emerge.
4. **Cleanup**
   - Once all first-party demos adopt screens, retire the legacy strategy and remove the feature flag.
   - Update validator checks to expect the new directory structure (`docs/ref/strat/`).

## Known Open Questions / Backlog

- Whether screens should support push/pop in the MVP or if tab-style switching is enough.
- Declarative vs. imperative specification of child panels (configuration structs vs. builder APIs).
- How to represent async screen loading (e.g., waiting spinners) within the global zone.
- Panel nesting or composite panels within a screen.

## Risks & Challenges

- **Layout churn**: switching screens replaces the active layout tree; ensure registry dirty flags reset cleanly or we risk flicker/duplicate renders.
- **Plugin lifecycle**: panels reused across screens must clean up properly (shared state, focus), otherwise residual state could leak.
- **Event storms**: global zones that rebroadcast every event to every panel may introduce redundant work—strategy implementations should short-circuit when possible.
- **Interop with existing demos**: we must keep a compatibility path so single-screen apps continue to work until they opt in to screens.
- **Testing complexity**: multi-screen flows require scripted tests to verify navigation, otherwise regressions will be hard to spot.

## Performance Considerations

- **Screen switch cost**: aim to reuse buffers where possible—avoid rebuilding layouts/panels on every activation if the screen was previously instantiated.
- **Render batching**: when a new screen activates, consolidate initial dirty zones into a single render pass to maintain the “zippy” feel.
- **Shared state access**: `ScreenState` wrapper should be lightweight; avoid extra locking/allocations compared to `SharedState`.
- **Audit & logging**: keep audit helpers screen-aware without duplicating events for every panel to prevent log spam.
- **Hot path impact**: ensure event routing through the screen manager remains inlined/minimal so per-frame overhead stays negligible.

Keep this document updated as we validate the approach and discover new patterns.
