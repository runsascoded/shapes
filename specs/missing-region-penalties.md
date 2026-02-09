# Missing Region Penalties Spec

**Problem**: When a region should exist (target > 0) but doesn't, we need a differentiable penalty that guides optimization to create it.

## Current Implementation (naive)

In `step.rs`, missing regions are penalized based on centroid distances:

```rust
// If parent regions exist (shapes already overlap pairwise):
total_contained_penalty += distance.recip() * target / np;  // 1/dist

// If shapes are completely disjoint:
total_disjoint_penalty += distance * target / nf;  // dist
```

**Problems**:
1. Uses centroid distance, not actual shape distance
2. Doesn't distinguish which specific overlaps are missing
3. For missing region "012" where "01*" exists but "0*2" doesn't, it may push wrong shapes
4. Inverse distance can blow up, raw distance doesn't provide strong gradient when far

## Prerequisite: Unified Shape Distance

All shapes should support `distance_to(other: &Shape) -> Dual`:

- **Polygon-Polygon**: Exact (min vertex-to-edge distance)
- **Circle-Circle**: Exact (center distance - r1 - r2)
- **Ellipse-anything**: Approximate ellipse as N-gon (N=20-40), then use polygon distance

```rust
impl Shape<D> {
    pub fn distance_to(&self, other: &Shape<D>) -> Dual {
        let p1 = self.to_polygon(32);   // N=32 for ellipses, identity for polygons
        let p2 = other.to_polygon(32);
        p1.distance_to(&p2)
    }

    pub fn to_polygon(&self, n: usize) -> Polygon<D> {
        match self {
            Shape::Polygon(p) => p.clone(),
            Shape::Circle(_) | Shape::XYRR(_) | Shape::XYRRT(_) => {
                let vertices = (0..n).map(|i| {
                    let theta = 2.0 * PI * i as f64 / n as f64;
                    self.point_at_theta(theta)
                }).collect();
                Polygon { vertices }
            }
        }
    }
}
```

This also enables **region-to-region distance**: min distance between any two points on the boundaries of two regions. Essential for the penalty framework below.

## Proposed: Parent/Child Penalty Framework

### Definitions

For a missing region with key K (e.g., "012" in a 5-shape diagram "012--"):
- **In-shapes**: Shapes that define the region (indices where K has a digit)
- **Out-shapes**: Shapes excluded from the region (indices where K has `-`)
- **Parent regions**: Regions where one in-shape is removed (e.g., "01-", "0-2", "-12" for "012")
- **Child regions**: Regions where one out-shape is added (e.g., "0123-", "012-4" for "012--")

### Penalty Strategy: Symmetric Cascading Walk

For missing region K (e.g., "123--"):

#### 1. Walk UP to parents (pull together)

Parents of "123--" are: "12---", "1-3--", "-23--"

For each parent:
- If parent **exists**: compute distance from parent region to the missing in-shape
  - E.g., for "123--" missing and "12---" exists: distance("12---" region, shape 3)
  - Penalty pulls shape 3 toward the "12---" region
- If parent **also missing**: recurse! Check *its* parents
  - E.g., if "12---" missing, check "1----", "-2---"
  - Eventually bottoms out at single-shape regions (always exist if shape exists)

```
123-- missing
├── 12--- exists? → pull shape 3 toward region "12---"
├── 1-3-- exists? → pull shape 2 toward region "1-3--"
└── -23-- missing? → recurse:
    ├── -2--- exists? → pull shape 3 toward shape 2
    └── --3-- exists? → pull shape 2 toward shape 3
```

#### 2. Walk DOWN to children (push apart)

Children of "123--" are: "1234-", "123-4"

For each child:
- If child **exists but shouldn't** (target = 0 but region exists):
  - This region is "blocking" the target region
  - Push the extra shape away from the would-be "123--" region
  - E.g., if "1234-" exists, push shape 4 away from region "123--"
  - But "123--" doesn't exist yet... so push shape 4 away from the parent regions that DO exist

