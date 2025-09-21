# Rehydration Prompt – Room Runtime (Phase 2)

Use this snippet when spinning up a fresh agent/session. It assumes the current
state of `main` and the latest process doc structure.

```
You are onboarding into the Room MVP project.
1. **Read `START.txt`** to orient on the entry point, then follow its links into `docs/procs/PROCESS.md`, `docs/procs/CONTINUE.md`, and `docs/procs/QUICK_REF.md` for current state.
2. Confirm the docs validator passes: `./bin/validate-docs.sh`.
3. Review the runtime roadmap (`docs/procs/TASKS.md`) with emphasis on SCREEN-102 and ROOM-613.
4. For screen work, read `docs/ref/strat/SCREEN_ZONE_STRATEGY.md` and note the migration plan / risks.
5. Existing runtime helpers to be aware of: cursor utilities (`src/cursor.rs`), bootstrap controls (`RoomRuntime::bootstrap_controls`), and the new `ScreenManager` scaffold.
6. When coding, keep single-screen flows operational unless you are explicitly migrating a demo; prefer legacy strategy until all examples adopt screens.
7. Update CONTINUE + QUICK_REF after each work block and rerun the validator before handoff.
```

Keep this prompt alongside the QUICK_REF so future runs have a concise hydration path.



  - Added the runtime::screens module with ScreenManager, ScreenDefinition, GlobalZoneStrategy, lifecycle handling, and
  a smoke test that exercises activation (src/runtime/screens/mod.rs).
  - Extended RoomRuntime to track the active screen size, optionally host a manager, apply screen layouts, and route
  events through the manager before legacy plugins while keeping single-screen flows intact (src/runtime/mod.rs:329,
  src/runtime/mod.rs:464).
  - Re-exported the screen toolkit for downstream crates (src/lib.rs:39).

  Process & Docs

  - Updated the screen strategy note with lifecycle diagrams, migration plan, and perf considerations; CONTINUE/
  QUICK_REF now reference the new work and SCREEN-102 focus, and SCREEN-101 is marked complete in the task list (docs/
  ref/strat/SCREEN_ZONE_STRATEGY.md, docs/procs/CONTINUE.md, docs/procs/QUICK_REF.md, docs/procs/TASKS.md).
  - Added docs/procs/REHYDRATE_PROMPT.md so future sessions have a ready-to-run hydration snippet.

  Tests: cargo fmt; cargo test; bin/validate-docs.sh.

  Next Steps

  1. Complete SCREEN-102 by invoking activate_screen during startup (with a legacy strategy) and migrating a demo to
  prove the flow.
  2. Outline the multi-screen workshop (SCREEN-106) once the manager is exercised, then follow the usual CONTINUE doc
  updates.
