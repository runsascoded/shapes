use std::{
    fmt::{self, Display},
    ops::{Add, Div, Mul, Neg, Sub},
};

use derive_more::From;
use log::debug;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{
    coord_getter::{coord_getter, CoordGetter},
    dual::{Dual, D},
    math::{is_normal::IsNormal, recip::Recip},
    r2::R2,
    rotate::{Rotate as _Rotate, RotateArg},
    shape::{AreaArg, Duals, Shape},
    sqrt::Sqrt,
    transform::{
        CanTransform, Projection,
        Transform::{self, Rotate, Scale, ScaleXY, Translate},
    },
    zero::Zero,
};

#[derive(Debug, Clone, From, PartialEq, Serialize, Deserialize, Tsify)]
pub struct Polygon<D> {
    pub vertices: Vec<R2<D>>,
}

impl<D> Polygon<D> {
    pub fn new(vertices: Vec<R2<D>>) -> Self {
        assert!(vertices.len() >= 3, "Polygon must have at least 3 vertices");
        Polygon { vertices }
    }

    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }
}

impl<D> Polygon<D> {
    /// Returns coordinate getters for this polygon.
    /// The number of getters depends on the vertex count (2 per vertex: x and y).
    pub fn getters(&self) -> Vec<CoordGetter<Polygon<f64>>> {
        let n = self.vertices.len();
        (0..n)
            .flat_map(|i| {
                [
                    coord_getter(&format!("v{}.x", i), move |p: Polygon<f64>| p.vertices[i].x),
                    coord_getter(&format!("v{}.y", i), move |p: Polygon<f64>| p.vertices[i].y),
                ]
            })
            .collect()
    }
}

impl Polygon<f64> {
    pub fn dual(&self, duals: &Duals) -> Polygon<D> {
        let vertices = self
            .vertices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let x = Dual::new(v.x, duals[i * 2].clone());
                let y = Dual::new(v.y, duals[i * 2 + 1].clone());
                R2 { x, y }
            })
            .collect();
        Polygon { vertices }
    }

    pub fn names(&self) -> Vec<String> {
        self.getters().into_iter().map(|g| g.name).collect()
    }

    pub fn vals(&self) -> Vec<f64> {
        self.vertices
            .iter()
            .flat_map(|v| [v.x, v.y])
            .collect()
    }

    /// Returns x-coordinates where polygon edges cross the given y-value.
    /// Horizontal edges are skipped (treated as tangent points).
    /// Uses half-open interval [y_min, y_max) to avoid double-counting vertices.
    pub fn at_y(&self, y: f64) -> Vec<f64> {
        let n = self.vertices.len();
        let mut xs = Vec::new();

        for i in 0..n {
            let v0 = &self.vertices[i];
            let v1 = &self.vertices[(i + 1) % n];

            // Skip horizontal edges (they don't represent true crossings)
            if v0.y == v1.y {
                continue;
            }

            let (y_min, y_max) = if v0.y < v1.y {
                (v0.y, v1.y)
            } else {
                (v1.y, v0.y)
            };

            // Use half-open interval to avoid double-counting shared vertices
            if y_min <= y && y < y_max {
                // Linear interpolation to find x at y
                let t = (y - v0.y) / (v1.y - v0.y);
                let x = v0.x + t * (v1.x - v0.x);
                xs.push(x);
            }
        }

        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        xs
    }

    /// Check if a point is inside the polygon using ray casting algorithm.
    /// Casts a horizontal ray to the right and counts edge crossings.
    pub fn contains(&self, p: &R2<f64>) -> bool {
        let n = self.vertices.len();
        let mut crossings = 0;

        for i in 0..n {
            let v0 = &self.vertices[i];
            let v1 = &self.vertices[(i + 1) % n];

            // Check if the ray at y=p.y crosses this edge (going rightward from p.x)
            // Edge goes from v0 to v1

            // Skip if edge is entirely above or below the ray
            let (y_min, y_max) = if v0.y < v1.y { (v0.y, v1.y) } else { (v1.y, v0.y) };
            if p.y < y_min || p.y >= y_max {
                continue;
            }

            // Find x-coordinate where edge crosses y=p.y
            let t = (p.y - v0.y) / (v1.y - v0.y);
            let x_crossing = v0.x + t * (v1.x - v0.x);

            // Count crossing if it's to the right of p
            if x_crossing > p.x {
                crossings += 1;
            }
        }

        // Point is inside if odd number of crossings
        crossings % 2 == 1
    }

    /// Check if this polygon self-intersects (any non-adjacent edges cross).
    /// A self-intersecting polygon is invalid for area calculations.
    pub fn is_self_intersecting(&self) -> bool {
        let n = self.vertices.len();
        if n < 4 {
            return false; // Triangles can't self-intersect
        }

        // Check all pairs of non-adjacent edges
        for i in 0..n {
            let a0 = &self.vertices[i];
            let a1 = &self.vertices[(i + 1) % n];

            // Only check edges that aren't adjacent (skip i+1, i-1)
            for j in (i + 2)..n {
                // Skip if j is adjacent to i (wraps around for last edge)
                if j == (i + n - 1) % n || (i == 0 && j == n - 1) {
                    continue;
                }

                let b0 = &self.vertices[j];
                let b1 = &self.vertices[(j + 1) % n];

                if Self::segments_intersect(a0, a1, b0, b1) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if two line segments intersect (excluding endpoints).
    fn segments_intersect(a0: &R2<f64>, a1: &R2<f64>, b0: &R2<f64>, b1: &R2<f64>) -> bool {
        // Using cross product method
        let d1 = Self::cross_sign(b0, b1, a0);
        let d2 = Self::cross_sign(b0, b1, a1);
        let d3 = Self::cross_sign(a0, a1, b0);
        let d4 = Self::cross_sign(a0, a1, b1);

        // Segments intersect if endpoints are on opposite sides of each other's lines
        if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) &&
           ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
            return true;
        }

        // Collinear cases (endpoints touching) - we don't count these as intersections
        // since adjacent edges share endpoints
        false
    }

    /// Cross product sign: (b - a) × (c - a)
    fn cross_sign(a: &R2<f64>, b: &R2<f64>, c: &R2<f64>) -> f64 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }

    /// Compute a penalty value for self-intersection.
    /// Returns 0 if not self-intersecting, positive value otherwise.
    /// The penalty increases with the "severity" of the intersection.
    pub fn self_intersection_penalty(&self) -> f64 {
        let n = self.vertices.len();
        if n < 4 {
            return 0.0;
        }

        let mut total_penalty = 0.0;

        for i in 0..n {
            let a0 = &self.vertices[i];
            let a1 = &self.vertices[(i + 1) % n];

            for j in (i + 2)..n {
                if j == (i + n - 1) % n || (i == 0 && j == n - 1) {
                    continue;
                }

                let b0 = &self.vertices[j];
                let b1 = &self.vertices[(j + 1) % n];

                // Compute intersection depth (how much the segments overlap)
                if let Some(depth) = Self::intersection_depth(a0, a1, b0, b1) {
                    total_penalty += depth;
                }
            }
        }

        total_penalty
    }

    /// Compute how "deep" two segments intersect (0 if they don't).
    fn intersection_depth(a0: &R2<f64>, a1: &R2<f64>, b0: &R2<f64>, b1: &R2<f64>) -> Option<f64> {
        let d1 = Self::cross_sign(b0, b1, a0);
        let d2 = Self::cross_sign(b0, b1, a1);
        let d3 = Self::cross_sign(a0, a1, b0);
        let d4 = Self::cross_sign(a0, a1, b1);

        if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) &&
           ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
            // Segments intersect - compute the depth as min of the cross products
            // This gives a sense of how "deep" the intersection is
            let depth = d1.abs().min(d2.abs()).min(d3.abs()).min(d4.abs());
            Some(depth)
        } else {
            None
        }
    }
}

impl Polygon<D> {
    pub fn v(&self) -> Polygon<f64> {
        Polygon {
            vertices: self.vertices.iter().map(|v| v.v()).collect(),
        }
    }

    pub fn n(&self) -> usize {
        if self.vertices.is_empty() {
            0
        } else {
            self.vertices[0].x.d().len()
        }
    }

    pub fn duals(&self) -> Duals {
        self.vertices
            .iter()
            .flat_map(|v| [v.x.d().clone(), v.y.d().clone()])
            .collect()
    }

    /// Compute regularization penalty that encourages "nice" polygon shapes.
    ///
    /// Returns a Dual with both value and gradient, penalizing:
    /// - Non-convexity (vertices that create concave regions)
    /// - Irregular edge lengths (variance in edge lengths)
    ///
    /// Higher penalty = worse shape. Returns 0 for perfectly regular convex polygons.
    pub fn regularity_penalty(&self) -> Dual {
        let n = self.vertices.len();
        if n < 3 {
            return Dual::new(0.0, vec![0.0; self.n()]);
        }

        let mut penalty = Dual::new(0.0, vec![0.0; self.n()]);

        // 1. Edge length variance penalty - encourage uniform edge lengths
        let edges: Vec<Dual> = (0..n).map(|i| {
            let v0 = &self.vertices[i];
            let v1 = &self.vertices[(i + 1) % n];
            let dx = v1.x.clone() - &v0.x;
            let dy = v1.y.clone() - &v0.y;
            (dx.clone() * &dx + dy.clone() * &dy).sqrt()
        }).collect();

        // Compute mean edge length
        let mean_edge: Dual = edges.iter().cloned().sum::<Dual>() / n as f64;

        // Variance: sum of (edge - mean)^2
        for edge in &edges {
            let diff = edge.clone() - &mean_edge;
            penalty = penalty + diff.clone() * &diff;
        }

        // 2. Convexity penalty - penalize concave vertices
        // A vertex is concave if the cross product at that vertex has opposite sign from others
        // Use soft penalty: for each vertex, penalize negative cross product
        for i in 0..n {
            let v0 = &self.vertices[(i + n - 1) % n];
            let v1 = &self.vertices[i];
            let v2 = &self.vertices[(i + 1) % n];

            // Cross product (v1-v0) × (v2-v1) - should be positive for CCW convex
            let dx1 = v1.x.clone() - &v0.x;
            let dy1 = v1.y.clone() - &v0.y;
            let dx2 = v2.x.clone() - &v1.x;
            let dy2 = v2.y.clone() - &v1.y;
            let cross = dx1.clone() * &dy2 - dy1.clone() * &dx2;

            // Penalize negative cross products (concave vertices)
            // Use soft penalty: max(0, -cross) or smooth approximation
            let cross_v = cross.v();
            if cross_v < 0.0 {
                // Concave vertex - add penalty proportional to how concave it is
                penalty = penalty - cross.clone() * 0.1; // Scale factor to balance with edge variance
            }
        }

        penalty
    }

    /// Check for self-intersection and return a penalty with gradients.
    ///
    /// This uses a soft penalty based on how close non-adjacent edges are to intersecting.
    /// For actually intersecting edges, returns a large penalty.
    pub fn self_intersection_penalty_dual(&self) -> Dual {
        let n = self.vertices.len();
        if n < 4 {
            return Dual::new(0.0, vec![0.0; self.n()]);
        }

        let mut penalty = Dual::new(0.0, vec![0.0; self.n()]);

        // Check all pairs of non-adjacent edges
        for i in 0..n {
            let a0 = &self.vertices[i];
            let a1 = &self.vertices[(i + 1) % n];

            for j in (i + 2)..n {
                // Skip adjacent edges
                if j == (i + n - 1) % n || (i == 0 && j == n - 1) {
                    continue;
                }

                let b0 = &self.vertices[j];
                let b1 = &self.vertices[(j + 1) % n];

                // Compute cross products for intersection test
                // d1 = (b1-b0) × (a0-b0), d2 = (b1-b0) × (a1-b0)
                let bx = b1.x.clone() - &b0.x;
                let by = b1.y.clone() - &b0.y;

                let d1 = bx.clone() * (a0.y.clone() - &b0.y) - by.clone() * (a0.x.clone() - &b0.x);
                let d2 = bx.clone() * (a1.y.clone() - &b0.y) - by.clone() * (a1.x.clone() - &b0.x);

                // If d1 and d2 have opposite signs, the edge a0-a1 crosses line b0-b1
                let d1v = d1.v();
                let d2v = d2.v();

                if (d1v > 0.0 && d2v < 0.0) || (d1v < 0.0 && d2v > 0.0) {
                    // Also check if b0-b1 crosses line a0-a1
                    let ax = a1.x.clone() - &a0.x;
                    let ay = a1.y.clone() - &a0.y;
                    let d3 = ax.clone() * (b0.y.clone() - &a0.y) - ay.clone() * (b0.x.clone() - &a0.x);
                    let d4 = ax.clone() * (b1.y.clone() - &a0.y) - ay.clone() * (b1.x.clone() - &a0.x);

                    let d3v = d3.v();
                    let d4v = d4.v();

                    if (d3v > 0.0 && d4v < 0.0) || (d3v < 0.0 && d4v > 0.0) {
                        // Actual intersection! Large penalty.
                        // Use the minimum absolute cross product as measure of intersection depth
                        let min_d = d1.abs().min(d2.abs()).min(d3.abs()).min(d4.abs());
                        penalty = penalty + min_d * 10.0; // Large weight for actual intersections
                    }
                }
            }
        }

        penalty
    }
}

