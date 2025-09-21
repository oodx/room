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

### [~] ROOM-META-002: Process doc normalization [2 pts]
- Update all process docs to reference `docs/procs/` and `docs/ref/` paths.
- Sweep remaining code/docs for legacy references and clean them up.

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

### [ ] WORKSHOP-201A: Boxy Dashboard Workshop Fixes [1 pt]
- Investigate defects discovered during workshop review (focus quirks, prompt experience).
- Acceptance: workshop runs end-to-end without issues; guide updated with troubleshooting tips.

### [x] WORKSHOP-202: Boxy Grid Workshop [2 pts]
- Introduced `examples/workshop_boxy_grid.rs` with multi-scenario grid walkthroughs.
- Documented exercises in `docs/ref/workshops/workshop_boxy_grid.md`.
- Acceptance met (example + guide) aligned with China’s grid workshop proposal.

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

### [ ] ROOM-613: Focus defaults & prompt detection [2 pts]
- Extend Focus API/runtime config so callers can declare a default focus zone (e.g., decorated prompt) during bootstrap.
- Ensure bundles/examples can mark prompt zones for immediate focus without manual wiring.
- Capture best practices for focus transitions in `docs/ref/CORE_PLUGIN_STRATEGY.md`.

	-### [x] SCREEN-101: Screen/global zone architecture spec [2 pts]
	- Finalize the Screen + GlobalZone strategy doc (`docs/ref/strat/SCREEN_ZONE_STRATEGY.md`) with lifecycle diagrams and update plan. ✅
	- Identify integration points with `RoomRuntime` and outline migration steps for existing demos. ✅

### [~] SCREEN-102: Screen manager implementation [5 pts]
- Implement `ScreenManager` with registration, activation, and lifecycle notifications.
- Wire manager into `RoomRuntime` event dispatch/render pipelines with feature flag or config toggle.
- Add smoke tests covering screen switching and layout swaps.
- Legacy strategy landed and `chat_demo`, `audit_demo`, `boxy_dashboard_runtime`, `control_room`, `runtime_first_paint`, `bootstrap_helper`, and `workshop_room_bootstrap` now activate the manager on startup; migrate remaining demos and add focused tests before closing.

### [ ] SCREEN-103: Global zone strategy trait + default implementation [3 pts]
- Define `GlobalZoneStrategy` trait (layout provisioning, panel registration, event mediation).
- Ship a blank/default strategy that simply hosts existing panels for backward compatibility.
- Ensure audit/bootstrap helpers work unchanged under the new layer.

### [ ] SCREEN-104: Screen-scoped state adapter [2 pts]
- Build `ScreenState` wrapper on top of `SharedState` with per-screen namespaces.
- Provide ergonomics for sharing data across screens when needed.
- Document usage in strategy reference material.

### [ ] SCREEN-105: Navigation & event routing [3 pts]
- Design a screen navigation API (command dispatcher / navigator helper) and map default key bindings.
- Ensure events bubble from panels → global zone → screen manager predictably.
- Cover edge cases (unknown screen id, rapid toggles) with tests.

### [ ] SCREEN-106: Multi-screen workshop & example [2 pts]
- Create an example/workshop demonstrating two screens (e.g., dashboard vs. settings) with runtime switching.
- Update documentation and QUICK_REF to highlight multi-screen capabilities.
- Capture follow-up learnings/new patterns in SCREEN_ZONE_STRAT.

### [x] WORKSHOP-203: Runtime bootstrap & cleanup flow [1 pt]
- Add a guided workshop that captures the first render, streams audit events, and exits cleanly. ✅ (`examples/workshop_room_bootstrap.rs`)
- Document the flow and cursor helper usage in `docs/ref/workshops/workshop_room_bootstrap.md`. ✅
- Ensure helper utilities (`cursor`, `BootstrapControls`) are exercised in examples/tests.

## MILESTONE 8: Declarative Layout & Examples (Priority: MEDIUM)

### [ ] ROOM-605: Layout DSL prototype [4 pts]
- Design minimal DSL (YAML/HTML-like) that compiles to `LayoutTree` + zone metadata.
- Support plugin/component declarations (e.g., `<chat-input channel="app:input"/>`).
- Recreate chat layout via DSL to validate compiler.

### [ ] ROOM-606: Align workshops with runtime [2 pts]
- Rework `chat_workshop` to run on `RoomRuntime` with token stream compliance.
- Promote slash command/preferences logic into reusable plugins/adapters.
- Document usage in README or dedicated tutorials.

---

## STATUS LEGEND
- [ ] Not Started
- [~] In Progress
- [x] Complete

(Use checklist updates alongside commits to reflect progress.)
