use std::{fmt::Display, ops::{Div, Mul}, rc::Rc, cell::RefCell};

use crate::{circle::Circle, intersection::{Intersection, Node}, dual::Dual};

type D = Dual;

#[derive(Debug, Clone)]
pub struct Edge {
    pub c: Rc<RefCell<Circle<D>>>,
    pub i0: Node,
    pub i1: Node,
}

impl Edge {
    pub fn t0(&self) -> D {
        self.i0.borrow().theta(self.c.borrow().idx)
    }
    pub fn t1(&self) -> D {
        self.i1.borrow().theta(self.c.borrow().idx)
    }
}

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({}, {}, {})", self.c.borrow(), self.i0.borrow(), self.i1.borrow())
    }
}