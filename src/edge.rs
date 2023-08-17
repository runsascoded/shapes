use std::fmt::Display;

use crate::{circle::Circle, intersection::Intersection};

#[derive(Debug, Clone)]
pub struct Edge<'a, D> {
    pub c: Circle<D>,
    pub intersections: &'a [ Intersection<'a, D>; 2 ],
}

impl<'a, D: Display> Display for Edge<'a, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({}, {}, {})", self.c, self.intersections[0], self.intersections[1])
    }
}