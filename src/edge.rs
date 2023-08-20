use std::{fmt::Display, rc::Rc, cell::RefCell, collections::HashSet};

use crate::{circle::C, intersection::Node, dual::D};

pub type E = Rc<RefCell<Edge>>;

#[derive(Debug, Clone)]
pub struct Edge {
    pub c: C,
    pub c0: C,
    pub c1: C,
    pub i0: Node,
    pub i1: Node,
    pub t0: D,
    pub t1: D,
    pub containers: Vec<C>,
    pub containments: Vec<bool>,
    pub expected_visits: usize,
    pub visits: usize,
}

impl Edge {
    pub fn secant_area(&self) -> D {
        let r = &self.c.borrow().r.clone();
        let theta = self.theta();
        r * r / 2. * (theta.clone() - theta.sin())
    }
    pub fn theta(&self) -> D {
        let theta = self.t1.clone() - &self.t0;
        if theta.re < 0. {
            panic!("Invalid edge {}, negative theta: {}", self, theta)
        }
        theta
    }
    pub fn container_idxs(&self) -> HashSet<usize> {
        self.containers.iter().map(|c| c.borrow().idx).collect()
    }
    pub fn all_idxs(&self) -> HashSet<usize> {
        let mut idxs = self.container_idxs();
        idxs.insert(self.c.borrow().idx);
        idxs
    }
}

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({}, {}, {})", self.c.borrow(), self.i0.borrow(), self.i1.borrow())
    }
}