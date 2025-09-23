# Workshop · Multi-Screen Navigation

This workshop introduces the new `ScreenNavigator` helpers and default navigation
hotkeys wired through `ScreenManager`.

## Launch

```bash
cargo run --example workshop_screen_navigation
```

## Exercises

1. **Cycle with default hotkeys** – Use `Ctrl+Tab` / `Ctrl+Shift+Tab` (or
   `Ctrl+BackTab`) to move forward/backward across screens. Watch the visit counter
   update independently for each screen.
2. **Jump via digits** – Press `1`, `2`, or `3` to request an explicit activation
   using `ScreenState::navigator`. Digits map to the order screens were registered.
3. **Inspect state isolation** – Toggle between screens while spamming `i` to
   refresh the body content. The visit counters remain scoped per screen, proving
   the `ScreenState` namespace isolation.
4. **Re-run lifecycle** – Hit `!` to re-render the info panel with the current
   screen ID and state handle identifier to confirm lifecycle hooks are still
   routed through the active strategy.

## Takeaways

- `ScreenManager` exposes `ScreenNavigator` via `ScreenState::navigator()` so
  strategies (and their panels/plugins) can request screen changes without
  touching the runtime directly.
- Default keyboard shortcuts now ship (`Ctrl+Tab`, `Ctrl+Shift+Tab`,
  `Ctrl+BackTab`) and are handled before the active screen sees the event.
- Per-screen state persists across activations thanks to the shared
  namespace adaptor introduced with `ScreenState`.
