
use std::{fmt::Display, ops::{Add, Mul, Sub, Div, Neg}};

use approx::{AbsDiffEq, RelativeEq};
use log::debug;
use ordered_float::OrderedFloat;

use crate::{math::{abs::AbsArg, is_zero::IsZero, recip::Recip}, r2::R2, transform::CanProject, ellipses::quartic::{Root, Quartic}, circle, dual::Dual, sqrt::Sqrt, trig::Trig};

use super::{xyrr::XYRR, bcdef::BCDEF};

/// Ellipse where "A" (the x² coefficient) is 1 and "B" (the xy coefficient) is zero:
///
/// x² + Cy² + Dx + Ey + F = 0
///
/// This means the ellipse is aligned with the x- and y-axes, which makes computing unit-circle intersections easier (the axis-alignment rotation can then be reverted, yielding the original (unrotated) ellipse's unit-circle intersections).
///
/// Ellipse-ellipse intersections are computed via the following steps:
/// 1. Project the plane so that one ellipse becomes a unit circle.
/// 2. Rotate the plane so that the other ellipse becomes axis-aligned (i.e. B == 0).
/// 3. Compute intersections of the axis-aligned ellipse with the unit circle.
/// 4. Revert 2. (rotate the plane back to its original orientation).
/// 5. Revert 1. (invert the projection).
#[derive(Debug, Clone, PartialEq)]
pub struct CDEF<D> {
    pub c: D,
    pub d: D,
    pub e: D,
    pub f: D,
}

pub trait UnitIntersectionsArg
: AbsArg
+ Display
+ IsZero
+ Quartic
+ Recip
+ circle::UnitIntersectionsArg
{}

impl UnitIntersectionsArg for f64 {}
impl UnitIntersectionsArg for Dual {}

