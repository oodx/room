# Room Runtime Strategy

This note captures the working plan for turning Room from a set of demos into a
general-purpose runtime that can power fancy CLI dashboards, remote/socket
interfaces, and future tmux-like shells. It focuses on the lifecycle, extension
points, and the tooling we need for observability and performance.

## Goals

- Keep the existing layout/registry/renderer core lean and reusable.
- Introduce a runtime coordinator that owns the event loop and exposes hooks.
- Make the system plugin-friendly so features like slash commands, modal
  overlays, or boxy focus can ship "batteries included".
- Support multiple drivers (local CLI, socket server, scripted tests) without
  rewriting the core.
- Provide instrumentation so we can measure layout/render performance and debug
  complex event flows.

## Runtime Skeleton

### Core Ownership

`RoomRuntime` (working name) owns:

- `LayoutTree` – compiled either programmatically or via a DSL.
- `ZoneRegistry` – tracks rects and dirty state.
- `AnsiRenderer` (or other backend) – applies minimal/atomic updates.
- Input/event sources – stdin, sockets, timers, scripted feeds.
- Plugin registry – ordered list of registered extensions.

### Lifecycle Loop

```
loop {
    events = collect_events();               // stdin, timers, sockets, etc.
    dispatch_events(events);                 // give plugins the first crack
    process_token_updates();                 // apply batched zone content
    render_dirty();                          // minimal diff render
    wait_for_next_event();                   // poll/sleep until something new
}
```

Key points:

- Resize detection feeds straight into `dispatch_events`.
- Plugins can request renders by marking zones dirty or emitting tokens.
- All zone updates land in the registry before a single render pass, keeping
  flicker-free atomic updates.

### Event Model

Define an `Event` enum that covers keyboard input, slash commands, timers,
network packets, focus changes, etc. Plugins opt-in to the variants they care
about. If a plugin handles an event fully (e.g., modal captures Esc), it can
consume it to stop propagation.

### Plugin Hooks

A `RoomPlugin` trait could expose:

- `on_register(&mut RuntimeCtx)` – initial setup, bind commands, allocate state.
- `on_event(&mut RuntimeCtx, &Event)` – react to user input or data streams.
- `before_render` / `after_render` – overlay content, diagnostics, focus rings.
- `on_tick` – periodic work when the runtime is idle.

The runtime exposes a `RuntimeCtx` that lets plugins:

- Push zone content or emit structured tokens.
- Read/write shared state (e.g., modal status, focus owners).
- Schedule timers or subscribe to event sources.

### Drivers / Adapters

Different binaries instantiate the runtime with different stacks:

- **CLI Driver**: crossterm-based input + default plugins (status bar, chat
  input). Hands raw keystrokes to the runtime.
- **Socket Driver**: async reader (Tokio/Tungstenite/etc.) that converts inbound
  messages into `Event::Network` for plugins that care. Local terminal can be
  optional (headless).
- **Scripted/Test Driver**: feed canned event sequences for CI/regression.

Because all drivers funnel through the same runtime lifecycle, features remain
portable.

### Bootstrap Helpers

- `BootstrapAudit`: Wrap any audit sink to buffer lifecycle events until the
  first render (or an override stage) so demos/tests do not spam logs before the
  UI appears. All runtime examples should opt into this helper by default.
- `BootstrapControls`: Request fine-grained control over bootstrap without
  immediately presenting the first frame. Callers can present or delay the
  initial render, pump a fixed number of synthetic ticks, or gate startup on the
  first key event before resuming the normal driver flow. See
  `examples/bootstrap_helper.rs` for a scripted usage pattern and migrate new
  demos/tests through this API as they land.
- `cursor` utilities: Use helpers such as `move_down_lines` or `move_to` to
  reposition the terminal cursor after printing captured frames so prompts stay
  tidy when the runtime exits.

## Declarative Layout Plan

Introduce a DSL (working name `layout-html`) that compiles into a
`LayoutTree + metadata` bundle:

- Zones specify constraints, identifiers, and channel bindings.
- Plugins can register DSL components (e.g., `<chat-input channel="app:input"/>`)
  which expand into layout subtrees plus associated hooks.
- Runtime loads DSL → builds layout → registers required plugins automatically.

This gives downstream projects a batteries-included experience: deliver a DSL
file + choose plugins + wire token streams, and the runtime does the rest.

## Observability & Tooling

### Benchmarking

To make sure the layout engine and runtime scale, we need reproducible micro
and macro benchmarks:

- **Layout Benchmarks**: solve increasingly complex trees (deep nesting, wide
  panes) under different terminal sizes, measuring time per solve. Capture
  percentile stats (p50/p95/p99) via Criterion and compare against baselines.
- **Render Benchmarks**: simulate dirty-zone workloads (single zone, many
  zones, full redraw) and measure both time and bytes written.
- **End-to-End Loops**: replay recorded event streams to measure latency from
  input → render (and the number of render passes per second).
- **Plugin Benchmarks**: microbenchmarks for high-traffic plugins (chat input,
  slash commands, panes) to make sure handler overhead stays bounded.

Use Criterion or similar to collect statistics. Wire benchmarks into CI to catch
regressions. For longer sessions, surface periodic summaries (e.g.
`layout_ms_avg`, `renders_per_min`) via the metrics/logging system.

### Logging & Event Tracing

**Objectives**

- Capture enough detail to debug plugin interactions and rendering order
  without spamming stdout.
- Keep overhead low so logging can stay enabled in development builds.

**Plan**

1. Provide a feature-gated `room_logger` facade that can log to rotating files
   or custom sinks.
2. Emit structured JSON lines with fields: timestamp (monotonic + wall-clock),
   event name, plugin/zone identifiers, payload.
3. Support log levels so trace-heavy output can be toggled per plugin.
4. Offer an opt-in "trace" mode that captures raw events, registry snapshots,
   and (optionally) ANSI output for replay.
5. Bundle helper scripts to pretty-print/filter logs and correlate them with
   benchmarks.

Longer term we can integrate with `tracing` so downstream apps hook into their
existing observability stacks.

### Metrics

- Track counts like render FPS, dirty-zone counts, plugin execution time.
- Hook into the logging pipeline or surface them via a `metrics` plugin that can
  render its own dashboard panel.

## Next Steps

1. Prototype `RoomRuntime` with minimal hooks and port `chat_demo` onto it.
2. Extract workshop behaviors (slash commands, preferences modal) into plugins.
3. Implement logging/telemetry scaffolding inside the runtime.
4. Stand up Criterion benchmarks for solver and renderer.
5. Design the first iteration of the layout DSL and prove it with the chat demo.
6. Expand plugin library (status bar, split panes, boxy window manager) and add
   CLI + socket drivers.

With this plan, Room stays a lightweight engine, but gains the lifecycle,
hooks, and tooling we need to power both local dashboards and remote-first
experiences.
