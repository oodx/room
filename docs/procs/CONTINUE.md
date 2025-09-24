# Continue Log – main · Runtime Audit & First-Paint Prototypes

## Summary
- Prototyped three new runtime workshops: `mud_mini_game`, `mud_boxy_game`, and `workshop_debug_zone`, each with draft guides capturing current behaviour (≈60% complete).
- Updated TASKS/BACKLOG/ROADMAP to track follow-up work for the new workshops (polish, exercises, tests) and documented partial status in the new workshop pages.
- Added logging/counter guards to the debug workshop so dirty-zone logs stay bounded and events identify their source (key, focus, mouse, action).

## Current Status
- **Branch**: main
- **Phase**: Runtime & Plugin Phase 2 (observability + hands-on workshops)
- **Progress**: Runtime core + screen manager remain stable; we now have prototype workshops covering multi-screen flows, MUD gameplay (ASCII + Boxy), and dirty-zone inspection. Each workshop is functional but needs exercises, troubleshooting, and additional UX polish before graduation.

## Next Steps (Priority Order)
1. Finish WORKSHOP-204/205/206 polish: add exercises + troubleshooting to the new workshop docs, tune Boxy tile sizing, and add scripted coverage for the debug workshop.
2. Resume WORKSHOP-201A fixes (prompt UX + guidance) so the legacy Boxy path reaches parity with the new material.
3. Outline WORKSHOP-301 (first-paint performance) once fresh runtime baselines are captured, then fold learnings back into `docs/ref/strat/RUNTIME_STRATEGY.md`.

## Blockers / Risks
- WORKSHOP-204/205 remain prototypes; without completing exercises/tests we risk shipping partially documented workflows.
- Boxy dashboard (WORKSHOP-201A) still needs UX fixes and better troubleshooting to match the new workshop quality bar.
- UAT sweep (ROOM-UAT-001) unchanged; cross-terminal verification still outstanding for all workshops/demos.

## Notes for Next Agent
- New workshop docs live at `docs/ref/workshops/workshop_mud_mini_game.md`, `workshop_mud_boxy_game.md`, and `workshop_debug_zone.md`; each lists TODOs and links to follow-up backlog items.
- If you iterate on these workshops, update TASKS/BACKLOG entries (WORKSHOP-204/205/206) and refresh the docs so validator checks stay quiet.
- Debug workshop now bounds the log and labels events; if you expand functionality, keep the log guard in sync and consider scripted tests.
- Boxy/dashboard legacy work (WORKSHOP-201A) and the Phase 2 UAT sweep remain the primary outstanding items beyond workshop polish.
- Continue using `bin/examples.sh run <example>` for quick validation; ensure new demos still opt into bootstrap/audit helpers.
