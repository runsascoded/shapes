use std::ops::{Add, Mul};

use crate::{r2::R2, shape::Shape, transform::{HasProjection, CanProject}};


pub trait Contains<O> {
    fn contains(&self, o: &O) -> bool;
}

impl<
    D
    : Clone
    + Into<f64>
    + Add<Output = D>
    + Mul<Output = D>
> Contains<R2<D>> for Shape<D>
where
    R2<D>: CanProject<D, Output = R2<D>>,
    Shape<D>: HasProjection<D>,
{
    fn contains(&self, p: &R2<D>) -> bool {
        p.apply(&self.projection()).norm2().into() <= 1.
    }
}

impl<D> Contains<Shape<D>> for Shape<D> {
    fn contains(&self, o: &Shape<D>) -> bool {
        todo!()
        //self.contains(&o.p)
    }
}