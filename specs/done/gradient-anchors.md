# Gradient Anchor Points for UI Visualization

## Problem

The APVD frontend wants to render arrows on the SVG diagram showing how the optimizer is pushing each shape variable. Currently, `Step` returns shapes as `Shape<Dual>` but the frontend strips gradients (extracting only `.v`). Raw variable gradients exist but aren't useful for visualization — users want to see arrows at meaningful geometric points, not abstract variable indices.

## Proposal

Compute "anchor points" on each shape as Dual-valued expressions, so forward-mode autodiff gives exact position gradients for free. Return these as a per-shape list of anchors on `Step`.

## Data Structures

```rust
/// A point on a shape where a gradient arrow should be drawn.
#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct GradientAnchor {
    /// Position of the anchor point (where to draw arrow base)
    pub position: R2<f64>,
    /// Error gradient at this anchor: d(error)/d(position)
    /// Frontend draws arrow in *negative* gradient direction (descent)
    pub gradient: R2<f64>,
    /// Human-readable label: "center", "0°", "90°", "v0", "v1", etc.
    pub label: String,
}

// On Step:
pub gradient_anchors: Vec<Vec<GradientAnchor>>  // indexed by shape idx
```

## Anchor Points per Shape Type

### Circle (center + radius)
- **center**: position = `(cx, cy)`, gradient from `(cx.d, cy.d)`
- **0° point**: position = `(cx + r, cy)`, gradient combines center + radius pressure

### XYRR (center + two radii)
- **center**: position = `(cx, cy)`
- **0° point**: position = `(cx + rx, cy)`, shows rx pressure + center drift
- **90° point**: position = `(cx, cy + ry)`, shows ry pressure + center drift

### XYRRT (center + two radii + rotation)
- **center**: position = `(cx, cy)`
- **0° point**: position = `(cx + rx*cos(θ), cy + rx*sin(θ))`, combines center + rx + rotation
- **90° point**: position = `(cx - ry*sin(θ), cy + ry*cos(θ))`, combines center + ry + rotation

The 0° and 90° anchors on XYRRT naturally encode rotation effects because the anchor positions are Dual expressions involving θ. No special rotation arrow needed — the direction the perimeter point "wants to move" already reflects rotational pressure.

### Polygon (each vertex)
- **vertex N**: position = `(vN.x, vN.y)`, gradient = `(vN.x.d, vN.y.d)`
- Label: `"v0"`, `"v1"`, etc.

## Computation

All anchor positions should be computed as `R2<Dual>` expressions inside the existing autodiff context, so gradients come from the chain rule automatically:

```rust
fn gradient_anchors<D: DualNum>(shape: &Shape<D>, error_grad: &[D]) -> Vec<GradientAnchor> {
    match shape {
        Shape::Circle { c, r } => vec![
            anchor("center", c),
            anchor("0°", R2 { x: c.x + r, y: c.y }),
        ],
        Shape::XYRR { c, r } => vec![
            anchor("center", c),
            anchor("0°", R2 { x: c.x + r.x, y: c.y }),
            anchor("90°", R2 { x: c.x, y: c.y + r.y }),
        ],
        Shape::XYRRT { c, r, t } => vec![
            anchor("center", c),
            anchor("0°", R2 {
                x: c.x + r.x * t.cos(),
                y: c.y + r.x * t.sin(),
            }),
            anchor("90°", R2 {
                x: c.x - r.y * t.sin(),
                y: c.y + r.y * t.cos(),
            }),
        ],
        Shape::Polygon { vertices } => vertices.iter().enumerate().map(|(i, v)| {
            anchor(&format!("v{i}"), v)
        }).collect(),
    }
}

fn anchor(label: &str, pos: R2<Dual>) -> GradientAnchor {
    GradientAnchor {
        position: R2 { x: pos.x.v(), y: pos.y.v() },
        gradient: R2 { x: pos.x.d(), y: pos.y.d() },
        label: label.to_string(),
    }
}
```

The key insight: because these positions are computed as Dual expressions during the same forward pass that computes areas/errors, the `.d()` values are the exact gradients of the total error with respect to moving that anchor point — no manual Jacobian needed.

## Integration Point

Compute in `Step::nxt()` (or `Step::new()`), after the error + penalty computation is done and all Duals have their gradients populated. Add `gradient_anchors` to the `Step` struct alongside `penalties`.

## Frontend Usage

The frontend will:
1. Add a `showGradientArrows` toggle (settings context, keyboard shortcut)
2. For each shape's anchors, render SVG arrows:
   - Base at `position`
   - Direction: `-gradient` (negative = descent direction, i.e. where the point *will* move)
   - Length scaled relative to shape size or with a user-adjustable multiplier
   - Color matching shape fill color
3. Optionally show an aggregated arrow per shape in the Shapes table (mean of all anchors' gradients for that shape = net centroid movement direction)

## Considerations

- **Performance**: Anchor computation is trivial compared to region/area computation. No concern.
- **Scaling**: Gradient magnitudes vary wildly (early training vs. near-convergence). The frontend should normalize/scale arrows, probably with a log scale or clamped range.
- **Polygon vertex count**: Polygons with many vertices will have many arrows. The frontend may want a density/decimation option, or just show all of them (typically 7-12 vertices).
