use std::ops::{Add, Div, Mul, Sub};

use crate::{
    math::recip::Recip,
    r2::R2,
    transform::{
        Projection,
        Transform::Translate,
    },
    zero::Zero,
};

use std::fmt::Display;
use std::ops::Neg;

use super::Polygon;

impl<D: Clone + Display + Recip + Add<Output = D> + Div<f64, Output = D>> Polygon<D>
where
    R2<D>: Neg<Output = R2<D>>,
{
    /// For polygons, "projection" is just translation to centroid (no scaling/rotation).
    /// Used by the scene analysis pipeline to normalize shapes before intersection.
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
        let mut best: Option<(f64, f64)> = None; // (dist_sq, param)

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
                let param = (i as f64) + t.clamp(0.0, 1.0);
                if best.is_none() || dist_sq < best.unwrap().0 {
                    best = Some((dist_sq, param));
                }
            }
        }

        if let Some((_, param)) = best {
            return param;
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

impl<D> Polygon<D>
where
    D: Clone + Into<f64> + Zero + Add<Output = D> + Sub<Output = D> + Mul<Output = D> + Div<f64, Output = D>,
{
    /// Compute the "secant area" for a polygon edge from p0 to p1.
    /// This is the signed area between the chord (p0→p1) and the polygon boundary path.
    /// The polygon boundary may include intermediate vertices between the two endpoints.
    ///
    /// coord0 and coord1 are perimeter_param values (edge_idx + t) of the endpoints.
    /// Vertices with integer perimeter_param strictly in (coord0, coord1) are intermediate.
    pub fn secant_area(&self, p0: &R2<D>, p1: &R2<D>, coord0: f64, coord1: f64) -> D {
        let n = self.vertices.len();

        // Find vertices with integer perimeter_param strictly between coord0 and coord1.
        // Vertex i has perimeter_param = i (integer). Handle wrap-around: coord1 may be > n.
        let first_idx = coord0.ceil() as usize;
        let last_idx = coord1.floor() as usize;

        let mut intermediate: Vec<usize> = Vec::new();
        for vi in first_idx..=last_idx {
            let vf = vi as f64;
            if vf > coord0 + 1e-9 && vf < coord1 - 1e-9 {
                intermediate.push(vi % n);
            }
        }

        if intermediate.is_empty() {
            return self.zero();
        }

        // Compute shoelace area of the closed polygon: p0 → v1 → ... → vk → p1 → p0.
        // This gives the signed area between the chord (p0→p1) and the polygon boundary path.
        let mut area = self.zero();

        // p0 → first intermediate vertex
        let first_v = &self.vertices[intermediate[0]];
        area = area + (p0.x.clone() * first_v.y.clone() - first_v.x.clone() * p0.y.clone());

        // Consecutive intermediate vertices
        for i in 0..intermediate.len() - 1 {
            let vi = &self.vertices[intermediate[i]];
            let vn = &self.vertices[intermediate[i + 1]];
            area = area + (vi.x.clone() * vn.y.clone() - vn.x.clone() * vi.y.clone());
        }

        // Last intermediate vertex → p1
        let last_v = &self.vertices[*intermediate.last().unwrap()];
        area = area + (last_v.x.clone() * p1.y.clone() - p1.x.clone() * last_v.y.clone());

        // Close: p1 → p0
        area = area + (p1.x.clone() * p0.y.clone() - p0.x.clone() * p1.y.clone());

        area / 2.
    }
}
