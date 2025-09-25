🎯 CURRENT FOCUS: Harden lifecycle coverage (timeouts + scripted harness + REPL tooling) before resuming focus/cursor retrofits and WORKSHOP-204/205/206 polish.

📊 PROJECT STATUS
- ✅ Layout engine + runtime plugins (Phase 1 complete, Phase 2 mid-flight)
- ✅ Audit + bootstrap helpers with demos (`runtime_first_paint`, `audit_demo`, `bootstrap_helper`) shipping
- ✅ SCREEN-106 multi-screen workshop landed (example + guide); docs/process logs updated
- 🔄 Updated prototypes: `mud_mini_game`, `mud_boxy_game`, and `workshop_debug_zone` still need to adopt the new `FocusController` pattern—current focus signals only fire at bootstrap.
- 🆕 Canonical lifecycle demos: `workshop_focus_events` + `workshop_cursor_events` (docs in `docs/ref/workshops/`) prove the runtime emits `FocusChanged` and cursor events as designed.
- 🆕 Lifecycle trace baseline lives in the harness (`tests/workshop_lifecycle_trace.rs`) and the REPL binary (`runtime_repl`); the legacy example was archived.
- 🧪 `runtime_repl` binary drives the harness interactively for manual inspection and command replay.
- ⬆️ Next major push: finish workshop polish (204/205/206), WORKSHOP-201A fixes, WORKSHOP-301 outline; UAT sweep still pending

-🚨 CRITICAL CONTEXT
- All demos should route through `BootstrapControls` + `BootstrapAudit` to keep logs quiet until the first frame—flag any new surfaces that bypass them.
- Screen manager exposes `ScreenState::navigator()` and default hotkeys (`Ctrl+Tab`, `Ctrl+Shift+Tab`/`Ctrl+BackTab`); `examples/workshop_screen_navigation.rs` and `examples/workshop_screen_multiscreen.rs` are the canonical guides.
- New workshop docs live under `docs/ref/workshops/workshop_mud_mini_game.md`, `workshop_mud_boxy_game.md`, `workshop_debug_zone.md`, plus lifecycle references `workshop_focus_events.md`, `workshop_cursor_events.md`, and the trace summary in `docs/ref/LIFECYCLE_PLAN.md`.
- Validator warnings block handoff; rerun `./bin/validate-docs.sh` after doc edits.
- Tests that touch the driver must use timeouts (`loop_iteration_limit`) or switch to `run_scripted`/`SimulatedLoop` to avoid stalls.
- Active Phase: Runtime & Plugin Phase 2 (observability polish + hands-on workshops).

📋 ACTIVE TASKS (see docs/procs/SPRINT.md for detail)
1. Test timeout sweep: update legacy driver-based tests to use `run_scripted` or `loop_iteration_limit` so suites finish fast.
2. WORKSHOP-204/205/206 polish: add exercises, troubleshooting, and tests to the new workshop docs/examples.
3. WORKSHOP-201A: Resolve Boxy dashboard workshop defects and update the walkthrough.
4. WORKSHOP-301 prep: Outline first-paint performance workshop, capture `cargo bench --bench runtime` baselines.
5. Plan the UAT sweeps when capacity frees up (ROOM-UAT-001 terminal coverage + ROOM-UAT-002 lifecycle verification).
6. Evaluate debug REPL/daemon spike for pausing + inspecting runtime state (capture in TASKS).

🏗️ ARCHITECTURE SNAPSHOT
- Primary modules: `src/runtime`, `src/layout`, `src/render`, `src/registry`
- Key examples: `examples/chat_demo.rs`, `examples/control_room.rs`
- Benchmarks: `benches/runtime.rs` with baseline `meta/snaps/runtime_*`

🛠️ COMMANDS
- Tests: `bin/tests.sh all` (or `bin/tests.sh lifecycle` for the harness)
- Examples: `bin/examples.sh run <example>`
- Runtime REPL: `cargo run --bin runtime_repl -- --mode manual`
- Bench baseline replay: `cargo bench --bench runtime -- --baseline phase2`
- Docs health: `./bin/validate-docs.sh`
- Chat demo smoke: `bin/examples.sh run chat_demo`

⚡ IMMEDIATE NEXT STEPS
1. Sweep existing tests for timeouts (convert driver loops to simulated/limited variants).
2. Wire `FocusController` ownership into the MUD/Boxy/debug workshops so their lifecycle callbacks mirror the new demos.
3. Finish WORKSHOP-204/205/206 doc polish (exercises/troubleshooting) and tighten Boxy/debug UX.
4. Address WORKSHOP-201A defects and update the guide with troubleshooting notes.
5. Re-run `cargo bench --bench runtime` before drafting WORKSHOP-301 notes so performance deltas are captured with fresh baselines.
6. Capture multi-screen findings for `docs/ref/strat/SCREEN_ZONE_STRATEGY.md` after the next validation pass, then queue ROOM-UAT-001/002 once workshops settle.
7. Prototype the debug REPL/daemon concept (even a doc spike) so we can inspect runtime state interactively.
