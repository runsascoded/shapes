use std::{iter::Sum, ops::{Div, Mul, Sub}};

use crate::{edge::Edge, intersection::Intersection};


pub struct Region<'a, D> {
    pub edges: Vec<Edge<'a, D>>,
    pub intersections: Vec<Intersection<'a, D>>,
}

impl<'a, D> Region<'a, D>
where
    &'a D: Mul<&'a D, Output = &'a D>,
    &'a D: Sub<&'a D, Output = &'a D>,
    &'a D: Div<f64, Output = &'a D>,
    &'a D: Sum<&'a D>
{
    pub fn polygon_area(&self) -> &D {
        let n = self.intersections.len();
        let iter = self.intersections.iter().enumerate();
        let pcs = iter.map(|(idx, cur)| {
            let nxt = &self.intersections[(idx + 1) % n];
            cur.x * nxt.y - cur.y * nxt.x
        });
        let sum = pcs.sum::<&D>();
        &(sum / 2.)
    }
    pub fn secant_area(&self) -> D {
        todo!();
    }
    pub fn area(&self) -> D {
        todo!();
    }
}