use std::{fmt::Display, rc::Rc, cell::RefCell, collections::BTreeSet, ops::{Add, Mul, Div, Sub}, f64::consts::TAU};

use crate::{boundary_coord::BoundaryCoord, math::deg::Deg, math::sqrt::Sqrt, node::N, r2::R2, set::S, shape::Shape::{Circle, XYRR, XYRRT, Polygon}, trig::Trig, dual::Dual, zero::Zero};

pub type E<D> = Rc<RefCell<Edge<D>>>;

#[derive(Debug, Clone)]
pub struct Edge<D> {
    pub idx: usize,
    pub set: S<D>,
    pub node0: N<D>,
    pub node1: N<D>,
    /// Boundary coordinate of start point (f64 since coords don't need gradients)
    pub coord0: f64,
    /// Boundary coordinate of end point (f64 since coords don't need gradients)
    pub coord1: f64,
    pub container_set_idxs: BTreeSet<usize>,
    pub is_component_boundary: bool,
    pub visits: usize,
}

/// Trait for computing secant areas (requires Trig for sin())
pub trait EdgeArg
: Clone
+ Display
+ Into<f64>
+ Trig
+ Add<Output = Self>
+ Add<f64, Output = Self>
+ Sub<Output = Self>
+ Mul<Output = Self>
+ Mul<f64, Output = Self>
+ Div<Output = Self>
+ Div<f64, Output = Self>
{}

impl EdgeArg for f64 {}
impl EdgeArg for Dual {}

impl<D> Edge<D> {
    pub fn set_idx(&self) -> usize {
        self.set.borrow().idx
    }

    /// Boundary coordinate span of this edge
    pub fn coord_span(&self) -> f64 {
        let span = self.coord1 - self.coord0;
        if span < 0. {
            panic!("Invalid edge {}, negative coord span: {}", self.idx, span)
        }
        span
    }

    /// Deprecated: use coord_span() instead
    pub fn theta(&self) -> f64 {
        self.coord_span()
    }
}

