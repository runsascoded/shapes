# Timing Metrics Specification (shapes crate)

## Overview

Instrument the Rust training code to report timing metrics. These metrics are used by:
- **WASM (static branch):** Reported via Worker message
- **WebSocket server (server branch):** Reported via RPC response
- **CLI (`apvd`):** Printed to console or JSON output

## Metrics to Capture

### Per-Step Timing

```rust
pub struct StepTiming {
    /// Gradient computation time (autodiff forward pass)
    pub gradient_ns: u64,

    /// Shape update time (apply gradients)
    pub update_ns: u64,

    /// Region computation time (intersections, areas)
    pub regions_ns: u64,

    /// Error computation time
    pub error_ns: u64,

    /// Total step time
    pub total_ns: u64,
}
```

### Batch Timing

```rust
pub struct BatchTiming {
    /// Number of steps in this batch
    pub steps: usize,

    /// Total computation time for batch
    pub compute_ns: u64,

    /// Per-step breakdown (optional, for detailed analysis)
    pub step_timings: Option<Vec<StepTiming>>,

    /// Derived: steps per second
    pub steps_per_second: f64,
}

impl BatchTiming {
    pub fn steps_per_second(&self) -> f64 {
        if self.compute_ns == 0 {
            return 0.0;
        }
        (self.steps as f64) / (self.compute_ns as f64 / 1_000_000_000.0)
    }
}
```

### Training Summary

```rust
pub struct TrainingSummary {
    /// Total steps trained
    pub total_steps: usize,

    /// Total training time
    pub total_ns: u64,

    /// Overall steps/second
    pub steps_per_second: f64,

    /// Breakdown by phase
    pub phase_breakdown: PhaseBreakdown,
}

pub struct PhaseBreakdown {
    /// Gradient computation (% of total)
    pub gradient_pct: f64,

    /// Region computation (% of total)
    pub regions_pct: f64,

    /// Error computation (% of total)
    pub error_pct: f64,

    /// Other overhead (% of total)
    pub overhead_pct: f64,
}
```

## Implementation

### Instrumentation Points

In `apvd-core/src/optimization/step.rs`:

```rust
use std::time::Instant;

impl Step<D> {
    pub fn make_step_timed(
        prev: &Step<D>,
        targets: &Targets<f64>,
        learning_rate: f64,
    ) -> (Step<D>, StepTiming) {
        let total_start = Instant::now();

        // Gradient computation
        let gradient_start = Instant::now();
        let gradients = compute_gradients(prev);
        let gradient_ns = gradient_start.elapsed().as_nanos() as u64;

        // Shape update
        let update_start = Instant::now();
        let new_shapes = apply_gradients(&prev.shapes, &gradients, learning_rate);
        let update_ns = update_start.elapsed().as_nanos() as u64;

        // Region computation
        let regions_start = Instant::now();
        let components = compute_regions(&new_shapes);
        let regions_ns = regions_start.elapsed().as_nanos() as u64;

        // Error computation
        let error_start = Instant::now();
        let (errors, total_error) = compute_errors(&components, targets);
        let error_ns = error_start.elapsed().as_nanos() as u64;

        let total_ns = total_start.elapsed().as_nanos() as u64;

        let timing = StepTiming {
            gradient_ns,
            update_ns,
            regions_ns,
            error_ns,
            total_ns,
        };

        let step = Step { shapes: new_shapes, components, errors, error: total_error, .. };
        (step, timing)
    }
}
```

### Batch Training with Timing

```rust
pub fn train_batch_timed(
    model: &mut Model,
    num_steps: usize,
    learning_rate: f64,
) -> BatchTiming {
    let start = Instant::now();
    let mut step_timings = Vec::with_capacity(num_steps);

    for _ in 0..num_steps {
        let (new_step, timing) = Step::make_step_timed(
            model.current_step(),
            &model.targets,
            learning_rate,
        );
        step_timings.push(timing);
        model.record_step(new_step);
    }

    let compute_ns = start.elapsed().as_nanos() as u64;

    BatchTiming {
        steps: num_steps,
        compute_ns,
        step_timings: Some(step_timings),
        steps_per_second: (num_steps as f64) / (compute_ns as f64 / 1_000_000_000.0),
    }
}
```

### WASM Export

In `apvd-wasm/src/lib.rs`:

```rust
#[wasm_bindgen]
pub struct TimingInfo {
    pub compute_ms: f64,
    pub steps: u32,
    pub steps_per_second: f64,
}

#[wasm_bindgen]
pub fn train_with_timing(/* ... */) -> TrainResultWithTiming {
    let timing = train_batch_timed(&mut model, num_steps, learning_rate);

    TrainResultWithTiming {
        // ... step data
        timing: TimingInfo {
            compute_ms: timing.compute_ns as f64 / 1_000_000.0,
            steps: timing.steps as u32,
            steps_per_second: timing.steps_per_second,
        },
    }
}
```

### Server RPC Response

In `apvd-server`:

```rust
#[derive(Serialize)]
pub struct TrainResponse {
    // ... existing fields
    pub timing: Option<BatchTiming>,
}
```

### CLI Output

```bash
$ apvd train config.json --steps 10000 --progress

Training...
  [████████████████████████████████] 10000/10000

Summary:
  Total time: 8.2s
  Rate: 1,220 steps/sec

  Breakdown:
    Gradient:  45.2%
    Regions:   32.1%
    Error:     18.4%
    Overhead:   4.3%
```

## Configuration

### Timing Granularity

```rust
pub enum TimingGranularity {
    /// No timing (fastest)
    None,

    /// Batch-level only (low overhead)
    Batch,

    /// Per-step timing (some overhead)
    Step,

    /// Per-phase within step (highest overhead, for profiling)
    Detailed,
}
```

Default to `Batch` for normal use, `Detailed` for profiling/debugging.

### Feature Flag

```toml
# Cargo.toml
[features]
default = ["timing"]
timing = []
```

When `timing` feature is disabled, timing calls become no-ops:

```rust
#[cfg(feature = "timing")]
fn record_timing(start: Instant) -> u64 {
    start.elapsed().as_nanos() as u64
}

#[cfg(not(feature = "timing"))]
fn record_timing(_start: Instant) -> u64 {
    0
}
```

## WASM Considerations

### `Instant` in WASM

`std::time::Instant` may not work in WASM. Use `web_sys::Performance` instead:

```rust
#[cfg(target_arch = "wasm32")]
fn now() -> f64 {
    web_sys::window()
        .expect("no window")
        .performance()
        .expect("no performance")
        .now()
}

#[cfg(not(target_arch = "wasm32"))]
fn now() -> f64 {
    // Use std::time::Instant converted to ms
}
```

Or use the `instant` crate which abstracts this:

```toml
[dependencies]
instant = { version = "0.1", features = ["wasm-bindgen"] }
```

## Recomputation Timing

For P-frame reconstruction:

```rust
pub struct RecomputeTiming {
    /// Source keyframe step index
    pub keyframe_step: usize,

    /// Target step index
    pub target_step: usize,

    /// Number of steps recomputed
    pub steps_recomputed: usize,

    /// Time to recompute
    pub compute_ns: u64,

    /// Time per recomputed step
    pub per_step_ns: u64,
}
```

This helps the UI show "Step 5432: recomputed from 5120 (312 steps, 8.2ms)".
