use std::{fmt::Display, ops::{Div, Mul}};

use crate::{circle::Circle, intersection::Intersection};

#[derive(Debug, Clone)]
pub struct Edge<D> {
    pub c: Circle<D>,
    pub i0: Intersection<D>,
    pub i1: Intersection<D>,
}

impl<D: Display + Clone + Mul<f64, Output = D> + Div<f64, Output = D>> Display for Edge<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E({}, {}, {})", self.c, self.i0, self.i1)
    }
}