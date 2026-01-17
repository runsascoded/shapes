# Shapes Codebase Cleanup Spec

Comprehensive cleanup plan based on code audit. Organized by priority and effort.

## High Priority (Correctness & Stability)

### 1. Error Handling Overhaul ✅ DONE

**Status**: `Scene::new`, `Step::new/nxt/step`, and `Model::new/train` now return `Result` types.

**Completed**:
- `Scene::new` returns `Result<Scene<D>, SceneError>`
- `compute_component_depths` and `compute_component_depth` return `Result`
- `Step::new`, `Step::nxt`, `Step::step` propagate `Result<Step, SceneError>`
- `Model::new` returns `Result<Model, SceneError>`, `train` returns `Result<(), SceneError>`
- WASM exports use `.expect()` to convert errors to JS exceptions
- `SceneError` has `Clone` derive for Result cloning

**Remaining** (lower priority):
- `step.rs:56`: `String::from_utf8().unwrap()` → Handle invalid UTF-8
- `shape.rs:88`: `panic!("Unrecognized coord keys")` → Return `Result`

### 2. Numerical Stability (xyrr.rs TODOs)

**Current state**: Lines 349, 364 note inaccurate roots from tiny x⁴/x³ coefficients.

**Goal**: Improve quartic root accuracy for edge cases.

**Options**:
1. Newton-Raphson refinement after initial root finding
2. Switch to companion matrix eigenvalue approach (more stable)
3. Interval bisection for root polishing

**Test cases needed**:
- Near-tangent ellipses
- Highly eccentric ellipses
- Large/small scale differences

### 3. Delete Dead Code ✅ DONE

