use std::{ops::{Div, Neg, Add, Mul, Sub}, f64::consts::PI, fmt::Display};

use crate::{intersection::Intersection, circle::{Circle, self}, dual::{D, Dual}, ellipses::cdef, r2::R2, transform::{CanProject, CanTransform, HasProjection}, shape::Shape, trig::Trig, sqrt::Sqrt};

pub trait Intersect<In, Out> {
    fn intersect(&self, other: &In) -> Vec<Intersection<Out>>;
}

impl Intersect<Circle<f64>, D> for Circle<f64> {
    fn intersect(&self, o: &Circle<f64>) -> Vec<Intersection<D>> {
        let c0 = self.dual(&vec![ vec![ 1., 0., 0., 0., 0., 0., ], vec![ 0., 1., 0., 0., 0., 0., ], vec![ 0., 0., 1., 0., 0., 0., ] ]);
        let c1 =    o.dual(&vec![ vec![ 0., 0., 0., 1., 0., 0., ], vec![ 0., 0., 0., 0., 1., 0., ], vec![ 0., 0., 0., 0., 0., 1., ] ]);
        let s0 = Shape::Circle(c0);
        let s1 = Shape::Circle(c1);
        s0.intersect(&s1)
    }
}

impl<
    D
    : Clone
    + Trig
    + Neg<Output = D>
    + cdef::UnitIntersectionsArg
    + circle::UnitIntersectionsArg
    + PointToThetaArg
> Intersect<Shape<D>, D> for Shape<D>
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
    fn intersect(&self, o: &Shape<D>) -> Vec<Intersection<D>> {
        println!("{} intersect {}", self.idx(), o.idx());
        let projection = o.projection();
        let rev = -projection.clone();
        let projected = self.apply(&projection);
        let unit_circle_intersections = projected.unit_circle_intersections();
        println!("fwd: {:?}", projection);
        println!("rev: {:?}", rev);
        let points = unit_circle_intersections.iter().map(|p| p.apply(&rev));
        println!("{} uci, {} points", unit_circle_intersections.len(), points.len());
        let intersections: Vec<_> = points.map(|p| {
            println!("point: {}", p);
            let x = p.x.clone();
            let y = p.y.clone();
            let p = R2 { x: x.clone(), y: y.clone() };
            let t0 = self.theta(p.clone());
            let t1 = o.theta(p.clone());
            Intersection { x, y, c0idx: self.idx(), c1idx: o.idx(), t0, t1, }
        }).collect();
        println!("{} intersected {}: {} intersections", self.idx(), o.idx(), intersections.len());
        intersections
    }
}

pub trait UnitCircleIntersections<D> {
    fn unit_circle_intersections(&self) -> Vec<R2<D>>;
}

impl<
    D
    : cdef::UnitIntersectionsArg
    + circle::UnitIntersectionsArg
> UnitCircleIntersections<D> for Shape<D>
where
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
        }
    }
}

pub trait PointToTheta<D> {
    fn theta(&self, p: R2<D>) -> D;
    fn point(&self, t: D) -> R2<D>;
    fn arc_midpoint(&self, t0: D, t1: D) -> R2<D>;
    fn contains(&self, p: &R2<D>) -> bool;
}

pub trait PointToThetaArg
: Clone
+ Display
+ Neg<Output = Self>
+ Into<f64>
+ Sqrt
+ Trig
+ PartialOrd
+ Add<Output = Self>
+ Add<f64, Output = Self>
+ Mul<Output = Self>
+ Div<f64, Output = Self>
{}

impl PointToThetaArg for f64 {}
impl PointToThetaArg for Dual {}

impl<D: PointToThetaArg> PointToTheta<D> for Shape<D>
where
    R2<D>: Neg<Output = R2<D>> + CanProject<D, Output = R2<D>>,
    f64: Div<D, Output = D>,
{
    fn theta(&self, p: R2<D>) -> D {
        match self {
            Shape::Circle(c) => p.apply(&c.projection()).atan2(),
            Shape::XYRR(e) => p.apply(&e.projection()).atan2(),
        }
    }
    fn point(&self, t: D) -> R2<D> {
        let unit_point = R2 { x: t.cos(), y: t.sin() };
        unit_point.apply(&-self.projection())
    }
    fn arc_midpoint(&self, t0: D, t1: D) -> R2<D> {
        let t1 = if t1 < t0 { t1 + 2. * PI } else { t1 };
        let t = (t0 + t1) / 2.;
        self.point(t)
    }
    fn contains(&self, p: &R2<D>) -> bool {
        p.apply(&self.projection()).norm().into() <= 1.
    }
}