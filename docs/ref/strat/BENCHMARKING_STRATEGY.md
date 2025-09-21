# Room Benchmarking Strategy

This note explains how we benchmark the Room runtime. It is written for someone
who is new to benchmarking and to Rust, so it walks through the tools, commands,
and patterns we rely on.

## Why Benchmark?
- Benchmarks measure how long operations take so we can spot regressions before
  they ship.
- Tests answer "does it work?"; benchmarks answer "how fast is it?".
- For Room we care about the event loop latency, render costs, and plugin
  overhead because users expect responsive layouts.

## Tooling Overview
- **Criterion** is the benchmarking harness for Rust. It handles warm-up runs,
  statistical analysis, and report generation.
- **Benches folder**: all Criterion entry points live under `benches/`. Our
  first target is `benches/runtime.rs`.
- **Runtime helpers**: `RoomRuntime::run_scripted` lets us replay a predefined
  list of `RuntimeEvent`s without needing a live terminal.
- **Diagnostics plugins**: `LifecycleLoggerPlugin` and
  `MetricsSnapshotPlugin` are optional and can provide logs/metrics inside a
  benchmark without touching stdout.

## File Walkthrough
```
benches/runtime.rs       # Criterion entry point (runtime_chat_script)
src/runtime/mod.rs       # Exposes run_scripted + diagnostics plumbing
src/runtime/diagnostics.rs # Lifecycle logger + metrics snapshot plugins
```

Key pieces inside `runtime.rs`:
1. A `NullSink` logger so logging does not hit disk/console during benches.
2. `scripted_events()` builds a deterministic sequence of resize/keypress/tick
   events that simulate a short chat interaction.
3. `build_runtime()` assembles a `RoomRuntime` with:
   - The standard chat layout.
   - Diagnostics plugins wired to the `NullSink` logger.
   - The `BenchChatPlugin` that mirrors the example chat behaviour without
     touching the real example file.
4. The Criterion function `runtime_chat_script` calls
   `runtime.run_scripted(&mut io::sink(), script.clone())` inside the measurement
   loop. Criterion clones the script so each iteration sees the same events.

## Installing Prerequisites
- `cargo bench` pulls Criterion automatically (it is already listed in
  `Cargo.toml`).
- Optional: install `gnuplot` if you want Criterion to produce nicer charts.
  Without it Criterion falls back to the Plotters backend, which is fine for us.

## Running Benchmarks
Common commands:
- `cargo bench` — build and run all benchmarks with optimized code.
- `cargo bench --bench runtime` — run only the runtime benchmark.
- `cargo bench --bench runtime -- --warm-up-time 1` — override defaults (here we
  shorten the warm-up phase to 1 second). Everything after `--` is forwarded to
  Criterion.
- `cargo bench --bench runtime --no-run` — compile the benchmark without
  running it, useful when you just want to check for build errors quickly.

Reading output:
- Criterion reports the mean time (`time: [202.24 µs 204.42 µs 206.76 µs]`) and
  warns about outliers. Occasional mild outliers are normal; a growing number of
  severe outliers usually signal jitter or a real regression.
- Detailed reports live under `target/criterion/`. Each run creates a timestamped
  directory with JSON data and chart assets.

### Plots and Charts
- Criterion renders HTML reports at
  `target/criterion/<bench-name>/report/index.html`. Open that file in a web
  browser to see line charts, box plots, and distribution histograms.
- By default we rely on the Plotters backend, which produces static SVG images
  (e.g. `mean.svg`, `PDF.svg`) inside the same `report/` directory. These work
  out-of-the-box without extra tooling.
- If `gnuplot` is installed, Criterion will use it automatically and generate
  high-quality PNG plots instead. The CLI warning `Gnuplot not found, using
  plotters backend` simply means it fell back to SVG charts.
- You can force a backend via
  `cargo bench --bench runtime -- --plotting-backend plotters` or
  `--plotting-backend gnuplot`.
- To share results, zip the entire `report/` directory or export individual SVG/
  PNG files into project docs. Avoid committing the large `target/criterion`
  folder; copy just the relevant charts into `docs/ref/benchmarks/` if you need them
  under version control.

## Maintaining Baselines
Criterion has built-in support for named baselines so you can compare future
runs without manually diffing files.

