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
        let r2 = r.clone() * r.clone();
        let z = (1. + cx2.clone() + cy2.clone() - r2.clone()) / 2.;
        let [ u, v ] = if cx.re == 0. {
            [ -cx.clone() / cy.clone(), z.clone() / cy.clone(), ]
        } else {
            [ -cy.clone() / cx.clone(), z.clone() / cx.clone(), ]
        };
        let a = 1. + u.clone() * u.clone();
        let b = u.clone() * v.clone();
        let c = v.clone() * v.clone() - 1.;
        let f = -b.clone() / a.clone();
        let dsq = f.clone() * f.clone() - c.clone() / a.clone();
        let d = dsq.clone().sqrt();
        let [ x0, y0, x1, y1 ] = if cx.re == 0. {
            let x0 = f.clone() + d.clone();
            let x1 = f.clone() - d.clone();
            let y0 = u.clone() * x0.clone() + v.clone();
            let y1 = u.clone() * x1.clone() + v.clone();
            [ x0, y0, x1, y1 ]
        } else {
            let y0 = f.clone() + d.clone();
            let y1 = f.clone() - d.clone();
            let x0 = u.clone() * y0.clone() + v.clone();
            let x1 = u.clone() * y1.clone() + v.clone();
            [ x0, y0, x1, y1 ]
        };
        [ R2 { x: x0, y: y0 }, R2 { x: x1, y: y1 } ]
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

        let projected = d1.project(&d0);
        let unit_intersections = projected.unit_intersections();
        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let [ p0, p1 ] = unit_intersections.map(|p| invert(p, &d0));
        // println!("intersections_projected1:");
        // println!("{}", p0);
        // println!("{}", p1);
        // println!();

        assert_relative_eq!(p0, r2(0.5, vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000],  0.866, vec![ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(0.5, vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000], -0.866, vec![-0.289, 0.500, -0.577,  0.289, 0.500, -0.577]), epsilon = 1e-3);
    }

    #[test]
    fn intersections_projected2() {
        let c0 = Circle { c: R2 { x: 1., y: 1. }, r: 2. };
        let c1 = Circle { c: R2 { x: 3., y: 1. }, r: 2. };
        let d0 = c0.dual(0, 3);
        let d1 = c1.dual(3, 0);

        let projected = d1.project(&d0);
        let unit_intersections = projected.unit_intersections();
        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let [ p0, p1 ] = unit_intersections.map(|p| invert(p, &d0));
        // println!("intersections_projected2");
        // println!("{}", p0);
        // println!("{}", p1);
        // println!();

        assert_relative_eq!(p0, r2(2., vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000],  2.732, vec![ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(2., vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000], -0.732, vec![-0.289, 0.500, -0.577,  0.289, 0.500, -0.577]), epsilon = 1e-3);
    }

    #[test]
    fn intersections_projected01() {
        let c0 = Circle { c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { c: R2 { x: 0., y: 1. }, r: 1. };
        let d0 = c0.dual(0, 3);
        let d1 = c1.dual(3, 0);

        let projected = d1.project(&d0);
        let unit_intersections = projected.unit_intersections();
        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let [ p0, p1 ] = unit_intersections.map(|p| invert(p, &d0));
        // println!("intersections_projected2");
        // println!("{}", p0);
        // println!("{}", p1);
        // println!();

        assert_relative_eq!(p0, r2( 0.866, vec![ 0.500,  0.289,  0.577, 0.500, -0.289,  0.577], 0.500, vec![ 0.866, 0.500, 1.000, -0.866, 0.500, -1.000]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(-0.866, vec![ 0.500, -0.289, -0.577, 0.500,  0.289, -0.577], 0.500, vec![-0.866, 0.500, 1.000,  0.866, 0.500, -1.000]), epsilon = 1e-3);
    }
}
