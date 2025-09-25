# SimulatedLoop - Headless TUI Runtime

SimulatedLoop enables Terminal User Interface (TUI) applications to run in headless environments without requiring a TTY (terminal device). This is essential for automated testing, CI/CD pipelines, and development scenarios where interactive terminal access is not available.

## Problem: TTY Dependency

Traditional TUI applications require a TTY to:
- Read keyboard input via `event::poll()`
- Control cursor positioning and colors
- Handle terminal resize events
- Manage raw mode terminal state

**What happens without TTY:**
```bash
cargo run --example my_tui_app
# Error: Terminal("No such device or address (os error 6)")
```

**Environments that lack TTY:**
- CI/CD pipelines (GitHub Actions, GitLab CI)
- Docker containers without `docker run -it`
- SSH connections without `-t` pseudo-terminal flag
- Background services and daemons
- Some remote development environments
- Automated testing frameworks

## Solution: SimulatedLoop

SimulatedLoop replaces the interactive event loop with a bounded, deterministic execution model:

```rust
use room_mvp::{RoomRuntime, RuntimeConfig, SimulatedLoop};

// Configure simulated execution
let mut config = RuntimeConfig::default();
config.simulated_loop = Some(SimulatedLoop::ticks(10));

// Runs headlessly without TTY
let mut runtime = RoomRuntime::with_config(layout, renderer, size, config)?;
let mut buffer = Vec::new();
runtime.run(&mut buffer)?;

// Captured output as ANSI text
println!("{}", String::from_utf8_lossy(&buffer));
```

## SimulatedLoop Configuration

### Constructor Methods

**`SimulatedLoop::ticks(iterations)`**
- Runs for specified number of iterations
- Emits synthetic tick events each iteration
- Simulates time progression for time-based UI updates

**`SimulatedLoop::silent(iterations)`**
- Runs for specified number of iterations
- No synthetic tick events
- Pure rendering without time simulation

### Example Configurations

```rust
// Test basic rendering (no time events)
config.simulated_loop = Some(SimulatedLoop::silent(3));

// Test time-based behavior (progress bars, animations)
config.simulated_loop = Some(SimulatedLoop::ticks(10));

// Quick smoke test
config.simulated_loop = Some(SimulatedLoop::silent(1));
```

## Execution Flow

1. **Bootstrap Phase** - Normal plugin initialization
2. **LoopSimulated Audit** - Marks start of bounded execution
3. **Bounded Iterations** - Executes N times instead of waiting for input
4. **Optional Tick Events** - Synthetic time progression (if `dispatch_ticks: true`)
5. **Render Each Frame** - Captures output to provided buffer
6. **Completion Audit** - `LoopSimulatedComplete` or `LoopSimulatedAborted`
7. **Finalize Phase** - Normal cleanup and teardown

## Use Cases

### 🧪 **Automated Testing**
```rust
#[test]
fn test_dashboard_renders_correctly() {
    let mut config = RuntimeConfig::default();
    config.simulated_loop = Some(SimulatedLoop::silent(1));

    let mut runtime = create_dashboard_runtime(config)?;
    let mut buffer = Vec::new();
    runtime.run(&mut buffer)?;

    let output = String::from_utf8_lossy(&buffer);
    assert!(output.contains("Dashboard"));
    assert!(output.contains("Status: BOOTING"));
}
```

### 🔄 **CI/CD Integration**
```yaml
# .github/workflows/test.yml
- name: Test TUI Examples
  run: |
    cargo run --example dashboard_demo
    cargo run --example lifecycle_trace
    # These now work headlessly with SimulatedLoop!
```

### 📸 **Documentation Screenshots**
```rust
// Generate terminal output for documentation
let mut config = RuntimeConfig::default();
config.simulated_loop = Some(SimulatedLoop::ticks(5));

let mut runtime = create_example_app(config)?;
let mut buffer = Vec::new();
runtime.run(&mut buffer)?;

// Save as .txt file for docs/examples/
std::fs::write("docs/examples/dashboard_output.txt", buffer)?;
```

### 🐛 **Development Debugging**
```rust
// Debug without terminal interaction
config.simulated_loop = Some(SimulatedLoop::ticks(3));

// See exactly what gets rendered at each step
runtime.run(&mut debug_buffer)?;
eprintln!("Debug output:\n{}", String::from_utf8_lossy(&debug_buffer));
```

## Safety Features

### Loop Guard Protection
Both simulated and interactive modes support safety limits:

```rust
config.loop_iteration_limit = Some(1000); // Prevents infinite loops
config.simulated_loop = Some(SimulatedLoop::ticks(50));
```

**Audit Trail:**
- `LoopGuardTriggered` - When iteration limit reached
- `LoopAborted` - Forced termination via safety guard
- `LoopSimulatedAborted` - Early exit due to fatal error
- `LoopSimulatedComplete` - Normal completion

## Migration Guide

### Before: CLI Driver Only
```rust
// Only works with interactive terminal
use room_mvp::CliDriver;

let runtime = RoomRuntime::new(layout, renderer, size)?;
CliDriver::new(runtime).run()?; // Fails in headless environments
```

### After: Dual Mode Support
```rust
// Works both interactively AND headlessly
let mut config = RuntimeConfig::default();

// Conditional configuration based on environment
if std::env::var("CI").is_ok() || !atty::is(atty::Stream::Stdout) {
    // Headless mode for CI/automation
    config.simulated_loop = Some(SimulatedLoop::ticks(5));
    let mut buffer = Vec::new();
    runtime.run(&mut buffer)?;
    println!("{}", String::from_utf8_lossy(&buffer));
} else {
    // Interactive mode for development
    CliDriver::new(runtime).run()?;
}
```

## Best Practices

### ✅ **DO**
- Use `SimulatedLoop::ticks()` for time-dependent UIs
- Use `SimulatedLoop::silent()` for static rendering tests
- Set reasonable iteration counts (1-20 for most tests)
- Add `loop_iteration_limit` as safety guard
- Test both simulated and interactive modes

### ❌ **DON'T**
- Set extremely high iteration counts (wastes resources)
- Rely on user input in simulated mode (won't work)
- Skip testing interactive mode entirely
- Forget to capture buffer output in headless scenarios

## Examples in Codebase

- **`examples/workshop_lifecycle_trace_01.rs`** - Demonstrates headless lifecycle tracing
- **`examples/bootstrap_helper.rs`** - Shows basic SimulatedLoop usage
- **`examples/workshop_lifecycle_trace_interactive.rs`** - Interactive version for comparison

## Troubleshooting

**"No simulated_loop configuration found"**
- Ensure `RuntimeConfig::simulated_loop` is set to `Some(SimulatedLoop::...)`

**"Output buffer is empty"**
- Check that plugins set zone content during `init()` or event handling
- Verify `max_iterations > 0`

**"Simulated loop never completes"**
- Check for infinite loops in plugin code
- Add `loop_iteration_limit` as safety guard
- Review audit logs for `LoopGuardTriggered` events

---

*SimulatedLoop enables robust testing and development workflows by making TUI applications truly portable across all execution environments.*