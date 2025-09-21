# room

Pilot workbench for a flicker-free terminal layout engine driven by RSB token streams.

## Components
- `room`: experimental crate with layout solver, zone registry, renderer, and token router.
- `docs/ref/STRATEGY.md`: architectural notes, verification checklist, and follow-up items.
- `docs/procs/TASKS.txt`: story-point backlog with completion status.

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

## Architecture

Room's architecture is built around several key components:
- Layout Solver: Intelligent terminal space allocation
- Zone Registry: Dynamic interface region management
- Renderer: Efficient, minimal-flicker content drawing
- Token Router: State-driven interface updates

Detailed architectural notes live in `docs/ref/STRATEGY.md`.

## Documentation

- 📁 `docs/procs/`: Process and status docs (START, PROCESS, CONTINUE, SPRINT, TASKS, ROADMAP, DONE)
- 📚 `docs/ref/`: Reference notes (`CORE_PLUGIN_STRATEGY.txt`, `FEATURES_RUNTIME_PHASE2.txt`, `PLUGIN_API.txt`, `SOCKET_STRAT.txt`, `SHARED_RUNTIME_STRAT.txt`, etc.)
- 🥚 `.eggs/`: Generated project summaries and agent analysis outputs

## Development Status

**Current Phase**: MVP Development
- [x] Basic layout engine
- [x] Token stream routing
- [x] Demo implementations
- [ ] Advanced plugin system
- [ ] Performance optimization

## Tasks & Roadmap

Track ongoing development in `docs/procs/TASKS.txt`. Story points and completion status are regularly updated; see `docs/procs/ROADMAP.txt` for phase-level goals.
