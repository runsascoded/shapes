use std::{fmt::Display, ops::{Div, Mul}};

use crate::{circle::Circle, intersection::Intersection, dual::Dual};

type D = Dual;

#[derive(Debug, Clone)]
pub struct Edge {
    pub c: Circle<D>,
    pub i0: Intersection,
    pub i1: Intersection,
}

impl Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({}, {}, {})", self.c, self.i0, self.i1)
    }
}