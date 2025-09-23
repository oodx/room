# room

Room is the OODX runtime workbench: a flicker-free terminal engine with a
runtime coordinator, plugin system, and workshop examples that exercise the new
screen manager and audit tooling.

## Getting Started
- `cargo fmt` – keep formatting clean (no manifest override needed).
- `cargo check` / `cargo test` – verify the crate from the repository root.
- `./bin/validate-docs.sh` – documentation validator with silent-success
  semantics; treat warnings as blockers for handoff.

## Demos & Workshops
- `cargo run --example chat_demo` – baseline runtime with bundled prompt and
  status plugins.
- `cargo run --example boxy_dashboard_runtime` – Boxy-driven dashboard running
  through the `ScreenManager` legacy strategy.
- `cargo run --example audit_demo` – first-paint audit walkthrough; uses the
  bootstrap helpers to buffer events until the first render.
- `cargo run --example workshop_screen_navigation` – exercises the default
  navigation hotkeys and `ScreenNavigator` API.
- `cargo run --example workshop_screen_multiscreen` – dashboard/settings/activity
  flow sharing runtime state across screens.
- `bin/examples.sh run <example>` – scripted launcher for any example.

The runtime utilities assume ANSI-capable terminals. When printing captured
frames outside the runtime, use `src/runtime/cursor` helpers to realign the
prompt before exit.

## Documentation Workflow
- Single entry point: `START.txt`.
- Process docs live in `docs/procs/` (`PROCESS.md`, `CONTINUE.md`, `QUICK_REF.md`,
  `SPRINT.md`, `TASKS.md`, `ROADMAP.md`, `DONE.md`). Update `CONTINUE.md` and
  `QUICK_REF.md` at the end of every session.
- Reference notes live in `docs/ref/` (strategy notes, plugin API, workshops,
  logging, benchmarking, screen strategy, and more).
- Long-lived analysis artefacts live in `.analysis/`; generated summaries stick
  to `.eggs/`.

Follow the hydration checklist in `docs/procs/PROCESS.md` whenever you pick up
the project. The validator protects critical docs and flags stale references.

## Architecture Snapshot
- `src/runtime` – runtime coordinator, screen manager, bootstrap/audit helpers.
- `src/layout` – constraint solver and zone registry integration.
- `src/render` – ANSI diff renderer.
- `examples/` – demos + workshops covering bootstrap flows, navigation, and
  Boxy integrations.

Reference deep dives:
- Runtime lifecycle & bootstrap: `docs/ref/strat/RUNTIME_STRATEGY.md`
- Screen/global zone design: `docs/ref/strat/SCREEN_ZONE_STRATEGY.md`
- Plugin API & shared state: `docs/ref/PLUGIN_API.md`,
  `docs/ref/strat/SHARED_RUNTIME_STRATEGY.md`
- Audit + benchmarks: `docs/ref/strat/LOGGING_STRATEGY.md`,
  `docs/ref/FEATURES_RUNTIME_PHASE2.md`, `docs/ref/strat/BENCHMARKING_STRATEGY.md`

## Project Status
- Current phase: Runtime & Plugin Phase 2 (observability polish + screen manager
  adoption).
- Recent work: screen-scoped state, navigation helpers, audit/first-paint demos,
  cursor utilities, documentation sweep.
- Upcoming focus: multi-screen workshop production (SCREEN-106), Boxy workshop
  fixes (WORKSHOP-201A), and cross-terminal smoke tests for bootstrap flows.

## Contribution Rules
- Always hydrate context via `START.txt` → `docs/procs/PROCESS.md` →
  `docs/procs/CONTINUE.md` before editing.
- Keep process docs in sync with code changes; log completions in
  `docs/procs/DONE.md` with timestamps.
- Run `./bin/validate-docs.sh` before leaving a session; update the script if
  you add or rename critical documentation.