Basic workflow:
1. Pick a name for the baseline (e.g. `main`).
2. Run the benchmark and save the baseline:
   `cargo bench --bench runtime -- --save-baseline main`
   This records the measurement in `target/criterion/runtime_chat_script/main/`.
3. Later, when you want to compare a new change, run:
   `cargo bench --bench runtime -- --baseline main`
   Criterion prints a comparison table (slower/faster) and writes a
   `comparison.html` report alongside the usual metrics.

Archiving baselines:
- If you want to keep historical results per commit, copy the generated
  baseline directory (e.g. `target/criterion/runtime_chat_script/main/`) into a
  separate folder such as `benchmarks/baselines/<git-hash>/`.
- Compressing the directory (`tar`, `zip`) keeps the JSON + charts together.
- To reuse an archived baseline later, copy it back into `target/criterion` with
  the same baseline name before running `cargo bench`.

Automation:
- Run `bin/snap_benchmarks.sh` after a benchmark session to mirror everything
  under `target/criterion/` into `meta/snaps/`. The script uses `rsync` so it
  updates deltas quickly and keeps baseline names intact.
- To restore from a snapshot, copy `meta/snaps/<bench>/<baseline>/` back into
  `target/criterion/<bench>/<baseline>/` before invoking `--baseline`.

## Adding New Benchmarks
Follow this pattern to add another scenario:
1. Create `benches/<name>.rs` and add a matching `[[bench]]` entry to
   `Cargo.toml` (set `harness = false` so Criterion controls the main function).
2. Describe the workload with a reusable helper (e.g., a new plugin or layout).
3. Generate scripted events or a synthetic token feed. Keep the sequence short so
   each iteration is fast (<1 ms if possible).
4. Reuse the diagnostics plugins to keep logging/metrics contained. Swap in a
   `NullSink` logger (as in `benches/runtime.rs`) if the plugin would otherwise
   spam stdout.
5. Run `cargo bench --bench <name> --no-run` to confirm it compiles, then run the
   benchmark and archive the first baseline results.

### Benchmarking Individual Plugins
- Create a dedicated bench function (or a new `benches/<plugin>_bench.rs`) that
  registers only the plugin(s) you want to measure plus any mandatory scaffolding.
- Tailor the scripted event list to hammer the plugin’s hot paths (e.g. rapid
  keypresses for an input plugin, resize storms for a layout helper).
- Use `register_plugin_with_priority` so the plugin sits at the same priority it
  would in production; this keeps interactions realistic.
- Wrap the plugin-specific state in the shared resource map if it needs shared
  dependencies; ad-hoc globals would skew results.
- Capture a baseline (`cargo bench --bench runtime -- --save-baseline plugin_focus`) and
  rerun against it after code changes (`--baseline plugin_focus`) to see the delta.
  See `runtime_focus_script` in `benches/runtime.rs` for a concrete example that
  stresses the shared `FocusRegistry` resource.

## Patterns and Best Practices
- **Scripted event loops**: use `RoomRuntime::run_scripted` to test runtime
  changes without depending on real input. Scripts stay deterministic, making it
  easier to compare results.
- **Isolated plugins**: keep benchmark-specific plugins in the bench file (like
  `BenchChatPlugin`) instead of editing `examples/` so workshops/demos stay
  untouched.
- **Logging discipline**: send logs to a `NullSink` (or lower the level) to avoid
  benchmarking the logger itself.
- **Metrics snapshots**: use a short interval (e.g. 250 ms) when you want
  periodic stats. Set the interval to `Duration::from_millis(0)` to disable
  automatic snapshots entirely.
- **Warm-up and sample counts**: Criterion chooses defaults that work well. Only
  tweak them if you have a specific goal (e.g. extremely short workloads might
  need more samples).

## Troubleshooting
- **"gnuplot not found"**: install gnuplot or ignore the message—Plotters
  automatically takes over.
- **Manifest errors about benches**: ensure `benches/<name>.rs` exists and that
  `Cargo.toml` has a matching `[[bench]]` entry.
- **Benchmark takes forever**: compare `runtime_chat_script` timing against past
  runs. A large regression likely means a change in render logic or an accidental
  `sleep`. Use logging snapshots to pinpoint where time is spent.

## Next Steps
- Extend the scripted event library so future benches can reuse common chat,
  dashboard, and modal scenarios.
- Add a "headless socket" benchmark once the socket adapter lands, following the
  same scripted-event pattern.
