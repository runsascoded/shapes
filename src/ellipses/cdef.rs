
use crate::{r2::R2, dual::D, ellipses::quartic::Root};

use super::quartic::quartic_roots;

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

// impl<D: UnitIntersectionsArg> CDEF<D>
// where f64: Mul<D, Output = D> + Div<D, Output = D>
impl CDEF<D>
{
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        println!("c: {}", self.c);
        println!("d: {}", self.d);
        println!("e: {}", self.e);
        println!("f: {}", self.f);
        let rd = -1. / self.d.clone();
        let c_2 = (self.c.clone() - 1.) * rd.clone();
        let c_1 = self.e.clone() * rd.clone();
        let c_0 = (1. + self.f.clone()) * rd;

        let a_4 = c_2.clone() * c_2.clone();
        let a_3 = 2. * c_2.clone() * c_1.clone();
        let a_2 = c_1.clone() * c_1.clone() + 2. * c_2.clone() * c_0.clone() + 1.;
        let a_1 = 2. * c_1.clone() * c_0.clone();
        let a_0 = c_0.clone() * c_0.clone() - 1.;
        let ys = quartic_roots(a_4, a_3, a_2, a_1, a_0);

        let f = |x: f64, y: f64| {
            x * x + self.c.clone() * y * y + self.d.clone() * x + self.e.clone() * y + self.f.clone()
        };
        let mut dual_roots: Vec<R2<D>> = Vec::new();
        for Root(y, double_root) in ys {
            let x0 = (1. - y.clone() * y.clone()).sqrt();
            let x1 = -x0.clone();
            let fx0 = f(x0.v(), y.v());
            let fx1 = f(x1.v(), y.v());
            if double_root {
                dual_roots.push(R2 { x: x0, y: y.clone() });
                dual_roots.push(R2 { x: x1, y: y.clone() });
            } else {
                let x = if fx0.abs() < fx1.abs() { x0 } else { x1 };
                dual_roots.push(R2 { x: x, y: y.clone() });
            }
        }
        dual_roots
    }
}
