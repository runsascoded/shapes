use std::ops::{Sub, Add, Mul};

use crate::{r2::R2, sqrt::Sqrt, shape::Shape};

pub trait Distance<O> {
    type Output;
    fn distance(&self, o: &O) -> Self::Output;
}

impl<D: Clone + Add<Output = D> + Mul<Output = D> + Sqrt> Distance<R2<D>> for R2<D>
where R2<D>: Sub<Output = R2<D>>
{
    type Output = D;
    fn distance(&self, o: &R2<D>) -> D {
        (self.clone() - o.clone()).r()
    }
}

impl<D: Clone + Add<Output = D> + Mul<Output = D> + Sqrt> Distance<Shape<D>> for Shape<D>
where R2<D>: Sub<Output = R2<D>>
{
    type Output = D;
    fn distance(&self, o: &Shape<D>) -> D {
        self.center().distance(&o.center())
    }
}