```
123-- missing, want to create it
├── 1234- exists with target=0 → push shape 4 away from "12---", "1-3--", etc.
└── 123-4 exists with target=0 → push shape 4 away from "12---", "1-3--", etc.
```

#### 3. Combined penalty

```rust
fn missing_region_penalty(key: &str, shapes: &[Shape<D>], regions: &RegionMap) -> Dual {
    let mut penalty = Dual::zero();

    // Walk UP: pull parent regions toward missing in-shapes
    for (parent_key, missing_shape_idx) in parents_of(key) {
        penalty += pull_penalty(parent_key, missing_shape_idx, shapes, regions);
    }

    // Walk DOWN: push child regions' extra shapes away
    for (child_key, extra_shape_idx) in children_of(key) {
        if regions.exists(child_key) && regions.target(child_key) == 0.0 {
            penalty += push_penalty(child_key, extra_shape_idx, shapes, regions);
        }
    }

    penalty
}

fn pull_penalty(region_key: &str, shape_idx: usize, shapes: &[Shape<D>], regions: &RegionMap) -> Dual {
    if let Some(region) = regions.get(region_key) {
        // Region exists: compute distance from region boundary to shape
        let dist = region.distance_to(&shapes[shape_idx]);
        smooth_pull_penalty(dist)
    } else {
        // Region also missing: recurse to its parents
        parents_of(region_key).map(|(parent, missing_idx)| {
            pull_penalty(parent, missing_idx, shapes, regions)
        }).sum()
    }
}
```

### Distance Functions

**Polygon-Polygon** (exact):
```rust
fn polygon_distance(p1: &Polygon<D>, p2: &Polygon<D>) -> Dual {
    let mut min_dist = Dual::infinity();
    for v in &p1.vertices {
        for edge in p2.edges() {
            min_dist = min_dist.min(point_to_segment_distance(v, edge));
        }
    }
    for v in &p2.vertices {
        for edge in p1.edges() {
            min_dist = min_dist.min(point_to_segment_distance(v, edge));
        }
    }
    min_dist
}
```

**Any Shape** (via N-gon approximation):
- Ellipses approximated as 20-40 point polygons
- Then use polygon-polygon distance
- Differentiable: gradients flow through the sampled points

**Region-to-Shape**:
- Region boundary is a collection of edge segments
- Distance = min over all boundary segments to shape boundary

### Smooth Penalty Function

Instead of raw distance or inverse distance, use smooth barriers:

```rust
fn distance_penalty(dist: Dual, target_overlap: f64) -> Dual {
    // Soft barrier that increases steeply as dist approaches 0 (shapes about to touch)
    // but provides gradient even when far apart

    // Option 1: Log barrier (infinite at 0, gentle far away)
    // -log(dist) if we want shapes to overlap

    // Option 2: Inverse with floor
    // 1 / (dist + epsilon)

    // Option 3: Exponential approach
    // exp(-dist / scale) * target_overlap

    // Recommend: Huber-like that transitions from linear to quadratic
    let scale = 0.1;  // Tune based on typical shape sizes
    if dist.v() > scale {
        // Far apart: linear penalty, constant gradient
        dist.clone() * target_overlap
    } else {
        // Close: quadratic penalty, gradient → 0 as dist → 0
        (dist.clone() * &dist / scale + scale) * target_overlap / 2.0
    }
}
```

### Algorithm Sketch

