use std::ops::{Add, Mul};

use crate::{r2::R2, shape::Shape, transform::{HasProjection, CanProject}, dual::Dual};


pub trait Contains<O> {
    fn contains(&self, o: &O) -> bool;
}

/// Helper trait for containment checks with f64 points, avoiding trait bound issues.
pub trait ContainsF64 {
    fn contains_f64(&self, p: &R2<f64>) -> bool;
}

impl<D: Clone + Into<f64>> ContainsF64 for Shape<D> {
    fn contains_f64(&self, p: &R2<f64>) -> bool {
        // Polygons: ray-casting containment check
        if let Shape::Polygon(polygon) = self {
            let vertices_f64: Vec<R2<f64>> = polygon.vertices.iter()
                .map(|v| R2 { x: v.x.clone().into(), y: v.y.clone().into() })
                .collect();
            let n = vertices_f64.len();
            let mut crossings = 0;
            for i in 0..n {
                let v0 = &vertices_f64[i];
                let v1 = &vertices_f64[(i + 1) % n];
                let (y_min, y_max) = if v0.y < v1.y { (v0.y, v1.y) } else { (v1.y, v0.y) };
                if p.y < y_min || p.y >= y_max {
                    continue;
                }
                let t = (p.y - v0.y) / (v1.y - v0.y);
                let x_crossing = v0.x + t * (v1.x - v0.x);
                if x_crossing > p.x {
                    crossings += 1;
                }
            }
            return crossings % 2 == 1;
        }

        // For circles/ellipses, check if point is inside using direct computation
        match self {
            Shape::Circle(c) => {
                let cx: f64 = c.c.x.clone().into();
                let cy: f64 = c.c.y.clone().into();
                let r: f64 = c.r.clone().into();
                let dx = p.x - cx;
                let dy = p.y - cy;
                dx * dx + dy * dy <= r * r
            }
            Shape::XYRR(e) => {
                let cx: f64 = e.c.x.clone().into();
                let cy: f64 = e.c.y.clone().into();
                let rx: f64 = e.r.x.clone().into();
                let ry: f64 = e.r.y.clone().into();
                let dx = (p.x - cx) / rx;
                let dy = (p.y - cy) / ry;
                dx * dx + dy * dy <= 1.0
            }
            Shape::XYRRT(e) => {
                let cx: f64 = e.c.x.clone().into();
                let cy: f64 = e.c.y.clone().into();
                let rx: f64 = e.r.x.clone().into();
                let ry: f64 = e.r.y.clone().into();
                let theta: f64 = e.t.clone().into();
                let cos_t = theta.cos();
                let sin_t = theta.sin();
                // Translate to center
                let dx = p.x - cx;
                let dy = p.y - cy;
                // Rotate by -theta
                let rotated_x = dx * cos_t + dy * sin_t;
                let rotated_y = -dx * sin_t + dy * cos_t;
                // Scale to unit circle
                let scaled_x = rotated_x / rx;
                let scaled_y = rotated_y / ry;
                scaled_x * scaled_x + scaled_y * scaled_y <= 1.0
            }
            Shape::Polygon(_) => unreachable!(), // Handled above
        }
    }
}

pub trait ShapeContainsPoint
: Clone
+ Into<f64>
+ Add<Output = Self>
+ Mul<Output = Self>
{}
impl ShapeContainsPoint for f64 {}
impl ShapeContainsPoint for Dual {}

impl<D: ShapeContainsPoint> Contains<R2<D>> for Shape<D>
where
    R2<D>: CanProject<D, Output = R2<D>>,
    Shape<D>: HasProjection<D>,
{
    fn contains(&self, p: &R2<D>) -> bool {
        // Polygons need ray-casting containment check; projection-based approach doesn't work
        if let Shape::Polygon(polygon) = self {
            // Convert point to f64 for ray casting
            let p_f64 = R2 { x: p.x.clone().into(), y: p.y.clone().into() };
            let vertices_f64: Vec<R2<f64>> = polygon.vertices.iter()
                .map(|v| R2 { x: v.x.clone().into(), y: v.y.clone().into() })
                .collect();
            // Ray casting algorithm
            let n = vertices_f64.len();
            let mut crossings = 0;
            for i in 0..n {
                let v0 = &vertices_f64[i];
                let v1 = &vertices_f64[(i + 1) % n];
                let (y_min, y_max) = if v0.y < v1.y { (v0.y, v1.y) } else { (v1.y, v0.y) };
                if p_f64.y < y_min || p_f64.y >= y_max {
                    continue;
                }
                let t = (p_f64.y - v0.y) / (v1.y - v0.y);
                let x_crossing = v0.x + t * (v1.x - v0.x);
                if x_crossing > p_f64.x {
                    crossings += 1;
                }
            }
            return crossings % 2 == 1;
        }

        // For circles/ellipses, use projection-based containment
        p.apply(&self.projection()).norm2().into() <= 1.
    }
}
