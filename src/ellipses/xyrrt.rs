use std::{ops::{Neg, Sub, Mul, Div}, fmt::Display};

use derive_more::From;
use log::debug;
use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{r2::R2, rotate::{Rotate as _Rotate, RotateArg}, dual::{D, Dual}, shape::{Duals, Shape}, transform::{Transform::{Rotate, Scale, ScaleXY, Translate, self}, Projection, CanTransform, CanProject}, math::{recip::Recip, deg::Deg}};

use super::{xyrr::{XYRR, TransformD, TransformR2, UnitCircleGap, CdefArg}, cdef, bcdef::BCDEF};

#[derive(Debug, Clone, From, PartialEq, Serialize, Deserialize, Tsify)]
pub struct XYRRT<D> {
    pub c: R2<D>,
    pub r: R2<D>,
    pub t: D,
}

impl XYRRT<f64> {
    pub fn dual(&self, duals: &Duals) -> XYRRT<D> {
        let cx = Dual::new(self.c.x, duals[0].clone());
        let cy = Dual::new(self.c.y, duals[1].clone());
        let rx = Dual::new(self.r.x, duals[2].clone());
        let ry = Dual::new(self.r.y, duals[3].clone());
        let  t = Dual::new(self.t  , duals[4].clone());
        let c = R2 { x: cx, y: cy };
        let r = R2 { x: rx, y: ry };
        XYRRT::from((c, r, t))
    }
}

impl<D: RotateArg> XYRRT<D> {
    pub fn rotate(&self, t: &D) -> XYRRT<D> {
        XYRRT {
            c: self.c.rotate(t),
            r: self.r.clone(),
            t: self.t.clone() + t.clone(),
        }
    }
}

pub trait LevelArg: RotateArg + Neg<Output = Self> {}
impl<D: RotateArg + Neg<Output = D>> LevelArg for D {}

impl<D: LevelArg> XYRRT<D> {
    /// Rotate the plane so that this ellipse ends up aligned with the x- and y-axes (i.e. θ == B == 0)
    pub fn level(&self) -> XYRR<D> {
        XYRR {
            c: self.c.clone().rotate(&-self.t.clone()),
            r: self.r.clone(),
        }
    }
}

pub trait UnitIntersectionsArg: cdef::UnitIntersectionsArg + LevelArg + Deg {}
impl<D: cdef::UnitIntersectionsArg + LevelArg + Deg> UnitIntersectionsArg for D {}

impl<D: UnitIntersectionsArg> XYRRT<D>
where
    R2<D>: CanProject<D, Output = R2<D>>,
    f64
    : Sub<D, Output = D>
    + Mul<D, Output = D>
    + Div<D, Output = D>,
{
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        debug!("XYRRT.unit_intersections: {}", self);
        self.level().unit_intersections().iter().map(|p| p.rotate(&self.t)).collect()
    }
}

impl XYRRT<D> {
    pub fn v(&self) -> XYRRT<f64> {
        XYRRT { c: self.c.v(), r: self.r.v(), t: self.t.v() }
    }
    pub fn n(&self) -> usize {
        self.c.x.d().len()
    }
}

impl<D: Display + Deg> Display for XYRRT<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ c: {}, r: {} {}° }}", self.c, self.r, self.t.deg_str())
    }
}

impl<D: Clone + Neg<Output = D> + Recip> XYRRT<D>
where R2<D>: Neg<Output = R2<D>>,
{
    pub fn projection(&self) -> Projection<D> {
        Projection(vec![
            Translate(-self.c.clone()),
            Rotate(-self.t.clone()),
            ScaleXY(self.r.clone().recip()),
        ])
    }
}

impl<D: LevelArg + CdefArg> XYRRT<D> {
    fn bcdef(&self) -> BCDEF<D> {
        self.level().cdef().rotate(&self.t.clone())
    }
}

impl<D: TransformD> CanTransform<D> for XYRRT<D>
where R2<D>: TransformR2<D>,
{
    type Output = Shape<D>;
    fn transform(&self, t: &Transform<D>) -> Shape<D> {
        let rv = match t.clone() {
            Translate(v) => Shape::XYRRT(XYRRT {
                c: self.c.clone() + v,
                r: self.r.clone(),
                t: self.t.clone(),
            }),
            Scale(v) => Shape::XYRRT(XYRRT {
                c: self.c.clone() * v.clone(),
                r: self.r.clone() * v,
                t: self.t.clone(),
            }),
            ScaleXY(xy) => Shape::XYRRT(self.bcdef().scale_xy(&xy).xyrrt().rotate(&self.t)),
            Rotate(a) => Shape::XYRRT(self.rotate(&a)),
        };
        rv
    }
}

impl<D: LevelArg + UnitCircleGap> XYRRT<D> {
    pub fn unit_circle_gap(&self) -> Option<D> {
        self.level().unit_circle_gap()
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;
    use test_log::test;
    use crate::intersect::Intersect;

    #[test]
    fn test_level() {
        let e = XYRRT {
            c: R2 { x: 1., y: 1. },
            r: R2 { x: 2., y: 3. },
            t: PI / 4.,
        };

        let l = e.level();

        assert_relative_eq!(l.c.x, 2_f64.sqrt());
        assert_relative_eq!(l.c.y, 0.);
        assert_relative_eq!(l.r.x, 2.);
        assert_relative_eq!(l.r.y, 3.);
    }

    #[test]
    fn unit_intersections1() {
        let e = XYRRT {
            c: R2 { x: 1., y: 1. },
            r: R2 { x: 2., y: 3. },
            t: PI / 4.,
        };
        let points = e.unit_intersections();
        assert_eq!(points, vec![
            R2 { x: -0.9661421617927795 , y:  0.2580103160851765 },
            R2 { x:  0.25801031608517655, y: -0.9661421617927797 },
        ]);
    }

    #[test]
    fn unit_intersections2() {
        let e = XYRRT {
            c: R2 { x: 1., y: 1. },
            r: R2 { x: 1., y: 1. },
            t: PI / 4.,
        };
        let points = e.unit_intersections();
        assert_eq!(points, vec![
            R2 { x: -2.220446049250313e-16, y: 1.0 },
            R2 { x: 0.9999999999999999, y: -1.1102230246251565e-16 },
        ]);
    }

    #[test]
    fn unit_intersections3() {
        let e = XYRRT {
            c: R2 { x: 1., y: 0. },
            r: R2 { x: 1., y: 1. },
            t: PI / 2.,
        };
        let points = e.unit_intersections();
        assert_eq!(points, vec![
            R2 { x: 0.5, y: 0.8660254037844386 },
            R2 { x: 0.49999999999999994, y: -0.8660254037844386 },
        ]);
    }

    #[test]
    fn intersections1() {
        let e0 = XYRRT { c: R2 { x: 0., y: 0. }, r: R2 { x: 2., y: 1. }, t: 0. };
        let e1 = XYRRT { c: R2 { x: 0., y: 0. }, r: R2 { x: 2., y: 1. }, t: PI / 4. };
        let points = Shape::XYRRT(e0).intersect(&Shape::XYRRT(e1));
        assert_eq!(points, vec![

        ]);
    }
}