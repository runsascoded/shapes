use std::{ops::{Mul, Div, Add, Sub, Neg}, fmt::Display};

use derive_more::From;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{r2::R2, rotate::{Rotate as _Rotate, RotateArg}, dual::{D, Dual}, shape::{Duals, Shape}, transform::{Transform::{Rotate, Scale, ScaleXY, Translate, self}, CanProject, CanTransform, Projection}, math::recip::Recip, sqrt::Sqrt, ellipses::xyrr};

use super::{xyrrt::XYRRT, cdef::{CDEF, self}, bcdef};

#[derive(Debug, Clone, From, PartialEq, Serialize, Deserialize, Tsify)]
pub struct XYRR<D> {
    pub c: R2<D>,
    pub r: R2<D>,
}

impl XYRR<f64> {
    pub fn dual(&self, duals: &Duals) -> XYRR<D> {
        let cx = Dual::new(self.c.x, duals[0].clone());
        let cy = Dual::new(self.c.y, duals[1].clone());
        let rx = Dual::new(self.r.x, duals[2].clone());
        let ry = Dual::new(self.r.y, duals[3].clone());
        let c = R2 { x: cx, y: cy };
        let r = R2 { x: rx, y: ry };
        XYRR::from((c, r))
    }
}

impl<D: RotateArg> XYRR<D> {
    pub fn rotate(&self, t: &D) -> XYRRT<D> {
        XYRRT {
            c: self.c.clone().rotate(t),
            r: self.r.clone(),
            t: t.clone(),
        }
    }
}

pub trait CdefArg
: Clone
+ Add<Output = Self>
+ Sub<Output = Self>
+ Mul<Output = Self>
+ Mul<f64, Output = Self>
+ Div<Output = Self>
{}
impl<
    D
    : Clone
    + Add<Output = D>
    + Sub<Output = D>
    + Mul<Output = D>
    + Mul<f64, Output = D>
    + Div<Output = D>
> CdefArg for D {}

impl<D: CdefArg> XYRR<D>
{
    pub fn cdef(&self) -> CDEF<D> {
        let rx2 = self.r.x.clone() * self.r.x.clone();
        let rr = self.r.x.clone() / self.r.y.clone();
        let rr2 = rr.clone() * rr.clone();
        CDEF {
            c: rr2.clone(),
            d: self.c.x.clone() * -2.,
            e: self.c.y.clone() * rr2.clone() * -2.,
            f: self.c.x.clone() * self.c.x.clone() + self.c.y.clone() * self.c.y.clone() * rr2.clone() - rx2.clone(),
        }
    }
}
impl<D: cdef::UnitIntersectionsArg + CdefArg> XYRR<D>
where R2<D>: CanProject<D, Output = R2<D>>,
{
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        debug!("XYRR.unit_intersections: {}", self);
        let points = self.cdef().unit_intersections(&self);
        debug!("XYRR.unit_intersections: {}, points {:?}", self, points);
        for point in &points {
            let r = point.norm();
            let projected = point.apply(&self.projection());
            let self_r = projected.norm();
            let err = (-1. + self_r.clone().into()).abs();
            if err > 1e-2 {
                if err > 0.1 {
                    panic!("Bad unit_intersections: {}\npoint: {}\nunit.r: {}\nself.r: {}", self, point, r, self_r);
                } else {
                    warn!("Bad unit_intersections: {}\npoint: {}\nunit.r: {}\nself.r: {}", self, point, r, self_r);
                }
            }
            // debug!("  point: {}, unit.r: {}, self.r: {}", point, r, self_r);
        }
        points
    }
}

impl XYRR<D> {
    pub fn v(&self) -> XYRR<f64> {
        XYRR { c: self.c.v(), r: self.r.v() }
    }
    pub fn n(&self) -> usize {
        self.c.x.d().len()
    }
}

impl<D: Display> Display for XYRR<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ c: {}, r: {} }}", self.c, self.r)
    }
}

impl<D: Clone + Recip> XYRR<D>
where R2<D>: Neg<Output = R2<D>>,
{
    pub fn projection(&self) -> Projection<D> {
        Projection(vec![
            Translate(-self.c.clone()),
            ScaleXY(self.r.clone().recip()),
        ])
    }
}

