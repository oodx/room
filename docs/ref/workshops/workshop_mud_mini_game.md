# Room Workshop: MUD Mini Game (ASCII)

**Status:** Prototype (≈60% complete) — follow-up work tracked in WORKSHOP-204-FOLLOWUP.

## Overview

`examples/mud_mini_game.rs` introduces a lightweight dungeon crawl built directly on `RoomRuntime`. Arrow keys let you
wander between rooms, number keys trigger contextual actions, and a compact status footer narrates each decision. The
workshop focuses on:

- Structuring application state (rooms, inventory, status) inside a single runtime plugin.
- Using multiple zones to present map, room detail, action list, inventory, and status in parallel.
- Exercising `set_zone` vs `set_zone_pre_rendered` when emitting ANSI content.

## Quick Start

```bash
cargo run --example mud_mini_game
```

**Controls**
- Arrow keys: move north/south/east/west when an exit exists.
- `1` – Inspect the current room.
- `2` – Collect one gem from the pedestal (if available).
- `3` – Drop the last collected gem back into the room.
- `4` – List current inventory contents.
- `Esc` / `Ctrl+Q`: quit the session.

## Workshop Flow

1. **Boot the runtime** and observe the initial render (map on the left, description + menu, inventory list,
   status footer).
2. **Traverse the atrium loop** using arrow keys to verify exits update in the detail panel.
3. **Collect gems** in the Observatory, Gallery, and Vault to see the inventory panel grow.
4. **Drop a gem** and confirm it reappears in the room’s gem list.
5. **Trigger inspect** after moving to ensure the status footer reports the latest description.

## Discussion Points

- The map grid is rendered as plain text with ASCII brackets. Consider using Boxy tiles or pre-rendered ANSI art to
  reduce flicker on larger maps.
- Inventory actions mutate shared state protected by `Arc<Mutex<EditorState>>`-like patterns. Explore splitting the
  plugin into map/inventory panels once the runtime supports multi-plugin coordination for related zones.
- Focus lifecycle still needs attention: the action menu never claims ownership through `FocusController`, so the
  current build only receives the bootstrap `FocusChanged` event. Mirror the pattern from
  `workshop_focus_events` to keep menu focus state in sync with the runtime.
- No scripted encounters exist yet; add NPCs or puzzles once the base navigation loop is hardened.

## TODO / Known Gaps

- Map rendering is static ASCII; a Boxy upgrade lives in a separate prototype (`mud_boxy_game`).
- Action list is linear and assumes a small number of actions; needs pagination or help text for expansion.
- Unit tests and scripted driver coverage are missing — add once mechanics stabilize.
- Workshop doc requires hands-on exercises and troubleshooting before it can graduate from prototype.
- Lifecycle upgrade pending — wire in a `FocusController` so cursor visibility and focus hints follow the runtime
  signals instead of local flags.

## Next Steps

- Follow WORKSHOP-204-FOLLOWUP in `docs/procs/BACKLOG.md` for polish tasks (encounters, workshop exercises, tests).
- Once stable, link this workshop from `docs/ref/workshops/QUICK_START.md` (or equivalent) for onboarding.
