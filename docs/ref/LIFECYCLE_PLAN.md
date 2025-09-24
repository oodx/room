# Runtime Lifecycle Plan

This plan captures the lifecycle names and touch points we want Room sessions to expose. It explains how applications hook into each phase, how focus/cursor/input state stays coordinated, and which audit markers keep the flow observable.

## Terminology
- **Lifecycle Event**: High-level signal from the driver or runtime (e.g. `Boot`, `UserReady`, `Cleanup`). These become `RuntimeEvent` variants and audit stages.
- **Plugin Hook**: A `RoomPlugin` callback associated with a lifecycle stage (`on_boot`, `on_cleanup`, etc.).
- **Driver Hook**: Transitions owned by adapters (such as `CliDriver`) that prepare or restore the terminal.
- **Audit Stage**: Structured telemetry emitted by the runtime audit layer so observers can reconstruct the session timeline.

## Phase Overview

| Phase    | Lifecycle Events                       | Description                                                |
|----------|----------------------------------------|------------------------------------------------------------|
| Warm-Up  | `Open → Boot → Setup → UserReady`       | Prepare the terminal, hydrate shared state, render first frame. |
| Run Loop | `LoopIn → (RuntimeEvent) → LoopOut`      | Normal event handling: input, ticks, rendering, metrics.       |
| Wind-Down | `UserEnd → Cleanup → End → Close`        | Flush data, restore terminal state, emit final telemetry.    |
| Error Path | `Error → RecoverOrFatal → (Fatal*)` | Handle unrecoverable faults with a controlled shutdown.     |

`Fatal*` expands to `Fatal → FatalCleanup → FatalClose` if recovery fails.

## Detailed Flow

### 1. Warm-Up
1. **`Open`** *(Driver Hook + Audit Stage)*
   - Fires before `CliDriver` enables raw mode and enters the alternate screen.
   - Use for logging, splash screens, external telemetry.

2. **`Boot`** *(Runtime Event + Plugin Hook)*
   - Raised at the start of `RoomRuntime::bootstrap_prepare`, before plugin `init` executes.
   - Plugins can allocate shared resources, register screen state, or schedule first renders here.

3. **`Setup`** *(Runtime Event + Audit Stage)*
   - Emitted once plugin `init` outcomes apply but before the first paint. Ideal for hiding the cursor, starting timers, or presenting loading hints.

4. **`UserReady`** *(Runtime Event + Audit Stage)*
   - Triggered when the first frame commits. Marks the point where user interaction is safe.

### 2. Run Loop
1. **`LoopIn`** *(Runtime Event + Audit Stage)*
   - Bookend emitted immediately before `dispatch_event` processes a key/mouse/tick/resize.
   - Carries metadata about the incoming event type for profiling.

2. **Actual runtime event** *(Existing)*
   - `RuntimeEvent::Key`, `RuntimeEvent::Mouse`, `RuntimeEvent::Tick`, `RuntimeEvent::Resize`, etc. The screen manager handles it first, then plugins run in priority order.
   - Side effects (focus shifts, cursor moves, zone updates) happen inside this block via `RuntimeContext` outcomes.

3. **`LoopOut`** *(Runtime Event + Audit Stage)*
   - Fires after the event finishes propagating. Useful for latency metrics and consumption tracking.

### 3. Wind-Down
1. **`UserEnd`** *(Runtime Event + Plugin Hook)*
   - Raised when `RuntimeContext::request_exit` sets `should_exit` but before `finalize` runs. Plugins can flush buffers, persist state, or show exit prompts.

2. **`Cleanup`** *(Runtime Event + Audit Stage)*
   - Emitted as the runtime finalises and prepares to hand control back to the driver. Cursor manager should restore defaults here.

3. **`End`** *(Driver Hook + Audit Stage)*
   - Fired while the driver leaves the alternate screen and disables raw mode.

4. **`Close`** *(Final Event + Audit Stage)*
   - Signals that teardown completed and the terminal is back to baseline. Downstream systems can archive logs or notify orchestrators.

### 4. Error Path
1. **`Error`** *(Runtime Event + Audit Stage)*
   - Raised when a plugin or runtime component reports a fatal-condition candidate via the error sink. Carries structured context (`category`, `source`, `message`).
   - Plugins can listen to apply emergency UI updates, log details, or attempt local recovery.

2. **`RecoverOrFatal`** *(Runtime Event + Plugin Hook)*
   - Fired after the error sink gives every plugin a chance to mutate the shared `RuntimeError`. Handlers set `error.recoverable = true` once their fix succeeds. If any handler marks it recovered, the session resumes the loop; otherwise the runtime promotes the failure to the fatal path.

3. **`Fatal → FatalCleanup → FatalClose`** *(Runtime Events + Audit Stages + Driver Hooks)*
   - `Fatal`: announce that recovery failed and the session will terminate.
   - `FatalCleanup`: mirror `Cleanup` but for failure; flush diagnostics, restore cursor defaults, and request emergency saves.
   - `FatalClose`: ensure terminal state returns to baseline even after a crash scenario. The driver mirrors `End`/`Close` but marks the session as failed.

