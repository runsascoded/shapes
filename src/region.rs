use std::{iter::Sum, ops::{Div, Mul, Sub}, fmt::{Formatter, Display, self}};

use crate::{edge::Edge, intersection::Intersection};

#[derive(Debug, Clone)]
pub struct Region<D> {
    pub edges: Vec<Edge<D>>,
    pub intersections: Vec<Intersection<D>>,
}

impl<'a, D: 'a + Clone + Sum + Mul<Output = D> + Sub<Output = D> + Div<f64, Output = D>> Region<D>
where
//     &'a D: Mul<&'a D, Output = &'a D>,
//     &'a D: Sub<&'a D, Output = &'a D>,
    // &'a D: Div<f64, Output = &'a D>,
    // &'a D: Sum<D>
{
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
        todo!();
    }
}

impl<D: Display> Display for Region<D> {
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