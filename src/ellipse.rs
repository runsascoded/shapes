use std::ops::{Mul, Div, Add, Sub, Neg};

use log::info;
use roots::find_roots_quartic;

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

pub trait UnitIntersectionsArg: Clone + Add<Output = Self> + Add<f64, Output = Self> + Sub<Output = Self> + Sub<f64, Output = Self> + Mul<Output = Self> + Div<Output = Self>
// where f64: Mul<Self, Output = Self> + Div<Self, Output = Self>
{}
impl<D: Clone + Add<Output = D> + Add<f64, Output = D> + Sub<Output = D> + Sub<f64, Output = D> + Mul<Output = D> + Div<Output = D>> UnitIntersectionsArg for D
// where f64: Mul<D, Output = D> + Div<D, Output = D>
{}

impl<D: UnitIntersectionsArg> ACDEF<D>
where f64: Mul<D, Output = D> + Div<D, Output = D>
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

    pub fn abcdef(&self) -> ABCDEF<D> {
        todo!()
        // ABCDEF {

        // }
    }

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

impl<D: UnitIntersectionsArg + RotateArg> XYRR<D>
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

// pub fn derivative<T>(coeffs: [f64; 5]) -> [f64; 4]
// where f64: Mul<T, Output = T> {
//     let [ a_4, a_3, a_2, a_1, a_0 ] = coeffs;
//     let d_3: f64 = f64::mul(a_4, 4.);
//     let d_2: f64 = f64::mul(a_3, 3.);
//     let d_1: f64 = f64::mul(a_2, 2.);
//     let d_0: f64 = a_1;
//     [ d_3, d_2, d_1, d_0 ]
// }

trait Scalar {
    fn to_f64(&self) -> f64;
}

impl Scalar for f64 {
    fn to_f64(&self) -> f64 { *self }
}

impl Scalar for Dual {
    fn to_f64(&self) -> f64 { self.v() }
}

impl<D: Clone + Scalar + Add<Output = D> + Sub<Output = D> + Sub<f64, Output = D> + Mul<Output = D> + Mul<f64, Output = D>> ABCDEF<D>
// where f64: Mul<D, Output = D> //+ Mul<f64, Output = f64>
{
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        let ac1 = self.a.clone() - self.c.clone() - 1.;  // A - C - 1
        let b2 = self.b.clone() * self.b.clone();  // B²
        let d2 = self.d.clone() * self.d.clone();  // D²
        let e2 = self.e.clone() * self.e.clone();  // E²
        let f2 = self.f.clone() * self.f.clone();  // F²
        let c_4 = ac1.clone() * ac1.clone() + b2.clone();  // (A - C - 1)² + B²
        let c_3 = (self.b.clone() * self.d.clone() - self.e.clone() * ac1.clone()) * 2.;  // 2(BD - E(A - C - 1))
        let c_2 = d2.clone() - b2 - e2;  // D² - B² - E²
        let c_1 = (self.e.clone() * self.f.clone() + self.b.clone() * self.d.clone()) * -2.;  // -2(EF + BD)
        let c_0 = f2 - d2;  // F² - D²
        let coeff_duals: [ &D; 5 ] = [
            &c_4,
            &c_3,
            &c_2,
            &c_1,
            &c_0,
        ];
        let coeffs: [ f64; 5 ] = [
            c_4.to_f64(),
            c_3.to_f64(),
            c_2.to_f64(),
            c_1.to_f64(),
            c_0.to_f64(),
        ];
        let [ a_4, a_3, a_2, a_1, a_0 ] = coeffs;
        let roots = find_roots_quartic(a_4, a_3, a_2, a_1, a_0);
        let d_3: f64 = f64::mul(a_4, 4.);
        let d_2: f64 = f64::mul(a_3, 3.);
        let d_1: f64 = f64::mul(a_2, 2.);
        let d_0: f64 = a_1;
        let fp = |x: f64| d_3 * x * x * x + d_2 * x * x + d_1 * x + d_0;

        let dual_roots: Vec<R2<D>> = vec![];
        for root in roots.as_ref() {
            let fd = fp(*root);
            if fd == 0. {
                // Multiple root
                let mut order = 2;
                let e_2: f64 = 3. * d_3;
                let e_1: f64 = 2. * d_2;
                let e_0: f64 = d_1;
                let fpp = |x: f64| e_2 * x * x + e_1 * x + e_0;
                let fdd = fpp(*root);
                if fdd == 0. {
                    let f_1 = 2. * e_2;
                    let f_0 = e_1;
                    let fppp = |x: f64| f_1 * x + f_0;
                    let fddd = fppp(*root);
                    if fddd == 0. {
                        order = 4;
                    } else {
                        order = 3;
                    }
                }
                info!("Skipping multiple root {} ({})", root, order);
            } else {
                for (idx, coeff) in coeff_duals.into_iter().enumerate().rev() {
                    let d = coeff.d();
                    if d != 0. {
                        let dual = Dual::new(*root, d);
                        dual_roots.push(R2 {
                            x: dual.clone(),
                            y: dual.clone(),
                        });
                        break;
                    }
                }
            }
        }
        dual_roots
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