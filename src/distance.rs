use std::ops::{Sub, Mul, Add, Div, Neg};

use log::debug;

use crate::{circle::Circle, ellipses::xyrr::XYRR, r2::R2, sqrt::Sqrt, shape::Shape, intersect::{PointToTheta, PointToThetaArg}, transform::CanProject};

pub trait Distance<O> {
    type Output;
    fn distance(&self, o: &O) -> Option<Self::Output>;
}

impl<
    'a,
    D
    : 'a
    + Clone
    + Into<f64>
    + Sqrt
    + Add<Output = D>
    + Sub<Output = D>
    + Mul<Output = D>
> Distance<Circle<D>> for Circle<D>
where
    R2<D>
    : Sub<Output = R2<D>>
    + Sub<&'a R2<D>, Output = R2<D>>,
{
    type Output = D;
    fn distance(&self, o: &Circle<D>) -> Option<D> {
        let distance = (self.c.clone() - o.c.clone()).norm();
        let gap = distance - self.r.clone() - o.r.clone();
        let gap_f64: f64 = gap.clone().into();
        if gap_f64 > 0. {
            Some(gap)
        } else {
            None
        }
    }
}

impl<
    D
    : PointToThetaArg
    + Sub<Output = D>
> Distance<XYRR<D>> for Circle<D>
where
    R2<D>
    : Neg<Output = R2<D>>
    + Sub<Output = R2<D>>
    + CanProject<D, Output = R2<D>>,
    f64: Div<D, Output = D>,
{
    type Output = D;
    fn distance(&self, o: &XYRR<D>) -> Option<D> {
        self.xyrr().distance(o)
    }
}

impl<
    D
    : PointToThetaArg
    + Sub<Output = D>
> Distance<XYRR<D>> for XYRR<D>
where
    R2<D>
    : Neg<Output = R2<D>>
    + Sub<Output = R2<D>>
    + CanProject<D, Output = R2<D>>,
    f64: Div<D, Output = D>,
{
    type Output = D;
    fn distance(&self, o: &XYRR<D>) -> Option<D> {
        let t0 = Shape::XYRR(self.clone()).theta(o.c.clone());
        let distance = (self.c.clone() - o.c.clone()).norm();
        let p0 = Shape::XYRR(self.clone()).point(t0.clone());
        let p1 = Shape::XYRR(o.clone()).point(-t0);
        let radii = (p0.clone() - self.c.clone()).norm() + (p1.clone() - o.c.clone()).norm();
        let gap = distance.clone() - radii.clone();
        debug!("distance: {} - {} = {}", distance, radii, gap.clone());
        if gap.clone().into() > 0. {
            Some(gap)
        } else {
            None
        }
    }
}

impl<
    'a,
    D
    : 'a
    + PointToThetaArg
    // + Clone
    // + Into<f64>
    // + Sqrt
    // + Add<Output = D>
    + Sub<Output = D>
    // + Mul<Output = D>
> Distance<Shape<D>> for Shape<D>
where
    R2<D>
    : Sub<Output = R2<D>>
    + Sub<&'a R2<D>, Output = R2<D>>
    + Neg<Output = R2<D>>
    + CanProject<D, Output = R2<D>>,
    f64
    : Div<D, Output = D>,
{
    type Output = D;
    fn distance(&self, o: &Shape<D>) -> Option<D> {
        match (self, o) {
            (Shape::Circle(c0), Shape::Circle(c1)) => c0.distance(c1),
            (Shape::Circle(c0), Shape::XYRR(e)) => c0.distance(e),
            (Shape::XYRR(e), Shape::Circle(o)) => o.distance(e),
            (Shape::XYRR(e), Shape::XYRR(o)) => e.distance(o),
        }
    }
}