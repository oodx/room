# Room Workshop: Focus Event Signals

**Status:** Prototype (≈40% complete) — follow-up tracked as WORKSHOP-207A in the backlog.

## Overview

`examples/workshop_focus_events.rs` isolates the runtime focus lifecycle. It pairs three panes (PrimaryA,
Secondary, Review) with a `FocusController` so you can confirm how `FocusChanged { from, to }` events fire
as ownership rotates between zones or is released entirely.

## Quick Start

```bash
cargo run --example workshop_focus_events
```

**Controls**
- `Tab` advances focus (PrimaryA → Secondary → Review).
- `Shift+Tab` moves focus back.
- `Space` releases focus so the registry reports `<none>`.
- `Esc` / `Ctrl+Q` exits.

## Workshop Path

1. **Launch the workshop** and watch the PrimaryA pane receive default focus. The log records the
   bootstrap transition from `<none>` to `workshop:focus.primary_a`.
2. **Cycle focus with `Tab`/`Shift+Tab`** to see focus ownership hop between the Primary, Secondary,
   and Review panes. Each change logs a `[FocusChanged] from -> to` line and updates the headers to
   reflect the active owner.
3. **Press `Space`** to release focus. The registry clears, the log shows `<none>`, and each pane drops
   its highlight — a quick way to validate the “no owner” path in the lifecycle.

## Implementation Notes

- The plugin stores a single `FocusController` keyed to `focus-workshop`, ensuring there is exactly one
  controller per owner entry in the registry.
- Focus changes are rendered purely off the lifecycle signal — the workshop does not mutate local state
  until `on_focus_change` fires. This keeps event ordering honest.
- Logs live in a `VecDeque<String>` so the most recent 12 events remain visible without scrolling.

## Known Gaps

- No scripted verification yet; wire up a driver once ROOM-UAT-002 covers focus lifecycles end-to-end.
- Pane content is intentionally minimal. Future iterations should add exercises that integrate focus with
  actionable UI (lists, toggles) before graduation and demonstrate multi-owner patterns.

## Next Steps

- Track polish in WORKSHOP-207A. Planned improvements: exercises, automated coverage, and parity with the
  cursor workshop once ROOM-UAT-002 is complete.
