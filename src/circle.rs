use std::{f64::consts::PI, fmt::Display};
// use derive_more::{Clone};

use nalgebra::{Const, SMatrix, SVector, RowDVector, Dyn, Matrix};
use num_dual::{DualNum, jacobian, DualVec, Derivative, DualVec64, DualDVec64};
use serde::{Deserialize, Serialize};
use crate::{
    r2::R2,
    region::Region,
    // dual::Dual,
    intersection::Intersection, edge::Edge, dual::Dual
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Circle<D> {
    pub c: R2<D>,
    pub r: D,
}

type D = Dual;
// type D = DualDVec64;

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
        let x = Dual::new(self.c.x, dv());
        let y = Dual::new(self.c.y, dv());
        let r = Dual::new(self.r  , dv());
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

impl Circle<D> {
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

        let z = (r0sq.clone() - r1sq.clone() + x1sq.clone() - x0sq.clone() + y1sq.clone() - y0sq.clone()) / 2.;
        let dy = y1.clone() - y0.clone();
        let dx = x1.clone() - x0.clone();
        // let dr = r1.clone() - r0.clone();
        let [ x_0, y_0, x_1, y_1 ] = if dx.re != 0. {
            let u = -dy.clone() / dx.clone();
            let v = z.clone() / dx.clone();
            let a = u.clone() * u.clone() + 1.;
            let b = u.clone() * (v.clone() - x0.clone()) - y0.clone();
            let vsq = v.clone() * v.clone();
            let c = vsq.clone() - v.clone() * x0.clone() * 2. + x0sq.clone() + y0sq.clone() - r0sq.clone();
            let f = -b.clone() / a.clone();
            let [ mut y_0, mut y_1 ] = [ f.clone(), f.clone(), ];
            let dsq = f.clone() * f.clone() - c.clone() / a.clone();
            if dsq.re() != 0. {
                let d = dsq.sqrt();
                y_0 += d.clone();
                y_1 -= d.clone();
            };
            let x_0 = u.clone() * y_0.clone() + v.clone();
            let x_1 = u.clone() * y_1.clone() + v.clone();
            println!("Solved for y then x:");
            println!("a: {}", a.clone());
            println!("b: {}", b.clone());
            println!("c: {}", c.clone());
            println!("f: {}", f.clone());
            println!("d²: {}", dsq.clone());
            println!("x0: {}", x_0.clone());
            println!("y0: {}", y_0.clone());
            println!("x1: {}", x_1.clone());
            println!("y1: {}", y_1.clone());
            [ x_0, y_0, x_1, y_1 ]
        } else {
            let u = -dx.clone() / dy.clone();
            let v = z.clone() / dy.clone();
            let a = u.clone() * u.clone() + 1.;
            let b = u.clone() * (v.clone() - y0.clone()) - x0.clone();
            let vsq = v.clone() * v.clone();
            let c = vsq.clone() - v.clone() * y0.clone() * 2. + x0sq.clone() + y0sq.clone() - r0sq.clone();
            let f = -b.clone() / a.clone();
            let [ mut x_0, mut x_1 ] = [ f.clone(), f.clone(), ];
            let dsq = f.clone() * f.clone() - c.clone() / a.clone();
            if dsq.re() != 0. {
                let d = dsq.sqrt();
                x_0 += d.clone();
                x_1 -= d.clone();
            }
            let y_0 = u.clone() * x_0.clone() + v.clone();
            let y_1 = u.clone() * x_1.clone() + v.clone();
            println!("Solved for x then y:");
            println!("a: {}", a.clone());
            println!("b: {}", b.clone());
            println!("c: {}", c.clone());
            println!("f: {}", f.clone());
            println!("d²: {}", dsq.clone());
            println!("x0: {}", x_0.clone());
            println!("y0: {}", y_0.clone());
            println!("x1: {}", x_1.clone());
            println!("y1: {}", y_1.clone());
            [ x_0, y_0, x_1, y_1 ]
        };
        [ R2 { x: x_0, y: y_0 }, R2 { x: x_1, y: y_1 } ]
    }
    pub fn unit_intersections(&self) -> [R2<D>; 2] {
        let cx = self.c.x.clone();
        let cy = self.c.y.clone();
        let r = self.r.clone();
        let cx2 = cx.clone() * &cx;
        let cy2 = cy.clone() * cy.clone();
        let C = cx2.clone() + cy2.clone();
        let r2 = r.clone() * r.clone();
        let D = C.clone() - r2.clone() + 1.;
        let a = C.clone() * 4.;
        let b = cx.clone() * D.clone() * -2.;
        let c = D.clone() * D.clone() - cy2.clone() * 4.;
        let f = -b.clone() / a.clone();
        println!("a: {}", a.clone());
        println!("b: {}", b.clone());
        println!("c: {}", c.clone());
        println!("f: {}", f.clone());

        let d = f.clone() * f.clone() - c.clone() / a.clone();
        println!("d²: {}", d.clone());
        let dsqrt = d.sqrt();
        let rhs = dsqrt.clone() / a.clone();

        let x0 = if d.re() == 0. { f.clone() } else { f.clone() - rhs.clone() };
        println!("x0: {}", x0.clone());

        let x0cx = x0.clone() - cx.clone();
        let x0cx2 = x0cx.clone() * x0cx.clone();

        let ydsq = r2.clone() - x0cx2.clone();
        let mut y0_0 = ydsq.clone().sqrt();
        let mut y0_1 = -y0_0.clone();
        y0_0 += cy.clone();
        y0_1 += cy.clone();
        let x0sq = x0.clone() * x0.clone();
        let check0_0 = (1. - x0sq.clone() - y0_0.clone() * y0_0.clone()).abs();
        let check0_1 = (1. - x0sq.clone() - y0_1.clone() * y0_1.clone()).abs();
        let y0 = if check0_0 < check0_1 { y0_0.clone() } else { y0_1.clone() };

        // let y0sq = r0sq - x0.clone() * x0.clone();
        // let y0_1 = y0sq.sqrt();
        // let y0_0 = -y0_1.clone();
        // let y0_0cy = y0_0.clone() - cy.clone();
        // let y0_1cy = y0_1.clone() - cy.clone();
        // let check0_0 = (x0cx2.clone() + y0_0cy.clone() * y0_0cy.clone() - r2.clone()).abs();
        // let check0_1 = (x0cx2.clone() + y0_1cy.clone() * y0_1cy.clone() - r2.clone()).abs();
        // let y0 = if check0_0 < check0_1 { y0_0.clone() } else { y0_1.clone() };
        // println!("checks0: {:?}, {:?}", check0_0, check0_1);
        // dbg!("y0: {}", y0.clone());

        let x1 = if d.re() == 0. { f.clone() } else { f.clone() + rhs.clone() };
        // dbg!("x1: {}", x1.clone());
        let y1 = if x0 == x1 { y0_0 } else {
            let y1sq = -x1.clone() * x1.clone() + 1.;
            let y1_0 = y1sq.sqrt();
            let y1_1 = -y1_0.clone();
            let x1cx = x1.clone() - cx.clone();
            let x1cx2 = x1cx.clone() * x1cx.clone();
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

    #[test]
    fn intersections1() {
        let c0 = Circle { c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { c: R2 { x: 1., y: 0. }, r: 1. };
        let d0 = c0.dual(0, 3);
        let d1 = c1.dual(3, 0);
        let [ p0, p1 ] = d0.intersections(&d1);
        // dbg!([ p0.clone(), p1.clone() ]);
        println!("unit_intersections");
        println!("{}", p0);
        println!("{}", p1);
        println!();
        assert_relative_eq!(p0, r2(0.5, vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000],  0.866, vec![ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(0.5, vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000], -0.866, vec![-0.289, 0.500, -0.577,  0.289, 0.500, -0.577]), epsilon = 1e-3);
    }

    #[test]
    fn intersections01() {
        let c0 = Circle { c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { c: R2 { x: 0., y: 1. }, r: 1. };
        let d0 = c0.dual(0, 3);
        let d1 = c1.dual(3, 0);
        let [ p0, p1 ] = d0.intersections(&d1);
        // dbg!([ p0.clone(), p1.clone() ]);
        println!("unit_intersections");
        println!("{}", p0);
        println!("{}", p1);
        println!();
        assert_relative_eq!(p0, r2( 0.866, vec![ 0.500,  0.289,  0.577, 0.500, -0.289,  0.577], 0.500, vec![ 0.866, 0.500, 1.000, -0.866, 0.500, -1.000]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(-0.866, vec![ 0.500, -0.289, -0.577, 0.500,  0.289, -0.577], 0.500, vec![-0.866, 0.500, 1.000,  0.866, 0.500, -1.000]), epsilon = 1e-3);
    }

    fn r2(x: f64, dx: Vec<f64>, y: f64, dy: Vec<f64>) -> R2<D> {
        R2 { x: Dual::new(x, dx), y: Dual::new(y, dy) }
    }

    #[test]
    fn intersections2() {
        let c0 = Circle { c: R2 { x: 1., y: 1. }, r: 2. };
        let c1 = Circle { c: R2 { x: 3., y: 1. }, r: 2. };
        let d0 = c0.dual(0, 3);
        let d1 = c1.dual(3, 0);
        let [ p0, p1 ] = d0.intersections(&d1);
        // dbg!([ p0.clone(), p1.clone() ]);
        println!("unit_intersections");
        println!("{}", p0);
        println!("{}", p1);
        println!();
        assert_relative_eq!(p0, r2(2., vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000],  2.732, vec![ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(2., vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000], -0.732, vec![-0.289, 0.500, -0.577,  0.289, 0.500, -0.577]), epsilon = 1e-3);
    }

    #[test]
    fn intersections_projected1() {
        let c0 = Circle { c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { c: R2 { x: 1., y: 0. }, r: 1. };
        let d0 = c0.dual(0, 3);
        let d1 = c1.dual(3, 0);

        // let projected = d0.project(&d1);
        // let unit_intersections = projected.unit_intersections();
        // let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        // let [ p0, p1 ] = unit_intersections.map(|p| invert(p, &d1));
        // // dbg!([ p0.clone(), p1.clone() ]);
        // println!("projected_intersections");
        // println!("{}", p0);
        // println!("{}", p1);
        // println!();
        let projected = d1.project(&d0);
        println!("projected: {}", projected);
        println!();
        let unit_intersections = projected.unit_intersections();
        println!("unit_intersections:");
        println!("{}", unit_intersections[0]);
        println!("{}", unit_intersections[1]);
        println!();
        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let [ p0, p1 ] = unit_intersections.map(|p| invert(p, &d0));
        // dbg!([ p0.clone(), p1.clone() ]);
        println!("inverted:");
        println!("{}", p0);
        println!("{}", p1);
        println!();

        assert_relative_eq!(p0, r2(0.5, vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000],  0.866, vec![ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(0.5, vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000], -0.866, vec![-0.289, 0.500, -0.577,  0.289, 0.500, -0.577]), epsilon = 1e-3);
    }

    #[test]
    fn intersections_projected2() {
        let c0 = Circle { c: R2 { x: 1., y: 1. }, r: 2. };
        let c1 = Circle { c: R2 { x: 3., y: 1. }, r: 2. };
        let d0 = c0.dual(0, 3);
        let d1 = c1.dual(3, 0);

        // let projected = d0.project(&d1);
        // let unit_intersections = projected.unit_intersections(&d1.r);
        // let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        // let [ p0, p1 ] = unit_intersections.map(|p| invert(p, &d1));
        // println!("projected_intersections");
        // println!("{}", p0);
        // println!("{}", p1);
        // println!();

        let projected = d1.project(&d0);
        let s = format!("{}", projected);
        println!("s: {}", s);
        let unit_intersections = projected.unit_intersections();
        println!("unit_intersections");
        println!("{}", unit_intersections[0]);
        println!("{}", unit_intersections[1]);
        println!();
        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let [ p0, p1 ] = unit_intersections.map(|p| invert(p, &d0));
        // dbg!([ p0.clone(), p1.clone() ]);
        println!("projected_intersections 2");
        println!("{}", p0);
        println!("{}", p1);
        println!();

        assert_relative_eq!(p0, r2(2., vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000], -0.732, vec![-0.289, 0.500, -0.577,  0.289, 0.500, -0.577]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(2., vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000],  2.732, vec![ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577]), epsilon = 1e-3);
    }
}
