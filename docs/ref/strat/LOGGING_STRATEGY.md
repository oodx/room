# Logging & Benchmarking Strategy

## Goals

- Provide a reusable logging facade for Room runtimes and plugins without hard
  coupling to any particular sink or format.
- Support pluggable metrics snapshots (render FPS, dirty zone counts, plugin
  timings) that can be published through the same facade.
- Stand up a Criterion benchmark harness that consumes recorded event traces and
  exercises layout solve + render loops, while relying on the logging facade for
  telemetry.

## Modules

1. `logging` module (`src/logging/mod.rs`)
   - Expose a `LogLevel` enum (`Trace`, `Debug`, `Info`, `Warn`, `Error`).
   - Define a `LogEvent` struct with timestamp, target (e.g., plugin or runtime),
     message, optional fields map.
   - Introduce a `LogSink` trait so callers can plug in different backends (file,
     stdout, in-memory for tests).
   - Provide a default `FileSink` implementation that writes newline-delimited
     JSON to a rotating log file (simple size-based rollover is enough for now).
   - Ship a lightweight `Logger` handle that clones an `Arc<dyn LogSink>` and
     formats events before dispatching.

2. `metrics` module (`src/metrics/mod.rs`)
   - Offer a `MetricSnapshot` struct capturing counters/gauges we care about
     (render count, dirty zones, event rates, plugin timings).
   - Provide helper methods for updating counters and converting snapshots into
     `LogEvent`s so they can be emitted through the logger.

3. Runtime integration (`src/runtime/mod.rs`)
   - Extend `RuntimeConfig` with optional logger and metrics toggles.
   - Emit `LogEvent`s at key lifecycle points (startup, event dispatch,
     render start/end, resize, exit) using the logger if configured.
   - Expose periodic metrics snapshots (e.g., once per tick) via the same logger.

4. Benchmarks (`benches/runtime.rs`)
   - Set up Criterion harness that replays a synthetic chat session using the new
     runtime API.
   - Collect statistics for layout solve + render iterations and emit benchmark
     metadata using the logger (to demonstrate reuse).

## Implementation Steps

1. Scaffold `logging` module with public facade and default JSONL file sink.
2. Add metrics helper module for runtime counters and conversion to log events.
3. Update `RoomRuntime` to accept an optional `Logger` in `RuntimeConfig`, emit
   lifecycle logs, and publish metrics snapshots.
4. Create `benches/runtime.rs` with Criterion-driven replay of an event script
   (using `ChatPlugin` data), recording solve/render times and verifying the
   logger integration.
5. Document usage in README or follow-up docs if needed.

## Follow-ups

- Add CLI flag or environment variable for enabling/disabling logging sinks.
- Expand metrics (histograms, plugin-specific counters) once we have more real
  workloads.
- Consider integrating with `tracing` for richer span-based telemetry when the
  API stabilises.
