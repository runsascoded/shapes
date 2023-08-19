use std::{fmt::Display, ops::{Div, Mul}, rc::Rc, cell::RefCell};

use crate::{circle::Circle, intersection::{Intersection, Node}, dual::Dual};

type C = Rc<RefCell<Circle<D>>>;
type D = Dual;

#[derive(Debug, Clone)]
pub struct Edge {
    pub c: C,
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