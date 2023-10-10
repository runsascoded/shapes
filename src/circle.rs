use std::{
    f64::consts::PI,
    fmt::{self, Display},
    ops::{Add, Div, Mul, Neg, Sub},
};

use crate::{
    coord_getter::{coord_getter, CoordGetter},
    dual::{Dual, D},
    ellipses::xyrr::{UnitCircleGap, XYRR},
    intersect::Intersect,
    math::{is_normal::IsNormal, recip::Recip},
    r2::R2,
    rotate::{Rotate as _Rotate, RotateArg},
    shape::{AreaArg, Duals, Shape, Shapes},
    sqrt::Sqrt,
    to::To,
    transform::Transform::{self, Rotate, Scale, ScaleXY, Translate},
    transform::{CanTransform, Projection},
};
use derive_more::From;
use log::debug;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Debug, Clone, Copy, From, PartialEq, Tsify, Serialize, Deserialize)]
pub struct Circle<D> {
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
        Circle::from((c, r))
    }

    pub fn getters() -> [ CoordGetter<Self>; 3 ] {[
        coord_getter("cx", |c: Self| c.c.x),
        coord_getter("cy", |c: Self| c.c.y),
        coord_getter( "r", |c: Self| c.r  ),
    ]}
    pub fn at_y(&self, y: f64) -> Vec<f64> {
        let uy = (y - self.c.y) / self.r;
        let uy2 = uy * uy;
        if uy2 > 1. {
            return vec![];
        }
        let ux1 = (1. - uy2).sqrt();
        let ux0 = -ux1;
        let x0 = self.c.x + ux0 * self.r;
        let x1 = self.c.x + ux1 * self.r;
        if x0 != x1 {
            vec![ x0, x1 ]
        } else {
            vec![ x0 ]
        }
    }
    pub fn names(&self) -> [String; 3] { Self::getters().map(|g| g.name).into() }
    pub fn vals(&self) -> [f64; 3] { [ self.c.x, self.c.y, self.r ] }
}

impl Circle<D> {
    pub fn v(&self) -> Circle<f64> {
        Circle {
            c: self.c.v(),
            r: self.r.v(),
        }
    }
    pub fn n(&self) -> usize {
        self.c.x.d().len()
    }
    pub fn duals(&self) -> [Vec<f64>; 3] {
        [ self.c.x.d().clone(), self.c.y.d().clone(), self.r.d().clone() ]
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
{
}

impl UnitIntersectionsArg for f64 {}
impl UnitIntersectionsArg for Dual {}

impl<D: UnitIntersectionsArg> Circle<D>
where
    f64: Add<D, Output = D>,
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
            intersections.push(R2 {
                x: x0.clone(),
                y: y0.clone(),
            });
        }
        if x1.is_normal() && y1.is_normal() {
            intersections.push(R2 {
                x: x1.clone(),
                y: y1.clone(),
            });
        }
        debug!(
            "Circle::unit_intserctions: {}, {} intersections: {:?}",
            self,
            intersections.len(),
            intersections
        );
        intersections
    }
}

impl<O, I: To<O>> To<Circle<O>> for Circle<I> {
    fn to(self) -> Circle<O> {
        Circle {
            c: self.c.to(),
            r: self.r.to(),
        }
    }
}

impl<D: Clone> To<XYRR<D>> for Circle<D> {
    fn to(self) -> XYRR<D> {
        XYRR {
            c: self.c,
            r: R2 {
                x: self.r.clone(),
                y: self.r.clone(),
            },
        }
    }
}

