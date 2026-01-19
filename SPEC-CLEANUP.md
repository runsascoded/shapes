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

**Remaining** (lower priority): ✅ DONE
- `step.rs:56`: ✅ Replaced with `str::repeat()`
- `shape.rs:88`: ✅ Already returns `Result<_, ShapeError>`

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

### 4. Reduce Clone Overhead ✅ PARTIAL

**Status**: Removed unnecessary clones where straightforward; deeper optimization requires trait bound changes.

**Completed**:
- `circle.rs`: Removed 4 unnecessary clones in `unit_intersections()` return path
- Replaced `String::from_utf8(vec![...]).unwrap()` with `str::repeat()` (not clone-related but cleaner)

**Remaining** (requires `Mul<&D>` trait bounds):
- `r2.rs:65-66`: `norm2()` still clones for multiplication
- `circle.rs:113-115`: Intermediate squaring operations

**Note**: Further optimization requires adding `for<'a> &'a D: Mul<D, Output = D>` bounds, which is a larger refactor. Consider adding benchmarks first to measure impact.

### 5. Simplify Trait Bounds ✅ PARTIAL

**Status**: Added initial trait aliases following existing `*Arg` pattern.

**Completed**:
- `NormArg` in `r2.rs` for `Clone + Add + Mul` (used by `norm2`, `r`)
- `DistanceArg` in `distance.rs` for `Clone + Add + Mul + Sqrt`
- Codebase already uses module-local `*Arg` traits extensively (e.g., `AreaArg`, `quartic::Arg`, `cubic::Arg`)

**Note**: The existing `*Arg` pattern is preferable to a centralized `DualNum` trait, as it keeps trait definitions close to their usage.

## Lower Priority (Maintainability)

### 6. Module Reorganization ✅ DONE

**New structure**:
```
src/
  lib.rs (re-exports for backwards compatibility)
  analysis/           # Scene analysis
    scene.rs, component.rs, region.rs, regions.rs
    edge.rs, node.rs, segment.rs, hull.rs
    intersect.rs, intersection.rs, contains.rs, gap.rs, distance.rs
    set.rs, theta_points.rs
  geometry/           # Shape types
    shape.rs, circle.rs, r2.rs
    ellipses/ (xyrr, xyrrt, cdef, bcdef, quartic)
    transform.rs, rotate.rs
  math/               # Numerical algorithms
    polynomial/ (quartic, cubic, quadratic)
    complex.rs, roots.rs, abs.rs, cbrt.rs, deg.rs, is_normal.rs, is_zero.rs,
    recip.rs, round.rs, d5.rs, float_arr.rs, float_wrap.rs, sqrt.rs, trig.rs, zero.rs
  optimization/       # Gradient descent
    model.rs, step.rs, history.rs, targets.rs
```

All modules re-exported at crate root - existing imports unchanged.

### 7. Documentation

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

### 8. Test Coverage ✅ EXPANDED

**Added tests** (total: 142 tests):
- `math/quadratic.rs` - 8 comprehensive tests (real roots, complex roots, edge cases)
- `distance.rs` - 6 tests for R2 distance calculations
- `targets.rs` - 6 additional tests (disjoints, neighbors, idx, none_key, total_area)
- `r2.rs` - 11 tests (norm2, norm, r, arithmetic, neg, atan2)
- `circle.rs` - 12 new tests (area, at_y, xyrr, unit_circle_gap, transforms, operators)
- `dual.rs` - 24 tests (constructors, all arithmetic ops, trig functions, is_normal, ordering)
- `math/cubic.rs`, `math/quartic.rs` - Already had good coverage

**Still missing**:
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

### 9. Address TODOs ✅ MOSTLY DONE

| Location | TODO | Status |
|----------|------|--------|
| `scene.rs:327` | SumOpt trait | ✅ Cleaned up |
| `math/cubic.rs:158,188` | Factor constants | ✅ Made static |
| `component.rs:86` | Shadow variable | ✅ Cleaned up |
| `model.rs:267` | Nondeterminism | ✅ Test passes, TODO removed |
| `targets.rs:140` | "TODO: fsck" | ✅ Cleaned up |

**Remaining TODOs** (low priority, complex to fix):
- `shape.rs:47` - CoordGetter lifetime issue (workaround in place)
- `ellipses/cdef.rs:232` - Type constraint note (explanatory comment)
- `ellipses/xyrr.rs:349,364` - Numerical stability (needs deeper algorithmic work)

## Dependency Updates ✅ AUDITED

### Status
Audited dependencies and updated where safe. Using flexible semver ranges.

### Updated:
- `itertools` 0.11 → 0.14

### Pinned (breaking changes in newer versions):
- `roots = "0.0.8"` - Latest version, no updates available
- `derive_more = "0.99"` - 2.x has breaking API changes
- `thiserror = "1.0"` - 2.x has breaking changes
- `num-dual = "0.7"` - 0.13 has breaking API changes
- `nalgebra = "0.32"` - 0.33+ incompatible with num-dual 0.7
- `polars = "0.44"` - 0.52 has breaking changes (dev-dependency only)

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
