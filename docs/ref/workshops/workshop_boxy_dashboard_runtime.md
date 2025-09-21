# Workshop 201 · Boxy Dashboard Runtime

This workshop builds on `examples/boxy_dashboard_runtime.rs` to explore focus
management, shared state, and live panel updates within the Room runtime.

## Prerequisites
- Terminal that supports crossterm raw mode (the example runs interactively)
- Room repo checked out with the Boxy dependency available (see README requirements)
- Familiarity with `bin/examples.sh`

## Running the Example
```bash
bin/examples.sh run boxy_dashboard_runtime
```
Keyboard shortcuts:
- `Tab` / `Shift+Tab`: cycle focus through panels and the prompt
- `Ctrl+R`: force refresh of status/metrics/log panels
- `Enter`: add prompt text to the activity log (does not clear input)
- `Ctrl+Enter`: submit and clear the prompt while keeping focus
- `Ctrl+C`: exit the runtime

## Exercise 1 · Focus Cycling
1. Start the example and press `Tab` several times; watch the footer reporting the current focus zone.
2. Use `Shift+Tab` to move focus backward.
3. Inspect `BoxyDashboardPlugin::rotate_focus` and `apply_focus` to see how focus is persisted via `FocusController`.

**Goal:** understand how Room coordinates focus between multiple Boxy panels and the prompt.

## Exercise 2 · Panel Refresh & Heights
1. Trigger `Ctrl+R` to force a refresh.
2. Observe how `cycle_panels` updates colors, titles, and status text.
3. Resize your terminal or adjust `Size::new(120, 34)` to see `recompute_heights` clamp panel heights using `compute_panel_height()`.

**Goal:** learn how dynamic panel content integrates with Room’s layout + Boxy rendering.

## Exercise 3 · Prompt Interaction & Logging
1. Type into the prompt and press `Enter`—messages appear in the activity log panel.
2. Try `Ctrl+Enter` to submit while maintaining focus.
3. Inspect the `BoxyPrompt` implementation for buffer management and cursor glyph handling.

**Goal:** explore shared state between the prompt and log panels, noting how Boxy config changes mark panels dirty.

## Bonus Exploration
- Modify the panel list in `init_panels` (e.g., add a new panel or change colors) and observe behaviour.
- Change the tick interval (`runtime.config_mut().tick_interval`) and confirm metrics update cadence.
- Integrate custom glyphs or metrics by editing `cycle_panels` and `metrics_text`.

## Wrap-Up
- Record any insights in `docs/procs/DONE.md` when you finish the workshop.
- Next workshop: First Paint Performance (WORKSHOP-301) leveraging `examples/runtime_first_paint.rs`.
