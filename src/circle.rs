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
        let sd = self.dual(0, 3);
        let od = o.dual(3, 0);
        let projected = sd.project(&od);
        let unit_intersections = projected.unit_intersections();

        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let points = unit_intersections.map(|p| invert(p, &od));
        let intersections = points.map(|p| Intersection { x: p.x, y: p.y, c1: sd.clone(), c2: od.clone() });
        let edge0 = Edge { c: sd, intersections: intersections.clone() };
        let edge1 = Edge { c: od, intersections: intersections.clone() };
        let region = Region { edges: vec![ edge0, edge1 ], intersections: intersections.into() };
        region
    }
}

impl<D: DualNum<f64> + PartialOrd> Circle<D> {
    pub fn intersections(&self, o: &Circle<D>) -> [R2<D>; 2] {
        let x0 = self.c.x.clone();
        let y0 = self.c.y.clone();
        let r0 = self.r.clone();
        let x0sq = x0.clone() * x0.clone();
        let y0sq = y0.clone() * y0.clone();
        let r0sq = r0.clone() * r0.clone();

        let x1 = o.c.x.clone();
        let y1 = o.c.y.clone();
        let r1 = o.r.clone();
        let x1sq = x1.clone() * x1.clone();
        let y1sq = y1.clone() * y1.clone();
        let r1sq = r1.clone() * r1.clone();

        let z = (r1sq.clone() - r0sq.clone() + x0sq.clone() - x1sq.clone() + y0sq.clone() - y1sq.clone()) / 2.;
        let dy = y1.clone() - y0.clone();
        let dx = x1.clone() - x0.clone();
        let dr = r1.clone() - r0.clone();
        let u = -dy.clone() / dx.clone();
        let v = -z.clone() / dx.clone();
        let a = u.clone() * u.clone() + 1.;
        let b = u.clone() * (v.clone() - x0.clone()) - y0.clone();
        let vsq = v.clone() * v.clone();
        let c = vsq.clone() - v.clone() * x0.clone() * 2. + x0sq.clone() + y0sq.clone() - r0sq.clone();
        let f = b.clone() / a.clone();
        let dsq = f.clone() * f.clone() - c.clone() / a.clone();
        let [ mut y_0, mut y_1 ] = [ -f.clone(), -f.clone(), ];
        if dsq.re() != 0. {
            let d = dsq.sqrt();
            y_0 += d.clone();
            y_1 -= d.clone();
        };
        let x_0 = u.clone() * y_0.clone() + v.clone();
        let x_1 = u.clone() * y_1.clone() + v.clone();
        [ R2 { x: x_0, y: y_0 }, R2 { x: x_1, y: y_1 } ]
    }
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
        let c = (self.c.clone() - o.c.clone()) / o.r.clone();
        let r = self.r.clone() / o.r.clone();
        Circle { c, r, }
    }
    pub fn invert(&self, p: R2<D>) -> R2<D> {
        p * self.r.clone() + self.c.clone()
    }
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

    fn dual(re: f64, eps: Vec<f64>) -> DualDVec64 {
        DualDVec64::new(re, Derivative::new(Some(Matrix::from(eps))))
    }

    #[test]
    fn unit_intersections() {
        let c0 = Circle { c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { c: R2 { x: 1., y: 0. }, r: 1. };
        let d0 = c0.dual(0, 3);
        let d1 = c1.dual(3, 0);
        let [ p0, p1 ] = d0.intersections(&d1);
        // dbg!([ p0.clone(), p1.clone() ]);
        println!("unit_intersections");
        println!("{}", p0);
        println!("{}", p1);
        assert_eq!(
            [ p0, p1 ],
            [
                R2 { x: dual( 0.5,                vec![  0.5,                  0.8660254037844386,  1.,                  0.5,                -0.8660254037844386, -1.                 ]),
                     y: dual( 0.8660254037844386, vec![  0.28867513459481287,  0.5,                 0.5773502691896257, -0.28867513459481287, 0.5,                 0.5773502691896257 ]), },
                R2 { x: dual( 0.5               , vec![  0.5,                 -0.8660254037844386,  1.,                  0.5,                 0.8660254037844386, -1.                 ]),
                     y: dual(-0.8660254037844386, vec![ -0.28867513459481287,  0.5,                -0.5773502691896257,  0.28867513459481287, 0.5,                -0.5773502691896257 ]), },
            ]
        );

    }
    #[test]
    fn projected_intersections() {
        let c0 = Circle { c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { c: R2 { x: 1., y: 0. }, r: 1. };
        let d0 = c0.dual(0, 3);
        let d1 = c1.dual(3, 0);

        let projected = d0.project(&d1);
        let unit_intersections = projected.unit_intersections();
        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let [ p0, p1 ] = unit_intersections.map(|p| invert(p, &d1));
        // dbg!([ p0.clone(), p1.clone() ]);
        println!("projected_intersections");
        println!("{}", p0);
        println!("{}", p1);
        println!();
        let projected = d1.project(&d0);
        let unit_intersections = projected.unit_intersections();
        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let [ p0, p1 ] = unit_intersections.map(|p| invert(p, &d0));
        // dbg!([ p0.clone(), p1.clone() ]);
        println!("{}", p0);
        println!("{}", p1);

        // let d0 = c0.dual(0, 3);
        // let d1 = c1.dual(3, 0);
        // let region = c0.intersect(&c1);
        // dbg!(&region);
        // let area = region.polygon_area();
        // dbg!(&area);
        // assert_eq!(area.re, 0.);

        // println!("region: {:?}", region);
        // println!("region: {}", region);
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