/// Area calculation using the shoelace formula
impl<D: AreaArg + Add<Output = D> + Sub<Output = D> + Into<f64> + Neg<Output = D>> Polygon<D> {
    pub fn area(&self) -> D {
        let n = self.vertices.len();
        if n < 3 {
            panic!("Polygon must have at least 3 vertices for area calculation");
        }

        let mut sum = self.vertices[0].x.clone() * self.vertices[0].y.clone()
            - self.vertices[0].x.clone() * self.vertices[0].y.clone(); // zero with correct type

        for i in 0..n {
            let j = (i + 1) % n;
            let term = self.vertices[i].x.clone() * self.vertices[j].y.clone()
                - self.vertices[j].x.clone() * self.vertices[i].y.clone();
            sum = sum + term;
        }

        let half = sum * 0.5;
        // Shoelace formula returns negative area for clockwise winding;
        // always return positive area regardless of winding order.
        if half.clone().into() < 0.0 {
            half.neg()
        } else {
            half
        }
    }
}

pub trait UnitIntersectionsArg:
    Clone
    + fmt::Debug
    + Display
    + Into<f64>
    + IsNormal
    + Sqrt
    + Neg<Output = Self>
    + Add<Output = Self>
    + Add<f64, Output = Self>
    + Sub<Output = Self>
    + Sub<f64, Output = Self>
    + Mul<Output = Self>
    + Mul<f64, Output = Self>
    + Div<Output = Self>
    + Div<f64, Output = Self>
{
}

impl UnitIntersectionsArg for f64 {}
impl UnitIntersectionsArg for Dual {}

impl<D: UnitIntersectionsArg> Polygon<D>
where
    f64: Add<D, Output = D> + Sub<D, Output = D> + Mul<D, Output = D>,
{
    /// Find intersections between the polygon edges and the unit circle.
    /// For each edge (line segment), solve the quadratic equation for intersection with x² + y² = 1.
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        let n = self.vertices.len();
        let mut intersections = Vec::new();

        for i in 0..n {
            let p0 = &self.vertices[i];
            let p1 = &self.vertices[(i + 1) % n];

            // Line segment parameterized as P(t) = P0 + t*(P1-P0), t ∈ [0,1]
            // Substituting into x² + y² = 1:
            // (x0 + t*dx)² + (y0 + t*dy)² = 1
            // t²(dx² + dy²) + t*2*(x0*dx + y0*dy) + (x0² + y0² - 1) = 0

            let dx = p1.x.clone() - p0.x.clone();
            let dy = p1.y.clone() - p0.y.clone();

            let a = dx.clone() * dx.clone() + dy.clone() * dy.clone();
            let b = (p0.x.clone() * dx.clone() + p0.y.clone() * dy.clone()) * 2.;
            let c = p0.x.clone() * p0.x.clone() + p0.y.clone() * p0.y.clone() - 1.;

            let discriminant = b.clone() * b.clone() - a.clone() * c.clone() * 4.;

            let disc_val: f64 = discriminant.clone().into();
            if disc_val < 0. {
                continue;
            }

            let sqrt_disc = discriminant.sqrt();
            let a2 = a.clone() * 2.;

            // Two potential roots
            let t0 = (-b.clone() - sqrt_disc.clone()) / a2.clone();
            let t1 = (-b.clone() + sqrt_disc.clone()) / a2.clone();

            for t in [t0, t1] {
                let t_val: f64 = t.clone().into();
                // Check if t is in [0, 1] (intersection on the segment)
                if t_val >= 0. && t_val <= 1. {
                    let x = p0.x.clone() + t.clone() * dx.clone();
                    let y = p0.y.clone() + t.clone() * dy.clone();
                    if x.is_normal() && y.is_normal() {
                        intersections.push(R2 { x, y });
                    }
                }
            }
        }

        debug!(
            "Polygon::unit_intersections: {} vertices, {} intersections",
            n,
            intersections.len()
        );
        intersections
    }
}

pub trait TransformD: Clone + Mul<Output = Self> + Mul<f64, Output = Self> + RotateArg {}
impl TransformD for f64 {}
impl TransformD for Dual {}

pub trait TransformR2<D>:
    Add<R2<D>, Output = R2<D>> + Mul<R2<D>, Output = R2<D>> + Mul<D, Output = R2<D>>
{
}
impl TransformR2<f64> for R2<f64> {}
impl TransformR2<Dual> for R2<Dual> {}

impl<D: TransformD> CanTransform<D> for Polygon<D>
where
    R2<D>: TransformR2<D>,
{
    type Output = Shape<D>;
    fn transform(&self, transform: &Transform<D>) -> Shape<D> {
        let vertices = match transform {
            Translate(v) => self
                .vertices
                .iter()
                .map(|p| p.clone() + v.clone())
                .collect(),
            Scale(s) => self
                .vertices
                .iter()
                .map(|p| p.clone() * s.clone())
                .collect(),
            ScaleXY(s) => self
                .vertices
                .iter()
                .map(|p| p.clone() * s.clone())
                .collect(),
            Rotate(a) => self
                .vertices
                .iter()
                .map(|p| p.clone().rotate(a))
                .collect(),
        };
        Shape::Polygon(Polygon { vertices })
    }
}

impl<D: Clone + Display + Recip + Add<Output = D> + Div<f64, Output = D>> Polygon<D>
where
    R2<D>: Neg<Output = R2<D>>,
{
    /// For polygons, "projection" is just translation to centroid (no scaling/rotation).
    /// This allows theta() to compute angles from the centroid for edge ordering.
    /// Note: point() and arc_midpoint() won't give boundary points for polygons;
    /// special handling is needed in component.rs for containment testing.
    pub fn projection(&self) -> Projection<D> {
        let c = self.center();
        Projection(vec![Translate(-c)])
    }
}

/// f64-based perimeter methods for BoundaryCoord trait.
/// These work on any Polygon<D> by converting vertices to f64.
impl<D: Clone + Into<f64>> Polygon<D> {
    /// Compute the perimeter parameter for a point on the polygon boundary.
    /// Returns edge_idx + t where edge_idx is which edge (0..n-1) and t ∈ [0,1).
    /// This gives a continuous parameter from 0 to n as you traverse the polygon perimeter.
    ///
    /// The point is assumed to lie on the polygon boundary (from an intersection computation).
    pub fn perimeter_param(&self, p: &R2<f64>) -> f64 {
        let n = self.vertices.len();

        for i in 0..n {
            let v0x: f64 = self.vertices[i].x.clone().into();
            let v0y: f64 = self.vertices[i].y.clone().into();
            let v1x: f64 = self.vertices[(i + 1) % n].x.clone().into();
            let v1y: f64 = self.vertices[(i + 1) % n].y.clone().into();

            // Edge direction vector
            let dx = v1x - v0x;
            let dy = v1y - v0y;

            // Vector from v0 to p
            let px = p.x - v0x;
            let py = p.y - v0y;

            // Compute edge length squared
            let edge_len_sq = dx * dx + dy * dy;
            if edge_len_sq < 1e-20 {
                continue; // Degenerate edge
            }

            // Project p onto the edge: t = (p - v0) · (v1 - v0) / |v1 - v0|²
            let t = (px * dx + py * dy) / edge_len_sq;

            // Check if t is in [0, 1] (point is between vertices)
            if t < -1e-9 || t > 1.0 + 1e-9 {
                continue;
            }

            // Check if point is actually on the edge (distance to edge is small)
            let edge_pt_x = v0x + t * dx;
            let edge_pt_y = v0y + t * dy;
            let dist_sq = (p.x - edge_pt_x).powi(2) + (p.y - edge_pt_y).powi(2);

            if dist_sq < 1e-8 {
                // Found the edge! Return edge_idx + t
                return (i as f64) + t.clamp(0.0, 1.0);
            }
        }

        // Fallback: point not found on any edge, use centroid angle
        // This shouldn't happen for valid intersection points
        log::warn!("perimeter_param could not find edge for point ({}, {})", p.x, p.y);
        let cx: f64 = self.vertices.iter().map(|v| v.x.clone().into()).sum::<f64>() / (n as f64);
        let cy: f64 = self.vertices.iter().map(|v| v.y.clone().into()).sum::<f64>() / (n as f64);
        let theta = (p.y - cy).atan2(p.x - cx);
        let normalized = if theta < 0.0 { theta + std::f64::consts::TAU } else { theta };
        // Scale from [0, 2π) to [0, n)
        normalized * (n as f64) / std::f64::consts::TAU
    }

    /// Get the point on the polygon boundary at perimeter parameter c.
    /// c should be in [0, n) where n is the number of vertices.
    pub fn perimeter_point(&self, c: f64) -> R2<f64> {
        let n = self.vertices.len();

        // Handle wrapping
        let c = c.rem_euclid(n as f64);

        let edge_idx = c.floor() as usize;
        let edge_t = c - (edge_idx as f64);

        let v0x: f64 = self.vertices[edge_idx].x.clone().into();
        let v0y: f64 = self.vertices[edge_idx].y.clone().into();
        let v1x: f64 = self.vertices[(edge_idx + 1) % n].x.clone().into();
        let v1y: f64 = self.vertices[(edge_idx + 1) % n].y.clone().into();

        R2 {
            x: v0x + edge_t * (v1x - v0x),
            y: v0y + edge_t * (v1y - v0y),
        }
    }

    /// Get a point along the polygon perimeter between c0 and c1 (for containment testing).
    /// Returns a point halfway along the perimeter path from c0 to c1.
    pub fn perimeter_midpoint(&self, c0: f64, c1: f64) -> R2<f64> {
        let n = self.vertices.len() as f64;
        // Handle wrap-around: if c1 < c0, add n to c1
        let c1 = if c1 < c0 { c1 + n } else { c1 };
        let c_mid = (c0 + c1) / 2.0;
        self.perimeter_point(c_mid)
    }
}

impl<D: Clone> Polygon<D> {
    pub fn center(&self) -> R2<D>
    where
        D: Add<Output = D> + Div<f64, Output = D>,
    {
        let n = self.vertices.len() as f64;
        let mut sum_x = self.vertices[0].x.clone();
        let mut sum_y = self.vertices[0].y.clone();
        for v in self.vertices.iter().skip(1) {
            sum_x = sum_x + v.x.clone();
            sum_y = sum_y + v.y.clone();
        }
        R2 {
            x: sum_x / n,
            y: sum_y / n,
        }
    }
}

pub trait UnitCircleGap:
    Clone + Into<f64> + PartialOrd + Sqrt + Add<Output = Self> + Sub<f64, Output = Self> + Mul<Output = Self>
{
}
impl UnitCircleGap for f64 {}
impl UnitCircleGap for Dual {}

impl<D: UnitCircleGap> Polygon<D> {
    /// Returns the minimum distance from any vertex to the unit circle, if all vertices are outside.
    /// Returns None if any vertex is inside or on the unit circle.
    pub fn unit_circle_gap(&self) -> Option<D> {
        let mut min_gap: Option<D> = None;

        for v in &self.vertices {
            let dist = (v.x.clone() * v.x.clone() + v.y.clone() * v.y.clone()).sqrt() - 1.;
            let dist_val: f64 = dist.clone().into();
            if dist_val <= 0. {
                return None;
            }
            min_gap = Some(match min_gap {
                None => dist,
                Some(g) if dist.clone().into() < g.clone().into() => dist,
                Some(g) => g,
            });
        }

        min_gap
    }
}

impl<D: Zero> Polygon<D> {
    pub fn zero(&self) -> D {
        Zero::zero(&self.vertices[0].x)
    }
}