// pub trait TransformR2
// : Add<Output = Self>
// + Mul<Output = Self>
// {}

pub trait TransformR2<D>
: Add<R2<D>, Output = R2<D>>
+ Mul<R2<D>, Output = R2<D>>
+ Mul<D, Output = R2<D>>
{}
impl TransformR2<f64> for R2<f64> {}
impl TransformR2<Dual> for R2<Dual> {}

pub trait TransformD: Clone + Display + Div<Output = Self> + RotateArg + xyrr::CdefArg + bcdef::XyrrtArg {}
impl<D: Clone + Display + Div<Output = D> + RotateArg + xyrr::CdefArg + bcdef::XyrrtArg> TransformD for D {}

impl<D: TransformD> CanTransform<D> for XYRR<D>
where R2<D>: TransformR2<D>,
{
    type Output = Shape<D>;
    fn transform(&self, t: &Transform<D>) -> Shape<D> {
        let rv = match t.clone() {
            Translate(v) => Shape::XYRR(XYRR {
                c: self.c.clone() + v,
                r: self.r.clone(),
            }),
            Scale(v) => Shape::XYRR(XYRR {
                c: self.c.clone() * v.clone(),
                r: self.r.clone() * v,
            }),
            ScaleXY(xy) => Shape::XYRR(self.scale_xy(xy)),
            Rotate(a) => Shape::XYRRT(self.rotate(&a)),
        };
        rv
    }
}

impl<D> XYRR<D>
where R2<D>: Clone + Mul<R2<D>, Output = R2<D>>
{
    pub fn scale_xy(&self, xy: R2<D>) -> XYRR<D> {
        XYRR {
            c: self.c.clone() * xy.clone(),
            r: self.r.clone() * xy,
        }
    }
}

pub trait UnitCircleGap
: Clone
+ Into<f64>
+ PartialOrd
+ Sqrt
+ Add<Output = Self>
+ Sub<f64, Output = Self>
+ Sub<Output = Self>
+ Mul<Output = Self>
{}
impl UnitCircleGap for f64 {}
impl UnitCircleGap for Dual {}

