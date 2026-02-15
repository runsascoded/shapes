# Error Components Spec

**Goal**: Enable the frontend to display stacked error bars showing distinct error sources:
1. **Area error (too small)**: target > actual (region needs to grow)
2. **Area error (too large)**: actual > target (region needs to shrink)
3. **Missing region penalty**: region should exist but doesn't
4. **Extra region penalty**: region exists but shouldn't (target = 0)
5. **Synthetic penalties**: perimeter:area ratio, fragment penalties, polygon regularization, etc.

## Current State (`shapes` repo)

### `Error` struct (`apvd-core/src/optimization/step.rs`)
```rust
pub struct Error {
    pub key: String,
    pub actual_area: Option<f64>,
    pub actual_frac: f64,
    pub target_area: f64,
    pub target_frac: f64,
    pub error: Dual,  // signed: actual_frac - target_frac
}
```

The `error` field is signed:
- **Positive** → region too big (actual > target)
- **Negative** → region too small (actual < target)

### Penalty terms (computed but not exposed)
Currently computed in `Step::nxt()`:
- `total_disjoint_penalty`: shapes that should overlap but don't
- `total_contained_penalty`: shapes where one should contain another
- `total_regularization_penalty`: polygon self-intersection + regularity

These are added to the error gradient only (`Dual::new(0., penalty.d())`), their values are invisible to the frontend.

## Proposed Changes

### 1. Add `ErrorKind` enum
```rust
#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub enum ErrorKind {
    /// Region exists and has target > 0, but actual != target
    AreaMismatch {
        /// Positive = too large, negative = too small
        signed_error: f64,
    },
    /// Region should exist (target > 0) but doesn't (actual = 0 or None)
    MissingRegion {
        target_frac: f64,
    },
    /// Region exists (actual > 0) but shouldn't (target = 0)
    ExtraRegion {
        actual_frac: f64,
    },
}
```

### 2. Update `Error` struct
```rust
pub struct Error {
    pub key: String,
    pub actual_area: Option<f64>,
    pub actual_frac: f64,
    pub target_area: f64,
    pub target_frac: f64,
    pub error: Dual,
    pub kind: ErrorKind,  // NEW: classify the error type
}
```

### 3. Add `Penalties` struct
```rust
#[derive(Clone, Debug, Default, Tsify, Serialize, Deserialize)]
pub struct Penalties {
    /// Penalty for shapes that should overlap but are disjoint
    pub disjoint: f64,
    /// Penalty for shapes where one should contain intersection with another
    pub contained: f64,
    /// Polygon self-intersection penalty
    pub self_intersection: f64,
    /// Polygon regularity penalty (edge variance + convexity)
    pub regularity: f64,
    /// Perimeter:area ratio penalty (encourages compact shapes)
    pub perimeter_ratio: f64,
    /// Fragment penalty (penalizes disconnected region components)
    pub fragment: f64,
}

impl Penalties {
    pub fn total(&self) -> f64 {
        self.disjoint + self.contained + self.self_intersection
            + self.regularity + self.perimeter_ratio + self.fragment
    }
}
```

### 4. Update `Step` struct
```rust
pub struct Step {
    pub shapes: Vec<Shape<D>>,
    pub components: Vec<regions::Component>,
    pub targets: Targets<f64>,
    pub total_area: Dual,
    pub errors: Errors,
    pub error: Dual,
    pub converged: bool,
    pub penalties: Penalties,  // NEW: expose penalty breakdown
}
```

### 5. Per-region penalty attribution (optional, Phase 2)
For stacked bars per region, we may want to attribute penalties to specific regions:

```rust
pub struct Error {
    // ... existing fields ...
    pub kind: ErrorKind,
    /// Penalty contributions attributed to this region
    pub penalties: Option<RegionPenalties>,
}

#[derive(Clone, Debug, Default, Tsify, Serialize, Deserialize)]
pub struct RegionPenalties {
    /// This region's share of missing-region penalty
    pub missing: f64,
    /// This region's share of perimeter:area penalty
    pub perimeter_ratio: f64,
    /// This region's share of fragment penalty (if fragmented)
    pub fragment: f64,
}
```

## Frontend Changes (apvd/static)

### Stacked error bar rendering
The `errorBarCell` in `TargetsTable` would render stacked segments:

```tsx
<td className={css.errorBarCell}>
  <div className={css.errorBarStack}>
    {error.kind === 'AreaMismatch' && error.signed_error < 0 && (
      <div className={css.errorBarTooSmall} style={{ width: `${pct(-error.signed_error)}%` }} />
    )}
    {error.kind === 'AreaMismatch' && error.signed_error > 0 && (
      <div className={css.errorBarTooLarge} style={{ width: `${pct(error.signed_error)}%` }} />
    )}
    {error.kind === 'MissingRegion' && (
      <div className={css.errorBarMissing} style={{ width: `${pct(error.target_frac)}%` }} />
    )}
    {error.kind === 'ExtraRegion' && (
      <div className={css.errorBarExtra} style={{ width: `${pct(error.actual_frac)}%` }} />
    )}
    {error.penalties?.perimeter_ratio && (
      <div className={css.errorBarPerimeter} style={{ width: `${pct(error.penalties.perimeter_ratio)}%` }} />
    )}
  </div>
</td>
```

### Color scheme
| Error Type | Color | Pattern |
|------------|-------|---------|
| Too small | Blue | Solid |
| Too large | Red | Solid |
| Missing region | Gray | Diagonal stripes (///) |
| Extra region | Orange | Diagonal stripes (\\\\) |
| Perimeter penalty | Purple | Dotted |
| Fragment penalty | Yellow | Cross-hatch |

### Toggle controls
Add checkbox to show/hide synthetic penalties in the error bars:
- [ ] Show penalty terms (default: off for cleaner view)

## Migration Path

1. **Phase 1**: Add `ErrorKind` to classify existing errors
   - Backward compatible: existing `error` field unchanged
   - Frontend can start using `kind` for coloring

2. **Phase 2**: Expose `Penalties` struct in `Step`
   - Shows global penalty totals
   - Frontend can display in summary section

3. **Phase 3**: Per-region penalty attribution
   - More complex, requires tracking which regions contribute to each penalty
   - Enables full stacked bar visualization

## Open Questions

1. **Penalty scaling**: Should penalties be in the same units as area error (fractions of total area)?
2. **Perimeter:area penalty**: How to define "ideal" ratio? Options:
   - Circle-equivalent (minimal perimeter for given area)
   - Convex hull ratio
   - User-configurable target
3. **Fragment penalty**: How to weight multiple disconnected components?
   - Count-based (N fragments → N-1 penalty)
   - Area-weighted (small fragments penalized more)