impl Mul<R2<f64>> for R2<Dual> {
    type Output = R2<Dual>;
    fn mul(self, rhs: R2<f64>) -> Self::Output {
        R2 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}
pub trait TransformD
: Clone
+ Mul<Output = Self>
+ Mul<f64, Output = Self>
+ RotateArg
{}
impl TransformD for f64 {}
impl TransformD for Dual {}
pub trait TransformR2<D>
: Add<R2<D>, Output = R2<D>>
+ Mul<R2<D>, Output = R2<D>>
+ Mul<D, Output = R2<D>>
+ To<R2<f64>>
{}
impl TransformR2<f64> for R2<f64> {}
impl TransformR2<Dual> for R2<Dual> {}

impl<D: TransformD> CanTransform<D> for Circle<D>
where
    R2<D>: TransformR2<D>,
{
    type Output = Shape<D>;
    fn transform(&self, transform: &Transform<D>) -> Shape<D> {
        match transform {
            Translate(v) => Circle {
                c: self.c.clone() + v.clone(),
                r: self.r.clone(),
            }
            .into(),
            Scale(s) => {
                let r = &self.r;
                Circle {
                    c: self.c.clone() * s.clone(),
                    r: r.clone() * s.clone(),
                }
                .into()
            }
            ScaleXY(s) => {
                let r = &self.r;
                XYRR {
                    c: self.c.clone() * s.clone(),
                    r: s * r.clone(),
                }
                .into()
            }
            Rotate(a) => {
                // let c = rotate::Rotate::rotate(&self.c.clone(), a);
                let c = self.c.clone().rotate(a);
                let r = self.r.clone();
                Circle { c, r }.into()
            }
        }
    }
}

impl<D: AreaArg> Circle<D> {
    pub fn area(&self) -> D {
        self.r.clone() * self.r.clone() * PI
    }
}

impl<D: Clone + Display + Recip> Circle<D>
where
    R2<D>: Neg<Output = R2<D>>,
{
    pub fn projection(&self) -> Projection<D> {
        let c = self.c.clone();
        let r = self.r.clone();
        let translate = Translate(-c.clone());
        let scale = Scale(r.recip());
        let transforms = vec![ translate, scale ];
        Projection(transforms)
    }
}

impl<D: Clone> Circle<D> {
    pub fn xyrr(&self) -> XYRR<D> {
        XYRR {
            c: self.c.clone(),
            r: R2 {
                x: self.r.clone(),
                y: self.r.clone(),
            },
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
        Circle { c: self.c * &rhs, r: self.r * rhs }
    }
}

impl Mul<i64> for Circle<f64> {
    type Output = Circle<f64>;
    fn mul(self, rhs: i64) -> Self::Output {
        Circle { c: self.c * &(rhs as f64), r: self.r * (rhs as f64) }
    }
}

impl Add<R2<f64>> for Circle<f64> {
    type Output = Circle<f64>;
    fn add(self, rhs: R2<f64>) -> Self::Output {
        Circle { c: self.c + rhs, r: self.r }
    }
}

impl Add<R2<i64>> for Circle<f64> {
    type Output = Circle<f64>;
    fn add(self, rhs: R2<i64>) -> Self::Output {
        Circle { c: self.c + R2 { x: rhs.x as f64, y: rhs.y as f64 }, r: self.r }
    }
}

impl Intersect<Circle<f64>, D> for Circle<f64> {
    fn intersect(&self, other: &Circle<f64>) -> Vec<R2<D>> {
        let [ s0, s1 ] = Shapes::from([
            (Shape::Circle(* self), vec![ true; 3 ]),
            (Shape::Circle(*other), vec![ true; 3 ]),
        ]);
        s0.intersect(&s1)
    }
}

impl<D: UnitCircleGap> Circle<D> {
    pub fn unit_circle_gap(&self) -> Option<D> {
        let distance = self.c.norm() - 1. - self.r.clone();
        if distance.clone().into() > 0. {
            Some(distance)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::NAN;

    use super::*;

    use crate::{
        duals::{D, Z},
        intersect::Intersect,
    };

    pub fn r2(x: f64, dx: Vec<f64>, y: f64, dy: Vec<f64>) -> R2<D> {
        R2 {
            x: Dual::new(x, dx),
            y: Dual::new(y, dy),
        }
    }

    fn swap_v(v: Vec<f64>) -> Vec<f64> {
        [&v[3..], &v[..3]].concat()
    }
    fn swap(p: R2<Dual>) -> R2<Dual> {
        R2 {
            x: Dual::new(p.x.v(), swap_v(p.x.d())),
            y: Dual::new(p.y.v(), swap_v(p.y.d())),
        }
    }

    fn check(c0: Circle<f64>, c1: Circle<f64>, expected0: R2<D>, expected1: R2<D>) {
        let intersections = c0.intersect(&c1);
        let p0 = intersections[0].clone();
        let p1 = intersections[1].clone();

        // println!("c0: {}", c0);
        // println!("c1: {}", c1);
        // println!("expected0: {}", expected0);
        // println!("expected1: {}", expected1);
        // println!("p0: {}", p0);
        // println!("p1: {}", p1);
        // println!("d0: {}", p0.clone() - expected0.clone());
        // println!("d1: {}", p1.clone() - expected1.clone());
        // println!();
        assert_relative_eq!(p0, expected0, epsilon = 1e-3);
        assert_relative_eq!(p1, expected1, epsilon = 1e-3);
    }

    fn test(c0: Circle<f64>, c1: Circle<f64>, expected0: R2<D>, expected1: R2<D>) {
        for dx in -1..2 {
            for dy in -1..2 {
                for scale in 1..3 {
                    let c0 = c0 * scale + R2 { x: dx, y: dy };
                    let c1 = c1 * scale + R2 { x: dx, y: dy };
                    let R2 { x: e0x, y: e0y } = expected0.clone();
                    let R2 { x: e1x, y: e1y } = expected1.clone();
                    let transform = |d: Dual, o: i64| {
                        Dual::new(d.v() * (scale as f64) + (o as f64), d.d().clone())
                    };
                    let e0 = R2 {
                        x: transform(e0x, dx),
                        y: transform(e0y, dy),
                    };
                    let e1 = R2 {
                        x: transform(e1x, dx),
                        y: transform(e1y, dy),
                    };
                    check(c0, c1, e0.clone(), e1.clone());
                    // The "dual" circles were created in the opposite order, swap the first and last 3 derivative components.
                    check(c1, c0, swap(e0), swap(e1));
                }
            }
        }
    }

    #[test]
    fn tangent_circles() {
        let [ s0, s1 ] = Shapes::from([
            (Shape::Circle(Circle { c: R2 { x: 0., y: 0. }, r: 2. }), vec![ Z, Z, Z ]),
            (Shape::Circle(Circle { c: R2 { x: 3., y: 0. }, r: 1. }), vec![ D, Z, Z ]),
        ]);
        let ps = s0.intersect(&s1);
        assert_eq!(ps, vec![
            // Heads up! Tangent points have NAN gradients. [`Scene`] detects/filters them, but at this level of the stack they are passed along
            R2 { x: Dual::new(2.0, vec![NAN]), y: Dual::new(0.0, vec![NAN]) },
            R2 { x: Dual::new(2.0, vec![NAN]), y: Dual::new(0.0, vec![NAN]) }
        ]);
    }

    #[test]
    fn region_r() {
        test(
            Circle { c: R2 { x: 0., y: 0. }, r: 1. },
            Circle { c: R2 { x: 1., y: 0. }, r: 1. },
            r2(0.5, vec![ 0.500,  0.866, 1.000, 0.500, -0.866, -1.000 ],  0.866, vec![  0.289, 0.500,  0.577, -0.289, 0.500,  0.577 ]),
            r2(0.5, vec![ 0.500, -0.866, 1.000, 0.500,  0.866, -1.000 ], -0.866, vec![ -0.289, 0.500, -0.577,  0.289, 0.500, -0.577 ]),
        );
    }

    #[test]
    fn region_u() {
        test(
            Circle { c: R2 { x: 0., y: 0. }, r: 1. },
            Circle { c: R2 { x: 0., y: 1. }, r: 1. },
            r2( 0.866, vec![ 0.500,  0.289,  0.577, 0.500, -0.289,  0.577], 0.500, vec![ 0.866, 0.500, 1.000, -0.866, 0.500, -1.000 ]),
            r2(-0.866, vec![ 0.500, -0.289, -0.577, 0.500,  0.289, -0.577], 0.500, vec![-0.866, 0.500, 1.000,  0.866, 0.500, -1.000 ]),
        );
    }

    #[test]
    fn region_ur() {
        test(
            Circle { c: R2 { x: 0., y: 0. }, r: 1. },
            Circle { c: R2 { x: 1., y: 1. }, r: 1. },
            r2( 0., vec![ 0., 0., 0., 1., 0., -1. ], 1., vec![ 0., 1., 1., 0., 0.,  0. ]),
            r2( 1., vec![ 1., 0., 1., 0., 0.,  0. ], 0., vec![ 0., 0., 0., 0., 1., -1. ]),
        );
    }
}