```rust
fn missing_region_penalty(
    missing_key: &str,
    shapes: &[Shape<D>],
    existing_regions: &HashSet<String>,
) -> Dual {
    let in_shapes: Vec<usize> = /* indices where key has digit */;
    let mut penalty = Dual::zero();

    // For each pair of in-shapes, check if their pairwise overlap exists
    for i in 0..in_shapes.len() {
        for j in (i+1)..in_shapes.len() {
            let pair_key = make_pair_key(in_shapes[i], in_shapes[j], shapes.len());
            if !existing_regions.contains(&pair_key) {
                // These two shapes should overlap but don't
                let dist = shape_distance(&shapes[in_shapes[i]], &shapes[in_shapes[j]]);
                penalty = penalty + distance_penalty(dist, target_area);
            }
        }
    }

    // If all pairwise overlaps exist but the full intersection doesn't,
    // we need a different strategy: find the closest point to all shapes
    // and push shapes toward it
    if penalty.v() == 0.0 && in_shapes.len() > 2 {
        // All pairs overlap but not all together
        // Compute centroid of existing pairwise intersections
        // Add penalty based on distances from non-overlapping shapes to that centroid
        // ... more complex logic
    }

    penalty
}
```

## Implementation Phases

### Phase 1: Unified Shape Distance

Add `to_polygon(n)` and `distance_to` to Shape trait:

```rust
impl Shape<D> {
    /// Convert any shape to a polygon approximation
    pub fn to_polygon(&self, n: usize) -> Polygon<D> {
        match self {
            Shape::Polygon(p) => p.clone(),
            _ => {
                let vertices = (0..n).map(|i| {
                    let theta = 2.0 * PI * i as f64 / n as f64;
                    self.point_at_theta(theta)
                }).collect();
                Polygon { vertices }
            }
        }
    }

    /// Distance to another shape (0 if overlapping, positive if disjoint)
    pub fn distance_to(&self, other: &Shape<D>) -> Dual {
        // Use N=32 for ellipse approximation - good visual fidelity
        let p1 = self.to_polygon(32);
        let p2 = other.to_polygon(32);
        p1.distance_to(&p2)
    }
}

impl Polygon<D> {
    pub fn distance_to(&self, other: &Polygon<D>) -> Dual {
        // Min distance over all vertex-to-edge pairs (both directions)
        // Returns 0 if polygons overlap
    }
}
```

### Phase 2: Region-to-Shape Distance

Extend distance computation to region boundaries:

```rust
impl Region {
    /// Min distance from region boundary to a shape
    pub fn distance_to_shape(&self, shape: &Shape<D>) -> Dual {
        self.segments.iter()
            .map(|seg| seg.distance_to_shape(shape))
            .min()
    }
}
```

### Phase 3: Parent/Child Penalty Framework

Implement the symmetric cascading walk:
- `pull_penalty`: Walk up to existing parents, pull missing shape toward them
- `push_penalty`: Walk down to existing children, push extra shapes away
- Memoize to avoid recomputing for shared ancestors/descendants

## N-gon Approximation Quality

For ellipse-to-polygon conversion:
- **N=20**: Good visual approximation, fast
- **N=32**: Excellent quality, recommended default
- **N=40**: Diminishing returns, use for high-precision needs

The approximation is differentiable because gradients flow through the sampled vertex positions. As N increases, the gradient becomes smoother.

## Open Questions

1. **Signed vs unsigned distance**: Should we use signed distance (negative when overlapping)?
   - Unsigned: simpler, only penalizes disjoint shapes
   - Signed: can also penalize too-much overlap (region too large)
   - Probably want unsigned for missing regions, signed for area errors

2. **Recursion depth**: How deep to walk the parent/child tree?
   - Could cap at 2-3 levels for performance
   - Or weight by depth (deeper = weaker penalty)

3. **Conflicting requirements**: Missing region A needs shapes X,Y to overlap; missing region B needs them apart
   - Weight penalties by target area
   - Or detect and report as infeasible layout

4. **Computational cost**: N² distance checks for N-gon approximation
   - N=32 → 1024 vertex pairs per shape pair
   - Can optimize: bounding box check first, then detailed

5. **Gradient smoothness**: At the "closest point" there's a kink in the distance function
   - Shouldn't matter much in practice
   - Could smooth with small epsilon if needed

## Recommendation

1. **Keep existing penalties** for now (directionally correct)
2. **Priority**: Implement `Shape::distance_to` via N-gon approximation
   - Unifies all shape types
   - Enables future penalty improvements
3. **Then**: Implement parent/child framework using the new distance function
