# Room Workshop: Debug Zone Explorer

**Status:** Prototype (≈60% complete) — follow-up tracked as WORKSHOP-206-FOLLOWUP in the backlog.

## Overview

`examples/workshop_debug_zone.rs` showcases a minimal editor + status footer paired with a “debug zone” that records every
`set_zone` call. The goal is to make dirty-zone activity visible while exploring focus changes, typing, and log
management inside a single `RoomRuntime` plugin.

## Quick Start

```bash
cargo run --example workshop_debug_zone
```

**Controls**
- Type characters to update the editor zone (`Enter` inserts a newline, `Backspace` deletes).
- `Tab` / `Shift+Tab` toggles focus between the editor and status footer.
- Scroll the mouse wheel (or send crossterm mouse events) to log `[Mouse] …` entries in the debug panel.
- `Ctrl+L` clears the debug log and resets the event counter.
- `Esc` / `Ctrl+Q` exits the workshop.

## Workshop Path

1. **Observe the initial render:** the instructions zone explains controls, the editor invites typing, and the debug
   panel lists the first batch of dirty events.
2. **Type into the editor** to see `set_zone` calls logged as “Editor updated”. Each keypress appends a row to the debug
   log, trimming once the log exceeds 12 entries.
3. **Toggle focus** with `Tab` to watch the status footer update and the debug panel record the focus change.
4. **Clear the log** (`Ctrl+L`) to understand how the workshop resets event counters without recursively logging the
   reset itself.

## Implementation Notes

- The workshop keeps a `VecDeque<String>` of log lines and wraps `set_zone` calls in a helper that records every update.
- Focus management uses `FocusController` and `ensure_focus_registry`, demonstrating how to move focus between zones in a
  `RoomPlugin`.
- The layout dedicates the bottom eight rows to the debug log so zone updates remain visible without scrolling.

## Known Gaps

- Cursor hinting is minimal; the terminal cursor is not repositioned, relying on the prime runtime behavior.
- No syntax highlighting or multi-line editing — the editor is intentionally simple.
- Tests and scripted driver coverage are missing; add once the log semantics are locked in.
- Workshop doc still needs hands-on exercises (e.g., extend the logger, emit custom events) before graduation.

## Next Steps

- Track outstanding work in WORKSHOP-206-FOLLOWUP (backlog). Key items: richer focus affordances, cursor hinting,
  scripted coverage, and expanded exercises.
- Compare with the ASCII MUD and Boxy MUD workshops to demonstrate how different demos surface runtime internals.
