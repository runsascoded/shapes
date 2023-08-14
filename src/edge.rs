use std::fmt::Display;

use crate::{circle::Circle, intersection::Intersection};

#[derive(Debug, Clone)]
pub struct Edge<D> {
    pub c: Circle<D>,
    pub intersections: [ Intersection<D>; 2 ],
}

impl<D: Display> Display for Edge<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({}, {}, {})", self.c, self.intersections[0], self.intersections[1])
    }
}