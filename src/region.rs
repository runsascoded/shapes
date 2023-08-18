use std::{iter::Sum, ops::{Div, Mul, Sub, Add}, fmt::{Formatter, Display, self}};

use crate::{edge::Edge, intersection::Intersection, dual::Dual};

#[derive(Debug, Clone)]
pub struct Segment {
    pub intersection_idx: usize,
    pub edge_idx: usize,
}

#[derive(Debug, Clone)]
pub struct Region {
    pub segments: Vec<Segment>,
}

type D = Dual;

impl Region {
    pub fn len(&self) -> usize {
        self.segments.len()
    }
    pub fn polygon_area(&self, intersections: &Vec<Intersection>) -> D {
        let n = self.len();
        let iter = self.segments.iter().enumerate();
        let pcs = iter.map(|(idx, cur_s)| {
            let nxt_s = &self.segments[(idx + 1) % n];
            let cur = &intersections[cur_s.intersection_idx];
            let nxt = &intersections[nxt_s.intersection_idx];
            cur.x.clone() * nxt.y.clone() - cur.y.clone() * nxt.x.clone()
        });
        let sum = pcs.sum::<D>();
        sum / 2.
    }
    pub fn secant_area(&self) -> D {
        todo!();
    }
    pub fn area(&self, intersections: &Vec<Intersection>) -> D {
        self.polygon_area(intersections) + self.secant_area()
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // let mut s: String = "Region(".to_owned();
        write!(f, "R(").unwrap();
        for (idx, segment) in self.segments.iter().enumerate() {
            if idx != 0 {
                write!(f, ", ").unwrap();
                // s.push_str(", ");
            }
            write!(f, "I{} E{}", segment.intersection_idx, segment.edge_idx).unwrap();
        }
        write!(f, ")")
    }
}