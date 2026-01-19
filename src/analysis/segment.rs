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

        // First try: edges from other shapes (standard behavior for circle/ellipse intersections)
        let other_shape_edges: Vec<_> = end.borrow().edges.iter().filter(|e| {
            e.borrow().set_idx() != shape_idx && e.borrow().visits < e.borrow().expected_visits()
        }).cloned().collect();

        // If no edges from other shapes, this might be a polygon vertex (not an intersection point).
        // In that case, continue along the same polygon's perimeter.
        let edges_to_use = if other_shape_edges.is_empty() {
            // Allow continuing on same shape, but only forward (not backtracking)
            // The "forward" edge is the one that starts at this node (node0 == end)
            end.borrow().edges.iter().filter(|e| {
                let e_ref = e.borrow();
                e_ref.set_idx() == shape_idx &&
                e_ref.idx != edge.borrow().idx &&  // not the edge we came from
                e_ref.visits < e_ref.expected_visits()
            }).cloned().collect()
        } else {
            other_shape_edges
        };

        edges_to_use.iter().map(|e| {
            let p: R2<f64> = e.borrow().node0.borrow().p.clone().to();
            let fwd = p == end_p;
            Segment { edge: e.clone(), fwd }
        }).collect()
    }
}

impl<D> Display for Segment<D>
where Edge<D>: Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} (fwd: {})", self.edge.borrow(), self.fwd)
    }
}
