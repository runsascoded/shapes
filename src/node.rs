use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::{Div, Mul, Add};
use std::rc::Rc;

use crate::fmt::Fmt;
use crate::intersection::Intersection;
use crate::dual::Dual;
use crate::edge::E;
use crate::math::deg::Deg;
use crate::r2::R2;

type D = Dual;
pub type N<D> = Rc<RefCell<Node<D>>>;

#[derive(Clone, Debug)]
pub struct Node<D> {
    pub idx: usize,
    pub p: R2<D>,
    pub intersections: Vec<Intersection<D>>,
    pub shape_thetas: BTreeMap<usize, D>,
    pub edges: Vec<E<D>>,
}

impl<
    D
    : Clone
    + Add<Output = D>
    + Mul<f64, Output = D>
    + Div<f64, Output = D>,
> Node<D>
where
    Intersection<D>: Display,
    R2<D>
    : Add<Output = R2<D>>
    // TODO: can't get these to derive for R2<Dual>
    // + Mul<f64, Output = R2<D>>
    // + Div<f64, Output = R2<D>>,
{
    pub fn theta(&self, idx: usize) -> D {
        self.shape_thetas.get(&idx).unwrap().clone()
    }
    pub fn add_edge(&mut self, edge: E<D>) {
        self.edges.push(edge);
    }
    pub fn merge(&mut self, intersection: Intersection<D>) {
        let p = self.p.clone();
        let o = intersection.p();
        let n: f64 = self.intersections.len() as f64;
        self.p = R2 {
            x: p.x * n / (n + 1.) + o.x / (n + 1.),
            y: p.y * n / (n + 1.) + o.y / (n + 1.),
        };
        // self.p = p * n / (n + 1.) + o / (n + 1.);
        if !self.shape_thetas.contains_key(&intersection.c0idx) {
            self.shape_thetas.insert(intersection.c0idx, intersection.t0.clone());
        }
        if !self.shape_thetas.contains_key(&intersection.c1idx) {
            self.shape_thetas.insert(intersection.c1idx, intersection.t1.clone());
        }
        self.intersections.push(intersection);
    }
}
impl Node<D> {
    pub fn v(&self) -> R2<f64> {
        self.p.v()
    }
}

impl<D: Display + Deg + Fmt> Display for Node<D>
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
