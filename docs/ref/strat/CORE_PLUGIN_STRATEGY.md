# Core Plugin Strategy

## Goals
- Identify the small, reusable plugin primitives that should live in the Room runtime crate.
- Separate "foundation" components (prompt, status, command dispatcher, focus utilities) from app-specific plugins (mux, chat variants, etc.).
- Provide guidance for downstream crates on how to extend Room with their own plugin bundles.

## Candidate Core Plugins
1. **Input Prompt** (already provided via `default_cli_bundle`) – handles text capture and shared submission state.
2. **Status Bar** (bundled) – surfaces basic focus + submission info; extensible via shared state.
3. **Slash Command Dispatcher** (planned) – watches shared input, parses `/command args`, and emits structured events for other plugins.
4. **Modal/Palette Focus Helper** (planned) – manages focus transitions for overlay/palette workflows using `FocusController`.
5. **Diagnostics Overlay** (existing diagnostics plugins, possibly a UI variant) – render runtime metrics or logging snippets directly in a zone.

## Scope Guidelines
- Core plugins should be protocol-agnostic and depend only on Room primitives (layout, shared state, focus helpers, logging).
- Avoid embedding project-specific behavior (e.g. mux command names, chat routing) in the core; instead expose hooks/events for downstream crates.
- Keep configuration simple (zones, priorities, optional callbacks). Complex state machines belong in app-specific crates.

## Packaging Ideas
- `room_mvp::bundles` – continue bundling foundational plugins for easy adoption (`default_cli_bundle`, future `default_command_bundle`).
- Downstream crates (e.g. `room_mux_plugins`) can re-export Room’s foundations and add mux-specific behaviors.
- Provide example bundles in `examples/` that demonstrate combining core plugins with custom ones.

## Default Focus Guidance
- The runtime now exposes `RuntimeConfig::default_focus_zone`; set this during bootstrap so prompt-centric plugins (like the default CLI bundle) receive focus automatically after `RoomRuntime::run` starts.
- When screens change, `ScreenManager::finish_activation` reapplies the configured zone, so legacy demos only need to configure the zone once (usually to point at the prompt).
- Focus controllers created by plugins should continue to use distinct owners (`FocusController::new`) so manual overrides (e.g. palettes) can temporarily claim focus without fighting the runtime owner.
- When a screen swaps to a different prompt, update `RuntimeConfig::default_focus_zone` before calling `activate` to steer the runtime owner to the new zone.

## Next Steps
1. Sketch `CommandDispatcherPlugin` API: target zone, shared input hook, callback/event emitter, optional command registry.
2. Design a lightweight focus palette helper that coordinates with `FocusRegistry` and exposes open/close operations.
3. Document extension points for custom plugins (shared state, focus, event hooks) so downstream projects follow consistent patterns.
4. Once Meteor/ASC100 protocol stabilizes, provide a reference socket bundle that pairs the dispatcher with remote transport.