impl<D: UnitIntersectionsArg> CDEF<D>
where R2<D>: CanProject<D, Output = R2<D>>
{
    pub fn points_err(&self, points: Vec<R2<D>>, xyrr: &XYRR<D>) -> f64 {
        points.iter().map(|p| {
            let r0: f64 = p.norm().into();
            let log_err0 = r0.ln().abs();
            let r1: f64 = p.apply(&xyrr.projection()).norm().into();
            let log_err1 = r1.ln().abs();
            // debug!("  point: {}, r0: {} ({}), r1: {} ({})", p, r0, log_err0, r1, log_err1);
            log_err0 + log_err1
        }).sum()
    }
    pub fn unit_intersections(&self, xyrr: &XYRR<D>) -> Vec<R2<D>> {
        let CDEF { c, d, e, f } = self;
        debug!("C: {}", c);
        debug!("D: {}", d);
        debug!("E: {}", e);
        debug!("F: {}", f);
        let d_zero = d.clone().is_zero();
        let e_zero = e.clone().is_zero();
        if d_zero {
            if e_zero {
                let fc = (-f.clone() - 1.) / (c.clone() - 1.);
                let fcf: f64 = fc.clone().into();
                if fcf >= 0. && fcf <= 1. {
                    let y0 = fc.sqrt();
                    let y1 = -y0.clone();
                    let x0 = (-y0.clone() * y0.clone() + 1.).sqrt();
                    let x1 = -x0.clone();
                    let points = vec![
                        R2 { x: x0.clone(), y: y0.clone() },
                        R2 { x: x0.clone(), y: y1.clone() },
                        R2 { x: x1.clone(), y: y0.clone() },
                        R2 { x: x1.clone(), y: y1.clone() },
                    ];
                    points
                } else {
                    vec![]
                }
            } else {
                let points = self._unit_intersections(xyrr, true);
                // let err = self.points_err(points.clone(), xyrr);
                // debug!("points err: {}", err);
                points
            }
        } else if e_zero {
            let points = self._unit_intersections(xyrr, false);
            // let err = self.points_err(points.clone(), xyrr);
            // debug!("points err: {}", err);
            points
        } else {
            let points0 = self._unit_intersections(xyrr, true);
            let err0 = self.points_err(points0.clone(), xyrr);
            let points1 = self._unit_intersections(xyrr, false);
            let err1 = self.points_err(points1.clone(), xyrr);
            // debug!("points errs: {} vs. {}", err0, err1);
            if points0.len() == 0 {
                points1
            } else if points1.len() == 0 {
                points0
            } else if err0 < err1 {
                points0
            } else {
                points1
            }
        }
    }
    pub fn _unit_intersections(&self, xyrr: &XYRR<D>, sub_y_solve_x: bool) -> Vec<R2<D>> {
        debug!("_unit_intersections, sub_y_solve_x: {}", sub_y_solve_x);
        let [ c_2, c_1, c_0 ] = if sub_y_solve_x {        // debug!("d_zero: {}", d_zero);
            let re = -self.e.clone().recip();
            [
                (-self.c.clone() + 1.) * re.clone(),
                self.d.clone() * re.clone(),
                (self.c.clone() + self.f.clone()) * re,
            ]
        } else {
            let rd = -self.d.clone().recip();
            [
                (self.c.clone() - 1.) * rd.clone(),
                self.e.clone() * rd.clone(),
                (self.f.clone() + 1.) * rd,
            ]
        };
        debug!("c_2: {}", c_2);
        debug!("c_1: {}", c_1);
        debug!("c_0: {}", c_0);

        let mut a_4 = c_2.clone() * c_2.clone();
        let mut a_3 = c_2.clone() * c_1.clone() * 2.;
        let a_2 = c_1.clone() * c_1.clone() + c_2.clone() * c_0.clone() * 2. + 1.;
        let a_1 = c_1.clone() * c_0.clone() * 2.;
        let a_0 = c_0.clone() * c_0.clone() - 1.;
        debug!("a_4: {}", a_4);
        debug!("a_3: {}", a_3);
        debug!("a_2: {}", a_2);
        debug!("a_1: {}", a_1);
        debug!("a_0: {}", a_0);
        let f_4: f64 = a_4.clone().into();
        let f_3: f64 = a_3.clone().into();
        let f_2: f64 = a_2.clone().into();
        // Very small a_4/a_3 coefficients can lead to significant numeric errors attempting to solve as quartic/cubic, just treat these as cubic/quadratic.
        if f_2 != 0. && (f_4 / f_2).abs() < 1e-8 && (f_3 / f_2).abs() < 1e-8 {
            // debug!("Setting a_4 and a_3 to 0.");
            let f: f64 = a_4.clone().into();
            a_4 = a_4 - f;
            let f: f64 = a_3.clone().into();
            a_3 = a_3 - f;
            // debug!("Set a_4 and a_3 to 0: {}, {}", a_4, a_3);
        }
        // if a_4.clone().into().abs() < 1e-7 {
        //     debug!("Setting a_4 to 0.");
        //     let f: f64 = a_4.clone().into();
        //     a_4 = a_4 - f;
        //     debug!("Set a_4 to 0: {}", a_4);
        // }
        // if a_3.clone().into().abs() < 1e-7 {
        //     let f: f64 = a_3.clone().into();
        //     a_3 = a_3 - f;
        //     debug!("Set a_3 to 0: {}", a_3);
        // }
        let roots = Quartic::quartic_roots(a_4, a_3, a_2, a_1, a_0);
        let mut points: Vec<R2<D>> = Vec::new();
        // debug!("Points:");
        for Root(r0, double_root) in &roots {
            let r1_0 = c_2.clone() * r0.clone() * r0.clone() + c_1.clone() * r0.clone() + c_0.clone();
            let p = if sub_y_solve_x {
                R2 { x: r0.clone(), y: r1_0.clone() }
            } else {
                R2 { x: r1_0.clone(), y: r0.clone() }
            };
            let p = if (-1. + p.norm2().into()).abs() <= 1e-3 {
                p
            } else {
                let r1_1p = (-r0.clone() * r0.clone() + 1.).sqrt();
                let r1_1n = -r1_1p.clone();
                let r0_p = (-r1_0.clone() * r1_0.clone() + 1.).sqrt();
                let r0_n = -r0_p.clone();
                let candidates = [
                    ( r0.clone(), r1_0.clone() ),
                    ( r0.clone(), r1_1p ),
                    ( r0.clone(), r1_1n ),
                    ( r0_p, r1_0.clone() ),
                    ( r0_n, r1_0.clone() ),
                ];
                let candidates: Vec<R2<D>> = candidates.into_iter().filter(|(c0, c1)| {
                    let f0: f64 = c0.clone().into();
                    let f1: f64 = c1.clone().into();
                    !f0.is_nan() && !f1.is_nan()
                }).map(|(c0, c1)|
                    if sub_y_solve_x {
                        R2 { x: c0, y: c1 }
                    } else {
                        R2 { x: c1, y: c0 }
                    }
                ).collect();
                // debug!("Comparing candidate points:");
                // for p in &candidates {
                //     let r1 = p.clone().apply(&xyrr.projection());
                //     let r1_err = (-1. + r1.clone().norm2().into()).abs();
                    // debug!("  p: {}, r1: {} ({})", p, r1, r1_err);
                // }
                let p =
                    candidates
                    .into_iter()
                    .min_by_key(|p| {
                        let n: f64 = p.apply(&xyrr.projection()).norm2().into();
                        OrderedFloat((-1. + n).abs())  // TODO: `where f64: Sub<D, Output = D>` prevents subtracting f64's from one another?
                    })
                    .unwrap();
                p
            };

            let R2 { x, y } = p.clone();
            // debug!("Using point: x {}, y {}", x, y);
            points.push(p);
            if *double_root {
                let p2 = if sub_y_solve_x {
                    R2 { x: -x.clone(), y:  y.clone() }
                } else {
                    R2 { x:  x.clone(), y: -y.clone() }
                };
                // debug!("Double-root point: {:?}", p2.clone());
                points.push(p2);
            }
        }
        points
    }
}

