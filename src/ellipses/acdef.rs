
use crate::{r2::R2, dual::D};

use super::quartic::quartic_roots;

/// Ellipse where "B" (the coefficient of xy) is zero:
///
/// Ax² + Cy² + Dx + Ey + F = 0
///
/// This means the ellipse is aligned with the x- and y-axes, which makes computing unit-circle intersections easier (the axis-alignment rotation can then be reverted, yielding the original (unrotated) ellipse's unit-circle intersections).
///
/// Ellipse-ellipse intersections are computed via the following steps:
/// 1. Project the plane so that one ellipse becomes a unit circle.
/// 2. Rotate the plane so that the other ellipse becomes axis-aligned (i.e. B == 0).
/// 3. Compute intersections of the axis-aligned ellipse with the unit circle.
/// 4. Revert 2. (rotate the plane back to its original orientation).
/// 5. Revert 1. (invert the projection).
pub struct ACDEF<D> {
    pub a: D,
    pub c: D,
    pub d: D,
    pub e: D,
    pub f: D,
}

// impl<D: UnitIntersectionsArg> ACDEF<D>
// where f64: Mul<D, Output = D> + Div<D, Output = D>
impl ACDEF<D>
{
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        let rd = -1. / self.d.clone();
        let c_2 = (self.c.clone() - self.a.clone()) * rd.clone();
        let c_1 = self.e.clone() * rd.clone();
        let c_0 = (self.a.clone() + self.f.clone()) * rd;

        let a_4 = c_2.clone() * c_2.clone();
        let a_3 = 2. * c_2.clone() * c_1.clone();
        let a_2 = c_1.clone() * c_1.clone() + 2. * c_2.clone() * c_0.clone() + 1.;
        let a_1 = 2. * c_1.clone() * c_0.clone();
        let a_0 = c_0.clone() * c_0.clone() - 1.;
        quartic_roots(a_4, a_3, a_2, a_1, a_0)
    }
}
