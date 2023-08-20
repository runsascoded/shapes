use std::{fmt::Display, ops::{Mul, Add}, rc::Rc, cell::RefCell, f64::consts::PI};

use derive_more::From;
use num_traits::Float;
use serde::{Deserialize, Serialize};
use crate::{
    r2::R2,
    intersection::Intersection, dual::{D, Dual},
};

#[derive(Debug, Clone, Copy, From, Serialize, Deserialize)]
pub struct Circle<D> {
    pub idx: usize,
    pub c: R2<D>,
    pub r: D,
}

pub type C = Rc<RefCell<Circle<D>>>;
pub type Duals = [Vec<f64>; 3];
pub type Split = (Circle<f64>, Duals);

impl<D: Clone + Float> Circle<D> {
    pub fn point(&self, t: D) -> R2<D> {
        let x = self.c.x.clone() + self.r.clone() * t.clone().cos();
        let y = self.c.y.clone() + self.r.clone() * t.clone().sin();
        R2 { x, y }
    }
}

impl Circle<f64> {
    pub fn dual<'a>(&self, duals: &Duals) -> Circle<D> {
        let x = Dual::new(self.c.x, duals[0].clone());
        let y = Dual::new(self.c.y, duals[1].clone());
        let r = Dual::new(self.r  , duals[2].clone());
        let c = R2 { x, y };
        Circle::from((self.idx, c, r))
    }
    pub fn intersect(&self, o: &Circle<f64>) -> Vec<Intersection> {
        let c0 = self.dual(&[ vec![ 1., 0., 0., 0., 0., 0., ], vec![ 0., 1., 0., 0., 0., 0., ], vec![ 0., 0., 1., 0., 0., 0., ] ]);
        let c1 = o.dual(&[ vec![ 0., 0., 0., 1., 0., 0., ], vec![ 0., 0., 0., 0., 1., 0., ], vec![ 0., 0., 0., 0., 0., 1., ] ]);
        c0.intersect(&c1)
    }
    pub fn arc_midpoint(&self, t0: f64, mut t1: f64) -> R2<f64> {
        if t1 < t0 {
            t1 += 2. * PI;
        }
        let t = (t0 + t1) / 2.;
        self.point(t)
    }
    pub fn contains(&self, p: &R2<f64>) -> bool {
        let x = p.x - self.c.x;
        let y = p.y - self.c.y;
        let r2 = self.r * self.r;
        let x2 = x * x;
        let y2 = y * y;
        x2 + y2 <= r2
    }
}

impl Circle<D> {
    pub fn v(&self) -> Circle<f64> {
        Circle { idx: self.idx, c: self.c.v(), r: self.r.v() }
    }
    pub fn split(&self) -> Split {
        ( self.v(), [ self.c.x.d().clone(), self.c.y.d().clone(), self.r.d().clone() ] )
    }
    pub fn intersect(&self, c1: &Circle<D>) -> Vec<Intersection> {
        let c0 = self;
        let projected = c0.project(&c1);
        let unit_intersections = projected.unit_intersections();
        let invert = |p: &R2<D>, c: &Circle<D>| c.invert(p);
        let points = unit_intersections.iter().map(|p| invert(p, &c1));
        let intersections = points.map(|p| {
            let x = p.x.clone();
            let y = p.y.clone();
            let p = R2 { x: x.clone(), y: y.clone() };
            let t0 = c0.theta(p.clone());
            let t1 = c1.theta(p.clone());
            Intersection { x, y, c0idx: c0.idx, c1idx: c1.idx, t0, t1, edges: Vec::new(), }
        });
        intersections.collect()
    }
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
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
        let mut intersections: Vec<R2<D>> = Vec::new();
        if x0.is_normal() && y0.is_normal() {
            intersections.push(R2 { x: x0, y: y0 });
        }
        if x1.is_normal() && y1.is_normal() {
            intersections.push(R2 { x: x1, y: y1 });
        }
        intersections
    }
    pub fn project(&self, o: &Circle<D>) -> Self {
        let c = (self.c.clone() - o.c.clone()) / o.r.clone();
        let r = self.r.clone() / o.r.clone();
        Circle { idx: self.idx, c, r, }
    }
    pub fn invert(&self, p: &R2<D>) -> R2<D> {
        self.c.clone() + p * self.r.clone()
    }
    pub fn theta(&self, p: R2<D>) -> D {
        let x = p.x.clone() - self.c.x.clone();
        let y = p.y.clone() - self.c.y.clone();
        y.clone().atan2(x.clone())
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
        Circle { idx: self.idx, c: self.c * &rhs, r: self.r * rhs }
    }
}

impl Mul<i64> for Circle<f64> {
    type Output = Circle<f64>;
    fn mul(self, rhs: i64) -> Self::Output {
        Circle { idx: self.idx, c: self.c * &(rhs as f64), r: self.r * (rhs as f64) }
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
        let p0 = intersections[0].p();
        let p1 = intersections[1].p();

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
