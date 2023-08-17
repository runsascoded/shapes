use std::{fmt::Display, ops::{Mul, Add}};

use derive_more::From;
use serde::{Deserialize, Serialize};
use crate::{
    r2::R2,
    region::Region,
    intersection::Intersection, edge::Edge, dual::Dual
};

#[derive(Debug, Clone, Copy, From, Serialize, Deserialize)]
pub struct Circle<D> {
    pub idx: usize,
    pub c: R2<D>,
    pub r: D,
}

type D = Dual;

impl Circle<f64> {
    pub fn dual<'a>(&self, pre_zeros: usize, post_zeros: usize) -> Circle<D> {
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
        Circle::from((self.idx, c, r))
        // &Circle { c, r }
    }
    pub fn intersect(&self, o: &Circle<f64>) -> [ Intersection<D>; 2 ] {
        // let c0 = self.dual(0, 3);
        // let c1 = o.dual(3, 0);
        // c0.intersect(&c1)
        self.dual(0, 3).intersect(&o.dual(3, 0))
    }
}

impl Circle<D> {
    pub fn intersect(&self, c1: &Circle<D>) -> [ Intersection<D>; 2 ] {
        let c0 = self;
        let projected = c0.project(&c1);
        let unit_intersections = projected.unit_intersections();
        let invert = |p: R2<D>, c: &Circle<D>| c.invert(p);
        let points = unit_intersections.map(|p| invert(p, &c1));
        let intersections = points.map(|p| {
            let x = p.x.clone();
            let y = p.y.clone();
            let p = R2 { x: x.clone(), y: y.clone() };
            let t0 = c0.theta(p.clone());
            let t1 = c1.theta(p.clone());
            Intersection { x, y, c0idx: c0.idx, c1idx: c1.idx, t0, t1 }
        });
        intersections
        // let edge0 = Edge { c: c0, intersections: &intersections };
        // let edge1 = Edge { c: c1, intersections: &intersections };
        // let edges = vec![ edge0, edge1 ];
        // for edge in edges {
        //     let e = edge.clone();
        //     let [ i0, i1 ] = e.intersections;
        //     i0.add_edge(e);
        //     i1.add_edge(e);
        // }
        // let region = Region { edges, intersections: intersections.into() };
        // region
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
        Circle { idx: self.idx, c, r, }
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

impl Mul<f64> for Circle<f64> {
    type Output = Circle<f64>;
    fn mul(self, rhs: f64) -> Self::Output {
        Circle { idx: self.idx, c: self.c * rhs, r: self.r * rhs }
    }
}

impl Mul<i64> for Circle<f64> {
    type Output = Circle<f64>;
    fn mul(self, rhs: i64) -> Self::Output {
        Circle { idx: self.idx, c: self.c * (rhs as f64), r: self.r * (rhs as f64) }
    }
}

impl Add<R2<f64>> for Circle<f64> {
    type Output = Circle<f64>;
    fn add(self, rhs: R2<f64>) -> Self::Output {
        Circle { idx: self.idx, c: self.c + rhs, r: self.r }
    }
}

impl Add<R2<i64>> for Circle<f64> {
    type Output = Circle<f64>;
    fn add(self, rhs: R2<i64>) -> Self::Output {
        Circle { idx: self.idx, c: self.c + R2 { x: rhs.x as f64, y: rhs.y as f64 }, r: self.r }
    }
}

fn r2(x: f64, dx: Vec<f64>, y: f64, dy: Vec<f64>) -> R2<D> {
    R2 { x: Dual::new(x, dx), y: Dual::new(y, dy) }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn swap_v(v: Vec<f64>) -> Vec<f64> {
        [&v[3..], &v[..3]].concat()
    }
    fn swap(p: R2<Dual>) -> R2<Dual> {
        R2 { x: Dual::new(p.x.v(), swap_v(p.x.d())), y: Dual::new(p.y.v(), swap_v(p.y.d())) }
    }

    fn check(c0: Circle<f64>, c1: Circle<f64>, expected0: R2<D>, expected1: R2<D>) {
        let intersections = c0.intersect(&c1);
        // assert_eq!(region.n(), 2);
        let [ p0, p1 ] = intersections.map(|p| R2 { x: p.x, y: p.y });

        // println!("c0: {}", c0);
        // println!("c1: {}", c1);
        // println!("expected0: {}", expected0);
        // println!("expected1: {}", expected1);
        // println!();
        assert_relative_eq!(p0, expected0, epsilon = 1e-3);
        assert_relative_eq!(p1, expected1, epsilon = 1e-3);
    }

    fn test(c0: Circle<f64>, c1: Circle<f64>, expected0: R2<D>, expected1: R2<D>) {
        for dx in -1..2 {
            for dy in -1..2 {
                for scale in 1..3 {
                    // println!("check: dx: {}, dy: {}, scale: {}", dx, dy, scale);
                    let c0 = c0 * scale + R2 { x: dx, y: dy };
                    let c1 = c1 * scale + R2 { x: dx, y: dy };
                    let R2 { x: e0x, y: e0y } = expected0.clone();
                    let R2 { x: e1x, y: e1y } = expected1.clone();
                    let transform = |d: Dual, o: i64| Dual::new(d.v() * (scale as f64) + (o as f64), d.d().clone());
                    let e0 = R2 { x: transform(e0x, dx), y: transform(e0y, dy) };
                    let e1 = R2 { x: transform(e1x, dx), y: transform(e1y, dy) };
                    check(c0, c1, e0.clone(), e1.clone());
                    // The "dual" circles were created in the opposite order, swap the first and last 3 derivative components.
                    check(c1, c0, swap(e0), swap(e1));
                }
            }
        }
    }

    #[test]
    fn region_r() {
        test(
            Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. },
            Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. },
            r2(0.5, vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000 ],  0.866, vec![  0.289, 0.500,  0.577, -0.289, 0.500,  0.577 ]),
            r2(0.5, vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000 ], -0.866, vec![ -0.289, 0.500, -0.577,  0.289, 0.500, -0.577 ]),
        );
    }

    #[test]
    fn region_u() {
        test(
            Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. },
            Circle { idx: 1, c: R2 { x: 0., y: 1. }, r: 1. },
            r2( 0.866, vec![ 0.500,  0.289,  0.577, 0.500, -0.289,  0.577], 0.500, vec![ 0.866, 0.500, 1.000, -0.866, 0.500, -1.000 ]),
            r2(-0.866, vec![ 0.500, -0.289, -0.577, 0.500,  0.289, -0.577], 0.500, vec![-0.866, 0.500, 1.000,  0.866, 0.500, -1.000 ]),
        );
    }

    #[test]
    fn region_ur() {
        test(
            Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. },
            Circle { idx: 1, c: R2 { x: 1., y: 1. }, r: 1. },
            r2( 0., vec![ 0., 0., 0., 1., 0., -1. ], 1., vec![ 0., 1., 1., 0., 0.,  0. ]),
            r2( 1., vec![ 1., 0., 1., 0., 0.,  0. ], 0., vec![ 0., 0., 0., 0., 1., -1. ]),
        );
    }
}
