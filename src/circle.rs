use std::{f64::consts::PI, fmt::Display};
// use derive_more::{Clone};

use nalgebra::{Const, SMatrix, SVector, RowDVector, Dyn, Matrix};
use num_dual::{DualNum, jacobian, DualVec, Derivative, DualVec64, DualDVec64};
use serde::{Deserialize, Serialize};
use crate::{
    r2::R2,
    region::Region,
    intersection::Intersection, edge::Edge, dual::Dual
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Circle<D> {
    pub c: R2<D>,
    pub r: D,
}

type D = Dual;

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
        let c = R2 { x, y };
        Circle { c, r }
    }
    pub fn intersect(&self, o: &Circle<f64>) -> Region<D> {
        let sd = self.dual(0, 3);
        let od = o.dual(3, 0);
        let projected = sd.project(&od);
        // println!("projected: {}", projected);
        // println!();

        let unit_intersections = projected.unit_intersections();
        // println!("unit_intersections:");
        // println!("{}", unit_intersections[0]);
        // println!("{}", unit_intersections[1]);
        // println!();

        let invert = |p: R2<D>, od: &Circle<D>| od.invert(p);
        let points = unit_intersections.map(|p| invert(p, &od));
        let [ p0, p1 ] = points.clone();
        // println!("pre-ints points:");
        // println!("{}", p0);
        // println!("{}", p1);
        // println!();

        let intersections = points.map(|p| Intersection { x: p.x.clone(), y: p.y.clone(), c1: sd.clone(), c2: od.clone() });
        let edge0 = Edge { c: sd, intersections: intersections.clone() };
        let edge1 = Edge { c: od, intersections: intersections.clone() };
        let region = Region { edges: vec![ edge0, edge1 ], intersections: intersections.into() };
        region
    }
}

impl Circle<D> {
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
    pub fn theta(&self, p: R2<D>) -> D {
        let x = p.x.clone() - self.c.x.clone();
        let y = p.y.clone() - self.c.y.clone();
        let theta = (y.clone() / x.clone()).atan();
        theta
    }
}

impl<D: Display> Display for Circle<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "C({:.3}, {:.3}, {:.3})", self.c.x, self.c.y, self.r)
    }
}

fn r2(x: f64, dx: Vec<f64>, y: f64, dy: Vec<f64>) -> R2<D> {
    R2 { x: Dual::new(x, dx), y: Dual::new(y, dy) }
}


#[cfg(test)]
mod tests {
    use super::*;

    use nalgebra::{Const, Matrix3x1};
    use num_dual::DualVec64;

struct Case {
    pub circles: [Circle<f64>; 2],
    pub expected: [R2<D>; 2],
}

    fn test(case: Case) {
        let Case { circles, expected } = case;
        let [ c0, c1 ] = circles;
        let [ expected0, expected1 ] = expected;
        let region = c0.intersect(&c1);
        assert_eq!(region.n(), 2);
        let [ p0, p1 ] = [region.intersections[0].clone(), region.intersections[1].clone()].map(|p| R2 { x: p.x, y: p.y });
        assert_relative_eq!(p0, expected0, epsilon = 1e-3);
        assert_relative_eq!(p1, expected1, epsilon = 1e-3);

        let swap = |v: Vec<f64>| [&v[3..], &v[..3]].concat();
        let expected0 = R2 { x: Dual::new(expected0.x.v(), swap(expected0.x.d())), y: Dual::new(expected0.y.v(), swap(expected0.y.d())) };
        let expected1 = R2 { x: Dual::new(expected1.x.v(), swap(expected1.x.d())), y: Dual::new(expected1.y.v(), swap(expected1.y.d())) };

        let region = c1.intersect(&c0);
        assert_eq!(region.n(), 2);
        let [ p0, p1 ] = [region.intersections[0].clone(), region.intersections[1].clone()].map(|p| R2 { x: p.x, y: p.y });
        assert_relative_eq!(p0, expected0, epsilon = 1e-3);
        assert_relative_eq!(p1, expected1, epsilon = 1e-3);
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
        println!("intersections_projected2:");
        println!("{}", p0);
        println!("{}", p1);
        println!();

        assert_relative_eq!(p0, r2(2., vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000],  2.732, vec![ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(2., vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000], -0.732, vec![-0.289, 0.500, -0.577,  0.289, 0.500, -0.577]), epsilon = 1e-3);
    }
    #[test]
    fn region2() {
        test(
            Case {
                circles: [
                    Circle { c: R2 { x: 1., y: 1. }, r: 2. },
                    Circle { c: R2 { x: 3., y: 1. }, r: 2. },
                ],
                expected: [
                    r2(2., vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000],  2.732, vec![ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577]),
                    r2(2., vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000], -0.732, vec![-0.289, 0.500, -0.577,  0.289, 0.500, -0.577]),
                ],
            }
        );
        // let c0 = Circle { c: R2 { x: 1., y: 1. }, r: 2. };
        // let c1 = Circle { c: R2 { x: 3., y: 1. }, r: 2. };

        // let expected0 = r2(2., vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000],  2.732, vec![ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577]);
        // let expected1 = r2(2., vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000], -0.732, vec![-0.289, 0.500, -0.577,  0.289, 0.500, -0.577]);

        // let region = c0.intersect(&c1);
        // assert_eq!(region.n(), 2);
        // let [ p0, p1 ] = [region.intersections[0].clone(), region.intersections[1].clone()].map(|p| R2 { x: p.x, y: p.y });
        // assert_relative_eq!(p0, expected0, epsilon = 1e-3);
        // assert_relative_eq!(p1, expected1, epsilon = 1e-3);

        // let region = c1.intersect(&c0);
        // assert_eq!(region.n(), 2);
        // let [ p0, p1 ] = [region.intersections[0].clone(), region.intersections[1].clone()].map(|p| R2 { x: p.x, y: p.y });
        // assert_relative_eq!(p0, expected0, epsilon = 1e-3);
        // assert_relative_eq!(p1, expected1, epsilon = 1e-3);
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
        // println!("intersections_projected01");
        // println!("{}", p0);
        // println!("{}", p1);
        // println!();

        assert_relative_eq!(p0, r2( 0.866, vec![ 0.500,  0.289,  0.577, 0.500, -0.289,  0.577], 0.500, vec![ 0.866, 0.500, 1.000, -0.866, 0.500, -1.000]), epsilon = 1e-3);
        assert_relative_eq!(p1, r2(-0.866, vec![ 0.500, -0.289, -0.577, 0.500,  0.289, -0.577], 0.500, vec![-0.866, 0.500, 1.000,  0.866, 0.500, -1.000]), epsilon = 1e-3);
    }
}
