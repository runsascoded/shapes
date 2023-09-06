use std::{fmt::Display, rc::Rc, cell::RefCell, collections::HashSet};

use crate::{node::N, dual::D, shape::{S, Shape}};

pub type E = Rc<RefCell<Edge>>;

#[derive(Debug, Clone)]
pub struct Edge {
    pub idx: usize,
    pub c: S,
    pub c0: S,
    pub c1: S,
    pub i0: N,
    pub i1: N,
    pub t0: D,
    pub t1: D,
    pub containers: Vec<S>,
    pub containments: Vec<bool>,
    pub expected_visits: usize,
    pub visits: usize,
}

impl Edge {
    pub fn secant_area(&self) -> D {
        //let r = &self.c.borrow().r.clone();
        let r2 = match &*self.c.borrow() {
            Shape::Circle(c) => c.clone().r * c.clone().r,
            Shape::XYRR(e) => e.r.clone().x * e.clone().r.y,
        };
        let theta = self.theta();
        r2 / 2. * (theta.clone() - theta.sin())
    }

    /// Angle span of this Edge, in terms of the shape whose border it is part of
    pub fn theta(&self) -> D {
        let theta = self.t1.clone() - &self.t0;
        if theta.re < 0. {
            panic!("Invalid edge {}, negative theta: {}", self, theta)
        }
        theta
    }

    /// Return all shape indices that either contain this Edge
    pub fn container_idxs(&self) -> HashSet<usize> {
        self.containers.iter().map(|c| c.borrow().idx()).collect()
    }

    /// Return all shape indices that either contain this Edge, or which this Edge runs along the border of
    pub fn all_idxs(&self) -> HashSet<usize> {
        let mut idxs = self.container_idxs();
        idxs.insert(self.c.borrow().idx());
        idxs
    }
}

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({}, {}, {})", self.c.borrow(), self.i0.borrow(), self.i1.borrow())
    }
}