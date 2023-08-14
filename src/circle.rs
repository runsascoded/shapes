use std::{f64::consts::PI, fmt::Display};
// use derive_more::{Clone};

use nalgebra::{Const, SMatrix, SVector, RowDVector, Dyn, Matrix};
use num_dual::{DualNum, jacobian, DualVec, Derivative, DualVec64, DualDVec64};
use serde::{Deserialize, Serialize};
use crate::{
    r2::R2,
    region::Region,
    // dual::Dual,
    intersection::Intersection, edge::Edge
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Circle<D> {
    pub c: R2<D>,
    pub r: D,
}

type D = DualDVec64;
// type Dual = DualVec64<Dyn>;

impl Circle<f64> {
    pub fn dual(&self, pre_zeros: usize, post_zeros: usize) -> Circle<D> {
        let n = 3;
        let mut idx = pre_zeros;
        let zeros = vec![0.; pre_zeros + n + post_zeros];
        let mut dv = || {
            let mut v = zeros.clone();
            v[idx] = 1.;
            idx += 1;
            v
        };
        let x = DualVec::new(self.c.x, Derivative::new(Some(Matrix::from(dv()))));
        let y = DualVec::new(self.c.y, Derivative::new(Some(Matrix::from(dv()))));
        let r = DualVec::new(self.r  , Derivative::new(Some(Matrix::from(dv()))));
        // // let x = Dual { v: self.c.x, d: dv() };
        // let y = Dual { v: self.c.y, d: dv() };
        // let r = Dual { v: self.r, d: dv() };
        let c = R2 { x, y };
        Circle { c, r }
    }
    pub fn intersect(&self, o: &Circle<f64>) -> Region<D> {
        let sd = self.dual(3, 0);
        let od = o.dual(0, 3);
        let unit_intersections = sd.project(&od).unit_intersections();

        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let points = unit_intersections.map(|p| invert(p, &od));
        let intersections = points.map(|p| Intersection { x: p.x, y: p.y, c1: sd.clone(), c2: od.clone() });
        let edge0 = Edge { c: sd, intersections: intersections.clone() };
        let edge1 = Edge { c: od, intersections: intersections.clone() };
        let region = Region { edges: vec![ edge0, edge1 ], intersections: intersections.into() };
        region
    }
}

// impl<D: Clone> Circle<D> {

// type D = DualDVec64;
// impl Circle<DualDVec64> {
// impl<D: DualNum<f64> + PartialOrd + Copy> Circle<D> {
    // pub fn x(&self) -> D {
    //     self.c.x.clone()
    // }
    // pub fn y(&self) -> D {
    //     self.c.y.clone()
    // }
    // pub fn area(&self) -> D {
    //     self.r.clone() * self.r.clone() * PI
    // }
    // pub fn intersect(&self, other: &Circle<D>) -> Region<D> {
    //     self.project(o).unit_intersection_dual_vecs();
    //     let dx = self.c.x - other.c.x;
    //     let dy = self.c.y - other.c.y;
    //     let d = (dx * dx + dy * dy).sqrt();
    //     d < self.r + other.r
    // }
    // pub fn intersect(&self, o: &Circle<D>) -> Region<D> {
    //     let points = self.project(o).unit_intersection_duals().map(|p| o.invert(&p));
    //     let intersections = points.map(|p| Intersection { x: p.x, y: p.y, c1: *self, c2: o });
    // }
// }

impl<D: DualNum<f64> + PartialOrd> Circle<D> {
    pub fn unit_intersections(&self) -> [R2<D>; 2] {
        let cx = self.c.x.clone();
        let cy = self.c.y.clone();
        let r = self.r.clone();
        let cx2 = cx.clone() * cx.clone();
        let cy2 = cy.clone() * cy.clone();
        let C = cx2.clone() + cy2.clone();
        let r2 = r.clone() * r.clone();
        let D = C.clone() - r2.clone() + 1.;
        let a = C.clone() * 4.;
        let b = cx.clone() * D.clone() * -4.;
        let c = D.clone() * D.clone() - cy2.clone() * 4.;

        let d = b.clone() * b.clone() - a.clone() * c * 4.;
        let dsqrt = d.sqrt();
        let rhs = dsqrt.clone() / 2. / a.clone();
        let lhs = -b.clone() / 2. / a.clone();

        let x0 = if d.re() == 0. { lhs.clone() } else { lhs.clone() - rhs.clone() };
        // dbg!("x0: {}", x0.clone());
        let y0sq = -x0.clone() * x0.clone() + 1.;
        let y0_0 = y0sq.sqrt();
        let y0_1 = -y0_0.clone();
        let x0cx = x0.clone() - cx.clone();
        let x0cx2 = x0cx.clone() * x0cx;
        let y0_0cy = y0_0.clone() - cy.clone();
        let y0_1cy = y0_1.clone() - cy.clone();
        let check0_0 = (x0cx2.clone() + y0_0cy.clone() * y0_0cy.clone() - r2.clone()).abs();
        let check0_1 = (x0cx2 + y0_1cy.clone() * y0_1cy.clone() - r2.clone()).abs();
        let y0 = if check0_0 < check0_1 { y0_0.clone() } else { y0_1.clone() };
        // println!("checks0: {:?}, {:?}", check0_0, check0_1);
        // dbg!("y0: {}", y0.clone());

        let x1 = if d.re() == 0. { lhs.clone() } else { lhs.clone() + rhs.clone() };
        // dbg!("x1: {}", x1.clone());
        let y1 = if x0 == x1 { y0_0 } else {
            let y1sq = -x1.clone() * x1.clone() + 1.;
            let y1_0 = y1sq.sqrt();
            let y1_1 = -y1_0.clone();
            let x1cx = x1.clone() - cx.clone();
            let x1cx2 = x1cx.clone() * x1cx;
            let y1_0cy = y1_0.clone() - cy.clone();
            let y1_1cy = y1_1.clone() - cy.clone();
            let check1_0 = (x1cx2.clone() + y1_0cy.clone() * y1_0cy.clone() - r2.clone()).abs();
            let check1_1 = (x1cx2.clone() + y1_1cy.clone() * y1_1cy.clone() - r2.clone()).abs();
            if check1_0 < check1_1 { y1_0.clone() } else { y1_1.clone() }
        };
        // dbg!("y1: {}", y1.clone());
        // println!("checks1: {:?}, {:?}", check1_0, check1_1);

        [
            R2 { x: x0, y: y0 },
            R2 { x: x1, y: y1 },
        ]
    }
    pub fn project(&self, o: &Circle<D>) -> Self {
        let c = self.c.clone() - o.c.clone();
        let r = self.r.clone() / o.r.clone();
        let c = c / o.r.clone();
        Circle { c, r, }
    }
    pub fn invert(&self, p: R2<D>) -> R2<D> {
        p * self.r.clone() + self.c.clone()
    }
    // pub fn unit_intersection_duals(&self) -> [ R2<Dual>; 2] {
    //     self.unit_intersection_dual_vecs().map(|i| R2::<Dual>::from(i))
    // }
    // pub fn unit_intersection_dual_vecs(&self) -> [ R2<DualVec<D, f64, Const<3>>>; 2] {
    //     let (value, grad) = jacobian(
    //         |v| {
    //             let [ cx, cy, r ] = [ v[0], v[1], v[2] ];
    //             let c = R2 { x: cx, y: cy };
    //             let points = Circle { c, r }.unit_intersections();
    //             let [ p0, p1 ] = points;
    //             SMatrix::from([p0.x, p0.y, p1.x, p1.y])
    //         },
    //         SMatrix::from([self.c.x, self.c.y, self.r]),
    //     );

    //     // Unwrap to R2<DualVec>'s
    //     let dv = |i: usize| {
    //         DualVec::<D, f64, Const<3>>::new(
    //             value[i],
    //             Derivative::new(Some(grad.row(i).transpose()))
    //         )
    //     };
    //     let p0 = R2 { x: dv(0), y: dv(1) };
    //     let p1 = R2 { x: dv(2), y: dv(3) };

    //     [ p0, p1 ]
    // }
}

impl<D: Display> Display for Circle<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "C({:.3}, {:.3}, {:.3})", self.c.x, self.c.y, self.r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{Const, Matrix3x1};
    use num_dual::DualVec64;

    #[test]
    fn unit_intersections() {
        let c0 = Circle {
            c: R2 { x: 0., y: 0. },
            r: 2.
        };
        let c1 = Circle {
            c: R2 { x: 3., y: 0. },
            r: 2.
        };
        let region = c0.intersect(&c1);
        dbg!(&region);
        let area = region.polygon_area();
        dbg!(&area);
        assert_eq!(area.re, 0.);
        // println!("region: {:?}", region);
        println!("region: {}", region);
        // let [ p0, p1 ]: [ R2<DualVec64<Const<3>>>; 2 ] = c.unit_intersection_dual_vecs();
        // assert_eq!(
        //     [ p0, p1 ],
        //     [
        //         R2 { x: DualVec64::new(-0.9114378277661477, Derivative::new(Some(Matrix3x1::from([0.5944911182523069 , 0.18305329048615926, -0.6220355269907728])))),
        //              y: DualVec64::new( 0.4114378277661476, Derivative::new(Some(Matrix3x1::from([1.3169467095138412 , 0.40550888174769345, -1.3779644730092273])))), },
        //         R2 { x: DualVec64::new( 0.4114378277661477, Derivative::new(Some(Matrix3x1::from([0.40550888174769306, 1.3169467095138407 , -1.3779644730092273])))),
        //              y: DualVec64::new(-0.9114378277661477, Derivative::new(Some(Matrix3x1::from([0.18305329048615915, 0.5944911182523069 , -0.622035526990773 ])))), },
        //     ]
        // );
    }
}
