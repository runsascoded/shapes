use std::fmt::Display;

use crate::circle::Circle;
use crate::edge::Edge;

#[derive(Clone, Debug)]
pub struct Intersection<'a, D> {
    pub x: D,
    pub y: D,
    pub c0: Circle<D>,
    pub c1: Circle<D>,
    pub t0: D,
    pub t1: D,
    pub edges: Option<[ [&'a Edge<'a, D>; 2]; 2]>,
}

impl<'a, D: Display> Display for Intersection<'a, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I({:.3}, {:.3}, {}, {})", self.x, self.y, self.c0, self.c1)
    }
}