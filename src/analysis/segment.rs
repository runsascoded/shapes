use std::{fmt::{Display, Formatter, self}, ops::Neg};

use crate::{edge::{E, EdgeArg, Edge}, node::N, r2::R2, to::To, zero::Zero};

#[derive(Debug, Clone)]
pub struct Segment<D> {
    pub edge: E<D>,
    pub fwd: bool,
}

impl<D: EdgeArg + Neg<Output = D> + Zero> Segment<D>
where
    R2<D>: To<R2<f64>>,
{
    pub fn secant_area(&self) -> D {
        let secant_area = self.edge.borrow().secant_area();
        if self.fwd { secant_area } else { -secant_area }
    }
    pub fn start(&self) -> N<D> {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.node0 } else { &e.node1 };
        i.clone()
    }
    pub fn end(&self) -> N<D> {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.node1 } else { &e.node0 };
        i.clone()
    }
    pub fn successors(&self) -> Vec<Segment<D>> {
        let end = self.end();
        let end_p: R2<f64> = end.borrow().p.clone().to();
        let edge = self.edge.clone();
        let shape_idx = edge.borrow().set_idx();
        let successors = end.borrow().edges.iter().filter(|e| {
            e.borrow().set_idx() != shape_idx && e.borrow().visits < e.borrow().expected_visits()
        }).map(|e| {
            let p: R2<f64> = e.borrow().node0.borrow().p.clone().to();
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
