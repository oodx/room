# Room Roadmap (High-Level)

## Phase 1 – Layout Engine MVP (Complete)
- Constraint solver, zone registry, ANSI renderer.
- Chat demo showcasing selective updates and footer input.
- Documentation in `docs/ref/strat/LAYOUT_ENGINE_STRATEGY.md`; pending clean-up tracked in `BACKLOG.md`.

## Phase 2 – Runtime & Plugins (In Progress)
- Build `RoomRuntime` coordinator with event loop and lifecycle hooks.
- Formalize plugin system; extract chat/workshop behaviors into reusable modules.
- Ship structured logging + metrics and Criterion benchmarks for regression tracking.
- Introduce Screens + Global Zone layer so applications can swap full-view layouts without restarting the runtime.
- Land the friendly lifecycle/Error path work (`ROOM-611`–`ROOM-614`) so sessions expose `Open → Close` signals, cursor/focus events, and safe fatal teardown.

## Phase 3 – Declarative Layout & Batteries Included UX
- Create layout DSL/compiler for zone declarations and plugin wiring.
- Provide standard components: chat panes, status bars, boxy window manager, split panes.
- Expand workshops/examples to demonstrate token-compliant patterns.
- Prototype experiential workshops (e.g. MUD mini game, Boxy MUD panels) to exercise runtime + Boxy integration, then
  harden them into fully guided tutorials.
- Introduce "debug" workshops that surface runtime internals (dirty logs, focus changes) to guide future tooling work.

## Phase 4 – Adapters & Integrations
- CLI driver (crossterm), socket/WebSocket driver, and scripted test harness.
- Explore tmux-like controller, chat server integration, and remote session support.
- Package runtime plugins/adapters for reuse across OODX projects.

## Phase 5 – Observability & Scaling
- Deep-dive profiling, runtime diagnostics dashboard, replay tooling.
- Long-running soak tests feeding synthetic workloads.
- Hardening for multi-plugin coexistence and backpressure handling.
