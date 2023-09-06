use std::{rc::Rc, cell::RefCell, ops::{Mul, Add}};

use derive_more::{From, Display};
use serde::{Deserialize, Serialize};
use tsify::{declare, Tsify};

use crate::{dual::D, circle, ellipses::xyrr, zero::Zero, transform::{Transform, CanTransform}, r2::R2};

#[declare]
pub type Duals = Vec<Vec<f64>>;
#[declare]
pub type Input = (Shape<f64>, Duals);

#[derive(Debug, Display, Clone, From, PartialEq, Serialize, Deserialize, Tsify)]
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

impl<D: Clone> Shape<D> {
    pub fn c(&self) -> R2<D> {
        match self {
            Shape::Circle(c) => c.c.clone(),
            Shape::XYRR(e) => e.c.clone(),
        }
    }
}

impl Shape<D> {
    pub fn v(&self) -> Shape<f64> {
        match self {
            Shape::Circle(c) => Shape::Circle(c.v()),
            Shape::XYRR(e) => Shape::XYRR(e.v()),
        }
    }
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
}

impl<
    D
    : Clone
    + PartialEq
    + Eq
    + Mul<Output = D>
> CanTransform<D> for Shape<D>
where
    R2<D>
    :
    Add<Output = R2<D>>
    + Mul<Output = R2<D>>
    + Mul<D, Output = R2<D>>,
{
    type Output = Shape<D>;
    fn transform(&self, transform: &Transform<D>) -> Shape<D> {
        match self {
            Shape::Circle(c) => c.transform(transform),
            Shape::XYRR(e) => Shape::XYRR(e.transform(transform)),
        }
    }
}

pub type S = Rc<RefCell<Shape<D>>>;
