🎯 CURRENT FOCUS: Close out WORKSHOP-201A, outline the first-paint workshop plan, and document multi-screen learnings.

📊 PROJECT STATUS
- ✅ Layout engine + runtime plugins (Phase 1 complete, Phase 2 mid-flight)
- ✅ Audit + bootstrap helpers with demos (`runtime_first_paint`, `audit_demo`, `bootstrap_helper`) shipping
- ✅ SCREEN-106 multi-screen workshop landed (example + guide); docs/process logs updated
- ⬆️ Next major push: WORKSHOP-201A fixes, WORKSHOP-301 outline, and screen-strategy write-up (UAT sweep parked in backlog)

-🚨 CRITICAL CONTEXT
- All demos should route through `BootstrapControls` + `BootstrapAudit` to keep logs quiet until the first frame—flag any new surfaces that bypass them.
- Screen manager exposes `ScreenState::navigator()` and default hotkeys (`Ctrl+Tab`, `Ctrl+Shift+Tab`/`Ctrl+BackTab`); `examples/workshop_screen_navigation.rs` and `examples/workshop_screen_multiscreen.rs` are the canonical guides.
- README + META_PROCESS now mirror the `docs/procs/*.md` layout—extend the validator + PROCESS doc if you add new critical notes.
- Validator warnings block handoff; rerun `./bin/validate-docs.sh` after doc edits.
- Active Phase: Runtime & Plugin Phase 2 (observability polish + navigation education).

📋 ACTIVE TASKS (see docs/procs/SPRINT.md for detail)
1. WORKSHOP-201A: Resolve Boxy dashboard workshop defects and update the walkthrough.
2. WORKSHOP-301 prep: Outline first-paint performance workshop, capture `cargo bench --bench runtime` baselines.
3. Fold multi-screen findings into `docs/ref/strat/SCREEN_ZONE_STRATEGY.md` once follow-up testing results arrive.
4. Plan the UAT sweep when capacity frees up (ROOM-UAT-001 in `docs/procs/BACKLOG.md`).

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
1. Address WORKSHOP-201A defects and update its guide with troubleshooting notes.
2. Re-run `cargo bench --bench runtime` before drafting WORKSHOP-301 notes so performance deltas are captured with fresh baselines.
3. Capture multi-screen findings for `docs/ref/strat/SCREEN_ZONE_STRATEGY.md` after the next validation pass.
4. Keep `docs/procs/DONE.md` updated as tasks close to preserve validator silence.
5. Schedule the UAT sweep (ROOM-UAT-001) once higher-priority workshop work lands.
