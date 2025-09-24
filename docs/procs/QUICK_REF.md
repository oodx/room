🎯 CURRENT FOCUS: Polish the new MUD + debug workshops (WORKSHOP-204/205/206), then return to WORKSHOP-201A and first-paint planning.

📊 PROJECT STATUS
- ✅ Layout engine + runtime plugins (Phase 1 complete, Phase 2 mid-flight)
- ✅ Audit + bootstrap helpers with demos (`runtime_first_paint`, `audit_demo`, `bootstrap_helper`) shipping
- ✅ SCREEN-106 multi-screen workshop landed (example + guide); docs/process logs updated
- 🔄 Updated prototypes: `mud_mini_game`, `mud_boxy_game`, and `workshop_debug_zone` now emit the friendly lifecycle/cursor/focus signals (still ≈60%; polish tracked in TASKS/BACKLOG)
- ⬆️ Next major push: finish workshop polish (204/205/206), WORKSHOP-201A fixes, WORKSHOP-301 outline; UAT sweep still pending

-🚨 CRITICAL CONTEXT
- All demos should route through `BootstrapControls` + `BootstrapAudit` to keep logs quiet until the first frame—flag any new surfaces that bypass them.
- Screen manager exposes `ScreenState::navigator()` and default hotkeys (`Ctrl+Tab`, `Ctrl+Shift+Tab`/`Ctrl+BackTab`); `examples/workshop_screen_navigation.rs` and `examples/workshop_screen_multiscreen.rs` are the canonical guides.
- New workshop docs live under `docs/ref/workshops/workshop_mud_mini_game.md`, `workshop_mud_boxy_game.md`, and `workshop_debug_zone.md`; each lists remaining TODOs.
- Validator warnings block handoff; rerun `./bin/validate-docs.sh` after doc edits.
- Active Phase: Runtime & Plugin Phase 2 (observability polish + hands-on workshops).

📋 ACTIVE TASKS (see docs/procs/SPRINT.md for detail)
1. WORKSHOP-204/205/206 polish: add exercises, troubleshooting, and tests to the new workshop docs/examples.
2. WORKSHOP-201A: Resolve Boxy dashboard workshop defects and update the walkthrough.
3. WORKSHOP-301 prep: Outline first-paint performance workshop, capture `cargo bench --bench runtime` baselines.
4. Plan the UAT sweeps when capacity frees up (ROOM-UAT-001 terminal coverage + ROOM-UAT-002 lifecycle verification).

🏗️ ARCHITECTURE SNAPSHOT
- Primary modules: `src/runtime`, `src/layout`, `src/render`, `src/registry`
- Key examples: `examples/chat_demo.rs`, `examples/control_room.rs`
- Benchmarks: `benches/runtime.rs` with baseline `meta/snaps/runtime_*`

🛠️ COMMANDS
- Tests: `cargo test --manifest-path Cargo.toml`
- Bench baseline replay: `cargo bench --bench runtime -- --baseline phase2`
- Docs health: `./bin/validate-docs.sh`
- Chat demo smoke: `cargo run --example chat_demo`

⚡ IMMEDIATE NEXT STEPS
1. Finish WORKSHOP-204/205/206 doc polish (exercises/troubleshooting) and tighten Boxy/debug UX.
2. Address WORKSHOP-201A defects and update the guide with troubleshooting notes.
3. Re-run `cargo bench --bench runtime` before drafting WORKSHOP-301 notes so performance deltas are captured with fresh baselines.
4. Capture multi-screen findings for `docs/ref/strat/SCREEN_ZONE_STRATEGY.md` after the next validation pass.
5. Keep `docs/procs/DONE.md` updated as tasks close to preserve validator silence; schedule the UAT sweep (ROOM-UAT-001) when capacity frees up.
