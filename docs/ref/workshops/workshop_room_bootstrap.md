# Workshop 203 · Runtime Bootstrap & Cleanup

This workshop demonstrates the expected UX flow for Room runtime initialization:
preparing the first frame, streaming audit events after the render, running the
interactive loop, and exiting cleanly so the shell prompt returns to a sane
position.

## Prerequisites
- Terminal that supports crossterm raw mode (alternate screen + cursor control)
- Familiarity with `examples/bootstrap_helper.rs` and audit helpers

## Running the Example
```bash
cargo run --example workshop_room_bootstrap              # full interactive flow
cargo run --example workshop_room_bootstrap -- capture   # capture-only walkthrough
```

During the run you will see audit output in the host terminal. The example does
not depend on Boxy, so it can run in minimal environments.

## Exercise 1 · Capture the First Frame
1. Run the workshop with the `capture` flag to view the pre-render snapshot.
2. Inspect the captured block—no raw-mode sequences have been emitted yet, but
   plugins populated the status zone.
3. Notice the cursor utility (`cursor::move_down_lines`) used to reposition the
   prompt after displaying the capture.

**Goal:** understand how `BootstrapControls` stages the first render before any
interactive driver takes over.

## Exercise 2 · Audit the Bootstrap Flow
1. Run the interactive version (without the `capture` flag).
2. Watch the `[AUDIT]` events stream alongside the runtime output.
3. Observe how the status zone tracks the bootstrap tick count before the
   runtime loop starts.

**Goal:** validate that audit gating keeps the terminal quiet until the first
frame commits.

## Exercise 3 · Exit Cleanly
1. With the runtime active, press `Esc` or `q` to request exit.
2. Confirm that the alternate screen is released and the shell prompt resumes
   immediately below the workshop’s final status message.
3. Examine the source to see how the cursor utilities help maintain terminal
   hygiene after teardown.

**Goal:** ensure Room leaves the terminal in a predictable state after the run.

## Bonus Exploration
- Modify the plugin to write additional lifecycle notes (e.g., ticks vs. events).
- Integrate `cursor::save_position` / `restore_position` to explore complex
  prompt rewrites.
- Replace the plain-text status zone with a Boxy panel for richer formatting.

## Wrap-Up
When finished, record observations in `docs/procs/DONE.md` (or link follow-up
work in `docs/procs/TASKS.md`) so future sessions know how bootstrap hygiene
was validated.