impl<D: EdgeArg + Zero> Edge<D> {
    /// Compute the angle span using node positions (preserves gradients for autodiff).
    /// Returns the arc angle from node0 to node1 on the shape.
    fn angle_span(&self) -> D {
        let p0 = self.node0.borrow().p.clone();
        let p1 = self.node1.borrow().p.clone();

        match &self.set.borrow().shape {
            Circle(c) => {
                // Angle from center to each node
                let dx0 = p0.x.clone() - c.c.x.clone();
                let dy0 = p0.y.clone() - c.c.y.clone();
                let dx1 = p1.x.clone() - c.c.x.clone();
                let dy1 = p1.y.clone() - c.c.y.clone();
                let theta0 = dy0.atan2(&dx0);
                let theta1 = dy1.atan2(&dx1);
                // Compute raw span and make it positive
                let span = theta1.clone() - theta0.clone();
                let span = if span.clone().into() < 0. { span + TAU } else { span };
                // Now span is in [0, 2π]. Compare to coord_span to pick correct direction.
                let span_val: f64 = span.clone().into();
                let complement_val = TAU - span_val;
                if (span_val - self.coord_span()).abs() < (complement_val - self.coord_span()).abs() {
                    span
                } else {
                    // Return complement: TAU - span = -span + TAU
                    span * -1. + TAU
                }
            }
            XYRR(e) => {
                // For axis-aligned ellipse, use scaled coordinates to get angle
                let scaled_p0_x = (p0.x.clone() - e.c.x.clone()) / e.r.x.clone();
                let scaled_p0_y = (p0.y.clone() - e.c.y.clone()) / e.r.y.clone();
                let scaled_p1_x = (p1.x.clone() - e.c.x.clone()) / e.r.x.clone();
                let scaled_p1_y = (p1.y.clone() - e.c.y.clone()) / e.r.y.clone();
                let theta0 = scaled_p0_y.atan2(&scaled_p0_x);
                let theta1 = scaled_p1_y.atan2(&scaled_p1_x);
                // Compute raw span and make it positive
                let span = theta1.clone() - theta0.clone();
                let span = if span.clone().into() < 0. { span + TAU } else { span };
                // Now span is in [0, 2π]. Compare to coord_span to pick correct direction.
                let span_val: f64 = span.clone().into();
                let complement_val = TAU - span_val;
                if (span_val - self.coord_span()).abs() < (complement_val - self.coord_span()).abs() {
                    span
                } else {
                    span * -1. + TAU
                }
            }
            XYRRT(e) => {
                // For rotated ellipse, rotate to local coords then scale
                let cos_t = e.t.clone().cos();
                let sin_t = e.t.clone().sin();

                let dx0 = p0.x.clone() - e.c.x.clone();
                let dy0 = p0.y.clone() - e.c.y.clone();
                let local0_x = dx0.clone() * cos_t.clone() + dy0.clone() * sin_t.clone();
                let local0_y = dy0.clone() * cos_t.clone() - dx0.clone() * sin_t.clone();

                let dx1 = p1.x.clone() - e.c.x.clone();
                let dy1 = p1.y.clone() - e.c.y.clone();
                let local1_x = dx1.clone() * cos_t.clone() + dy1.clone() * sin_t.clone();
                let local1_y = dy1.clone() * cos_t.clone() - dx1.clone() * sin_t.clone();

                let scaled0_x = local0_x / e.r.x.clone();
                let scaled0_y = local0_y / e.r.y.clone();
                let scaled1_x = local1_x / e.r.x.clone();
                let scaled1_y = local1_y / e.r.y.clone();

                let theta0 = scaled0_y.atan2(&scaled0_x);
                let theta1 = scaled1_y.atan2(&scaled1_x);
                // Compute raw span and make it positive
                let span = theta1.clone() - theta0.clone();
                let span = if span.clone().into() < 0. { span + TAU } else { span };
                // Now span is in [0, 2π]. Compare to coord_span to pick correct direction.
                let span_val: f64 = span.clone().into();
                let complement_val = TAU - span_val;
                if (span_val - self.coord_span()).abs() < (complement_val - self.coord_span()).abs() {
                    span
                } else {
                    span * -1. + TAU
                }
            }
            Polygon(polygon) => {
                // Polygons don't use angle-based secant area, return zero
                polygon.vertices[0].x.clone().zero()
            }
        }
    }

    pub fn secant_area(&self) -> D {
        match &self.set.borrow().shape {
            Circle(c) => {
                let r2 = c.clone().r * c.clone().r;
                let theta = self.angle_span();
                r2 / 2. * (theta.clone() - theta.sin())
            },
            XYRR(e) => {
                let r2 = e.r.clone().x * e.clone().r.y;
                let theta = self.angle_span();
                r2 / 2. * (theta.clone() - theta.sin())
            },
            XYRRT(e) => {
                let r2 = e.r.clone().x * e.clone().r.y;
                let theta = self.angle_span();
                r2 / 2. * (theta.clone() - theta.sin())
            },
            Polygon(polygon) => {
                // Polygon edges are straight lines, but the boundary path between two
                // intersection points may pass through intermediate polygon vertices.
                // secant_area computes the signed area between the chord and that path.
                let p0 = self.node0.borrow().p.clone();
                let p1 = self.node1.borrow().p.clone();
                polygon.secant_area(&p0, &p1, self.coord0, self.coord1)
            },
        }
    }

