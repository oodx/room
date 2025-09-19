# room

Pilot workbench for a flicker-free terminal layout engine driven by RSB token streams.

## Components
- `room`: experimental crate with layout solver, zone registry, renderer, and token router.
- `docs/STRATEGY.md`: architectural notes, verification checklist, and follow-up items.
- `TASKS.txt`: story-point backlog with completion status.

## Quickstart
```bash
cargo fmt --manifest-path room/Cargo.toml
cargo test --manifest-path room/Cargo.toml
```

## Demo
Run the chat dashboard demo (uses ANSI cursor positioning and raw mode):
```bash
cargo run --example chat_demo --manifest-path room/Cargo.toml
```
Press `Esc` to exit the demo.

### Boxy dashboard demo

Render three focusable Boxy panels with status updates and cursor tracking:

```bash
cargo run --example boxy_dashboard --manifest-path room/Cargo.toml
```
Requires the local `projects/boxy` and `projects/rsb` checkouts; press `Esc` to exit, `Tab` to cycle focus.
