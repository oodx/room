# Socket Transport Strategy Notes

## Goals
- Keep the TCP transport generic. The core driver should only manage connections, buffering, and handing each payload to a strategy.
- Allow multiple strategies (Meteor/ASC100, JSON stub, etc.) without touching the driver.
- Keep outbound frames optional so protocols that only push inbound events can stay minimal.

## Current Implementation (`src/runtime/driver/socket.rs`)
- `SocketDriver<S>` owns the transport. `S: SocketStrategy` handles protocol specifics.
- Strategy trait hooks:
  - `decode` converts an inbound payload into `RuntimeEvent`s.
  - `after_events` (optional) can inspect runtime state after dispatch and return a payload to send back to the client.
  - `encode` turns outbound data into a wire string (newline-delimited for now).
- Default strategy (`JsonEventStrategy`) is an MVP stub for key/resize/tick/paste events encoded as JSON objects. It emits no outbound frames.

## Meteor / ASC100 Integration
- Implement a new `SocketStrategy` that:
  - Accepts ASC100 envelopes (likely `type: "token"` etc.).
  - Uses `ZoneTokenRouter` to apply Meteor tokens to the runtime.
  - Emits ASC100 render frames via `after_events` when dirty zones exist.
- Register that strategy via `SocketDriver::bind` to swap out the default JSON handler.
- Driver already clones the underlying stream for writing; the strategy can choose when/how much to send back.

## Future Enhancements
- Streaming render responses: wrap renderer output into ASC100 diff packets and emit from `after_events`.
- Heartbeats / keep-alive frames: strategies can emit periodic pings by returning data even if no events arrived.
- Async/Non-blocking support: replace `TcpListener` with Tokio version and make strategy async-aware once the runtime exposes async hooks.

## Usage Example (MVP JSON)
```rust
let strategy = JsonEventStrategy;
let driver = SocketDriver::bind("127.0.0.1:9000", runtime, Size::new(80, 24), strategy)?;
driver.run()?;
```

Swap `JsonEventStrategy` with your custom implementation once the Meteor + ASC100 protocol is ready.
