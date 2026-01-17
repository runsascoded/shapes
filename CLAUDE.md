# shapes - Differentiable Shape Intersections

Rust library (compiled to WASM) for computing shape intersections with automatic differentiation. Used by [apvd] for area-proportional Venn diagram generation.

[apvd]: https://github.com/runsascoded/apvd

## Build

```bash
wasm-pack build --target web   # WASM → pkg/
cargo build                     # Native
cargo test                      # Tests
```

Output goes to `pkg/` with TypeScript bindings.

## Architecture

### Shape Types (`src/shape.rs`)
- **Circle**: center + radius
- **XYRR**: axis-aligned ellipse (center + x/y radii)
- **XYRRT**: rotated ellipse (center + x/y radii + theta)

### Core Modules

**Math & Autodiff**:
- `dual.rs`: Forward-mode AD wrapper around `num_dual::DualDVec64`
- `math/quartic.rs`, `cubic.rs`, `quadratic.rs`: Polynomial solvers

**Geometry**:
- `ellipses/quartic.rs`: Quartic solver for ellipse-ellipse intersections
- `ellipses/xyrr.rs`, `xyrrt.rs`: Ellipse-specific computations
- `circle.rs`: Circle operations
- `r2.rs`: 2D point type
- `intersect.rs`, `intersection.rs`: Shape intersection detection

**Regions & Optimization**:
- `region.rs`, `regions.rs`: Region identification and area computation
- `targets.rs`: Target region sizes with inclusive/exclusive expansion
- `model.rs`: Training loop (gradient descent)
- `step.rs`: Single optimization step
- `history.rs`: Step history tracking

**WASM Exports** (`lib.rs`):
- `make_model(inputs, targets)`: Create optimization model
- `train(model, max_step_error_ratio, max_steps)`: Run training
- `make_step(inputs, targets)`: Single step
- `expand_targets(targets)`: Expand inclusive/exclusive targets
- `init_logs()`, `update_log_level(level)`: Logging

## Key Dependencies

- **num-dual**: Forward-mode automatic differentiation
- **nalgebra**: Linear algebra (vectors, matrices)
- **wasm-bindgen**: JS/WASM FFI
- **serde-wasm-bindgen**: Serialization to/from JS
- **tsify**: TypeScript type generation
- **approx**: Floating-point comparison with tolerance

## Data Flow

1. JS passes shape specs + target sizes to `make_model`
2. Rust computes current region areas via:
   - Find shape intersection points (quartic solver for ellipses)
   - Build boundary graph (edges, segments)
   - Compute signed area of each region
3. Autodiff computes error gradient w.r.t. shape parameters
4. Gradient descent updates shapes to minimize area error
5. Model returned to JS with step history

## Tests

```bash
cargo test                    # All tests
cargo test fizz_buzz_bazz     # Specific test
RUST_LOG=debug cargo test     # With logging
```

Test data in `testdata/` (CSV files with expected values).

## Known Issues

- Issue #9: Only absolute error metric (no relative)
- Issue #10: Basic missing-region penalty
- Quartic solver can have numerical stability issues at edge cases
