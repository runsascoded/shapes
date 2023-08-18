use std::cell::RefCell;
use std::f64::consts::PI;
use std::fmt::Display;
use std::ops::{Mul, Div};
use std::rc::Rc;

use crate::circle::Circle;
use crate::deg::Deg;
use crate::dual::Dual;
use crate::edge::Edge;

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
    // pub edges: [ [usize; 2]; 2],
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
}

impl Display for Intersection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I({:.3}, {:.3}, C{}({})/C{}({})", self.x, self.y, self.c0idx, self.t0.deg().s(0), self.c1idx, self.t1.deg().s(0))
    }
}
