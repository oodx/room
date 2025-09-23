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
- ROOM-UAT-001: Runtime UAT smoke tests across terminals
  - Execute cross-terminal walkthroughs for `audit_demo`, `runtime_first_paint`, `bootstrap_helper`, `chat_workshop`,
    `boxy_dashboard`, `workshop_screen_navigation`, `workshop_screen_multiscreen`, `workshop_room_bootstrap`, and `pilot_mini_editor`.
  - Capture ANSI/focus issues and log follow-up tasks; keep debt visible until the sweep completes.

(Archived from `docs/procs/TASKS.md` on 2025-09-18.)
