# Per-Region Penalties & Convergence Fix

## Problem

Penalties are currently computed and returned only as Step-level aggregates. This has two issues:

1. **Convergence ignores penalties**: `converged` checks `error.v() < CONVERGENCE_THRESHOLD`, but penalties are added via `Dual::new(0., penalty.d())` — only gradients, not values. Training declares convergence at error ~1e-9 while penalties total ~0.04.

2. **No per-region/per-shape breakdown**: The UI can't show which shapes or regions are driving penalties. Ideally penalty contributions would appear alongside area errors in the targets table.

## Current Architecture (`optimization/step.rs`)

Penalties are computed in `Step::nxt()`:
- **Per-shape** (polygons only): `self_intersection`, `regularity`, `perimeter_area` — iterated per shape, summed into scalars
- **Per-region**: `fragmentation` — iterated per region key, summed
- **Per-missing-region**: `disjoint`, `contained` — iterated per missing target, summed

All 6 are stored as `f64` in `Penalties`, returned on `Step`.

Gradients flow into `error` via `Dual::new(0., penalty.d())` — penalty values are invisible to convergence.

## Proposed Changes

### 1. Fix convergence to include penalty values

Change the convergence check to include penalty magnitudes:

```rust
let penalty_total = penalties.total();
let converged = error.v() + penalty_total < CONVERGENCE_THRESHOLD;
```

Or use a separate (possibly looser) threshold for penalties, since they're on a different scale than area error.

### 2. Per-region penalty breakdown

Return penalty contributions per region, so the frontend can display them in the targets table error bars.

#### Fragmentation → per-region

Already computed per region key. Return `HashMap<String, f64>` mapping region key → fragment area penalty.

#### Perimeter/area → per-shape (and optionally per-disjoint-region)

Currently per-shape (`P²/(4πA) - 1`). Two levels make sense:
- **Per-shape**: each polygon should be generally round
- **Per-disjoint-region** (DR): intersection regions shouldn't be thin slivers. The P/A ratio of each DR's boundary segments could be computed similarly.

#### Regularity → per-shape

Already per-shape. Return `Vec<f64>` indexed by shape.

### 3. Exterior angle penalty (new, per-shape and/or per-DR)

Sum of squares of exterior angles, with extra penalty for negative (concave) exterior angles. This is a clean, differentiable roundness measure that:
- Naturally penalizes concavities (negative exterior angles)
- Complements or could replace the regularity + P/A penalties
- Can be applied per-shape (polygon vertices) and per-DR (region boundary vertices at intersection points)

Concave vertex penalty could use something like `angle² * k` where `k` is larger for negative angles, e.g.:

```rust
let penalty = if exterior_angle < 0.0 {
    exterior_angle.powi(2) * concavity_weight  // e.g. 10x
} else {
    exterior_angle.powi(2)
};
```

### 4. Suggested `Penalties` struct evolution

```rust
pub struct Penalties {
    // Step-level totals (keep for backward compat / quick summary)
    pub disjoint: f64,
    pub contained: f64,
    pub self_intersection: f64,
    pub regularity: f64,
    pub fragmentation: f64,
    pub perimeter_area: f64,

    // Per-region breakdown (new)
    pub per_region: HashMap<String, RegionPenalties>,

    // Per-shape breakdown (new)
    pub per_shape: Vec<ShapePenalties>,
}

pub struct RegionPenalties {
    pub fragmentation: f64,
    pub perimeter_area: f64,  // P/A of the DR boundary
    // pub exterior_angles: f64,  // future
}

pub struct ShapePenalties {
    pub self_intersection: f64,
    pub regularity: f64,
    pub perimeter_area: f64,
    // pub exterior_angles: f64,  // future
}
```

## Frontend Integration

With per-region penalties, the APVD frontend can:
- Show penalty bars alongside area-error bars in the Targets table
- Color-code or annotate which regions have shape quality issues vs. area mismatch
- Show per-shape penalties in the Shapes table

## Priority

1. **Convergence fix** — most impactful, simplest change
2. **Per-region fragmentation** — already computed per-region, just needs to be returned
3. **Per-shape breakdown** — already computed per-shape, just needs struct
4. **Per-DR perimeter/area** — new computation on region boundaries
5. **Exterior angle penalty** — new penalty type, most exploratory
