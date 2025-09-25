# ROOM RUNTIME & PLUGIN ROADMAP
# Generated: 2025-09-18
# Epic 2: Runtime, Plugins, and Tooling

## REFERENCES
- REF-ROOM-101: docs/ref/strat/RUNTIME_STRATEGY.md — runtime lifecycle, logging, benchmarking.
- REF-ROOM-102: docs/procs/BACKLOG.md — open items from MVP roadmap.
- REF-ROOM-103: docs/ref/strat/LAYOUT_ENGINE_STRATEGY.md — original architecture notes.

## MILESTONE META: Self-Hydrating Workflow (Priority: CRITICAL)

### [x] ROOM-META-001: Validator realignment [2 pts]
- Replace Boxy-specific checks with Room-aware structure validation.
- Keep silent-success/noisy-failure semantics and add freshness warnings.

### [x] ROOM-META-002: Process doc normalization [2 pts]
- Update all process docs to reference `docs/procs/` and `docs/ref/` paths. ✅
- Swept repository docs for legacy references, refreshed README/META_PROCESS, and recorded completion in CONTINUE/DONE.

### [x] ROOM-META-003: Analysis hydration [1 pt]
- Seed `.analysis/consolidated_wisdom.txt` with current architecture learnings.
- Track technical debt snapshot in `.analysis/technical_debt.txt`.

### [x] ROOM-META-004: Legacy reference sweep [1 pt]
- Repository-wide search confirms only intentional references remain.
- Summary recorded in CONTINUE; no follow-up tickets needed.

## MILESTONE 6: Runtime Core (Priority: CRITICAL)

### [x] ROOM-601: Scaffold RoomRuntime coordinator [5 pts]
- Implement `RoomRuntime` struct owning layout, registry, renderer, event loop.
- Add basic `Event` enum and polling over stdin + timer.
- Provide `run()` with graceful shutdown and cursor restore guarantees.

### [x] ROOM-602: Define plugin system & port chat demo [5 pts]
- Introduce `RoomPlugin` trait with `on_event`/`before_render` hooks.
- Extract chat input, status bar, scripted bot into plugins.
- Update `examples/chat_demo.rs` (or new runtime demo) to use runtime + plugins.

### [x] ROOM-607: Bundle default CLI chrome [3 pts]
- Provide reusable input prompt + status bar plugins with shared state helpers.
- Document usage in PLUGIN_API and SHARED_RUNTIME_STRAT.
- Update core demos to consume the bundle where practical.

### [x] ROOM-608: Socket driver stub [2 pts]
- Add TCP driver that replays JSON events through `RoomRuntime::run_scripted`.
- Re-export driver for downstream harnesses and document usage.
- Map key/resize/tick/paste events for remote automation.

### [ ] ROOM-609: Socket streaming + response channel [3 pts]
- BLOCKED: requires Meteor + ASC100 protocol finalisation.
- Support bidirectional frames so remote clients can receive incremental renders.
- Handle long-lived sessions with heartbeat/tick management.
- Provide example harness that tails output over the socket driver.

### [x] ROOM-610: Capture runtime baselines [2 pts]
- Baseline `phase2` captured via `cargo bench --bench runtime -- --save-baseline phase2`.
- Snapshots mirrored into `meta/snaps/` with `bin/snap_benchmarks.sh phase2`.
- Documented workflow in `docs/ref/FEATURES_RUNTIME_PHASE2.md`.

### [x] WORKSHOP-101: Layout Fundamentals Workshop [2 pts]
- `examples/workshop_layout_fundamentals.rs` delivers three guided scenarios.
- `docs/ref/workshops/workshop_layout_fundamentals.md` documents the walkthrough and exercises.
- Acceptance met per `.eggs/egg.1.examples-workshop-patterns.txt` "Basic Layout Composition" guidance.

### [x] WORKSHOP-201: Boxy Dashboard Runtime Workshop [2 pts]
- Enhanced `examples/boxy_dashboard_runtime.rs` with workshop header instructions.
- Authored `docs/ref/workshops/workshop_boxy_dashboard_runtime.md` covering focus cycling, refresh flow, prompt exercises.
- Acceptance met (workshop flow + guidance captured).

### [x] WORKSHOP-201A: Boxy Dashboard Workshop Fixes [1 pt]
- Adjusted the dashboard runtime so `Enter` logs without clearing the prompt while `Ctrl+Enter` submit-and-clears, and
  set `RuntimeConfig::default_focus_zone` to keep the prompt focused.
- Added troubleshooting notes to `docs/ref/workshops/workshop_boxy_dashboard_runtime.md` covering prompt behaviour and
  refresh tips.

