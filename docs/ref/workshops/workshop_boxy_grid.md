# Workshop 202 · Boxy Grid Layouts

This workshop expands on `examples/workshop_boxy_grid.rs` to practise composing
multi-panel Boxy layouts using Room's layout solver. The example now uses the
Room renderer to paint the panels directly in their solved rectangles, so run
it in a real terminal to see the boxes snap into position.

## Prerequisites
- `cargo run --example workshop_boxy_grid` (or `bin/examples.sh run workshop_boxy_grid`)
- Familiarity with `docs/ref/workshops/workshop_boxy_dashboard_runtime.md`
- Optional: review `docs/ref/LAYOUT_ENGINE_STRATEGY.md` layout section

## Scenario A · 2x2 Grid
1. Run the default scenario:
   ```bash
   bin/examples.sh run workshop_boxy_grid
   ```
2. Inspect the rect output and the rendered grid; note how padding=1 and gap=1
   reduce usable space and how the fixed panel heights clip overflow text.
3. Modify `LayoutTree` to make the top row `Fixed(10)` and observe the panel heights.
4. Extend `panels` to include glyphs or status text that reflects the rect dimensions.

**Discussion:**
- Flex rows split height evenly after subtracting padding/gaps.
- Nesting `Direction::Row` inside a column yields a grid without special APIs.

## Scenario B · Wide Metric Strip
1. Run the wide configuration:
   ```bash
   bin/examples.sh run workshop_boxy_grid -- wide
   ```
2. Observe width distribution (Flex(2) + Flex(1) + Flex(1)).
3. Add a fixed width (e.g., `WidthConfig { fixed_width: Some(18), enable_wrapping: true, .. }`)
   to the metric panel and re-run to confirm the renderer still wraps inside the layout rect.
4. Experiment with terminal width changes by editing `Size::new(120, 20)`.

**Discussion:**
- Flex weights act like ratios; with `[2,1,1]` the main panel gets 50% of the row.
- Fixed widths trump flex but respect terminal bounds—use this for nav rails/sidebars.

## Bonus Challenges
- Add a third scenario representing a responsive dashboard (e.g., 3x3 grid).
- Wire these boxes into the Room runtime similar to `boxy_dashboard_runtime.rs`.
- Try mixing percent constraints with flex to mirror CSS grid templates.

## Wrap-Up
- Log interesting findings in `docs/procs/DONE.md` and consider adding automated
  tests or benchmarks for desired grid configurations.
