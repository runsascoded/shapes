use std::{fmt::Display, rc::Rc, cell::RefCell};

use crate::{circle::C, intersection::Node, dual::D};

pub type E = Rc<RefCell<Edge>>;

#[derive(Debug, Clone)]
pub struct Edge {
    pub c: C,
    pub c0: C,
    pub c1: C,
    pub i0: Node,
    pub i1: Node,
    pub containers: Vec<C>,
    pub containments: Vec<bool>,
}

impl Edge {
    pub fn t0(&self) -> D {
        self.i0.borrow().theta(self.c.borrow().idx)
    }
    pub fn t1(&self) -> D {
        self.i1.borrow().theta(self.c.borrow().idx)
    }
    pub fn midpoint(&self) -> D {
        (self.t0() + self.t1()) / 2.
    }
}

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({}, {}, {})", self.c.borrow(), self.i0.borrow(), self.i1.borrow())
    }
}