### [x] ROOM-615: Boxy dashboard lifecycle regression [2 pts]
- FIXED: UserReady emission decoupled from dirty zone rendering in commit 6de0bd9.
- Root cause: UserReady was only firing when `!dirty.is_empty()` in `render_if_needed()`, breaking CLI driver contract.
- Solution: Moved UserReady emission outside dirty zone conditional - now fires after bootstrap completion regardless of zone state.
- CLI driver should now properly enter event loop and process keyboard/focus events without stalling in `event::poll()`.

### [x] WORKSHOP-202: Boxy Grid Workshop [2 pts]
- Introduced `examples/workshop_boxy_grid.rs` with multi-scenario grid walkthroughs.
- Documented exercises in `docs/ref/workshops/workshop_boxy_grid.md`.
- Acceptance met (example + guide) aligned with China’s grid workshop proposal.

### Workshop Foundations (WKSP)
- [ ] WKSP-00: Lifecycle contract smoke — guarantee `Open → Boot → Setup → render → UserReady` before user interaction; provide programmatic triggers for graceful exit, recoverable error, and fatal path so teardown ordering is verifiable.
- [ ] WKSP-01: Bootstrap render baseline — default focus/cursor resolved, no blank first frame before `UserReady`.
- [ ] WKSP-02: Event loop pacing — configurable tick interval, loop exits immediately on request, idle-friendly polling.
- [ ] WKSP-03: Input plumbing — all key/paste/mouse surfaces flow through `RuntimeEvent`; Esc/q/Ctrl+C always exit.
- [ ] WKSP-04: Focus/Cursor integration — `FocusController` emits `FocusChanged`, cursor hints reflect ownership, cursor visibility toggles correctly.
- [ ] WKSP-05: Audit visibility — standardized bootstrap-friendly audit sink to observe lifecycle without custom wiring.
- [ ] WKSP-06: Resource cleanup — demos restore terminal state via `Runtime::finalize()` and leave no orphaned PTYs/binaries.
- [ ] WKSP-07: Scriptable harness — each workshop exposes a non-TTY harness to replay lifecycle/input scenarios for tests/UAT.

### [ ] WORKSHOP-301: First Paint Performance Workshop [2 pts]
- Extend `examples/runtime_first_paint.rs` into a performance-focused tutorial covering audit hooks and first-render metrics.
- Acceptance: written instructions + baseline expectations for render timing and audit output.

## MILESTONE 7: Observability & Performance (Priority: HIGH)

### [~] ROOM-603: Structured logging + metrics scaffold [3 pts]
- Add feature-gated logging facade with rotating JSONL file output.
- Emit runtime lifecycle events (`event_received`, `token_applied`, `render_span`).
- Surface basic metrics (render FPS, dirty zone counts) via log snapshots.

### [~] ROOM-604: Benchmark harness (Criterion) [3 pts]
- Add benchmarking crate covering layout solve, render diffs, end-to-end loop.
- Ensure benches replay recorded event traces for regression detection.
- Wire into CI (or document manual invocation) with baseline comparison guidance.

### [ ] ROOM-611: Dependency refresh sweep [2 pts]
- Update core dependencies (blake3, crossterm, serde stack, unicode-width, thiserror, strip-ansi-escapes).
- Align rsb crates with latest 0.2.18 release while keeping local overrides where required.
- Verify Boxy/git integration remains stable after bumps and rerun `cargo check` on examples/benches.

### [x] ROOM-612: Runtime bootstrap helpers [2 pts]
- Provide helper(s) on `RoomRuntime` to force an initial render/tick, optionally wait for N ticks, or gate on first key input. ✅ (`BootstrapControls` with tick + key gating)
- Update examples to opt into the bootstrap helper so first-run renders never start blank. ✅ (`audit_demo`, `runtime_first_paint`, `bootstrap_helper`)
- Document usage alongside existing driver/bundle guidance. ✅ (`docs/ref/strat/RUNTIME_STRATEGY.md` section)

### [x] ROOM-613: Focus defaults & prompt detection [2 pts]
- Extend Focus API/runtime config so callers can declare a default focus zone (e.g., decorated prompt) during bootstrap. ✅ (`RuntimeConfig::default_focus_zone`)
- Ensure bundles/examples can mark prompt zones for immediate focus without manual wiring. ✅ (prompt-driven demos now set the zone; screen manager reapplies focus on activation)
- Capture best practices for focus transitions in `docs/ref/CORE_PLUGIN_STRATEGY.md`. ✅ (new "Default Focus" guidance)

	-### [x] SCREEN-101: Screen/global zone architecture spec [2 pts]
	- Finalize the Screen + GlobalZone strategy doc (`docs/ref/strat/SCREEN_ZONE_STRATEGY.md`) with lifecycle diagrams and update plan. ✅
	- Identify integration points with `RoomRuntime` and outline migration steps for existing demos. ✅

