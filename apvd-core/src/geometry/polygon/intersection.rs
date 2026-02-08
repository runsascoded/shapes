use std::{
    fmt::{self, Display},
    ops::{Add, Div, Mul, Neg, Sub},
};

use log::debug;

use crate::{
    dual::Dual,
    math::is_normal::IsNormal,
    r2::R2,
    sqrt::Sqrt,
};

use super::Polygon;

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