impl<D: UnitCircleGap> XYRR<D> {
    pub fn unit_circle_gap(&self) -> Option<D> {
        let r = &self.r;
        let distance = self.c.norm() - 1. - (if r.x < r.y { r.x.clone() } else { r.y.clone() });
        if distance.clone().into() > 0. {
            Some(distance)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt, f64::NAN};

    use crate::{dual::{Dual, d_fns}, circle::Circle, intersect::Intersect, to::To, shape::Shape};

    use super::*;
    use approx::{AbsDiffEq, RelativeEq};
    use ordered_float::OrderedFloat;
    use test_log::test;

    #[derive(Clone, Debug, PartialEq)]
    pub struct Points<D>(pub Vec<R2<D>>);
    impl<D: Clone + Into<f64> + AbsDiffEq<Epsilon = f64>> AbsDiffEq for Points<D> {
        type Epsilon = f64;
        fn default_epsilon() -> f64 {
            f64::default_epsilon()
        }
        fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
            let mut actual = self.0.clone();
            actual.sort_by_key(|p| (OrderedFloat(p.x.clone().into()), OrderedFloat(p.y.clone().into())));
            let mut expected = other.0.clone();
            expected.sort_by_key(|p| (OrderedFloat(p.x.clone().into()), OrderedFloat(p.y.clone().into())));
            actual.abs_diff_eq(&expected, epsilon)
        }
    }
    impl<D: Clone + Into<f64> + RelativeEq<Epsilon = f64>> RelativeEq for Points<D>
    {
        fn default_max_relative() -> f64 {
            f64::default_max_relative()
        }
        fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
            let mut actual = self.0.clone();
            actual.sort_by_key(|p| (OrderedFloat(p.x.clone().into()), OrderedFloat(p.y.clone().into())));
            let mut expected = other.0.clone();
            expected.sort_by_key(|p| (OrderedFloat(p.x.clone().into()), OrderedFloat(p.y.clone().into())));
            actual.relative_eq(&expected, epsilon, max_relative)
        }
    }

    fn check<D: Clone + fmt::Debug + Into<f64> + RelativeEq<Epsilon = f64>>(actual: Vec<R2<D>>, expected: [R2<D>; 2], max_epsilon: f64) {
        let actual = Points(actual);
        let expected = Points(expected.to_vec());
        assert_relative_eq!(actual, expected, max_relative = max_epsilon);
    }

    #[test]
    fn test_unit_intersections_d() {
        let e: XYRR<f64> = Circle { c: R2 { x: 0., y: -1. }, r: 1. }.to();
        let points = e.unit_intersections();
        check(points, [
            R2 { x: -0.8660254037844386, y: -0.5 },
            R2 { x:  0.8660254037844386, y: -0.5 },
        ], 1e-15);
    }

    #[test]
    fn test_perturbed_unit_circle1() {
        // Quartic solvers struggle to retain accuracy computing this slightly-perturbed unit-circle
        let e = XYRR {
            c: R2 { x: -1.100285308561806, y: -1.1500279763995946e-5 },
            r: R2 { x:  1.000263820108834, y:  1.0000709021402923 }
        };
        let points = e.unit_intersections();
        check(points, [
            // TODO: these are not super accurate, tiny x⁴ and x³ coefficients introduce numerical error, some semblance of accuracy is recovered with some kludgy checks in CDEF::unit_intersections, but better root-refinement / -finding algos would be good.
            R2 { x: -0.5500509220473759, y:  0.835131117343158  },
            R2 { x: -0.5499993628836819, y: -0.8351650739988736 },
        ], 1e-15);
    }

    #[test]
    fn test_perturbed_unit_circle2() {
        // Quartic solvers struggle to retain accuracy computing this slightly-perturbed unit-circle
        let e = XYRR {
            c: R2 { x: -1.1, y: -1e-5 },
            r: R2 { x:  1.0002, y:  1.00007 }
        };
        let points = e.unit_intersections();
        check(points, [
            // TODO: these are not super accurate, tiny x⁴ and x³ coefficients introduce numerical error, some semblance of accuracy is recovered with some kludgy checks in CDEF::unit_intersections, but better root-refinement / -finding algos would be good.
            R2 { x: -0.5499644615556463, y: 0.835188057281597 },
            R2 { x: -0.5498367543659697, y: -0.8352721374188752 },
        ], 1e-15);
    }

    #[test]
    fn test_unit_intersections_1_1_2_3() {
        let e = XYRR {
            c: R2 { x: Dual::new(1., vec![1.,0.,0.,0.]),
                    y: Dual::new(1., vec![0.,1.,0.,0.]), },
            r: R2 { x: Dual::new(2., vec![0.,0.,1.,0.]),
                    y: Dual::new(3., vec![0.,0.,0.,1.]), },
        };
        let points = e.unit_intersections();
        check(points, [
            R2 { x: Dual::new(-0.6000000000000008 , vec![  1.6000000000000023,  0.7999999999999998 , -1.2800000000000025, -0.48                ]),
                 y: Dual::new(-0.7999999999999974 , vec![ -1.2000000000000064, -0.6000000000000028 ,  0.9600000000000062,  0.360000000000003   ]), },
            R2 { x: Dual::new(-0.9477785230650169 , vec![  0.6840738256449916,  0.10630975246195229, -0.6662121528911189, -0.0241348206854628  ]),
                 y: Dual::new( 0.31892925738585376, vec![  2.0328974690234993,  0.3159261743550079 , -1.9798170148786007, -0.07172269139307064 ]), },
        ], 1e-15);
    }

    #[test]
    fn test_unit_intersections_1_n1_1_1() {
        let e = XYRR {
            c: R2 { x: Dual::new( 1., vec![1.,0.,0.,0.]),
                    y: Dual::new(-1., vec![0.,1.,0.,0.]), },
            r: R2 { x: Dual::new( 1., vec![0.,0.,1.,0.]),
                    y: Dual::new( 1., vec![0.,0.,0.,1.]), },
        };
        let points = e.unit_intersections();
        check(points, [
            R2 { x: Dual::new( 1.000, vec![ 0.000,  0.000,  0.000,  0.000]),
                 y: Dual::new( 0.000, vec![ 0.000,  1.000,  0.000,  1.000]), },
            R2 { x: Dual::new( 0.000, vec![ 1.000,  0.000, -1.000,  0.000]),
                 y: Dual::new(-1.000, vec![ 0.000,  0.000,  0.000,  0.000]), },
        ], 1e-3);
    }

    #[test]
    fn circle_l() {
        let e = XYRR {
            c: R2 { x: -1.4489198414355153, y: 0.               , },
            r: R2 { x:  1.29721671027373  , y: 1.205758072744277, },
        };
        let points = e.unit_intersections();

        // WRONG: find_roots_sturm: missing other half (+y)
        // assert_eq!(points, [ R2 { x: -0.5280321051800396, y: -0.8492244095691155 }, ] );

        // OK: find_roots_eigen, crate::math::quartic::quartic
        check(points, [
            R2 { x: -0.5280321050819483, y: 0.8492244085062177 },
            R2 { x: -0.5280321050819483, y: -0.8492244085062177 },
        ], 1e-14);
    }

    #[test]
    fn tangent_circles() {
        let ( z, d ) = d_fns(1);
        let ellipses = [
            Shape::XYRR(XYRR { c: R2 { x: 0., y: 0. }, r: R2 { x: 2., y: 2., } }.dual(&vec![ z( ), z( ), z( ), z( ) ])),
            Shape::XYRR(XYRR { c: R2 { x: 3., y: 0. }, r: R2 { x: 1., y: 1., } }.dual(&vec![ d(0), z( ), z( ), z( ) ])),
        ];
        let [ e0, e1 ] = ellipses;
        let ps = e0.intersect(&e1);
        assert_eq!(ps, vec![
            // Heads up! Tangent points have NAN gradients. [`Scene`] detects/filters them, but at this level of the stack they are passed along
            R2 { x: Dual::new(2.0, vec![NAN]), y: Dual::new(0.0, vec![NAN]) },
            R2 { x: Dual::new(2.0, vec![NAN]), y: Dual::new(0.0, vec![NAN]) }
        ]);
    }

    #[test]
    fn ellipses4_0_2() {
        let e = XYRR { c: R2 { x: -0.6708203932499369, y: 0.34164078649987384 }, r: R2 { x: 0.5, y: 2.0 } };
        let points = e.unit_intersections();
        check(points, [
            R2 { x: -0.19700713608839127, y:  0.9804020544298395 },
            R2 { x: -0.29053402101498915, y: -0.9568646626523847 },
        ], 1e-10);
    }

    #[test]
    fn ellipses4_0_1() {
        let e0 = XYRR { c: R2 { x: 0.4472135954999579, y: 1.7888543819998317 }, r: R2 { x: 1.,                y: 2. } };
        let e1 = XYRR { c: R2 { x: 1.447213595499943 , y: 1.7888543819998468 }, r: R2 { x: 1.000000000000004, y: 1.9999999999999956 } };
        let points = Shape::XYRR(e0).intersect(&Shape::XYRR(e1));
        debug!("points: {:?}", points);
        assert_eq!(points, vec![
            R2 { x: 0.9472135954999445, y: 3.5209051895687242   },
            R2 { x: 0.9472135954999577, y: 0.056803574430954074 },
        ]);
    }

    #[test]
    fn fizz_buzz_bazz_step2() {
        let [ e0, e1 ] = [
            XYRR { c: R2 { x: 0.9640795213008657, y: 0.14439141636463643 }, r: R2 { x: 1.022261085060523, y: 1.0555121335932487 } },
            XYRR { c: R2 { x: 0.1445171704183926, y: 0.964205275354622   }, r: R2 { x: 0.9998075759976, y: 0.9049240408142587 } },
        ];
        let shapes = [ e0, e1 ].map(|e| Shape::XYRR(e));
        let points = shapes[0].intersect(&shapes[1]);
        assert_eq!(points, vec![
            R2 { x:  1.1130898341623665 , y: 1.1886298166805425  },
            R2 { x: -0.05613854711079608, y: 0.07769290045110777 },
        ]);
    }
}
