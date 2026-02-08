use crate::{
    dual::{Dual, D},
    r2::R2,
    shape::Duals,
};

use super::Polygon;

impl Polygon<f64> {
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
