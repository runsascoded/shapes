use std::{ops::{Neg, Sub, Mul, Div}, fmt::Display};

use approx::{RelativeEq, AbsDiffEq};
use derive_more::From;
use log::debug;
use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{r2::R2, rotate::{Rotate as _Rotate, RotateArg}, dual::{D, Dual}, shape::{Duals, Shape}, transform::{Transform::{Rotate, Scale, ScaleXY, Translate, self}, Projection, CanTransform, CanProject}, math::{recip::Recip, deg::Deg}};

use super::{xyrr::{XYRR, TransformD, TransformR2, UnitCircleGap, CdefArg}, cdef, bcdef::{BCDEF, self}};

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
    pub fn duals(&self) -> [Vec<f64>; 5] {
        [ self.c.x.d(), self.c.y.d(), self.r.x.d(), self.r.y.d(), self.t.d() ]
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

impl<D: Deg + Display + LevelArg + CdefArg + cdef::RotateArg + bcdef::LevelArg> XYRRT<D> {
    fn bcdef(&self) -> BCDEF<D> {
        let level = self.level();
        let cdef = level.cdef();
        let bcdef = cdef.rotate(&self.t.clone());
        debug!("level: {}", level);
        debug!("cdef: {}", cdef);
        bcdef
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
            ScaleXY(xy) => {
                debug!("Scaling {}", self);
                let bcdef = self.bcdef();
                debug!("bcdef: {}", bcdef);
                let scaled = bcdef.scale_xy(&xy);
                debug!("scaled bcdef: {}", scaled);
                let xyrrt = scaled.xyrrt();
                debug!("scaled xyrrt: {}", xyrrt);
                Shape::XYRRT(xyrrt)
                // let rotated = xyrrt.rotate(&self.t);
                // debug!("rotated: {}", rotated);
                // Shape::XYRRT(rotated)
            },
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

impl<D: AbsDiffEq<Epsilon = f64> + Clone> AbsDiffEq for XYRRT<D> {
    type Epsilon = D::Epsilon;
    fn default_epsilon() -> Self::Epsilon {
        D::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.c.abs_diff_eq(&other.c, epsilon.clone())
        && self.r.abs_diff_eq(&other.r, epsilon.clone())
        && self.t.abs_diff_eq(&other.t, epsilon.clone())
    }
}

impl<D: RelativeEq<Epsilon = f64> + Clone> RelativeEq for XYRRT<D> {
    fn default_max_relative() -> Self::Epsilon {
        D::default_max_relative()
    }
    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        self.c.relative_eq(&other.c, epsilon.clone(), max_relative.clone())
        && self.r.relative_eq(&other.r, epsilon.clone(), max_relative.clone())
        && self.t.relative_eq(&other.t, epsilon.clone(), max_relative.clone())
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
        let e1 = XYRRT { c: R2 { x: 0., y: 0. }, r: R2 { x: 4., y: 1. }, t: PI / 4. };
        let points = Shape::XYRRT(e0).intersect(&Shape::XYRRT(e1));
        assert_eq!(points, vec![
            R2 { x:  1.76727585143249  , y:  0.4681709263035157 },
            R2 { x:  0.4311377406882232, y: -0.9764886390217573 },
            R2 { x: -0.4311377406882232, y:  0.9764886390217573 },
            R2 { x: -1.76727585143249  , y: -0.4681709263035157 },
        ]);
    }

    #[test]
    fn bcdef_roundtrip() {
        let e0 = XYRRT {
            c: R2 { x: 0., y: 0. },
            r: R2 { x: 2., y: 1. },
            t: PI / 4.,
        };
        let bcdef = e0.bcdef();
        debug!("bcdef: {}", bcdef);
        let e1 = bcdef.xyrrt();
        assert_relative_eq!(e0.c, e1.c, max_relative = 1e-15);
        assert_relative_eq!(e0.r, e1.r, max_relative = 1e-15);
        assert_relative_eq!(e0.t, e1.t, max_relative = 1e-15);
    }
}