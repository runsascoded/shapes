use std::f64::consts::PI;

use nalgebra::{Const, SMatrix};
use num_dual::{DualNum, jacobian, DualVec, Derivative};
use crate::r2::R2;

pub struct Circle<D> {
    c: R2<D>,
    r: D,
}

impl<D: DualNum<f64> + PartialOrd + Copy> Circle<D> {
    pub fn x(&self) -> D {
        self.c.x
    }
    pub fn y(&self) -> D {
        self.c.y
    }
    pub fn area(&self) -> D {
        self.r * self.r * PI
    }
    pub fn unit_intersections(&self) -> [R2<D>; 2] {
        let cx = self.c.x;
        let cy = self.c.y;
        let r = self.r;
        let cx2 = cx * cx;
        let cy2 = cy * cy;
        let C = cx2 + cy2;
        let r2 = r * r;
        let D = C - r2 + 1.;
        let a = C * 4.;
        let b = cx * D * -4.;
        let c = D * D - cy2 * 4.;

        let d = b * b - a * c * 4.;
        let dsqrt = d.sqrt();
        let rhs = dsqrt / 2. / a;
        let lhs = -b / 2. / a;

        let x0 = lhs - rhs;
        let y0sq = -x0 * x0 + 1.;
        let y0_0 = y0sq.sqrt();
        let y0_1 = -y0_0;
        let x0cx = x0 - cx;
        let x0cx2 = x0cx * x0cx;
        let y0_0cy = y0_0 - cy;
        let y0_1cy = y0_1 - cy;
        let check0_0 = (x0cx2 + y0_0cy * y0_0cy - r2).abs();
        let check0_1 = (x0cx2 + y0_1cy * y0_1cy - r2).abs();
        let y0 = if check0_0 < check0_1 { y0_0 } else { y0_1 };
        // println!("checks0: {:?}, {:?}", check0_0, check0_1);

        let x1 = lhs + rhs;
        let y1sq = -x1 * x1 + 1.;
        let y1_0 = y1sq.sqrt();
        let y1_1 = -y1_0;
        let x1cx = x1 - cx;
        let x1cx2 = x1cx * x1cx;
        let y1_0cy = y1_0 - cy;
        let y1_1cy = y1_1 - cy;
        let check1_0 = (x1cx2 + y1_0cy * y1_0cy - r2).abs();
        let check1_1 = (x1cx2 + y1_1cy * y1_1cy - r2).abs();
        let y1 = if check1_0 < check1_1 { y1_0 } else { y1_1 };
        // println!("checks1: {:?}, {:?}", check1_0, check1_1);

        [
            R2 { x: x0, y: y0 },
            R2 { x: x1, y: y1 },
        ]
    }
    pub fn unit_intersection_duals(&self) -> [ R2<DualVec<D, f64, Const<3>>>; 2] {
        let (value, grad) = jacobian(
            |v| {
                let [ cx, cy, r ] = [ v[0], v[1], v[2] ];
                let c = R2 { x: cx, y: cy };
                let points = Circle { c, r }.unit_intersections();
                let [ p0, p1 ] = points;
                SMatrix::from([p0.x, p0.y, p1.x, p1.y])
            },
            SMatrix::from([self.c.x, self.c.y, self.r]),
        );

        // Unwrap to R2<DualVec>'s
        let dv = |i: usize| {
            DualVec::<D, f64, Const<3>>::new(
                value[i],
                Derivative::new(Some(grad.row(i).transpose()))
            )
        };
        let p0 = R2 { x: dv(0), y: dv(1) };
        let p1 = R2 { x: dv(2), y: dv(3) };

        [ p0, p1 ]
    }
    pub fn project(&self, o: &Circle<D>) -> Self {
        let c = self.c - o.c;
        let r = self.r / o.r;
        Circle { c, r, }
    }
    pub fn invert(&self, p: &R2<D>) -> R2<D> {
        *p * self.r + self.c
    }
    pub fn intersect(&self, o: &Circle<D>) -> [R2<D>; 2] {
        self.project(o).unit_intersections().map(|p| o.invert(&p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{Const, Matrix3x1};
    use num_dual::DualVec64;

    #[test]
    fn unit_intersections() {
        let c = Circle {
            c: R2 { x: 1., y: 1. },
            r: 2.
        };
        let [ p0, p1 ]: [ R2<DualVec64<Const<3>>>; 2 ] = c.unit_intersection_duals();
        assert_eq!(
            [ p0, p1 ],
            [
                R2 { x: DualVec64::new(-0.9114378277661477, Derivative::new(Some(Matrix3x1::from([0.5944911182523069 , 0.18305329048615926, -0.6220355269907728])))),
                     y: DualVec64::new( 0.4114378277661476, Derivative::new(Some(Matrix3x1::from([1.3169467095138412 , 0.40550888174769345, -1.3779644730092273])))), },
                R2 { x: DualVec64::new( 0.4114378277661477, Derivative::new(Some(Matrix3x1::from([0.40550888174769306, 1.3169467095138407 , -1.3779644730092273])))),
                     y: DualVec64::new(-0.9114378277661477, Derivative::new(Some(Matrix3x1::from([0.18305329048615915, 0.5944911182523069 , -0.622035526990773 ])))), },
            ]
        );
    }
}
