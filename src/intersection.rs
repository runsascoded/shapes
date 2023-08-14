use std::fmt::Display;

use crate::circle::Circle;

#[derive(Clone, Debug)]
pub struct Intersection<D> {
    pub x: D,
    pub y: D,
    pub c1: Circle<D>,
    pub c2: Circle<D>,
}

impl<D: Display> Display for Intersection<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I({}, {}, {}, {})", self.x, self.y, self.c1, self.c2)
    }
}