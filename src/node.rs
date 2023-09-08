use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

use crate::intersection::Intersection;
use crate::dual::Dual;
use crate::edge::E;
use crate::r2::R2;

type D = Dual;
pub type N<D> = Rc<RefCell<Node<D>>>;

#[derive(Clone, Debug)]
pub struct Node<D> {
    pub idx: usize,
    pub i: Intersection<D>,
    pub edges: Vec<E<D>>,
}

impl<D: Clone> Node<D>
where Intersection<D>: Display
{
    pub fn theta(&self, idx: usize) -> D {
        let i = &self.i;
        if idx == i.c0idx {
            i.t0.clone()
        } else if idx == i.c1idx {
            i.t1.clone()
        } else {
            panic!("Invalid circle index {} ({}, {}): {}", idx, i.c0idx, i.c1idx, i);
        }
    }
    pub fn add_edge(&mut self, edge: E<D>) {
        self.edges.push(edge);
    }
    pub fn p(&self) -> R2<D> {
        self.i.p()
    }
    pub fn other(&self, cidx: usize) -> usize {
        self.i.other(cidx)
    }
}
impl Node<D> {
    pub fn v(&self) -> R2<f64> {
        self.i.v().p()
    }
}

impl<D: Display> Display for Node<D>
where
    Intersection<D>: Display
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.i.fmt(f)
    }
}