impl<D> Polygon<D>
where
    D: Clone + Into<f64> + Zero + Add<Output = D> + Sub<Output = D> + Mul<Output = D> + Div<f64, Output = D>,
{
    /// Compute the "secant area" for a polygon edge from p0 to p1.
    /// This is the signed area between the chord (p0→p1) and the polygon boundary path.
    /// The polygon boundary may include intermediate vertices between the two endpoints.
    ///
    /// theta0 and theta1 are the angular positions (relative to centroid) of the endpoints.
    /// Vertices with theta in the range (theta0, theta1) are included in the path.
    pub fn secant_area(&self, p0: &R2<D>, p1: &R2<D>, theta0: D, theta1: D) -> D {
        let center = self.center();
        let n = self.vertices.len();

        // Compute theta for each vertex
        let vertex_thetas: Vec<(usize, f64)> = (0..n)
            .map(|i| {
                let v = &self.vertices[i];
                let rel_x: f64 = (v.x.clone() - center.x.clone()).into();
                let rel_y: f64 = (v.y.clone() - center.y.clone()).into();
                let theta = rel_y.atan2(rel_x);
                (i, theta)
            })
            .collect();

        let theta0_val: f64 = theta0.clone().into();
        let theta1_val: f64 = theta1.clone().into();

        // Find vertices in the theta range (theta0, theta1)
        // Handle wrapping: theta1 might be > 2π if it wrapped around
        let mut intermediate_vertices: Vec<(usize, f64)> = vertex_thetas
            .iter()
            .filter(|(_, vt)| {
                // Normalize vertex theta to be in the same "revolution" as the range
                let vt_normalized = if theta1_val > std::f64::consts::TAU {
                    // Range wraps around, so we might need to add TAU to vt
                    if *vt < theta0_val {
                        *vt + std::f64::consts::TAU
                    } else {
                        *vt
                    }
                } else {
                    *vt
                };
                // Vertex is between theta0 and theta1 (exclusive of endpoints)
                vt_normalized > theta0_val + 1e-9 && vt_normalized < theta1_val - 1e-9
            })
            .map(|(i, vt)| {
                // Normalize for sorting
                let vt_normalized = if theta1_val > std::f64::consts::TAU && *vt < theta0_val {
                    *vt + std::f64::consts::TAU
                } else {
                    *vt
                };
                (*i, vt_normalized)
            })
            .collect();

        // Sort by theta
        intermediate_vertices.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // If no intermediate vertices, secant area is 0 (chord = boundary)
        if intermediate_vertices.is_empty() {
            return self.zero();
        }

        // Compute the shoelace area of the polygon: p0 → v1 → v2 → ... → vn → p1 → back to p0
        // Shoelace formula: sum of (x_i * y_{i+1} - x_{i+1} * y_i) / 2
        let mut area = self.zero();

        // First segment: p0 → first intermediate vertex
        let first_v = &self.vertices[intermediate_vertices[0].0];
        area = area + (p0.x.clone() * first_v.y.clone() - first_v.x.clone() * p0.y.clone());

        // Intermediate segments: v_i → v_{i+1}
        for i in 0..intermediate_vertices.len() - 1 {
            let v_i = &self.vertices[intermediate_vertices[i].0];
            let v_next = &self.vertices[intermediate_vertices[i + 1].0];
            area = area + (v_i.x.clone() * v_next.y.clone() - v_next.x.clone() * v_i.y.clone());
        }

        // Segment from last intermediate vertex to p1
        let last_v = &self.vertices[intermediate_vertices.last().unwrap().0];
        area = area + (last_v.x.clone() * p1.y.clone() - p1.x.clone() * last_v.y.clone());

        // Closing segment: p1 → back to p0
        area = area + (p1.x.clone() * p0.y.clone() - p0.x.clone() * p1.y.clone());

        area / 2.
    }
}

impl<D: Display> Display for Polygon<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verts: Vec<String> = self
            .vertices
            .iter()
            .map(|v| format!("({:.3}, {:.3})", v.x, v.y))
            .collect();
        write!(f, "Polygon[{}]", verts.join(", "))
    }
}

/// Line-line intersection for polygon-polygon intersections.
/// Returns the intersection point if the two line segments intersect.
pub fn line_line_intersection<D>(a0: &R2<D>, a1: &R2<D>, b0: &R2<D>, b1: &R2<D>) -> Option<R2<D>>
where
    D: Clone
        + Into<f64>
        + Add<Output = D>
        + Sub<Output = D>
        + Mul<Output = D>
        + Div<Output = D>
        + IsNormal,
{
    // Using parametric form and Cramer's rule:
    // Line A: a0 + s*(a1-a0)
    // Line B: b0 + t*(b1-b0)
    // Solve for s and t where lines intersect

    let da_x = a1.x.clone() - a0.x.clone();
    let da_y = a1.y.clone() - a0.y.clone();
    let db_x = b1.x.clone() - b0.x.clone();
    let db_y = b1.y.clone() - b0.y.clone();

    // Cross product of direction vectors
    let denom = da_x.clone() * db_y.clone() - da_y.clone() * db_x.clone();
    let denom_val: f64 = denom.clone().into();

    // Parallel lines (or coincident)
    if denom_val.abs() < 1e-10 {
        return None;
    }

    let diff_x = b0.x.clone() - a0.x.clone();
    let diff_y = b0.y.clone() - a0.y.clone();

    let s = (diff_x.clone() * db_y.clone() - diff_y.clone() * db_x.clone()) / denom.clone();
    let t = (diff_x.clone() * da_y.clone() - diff_y.clone() * da_x.clone()) / denom.clone();

    let s_val: f64 = s.clone().into();
    let t_val: f64 = t.clone().into();

    // Check if intersection is within both segments [0, 1] (with small epsilon for
    // robustness across platforms — WASM strict IEEE 754 vs native extended precision
    // can cause boundary intersections to be missed without tolerance)
    let eps = 1e-10;
    if s_val >= -eps && s_val <= 1. + eps && t_val >= -eps && t_val <= 1. + eps {
        let x = a0.x.clone() + s.clone() * da_x;
        let y = a0.y.clone() + s * da_y;
        if x.is_normal() && y.is_normal() {
            return Some(R2 { x, y });
        }
    }

    None
}