### [x] SCREEN-102: Screen manager implementation [5 pts]
- Implement `ScreenManager` with registration, activation, and lifecycle notifications.
- Wire manager into `RoomRuntime` event dispatch/render pipelines with feature flag or config toggle.
- Add smoke tests covering screen switching and layout swaps.
- Legacy strategy landed and all runtime-driven demos (`chat_demo`, `audit_demo`, `boxy_dashboard_runtime`, `boxy_dashboard`, `control_room`, `runtime_first_paint`, `bootstrap_helper`, `workshop_room_bootstrap`, `chat_workshop`) now activate the manager on startup; focused tests live in `runtime::screens` to guard activation.

### [x] SCREEN-103: Global zone strategy trait + default implementation [3 pts]
- Defined `GlobalZoneStrategy` with screen-scoped state access so strategies can register panels, react to lifecycle
  events, and mediate runtime events without losing legacy behaviour.
- Extended `LegacyScreenStrategy` to accept `ScreenState` handles while preserving existing demos.
- Screen manager now exposes `active_state`/`screen_state` helpers and re-applies configured focus after
  `register_panels`, keeping audit/bootstrap helpers unchanged.

### [x] SCREEN-104: Screen-scoped state adapter [2 pts]
- Added `ScreenState` + internal namespace store so each screen receives a persistent `SharedState` clone across
  activations, with tests covering isolation and lazy init.
- ScreenManager hands the scoped state to strategies during lifecycle + event handling, and exposes the handles for
  callers that need to seed data before activation.
- Updated `docs/ref/strat/SCREEN_ZONE_STRATEGY.md` with the new API surface and usage guidance.

### [x] SCREEN-105: Navigation & event routing [3 pts]
- Added `ScreenNavigator` + `ScreenState::navigator` so strategies/panels can enqueue screen switches without touching
  the runtime directly.
- Screen manager now drains pending navigation requests post-event, ships default hotkeys (`Ctrl+Tab`,
  `Ctrl+Shift+Tab`/`Ctrl+BackTab`), and preserves registration order for cycling.
- Unit tests cover state isolation, explicit navigation requests, and hotkey cycling (forward/backward).
- New workshop (`examples/workshop_screen_navigation.rs`) exercises the navigator API alongside the defaults; guide
  lives in `docs/ref/workshops/workshop_screen_navigation.md`.

### [x] SCREEN-106: Multi-screen workshop & example [2 pts]
- Added `examples/workshop_screen_multiscreen.rs` showcasing dashboard/settings/activity screens, shared runtime state, and navigator shortcuts. ✅
- Authored `docs/ref/workshops/workshop_screen_multiscreen.md` with guided exercises and takeaways. ✅
- Updated process docs/QUICK_REF to highlight the new multi-screen flow and recorded completion in CONTINUE + DONE. ✅

### [x] WORKSHOP-203: Runtime bootstrap & cleanup flow [1 pt]
- Add a guided workshop that captures the first render, streams audit events, and exits cleanly. ✅ (`examples/workshop_room_bootstrap.rs`)
- Document the flow and cursor helper usage in `docs/ref/workshops/workshop_room_bootstrap.md`. ✅
- Ensure helper utilities (`cursor`, `BootstrapControls`) are exercised in examples/tests.

## MILESTONE 8: Declarative Layout & Examples (Priority: MEDIUM)

### [~] WORKSHOP-204: Room MUD Mini Game (ASCII) [2 pts]
- `examples/mud_mini_game.rs` scaffolds a room-to-room crawl with inventory + gem collection; cursor + lifecycle issues resolved.
- Workshop doc pending polish (partial notes live in new draft); needs action list + troubleshooting before marking complete.
- Follow-up: flesh out NPC interactions and automated tests, align with token protocol guidelines.

### [~] WORKSHOP-205: Boxy MUD Panels [2 pts]
- `examples/mud_boxy_game.rs` renders the dungeon using Boxy tiles + emoji loot, with navigation and inventory panels.
- Workshop guide drafted; remaining gaps include focus cues, Boxy color palette tuning, and edge-case handling for map layout.
- Follow-up: integrate default CLI bundle for prompts, document outstanding behaviors, and add exercises around Boxy customization.

