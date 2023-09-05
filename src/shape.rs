use std::{rc::Rc, cell::RefCell};

use derive_more::{From, Into};
use tsify::declare;

use crate::{dual::D, circle, ellipses::xyrr, zero::Zero, transform::{Projection, Transform}};

#[declare]
pub type Duals = Vec<Vec<f64>>;
#[declare]
pub type Input = (Shape<f64>, Duals);

#[derive(Debug, Clone, From)]
pub enum Shape<D> {
    Circle(circle::Circle<D>),
    XYRR(xyrr::XYRR<D>),
}

impl Shape<f64> {
    pub fn dual(&self, duals: &Duals) -> Shape<D> {
        match self {
            Shape::Circle(c) => Shape::Circle(c.dual(duals)),
            Shape::XYRR(e) => Shape::XYRR(e.dual(duals)),
        }
    }
}

impl<D> Shape<D> {
    pub fn idx(&self) -> usize {
        match self {
            Shape::Circle(c) => c.idx,
            Shape::XYRR(e) => e.idx,
        }
    }
}

impl Shape<D> {
    pub fn n(&self) -> usize {
        match self {
            Shape::Circle(c) => c.n(),
            Shape::XYRR(e) => e.n(),
        }
    }
    pub fn zero(&self) -> D {
        match self {
            Shape::Circle(c) => Zero::zero(&c.r),
            Shape::XYRR(e) => Zero::zero(&e.c.x),
        }
    }
    pub fn intersect(&self, other: &Shape<D>) -> bool {
        match self {
            Shape::Circle(c) => c.intersect(other),
            Shape::XYRR(e) => e.intersect(other),
        }
    }
    pub fn apply(&self, projection: &Projection<D>) -> Shape<D> {
        projection.0.iter().fold(*self, |c, t| c.transform(t))
    }
    pub fn transform(&self, transform: &Transform<D>) -> Shape<D> {
        match self {
            Shape::Circle(c) => c.transform(transform),
            Shape::XYRR(e) => Shape::XYRR(e.transform(transform)),
        }
    }
}

pub type S = Rc<RefCell<Shape<D>>>;
