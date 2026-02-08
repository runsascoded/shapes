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
        let start_p: R2<f64> = self.start().borrow().p.clone().to();
        let edge = self.edge.clone();
        let shape_idx = edge.borrow().set_idx();

        // Check if this node is on multiple shapes (intersection) or just one (polygon vertex)
        let is_intersection = end.borrow().shape_coords.len() > 1;

        let edges_to_use: Vec<_> = if is_intersection {
            // Intersection node: switch to edges from OTHER shapes
            end.borrow().edges.iter().filter(|e| {
                e.borrow().set_idx() != shape_idx && e.borrow().visits < e.borrow().expected_visits()
            }).cloned().collect()
        } else {
            // Polygon vertex (not an intersection): continue along SAME shape
            end.borrow().edges.iter().filter(|e| {
                let e_ref = e.borrow();
                e_ref.set_idx() == shape_idx &&
                e_ref.idx != edge.borrow().idx &&  // not the edge we came from
                e_ref.visits < e_ref.expected_visits()
            }).cloned().collect()
        };

        // Build segment candidates with direction info
        let mut candidates: Vec<Segment<D>> = edges_to_use.iter().map(|e| {
            let p: R2<f64> = e.borrow().node0.borrow().p.clone().to();
            let fwd = p == end_p;
            Segment { edge: e.clone(), fwd }
        }).collect();

        // Sort by CW turning angle from incoming direction (standard planar face enumeration).
        // The tightest right turn traces the smallest face, so DFS tries the correct
        // successor first and avoids unnecessary backtracking.
        let in_dx = end_p.x - start_p.x;
        let in_dy = end_p.y - start_p.y;
        if in_dx != 0.0 || in_dy != 0.0 {
            // Reverse of incoming direction (points back along the edge we came from)
            let rev_dx = -in_dx;
            let rev_dy = -in_dy;
            candidates.sort_by(|a, b| {
                let cw_angle = |seg: &Segment<D>| -> f64 {
                    let next_p: R2<f64> = seg.end().borrow().p.clone().to();
                    let out_dx = next_p.x - end_p.x;
                    let out_dy = next_p.y - end_p.y;
                    // CW angle from reverse to outgoing: negate cross product for CW
                    let cross = rev_dx * out_dy - rev_dy * out_dx;
                    let dot = rev_dx * out_dx + rev_dy * out_dy;
                    let angle = (-cross).atan2(dot);
                    if angle < 0.0 { angle + std::f64::consts::TAU } else { angle }
                };
                cw_angle(a).partial_cmp(&cw_angle(b)).unwrap()
            });
        }

        candidates
    }
}

impl<D> Display for Segment<D>
where Edge<D>: Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} (fwd: {})", self.edge.borrow(), self.fwd)
    }
}
