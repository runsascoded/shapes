use std::{fmt::Display, rc::Rc, cell::RefCell, collections::BTreeSet, ops::{Mul, Div, Sub}};

use crate::{math::deg::Deg, node::N, shape::{S, Shape}, trig::Trig, dual::Dual};

pub type E<D> = Rc<RefCell<Edge<D>>>;

#[derive(Debug, Clone)]
pub struct Edge<D> {
    pub idx: usize,
    pub c: S<D>,
    pub n0: N<D>,
    pub n1: N<D>,
    pub t0: D,
    pub t1: D,
    pub container_idxs: BTreeSet<usize>,
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

impl<D: EdgeArg> Edge<D> {
    pub fn secant_area(&self) -> D {
        let r2 = match &*self.c.borrow() {
            Shape::Circle(c) => c.clone().r * c.clone().r,
            Shape::XYRR(e) => e.r.clone().x * e.clone().r.y,
        };
        let theta = self.theta();
        r2 / 2. * (theta.clone() - theta.sin())
    }
    /// Angle span of this Edge, in terms of the shape whose border it is part of
    pub fn theta(&self) -> D {
        let theta = self.t1.clone() - self.t0.clone();
        if theta.clone().into() < 0. {
            panic!("Invalid edge {}, negative theta: {}", self, theta)
        }
        theta
    }
    pub fn shape_idx(&self) -> usize {
        self.c.borrow().idx()
    }
    /// Return all shape indices that either contain this Edge, or which this Edge runs along the border of
    pub fn all_idxs(&self) -> BTreeSet<usize> {
        let mut idxs = self.container_idxs.clone();
        idxs.insert(self.c.borrow().idx());
        idxs
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
        let containers: Vec<String> = self.container_idxs.iter().map(|idx| format!("{}", idx)).collect();
        write!(
            f,
            "C{}: {}({}) → {}({}), containers: [{}] ({})",
            self.c.borrow().idx(),
            self.n0.borrow().idx, self.t0.clone().into().deg_str(),
            self.n1.borrow().idx, self.t1.clone().into().deg_str(),
            containers.join(","),
            if self.is_component_boundary { "external" } else { "internal" },
        )
    }
}