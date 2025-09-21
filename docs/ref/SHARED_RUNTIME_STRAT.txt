# Shared Runtime State Strategy

This note describes the `SharedState` utility introduced under
`src/runtime/shared_state.rs`. It is a reusable, runtime-agnostic resource map
that lets plugins and adapters exchange state without hard-coding globals or
singletons.

## Concept
- `SharedState` is a type-erased map keyed by `TypeId`. Each stored value is an
  `Arc<T>` so readers can clone handles without copying the underlying data.
- Any project (not just Room) can instantiate a `SharedState` and pass clones to
  cooperating components.
- The map is concurrency-safe (`RwLock` inside) so multiple threads can read
  concurrently. Writers acquire the lock when inserting new resources.

## Core API
- `SharedState::new()` — create an empty map.
- `SharedState::insert_arc(Arc<T>)` — install a pre-built resource. Fails if the
  type is already registered.
- `SharedState::get<T>() -> Result<Arc<T>, SharedStateError>` — fetch the shared
  resource for type `T`.
- `SharedState::get_or_insert_with<T, F>(F) -> Result<Arc<T>, SharedStateError>` —
  lazily initialise a resource if it does not exist. The initializer runs once.
- Errors:
  - `AlreadyExists` — attempted to insert a duplicate type.
  - `Missing` — requesting a resource that has not been registered.
  - `TypeMismatch` — the stored value was not an `Arc<T>` (should not happen if
    the helper functions are used).
  - `Poisoned` — the internal lock was poisoned by a panic.

## Room Integration
- `RoomRuntime` holds a `SharedState` internally and exposes it via
  `RoomRuntime::shared_state_handle()`.
- Plugins receive access through `RuntimeContext::shared` and
  `RuntimeContext::shared_init`, so they can collaborate on shared resources.
  ```rust
  #[derive(Default)]
  struct FocusRegistry { /* ... */ }

  impl RoomPlugin for FocusPlugin {
      fn init(&mut self, ctx: &mut RuntimeContext<'_>) -> Result<()> {
          let registry = ctx
              .shared_init::<FocusRegistry, _>(FocusRegistry::default)
              .expect("shared state available");
          // use registry (Arc<FocusRegistry>)
          Ok(())
      }
  }
  ```
- Adapters can pre-register resources before the runtime starts:
  ```rust
  let runtime = RoomRuntime::new(layout, renderer, size)?;
  runtime
      .shared_state_handle()
      .insert_arc(Arc::new(FocusRegistry::default()))?;
  ```

## Usage Patterns
- **Focus management**: store a `FocusRegistry` so keyboard handlers and status
  bars agree on the active zone.
- **Default bundle state**: the CLI bundle registers a `RwLock<InputSharedState>`
  so the input prompt and status bar can share submission history without
  bespoke wiring.
- **Command bus**: keep an `Arc<Mutex<Vec<Command>>>` that multiple plugins can
  append to; a dispatcher plugin can drain it each frame.
- **Config knobs**: expose shared settings (theme, refresh rates) that adapters
  or plugins can update at runtime.
- `FocusRegistry` support ships in `room_mvp::focus`; use
  `ensure_focus_registry(ctx)` inside plugins and `FocusController` helpers to
  coordinate focus ownership.

## Best Practices
- Prefer `shared_init` from plugins so initialization happens exactly once and
  there is no race to insert a resource.
- Wrap mutation inside your own synchronization primitives (`Mutex`, `RwLock`) if
  a resource will be mutated frequently.
- Use dedicated structs/enums per shared resource instead of generic maps of
  strings; `TypeId` ensures each type is unique, and strongly typed data keeps
  the interface maintainable.
- Consider creating helper modules that wrap `SharedState` for common patterns
  (e.g. `runtime::focus::FocusRegistrar`) to reduce boilerplate.

## Extending Beyond Room
- Because `SharedState` is independent of `RoomRuntime`, other crates can pull it
  in (via the re-export in `room_mvp::SharedState`).
- Combine it with other event loops or orchestrators to provide consistent
  resource management across projects.

## Future Ideas
- Add optional diagnostics that list registered types for debugging.
- Provide a typed key wrapper to allow storing multiple instances per type when
  needed (e.g. keyed by string + type).
- Investigate an async-aware variant using `tokio::sync::RwLock` for async
  runtimes.
