# Continue Log – main · Runtime Audit & First-Paint Prototypes

## Summary
- Lifecycle trace harness now lives under tests (`tests/workshop_lifecycle_trace.rs`) and the `runtime_repl` binary; the old example has been retired.
- Runtime audit gains `LoopSimulatedAborted` and `LoopAborted` stages so fatal paths are visible in both simulated and real driver loops; docs updated accordingly.
- Added a scripted lifecycle harness in `tests/workshop_lifecycle_trace.rs` so graceful/recoverable/fatal paths are asserted automatically; tests use simulated loops to avoid long-running drivers.
- Prototyped a runtime REPL (`cargo run --bin runtime_repl`) that drives `RoomRuntime` via the lifecycle harness for interactive inspection/command replay.
- FIXED (commit 6de0bd9): Critical regression in `boxy_dashboard_runtime` where UserReady was only firing when zones were dirty, causing CLI driver to stall in `event::poll()`. UserReady emission now decoupled from dirty zone rendering and fires after bootstrap completion regardless of zone state.
- COMPLETED (commit 1a951e9): Full `SimulatedLoop` execution implementation with early branching in `RoomRuntime::run()`, proper audit stage emissions, and loop guard enforcement. Examples like `workshop_lifecycle_trace_01.rs` now run headlessly using `SimulatedLoop::ticks(6)` without TTY access.

## Current Status
- **Branch**: main
- **Phase**: Runtime & Plugin Phase 2 (observability + hands-on workshops)
- **Progress**: Runtime core + screen manager remain stable; prototype workshops now cover multi-screen flows, MUD gameplay (ASCII + Boxy), dirty-zone inspection, and the new focus/cursor lifecycle demos. Existing MUD/Boxy/debug guides still need repairs to adopt `FocusController` before graduation.

## Next Steps (Priority Order)
1. Test UserReady fix in real CLI scenarios - verify `boxy_dashboard_runtime` now properly processes keyboard/focus events without stalling.
2. Add timeouts / bounded loops to the remaining driver-based tests (set `loop_iteration_limit` or switch to `run_scripted`) so suites can never stall longer than a few seconds.
3. Update additional examples to use simulated execution where appropriate (lifecycle trace now works headlessly with `SimulatedLoop::ticks(6)`).
5. Design focus/input/cursor drills on top of the lifecycle trace infrastructure (WKSP-01..04) now that WKSP-00 is locked by the harness.
6. Reintroduce content-rich workshops (MUD, Boxy, Debug) once lifecycle + focus infrastructure is stable; keep TASKS entries updated.
7. Schedule ROOM-UAT-002 to exercise lifecycle signals across representative demos after the focus/input drills land.
8. Iterate on the debug REPL (extend beyond lifecycle harness, explore socket control channel) and capture follow-up requirements.

## Blockers / Risks
- WORKSHOP-204/205 remain prototypes; without completing exercises/tests we risk shipping partially documented workflows.
- Boxy dashboard (WORKSHOP-201A) still needs UX fixes and better troubleshooting to match the new workshop quality bar.
- UAT sweeps: ROOM-UAT-001 (terminal coverage) and ROOM-UAT-002 (lifecycle verification) both pending; track findings before promoting the workshops.

## Notes for Next Agent
- Workshop directories are empty (`examples/bak/` holds the old versions). Continue rebuilding from the WKSP checklist; avoid restoring the archived files wholesale.
- The lifecycle trace example now prints condensed stage summaries; rely on that output when writing or extending scripted assertions.
- Prefer `run_scripted`, `SimulatedLoop`, or explicit `loop_iteration_limit` when adding tests—interactive drivers must include a timeout path.
- Use the new `runtime_repl` binary for interactive lifecycle debugging; capture enhancements (socket control, pause/step UI) under ROOM-616.
- After the timeout sweep, replicate the trace pattern for focus/input/cursor drills before reintroducing complex demos.
- The UserReady fix (commit 6de0bd9) should resolve the dashboard regression, but simulated execution still needs implementation before lifecycle examples work properly.
- Validation needed: confirm UserReady fix works in real CLI scenarios and that `boxy_dashboard_runtime` processes input events correctly.
