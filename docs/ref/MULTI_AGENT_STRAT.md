# Multi-Agent Room Strategy

This note sketches how to evolve Room into a multi-agent terminal client. The
scenarios assume "agent" means an independent conversational or task runner
that needs its own UI surface, keyboard focus, and lifecycle.

## Goals

* Support multiple agents concurrently without the demo code duplicating
  boilerplate per agent.
* Preserve Room's focus/cursor semantics so the user can switch panes with
  predictable hotkeys.
* Keep transport flexible: each agent might run locally (CLI driver), in a
  background task (socket), or via a remote API.

## Architectural Options

1. **Multi-runtime orchestrator** — spawn one `RoomRuntime` per agent and run
   them side-by-side, coordinating input/output externally. Each runtime owns
   its own driver loop (typically `CliDriver` or `SocketDriver`). A coordinator
   multiplexes keystrokes, composites rendered output, and handles focus.
2. **Single runtime with panes** — extend Room to manage multiple agent panes
   under one driver. Each pane is effectively a runtime-in-miniature with its
   own plugin stack, tick schedule, and shared state namespace. Screen manager
   (or a new PaneManager) routes events to the active pane.

Both approaches benefit from an `AgentRuntime` abstraction that wraps layout,
plugin setup, and driver configuration so agents can be spun up consistently.

## Decision Points

### Agent Lifecycle

* What triggers agent creation/destruction? User commands, background tasks,
  or network events?
* Do agents run headless (no UI) when not visible? If yes, you may want a
  shared scheduler that feeds ticks even off-screen.

### Focus Management

* How does keyboard focus move between agents/panes? Reuse existing hotkeys
  (Ctrl+Tab, etc.) or define new commands?
* Do agents know about each other (to adjust overlays/status) or are they
  isolated?

### Transport

* CLI driver: simplest path for local agents. Each runtime runs in its own
  terminal or multiplexed PTY.
* Socket driver/API: if agents live in separate processes or machines, a socket
  or WebSocket bridge sends `RuntimeEvent`s and receives render snapshots.
* REPL/daemon: for debugging, you may run a single runtime and feed it scripted
  events while the agent logic runs elsewhere.

### Shared Data

* If agents need to share state (e.g., conversation history), consider a shared
  store outside the runtime so multiple agent contexts can read/write.
* For UI-only coordination (like showing "Agent B is typing"), the orchestrator
  can push status messages into each runtime's status zone.

## Recommended Layers

```text
+---------------------------+
| Multi-Agent Orchestrator  |  (focus switching, hotkeys, pane layout)
+---------------------------+
| AgentRuntime Abstraction  |  (wraps RoomRuntime + plugins per agent)
+---------------------------+
| Room Runtime / Drivers    |  (existing Room MVP code)
+---------------------------+
```

### AgentRuntime (suggested responsibilities)

* Build the agent's layout tree and register plugins.
* Configure driver specifics (tick interval, default focus zone, audit hooks).
* Provide start/stop methods returning a handle (for CLI or socket mode).

Mastering this layer means you can swap out the underlying driver (CLI, socket,
repl) without touching agent logic.

## Implementation Steps

1. **Wrap existing demos** — extract the setup logic from chat/mud demos into a
   reusable `AgentRuntime` builder. Ensure it can run under `CliDriver` or a
   scripted harness.
2. **Add orchestrator** — write a small manager that can host two agents:
   * Option A: Each agent gets its own PTY; orchestrator just launches them.
   * Option B: One runtime, multiple panes. Prototype pane-swapping using
     `ScreenManager` or an extension thereof.
3. **Transport experiments** — build a socket server (see
   `src/runtime/driver/socket.rs`) that proxies `RuntimeEvent`s and render
   snapshots for an agent. Use it to drive an agent from a remote process.
4. **Focus/broadcast hooks** — add global commands (e.g., `/switch 2`) that
   instruct the orchestrator to give focus to another agent. Broadcast simple
   notifications ("Agent 1 replied") into other panes.
5. **Persisted sessions (optional)** — to mimic tmux, add session serialization
   (store agent state to disk) and a control channel (socket) to detach/attach.

## Missing Runtime Features (Future Work)

* Pane/window manager inside Room runtime (multiple render surfaces per driver).
* Scrollback buffer per pane.
* Detached sessions and reconnection (requires socket driver and persistent
  state).
* Shared event bus for cross-agent communication.

Until those land, the multi-runtime orchestrator is the pragmatic route: it
keeps the core runtime untouched while you iterate on multi-agent UX.
