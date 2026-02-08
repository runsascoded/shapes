mod intersection;
mod penalties;
mod perimeter;
mod transforms;

pub use intersection::*;
pub use transforms::*;

use std::{
    fmt::Display,
    ops::{Add, Div, Neg, Sub},
};

use derive_more::From;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{
    coord_getter::{coord_getter, CoordGetter},
    dual::{Dual, D},
    r2::R2,
    shape::{AreaArg, Duals},
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

impl<D: Zero> Polygon<D> {
    pub fn zero(&self) -> D {
        Zero::zero(&self.vertices[0].x)
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

#[cfg(test)]
mod tests;