pub trait RotateArg
: Clone
+ Recip
+ Trig
+ Add<Output = Self>
+ Sub<Output = Self>
+ Sub<f64, Output = Self>
+ Mul<Output = Self>
+ Mul<f64, Output = Self>
{}
impl<
    D
    : Clone
    + Recip
    + Trig
    + Add<Output = D>
    + Sub<Output = D>
    + Sub<f64, Output = D>
    + Mul<Output = D>
    + Mul<f64, Output = D>
> RotateArg for D {}

impl<D: RotateArg> CDEF<D> {
    pub fn rotate(&self, t: &D) -> BCDEF<D> {
        let cos = t.cos();
        let cos2 = cos.clone() * cos.clone();
        let sin = t.sin();
        let sin2 = sin.clone() * sin.clone();
        let CDEF { c, d, e, f } = self;
        let a = cos2.clone() + c.clone() * sin2.clone();
        let ar = a.recip();
        BCDEF {
            b: ar.clone() * -2. * cos.clone() * sin.clone() * (c.clone() - 1.),
            c: ar.clone() * (sin2.clone() + c.clone() * cos2.clone()),
            d: ar.clone() * (d.clone() * cos.clone() - e.clone() * sin.clone()),
            e: ar.clone() * (d.clone() * sin.clone() + e.clone() * cos.clone()),
            f: ar * f.clone(),
            t: t.clone(),
        }
    }
}

pub trait XyrrArg
: Clone
+ Sqrt
+ Add<Output = Self>
+ Sub<Output = Self>
+ Mul<Output = Self>
+ Mul<f64, Output = Self>
+ Div<Output = Self>
+ Div<f64, Output = Self>
{}
impl<
    D
    : Clone
    + Sqrt
    + Add<Output = D>
    + Sub<Output = D>
    + Mul<Output = D>
    + Mul<f64, Output = D>
    + Div<Output = D>
    + Div<f64, Output = D>
> XyrrArg for D {}

impl<D: XyrrArg> CDEF<D> {
    pub fn xyrr(&self) -> XYRR<D> {
        let CDEF { c, d, e, f } = self;
        let rx = (d.clone() * d.clone() + e.clone() * e.clone() / c.clone() - f.clone() * 4.).sqrt() / 2.;
        let ry = rx.clone() / c.sqrt();
        let cx = d.clone() / -2.;
        let cy = e.clone() / -2. / c.clone();
        XYRR { c: R2 { x: cx, y: cy }, r: R2 { x: rx, y: ry } }
    }
}

impl<D: Clone + Display + Neg<Output = D>> Display for CDEF<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x² + {}y² + {}x + {}y = {}", self.c, self.d, self.e, -self.f.clone())
    }
}

impl<D: AbsDiffEq<Epsilon = f64> + Clone> AbsDiffEq for CDEF<D> {
    type Epsilon = D::Epsilon;
    fn default_epsilon() -> Self::Epsilon {
        D::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.c.abs_diff_eq(&other.c, epsilon.clone())
        && self.d.abs_diff_eq(&other.d, epsilon.clone())
        && self.e.abs_diff_eq(&other.e, epsilon.clone())
        && self.f.abs_diff_eq(&other.f, epsilon)
    }
}

impl<D: RelativeEq<Epsilon = f64> + Clone> RelativeEq for CDEF<D> {
    fn default_max_relative() -> Self::Epsilon {
        D::default_max_relative()
    }
    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        self.c.relative_eq(&other.c, epsilon.clone(), max_relative.clone())
        && self.d.relative_eq(&other.d, epsilon.clone(), max_relative.clone())
        && self.e.relative_eq(&other.e, epsilon.clone(), max_relative.clone())
        && self.f.relative_eq(&other.f, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use crate::ellipses::xyrrt::XYRRT;

    use super::*;
    use test_log::test;

    #[test]
    fn rotate_roundtrip() {
        let cdef = CDEF { c: 4., d: 0., e: 0., f: -4., };
        // let CDEF { c, d, e, f } = cdef.clone();
        assert_eq!(cdef.rotate(&0.).level(), cdef);
        let pi4 = cdef.rotate(&(PI / 4.));
        assert_relative_eq!(pi4, BCDEF { b: -1.2, c: 1., d: 0., e: 0., f: -1.6, t: PI / 4., }, max_relative = 1e-15);
        assert_relative_eq!(pi4.level(), cdef, max_relative = 1e-15);
        assert_relative_eq!(pi4.xyrrt(), XYRRT { c: R2 { x: 0., y: 0., }, r: R2 { x: 2., y: 1., }, t: PI / 4., }, max_relative = 1e-15);
        // let pi2 = cdef.rotate(&(PI / 2.));
        // assert_relative_eq!(pi2.level(), cdef, max_relative = 1e-15);
        // assert_relative_eq!(pi4.level(), cdef, max_relative = 1e-15);
    }
}