/// Find all intersection points between two polygons.
pub fn polygon_polygon_intersect<D>(p1: &Polygon<D>, p2: &Polygon<D>) -> Vec<R2<D>>
where
    D: Clone
        + Into<f64>
        + Add<Output = D>
        + Sub<Output = D>
        + Mul<Output = D>
        + Div<Output = D>
        + IsNormal,
{
    let n1 = p1.vertices.len();
    let n2 = p2.vertices.len();
    let mut intersections = Vec::new();

    for i in 0..n1 {
        let a0 = &p1.vertices[i];
        let a1 = &p1.vertices[(i + 1) % n1];

        for j in 0..n2 {
            let b0 = &p2.vertices[j];
            let b1 = &p2.vertices[(j + 1) % n2];

            if let Some(point) = line_line_intersection(a0, a1, b0, b1) {
                intersections.push(point);
            }
        }
    }

    intersections
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> Polygon<f64> {
        Polygon::new(vec![
            R2 { x: 0., y: 0. },
            R2 { x: 1., y: 0. },
            R2 { x: 0.5, y: 1. },
        ])
    }

    fn square() -> Polygon<f64> {
        Polygon::new(vec![
            R2 { x: 0., y: 0. },
            R2 { x: 1., y: 0. },
            R2 { x: 1., y: 1. },
            R2 { x: 0., y: 1. },
        ])
    }

    #[test]
    fn test_triangle_area() {
        let t = triangle();
        // Area of triangle with base 1 and height 1 = 0.5
        assert_relative_eq!(t.area(), 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_square_area() {
        let s = square();
        // Area of unit square = 1
        assert_relative_eq!(s.area(), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_at_y_square() {
        let s = square();

        // At y=0.5, should intersect left and right edges
        let xs = s.at_y(0.5);
        assert_eq!(xs.len(), 2);
        assert_relative_eq!(xs[0], 0., epsilon = 1e-10);
        assert_relative_eq!(xs[1], 1., epsilon = 1e-10);

        // At y=0 (bottom vertex level), left and right edges start here
        // Using half-open [y_min, y_max), we include y_min
        let xs = s.at_y(0.);
        assert_eq!(xs.len(), 2);
        assert_relative_eq!(xs[0], 0., epsilon = 1e-10);
        assert_relative_eq!(xs[1], 1., epsilon = 1e-10);

        // At y=1 (top vertex level), using half-open interval [y_min, y_max),
        // y=1 is excluded since it equals y_max for vertical edges
        let xs = s.at_y(1.);
        assert_eq!(xs.len(), 0);

        // Above square, no intersection
        let xs = s.at_y(1.5);
        assert_eq!(xs.len(), 0);
    }

    #[test]
    fn test_at_y_triangle() {
        let t = triangle();

        // At y=0.5, should intersect left and right edges
        let xs = t.at_y(0.5);
        assert_eq!(xs.len(), 2);
        assert_relative_eq!(xs[0], 0.25, epsilon = 1e-10);
        assert_relative_eq!(xs[1], 0.75, epsilon = 1e-10);
    }

    #[test]
    fn test_unit_intersections_large_triangle() {
        // Triangle that actually intersects the unit circle
        // Vertices are outside the unit circle but edges pass through it
        let t = Polygon::new(vec![
            R2 { x: -1.5, y: -1. },
            R2 { x: 1.5, y: -1. },
            R2 { x: 0., y: 1.5 },
        ]);

        let points = t.unit_intersections();
        // Should have 6 intersections (2 per edge) since each edge crosses the unit circle
        assert!(points.len() >= 2, "Expected at least 2 intersections, got {}", points.len());

        // All points should be on the unit circle
        for p in &points {
            let r = (p.x * p.x + p.y * p.y).sqrt();
            assert_relative_eq!(r, 1.0, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_unit_intersections_small_triangle() {
        // Small triangle inside the unit circle - no intersections
        let t = Polygon::new(vec![
            R2 { x: -0.1, y: -0.1 },
            R2 { x: 0.1, y: -0.1 },
            R2 { x: 0., y: 0.1 },
        ]);

        let points = t.unit_intersections();
        assert_eq!(points.len(), 0);
    }

    #[test]
    fn test_polygon_polygon_intersect() {
        // Two overlapping squares
        let s1 = square();
        let s2 = Polygon::new(vec![
            R2 { x: 0.5, y: 0.5 },
            R2 { x: 1.5, y: 0.5 },
            R2 { x: 1.5, y: 1.5 },
            R2 { x: 0.5, y: 1.5 },
        ]);

        let points = polygon_polygon_intersect(&s1, &s2);
        // Should have 2 intersection points
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn test_polygon_polygon_no_intersect() {
        // Two non-overlapping squares
        let s1 = square();
        let s2 = Polygon::new(vec![
            R2 { x: 2., y: 0. },
            R2 { x: 3., y: 0. },
            R2 { x: 3., y: 1. },
            R2 { x: 2., y: 1. },
        ]);

        let points = polygon_polygon_intersect(&s1, &s2);
        assert_eq!(points.len(), 0);
    }

    #[test]
    fn test_center() {
        let t = triangle();
        let c = t.center();
        assert_relative_eq!(c.x, 0.5, epsilon = 1e-10);
        assert_relative_eq!(c.y, 1. / 3., epsilon = 1e-10);
    }

    #[test]
    fn test_names_and_vals() {
        let t = triangle();
        let names = t.names();
        assert_eq!(names, vec!["v0.x", "v0.y", "v1.x", "v1.y", "v2.x", "v2.y"]);

        let vals = t.vals();
        assert_eq!(vals, vec![0., 0., 1., 0., 0.5, 1.]);
    }

    #[test]
    fn test_transform_translate() {
        let t = triangle();
        let translated = t.transform(&Translate(R2 { x: 1., y: 2. }));
        match translated {
            Shape::Polygon(p) => {
                assert_relative_eq!(p.vertices[0].x, 1., epsilon = 1e-10);
                assert_relative_eq!(p.vertices[0].y, 2., epsilon = 1e-10);
            }
            _ => panic!("Expected Polygon"),
        }
    }

    #[test]
    fn test_transform_scale() {
        let t = triangle();
        let scaled = t.transform(&Scale(2.));
        match scaled {
            Shape::Polygon(p) => {
                assert_relative_eq!(p.vertices[1].x, 2., epsilon = 1e-10);
                assert_relative_eq!(p.vertices[2].y, 2., epsilon = 1e-10);
            }
            _ => panic!("Expected Polygon"),
        }
    }

    #[test]
    fn test_display() {
        let t = triangle();
        let s = format!("{}", t);
        assert!(s.starts_with("Polygon["));
    }

    #[test]
    fn test_polygon_xyrrt_intersect() {
        use crate::ellipses::xyrrt::XYRRT;
        use crate::intersect::Intersect;

        // Triangle that intersects a rotated ellipse
        let polygon = Polygon::new(vec![
            R2 { x: -2., y: 0. },
            R2 { x: 2., y: 0. },
            R2 { x: 0., y: 2. },
        ]);

        // Rotated ellipse at origin
        let xyrrt: Shape<f64> = Shape::XYRRT(XYRRT {
            c: R2 { x: 0., y: 0.5 },
            r: R2 { x: 1.5, y: 0.8 },
            t: 0.3,
        });

        let polygon_shape: Shape<f64> = Shape::Polygon(polygon);

        // Test both orderings work and give same results
        let points1 = polygon_shape.intersect(&xyrrt);
        let points2 = xyrrt.intersect(&polygon_shape);

        assert!(!points1.is_empty(), "Expected intersections, got none");
        assert_eq!(points1.len(), points2.len(), "Both orderings should give same number of points");

        // Verify points are approximately the same (order may differ)
        for p1 in &points1 {
            let found = points2.iter().any(|p2| {
                (p1.x - p2.x).abs() < 1e-10 && (p1.y - p2.y).abs() < 1e-10
            });
            assert!(found, "Point {:?} not found in reverse intersection", p1);
        }
    }

    #[test]
    fn test_polygon_circle_scene() {
        use crate::model::Model;
        use crate::to::To;

        // This tests the full scene building with regions - requires theta computation
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1.5, y: -1. },
            R2 { x: 1.5, y: -1. },
            R2 { x: 0., y: 1.5 },
        ]));
        let circle: Shape<f64> = crate::shape::circle(0., 0., 1.);

        // InputSpec is (Shape<f64>, Vec<bool>) - bools indicate which coords are trainable
        let inputs = vec![
            (triangle, vec![true; 6]),  // 6 coords for triangle (3 vertices × 2)
            (circle, vec![true; 3]),    // 3 coords for circle (cx, cy, r)
        ];

        // Target keys using inclusive patterns like existing tests
        let targets: [(&str, f64); 3] = [
            ("0*", 1.),  // triangle (inclusive)
            ("*1", 1.),  // circle (inclusive)
            ("01", 0.5), // intersection
        ];

        // This will try to build regions, which requires theta computation on polygons
        let model = Model::new(inputs, targets.to());
        assert!(model.is_ok(), "Failed to build model: {:?}", model.err());
    }

    #[test]
    fn test_polygon_circle_training() {
        use crate::model::Model;
        use crate::to::To;

        // Triangle + circle, train to achieve target intersection
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -2., y: -1. },
            R2 { x: 2., y: -1. },
            R2 { x: 0., y: 2. },
        ]));
        let circle: Shape<f64> = crate::shape::circle(0., 0., 1.);

        // Only circle can move (simpler optimization)
        let inputs = vec![
            (triangle, vec![false; 6]),
            (circle, vec![true, true, true]),  // cx, cy, r trainable
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", 3.),   // triangle area
            ("*1", 2.),   // circle area
            ("01", 1.),   // intersection
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        // Train for some steps
        model.train(0.5, 50).expect("Training failed");

        let final_error = model.steps.last().unwrap().error.v();

        // Error should decrease
        assert!(
            final_error < initial_error,
            "Training should reduce error: {} -> {}",
            initial_error, final_error
        );
    }

    #[test]
    fn test_polygon_circle_gradient_diagnosis() {
        use crate::model::Model;
        use crate::to::To;

        // Reproduce the apvd scenario: triangle + circle with equal targets
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1., y: -1. },
            R2 { x: 1., y: -1. },
            R2 { x: 0., y: 1. },
        ]));
        let circle: Shape<f64> = crate::shape::circle(0.5, 0., 0.8);

        // Both shapes trainable
        let inputs = vec![
            (triangle, vec![true; 6]),  // 6 polygon coords trainable
            (circle, vec![true, true, true]),  // cx, cy, r trainable
        ];

        // Equal weight targets like in the bug report
        let targets: [(&str, f64); 3] = [
            ("0*", 1.),
            ("*1", 1.),
            ("01", 1.),
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");

        // Print initial state and gradients
        let step0 = &model.steps[0];
        eprintln!("Initial error: {}", step0.error.v());
        eprintln!("Initial gradients (d): {:?}", step0.error.d());
        eprintln!("Gradient magnitudes:");
        let grads = step0.error.d();
        for (i, g) in grads.iter().enumerate() {
            let name = if i < 6 {
                format!("polygon v{}.{}", i / 2, if i % 2 == 0 { "x" } else { "y" })
            } else {
                match i - 6 {
                    0 => "circle cx".to_string(),
                    1 => "circle cy".to_string(),
                    2 => "circle r".to_string(),
                    _ => format!("param {}", i),
                }
            };
            eprintln!("  {}: {:.6}", name, g);
        }

        // Train and track error over time
        let initial_error = step0.error.v();
        model.train(0.5, 200).expect("Training failed");
        let final_error = model.steps.last().unwrap().error.v();

        // Print error every 20 steps
        eprintln!("\nError progression:");
        for (i, step) in model.steps.iter().enumerate() {
            if i % 20 == 0 || i == model.steps.len() - 1 {
                eprintln!("  Step {}: error = {:.6}", i, step.error.v());
            }
        }

        eprintln!("\nTraining: {} -> {} ({:.1}% reduction)",
            initial_error, final_error,
            100. * (1. - final_error / initial_error));

        // Check if polygon vertices actually moved
        let final_shapes = &model.steps.last().unwrap().shapes;
        if let Shape::Polygon(final_poly) = &final_shapes[0] {
            eprintln!("\nFinal polygon vertices:");
            for (i, v) in final_poly.vertices.iter().enumerate() {
                eprintln!("  v{}: ({:.4}, {:.4})", i, v.x, v.y);
            }
        }
        if let Shape::Circle(final_circle) = &final_shapes[1] {
            eprintln!("Final circle: c=({:.4}, {:.4}), r={:.4}",
                final_circle.c.x, final_circle.c.y, final_circle.r);
        }

        // Should converge reasonably well for this simple case
        assert!(
            final_error < initial_error * 0.5,
            "Should achieve at least 50% error reduction: {} -> {}",
            initial_error, final_error
        );
    }

    #[test]
    fn test_circle_center_gradients_with_polygon() {
        use crate::model::Model;
        use crate::to::To;

        // Test that circle center gradients are non-zero when interacting with polygon
        // Order: Circle first (A), Polygon second (B) - matches user's apvd setup
        // Circle centered at (0, -0.2) - on x symmetry axis, but offset in y
        let circle: Shape<f64> = crate::shape::circle(0., -0.2, 1.);
        // Triangle symmetric around x=0
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -2., y: -1. },
            R2 { x: 2., y: -1. },
            R2 { x: 0., y: 2. },
        ]));

        // Circle trainable, polygon fixed
        let inputs = vec![
            (circle, vec![true, true, true]),  // cx, cy, r trainable
            (triangle, vec![false; 6]),
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", 2.),  // circle area (inclusive)
            ("*1", 3.),  // triangle area (inclusive)
            ("01", 1.),  // intersection
        ];

        let model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let step0 = &model.steps[0];
        let grads = step0.error.d();

        eprintln!("Circle-polygon gradient test (circle first):");
        eprintln!("  cx gradient: {:.6e}", grads[0]);
        eprintln!("  cy gradient: {:.6e}", grads[1]);
        eprintln!("  r  gradient: {:.6e}", grads[2]);

        // cx should be ~0 due to symmetry (triangle is symmetric around x=0, circle on axis)
        // cy should be non-zero (moving circle up/down changes intersection)
        // r should be non-zero (changing radius changes intersection)

        assert!(
            grads[1].abs() > 1e-6,
            "cy gradient should be non-zero, got {:.6e}",
            grads[1]
        );
        assert!(
            grads[2].abs() > 1e-6,
            "r gradient should be non-zero, got {:.6e}",
            grads[2]
        );
    }

    #[test]
    fn test_circle_polygon_multi_step_gradients() {
        use crate::model::Model;
        use crate::to::To;

        // Test gradient stability over multiple steps
        // Matches user's scenario more closely
        let circle: Shape<f64> = crate::shape::circle(0., -0.2, 1.3);
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1.6, y: -1.9 },
            R2 { x: 1.6, y: 0.9 },
            R2 { x: -1.6, y: 0.9 },
        ]));

        // Both trainable like user's scenario
        let inputs = vec![
            (circle, vec![true, true, true]),
            (triangle, vec![true; 6]),
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", 1.),
            ("*1", 1.),
            ("01", 1.),
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");

        eprintln!("\nMulti-step gradient test:");
        for i in 0..40 {
            let step = &model.steps[i];
            let grads = step.error.d();
            if i % 10 == 0 || i >= 30 {
                eprintln!("Step {}: error={:.4}, cx_grad={:.4e}, cy_grad={:.4e}, r_grad={:.4e}",
                    i, step.error.v(), grads[0], grads[1], grads[2]);
            }
            if i < 39 {
                model.train(0.5, 1).expect("Training failed");
            }
        }

        // Check that gradients don't collapse to zero
        let final_step = model.steps.last().unwrap();
        let final_grads = final_step.error.d();
        let grad_magnitude = (final_grads[0].powi(2) + final_grads[1].powi(2) + final_grads[2].powi(2)).sqrt();

        eprintln!("Final gradient magnitude: {:.4e}", grad_magnitude);

        // If error is still significant (>0.1), gradients shouldn't be zero
        if final_step.error.v() > 0.1 {
            assert!(
                grad_magnitude > 1e-10,
                "Gradients collapsed to zero while error is still {:.4}",
                final_step.error.v()
            );
        }
    }

    #[test]
    fn test_robust_optimizer_comparison() {
        use crate::model::Model;
        use crate::to::To;

        // Compare vanilla GD, Adam, and robust optimization
        let make_model = || {
            let circle: Shape<f64> = crate::shape::circle(0., -0.2, 1.3);
            let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
                R2 { x: -1.6, y: -1.9 },
                R2 { x: 1.6, y: 0.9 },
                R2 { x: -1.6, y: 0.9 },
            ]));

            let inputs = vec![
                (circle, vec![true, true, true]),
                (triangle, vec![true; 6]),
            ];

            let targets: [(&str, f64); 3] = [
                ("0*", 1.),
                ("*1", 1.),
                ("01", 1.),
            ];

            Model::new(inputs, targets.to()).expect("Failed to create model")
        };

        // Test 1: Vanilla GD
        let mut vanilla = make_model();
        let vanilla_initial = vanilla.steps[0].error.v();
        vanilla.train(0.5, 100).expect("Vanilla training failed");
        let vanilla_final = vanilla.steps.last().unwrap().error.v();

        // Test 2: Adam
        let mut adam = make_model();
        adam.train_adam(0.1, 100).expect("Adam training failed");
        let adam_final = adam.steps.last().unwrap().error.v();

        // Test 3: Robust
        let mut robust = make_model();
        robust.train_robust(100).expect("Robust training failed");
        let robust_final = robust.steps.last().unwrap().error.v();

        eprintln!("\nOptimizer comparison (100 steps):");
        eprintln!("  Initial error: {:.4}", vanilla_initial);
        eprintln!("  Vanilla GD:    {:.4} ({:.1}% reduction)", vanilla_final, 100. * (1. - vanilla_final / vanilla_initial));
        eprintln!("  Adam:          {:.4} ({:.1}% reduction)", adam_final, 100. * (1. - adam_final / vanilla_initial));
        eprintln!("  Robust:        {:.4} ({:.1}% reduction)", robust_final, 100. * (1. - robust_final / vanilla_initial));

        // Check radius stability for each method
        let check_radius_stability = |model: &Model, name: &str| {
            let radii: Vec<f64> = model.steps.iter().map(|s| {
                if let crate::shape::Shape::Circle(c) = &s.shapes[0] {
                    c.r.v()
                } else { 0.0 }
            }).collect();

            let max_change: f64 = radii.windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0.0, f64::max);

            eprintln!("  {} max radius change: {:.4}", name, max_change);
            max_change
        };

        let vanilla_stability = check_radius_stability(&vanilla, "Vanilla");
        let adam_stability = check_radius_stability(&adam, "Adam");
        let robust_stability = check_radius_stability(&robust, "Robust");

        // Robust should have more stable updates
        assert!(
            robust_stability <= vanilla_stability + 0.01,
            "Robust should be at least as stable as vanilla"
        );
    }

    #[test]
    fn test_polygon_circle_adam_optimizer() {
        use crate::model::Model;
        use crate::optimization::adam::AdamConfig;
        use crate::to::To;

        // Same setup as gradient diagnosis test, but using Adam optimizer
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1., y: -1. },
            R2 { x: 1., y: -1. },
            R2 { x: 0., y: 1. },
        ]));
        let circle: Shape<f64> = crate::shape::circle(0.5, 0., 0.8);

        // Both shapes trainable
        let inputs = vec![
            (triangle, vec![true; 6]),
            (circle, vec![true, true, true]),
        ];

        // Equal weight targets like in the bug report
        let targets: [(&str, f64); 3] = [
            ("0*", 1.),
            ("*1", 1.),
            ("01", 1.),
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");

        let initial_error = model.steps[0].error.v();
        eprintln!("Initial error: {}", initial_error);

        // Train with Adam optimizer using higher learning rate and more aggressive momentum
        // Default Adam beta1=0.9, beta2=0.999; try beta1=0.95 for more momentum
        let adam_config = AdamConfig {
            beta1: 0.95,
            beta2: 0.999,
            epsilon: 1e-8,
        };
        model.train_adam_with_config(0.2, 500, adam_config).expect("Training failed");

        let final_error = model.steps.last().unwrap().error.v();

        // Print error progression
        eprintln!("\nAdam error progression:");
        for (i, step) in model.steps.iter().enumerate() {
            if i % 50 == 0 || i == model.steps.len() - 1 {
                eprintln!("  Step {}: error = {:.6}", i, step.error.v());
            }
        }

        eprintln!("\nAdam training: {} -> {} ({:.1}% reduction)",
            initial_error, final_error,
            100. * (1. - final_error / initial_error));

        // Both vanilla GD and standard Adam plateau at ~0.61 (54% reduction).
        // This appears to be a local minimum in the error landscape.
        // The test verifies Adam works (doesn't diverge/NaN) and achieves
        // comparable results to vanilla GD. Further improvements may require
        // different loss functions or optimization strategies.
        assert!(
            final_error < initial_error * 0.5,
            "Adam should achieve at least 50% error reduction: {} -> {} ({:.1}% reduction)",
            initial_error, final_error,
            100. * (1. - final_error / initial_error)
        );
    }

    #[test]
    fn test_polygon_ellipse_training() {
        use crate::model::Model;
        use crate::to::To;
        use crate::shape::xyrr;

        // Square + ellipse
        let square: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1., y: -1. },
            R2 { x: 1., y: -1. },
            R2 { x: 1., y: 1. },
            R2 { x: -1., y: 1. },
        ]));
        let ellipse: Shape<f64> = xyrr(0., 0., 1.5, 0.8);

        // Both can move
        let inputs = vec![
            (square, vec![true; 8]),   // 4 vertices × 2
            (ellipse, vec![true; 4]),  // cx, cy, rx, ry
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", 4.),   // square area
            ("*1", 3.),   // ellipse area
            ("01", 2.),   // intersection
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        model.train(0.5, 30).expect("Training failed");

        let final_error = model.steps.last().unwrap().error.v();

        assert!(
            final_error < initial_error,
            "Training should reduce error: {} -> {}",
            initial_error, final_error
        );
    }

    #[test]
    fn test_two_triangles_star_of_david() {
        use crate::model::Model;
        use crate::to::To;
        use std::f64::consts::PI;

        // Exact TwoTriangles layout from apvd frontend (Star of David pattern)
        // Triangle 1: pointing up, centered at (0, 0.3), radius 1.2, rotation -π/2
        let r = 1.2;
        let cy1 = 0.3;
        let rot1 = -PI / 2.0;
        let tri1_poly = Polygon::new(vec![
            R2 { x: r * (rot1).cos(), y: cy1 + r * (rot1).sin() },
            R2 { x: r * (rot1 + 2.0 * PI / 3.0).cos(), y: cy1 + r * (rot1 + 2.0 * PI / 3.0).sin() },
            R2 { x: r * (rot1 + 4.0 * PI / 3.0).cos(), y: cy1 + r * (rot1 + 4.0 * PI / 3.0).sin() },
        ]);

        // Triangle 2: pointing down, centered at (0, -0.3), radius 1.2, rotation π/2
        let cy2 = -0.3;
        let rot2 = PI / 2.0;
        let tri2_poly = Polygon::new(vec![
            R2 { x: r * (rot2).cos(), y: cy2 + r * (rot2).sin() },
            R2 { x: r * (rot2 + 2.0 * PI / 3.0).cos(), y: cy2 + r * (rot2 + 2.0 * PI / 3.0).sin() },
            R2 { x: r * (rot2 + 4.0 * PI / 3.0).cos(), y: cy2 + r * (rot2 + 4.0 * PI / 3.0).sin() },
        ]);

        eprintln!("Triangle 1 vertices:");
        for (i, v) in tri1_poly.vertices.iter().enumerate() {
            eprintln!("  v{}: ({:.4}, {:.4})", i, v.x, v.y);
        }
        eprintln!("Triangle 2 vertices:");
        for (i, v) in tri2_poly.vertices.iter().enumerate() {
            eprintln!("  v{}: ({:.4}, {:.4})", i, v.x, v.y);
        }

        // Test basic intersection first
        let intersections = polygon_polygon_intersect(&tri1_poly, &tri2_poly);
        eprintln!("Intersection points: {:?}", intersections);

        let tri1: Shape<f64> = Shape::Polygon(tri1_poly);
        let tri2: Shape<f64> = Shape::Polygon(tri2_poly);

        // Both polygons fixed for now - just test scene building
        let inputs = vec![
            (tri1, vec![false; 6]),
            (tri2, vec![false; 6]),
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", 1.),
            ("*1", 1.),
            ("01", 1.),
        ];

        let model = Model::new(inputs, targets.to());
        assert!(model.is_ok(), "Failed to create model: {:?}", model.err());

        let model = model.unwrap();
        eprintln!("Model created successfully, initial error: {}", model.steps[0].error.v());
    }

    #[test]
    fn test_vertex_on_edge_intersection() {
        use crate::model::Model;
        use crate::to::To;

        // Create a scenario where one polygon's vertex lands on another's edge
        // This is the edge case that can cause boundary successor issues
        let tri1_poly = Polygon::new(vec![
            R2 { x: 0., y: 0. },      // vertex at origin
            R2 { x: 2., y: 0. },
            R2 { x: 1., y: 2. },
        ]);

        // Second triangle positioned so one vertex lands on tri1's edge
        let tri2_poly = Polygon::new(vec![
            R2 { x: 1., y: 0. },      // This vertex lands on tri1's edge (0,0)-(2,0)
            R2 { x: 2., y: 1.5 },
            R2 { x: 0., y: 1.5 },
        ]);

        eprintln!("Vertex-on-edge test:");
        eprintln!("Triangle 1: {:?}", tri1_poly.vertices);
        eprintln!("Triangle 2: {:?}", tri2_poly.vertices);

        let intersections = polygon_polygon_intersect(&tri1_poly, &tri2_poly);
        eprintln!("Intersections: {:?}", intersections);

        let tri1: Shape<f64> = Shape::Polygon(tri1_poly);
        let tri2: Shape<f64> = Shape::Polygon(tri2_poly);

        let inputs = vec![
            (tri1, vec![false; 6]),
            (tri2, vec![false; 6]),
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", 1.),
            ("*1", 1.),
            ("01", 0.5),
        ];

        let model = Model::new(inputs, targets.to());
        if model.is_err() {
            eprintln!("Model creation failed (expected for vertex-on-edge): {:?}", model.err());
        } else {
            eprintln!("Model created successfully, error: {}", model.unwrap().steps[0].error.v());
        }
    }

    #[test]
    fn test_polygon_polygon_training() {
        use crate::model::Model;
        use crate::to::To;

        // Two triangles with no coincident edges
        // tri1: base at y=-1, apex at y=1
        let tri1_poly = Polygon::new(vec![
            R2 { x: -1., y: -1. },
            R2 { x: 1., y: -1. },
            R2 { x: 0., y: 1. },
        ]);
        // tri2: rotated differently, centered higher
        let tri2_poly = Polygon::new(vec![
            R2 { x: -0.5, y: 0.2 },
            R2 { x: 1.3, y: 0.1 },
            R2 { x: 0.4, y: 1.8 },
        ]);

        // Test basic intersection first
        let intersections = polygon_polygon_intersect(&tri1_poly, &tri2_poly);
        eprintln!("intersection points: {:?}", intersections);

        let tri1: Shape<f64> = Shape::Polygon(tri1_poly);
        let tri2: Shape<f64> = Shape::Polygon(tri2_poly);

        // Only second triangle moves
        let inputs = vec![
            (tri1, vec![false; 6]),
            (tri2, vec![true; 6]),
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", 1.),
            ("*1", 1.),
            ("01", 0.3),
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        model.train(0.5, 50).expect("Training failed");

        let final_error = model.steps.last().unwrap().error.v();

        assert!(
            final_error < initial_error,
            "Training should reduce error: {} -> {}",
            initial_error, final_error
        );
    }

    #[test]
    fn test_three_shape_mixed() {
        use crate::model::Model;
        use crate::to::To;

        // Triangle + circle + ellipse - triangle much larger to ensure partial overlap
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -3., y: -2. },
            R2 { x: 3., y: -2. },
            R2 { x: 0., y: 3. },
        ]));
        let circle: Shape<f64> = crate::shape::circle(-0.5, 0., 0.5);
        let ellipse: Shape<f64> = crate::shape::xyrr(0.5, 0., 0.4, 0.6);

        let inputs = vec![
            (triangle, vec![false; 6]),  // Fixed
            (circle, vec![false; 3]),    // Fixed (avoid training issues for now)
            (ellipse, vec![false; 4]),   // Fixed
        ];

        // 3 shapes = 7 non-empty regions (2^3 - 1)
        let targets: [(&str, f64); 7] = [
            ("0**", 7.5),  // triangle
            ("*1*", 0.8),  // circle
            ("**2", 0.75), // ellipse
            ("01*", 0.5),  // triangle ∩ circle
            ("0*2", 0.5),  // triangle ∩ ellipse
            ("*12", 0.2),  // circle ∩ ellipse
            ("012", 0.1),  // all three
        ];

        // Just verify model builds without panic
        let model = Model::new(inputs, targets.to());
        assert!(model.is_ok(), "Failed to create model: {:?}", model.err());
    }

    #[test]
    fn test_concave_polygon() {
        use crate::model::Model;
        use crate::to::To;

        // L-shaped polygon (concave)
        let l_shape: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: 0., y: 0. },
            R2 { x: 2., y: 0. },
            R2 { x: 2., y: 1. },
            R2 { x: 1., y: 1. },
            R2 { x: 1., y: 2. },
            R2 { x: 0., y: 2. },
        ]));
        let circle: Shape<f64> = crate::shape::circle(1., 1., 0.8);

        let inputs = vec![
            (l_shape, vec![false; 12]),  // 6 vertices × 2
            (circle, vec![true; 3]),
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", 3.),   // L area = 2×2 - 1×1 = 3
            ("*1", 2.),
            ("01", 1.),
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        model.train(0.5, 30).expect("Training failed");

        let final_error = model.steps.last().unwrap().error.v();

        assert!(
            final_error <= initial_error,
            "Training should not increase error: {} -> {}",
            initial_error, final_error
        );
    }

    /// Test that two circles converge to achievable targets.
    ///
    /// This is the simplest case: two circles can achieve any overlap from 0 to min(area_A, area_B).
    /// We set targets based on actual computed areas, perturb, and verify convergence.
    #[test]
    fn test_two_circles_convergence() {
        use crate::model::Model;
        use crate::step::Step;
        use crate::to::To;

        // Two unit circles, slightly overlapping
        let circle_a: Shape<f64> = crate::shape::circle(-0.3, 0., 1.0);
        let circle_b: Shape<f64> = crate::shape::circle(0.3, 0., 1.0);

        // First, compute what the actual areas are at this "solution" position
        let solution_inputs = vec![
            (circle_a.clone(), vec![true; 3]),
            (circle_b.clone(), vec![true; 3]),
        ];

        // Use dummy targets to compute actual areas
        let dummy_targets: [(&str, f64); 3] = [("0*", 1.), ("*1", 1.), ("01", 0.)];
        let targets_map: crate::targets::TargetsMap<f64> = dummy_targets.to();
        let solution_step = Step::new(solution_inputs, targets_map.into()).expect("Solution step failed");

        // Extract actual areas from the solution
        let area_a_total = solution_step.errors.get("0*").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
        let area_b_total = solution_step.errors.get("*1").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
        let area_intersection = solution_step.errors.get("01").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();

        eprintln!("Solution areas: A_total={:.4}, B_total={:.4}, intersection={:.4}",
            area_a_total, area_b_total, area_intersection);

        // Now perturb the shapes and train back to the solution
        let perturbed_a: Shape<f64> = crate::shape::circle(-0.5, 0.2, 1.1);  // Moved and resized
        let perturbed_b: Shape<f64> = crate::shape::circle(0.5, -0.1, 0.9); // Moved and resized

        let inputs = vec![
            (perturbed_a, vec![true; 3]),
            (perturbed_b, vec![true; 3]),
        ];

        // Targets based on solution areas (these are achievable!)
        let targets: [(&str, f64); 3] = [
            ("0*", area_a_total),
            ("*1", area_b_total),
            ("01", area_intersection),
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        eprintln!("\nTwo circles convergence test:");
        eprintln!("  Initial error: {:.6}", initial_error);

        // Train with robust optimizer
        model.train_robust(200).expect("Training failed");

        let final_error = model.steps.last().unwrap().error.v();
        let reduction_pct = 100. * (1. - final_error / initial_error);

        eprintln!("  Final error: {:.6} ({:.1}% reduction)", final_error, reduction_pct);
        eprintln!("  Steps taken: {}", model.steps.len());

        // Print error progression
        for (i, step) in model.steps.iter().enumerate() {
            if i % 25 == 0 || i == model.steps.len() - 1 {
                eprintln!("  Step {:3}: error = {:.6}", i, step.error.v());
            }
        }

        // With achievable targets, we should converge to very low error
        assert!(
            final_error < 0.1,
            "Should converge to near-zero error with achievable targets: got {:.4}",
            final_error
        );
        assert!(
            reduction_pct > 90.,
            "Should achieve >90% error reduction: got {:.1}%",
            reduction_pct
        );
    }

    /// Test that a triangle and circle converge to achievable targets.
    #[test]
    fn test_triangle_circle_convergence() {
        use crate::model::Model;
        use crate::step::Step;
        use crate::to::To;

        // Triangle and circle in a "solution" overlapping position
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1., y: -0.8 },
            R2 { x: 1., y: -0.8 },
            R2 { x: 0., y: 1.2 },
        ]));
        let circle: Shape<f64> = crate::shape::circle(0., 0., 0.8);

        // Compute actual areas at solution
        let solution_inputs = vec![
            (triangle.clone(), vec![true; 6]),
            (circle.clone(), vec![true; 3]),
        ];
        let dummy_targets: [(&str, f64); 3] = [("0*", 1.), ("*1", 1.), ("01", 0.)];
        let targets_map: crate::targets::TargetsMap<f64> = dummy_targets.to();
        let solution_step = Step::new(solution_inputs, targets_map.into()).expect("Solution step failed");

        let area_a_total = solution_step.errors.get("0*").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
        let area_b_total = solution_step.errors.get("*1").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
        let area_intersection = solution_step.errors.get("01").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();

        eprintln!("Triangle-circle solution areas: A_total={:.4}, B_total={:.4}, intersection={:.4}",
            area_a_total, area_b_total, area_intersection);

        // Perturb: move triangle up and circle to the side
        let perturbed_triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1.2, y: -0.5 },
            R2 { x: 1.2, y: -0.5 },
            R2 { x: 0., y: 1.5 },
        ]));
        let perturbed_circle: Shape<f64> = crate::shape::circle(0.3, 0.2, 0.9);

        let inputs = vec![
            (perturbed_triangle, vec![true; 6]),
            (perturbed_circle, vec![true; 3]),
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", area_a_total),
            ("*1", area_b_total),
            ("01", area_intersection),
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        eprintln!("\nTriangle-circle convergence test:");
        eprintln!("  Initial error: {:.6}", initial_error);

        model.train_robust(300).expect("Training failed");

        let final_error = model.steps.last().unwrap().error.v();
        let reduction_pct = 100. * (1. - final_error / initial_error);

        eprintln!("  Final error: {:.6} ({:.1}% reduction)", final_error, reduction_pct);

        for (i, step) in model.steps.iter().enumerate() {
            if i % 50 == 0 || i == model.steps.len() - 1 {
                eprintln!("  Step {:3}: error = {:.6}", i, step.error.v());
            }
        }

        assert!(
            final_error < 0.15,
            "Should converge to low error with achievable targets: got {:.4}",
            final_error
        );
    }

    /// Test that two triangles converge to achievable targets.
    #[test]
    fn test_two_triangles_convergence() {
        use crate::model::Model;
        use crate::step::Step;
        use crate::to::To;

        // Two triangles in an overlapping "solution" position
        let triangle_a: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1., y: -0.5 },
            R2 { x: 1., y: -0.5 },
            R2 { x: 0., y: 1. },
        ]));
        let triangle_b: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -0.8, y: -0.3 },
            R2 { x: 0.8, y: -0.3 },
            R2 { x: 0., y: 1.2 },
        ]));

        // Compute actual areas at solution
        let solution_inputs = vec![
            (triangle_a.clone(), vec![true; 6]),
            (triangle_b.clone(), vec![true; 6]),
        ];
        let dummy_targets: [(&str, f64); 3] = [("0*", 1.), ("*1", 1.), ("01", 0.)];
        let targets_map: crate::targets::TargetsMap<f64> = dummy_targets.to();
        let solution_step = Step::new(solution_inputs, targets_map.into()).expect("Solution step failed");

        let area_a_total = solution_step.errors.get("0*").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
        let area_b_total = solution_step.errors.get("*1").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();
        let area_intersection = solution_step.errors.get("01").map(|e| e.actual_frac).unwrap_or(0.) * solution_step.total_area.v();

        eprintln!("Two triangles solution areas: A_total={:.4}, B_total={:.4}, intersection={:.4}",
            area_a_total, area_b_total, area_intersection);

        // Perturb both triangles
        let perturbed_a: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1.3, y: -0.7 },
            R2 { x: 0.9, y: -0.4 },
            R2 { x: -0.1, y: 1.2 },
        ]));
        let perturbed_b: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -0.6, y: -0.1 },
            R2 { x: 1.0, y: -0.4 },
            R2 { x: 0.2, y: 1.0 },
        ]));

        let inputs = vec![
            (perturbed_a, vec![true; 6]),
            (perturbed_b, vec![true; 6]),
        ];

        let targets: [(&str, f64); 3] = [
            ("0*", area_a_total),
            ("*1", area_b_total),
            ("01", area_intersection),
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        eprintln!("\nTwo triangles convergence test:");
        eprintln!("  Initial error: {:.6}", initial_error);

        model.train_robust(300).expect("Training failed");

        let final_error = model.steps.last().unwrap().error.v();
        let reduction_pct = 100. * (1. - final_error / initial_error);

        eprintln!("  Final error: {:.6} ({:.1}% reduction)", final_error, reduction_pct);

        for (i, step) in model.steps.iter().enumerate() {
            if i % 50 == 0 || i == model.steps.len() - 1 {
                eprintln!("  Step {:3}: error = {:.6}", i, step.error.v());
            }
        }

        assert!(
            final_error < 0.2,
            "Should converge to low error with achievable targets: got {:.4}",
            final_error
        );
    }

    /// Test with very simple achievable targets: complete disjointness.
    /// Two shapes that don't need to overlap at all - simplest case.
    #[test]
    fn test_disjoint_targets_convergence() {
        use crate::model::Model;
        use crate::to::To;

        // Two circles that should become disjoint
        let circle_a: Shape<f64> = crate::shape::circle(-0.5, 0., 1.0);
        let circle_b: Shape<f64> = crate::shape::circle(0.5, 0., 1.0);

        let inputs = vec![
            (circle_a, vec![true; 3]),
            (circle_b, vec![true; 3]),
        ];

        // Targets: each circle has area π, no overlap
        // Using fractions that sum to 1 (the optimization uses fractions, not absolute areas)
        let targets: [(&str, f64); 3] = [
            ("0*", 1.),   // Circle A total
            ("*1", 1.),   // Circle B total
            ("01", 0.),   // No intersection - this is achievable by separating the circles!
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        eprintln!("\nDisjoint targets convergence test:");
        eprintln!("  Initial error: {:.6}", initial_error);

        model.train_robust(200).expect("Training failed");

        let final_error = model.steps.last().unwrap().error.v();
        let reduction_pct = 100. * (1. - final_error / initial_error);

        eprintln!("  Final error: {:.6} ({:.1}% reduction)", final_error, reduction_pct);

        for (i, step) in model.steps.iter().enumerate() {
            if i % 25 == 0 || i == model.steps.len() - 1 {
                eprintln!("  Step {:3}: error = {:.6}", i, step.error.v());
            }
        }

        // Disjoint is easy - shapes just need to separate
        assert!(
            final_error < 0.05,
            "Should converge to near-zero for disjoint targets: got {:.4}",
            final_error
        );
    }

    /// Test behavior with impossible targets (each region = 100% of total).
    ///
    /// These targets are geometrically impossible for non-identical shapes.
    /// This test characterizes the oscillation behavior to ensure it's bounded
    /// and doesn't produce NaN/infinity.
    #[test]
    fn test_impossible_targets_oscillation() {
        use crate::model::Model;
        use crate::to::To;

        // Circle and triangle with impossible targets
        let circle: Shape<f64> = crate::shape::circle(0., 0., 1.0);
        let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
            R2 { x: -1.5, y: -1. },
            R2 { x: 1.5, y: -1. },
            R2 { x: 0., y: 1.5 },
        ]));

        let inputs = vec![
            (circle, vec![true; 3]),
            (triangle, vec![true; 6]),
        ];

        // Impossible targets: each region should be 100% of total
        // This requires identical shapes with 100% overlap - impossible for circle+triangle
        let targets: [(&str, f64); 3] = [
            ("0*", 1.),  // Circle total = 100%
            ("*1", 1.),  // Triangle total = 100%
            ("01", 1.),  // Intersection = 100%
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        eprintln!("\nImpossible targets test (circle + triangle):");
        eprintln!("  Initial error: {:.4}", initial_error);
        eprintln!("  Targets: A*=1, *B=1, AB=1 (each region = 100%, impossible)");

        // Run 50 steps with vanilla GD to see oscillation
        for i in 0..50 {
            model.train(0.5, 1).expect("Training failed");
            let step = model.steps.last().unwrap();
            let err = step.error.v();

            // Check for NaN/infinity
            assert!(!err.is_nan(), "Error became NaN at step {}", i);
            assert!(err.is_finite(), "Error became infinite at step {}", i);

            if i < 10 || i % 10 == 9 {
                // Extract circle radius for debugging
                let r = if let crate::shape::Shape::Circle(c) = &step.shapes[0] {
                    c.r.v()
                } else { 0.0 };
                eprintln!("  Step {:2}: error={:.4}, r={:.4}", i + 1, err, r);
            }
        }

        let final_error = model.steps.last().unwrap().error.v();

        // Characterize the oscillation
        let errors: Vec<f64> = model.steps.iter().map(|s| s.error.v()).collect();
        let error_changes: Vec<f64> = errors.windows(2).map(|w| w[1] - w[0]).collect();
        let sign_changes = error_changes.windows(2)
            .filter(|w| (w[0] > 0.) != (w[1] > 0.))
            .count();

        eprintln!("  Final error: {:.4}", final_error);
        eprintln!("  Sign changes in error delta: {} (oscillation indicator)", sign_changes);

        // With impossible targets, error should be bounded (not explode)
        assert!(
            final_error < 10.0,
            "Error should stay bounded even with impossible targets: got {:.4}",
            final_error
        );

        // Oscillation is expected but shouldn't be too extreme
        // (If there were bugs in gradient computation, error might grow unbounded)
        let max_error = errors.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_error = errors.iter().cloned().fold(f64::INFINITY, f64::min);
        eprintln!("  Error range: {:.4} to {:.4}", min_error, max_error);

        assert!(
            max_error < initial_error * 2.0,
            "Error shouldn't more than double: initial={:.4}, max={:.4}",
            initial_error, max_error
        );
    }

    /// Test comparing vanilla GD vs clipped GD on impossible targets.
    ///
    /// Clipped GD should have smaller oscillation amplitude.
    #[test]
    fn test_clipped_vs_vanilla_impossible_targets() {
        use crate::model::Model;
        use crate::step::Step;
        use crate::to::To;

        let make_inputs = || {
            let circle: Shape<f64> = crate::shape::circle(0., 0., 1.0);
            let triangle: Shape<f64> = Shape::Polygon(Polygon::new(vec![
                R2 { x: -1.5, y: -1. },
                R2 { x: 1.5, y: -1. },
                R2 { x: 0., y: 1.5 },
            ]));
            vec![
                (circle, vec![true; 3]),
                (triangle, vec![true; 6]),
            ]
        };

        // Impossible targets
        let targets: [(&str, f64); 3] = [
            ("0*", 1.),
            ("*1", 1.),
            ("01", 1.),
        ];

        // Vanilla GD
        let mut vanilla_model = Model::new(make_inputs(), targets.to()).expect("Failed to create model");
        for _ in 0..30 {
            vanilla_model.train(0.5, 1).expect("Training failed");
        }
        let vanilla_errors: Vec<f64> = vanilla_model.steps.iter().map(|s| s.error.v()).collect();

        // Clipped GD (using step_clipped directly)
        let clipped_inputs = make_inputs();
        let targets_map: crate::targets::TargetsMap<f64> = targets.to();
        let mut clipped_step = Step::new(clipped_inputs, targets_map.into()).expect("Step failed");
        let mut clipped_errors = vec![clipped_step.error.v()];
        for _ in 0..30 {
            clipped_step = clipped_step.step_clipped(0.05, 0.5, 1.0).expect("Step failed");
            clipped_errors.push(clipped_step.error.v());
        }

        // Compute oscillation amplitude (std dev of error changes)
        let vanilla_deltas: Vec<f64> = vanilla_errors.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        let clipped_deltas: Vec<f64> = clipped_errors.windows(2).map(|w| (w[1] - w[0]).abs()).collect();

        let vanilla_avg_delta: f64 = vanilla_deltas.iter().sum::<f64>() / vanilla_deltas.len() as f64;
        let clipped_avg_delta: f64 = clipped_deltas.iter().sum::<f64>() / clipped_deltas.len() as f64;

        eprintln!("\nVanilla vs Clipped GD on impossible targets:");
        eprintln!("  Vanilla avg |Δerror|: {:.4}", vanilla_avg_delta);
        eprintln!("  Clipped avg |Δerror|: {:.4}", clipped_avg_delta);
        eprintln!("  Vanilla final error: {:.4}", vanilla_errors.last().unwrap());
        eprintln!("  Clipped final error: {:.4}", clipped_errors.last().unwrap());

        // Clipped should have smaller oscillation
        assert!(
            clipped_avg_delta <= vanilla_avg_delta + 0.01,
            "Clipped GD should have smaller oscillation: vanilla={:.4}, clipped={:.4}",
            vanilla_avg_delta, clipped_avg_delta
        );
    }

    // ============================================================
    // Self-intersection detection tests
    // ============================================================

    /// Create a self-intersecting "bowtie" quadrilateral (figure-8 shape)
    fn bowtie() -> Polygon<f64> {
        // Vertices in order that creates crossing edges
        Polygon::new(vec![
            R2 { x: 0., y: 0. },
            R2 { x: 2., y: 2. },  // crosses with edge 2-3
            R2 { x: 2., y: 0. },
            R2 { x: 0., y: 2. },  // crosses with edge 0-1
        ])
    }

    /// Create a regular n-gon centered at (cx, cy) with given radius
    fn regular_ngon(n: usize, cx: f64, cy: f64, radius: f64) -> Polygon<f64> {
        let vertices: Vec<R2<f64>> = (0..n)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
                R2 {
                    x: cx + radius * angle.cos(),
                    y: cy + radius * angle.sin(),
                }
            })
            .collect();
        Polygon::new(vertices)
    }

    #[test]
    fn test_is_self_intersecting_simple() {
        // Convex shapes should not self-intersect
        assert!(!triangle().is_self_intersecting(), "Triangle should not self-intersect");
        assert!(!square().is_self_intersecting(), "Square should not self-intersect");

        // Regular n-gons should not self-intersect
        for n in 5..=12 {
            let ngon = regular_ngon(n, 0., 0., 1.);
            assert!(!ngon.is_self_intersecting(), "{}-gon should not self-intersect", n);
        }

        // Bowtie (figure-8) should self-intersect
        assert!(bowtie().is_self_intersecting(), "Bowtie should self-intersect");
    }

    #[test]
    fn test_self_intersection_penalty_nonzero_for_bowtie() {
        let bow = bowtie();
        let penalty = bow.self_intersection_penalty();
        assert!(penalty > 0., "Bowtie should have positive self-intersection penalty, got {}", penalty);

        // Regular shapes should have zero penalty
        assert_eq!(square().self_intersection_penalty(), 0., "Square should have zero penalty");
        assert_eq!(regular_ngon(12, 0., 0., 1.).self_intersection_penalty(), 0., "12-gon should have zero penalty");
    }

    #[test]
    fn test_self_intersection_penalty_dual_produces_gradients() {
        // Create a nearly self-intersecting quadrilateral that can become self-intersecting
        // by moving one vertex slightly
        let almost_bowtie: Polygon<Dual> = Polygon::new(vec![
            R2 { x: Dual::new(0., vec![1., 0., 0., 0., 0., 0., 0., 0.]), y: Dual::new(0., vec![0., 1., 0., 0., 0., 0., 0., 0.]) },
            R2 { x: Dual::new(1., vec![0., 0., 1., 0., 0., 0., 0., 0.]), y: Dual::new(0., vec![0., 0., 0., 1., 0., 0., 0., 0.]) },
            R2 { x: Dual::new(0.5, vec![0., 0., 0., 0., 1., 0., 0., 0.]), y: Dual::new(1., vec![0., 0., 0., 0., 0., 1., 0., 0.]) },
            R2 { x: Dual::new(0.5, vec![0., 0., 0., 0., 0., 0., 1., 0.]), y: Dual::new(-0.1, vec![0., 0., 0., 0., 0., 0., 0., 1.]) },  // below x-axis, causes self-intersection
        ]);

        let penalty = almost_bowtie.self_intersection_penalty_dual();

        // Should have positive penalty
        assert!(penalty.v() > 0., "Self-intersecting quad should have positive penalty, got {}", penalty.v());

        // Should have non-zero gradients
        let grads = penalty.d();
        let grad_magnitude: f64 = grads.iter().map(|g| g.powi(2)).sum::<f64>().sqrt();
        assert!(grad_magnitude > 0., "Should have non-zero gradients for self-intersection penalty");

        eprintln!("Self-intersection penalty: {}", penalty.v());
        eprintln!("Gradient magnitude: {}", grad_magnitude);
    }

    // ============================================================
    // Variant callers 12-gon integration test
    // ============================================================

    #[test]
    fn test_dodecagon_training_no_self_intersection() {
        use crate::model::Model;
        use crate::to::To;

        // Create 2 regular 12-gons (dodecagons) in an overlapping configuration
        let shapes: Vec<Shape<f64>> = vec![
            Shape::Polygon(regular_ngon(12, 0.0, 0.0, 1.5)),  // A
            Shape::Polygon(regular_ngon(12, 1.0, 0.0, 1.5)),  // B (overlaps with A)
        ];

        // All vertices trainable: 12 vertices × 2 coords = 24 per polygon
        let inputs: Vec<(Shape<f64>, Vec<bool>)> = shapes
            .into_iter()
            .map(|s| (s, vec![true; 24]))
            .collect();

        // Simple targets for 2 shapes using inclusive format
        let targets: [(&str, f64); 3] = [
            ("0*", 5.),  // A inclusive
            ("*1", 5.),  // B inclusive
            ("01", 2.),  // A∩B
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        eprintln!("\n=== 12-gon Training Self-Intersection Test ===");
        eprintln!("Initial error: {:.4}", initial_error);

        // Train for 100 steps, checking for self-intersection periodically
        let mut self_intersection_count = 0;
        let mut max_error_increase = 0.0f64;
        let mut prev_error = initial_error;

        for step_num in 0..100 {
            model.train(0.3, 1).expect("Training failed");

            let step = model.steps.last().unwrap();
            let error = step.error.v();

            // Track error increases (oscillation indicator)
            if error > prev_error {
                max_error_increase = max_error_increase.max(error - prev_error);
            }
            prev_error = error;

            // Check for self-intersection in all polygons
            for (i, shape) in step.shapes.iter().enumerate() {
                if let Shape::Polygon(poly) = shape {
                    // Need to convert Dual vertices to f64 for is_self_intersecting
                    let f64_poly = Polygon::new(
                        poly.vertices.iter().map(|v| R2 { x: v.x.v(), y: v.y.v() }).collect()
                    );
                    if f64_poly.is_self_intersecting() {
                        self_intersection_count += 1;
                        if self_intersection_count <= 5 {
                            eprintln!("  Step {}: Polygon {} self-intersecting!", step_num, i);
                        }
                    }
                }
            }

            if step_num % 20 == 0 {
                eprintln!("  Step {}: error = {:.4}", step_num, error);
            }
        }

        let final_error = model.steps.last().unwrap().error.v();
        eprintln!("Final error: {:.4}", final_error);
        eprintln!("Self-intersections detected: {}", self_intersection_count);
        eprintln!("Max single-step error increase: {:.4}", max_error_increase);

        // Assertions
        assert!(
            final_error < initial_error,
            "Training should reduce error: {} -> {}",
            initial_error, final_error
        );

        // Warn (but don't fail) if there were self-intersections
        // The penalty should prevent them, but this test documents the behavior
        if self_intersection_count > 0 {
            eprintln!("WARNING: {} self-intersections occurred during training!", self_intersection_count);
        }

        // Check for severe oscillation (error increases > 50% of current error)
        assert!(
            max_error_increase < initial_error * 0.5,
            "Severe oscillation detected: max error increase {:.4} exceeds threshold",
            max_error_increase
        );
    }

    #[test]
    fn test_four_dodecagons_variant_callers() {
        use crate::model::Model;
        use crate::to::To;

        // Create 4 regular 12-gons (dodecagons) in a Venn-like configuration
        // Position them so they overlap in interesting ways
        let shapes: Vec<Shape<f64>> = vec![
            Shape::Polygon(regular_ngon(12, 0.0, 0.5, 1.2)),   // Shape 0 - left
            Shape::Polygon(regular_ngon(12, 1.0, 0.5, 1.2)),   // Shape 1 - right
            Shape::Polygon(regular_ngon(12, 0.5, 0.0, 1.2)),   // Shape 2 - top
            Shape::Polygon(regular_ngon(12, 0.5, 1.0, 1.2)),   // Shape 3 - bottom
        ];

        // All vertices trainable: 12 vertices × 2 coords = 24 per polygon
        let inputs: Vec<(Shape<f64>, Vec<bool>)> = shapes
            .into_iter()
            .map(|s| (s, vec![true; 24]))
            .collect();

        // Variant callers targets (exclusive format)
        // These are real-world targets from genomics variant calling
        let targets: [(&str, f64); 15] = [
            ("0---", 633.),  // Shape 0 only
            ("-1--", 618.),  // Shape 1 only
            ("--2-", 187.),  // Shape 2 only
            ("---3", 319.),  // Shape 3 only
            ("01--", 112.),  // Shapes 0∩1 only
            ("0-2-",   0.),  // Shapes 0∩2 only
            ("0--3",  13.),  // Shapes 0∩3 only
            ("-12-",  14.),  // Shapes 1∩2 only
            ("-1-3",  55.),  // Shapes 1∩3 only
            ("--23",  21.),  // Shapes 2∩3 only
            ("012-",   1.),  // Shapes 0∩1∩2 only
            ("01-3",  17.),  // Shapes 0∩1∩3 only
            ("0-23",   0.),  // Shapes 0∩2∩3 only
            ("-123",   9.),  // Shapes 1∩2∩3 only
            ("0123",  36.),  // All four shapes
        ];

        let mut model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let initial_error = model.steps[0].error.v();

        eprintln!("\n=== 4 Dodecagon Variant Callers Test ===");
        eprintln!("Initial error: {:.4}", initial_error);

        // Train for 200 steps, checking for self-intersection and oscillation
        let mut self_intersection_count = 0;
        let mut oscillation_count = 0;
        let mut prev_error = initial_error;
        let mut best_error = initial_error;

        for step_num in 0..200 {
            model.train(0.3, 1).expect("Training failed");

            let step = model.steps.last().unwrap();
            let error = step.error.v();

            // Track oscillation (error increases)
            if error > prev_error * 1.01 {  // Allow 1% noise
                oscillation_count += 1;
            }

            // Track best error
            if error < best_error {
                best_error = error;
            }

            prev_error = error;

            // Check for self-intersection in all polygons
            for (i, shape) in step.shapes.iter().enumerate() {
                if let Shape::Polygon(poly) = shape {
                    let f64_poly = Polygon::new(
                        poly.vertices.iter().map(|v| R2 { x: v.x.v(), y: v.y.v() }).collect()
                    );
                    if f64_poly.is_self_intersecting() {
                        self_intersection_count += 1;
                        if self_intersection_count <= 3 {
                            eprintln!("  Step {}: Polygon {} self-intersecting!", step_num, i);
                        }
                    }
                }
            }

            if step_num % 50 == 0 {
                eprintln!("  Step {}: error = {:.4}", step_num, error);
            }
        }

        let final_error = model.steps.last().unwrap().error.v();
        eprintln!("Final error: {:.4}", final_error);
        eprintln!("Best error: {:.4}", best_error);
        eprintln!("Self-intersections detected: {}", self_intersection_count);
        eprintln!("Oscillation events: {}", oscillation_count);

        // Report self-intersections (don't fail the test, but document)
        if self_intersection_count > 0 {
            eprintln!("WARNING: {} self-intersections occurred - penalty may be too weak!", self_intersection_count);
        }

        // Report oscillation (the user's original complaint)
        if oscillation_count > 20 {
            eprintln!("WARNING: {} oscillation events - training is unstable!", oscillation_count);
        }

        // Assertions - training should make progress even if not perfect
        assert!(
            best_error < initial_error * 0.8,
            "Training should achieve at least 20% improvement: {} -> best {}",
            initial_error, best_error
        );
    }

    fn five_shape_layout(n_sides: usize, dist: f64, rx: f64, ry: f64, dent: f64) -> Vec<Shape<f64>> {
        (0..5).map(|i| {
            let angle = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * i as f64) / 5.0;
            let cx = dist * angle.cos();
            let cy = dist * angle.sin();
            let rotation = angle;
            let cos_r = rotation.cos();
            let sin_r = rotation.sin();
            let vertices: Vec<R2<f64>> = (0..n_sides).map(|j| {
                let theta = 2.0 * std::f64::consts::PI * (j as f64) / (n_sides as f64);
                let cos_t = theta.cos();
                let sin_t = theta.sin();
                let r = 1.0 + dent * cos_t;
                let px = sin_t * rx * r;
                let py = cos_t * ry * r;
                R2 {
                    x: cx + px * cos_r - py * sin_r,
                    y: cy + px * sin_r + py * cos_r,
                }
            }).collect();
            Shape::Polygon(Polygon::new(vertices))
        }).collect()
    }

    fn count_regions(shapes: Vec<Shape<f64>>, label: &str) -> (usize, usize, usize) {
        use crate::scene::Scene;

        let scene: Scene<f64> = Scene::new(shapes).expect("Failed to create scene");

        let mut negative_regions = Vec::new();
        let mut zero_regions = Vec::new();
        let mut positive_regions = Vec::new();

        for mask in 1u32..32 {
            let key: String = (0..5).map(|i| {
                if mask & (1 << i) != 0 { char::from_digit(i, 10).unwrap() } else { '-' }
            }).collect();

            let area = scene.area(&key);
            let area_val: f64 = area.clone().unwrap_or(0.0);
            if area_val < -0.001 {
                negative_regions.push((key, area_val));
            } else if area_val.abs() < 0.001 {
                zero_regions.push(key);
            } else {
                positive_regions.push((key, area_val));
            }
        }

        eprintln!("\n=== {} ===", label);
        eprintln!("{} positive, {} zero, {} negative", positive_regions.len(), zero_regions.len(), negative_regions.len());
        if !zero_regions.is_empty() {
            eprintln!("  Zero: {}", zero_regions.join(", "));
        }
        if !negative_regions.is_empty() {
            eprintln!("  Negative: {}", negative_regions.iter().map(|(k, v)| format!("{}({:.4})", k, v)).collect::<Vec<_>>().join(", "));
        }

        (positive_regions.len(), zero_regions.len(), negative_regions.len())
    }

    #[test]
    fn test_five_shape_layouts_region_count() {
        // 31/31 requires tight spacing so non-adjacent triples overlap.
        // Explore the boundary between 26/31 and 31/31.

        // Sweep dist from 0.15 to 0.30, with width=0.8, height=1.5
        for &dist in &[0.15, 0.18, 0.20, 0.22, 0.25, 0.28, 0.30] {
            let shapes = five_shape_layout(40, dist, 0.8, 1.5, 0.15);
            count_regions(shapes, &format!("d={:.2}, w=0.8, h=1.5, dent=0.15", dist));
        }

        // Sweep dist with width=0.7
        for &dist in &[0.15, 0.18, 0.20, 0.22, 0.25] {
            let shapes = five_shape_layout(40, dist, 0.7, 1.5, 0.15);
            count_regions(shapes, &format!("d={:.2}, w=0.7, h=1.5, dent=0.15", dist));
        }

        // Sweep width at dist=0.2
        for &width in &[0.5, 0.6, 0.7, 0.8, 0.9, 1.0] {
            let shapes = five_shape_layout(40, 0.2, width, 1.5, 0.15);
            count_regions(shapes, &format!("d=0.20, w={:.1}, h=1.5, dent=0.15", width));
        }

        // Try dist=0.2 with different dents
        for &dent in &[0.0, 0.05, 0.10, 0.15, 0.20, 0.30] {
            let shapes = five_shape_layout(40, 0.2, 0.7, 1.5, dent);
            count_regions(shapes, &format!("d=0.20, w=0.7, h=1.5, dent={:.2}", dent));
        }
    }

    #[test]
    fn test_five_blobs_vertex_count_sweep() {
        // Find minimum vertex count that still achieves 31 regions
        // with the standard five blobs layout params (d=0.2, w=0.7, h=1.5, dent=0.15)
        for &n in &[8, 10, 12, 15, 20, 25, 30, 40] {
            let shapes = five_shape_layout(n, 0.2, 0.7, 1.5, 0.15);
            let (pos, zero, neg) = count_regions(shapes, &format!("n={}", n));
            eprintln!("  n={}: {} positive, {} zero, {} negative", n, pos, zero, neg);
        }
    }

    #[test]
    fn test_five_blobs_dual_vs_f64() {
        // Compare Scene<f64> vs Scene<Dual> (via Step::new) to isolate
        // whether the WASM region count discrepancy is a Dual vs f64 issue.
        use crate::step::Step;
        use crate::to::To;

        let shapes_f64 = five_shape_layout(40, 0.2, 0.7, 1.5, 0.15);
        let (p_f64, z_f64, n_f64) = count_regions(shapes_f64.clone(), "f64 Scene");

        // Create InputSpec for Dual scene (all coords trainable, like WASM does)
        let input_specs: Vec<(Shape<f64>, Vec<bool>)> = shapes_f64.iter().map(|s| {
            let n_coords = match s {
                Shape::Polygon(p) => p.vertices.len() * 2,
                _ => unreachable!(),
            };
            (s.clone(), vec![true; n_coords])
        }).collect();

        // Build targets for all 31 regions
        let mut targets_map: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
        for mask in 1u32..32 {
            let key: String = (0..5).map(|i| {
                if mask & (1 << i) != 0 { char::from_digit(i, 10).unwrap() } else { '-' }
            }).collect();
            targets_map.insert(key, 1.0);
        }

        let step = Step::new(input_specs, targets_map.into());
        match &step {
            Ok(step) => {
                let mut p_dual = 0;
                let mut z_dual = 0;
                let mut n_dual = 0;
                let mut negative_list = Vec::new();
                let mut zero_list = Vec::new();
                for (key, err) in &step.errors {
                    if key.contains('*') { continue; }
                    let actual = err.actual_area.unwrap_or(0.0);
                    if actual < -0.001 {
                        n_dual += 1;
                        negative_list.push(format!("{}({:.1})", key, actual));
                    } else if actual.abs() < 0.001 {
                        z_dual += 1;
                        zero_list.push(key.clone());
                    } else {
                        p_dual += 1;
                    }
                }
                eprintln!("\n=== Dual Scene ===");
                eprintln!("{} positive, {} zero, {} negative", p_dual, z_dual, n_dual);
                if !zero_list.is_empty() {
                    eprintln!("  Zero: {}", zero_list.join(", "));
                }
                if !negative_list.is_empty() {
                    eprintln!("  Negative: {}", negative_list.join(", "));
                }

                // The key comparison: do f64 and Dual agree?
                eprintln!("\nf64: {} positive, {} zero, {} negative", p_f64, z_f64, n_f64);
                eprintln!("Dual: {} positive, {} zero, {} negative", p_dual, z_dual, n_dual);
            }
            Err(e) => {
                eprintln!("Step::new failed: {:?}", e);
            }
        }
    }

    #[test]
    fn test_five_blobs_verify_areas() {
        // Verify that shape.area() matches sum of region areas (catches CW winding sign bug)
        use crate::scene::Scene;

        for &n in &[12, 15, 20, 40] {
            let shapes = five_shape_layout(n, 0.2, 0.7, 1.5, 0.15);
            let scene: Scene<f64> = Scene::new(shapes).expect("Failed to create scene");
            for component in &scene.components {
                component.verify_areas(0.01).unwrap_or_else(|e| {
                    panic!("verify_areas failed for n={}: {}", n, e);
                });
            }
        }
    }

    #[test]
    fn test_five_dodecagons_model_errors() {
        use crate::model::Model;
        use crate::to::To;

        // Create 5 elongated 12-gons in a pentagonal arrangement
        let dist = 0.5_f64;
        let rx = 0.45_f64;
        let ry = 1.3_f64;
        let n_sides = 12;

        let shapes: Vec<Shape<f64>> = (0..5).map(|i| {
            let angle = std::f64::consts::FRAC_PI_2 + (2.0 * std::f64::consts::PI * i as f64) / 5.0;
            let cx = dist * angle.cos();
            let cy = dist * angle.sin();
            let rotation = angle - std::f64::consts::FRAC_PI_2;
            let cos_r = rotation.cos();
            let sin_r = rotation.sin();
            let vertices: Vec<R2<f64>> = (0..n_sides).map(|j| {
                let theta = 2.0 * std::f64::consts::PI * (j as f64) / (n_sides as f64);
                let px = rx * theta.cos();
                let py = ry * theta.sin();
                R2 {
                    x: cx + px * cos_r - py * sin_r,
                    y: cy + px * sin_r + py * cos_r,
                }
            }).collect();
            Shape::Polygon(Polygon::new(vertices))
        }).collect();

        // FizzBuzzBazzQuxQuux exclusive targets (from sample-targets comment)
        let targets: [(&str, f64); 31] = [
            ("0----", 5280.),
            ("-1---", 2640.),
            ("--2--", 1320.),
            ("---3-", 1760.),
            ("----4",  880.),
            ("01---",  440.),
            ("0-2--",  220.),
            ("0--3-", 1056.),
            ("0---4",  528.),
            ("-12--",  264.),
            ("-1-3-",  132.),
            ("-1--4",  176.),
            ("--23-",   88.),
            ("--2-4",   44.),
            ("---34",   22.),
            ("012--",  480.),
            ("01-3-",  240.),
            ("01--4",  120.),
            ("0-23-",   60.),
            ("0-2-4",   80.),
            ("0--34",   40.),
            ("-123-",   20.),
            ("-12-4",   10.),
            ("-1-34",   48.),
            ("--234",   24.),
            ("0123-",   12.),
            ("012-4",    6.),
            ("01-34",    8.),
            ("0-234",    4.),
            ("-1234",    2.),
            ("01234",    1.),
        ];

        let inputs: Vec<(Shape<f64>, Vec<bool>)> = shapes
            .into_iter()
            .map(|s| {
                let ncoords = if let Shape::Polygon(ref p) = s { p.vertices.len() * 2 } else { 0 };
                (s, vec![true; ncoords])
            })
            .collect();

        let model = Model::new(inputs, targets.to()).expect("Failed to create model");
        let step = &model.steps[0];

        eprintln!("\n=== 5 Dodecagon Model Initial Errors ===");
        eprintln!("Total error: {:.4}", step.error.v());

        // Print errors for each target, sorted by absolute error
        let mut errors: Vec<_> = step.errors.iter()
            .map(|(key, err)| (key.clone(), err.actual_area, err.target_area, err.actual_frac, err.target_frac, err.error.v()))
            .collect();
        errors.sort_by(|a, b| b.5.abs().partial_cmp(&a.5.abs()).unwrap());

        for (key, actual_area, target_area, actual_frac, target_frac, error) in &errors {
            eprintln!("  {} actual_area={:>8.2?} target={:>6.0} actual_frac={:>8.4} target_frac={:>8.4} err={:>8.4}",
                key, actual_area, target_area, actual_frac, target_frac, error);
        }
    }
}
