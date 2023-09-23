use std::{ops::{Div, Neg, Add, Mul, Sub}, fmt::Display};

use crate::{circle::{Circle, self}, dual::{D, Dual}, ellipses::cdef, r2::R2, transform::{CanProject, CanTransform, HasProjection}, shape::Shape, trig::Trig, theta_points::ThetaPointsArg, rotate::RotateArg};

pub trait Intersect<In, Out> {
    fn intersect(&self, other: &In) -> Vec<R2<Out>>;
}

pub trait IntersectShapesArg
: Clone
+ Display
+ PartialOrd
+ Trig
+ Neg<Output = Self>
+ cdef::UnitIntersectionsArg
+ circle::UnitIntersectionsArg
+ ThetaPointsArg
{}

impl IntersectShapesArg for f64 {}
impl IntersectShapesArg for Dual {}

impl<D: IntersectShapesArg> Intersect<Shape<D>, D> for Shape<D>
where
    R2<D>
    : Neg<Output = R2<D>>
    + CanTransform<D, Output = R2<D>>,
    Shape<D>
    : CanTransform<D, Output = Shape<D>>,
    f64
    : Add<D, Output = D>
    + Sub<D, Output = D>
    + Mul<D, Output = D>
    + Div<D, Output = D>,
{
    fn intersect(&self, o: &Shape<D>) -> Vec<R2<D>> {
        let projection = o.projection();
        let rev = -projection.clone();
        let projected = self.apply(&projection);
        // println!("Intersecting:");
        // println!("  self: {:?}", self);
        // println!("  other: {:?}", o);
        // println!("  projection: {:?}", projection);
        // println!("  projected: {:?}", projected);
        let unit_circle_intersections = projected.unit_circle_intersections();
        let points = unit_circle_intersections.iter().map(|p| p.apply(&rev));
        // println!("reverse projection: {:?}", rev);
        // println!("points: {:?}", points.clone().collect::<Vec<_>>());
        // println!();
        points.collect()
        // points.map(|p| {
        //     let x = p.x.clone();
        //     let y = p.y.clone();
        //     let p = R2 { x: x.clone(), y: y.clone() };
        //     let t0 = self.theta(p.clone());
        //     let t1 = o.theta(p.clone());
        //     Intersection { x, y, c0idx: self.idx(), c1idx: o.idx(), t0, t1, }
        // }).collect()
    }
}

pub trait UnitCircleIntersections<D> {
    fn unit_circle_intersections(&self) -> Vec<R2<D>>;
}

impl<
    D
    : cdef::UnitIntersectionsArg
    + circle::UnitIntersectionsArg
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
