use std::{fmt::Display, rc::Rc, cell::RefCell, collections::BTreeSet, ops::{Mul, Div, Sub}, f64::consts::TAU};


use crate::{math::deg::Deg, node::N, set::S, shape::Shape::{Circle, XYRR, XYRRT, Polygon}, trig::Trig, dual::Dual, zero::Zero};

pub type E<D> = Rc<RefCell<Edge<D>>>;

#[derive(Debug, Clone)]
pub struct Edge<D> {
    pub idx: usize,
    pub set: S<D>,
    pub node0: N<D>,
    pub node1: N<D>,
    pub theta0: D,
    pub theta1: D,
    pub container_set_idxs: BTreeSet<usize>,
    pub is_component_boundary: bool,
    pub visits: usize,
}

pub trait EdgeArg
: Clone
+ Display
+ Into<f64>
+ Trig
+ Sub<Output = Self>
+ Mul<Output = Self>
+ Div<f64, Output = Self>
{}

impl EdgeArg for f64 {}
impl EdgeArg for Dual {}

impl<D> Edge<D> {
    pub fn set_idx(&self) -> usize {
        self.set.borrow().idx
    }
}
impl<D: EdgeArg + Zero> Edge<D> {
    pub fn secant_area(&self) -> D {
        match &self.set.borrow().shape {
            Circle(c) => {
                let r2 = c.clone().r * c.clone().r;
                let theta = self.theta();
                r2 / 2. * (theta.clone() - theta.sin())
            },
            XYRR(e) => {
                let r2 = e.r.clone().x * e.clone().r.y;
                let theta = self.theta();
                r2 / 2. * (theta.clone() - theta.sin())
            },
            XYRRT(e) => {
                let r2 = e.r.clone().x * e.clone().r.y;
                let theta = self.theta();
                r2 / 2. * (theta.clone() - theta.sin())
            },
            Polygon(p) => {
                // Polygon edges are straight lines, no secant area
                p.zero()
            },
        }
    }
    /// Angle span of this Edge, in terms of the shape whose border it is part of
    pub fn theta(&self) -> D {
        let theta = self.theta1.clone() - self.theta0.clone();
        if theta.clone().into() < 0. {
            panic!("Invalid edge {}, negative theta: {}", self, theta)
        }
        theta
    }
    /// Return all shape indices that either contain this Edge, or which this Edge runs along the border of
    pub fn all_idxs(&self) -> BTreeSet<usize> {
        let mut idxs = self.container_set_idxs.clone();
        idxs.insert(self.set.borrow().idx);
        idxs
    }
}

impl<D: Clone + Into<f64>> Edge<D> {
    pub fn contains_theta(&self, theta: f64) -> bool {
        let theta0: f64 = self.theta0.clone().into();
        let theta1: f64 = self.theta1.clone().into();
        let theta = if theta < theta0 { theta + TAU } else { theta };
        theta0 <= theta && theta <= theta1
    }
}

impl<D> Edge<D> {
    pub fn expected_visits(&self) -> usize {
        if self.is_component_boundary { 1 } else { 2 }
    }
}

impl<
    D
    : Clone
    + Display
    + Into<f64>
> Display for Edge<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let containers: Vec<String> = self.container_set_idxs.iter().map(|idx| format!("{}", idx)).collect();
        write!(
            f,
            "C{}: {}({}) → {}({}), containers: [{}] ({})",
            self.set.borrow().idx,
            self.node0.borrow().idx, self.theta0.clone().into().deg_str(),
            self.node1.borrow().idx, self.theta1.clone().into().deg_str(),
            containers.join(","),
            if self.is_component_boundary { "external" } else { "internal" },
        )
    }
}