    /// Compute the boundary length of this edge (f64, no gradients).
    /// For circles: exact arc length = r * theta.
    /// For ellipses: numerical approximation via sampled points.
    /// For polygons: sum of vertex-to-vertex distances along the boundary path.
    pub fn boundary_length(&self) -> f64 {
        let set = self.set.borrow();
        match &set.shape {
            Circle(c) => {
                let r: f64 = c.r.clone().into();
                r * self.coord_span()
            }
            Polygon(polygon) => {
                let n0 = self.node0.borrow();
                let p0 = R2 { x: n0.p.x.clone().into(), y: n0.p.y.clone().into() };
                drop(n0);
                let n1 = self.node1.borrow();
                let p1 = R2 { x: n1.p.x.clone().into(), y: n1.p.y.clone().into() };
                drop(n1);
                let n = polygon.vertices.len();
                let first_idx = self.coord0.ceil() as usize;
                let last_idx = self.coord1.floor() as usize;

                let mut intermediate: Vec<usize> = Vec::new();
                for vi in first_idx..=last_idx {
                    let vf = vi as f64;
                    if vf > self.coord0 + 1e-9 && vf < self.coord1 - 1e-9 {
                        intermediate.push(vi % n);
                    }
                }

                if intermediate.is_empty() {
                    let dx = p1.x - p0.x;
                    let dy = p1.y - p0.y;
                    return (dx * dx + dy * dy).sqrt();
                }

                let vf = |idx: usize| -> R2<f64> {
                    let v = &polygon.vertices[idx];
                    R2 { x: v.x.clone().into(), y: v.y.clone().into() }
                };
                let mut length = 0.0;
                let first_v = vf(intermediate[0]);
                length += ((first_v.x - p0.x).powi(2) + (first_v.y - p0.y).powi(2)).sqrt();
                for i in 0..intermediate.len() - 1 {
                    let vi = vf(intermediate[i]);
                    let vn = vf(intermediate[i + 1]);
                    length += ((vn.x - vi.x).powi(2) + (vn.y - vi.y).powi(2)).sqrt();
                }
                let last_v = vf(*intermediate.last().unwrap());
                length += ((p1.x - last_v.x).powi(2) + (p1.y - last_v.y).powi(2)).sqrt();
                length
            }
            _ => {
                // Ellipses: numerical approximation via sampled points along the arc
                let n_samples = 32;
                let n0 = self.node0.borrow();
                let p0 = R2 { x: n0.p.x.clone().into(), y: n0.p.y.clone().into() };
                drop(n0);
                let mut length = 0.0;
                let mut prev = p0;
                let span = self.coord_span();
                for i in 1..=n_samples {
                    let t = i as f64 / n_samples as f64;
                    let coord = self.coord0 + t * span;
                    let pt = set.shape.point(coord);
                    let dx = pt.x - prev.x;
                    let dy = pt.y - prev.y;
                    length += (dx * dx + dy * dy).sqrt();
                    prev = pt;
                }
                length
            }
        }
    }

    /// Maximum angular step (radians) for numerical arc-length sampling on ellipses.
    /// Smaller = more accurate but slower. PI/16 ≈ 11° gives sub-percent accuracy.
    const MAX_ARC_DELTA_THETA: f64 = std::f64::consts::PI / 16.0;

