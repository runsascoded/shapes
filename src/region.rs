use std::fmt::{Formatter, Display, self};

use crate::{edge::E, node::N, dual::{Dual, D}};

#[derive(Debug, Clone)]
pub struct Segment {
    pub edge: E,
    pub fwd: bool,
}

impl Segment {
    pub fn secant_area(&self) -> D {
        self.edge.borrow().secant_area()
    }
    pub fn start(&self) -> N {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.i0 } else { &e.i1 };
        i.clone()
    }
    pub fn end(&self) -> N {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.i1 } else { &e.i0 };
        i.clone()
    }
    pub fn successors(&self) -> Vec<Segment> {
        let end = self.end();
        let end_p = end.borrow().p();
        let edge = self.edge.clone();
        let idx = edge.borrow().c.borrow().idx();
        let successors = end.borrow().edges.iter().filter(|e| {
            e.borrow().c.borrow().idx() != idx && e.borrow().visits < e.borrow().expected_visits
        }).map(|e| {
            let p = e.borrow().i0.borrow().p();
            let fwd = p == end_p;
            Segment { edge: e.clone(), fwd }
        }).collect();
        successors
    }
}

#[derive(Debug, Clone)]
pub struct Region {
    pub key: String,
    pub segments: Vec<Segment>,
    pub container_idxs: Vec<usize>,
    pub container_bmp: Vec<bool>,
}

impl Region {
    pub fn len(&self) -> usize {
        self.segments.len()
    }
    pub fn polygon_area(&self) -> D {
        (self.segments.iter().map(|s| {
            let cur = s.start().borrow().p();
            let nxt = s.end().borrow().p();
            cur.x * nxt.y - cur.y * nxt.x
        }).sum::<D>() / 2.).abs()
    }
    pub fn secant_area(&self) -> D {
        self.segments.iter().map(|s| {
            let area = s.secant_area();
            let idx = s.edge.borrow().c.borrow().idx();
            if self.container_idxs.contains(&idx) { area } else { -area }
        }).sum::<D>()
    }
    pub fn area(&self) -> D {
        self.polygon_area() + self.secant_area()
    }
    pub fn matches(&self, key: &String) -> bool {
        for (idx, ch) in (&key).chars().enumerate() {
            if ch == '-' && self.container_bmp[idx] {
                return false;
            }
            if ch != '*' && !self.container_bmp[idx] {
                return false
            }
        }
        true
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f, "R({})",
            self.segments.iter().map(|s| {
                format!("{}", s.edge.borrow())
            }).collect::<Vec<String>>().join(", ")
        )
    }
}