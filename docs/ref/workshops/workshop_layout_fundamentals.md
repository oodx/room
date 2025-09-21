# Workshop 101 · Layout Fundamentals

This workshop accompanies `examples/workshop_layout_fundamentals.rs`. It introduces
three core scenarios that highlight how Room's `LayoutTree` distributes space across
terminal zones. Work through the exercises in order and modify the example between runs
to explore the solver's behaviour.

## Prerequisites
- Rust toolchain + Room workspace checked out
- Familiarity with `cargo run --example <name>`
- Optional: open `docs/ref/LAYOUT_ENGINE_STRATEGY.md` for deeper architecture notes

## Running the Workshop
```bash
cargo run --example workshop_layout_fundamentals                  # Scenario 1 (basic)
cargo run --example workshop_layout_fundamentals -- with-gap      # Scenario 2 (gap + padding)
cargo run --example workshop_layout_fundamentals -- nested        # Scenario 3 (nested sidebar)
```
Each run prints the solved rectangles for a synthetic `Size::new(80, 24)` terminal and
suggests follow-up experiments.

---

## Scenario 1 · Header / Body / Footer
**Goal:** understand fixed vs. flexible constraints.

- Layout: three stacked rows with `Fixed(3)`, `Flex(1)`, `Fixed(3)`.
- Exercise A: increase the terminal height to 40 and observe the body rect.
- Exercise B: replace `Flex(1)` with `Percent(50)` and note the change.
- Expected Outcome: the body consumes all remaining height when `Flex(1)` is used.

**Discussion:**
- Flex constraints divide leftover space proportionally.
- Fixed sections clip or underflow if total demand exceeds terminal size.

---

## Scenario 2 · Padding & Gaps
**Goal:** observe how padding and inter-child gaps affect allocation.

- Layout: column with 1-row gap between toolbar/content/status and padding of 1.
- Exercise A: set padding to 2 and inspect how width/height shrink.
- Exercise B: remove the gap and compare the resulting rects.
- Exercise C: change the middle constraint to `Min(10)` and rerun.

**Discussion:**
- Padding is removed from the available axis length before constraints are applied.
- Gap multiplies by `children.len() - 1`; large gaps can starve flex children.

---

## Scenario 3 · Nested Sidebar Timeline
**Goal:** practise composing layouts by nesting row/column nodes.

- Outer column: header/body/footer similar to Scenario 1.
- Inner row: fixed-width sidebar (`Fixed(24)`) + flexible timeline (`Flex(2)`).
- Exercise A: swap the timeline constraint to `Flex(1)` and add a third panel.
- Exercise B: simulate a narrower terminal (`Size::new(60, 24)`) and evaluate the sidebar.
- Exercise C: add padding to the outer column; confirm rectangles adjust as expected.

**Discussion:**
- Nested layouts allow you to mix row/column axes arbitrarily.
- Fixed widths should be validated against minimal terminal dimensions.

---

## Reflection & Next Steps
- Capture any interesting solver behaviours (e.g., overflow cases) in `docs/ref/LAYOUT_ENGINE_STRATEGY.md`.
- Extend the example with your own scenario (e.g., percent-based grids) and share learnings in `docs/procs/DONE.md` when promoting the workshop.
- Ready for more? See `docs/procs/TASKS.md` for upcoming workshops (Boxy dashboard, first paint performance).
