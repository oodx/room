🎯 CURRENT FOCUS: Integrate the runtime audit hooks + first-paint helpers, migrate demos onto the new screen manager (using the legacy strategy), and finish the doc/reference sweep.

📊 PROJECT STATUS
- ✅ Layout engine + runtime plugins (Phase 1 complete, Phase 2 partially delivered)
- ✅ Audit module + demos (runtime_first_paint, audit_demo) landed
- ⬆️ Next major push: Focus defaults (ROOM-613), bootstrap control adoption, and doc sweep

🚨 CRITICAL CONTEXT
- Audit sinks should opt into the new audit + control helpers (ROOM-612) so streams stay silent until the first render commits; pair with cursor helpers when printing captures.
- `examples/audit_demo.rs` renders a placeholder first; confirm ANSI output looks good before logging issues.
- Validator still targets Room docs; treat any new warnings as blockers for handoff.
- Active Phase: Runtime & Plugin Phase 2, observability polish + doc hygiene.

📋 ACTIVE TASKS (see docs/procs/SPRINT.md for detail)
1. Finish the doc/reference sweep and record the clean pass. ✅
2. Spec default focus wiring (ROOM-613) and continue wiring the screen manager (SCREEN-102 legacy strategy + activation hooks) across first-party demos.
3. Draft the multi-screen workshop plan (SCREEN-106) so we can demo screen switching.
4. Exercise audit demos plus the new `bootstrap_helper`/`workshop_room_bootstrap` examples across terminals; log any rendering issues.
5. Keep CONTINUE/QUICK_REF aligned whenever sprint priorities shift.

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
1. Smoke-test Boxy surfaces (dashboard, runtime prompt, workshops) under different terminals to confirm pre-render alignment.
2. Verify the screen-managed demos (`chat_demo`, `audit_demo`, `boxy_dashboard_runtime`) and line up the next runtime/workshop target for migration.
3. Start outlining WORKSHOP-301 (first paint) in code + docs once WORKSHOP-201A is resolved.
4. Capture perf data (`cargo bench --bench runtime`) after the rendering change and note any deltas.
5. Continue logging completed sessions in DONE to keep history tidy.
