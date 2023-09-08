use std::{ops::{Div, Neg, Add, Mul}, f64::consts::PI, fmt::Display};

use crate::{dual::Dual, r2::R2, transform::{CanProject, HasProjection}, shape::Shape, trig::Trig, sqrt::Sqrt};

pub trait ThetaPoints<D> {
    fn theta(&self, p: R2<D>) -> D;
    fn point(&self, t: D) -> R2<D>;
    fn arc_midpoint(&self, t0: D, t1: D) -> R2<D>;
    fn contains(&self, p: &R2<D>) -> bool;
}

pub trait ThetaPointsArg
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

impl ThetaPointsArg for f64 {}
impl ThetaPointsArg for Dual {}

impl<D: ThetaPointsArg> ThetaPoints<D> for Shape<D>
where
    R2<D>: Neg<Output = R2<D>> + CanProject<D, Output = R2<D>>,
    f64: Div<D, Output = D>,
{
    fn theta(&self, p: R2<D>) -> D {
        p.apply(&self.projection()).atan2()
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