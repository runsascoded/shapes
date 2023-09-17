use std::fmt::{Display, Formatter, self};

use crate::{edge::{E, EdgeArg, Edge}, node::N, intersection::Intersection, r2::R2, to::To};

#[derive(Debug, Clone)]
pub struct Segment<D> {
    pub edge: E<D>,
    pub fwd: bool,
}

impl<D: EdgeArg> Segment<D>
where
    Intersection<D>: Display,
    R2<D>: To<R2<f64>>,
{
    pub fn secant_area(&self) -> D {
        self.edge.borrow().secant_area()
    }
    pub fn start(&self) -> N<D> {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.i0 } else { &e.i1 };
        i.clone()
    }
    pub fn end(&self) -> N<D> {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.i1 } else { &e.i0 };
        i.clone()
    }
    pub fn successors(&self) -> Vec<Segment<D>> {
        let end = self.end();
        let end_p: R2<f64> = end.borrow().p.clone().to();
        let edge = self.edge.clone();
        let idx = edge.borrow().c.borrow().idx();
        let successors = end.borrow().edges.iter().filter(|e| {
            e.borrow().c.borrow().idx() != idx && e.borrow().visits < e.borrow().expected_visits
        }).map(|e| {
            let p: R2<f64> = e.borrow().i0.borrow().p.clone().to();
            let fwd = p == end_p;
            Segment { edge: e.clone(), fwd }
        }).collect();
        successors
    }
}

impl<D> Display for Segment<D>
where Edge<D>: Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} (fwd: {})", self.edge.borrow(), self.fwd)
    }
}
