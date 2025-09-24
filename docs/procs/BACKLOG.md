# Room Backlog

## Pending Tickets (from MVP roadmap)
- ROOM-501: Integrate with RSB test harness patterns
  - Mirror MODULE_SPEC layout (mod orchestrator, submodules, macros).
  - Add `cargo test` + optional `bin/test.sh room` shim for cross-repo consistency.
  - Document expectations referencing REF-ROOM-005.
- ROOM-502: Write developer guide & troubleshooting
  - Expand README with quickstart, token prefix rules, cursor requirements.
  - Document known limitations (overlapping zones, terminal compatibility, unicode caveats).
  - Capture follow-up research topics in `CONTINUE.md`.

## Workshop Planning (Phase 2 Alignment)
- WORKSHOP-201A: Boxy dashboard workshop fixes
  - Address defects discovered in `examples/boxy_dashboard_runtime.rs` workshop flow (focus quirks, prompt UX).
  - Acceptance: workshop runs end-to-end without the noted issues; guide updated with troubleshooting tips.
- WORKSHOP-301: First Paint Performance Workshop
  - Build on `examples/runtime_first_paint.rs` to teach first-render metrics and audit hooks.
  - Acceptance: documented tasks + baseline expectations for render timing.
- WORKSHOP-PILOT-EDITOR-REVIEW: Pilot Mini Editor Cursor & Lifecycle Issues
  - Review and fix persistent issues in `examples/pilot_mini_editor.rs`:
    - Cursor visibility: CLI driver hides cursor, no clean framework-level solution found
    - First-paint inconsistency: Occasional race conditions in zone initialization despite screen manager
    - Cursor management: Multiple failed approaches (ANSI injection corrupts zones, gets overwritten, causes jumping)
  - Investigation needed: Room framework may need cursor visibility API at runtime level
  - Acceptance: Stable cursor behavior and deterministic first-paint across all runs
- WORKSHOP-204-FOLLOWUP: Flesh out Room MUD Mini Game
  - Add encounter scripting, NPC prompts, and automated regressions for the ASCII map walkthrough.
  - Finish workshop doc exercises + troubleshooting, ensure validator passes without TODO markers.
  - Acceptance: workshop reaches 100% guidance coverage with stable inventory/actions.
- WORKSHOP-205-FOLLOWUP: Polish Boxy MUD Panels
  - Improve Boxy tile sizing + palette, integrate prompts, and capture Boxy-specific troubleshooting tips.
  - Extend doc with customization exercises and note Boxy best practices; add tests for emoji layout.
  - Acceptance: Boxy workshop ready for handoff with documented limitations cleared.
- WORKSHOP-206-FOLLOWUP: Debug Zone Enhancements
  - Add cursor hinting, richer diagnostics (e.g., before/after snapshots), and scripted regression coverage.
  - Expand workshop doc with hands-on exercises and troubleshooting checklist.
  - Acceptance: debug workshop graduates with clear guidance and automated verification.
- ROOM-UAT-001: Runtime UAT smoke tests across terminals
  - Execute cross-terminal walkthroughs for `audit_demo`, `runtime_first_paint`, `bootstrap_helper`, `chat_workshop`,
    `boxy_dashboard`, `workshop_screen_navigation`, `workshop_screen_multiscreen`, `workshop_room_bootstrap`, and `pilot_mini_editor`.
  - Capture ANSI/focus issues and log follow-up tasks; keep debt visible until the sweep completes.

(New) Lifecycle & Reliability
- ROOM-611: Implement friendly runtime lifecycle events (see TASKS) — required for docs/ref/LIFECYCLE_PLAN.md alignment.
- ROOM-612: Ship shared Cursor manager + cursor events — unblock cursor/focus automation and workshops.
- ROOM-613: Focus change notifications — expose `FocusChanged` signals for components.
- ROOM-614: Runtime error sink and fatal teardown — add `Error`/`Fatal*` path to keep sessions recoverable.

(Archived from `docs/procs/TASKS.md` on 2025-09-18.)