### [~] WORKSHOP-206: Debug Zone Explorer [1 pt]
- `examples/workshop_debug_zone.rs` logs every `set_zone` call into a dedicated debug panel while showcasing focus toggling.
- Workshop doc summarizes current behavior but lacks detailed exercises, troubleshooting, and automated coverage.
- Follow-up: add cursor hinting, richer diagnostics, and scripted regression tests before moving to DONE.

### [ ] ROOM-605: Layout DSL prototype [4 pts]
- Design minimal DSL (YAML/HTML-like) that compiles to `LayoutTree` + zone metadata.
- Support plugin/component declarations (e.g., `<chat-input channel="app:input"/>`).
- Recreate chat layout via DSL to validate compiler.

### [ ] ROOM-606: Align workshops with runtime [2 pts]
- Rework `chat_workshop` to run on `RoomRuntime` with token stream compliance.
- Promote slash command/preferences logic into reusable plugins/adapters.
- Document usage in README or dedicated tutorials.

## MILESTONE 9: Lifecycle & Reliability Enhancements (Priority: CRITICAL)

### [x] ROOM-611: Implement friendly runtime lifecycle events [3 pts]
- Add the `Open`, `Boot`, `Setup`, `UserReady`, `LoopIn`, `LoopOut`, `UserEnd`, `Cleanup`, `End`, and `Close` variants to `RuntimeEvent` and emit them at the documented checkpoints.
- Provide matching plugin hooks (`on_boot`, `on_user_ready`, `on_user_end`, etc.) and ensure `CliDriver` surfaces driver-owned markers.
- Update existing demos/tests to rely on the new lifecycle signals.

### [x] ROOM-612: Ship shared Cursor manager + cursor events [3 pts]
- Introduce a `Cursor` struct (position, visibility, `char`, style) and a runtime-managed `CursorManager` shared resource.
- Emit `CursorMoved`, `CursorShown`, and `CursorHidden` events after each cursor change; expose optional `RoomPlugin::on_cursor_change` hook.
- Refactor default bundles/workshops to consume the manager instead of ad-hoc cursor math.

### [x] ROOM-613: Focus change notifications [2 pts]
- Extend `FocusController` to emit `FocusChanged` runtime events with `{ from, to }` payloads and provide an `on_focus_change` hook.
- Ensure focus transitions remain synchronized with cursor updates and screen manager activation.
- Add coverage in focus-related tests/workshops to validate the new signals.

### [x] ROOM-614: Runtime error sink and fatal teardown [4 pts]
- Implement a structured error sink that emits `Error` events, coordinates recovery handlers, and escalates to `Fatal`, `FatalCleanup`, and `FatalClose` when necessary.
- Wire audit stages and driver teardown to respect the fatal path while restoring terminal state safely.
- Add regression tests covering recoverable errors and fatal shutdown flows.

### [x] ROOM-617: SimulatedLoop execution implementation [3 pts]
- COMPLETED: Wired `RuntimeConfig::simulated_loop` and `loop_iteration_limit` into `RoomRuntime::run()` execution path in commit 1a951e9.
- Added early branching in `run()` to detect simulated mode and call `run_simulated_internal()` helper.
- Implemented loop guard enforcement with iteration counters across CLI, scripted, and simulated modes.
- Added proper audit stage emissions: `LoopSimulated` → `LoopSimulatedComplete`/`LoopSimulatedAborted` based on `fatal_active`.
- Emit `LoopGuardTriggered` + `LoopAborted` pair when iteration limits are reached.
- Updated `workshop_lifecycle_trace_01.rs` to use `SimulatedLoop::ticks(6)` - now runs headlessly without TTY access.
- DOCUMENTED: Created comprehensive `docs/SIMULATED_LOOP.md` with usage patterns, examples, and troubleshooting guide.

### [ ] ROOM-615: Test timeout sweep [2 pts]
- Audit existing integration/unit tests that rely on the CLI driver and add `loop_iteration_limit` or port them to `run_scripted`/`SimulatedLoop` helpers.
- Ensure every new lifecycle/lifecycle-adjacent test exits on its own (≤ 5s) so CI and local runs never hang.
- Update harness docs/README snippets with the timeout guidance.

### [~] ROOM-616: Debug REPL/daemon spike [3 pts]
- Prototype a runtime controller that exposes pause/step/inject capabilities over a local socket or REPL.
  * First pass (`cargo run --bin runtime_repl`) uses the lifecycle harness for manual inspection; next step is adding a socket control channel + richer UI.
- Stream audit events + current frame so engineers can inspect the lifecycle interactively.
- Document learnings and decide whether to graduate the spike into a supported debug tool.

---

## STATUS LEGEND
- [ ] Not Started
- [~] In Progress
- [x] Complete

(Use checklist updates alongside commits to reflect progress.)
