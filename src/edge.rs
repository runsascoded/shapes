use std::{fmt::Display, ops::{Div, Mul}};

use crate::{circle::Circle, intersection::{Intersection, Node}, dual::Dual};

type D = Dual;

#[derive(Debug, Clone)]
pub struct Edge {
    pub c: Circle<D>,
    pub i0: Node,
    pub i1: Node,
}

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({}, {}, {})", self.c, self.i0.borrow(), self.i1.borrow())
    }
}