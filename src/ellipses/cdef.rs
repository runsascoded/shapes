
use std::{fmt::Display, ops::{Div, Sub}};

use log::debug;

use crate::{math::{abs::AbsArg, quartic::quartic}, r2::R2, ellipses::quartic::{Root, Quartic}, circle, dual::Dual};

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
+ Quartic
+ circle::UnitIntersectionsArg
{}

impl UnitIntersectionsArg for f64 {}
impl UnitIntersectionsArg for Dual {}

impl<D: UnitIntersectionsArg> CDEF<D>
where
    f64
    : Sub<D, Output = D>
    + Div<D, Output = D>,
{
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        debug!("c: {}", self.c);
        debug!("d: {}", self.d);
        debug!("e: {}", self.e);
        debug!("f: {}", self.f);
        let d_zero = self.d.clone().into() == 0.;
        // debug!("d_zero: {}", d_zero);
        let [ c_2, c_1, c_0 ] = if d_zero {
            let re = -1. / self.e.clone();
            [
                (1. - self.c.clone()) * re.clone(),
                self.d.clone() * re.clone(),
                (self.c.clone() + self.f.clone()) * re,
            ]
        } else {
            let rd = -1. / self.d.clone();
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
        // if a_4.clone().into() < 1e-7 {
        //     debug!("Setting a_4 to 0.");
        //     a_4 = a_4.zero();
        // }
        // if a_3.clone().into() < 1e-7 {
        //     debug!("Setting a_3 to 0.");
        //     a_3 = a_3.zero();
        // }
        let roots = Quartic::quartic_roots(a_4, a_3, a_2, a_1, a_0);
        let mut points: Vec<R2<D>> = Vec::new();
        debug!("Points:");
        for Root(r0, double_root) in &roots {
            let r1 = c_2.clone() * r0.clone() * r0.clone() + c_1.clone() * r0.clone() + c_0.clone();
            let [ x, y ] = if d_zero {
                [ r0.clone(), r1.clone() ]
            } else {
                [ r1.clone(), r0.clone() ]
            };
            if *double_root {
                debug!("  double: {:?}, {:?}", r0, r1);
                points.push(R2 { x:  x.clone(), y: y.clone() });
                points.push(R2 { x: -x.clone(), y: y.clone() });
            } else {
                debug!("  r0 {:?}, r1 {:?}", r0, r1);
                debug!("  x: {}, y: {}", x, y);
                points.push(R2 { x, y });
            }
        }
        // debug!("roots: {:?}", &roots);
        points
    }
}
