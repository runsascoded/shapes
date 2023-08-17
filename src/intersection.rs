use std::fmt::Display;

use crate::circle::Circle;

#[derive(Clone, Debug)]
pub struct Intersection<D> {
    pub x: D,
    pub y: D,
    pub c0: Circle<D>,
    pub c1: Circle<D>,
    pub t0: D,
    pub t1: D,
}

impl<D: Display> Display for Intersection<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I({:.3}, {:.3}, {}, {})", self.x, self.y, self.c0, self.c1)
    }
}