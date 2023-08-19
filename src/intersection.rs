use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

use crate::deg::Deg;
use crate::dual::Dual;
use crate::edge::E;
use crate::r2::R2;

type D = Dual;
pub type Node = Rc<RefCell<Intersection>>;

#[derive(Clone, Debug)]
pub struct Intersection {
    pub x: D,
    pub y: D,
    pub c0idx: usize,
    pub c1idx: usize,
    pub t0: D,
    pub t1: D,
    pub edges: Vec<E>,
}

impl Intersection {
    pub fn theta(&self, idx: usize) -> D {
        if idx == self.c0idx {
            self.t0.clone()
        } else if idx == self.c1idx {
            self.t1.clone()
        } else {
            panic!("Invalid circle index {} ({}, {})", idx, self.c0idx, self.c1idx);
        }
    }
    pub fn add_edge(&mut self, edge: E) {
        self.edges.push(edge);
    }
    pub fn p(&self) -> R2<D> {
        R2 { x: self.x.clone(), y: self.y.clone() }
    }
    pub fn other(&self, cidx: usize) -> usize {
        if cidx == self.c0idx {
            self.c1idx
        } else if cidx == self.c1idx {
            self.c0idx
        } else {
            panic!("Invalid circle index {} ({}, {})", cidx, self.c0idx, self.c1idx);
        }
    }
}

impl Display for Intersection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I({:.3}, {:.3}, C{}({})/C{}({})", self.x, self.y, self.c0idx, self.t0.deg().s(0), self.c1idx, self.t1.deg().s(0))
    }
}