## Focus and Cursor Signals

### Cursor
- Introduce a `Cursor` struct carried with every cursor event:
  ```rust
  pub struct Cursor {
      pub position: (u16, u16),
      pub visible: bool,
      pub char: Option<char>,
      pub style: Option<CursorStyle>, // e.g. colour/blink metadata
  }
  ```
- Cursor lifecycle events:
  - `CursorMoved(Cursor)`
  - `CursorShown(Cursor)`
  - `CursorHidden(Cursor)`
- A shared `CursorManager` initialises during `Boot` via `RuntimeContext::shared_init` and becomes the single source of truth. It translates zone-relative offsets, clamps within bounds, and emits the events above after each change. Rendering still relies on `RuntimeContext::set_cursor_hint` so the ANSI renderer restores the cursor at the right position.

### Focus
- Emit `FocusChanged { from: Option<FocusTarget>, to: Option<FocusTarget> }` whenever `FocusController` updates. A `FocusTarget` can include the zone id and an optional component id so complex widgets (lists, toggles, etc.) can distinguish sub-elements.
- Provide optional `RoomPlugin::on_focus_change` and `RoomPlugin::on_cursor_change` hooks that fire exactly once per change; heavier consumers can still handle the explicit runtime events.
- During `Boot`, focus initialises from `RuntimeConfig::default_focus_zone`. During the run loop, focus changes stay synchronous with the key/mouse events that triggered them. During `Cleanup` and `FatalCleanup`, focus resets to defaults so the next session starts fresh.

## Audit Integration
- Extend `RuntimeAuditStage` with: `Open`, `Boot`, `Setup`, `UserReady`, `LoopIn`, `LoopOut`, `UserEnd`, `Cleanup`, `End`, `Close`, `Error`, `RecoverOrFatal`, `Fatal`, `FatalCleanup`, `FatalClose`, plus the cursor/focus signals (`CursorMoved`, `CursorHidden`, `CursorShown`, `FocusChanged`).
- Include structured fields for each stage where relevant: `{ "event": "Key", "consumed": true }` for loop events, `{ "cursor": { ... } }` for cursor updates, `{ "focused": "zone_id" }` for focus shifts, `{ "uptime_ms": u128 }` for teardown stages, `{ "error": { "category": ..., "source": ... } }` for the error path.
- Update `BootstrapAudit` to flush buffered events once `UserReady` lands, ensuring observers see a coherent first frame before live streaming begins.
- Use `LoopSimulated` + `LoopSimulatedComplete` to highlight bounded runs. The `examples/workshop_lifecycle_trace.rs` workshop now condenses the audit into a single stage timeline and emits `EventLoop[n]` counters, with branch notes clarifying when a recoverable error was handled or a fatal teardown fired.

## Adoption Checklist
1. **Runtime API**
   - Add lifecycle `RuntimeEvent` variants (`Boot`, `Setup`, `UserReady`, `LoopIn`, `LoopOut`, `UserEnd`, `Cleanup`, `Close`, `Error`, `RecoverOrFatal`, `Fatal`, `FatalCleanup`, `FatalClose`).
   - Introduce cursor/focus events and corresponding optional plugin hooks.
   - Wire emissions into `bootstrap_prepare`, `dispatch_event`, `render_if_needed`, the error sink, `finalize`, and driver entry/exit paths.

2. **Driver Updates**
   - Expose callbacks or observers so `CliDriver` can emit the driver-owned markers (`Open`, `End`, `Close`, `FatalClose`).
   - Ensure driver teardown paths respect `FatalCleanup` / `FatalClose` when the runtime reports an unrecoverable issue.

3. **Shared Resources**
   - Implement `CursorManager` with the `Cursor` struct and update bundles to consume it.
   - Ensure `FocusController` publishes `FocusChanged` events and cooperates with the cursor manager.
   - Build a runtime error sink that captures `RuntimeError` values, emits `Error`, orchestrates recovery handlers, and escalates to `Fatal*` if needed.

4. **Documentation & Tooling**
   - Refresh `docs/ref/strat/RUNTIME_STRATEGY.md` and workshop guides to reference the new lifecycle terminology and error path.
   - Add validator checks so new workshops respond to `UserReady`/`UserEnd` and verify that fatal paths restore terminal state.

5. **Testing**
   - Extend bootstrap tests to prove `on_boot` and `on_user_end` fire once per session.
   - Add scripted driver tests for cursor visibility resets, audit ordering through `Open → Close`, and fatal-path teardown (`Error → RecoverOrFatal → FatalClose`).
   - Unit-test the error sink to confirm recoverable errors rejoin the run loop and fatal errors trigger the cleanup sequence.

With these concise names, structured payloads, and a dedicated error lane, Room apps get a readable timeline, components can react to focus/cursor changes cleanly, and failure scenarios degrade gracefully with consistent telemetry.
