use std::f64::consts::PI;
use std::fmt::Display;
use std::ops::{Mul, Div};

use crate::circle::Circle;
use crate::edge::Edge;

#[derive(Clone, Debug)]
pub struct Intersection<D> {
    pub x: D,
    pub y: D,
    pub c0idx: usize,
    pub c1idx: usize,
    pub t0: D,
    pub t1: D,
    // pub edges: [ [usize; 2]; 2],
}

impl<D: Display + Clone + Mul<f64, Output = D> + Div<f64, Output = D>> Display for Intersection<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I({:.3}, {:.3}, C{}/C{}", self.x, self.y, self.c0idx, self.c1idx)
    }
}