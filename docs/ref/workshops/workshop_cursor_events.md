# Room Workshop: Cursor Event Signals

**Status:** Prototype (≈40% complete) — follow-up tracked as WORKSHOP-207B in the backlog.

## Overview

`examples/workshop_cursor_events.rs` provides a minimal surface to watch cursor lifecycle updates.
Each call to `set_cursor_in_zone`, `show_cursor`, or `hide_cursor` feeds the runtime, which then emits
`CursorMoved`/`CursorShown`/`CursorHidden` notifications. The workshop renders those notifications and
keeps the caret aligned inside a bounded grid so movement is easy to visualize.

## Quick Start

```bash
cargo run --example workshop_cursor_events
```

**Controls**
- Arrow keys move the caret within the grid.
- `Space` toggles visibility.
- `r` resets the caret to the origin.
- `Esc` / `Ctrl+Q` exits.

## Workshop Path

1. **Launch the workshop**: the caret appears at the origin and the log records the initial
   `[CursorMoved]` event produced during bootstrap.
2. **Move the caret** with the arrow keys. Each step updates the field, calls `set_cursor_in_zone`,
   and triggers a `CursorMoved` entry that includes the runtime-computed row/column.
3. **Toggle visibility** with `Space` to confirm `CursorShown`/`CursorHidden` events arrive and the
   caret highlight reflects the runtime state.
4. **Reset with `r`** to double-check that the runtime clamps the cursor inside the field and emits
   the expected lifecycle events.

## Implementation Notes

- The workshop redraws the field whenever the caret moves, placing an ASCII `X` at the local cursor
  coordinates so visual and runtime positions stay in sync.
- Cursor visibility is tracked via the emitted events rather than local assumptions, ensuring the
  log mirrors runtime behaviour.
- The layout dedicates the bottom eight rows to a rolling event log (12 entries) for easy inspection.

## Known Gaps

- Automated coverage is pending; incorporate the sample into ROOM-UAT-002 once focus/cursor sweeps are
  scripted. The log format should stay stable for those checks.
- Future work: add examples of cursor styling (glyph + colour) once the renderer API matures.

## Next Steps

- Track polish in WORKSHOP-207B and link findings back into the lifecycle plan. Coordinate with the
  focus workshop so both demos cover the canonical focus/cursor pathways.
