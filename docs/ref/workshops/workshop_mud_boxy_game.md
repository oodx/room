# Room Workshop: Boxy MUD Mini Game

**Status:** Prototype (≈60% complete) — polish tracked via WORKSHOP-205-FOLLOWUP.

## Overview

`examples/mud_boxy_game.rs` layers Boxy-rendered panels on top of the Room runtime to create a colorful dungeon
exploration demo. Emoji treasures decorate each room, the detail panel narrates discoveries, and the navigation bar
summarises available actions. The workshop illustrates:

- Mixing Boxy-rendered tiles with standard Room zones using `set_zone_pre_rendered`.
- Building a map composed of multiple Boxy panels stitched into a single render target.
- Coordinating navigation, inventory, and status feedback within one plugin.

## Quick Start

```bash
cargo run --example mud_boxy_game
```

**Controls**
- Arrow keys: move between rooms (if an exit exists).
- `1` – Inspect the current room.
- `2` – Collect one emoji treasure (when present).
- `3` – Drop the most recently collected emoji back into the room.
- `4` – Recite current inventory contents.
- `Esc` / `Ctrl+Q`: quit the session.

## Guided Walkthrough

1. **Observe the map row:** each visible room is rendered as a Boxy tile. The currently occupied room is gold with a
   highlighted header.
2. **Traverse the gallery loop:** walk from Atrium → Gallery → Garden → Vault to see tiles update and emojis move.
3. **Collect & drop treasures:** use actions `2` and `3` to move emoji items between the tile and the inventory panel.
4. **Inspect inventory:** confirm the inventory panel is kept in sync, and note how the navigation bar remains visible.
5. **Review status footer:** every move or action writes a short status update; identify cases where messaging should be
   expanded before release.

## Architecture Notes

- Each map cell is rendered by `render_room_tile` which clones a `BoxyConfig` template, tweaks colors, and calls
  `render_to_string`. The resulting lines are stitched together to form the map zone content.
- Navigation and inventory rely on the same runtime plugin as the ASCII version; consider splitting into dedicated
  plugins once multi-plugin coordination patterns solidify.
- We rely on fixed `WidthConfig::fixed_width` values — future polish should derive width/height from the solved rects to
  better handle terminal resizing.
- Focus lifecycle is still pending: the navigation bar never acquires ownership through `FocusController`, so the
  current build only receives the bootstrap `FocusChanged` event. Port the approach from `workshop_focus_events` before
  treating this as production-ready guidance.

## TODO / Known Gaps

- Boxy sizing is static; narrow terminals may clip tiles or wrap emoji awkwardly.
- Focus cues are limited to color changes. Add additional affordances (e.g., footers, borders) before graduation.
- No error handling when actions are spammed; inventory duplicates can occur if future mechanics expand.
- Workshop exercises and troubleshooting are still pending; needs examples covering Boxy palette tweaking, wrapping, and
  navigation hints.
- Lifecycle integration TODO — wire up a `FocusController` so cursor hints match runtime state instead of relying on
  local flags.

## Next Steps

- See WORKSHOP-205-FOLLOWUP in `docs/procs/BACKLOG.md` for outstanding tasks (palette tuning, documentation, tests).
- Once stabilized, cross-link from the Boxy workshop index so contributors can compare ASCII vs Boxy implementations.
