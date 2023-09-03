use std::ops::{Mul, Div, Add, Sub, Neg};

use crate::{r2::R2, rotate::{Rotate, RotateArg}, math_ops::Trig};


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
    a: D,
    c: D,
    d: D,
    e: D,
    f: D,
}

impl<D> ACDEF<D> {
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        todo!()
    }
}

pub struct XYRRT<D> {
    c: R2<D>,
    rx: D,
    ry: D,
    t: D,
}

impl<D: RotateArg + Neg<Output = D>> XYRRT<D> {
    // pub fn rotate(&self, t: &D) -> XYRRT<D> {
    //     todo!()
    // }

    /// Rotate the plane so that this ellipse ends up aligned with the x- and y-axes (i.e. θ == B == 0)
    pub fn level(&self) -> XYRR<D> {
        XYRR {
            c: self.c.clone().rotate(&-self.t.clone()),
            rx: self.rx.clone(),
            ry: self.ry.clone(),
        }
    }
}

pub struct XYRR<D> {
    c: R2<D>,
    rx: D,
    ry: D,
}

impl<D: Clone + Trig + Add<Output = D> + Sub<Output = D> + Sub<f64, Output = D> + Mul<Output = D>> XYRR<D>
where
    f64: Mul<D, Output = D> + Div<D, Output = D>,
{
    pub fn acdef(&self) -> ACDEF<D> {
        let rxr = 1. / self.rx.clone();
        let ryr = 1. / self.ry.clone();
        let r_x = self.c.x.clone() * rxr.clone();
        let r_y = self.c.y.clone() * ryr.clone();
        ACDEF {
            a: rxr.clone() * rxr,
            c: ryr.clone() * ryr,
            d: -2. * r_x.clone(),
            e: -2. * r_y.clone(),
            f: r_x.clone() * r_x + r_y.clone() * r_y - 1.,
        }
    }
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        self.acdef().unit_intersections()
    }
    pub fn rotate(&self, t: &D) -> XYRRT<D> {
        XYRRT {
            c: self.c.clone().rotate(t),
            rx: self.rx.clone(),
            ry: self.ry.clone(),
            t: t.clone(),
        }
    }
}

/// Ax² + Bxy + Cy² + Dx + Ey + F = 0
pub struct ABCDEF<D> {
    a: D,
    b: D,
    c: D,
    d: D,
    e: D,
    f: D,
}

impl<D: Clone + Add<Output = D> + Sub<Output = D> + Sub<f64, Output = D> + Mul<Output = D>> ABCDEF<D>
where f64: Mul<D, Output = D>
{
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        let ac1 = self.a.clone() - self.c.clone() - 1.;  // A - C - 1
        let b2 = self.b.clone() * self.b.clone();  // B²
        let d2 = self.d.clone() * self.d.clone();  // D²
        let e2 = self.e.clone() * self.e.clone();  // E²
        let f2 = self.f.clone() * self.f.clone();  // F²
        let c_4 = ac1.clone() * ac1.clone() + b2.clone();  // (A - C - 1)² + B²
        let c_3 = 2. * (self.b.clone() * self.d.clone() - self.e.clone() * ac1.clone());  // 2(BD - E(A - C - 1))
        let c_2 = d2.clone() - b2 - e2;  // D² - B² - E²
        let c_1 = -2. * (self.e.clone() * self.f.clone() + self.b.clone() * self.d.clone());  // -2(EF + BD)
        let c_0 = f2 - d2;  // F² - D²
        todo!()
    }
    // pub fn rotate(&self, t: &D) -> ABCDEF<D> {
    //     todo!()
    // }
    // Rotate the plane so that this ellipse ends up aligned with the x- and y-axes (i.e. θ == B == 0)
    // pub fn level(&self) -> ACDEF<D> {
    //     todo!()
    // }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    #[test]
    fn test_level() {
        let e = XYRRT {
            c: R2 { x: 1., y: 1. },
            rx: 2.,
            ry: 3.,
            t: PI / 4.,
        };

        let l = e.level();

        assert_relative_eq!(l.c.x, 2_f64.sqrt());
        assert_relative_eq!(l.c.y, 0.);
        assert_relative_eq!(l.rx, 2.);
        assert_relative_eq!(l.ry, 3.);
    }
}