- **`float_vec.rs`**: Already deleted (file doesn't exist)

## Medium Priority (Performance)

### 4. Reduce Clone Overhead

**Current state**: 600+ `.clone()` calls, many in hot paths.

**High-impact locations**:
- `scene.rs:77-78`: Clone on every shape pair intersection
- `circle.rs:113-115`: `cx.clone() * cx.clone()` → Use references
- `r2.rs:65-66`: `norm2()` clones coordinates unnecessarily

**Pattern to apply**:
```rust
// Before
let result = self.x.clone() * self.x.clone() + self.y.clone() * self.y.clone();

// After (if D: Copy)
let result = self.x * self.x + self.y * self.y;

// Or (if D: Clone but expensive)
let x = &self.x;
let result = x.clone() * x.clone(); // One clone instead of two
```

**Estimate**: 20-30% perf improvement in hot paths.

### 5. Simplify Trait Bounds ✅ PARTIAL

**Status**: Added initial trait aliases following existing `*Arg` pattern.

**Completed**:
- `NormArg` in `r2.rs` for `Clone + Add + Mul` (used by `norm2`, `r`)
- `DistanceArg` in `distance.rs` for `Clone + Add + Mul + Sqrt`
- Codebase already uses module-local `*Arg` traits extensively (e.g., `AreaArg`, `quartic::Arg`, `cubic::Arg`)

**Note**: The existing `*Arg` pattern is preferable to a centralized `DualNum` trait, as it keeps trait definitions close to their usage.

## Lower Priority (Maintainability)

### 6. Module Reorganization

**Current structure** (flat, 47 pub mods in lib.rs):
```
src/
  lib.rs
  shape.rs, scene.rs, circle.rs, ...
  ellipses/
  math/
```

**Proposed structure**:
```
src/
  lib.rs (re-exports only)
  geometry/
    mod.rs
    shape.rs
    circle.rs
    ellipses/ (xyrr, xyrrt, cdef, bcdef)
    r2.rs
  math/
    mod.rs
    polynomial/ (quartic, cubic, quadratic)
    complex.rs
    trig.rs
  analysis/
    mod.rs
    scene.rs
    component.rs
    region.rs
    regions.rs
  optimization/
    mod.rs
    model.rs
    step.rs
    history.rs
  wasm/
    mod.rs
    bindings.rs (current lib.rs WASM exports)
```

### 7. Split Large Files

**scene.rs (816 lines)**:
- Extract test helpers → `scene/tests.rs`
- Extract intersection detection → `scene/intersect.rs`
- Main Scene struct stays in `scene/mod.rs`

**component.rs (464 lines)**:
- Extract `traverse()` function → `component/traverse.rs`
- Extract edge construction → `component/edges.rs`

### 8. Documentation

**Add rustdoc to**:
- All `pub fn` in lib.rs (WASM API)
- `Scene`, `Component`, `Region` structs
- `Shape` enum and variants
- Complex algorithms (`traverse`, `compute_component_depth`)

**Template**:
```rust
/// Computes the area of all regions formed by shape intersections.
///
/// # Arguments
/// * `shapes` - Vector of shapes to analyze
/// * `targets` - Target areas for optimization
///
/// # Returns
/// A `Model` containing optimization history and final state.
///
/// # Panics
/// If shapes contain invalid coordinates (NaN, Inf).
pub fn make_model(...) -> Model { ... }
```

### 9. Test Coverage ✅ PARTIAL

**Added tests**:
- `math/quadratic.rs` - 8 comprehensive tests (real roots, complex roots, edge cases)
- `distance.rs` - 6 tests for R2 distance calculations
- `math/cubic.rs`, `math/quartic.rs` - Already had good coverage

**Still missing**:
- `targets.rs` - Target expansion/validation
- `hull.rs` - Convex hull (depends on complex Segment setup)
- Error paths (invalid inputs)
- Property-based tests

**Future**: Add property-based tests (proptest crate):
```rust
proptest! {
    #[test]
    fn circle_area_positive(r in 0.1f64..1000.0) {
        let c = Circle::new(0.0, 0.0, r);
        assert!(c.area() > 0.0);
        assert!((c.area() - PI * r * r).abs() < 1e-10);
    }
}
```

### 10. Address TODOs

| Location | TODO | Action |
|----------|------|--------|
| `scene.rs:327` | SumOpt trait | Implement or remove comment |
| `math/cubic.rs:158,188` | Factor constants | Make static or const |
| `component.rs:86` | Shadow variable | Rename for clarity |
| `model.rs:267` | Nondeterminism | Investigate and fix |
| `targets.rs:140` | "TODO: fsck" | Clarify or remove |

## Dependency Updates

### Check versions
```bash
cargo outdated
```

### Specific concerns:
- `roots = "0.0.8"` - Very old, check for updates
- `polars = "*"` - Pin to specific version, or replace with lighter CSV lib if only used for test data

## Implementation Order

**Phase 1 (Week 1)**: Critical correctness
1. Delete `float_vec.rs`
2. Add error types, convert 5 most critical panic sites
3. Add docs to WASM API (lib.rs exports)

**Phase 2 (Week 2-3)**: Performance
4. Audit clone usage, fix top 10 hot spots
5. Create trait aliases
6. Add missing unit tests for math modules

**Phase 3 (Month 2)**: Organization
7. Split scene.rs
8. Begin module reorganization (one domain at a time)
9. Property-based tests

**Phase 4 (Ongoing)**: Polish
10. Address remaining TODOs
11. Complete rustdoc coverage
12. Dependency audit

## Performance Benchmarks (TODO)

Add benchmarks to catch regressions:

**Rust benchmarks** (criterion):
- Scene construction with various shape counts (2-10 shapes)
- Gradient computation (single step)
- Full optimization runs (measure convergence speed)

**WASM/Browser benchmarks** (in `../apvd` app):
- End-to-end optimization timing
- Measure as part of browser e2e tests
- Compare against baseline timings

Priority: Add before major refactors to establish baselines.

## Notes

- Keep WASM API stable during refactors
- Run `wasm-pack build` after each phase to verify JS compatibility
- Profile before/after clone optimization (`cargo flamegraph`)
