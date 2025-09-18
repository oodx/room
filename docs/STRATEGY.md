# Room Layout Engine MVP Strategy

## Goals
- Map RSB/Meteor-style token streams to named layout zones with minimal coupling.
- Compute integer-based layouts and render updates without full-screen redraws or flicker.
- Maintain a persistent bottom chat input bar while other regions update independently.
- Leverage existing tooling (RSB token bucket, XStream routing, Boxy width helpers) and align with RSB testing conventions.

## Data Flow Overview
```
Token Stream → TokenBucket → Layout Graph → Zone Registry → Renderer
      ↑             ↑             ↓                 ↓           ↓
 XStream gates   Meteor ctx   Constraint solve   Dirty tracking  ANSI output
```

1. **Ingest**: Parse incoming token stream (`k=v;prefix:k=v`) with `TokenBucket::from_tokens`, respecting Meteor context/namespace rules for forward compatibility.
2. **Transform**: Use XStream combinators to fork tokens by zone prefix and merge updates into per-zone state payloads.
3. **Layout Solve**: Build a tree of `Node` structures (direction, constraints, children). Run integer constraint solver to produce `Rect { x, y, w, h }` for each zone.
4. **Register Zones**: Store zones in a registry keyed by fully qualified namespace (e.g., `app:chat.timeline`). Each entry keeps the rect, last rendered buffer hash, and cursor anchor.
5. **Render**: For each dirty zone, use Boxy width utilities to wrap/clip text to the rect, emit ANSI cursor position (CSI `[{row};{col}H`), draw diff, and restore cursor for input bar.

## Architectural Components

### 1. Token Adapter Layer
- Provide a `TokenFeed` trait wrapping RSB buckets or Meteor contexts.
- Support prefix routing: map `chat.timeline:*` tokens to timeline zone, `chat.sidebar:*` to sidebar, etc.
- Apply namespace depth warnings as defined in `TOKEN_NAMESPACE_CONCEPT.md` (warning at 4, error after 4).

### 2. Layout Solver
- Enumerate constraint types: `Fixed(u16)`, `Percent(u8)`, `Min(u16)`, `Max(u16)`, `Flex(u8)`.
- Support vertical (`Column`) and horizontal (`Row`) containers with optional `gap` and `padding`.
- Allow nested layouts by associating child nodes with parent zone ids.
- Provide deterministic allocation order: fixed → min/max clamp → percent → flex.
- Output `LayoutResult` containing zone rects + overflow flags.

### 3. Zone Registry
- Structure: `HashMap<ZoneId, ZoneState>` where `ZoneState` holds rect, z-order (for future modals), last hash, and optional viewport state (scroll offset for timeline).
- API:
  - `apply_tokens(&mut self, tokens: &[Token])` → mark zones dirty.
  - `iter_dirty(&self)` → collect zones needing redraw.
  - `update_hash(&mut self, zone_id, new_hash)` after rendering.
- Persist chat input zone even when other regions reload.

### 4. Renderer
- Provide `RendererBackend` trait with default ANSI implementation (using `std::io::Write`).
- Steps per zone:
  1. Move cursor to `rect.y + padding_top`, `rect.x + padding_left` using CSI sequences.
  2. Render content line-by-line, truncating width with Boxy `unicode_width` helpers.
  3. Pad remaining lines with spaces to fully cover rect (avoid ghost pixels).
  4. Restore cursor to chat input prompt after updates.
- Offer fallbacks for terminals without cursor positioning (full redraw) guarded by feature flag.

### 5. Chat Input Integration
- Reserve bottom `FooterZone` with height=3 (status, input, instructions).
- Maintain separate input buffer and editing state; on every loop, re-render footer last to keep cursor anchored.
- Use XStream timed gate to throttle input echo tokens vs. layout updates.
- Provide `InputController` handling key events, storing unsubmitted message, pushing tokens (`chat.input:text`) back into main channel.

## Update Loop Strategy
```
loop {
    drain token stream → bucket
    registry.apply_tokens(bucket)
    if terminal resized → recalc layout + mark all zones dirty except footer anchor
    for zone in registry.iter_dirty() {
        renderer.render(zone)
    }
    renderer.restore_cursor(footer.prompt_cursor)
    flush stdout
}
```
- Differential updates rely on hashing normalized content (`blake3` or `xxhash`).
- For streaming zones (chat timeline), maintain ring buffer per zone; only render new rows appended since last hash.
- Use event loop with non-blocking input (e.g., `crossterm::event::poll`) to capture user typing while streaming tokens.

## Testing Approach
- Unit tests for constraint solver using synthetic terminal sizes, verifying sum of widths/heights equals container minus padding/gap.
- Registry tests ensuring repeated identical tokens do not trigger redraws.
- Renderer snapshot tests (capture ANSI output) comparing expected cursor commands.
- Integration test via example binary feeding scripted token stream to confirm chat input remains stable.
- Hook into RSB-style harness: add `cargo test --package room --all-targets` and optional `bin/test_room.sh` wrapper calling `bin/test.sh` expectations.

## Implementation Notes
- Place prototype crate under `pilot/room_mvp/` until stabilized (per AGENTS.md guidance).
- Respect ASCII-first output but allow Boxy width calculators for multibyte strings.
- Document namespace expectations and bracket transformations in README and CONTINUE notes.
- Monitor for future Meteor migration; keep context handling feature-gated for now (`meteor_namespace` feature flag).

## Open Questions
- Do we need off-screen buffers for diffing more complex widgets (tables)?
- What is the policy for overlapping zones (modals)? Plan to add z-index support in v2.
- Should chat timeline support scrollback? Possibly using registry viewport metadata.

## Implementation Snapshot (2025-09-18)

- Crate scaffolded under `pilot/room_mvp` with modules: `layout`, `registry`, `render`, `tokens`, and `zone` following RSB `MODULE_SPEC` conventions.
- Constraint solver supports fixed/percent/min/max/flex constraints, padding, gap, and arbitrary nesting; see `layout::tests::*` for guard cases.
- Zone registry tracks rects + hashed buffers to guarantee flicker-free diffs; `registry::tests` verifies dirty detection.
- Renderer streams ANSI cursor targets through Boxy width helpers ensuring multi-width glyphs stay aligned.
- Token router consumes RSB streams (`ctx`/`ns` tokens) and folds into zone updates; test demonstrates context switching.
- Chat demo (`cargo run --example chat_demo`) wires everything together with resize handling and live input pinned to the footer zone.

## Verification Checklist

- `cargo fmt` — formatting gate
- `cargo test` — unit coverage for solver, registry, renderer, tokens
- `cargo run --example chat_demo` — manual smoke of selective updates & input bar

## Immediate Follow-ups

- ROOM-501: add bin/test glue so the pilot matches RSB harness conventions.
- ROOM-502: expand README/dev notes with setup steps, token prefix guidelines, and troubleshooting for terminal quirks.

## Next Actions
1. Create pilot crate with module skeleton (layout, registry, renderer, demo).
2. Implement minimal solver & registry for static layout to validate pipeline.
3. Build chat demo harness and iterate on diffing correctness.
4. Expand tests and documentation before promoting out of pilot.
