use std::{ops::{Div, Neg, Add, Mul, Sub}, fmt::Display};

use log::debug;

use crate::{circle, dual::Dual, ellipses::{cdef, xyrrt}, r2::R2, transform::{CanProject, CanTransform, HasProjection}, shape::Shape, trig::Trig, theta_points::ThetaPointsArg, rotate::RotateArg};

pub trait Intersect<In, Out> {
    fn intersect(&self, other: &In) -> Vec<R2<Out>>;
    // fn _intersect(&self, other: &In) -> Vec<R2<Out>>;
}

pub trait IntersectShapesArg
: Clone
+ Display
+ PartialOrd
+ Trig
+ Neg<Output = Self>
+ cdef::UnitIntersectionsArg
+ circle::UnitIntersectionsArg
+ xyrrt::UnitIntersectionsArg
+ ThetaPointsArg
{}

impl IntersectShapesArg for f64 {}
impl IntersectShapesArg for Dual {}

impl<D: IntersectShapesArg> Intersect<Shape<D>, D> for Shape<D>
where
    R2<D>
    : Neg<Output = R2<D>>
    + CanTransform<D, Output = R2<D>>,
    Shape<D>: CanTransform<D, Output = Shape<D>>,
    f64
    : Add<D, Output = D>
    + Sub<D, Output = D>
    + Mul<D, Output = D>
    + Div<D, Output = D>,
{
    fn intersect(&self, o: &Shape<D>) -> Vec<R2<D>> {
        match (self, o) {
            (Shape::Circle(_), _) => self._intersect(o),
            (_, Shape::Circle(_)) => o.intersect(&self),
            (Shape::XYRR(_), _) => self._intersect(o),
            (_, Shape::XYRR(_)) => o.intersect(&self),
            (Shape::XYRRT(_), Shape::XYRRT(_)) => self._intersect(o),
        }
    }
}

pub trait UnitCircleIntersections<D> {
    fn unit_circle_intersections(&self) -> Vec<R2<D>>;
}

impl<
    D
    : cdef::UnitIntersectionsArg
    + circle::UnitIntersectionsArg
    + xyrrt::UnitIntersectionsArg
    + RotateArg
    + Neg<Output = D>
> UnitCircleIntersections<D> for Shape<D>
where
    R2<D>: CanProject<D, Output = R2<D>>,
    f64
    : Add<D, Output = D>
    + Sub<D, Output = D>
    + Mul<D, Output = D>
    + Div<D, Output = D>,
{
    fn unit_circle_intersections(&self) -> Vec<R2<D>> {
        match self {
            Shape::Circle(c) => c.unit_intersections(),
            Shape::XYRR(e) => e.unit_intersections(),
            Shape::XYRRT(e) => e.unit_intersections(),
        }
    }
}
