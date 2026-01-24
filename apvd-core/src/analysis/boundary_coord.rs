use std::f64::consts::TAU;

use crate::{r2::R2, shape::Shape};

/// Trait for computing boundary coordinates on shapes.
///
/// A boundary coordinate is a monotonic parameter that orders points around a shape's perimeter.
/// For circles/ellipses, this is the angle θ ∈ [0, 2π).
/// For polygons, this is edge_idx + t where edge_idx is the edge number and t ∈ [0, 1).
///
/// Key invariant: `coord(point(c)) ≈ c` (round-trip consistency)
///
/// These computations use f64 regardless of whether the shape uses Dual numbers,
/// since boundary coordinates are only used for ordering and containment testing,
/// not for gradient computation.
pub trait BoundaryCoord {
    /// Get the boundary coordinate for a point on the shape's perimeter.
    fn coord(&self, p: &R2<f64>) -> f64;

    /// Get the point on the perimeter at the given boundary coordinate.
    fn point(&self, c: f64) -> R2<f64>;

    /// Get a point on the perimeter between two boundary coordinates.
    /// Used for containment testing - doesn't need to be the exact midpoint.
    fn midpoint(&self, c0: f64, c1: f64) -> R2<f64>;

    /// Get the period of the boundary coordinate (for wrap-around).
    /// TAU for circles/ellipses, n_vertices for polygons.
    fn coord_period(&self) -> f64;
}

impl<D: Clone + Into<f64>> BoundaryCoord for Shape<D> {
    fn coord(&self, p: &R2<f64>) -> f64 {
        match self {
            Shape::Polygon(polygon) => polygon.perimeter_param(p),
            Shape::Circle(c) => {
                // Transform point to circle-local coords and get angle
                let cx: f64 = c.c.x.clone().into();
                let cy: f64 = c.c.y.clone().into();
                (p.y - cy).atan2(p.x - cx)
            }
            Shape::XYRR(e) => {
                // Transform point to ellipse-local coords (scaled to unit circle) and get angle
                let cx: f64 = e.c.x.clone().into();
                let cy: f64 = e.c.y.clone().into();
                let rx: f64 = e.r.x.clone().into();
                let ry: f64 = e.r.y.clone().into();
                let local_x = (p.x - cx) / rx;
                let local_y = (p.y - cy) / ry;
                local_y.atan2(local_x)
            }
            Shape::XYRRT(e) => {
                // Transform point to rotated ellipse-local coords and get angle
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
                let local_x = rotated_x / rx;
                let local_y = rotated_y / ry;
                local_y.atan2(local_x)
            }
        }
    }

    fn point(&self, c: f64) -> R2<f64> {
        match self {
            Shape::Polygon(polygon) => polygon.perimeter_point(c),
            Shape::Circle(circle) => {
                let cx: f64 = circle.c.x.clone().into();
                let cy: f64 = circle.c.y.clone().into();
                let r: f64 = circle.r.clone().into();
                R2 {
                    x: cx + r * c.cos(),
                    y: cy + r * c.sin(),
                }
            }
            Shape::XYRR(e) => {
                let cx: f64 = e.c.x.clone().into();
                let cy: f64 = e.c.y.clone().into();
                let rx: f64 = e.r.x.clone().into();
                let ry: f64 = e.r.y.clone().into();
                R2 {
                    x: cx + rx * c.cos(),
                    y: cy + ry * c.sin(),
                }
            }
            Shape::XYRRT(e) => {
                let cx: f64 = e.c.x.clone().into();
                let cy: f64 = e.c.y.clone().into();
                let rx: f64 = e.r.x.clone().into();
                let ry: f64 = e.r.y.clone().into();
                let theta: f64 = e.t.clone().into();
                let cos_t = theta.cos();
                let sin_t = theta.sin();
                // Point on unit circle
                let ux = c.cos();
                let uy = c.sin();
                // Scale by radii
                let sx = rx * ux;
                let sy = ry * uy;
                // Rotate by theta
                let rotated_x = sx * cos_t - sy * sin_t;
                let rotated_y = sx * sin_t + sy * cos_t;
                // Translate to center
                R2 {
                    x: cx + rotated_x,
                    y: cy + rotated_y,
                }
            }
        }
    }

    fn midpoint(&self, c0: f64, c1: f64) -> R2<f64> {
        match self {
            Shape::Polygon(polygon) => polygon.perimeter_midpoint(c0, c1),
            _ => {
                // For circles/ellipses, get midpoint of arc
                let c1 = if c1 < c0 { c1 + TAU } else { c1 };
                let c = (c0 + c1) / 2.;
                self.point(c)
            }
        }
    }

    fn coord_period(&self) -> f64 {
        match self {
            Shape::Polygon(polygon) => polygon.vertices.len() as f64,
            _ => TAU,
        }
    }
}
