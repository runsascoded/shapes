use std::{iter::Sum, ops::{Div, Mul, Sub, Add}, fmt::{Formatter, Display, self}};

use crate::{edge::Edge, intersection::Intersection};

#[derive(Debug, Clone)]
pub struct Region<'a, D> {
    pub edges: Vec<&'a Edge<'a, D>>,
    pub intersections: Vec<&'a Intersection<'a, D>>,
}

impl<'a, D: Clone + Sum + Add<Output = D> + Mul<Output = D> + Sub<Output = D> + Div<f64, Output = D>> Region<'a, D> {
    pub fn n(self: &'a Region<'a, D>) -> usize {
        assert_eq!(self.edges.len(), self.intersections.len());
        self.edges.len()
    }
    pub fn polygon_area(&self) -> D {
        let n = self.intersections.len();
        let iter = self.intersections.iter().enumerate();
        let pcs = iter.map(|(idx, cur)| {
            let nxt = &self.intersections[(idx + 1) % n];
            cur.x.clone() * nxt.y.clone() - cur.y.clone() * nxt.x.clone()
        });
        let sum = pcs.sum::<D>();
        sum / 2.
    }
    pub fn secant_area(&self) -> D {
        todo!();
    }
    pub fn area(&self) -> D {
        self.polygon_area() + self.secant_area()
    }
}

impl<'a, D: Display> Display for Region<'a, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // let mut s: String = "Region(".to_owned();
        write!(f, "R(").unwrap();
        for (idx, (intersection, edge)) in self.intersections.iter().zip(self.edges.iter()).enumerate() {
            if idx != 0 {
                write!(f, ", ").unwrap();
                // s.push_str(", ");
            }
            intersection.fmt(f).unwrap();
            write!(f, " ").unwrap();
            edge.fmt(f).unwrap();
        }
        write!(f, ")")
    }
}