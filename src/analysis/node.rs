use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::{Div, Mul, Add};
use std::rc::Rc;

use crate::fmt::DisplayNum;
use crate::intersection::Intersection;
use crate::dual::Dual;
use crate::edge::E;
use crate::r2::R2;

type D = Dual;
pub type N<D> = Rc<RefCell<Node<D>>>;

#[derive(Clone, Debug)]
pub struct Node<D> {
    pub idx: usize,
    pub p: R2<D>,
    pub n: usize,
    pub shape_thetas: BTreeMap<usize, D>,
    pub edges: Vec<E<D>>,
}

impl<D: Clone> Node<D> {
    pub fn theta(&self, idx: usize) -> D {
        self.shape_thetas.get(&idx).unwrap().clone()
    }
}
impl<D> Node<D> {
    pub fn add_edge(&mut self, edge: E<D>) {
        self.edges.push(edge);
    }
}
impl Node<D> {
    pub fn v(&self) -> R2<f64> {
        self.p.v()
    }
}

impl<D> Node<D>
where
    D: Clone + Add<Output = D> + Mul<f64, Output = D> + Div<f64, Output = D>,
    R2<D>: Add<Output = R2<D>> + Mul<f64, Output = R2<D>> + Div<f64, Output = R2<D>>,
{
    pub fn merge(&mut self, o: R2<D>, set0_idx: usize, set0_theta: &D, set1_idx: usize, set1_theta: &D) {
        let p = self.p.clone();
        let n: f64 = self.n as f64;
        self.p = p * n / (n + 1.) + o / (n + 1.);
        self.n += 1;
        self.shape_thetas.entry(set0_idx).or_insert_with(|| set0_theta.clone());
        self.shape_thetas.entry(set1_idx).or_insert_with(|| set1_theta.clone());
    }
}
impl<D: DisplayNum> Display for Node<D>
where
    Intersection<D>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "N{}({}, {}: {})",
            self.idx,
            self.p.x.s(3),
            self.p.y.s(3),
            self.shape_thetas.iter().map(|(cidx, theta)| format!("C{}({})", cidx, theta.deg_str())).collect::<Vec<_>>().join(", ")
        )
    }
}
