use std::{fmt::{Display, self}, ops::{Mul, Add, Neg, Div, Sub}};

use derive_more::From;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use crate::{
    dual::{D, Dual},
    r2::R2, shape::{Duals, Shape}, transform::{Projection, CanTransform}, transform::Transform::{Scale, Translate, self}, ellipses::xyrr::XYRR, sqrt::Sqrt, math::IsNormal
};

#[derive(Debug, Clone, Copy, From, PartialEq, Tsify, Serialize, Deserialize)]
pub struct Circle<D> {
    pub idx: usize,
    pub c: R2<D>,
    pub r: D,
}

impl<D: Eq> Eq for Circle<D> {}

impl Circle<f64> {
    pub fn dual(&self, duals: &Duals) -> Circle<D> {
        let x = Dual::new(self.c.x, duals[0].clone());
        let y = Dual::new(self.c.y, duals[1].clone());
        let r = Dual::new(self.r  , duals[2].clone());
        let c = R2 { x, y };
        Circle::from((self.idx, c, r))
    }
}

impl Circle<D> {
    pub fn v(&self) -> Circle<f64> {
        Circle { idx: self.idx, c: self.c.v(), r: self.r.v() }
    }
    pub fn n(&self) -> usize {
        self.c.x.d().len()
    }
}

pub trait UnitIntersectionsArg
    : Clone
    + fmt::Debug
    + Display
    + Into<f64>
    + IsNormal
    + Sqrt
    + Neg<Output = Self>
    + Add<Output = Self>
    + Add<f64, Output = Self>
    + Sub<Output = Self>
    + Sub<f64, Output = Self>
    + Mul<Output = Self>
    + Mul<f64, Output = Self>
    + Div<Output = Self>
    + Div<f64, Output = Self>
{}

impl UnitIntersectionsArg for f64 {}
impl UnitIntersectionsArg for Dual {}

impl<D: UnitIntersectionsArg> Circle<D>
where
    f64: Add<D, Output = D>
{
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        let cx = self.c.x.clone();
        let cy = self.c.y.clone();
        let r = self.r.clone();
        let cx2 = cx.clone() * cx.clone();
        let cy2 = cy.clone() * cy.clone();
        let r2 = r.clone() * r.clone();
        let z = (1. + cx2.clone() + cy2.clone() - r2.clone()) / 2.;
        let [ u, v ] = if cx.clone().into() == 0. {
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
        let [ x0, y0, x1, y1 ] = if cx.into() == 0. {
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
            intersections.push(R2 { x: x0.clone(), y: y0.clone() });
        }
        if x1.is_normal() && y1.is_normal() {
            intersections.push(R2 { x: x1.clone(), y: y1.clone() });
        }
        intersections
    }
}

impl<
    D
    : Clone
    + PartialEq
    + Mul<Output = D>
> CanTransform<D> for Circle<D>
where
    R2<D>
    : Add<Output = R2<D>>
    + Mul<Output = R2<D>>
    + Mul<D, Output = R2<D>>,
{
    type Output = Shape<D>;
    fn transform(&self, transform: &Transform<D>) -> Shape<D> {
        match transform {
            Translate(v) =>
                Circle {
                    idx: self.idx,
                    c: self.c.clone() + v.clone(),
                    r: self.r.clone()
                }.into(),
            Scale(s) => {
                let R2 { x, y } = s.clone();
                if x == y {
                    Circle {
                        idx: self.idx,
                        c: self.c.clone() * x.clone(),
                        r: self.r.clone() * x,
                    }.into()
                } else {
                    XYRR {
                        idx: self.idx,
                        c: s.clone() * self.c.clone(),
                        r: s.clone() * self.r.clone(),
                    }.into()
                }
            },
            // Rotate(a) => {
            //     let c = rotate::Rotate::rotate(&self.c.clone(), a);
            //     // let c = self.c.clone().rotate(a);
            //     let r = self.r.clone();
            //     Circle { idx: self.idx, c, r, }.into()
            // },
        }
    }
}

impl<D: Clone + Display> Circle<D>
where
    R2<D>: Neg<Output = R2<D>>,
    f64: Div<D, Output = D>,
{
    pub fn projection(&self) -> Projection<D> {
        let c = self.c.clone();
        let r = self.r.clone();
        let translate = Translate(-c.clone());
        let scale = Scale(R2 { x: 1. / r.clone(), y: 1. / r.clone() });
        let transforms = vec![ translate, scale ];
        Projection(transforms)
    }
}

impl<D: Clone> Circle<D> {
    pub fn xyrr(&self) -> XYRR<D> {
        XYRR {
            idx: self.idx,
            c: self.c.clone(),
            r: R2 { x: self.r.clone(), y: self.r.clone() },
        }
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

    use crate::intersect::Intersect;

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
