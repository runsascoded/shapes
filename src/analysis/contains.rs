use std::ops::{Add, Mul};

use crate::{r2::R2, shape::Shape, transform::{HasProjection, CanProject}, dual::Dual};


pub trait Contains<O> {
    fn contains(&self, o: &O) -> bool;
}

pub trait ShapeContainsPoint
: Clone
+ Into<f64>
+ Add<Output = Self>
+ Mul<Output = Self>
{}
impl ShapeContainsPoint for f64 {}
impl ShapeContainsPoint for Dual {}

impl<D: ShapeContainsPoint> Contains<R2<D>> for Shape<D>
where
    R2<D>: CanProject<D, Output = R2<D>>,
    Shape<D>: HasProjection<D>,
{
    fn contains(&self, p: &R2<D>) -> bool {
        p.apply(&self.projection()).norm2().into() <= 1.
    }
}
