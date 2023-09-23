use std::{fmt::Display, rc::Rc, cell::RefCell, collections::BTreeSet, ops::{Mul, Div, Sub}};

use crate::{math::deg::Deg, node::N, set::S, shape::Shape::{Circle, XYRR, XYRRT}, trig::Trig, dual::Dual};

pub type E<D> = Rc<RefCell<Edge<D>>>;

#[derive(Debug, Clone)]
pub struct Edge<D> {
    pub idx: usize,
    pub set: S<D>,
    pub node0: N<D>,
    pub node1: N<D>,
    pub theta0: D,
    pub theta1: D,
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
        let r2 = match &self.set.borrow().shape {
            Circle(c) => c.clone().r * c.clone().r,
            XYRR(e) => e.r.clone().x * e.clone().r.y,
            XYRRT(e) => e.r.clone().x * e.clone().r.y,
        };
        let theta = self.theta();
        r2 / 2. * (theta.clone() - theta.sin())
    }
    /// Angle span of this Edge, in terms of the shape whose border it is part of
    pub fn theta(&self) -> D {
        let theta = self.theta1.clone() - self.theta0.clone();
        if theta.clone().into() < 0. {
            panic!("Invalid edge {}, negative theta: {}", self, theta)
        }
        theta
    }
    pub fn set_idx(&self) -> usize {
        self.set.borrow().idx
    }
    /// Return all shape indices that either contain this Edge, or which this Edge runs along the border of
    pub fn all_idxs(&self) -> BTreeSet<usize> {
        let mut idxs = self.container_idxs.clone();
        idxs.insert(self.set.borrow().idx);
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
            self.set.borrow().idx,
            self.node0.borrow().idx, self.theta0.clone().into().deg_str(),
            self.node1.borrow().idx, self.theta1.clone().into().deg_str(),
            containers.join(","),
            if self.is_component_boundary { "external" } else { "internal" },
        )
    }
}