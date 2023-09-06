use std::fmt::{Display, Formatter, self};

use crate::{edge::E, dual::D, node::N};

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
        let end_p = end.borrow().v();
        let edge = self.edge.clone();
        let idx = edge.borrow().c.borrow().idx();
        let successors = end.borrow().edges.iter().filter(|e| {
            e.borrow().c.borrow().idx() != idx && e.borrow().visits < e.borrow().expected_visits
        }).map(|e| {
            let p = e.borrow().i0.borrow().v();
            let fwd = p == end_p;
            Segment { edge: e.clone(), fwd }
        }).collect();
        successors
    }
}

impl Display for Segment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} (fwd: {})", self.edge.borrow(), self.fwd)
    }
}
