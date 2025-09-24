# Continue Log – main · Runtime Audit & First-Paint Prototypes

## Summary
- Lifecycle trace harness now lives under tests (`tests/workshop_lifecycle_trace.rs`) and the `runtime_repl` binary; the old example has been retired.
- Runtime audit gains `LoopSimulatedAborted` and `LoopAborted` stages so fatal paths are visible in both simulated and real driver loops; docs updated accordingly.
- Added a scripted lifecycle harness in `tests/workshop_lifecycle_trace.rs` so graceful/recoverable/fatal paths are asserted automatically; tests use simulated loops to avoid long-running drivers.
- Prototyped a runtime REPL (`cargo run --bin runtime_repl`) that drives `RoomRuntime` via the lifecycle harness for interactive inspection/command replay.
- Regression surfaced in `boxy_dashboard_runtime`: first frame renders but no `RuntimeEvent::Key`/`FocusChanged` ever reach the plugin. Instrumentation confirms `user_ready` likely never fires, so the driver loop may gate input until `UserReady` and the runtime currently fails to emit it for this example.

## Current Status
- **Branch**: main
- **Phase**: Runtime & Plugin Phase 2 (observability + hands-on workshops)
- **Progress**: Runtime core + screen manager remain stable; prototype workshops now cover multi-screen flows, MUD gameplay (ASCII + Boxy), dirty-zone inspection, and the new focus/cursor lifecycle demos. Existing MUD/Boxy/debug guides still need repairs to adopt `FocusController` before graduation.

## Next Steps (Priority Order)
1. Add timeouts / bounded loops to the remaining driver-based tests (set `loop_iteration_limit` or switch to `run_scripted`) so suites can never stall longer than a few seconds.
2. Diagnose why `boxy_dashboard_runtime` never emits `UserReady`/input events. Trace `CliDriver::run` vs `RuntimeRuntime::render_if_needed` to verify the first frame commits and the driver exits bootstrap.
3. Design focus/input/cursor drills on top of the lifecycle trace infrastructure (WKSP-01..04) now that WKSP-00 is locked by the harness.
4. Reintroduce content-rich workshops (MUD, Boxy, Debug) once lifecycle + focus infrastructure is stable; keep TASKS entries updated.
5. Schedule ROOM-UAT-002 to exercise lifecycle signals across representative demos after the focus/input drills land.
6. Iterate on the debug REPL (extend beyond lifecycle harness, explore socket control channel) and capture follow-up requirements.

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
- For the dashboard regression, capture whether `RuntimeRuntime::render_if_needed` ever emits `UserReady`; if it does not, audit plugin `init`/screen activation to see who is suppressing the initial render.
