use std::{fmt::Display, rc::Rc, cell::RefCell, collections::BTreeSet, ops::{Add, Mul, Div, Sub}, f64::consts::TAU};

use crate::{math::deg::Deg, node::N, set::S, shape::Shape::{Circle, XYRR, XYRRT, Polygon}, trig::Trig, dual::Dual, zero::Zero};

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