    /// Compute the boundary length of this edge as a Dual (preserves gradients).
    /// For circles: exact arc length = r * angle_span (both Dual).
    /// For polygons: sum of vertex-to-vertex distances along the boundary path (Dual coords).
    /// For ellipses: numerical approximation via adaptive sampling (Dual coords).
    pub fn boundary_length_dual(&self) -> D where D: Sqrt {
        let set = self.set.borrow();
        match &set.shape {
            Circle(c) => {
                c.r.clone() * self.angle_span()
            }
            Polygon(polygon) => {
                let p0 = self.node0.borrow().p.clone();
                let p1 = self.node1.borrow().p.clone();
                let n = polygon.vertices.len();
                let first_idx = self.coord0.ceil() as usize;
                let last_idx = self.coord1.floor() as usize;

                let mut intermediate: Vec<usize> = Vec::new();
                for vi in first_idx..=last_idx {
                    let vf = vi as f64;
                    if vf > self.coord0 + 1e-9 && vf < self.coord1 - 1e-9 {
                        intermediate.push(vi % n);
                    }
                }

                let dist = |a: &R2<D>, b: &R2<D>| -> D {
                    let dx = b.x.clone() - a.x.clone();
                    let dy = b.y.clone() - a.y.clone();
                    (dx.clone() * dx + dy.clone() * dy).sqrt()
                };

                if intermediate.is_empty() {
                    return dist(&p0, &p1);
                }

                let mut length = dist(&p0, &polygon.vertices[intermediate[0]]);
                for i in 0..intermediate.len() - 1 {
                    length = length + dist(
                        &polygon.vertices[intermediate[i]],
                        &polygon.vertices[intermediate[i + 1]],
                    );
                }
                length + dist(&polygon.vertices[*intermediate.last().unwrap()], &p1)
            }
            XYRR(e) => {
                // Numerical arc length: sample points along the ellipse arc using D-typed coords.
                // Use node0/node1 positions as exact endpoints; subdivide interior adaptively.
                let span = self.coord_span();
                let n_segments = (span / Self::MAX_ARC_DELTA_THETA).ceil().max(1.0) as usize;
                let p0 = self.node0.borrow().p.clone();
                let mut length = p0.x.clone().zero();
                let mut prev = p0;
                for i in 1..n_segments {
                    let t = i as f64 / n_segments as f64;
                    let theta = self.coord0 + t * span;
                    let pt = R2 {
                        x: e.c.x.clone() + e.r.x.clone() * theta.cos(),
                        y: e.c.y.clone() + e.r.y.clone() * theta.sin(),
                    };
                    let dx = pt.x.clone() - prev.x.clone();
                    let dy = pt.y.clone() - prev.y.clone();
                    length = length + (dx.clone() * dx + dy.clone() * dy).sqrt();
                    prev = pt;
                }
                // Final segment to node1
                let p1 = self.node1.borrow().p.clone();
                let dx = p1.x.clone() - prev.x.clone();
                let dy = p1.y.clone() - prev.y.clone();
                length + (dx.clone() * dx + dy.clone() * dy).sqrt()
            }
            XYRRT(e) => {
                // Same as XYRR but with rotation applied
                let span = self.coord_span();
                let n_segments = (span / Self::MAX_ARC_DELTA_THETA).ceil().max(1.0) as usize;
                let cos_rot = e.t.clone().cos();
                let sin_rot = e.t.clone().sin();
                let p0 = self.node0.borrow().p.clone();
                let mut length = p0.x.clone().zero();
                let mut prev = p0;
                for i in 1..n_segments {
                    let t = i as f64 / n_segments as f64;
                    let theta = self.coord0 + t * span;
                    let local_x = e.r.x.clone() * theta.cos();
                    let local_y = e.r.y.clone() * theta.sin();
                    let pt = R2 {
                        x: e.c.x.clone() + local_x.clone() * cos_rot.clone() - local_y.clone() * sin_rot.clone(),
                        y: e.c.y.clone() + local_x * sin_rot.clone() + local_y * cos_rot.clone(),
                    };
                    let dx = pt.x.clone() - prev.x.clone();
                    let dy = pt.y.clone() - prev.y.clone();
                    length = length + (dx.clone() * dx + dy.clone() * dy).sqrt();
                    prev = pt;
                }
                // Final segment to node1
                let p1 = self.node1.borrow().p.clone();
                let dx = p1.x.clone() - prev.x.clone();
                let dy = p1.y.clone() - prev.y.clone();
                length + (dx.clone() * dx + dy.clone() * dy).sqrt()
            }
        }
    }

    /// Return all shape indices that either contain this Edge, or which this Edge runs along the border of
    pub fn all_idxs(&self) -> BTreeSet<usize> {
        let mut idxs = self.container_set_idxs.clone();
        idxs.insert(self.set.borrow().idx);
        idxs
    }
}

impl<D> Edge<D> {
    pub fn contains_coord(&self, coord: f64) -> bool {
        let coord = if coord < self.coord0 { coord + TAU } else { coord };
        self.coord0 <= coord && coord <= self.coord1
    }

    /// Deprecated: use contains_coord() instead
    pub fn contains_theta(&self, theta: f64) -> bool {
        self.contains_coord(theta)
    }

    pub fn expected_visits(&self) -> usize {
        if self.is_component_boundary { 1 } else { 2 }
    }
}

impl<D: Clone + Display + Into<f64>> Display for Edge<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let containers: Vec<String> = self.container_set_idxs.iter().map(|idx| format!("{}", idx)).collect();
        write!(
            f,
            "C{}: {}({}) → {}({}), containers: [{}] ({})",
            self.set.borrow().idx,
            self.node0.borrow().idx, self.coord0.deg_str(),
            self.node1.borrow().idx, self.coord1.deg_str(),
            containers.join(","),
            if self.is_component_boundary { "external" } else { "internal" },
        )
    }
}
