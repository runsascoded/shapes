use std::fmt::{Formatter, Display, self};

use crate::{edge::E, intersection::Node, dual::Dual};

#[derive(Debug, Clone)]
pub struct Segment {
    pub edge: E,
    pub fwd: bool,
}

impl Segment {
    pub fn sgn(&self) -> f64 {
        if self.fwd { 1. } else { -1. }
    }
    pub fn secant_area(&self) -> D {
        self.edge.borrow().secant_area() * self.sgn()
    }
    pub fn start(&self) -> Node {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.i0 } else { &e.i1 };
        i.clone()
    }
    pub fn end(&self) -> Node {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.i1 } else { &e.i0 };
        i.clone()
    }
    pub fn successor_candidates(&self) -> Vec<Segment> {
        let end = self.end();
        let end_p = end.borrow().p();
        // println!("end: {}", end_p);
        let edge = self.edge.clone();
        let idx = edge.borrow().c.borrow().idx;
        let successors = end.borrow().edges.iter().filter(|e| {
            e.borrow().c.borrow().idx != idx
        }).map(|e| {
            let p = e.borrow().i0.borrow().p();
            let fwd = p == end_p;
            // println!("chk: {}: {}", p, fwd);
            Segment { edge: e.clone(), fwd }
        }).collect();
        successors
    }
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
    pub fn polygon_area(&self) -> D {
        self.segments.iter().map(|s| {
            let cur = s.start().borrow().p();
            let nxt = s.end().borrow().p();
            cur.x * nxt.y - cur.y * nxt.x
        }).sum::<D>() / 2.
    }
    pub fn secant_area(&self) -> D {
        self.segments.iter().map(|s| s.secant_area()).sum::<D>()
    }
    pub fn area(&self) -> D {
        self.polygon_area() + self.secant_area()
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // let mut s: String = "Region(".to_owned();
        write!(
            f, "R({})",
            self.segments.iter().map(|s| {
                format!("{}", s.edge.borrow())
            }).collect::<Vec<String>>().join(", ")
        )
    }
}