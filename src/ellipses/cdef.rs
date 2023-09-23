
use std::fmt::Display;

use log::debug;
use ordered_float::OrderedFloat;

use crate::{math::{abs::AbsArg, is_zero::IsZero, recip::Recip}, r2::R2, transform::CanProject, ellipses::quartic::{Root, Quartic}, circle, dual::Dual};

use super::xyrr::XYRR;

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
#[derive(Debug, Clone)]
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
        // debug!("c: {}", self.c);
        // debug!("d: {}", self.d);
        // debug!("e: {}", self.e);
        // debug!("f: {}", self.f);
        let d_zero = self.d.clone().is_zero();
        let e_zero = self.e.clone().is_zero();
        if d_zero {
            let points = self._unit_intersections(xyrr, true);
            let err = self.points_err(points.clone(), xyrr);
            // debug!("points err: {}", err);
            points
        } else if e_zero {
            let points = self._unit_intersections(xyrr, false);
            let err = self.points_err(points.clone(), xyrr);
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
        // debug!("_unit_intersections, sub_y_solve_x: {}", sub_y_solve_x);
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
        // debug!("c_2: {}", c_2);
        // debug!("c_1: {}", c_1);
        // debug!("c_0: {}", c_0);

        let mut a_4 = c_2.clone() * c_2.clone();
        let mut a_3 = c_2.clone() * c_1.clone() * 2.;
        let a_2 = c_1.clone() * c_1.clone() + c_2.clone() * c_0.clone() * 2. + 1.;
        let a_1 = c_1.clone() * c_0.clone() * 2.;
        let a_0 = c_0.clone() * c_0.clone() - 1.;
        // debug!("a_4: {}", a_4);
        // debug!("a_3: {}", a_3);
        // debug!("a_2: {}", a_2);
        // debug!("a_1: {}", a_1);
        // debug!("a_0: {}", a_0);
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
