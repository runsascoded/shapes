
use std::{fmt::Display, ops::{Div, Sub}};

use crate::{math::AbsArg, r2::R2, ellipses::quartic::{Root, Quartic}, circle, dual::Dual};

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
        // println!("c: {}", self.c);
        // println!("d: {}", self.d);
        // println!("e: {}", self.e);
        // println!("f: {}", self.f);
        let rd = -1. / self.d.clone();
        let c_2 = (self.c.clone() - 1.) * rd.clone();
        let c_1 = self.e.clone() * rd.clone();
        let c_0 = (self.f.clone() + 1.) * rd;

        let a_4 = c_2.clone() * c_2.clone();
        let a_3 = c_2.clone() * c_1.clone() * 2.;
        let a_2 = c_1.clone() * c_1.clone() + c_2.clone() * c_0.clone() * 2. + 1.;
        let a_1 = c_1.clone() * c_0.clone() * 2.;
        let a_0 = c_0.clone() * c_0.clone() - 1.;
        let ys = Quartic::quartic_roots(a_4, a_3, a_2, a_1, a_0);
        let mut dual_roots: Vec<R2<D>> = Vec::new();
        for Root(y, double_root) in ys {
            let x = c_2.clone() * y.clone() * y.clone() + c_1.clone() * y.clone() + c_0.clone();
            if double_root {
                dual_roots.push(R2 { x:  x.clone(), y: y.clone() });
                dual_roots.push(R2 { x: -x, y: y.clone() });
            } else {
                dual_roots.push(R2 { x, y: y.clone() });
            }
        }
        dual_roots
    }
}
