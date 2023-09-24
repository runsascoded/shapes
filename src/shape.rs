use std::{ops::{Neg, Add, Sub, Mul, Div}, fmt};

use derive_more::{From, Deref, Display};
use log::debug;
use serde::{Deserialize, Serialize};
use tsify::{declare, Tsify};

use crate::{dual::{D, Dual}, circle::{self, Circle}, ellipses::{xyrr::{self, XYRR, UnitCircleGap}, xyrrt::{self, XYRRT, LevelArg}}, zero::Zero, transform::{Transform, CanProject, CanTransform, HasProjection, Projection}, r2::R2, math::recip::Recip, intersect::{IntersectShapesArg, UnitCircleIntersections}, duals::InitDuals};

#[declare]
pub type Duals = Vec<Vec<f64>>;
#[declare]
pub type Input = (Shape<f64>, Duals);
pub type InputSpec = (Shape<f64>, Vec<bool>);

#[derive(Debug, Display, Clone, From, PartialEq, Serialize, Deserialize, Tsify)]
pub enum Shape<D> {
    Circle(circle::Circle<D>),
    XYRR(xyrr::XYRR<D>),
    XYRRT(xyrrt::XYRRT<D>),
}

pub struct Shapes {}
impl Shapes {
    pub fn from<const N: usize>(input_specs: [InputSpec; N]) -> [ Shape<D>; N ] {
        InitDuals::from(input_specs)
    }
    pub fn from_vec(input_specs: &Vec<InputSpec>) -> Vec<Shape<D>> {
        InitDuals::from_vec(input_specs)
    }
}

#[derive(Deref)]
pub struct CoordGetter(pub Box<dyn Fn(Shape<f64>) -> f64>);

pub fn circle<D>(cx: D, cy: D, r: D) -> Shape<D> {
    Shape::Circle(Circle { c: R2 { x: cx, y: cy }, r })
}
pub fn xyrr<D>(cx: D, cy: D, rx: D, ry: D) -> Shape<D> {
    Shape::XYRR(XYRR { c: R2 { x: cx, y: cy }, r: R2 { x: rx, y: ry } })
}
pub fn xyrrt<D>(cx: D, cy: D, rx: D, ry: D, t: D) -> Shape<D> {
    Shape::XYRRT(XYRRT { c: R2 { x: cx, y: cy }, r: R2 { x: rx, y: ry }, t })
}

impl<D> Shape<D> {
    pub fn getters(&self, shape_idx: usize) -> Vec<CoordGetter> {
        match self {
            Shape::Circle(_) => Circle::getters().map(|f| {
                CoordGetter(Box::new(move |s: Shape<f64>| match s {
                    Shape::Circle(c) => f(c),
                    _ => panic!("Expected Circle at idx {}", shape_idx),
                }))
            }).into_iter().collect(),
            Shape::XYRR(_) => XYRR::getters().map(|f| {
                CoordGetter(Box::new(move |s: Shape<f64>| match s {
                    Shape::XYRR(e) => f(e),
                    _ => panic!("Expected XYRR at idx {}", shape_idx),
                }))
            }).into_iter().collect(),
            Shape::XYRRT(_) => XYRRT::getters().map(|f| {
                CoordGetter(Box::new(move |s: Shape<f64>| match s {
                    Shape::XYRRT(e) => f(e),
                    _ => panic!("Expected XYRRT at idx {}", shape_idx),
                }))
            }).into_iter().collect(),
        }
    }
}

impl Shape<f64> {
    pub fn dual(&self, duals: &Duals) -> Shape<D> {
        match self {
            Shape::Circle(c) => Shape::Circle(c.dual(duals)),
            Shape::XYRR(e) => Shape::XYRR(e.dual(duals)),
            Shape::XYRRT(e) => Shape::XYRRT(e.dual(duals)),
        }
    }
}

impl<D: Clone> Shape<D> {
    pub fn center(&self) -> R2<D> {
        match self {
            Shape::Circle(c) => c.c.clone(),
            Shape::XYRR(e) => e.c.clone(),
            Shape::XYRRT(e) => e.c.clone(),
        }
    }
}

impl<D: Clone> Shape<D> {
    pub fn c(&self) -> R2<D> {
        match self {
            Shape::Circle(c) => c.c.clone(),
            Shape::XYRR(e) => e.c.clone(),
            Shape::XYRRT(e) => e.c.clone(),
        }
    }
}

impl Shape<D> {
    pub fn v(&self) -> Shape<f64> {
        match self {
            Shape::Circle(c) => Shape::Circle(c.v()),
            Shape::XYRR(e) => Shape::XYRR(e.v()),
            Shape::XYRRT(e) => Shape::XYRRT(e.v()),
        }
    }
    pub fn n(&self) -> usize {
        match self {
            Shape::Circle(c) => c.n(),
            Shape::XYRR(e) => e.n(),
            Shape::XYRRT(e) => e.n(),
        }
    }
}

impl<D: Zero> Shape<D> {
    pub fn zero(&self) -> D {
        match self {
            Shape::Circle(c) => Zero::zero(&c.r),
            Shape::XYRR(e) => Zero::zero(&e.c.x),
            Shape::XYRRT(e) => Zero::zero(&e.c.x),
        }
    }
}

impl<D: Clone + fmt::Display + Neg<Output = D> + Recip> HasProjection<D> for Shape<D>
where
    R2<D>: Neg<Output = R2<D>>,
{
    fn projection(&self) -> Projection<D> {
        match self {
            Shape::Circle(c) => c.projection(),
            Shape::XYRR(e) => e.projection(),
            Shape::XYRRT(e) => e.projection(),
        }
    }
}

impl<
    D
    : circle::TransformD
    + xyrr::TransformD
> CanTransform<D> for Shape<D>
where
    R2<D>
    : circle::TransformR2<D>
    + xyrr::TransformR2<D>,
{
    type Output = Shape<D>;
    fn transform(&self, transform: &Transform<D>) -> Shape<D> {
        match self {
            Shape::Circle(c) => c.transform(transform),
            Shape::XYRR(e) => e.transform(transform),
            Shape::XYRRT(e) => e.transform(transform),
        }
    }
}

impl<D: LevelArg + UnitCircleGap> Shape<D> {
    pub fn unit_circle_gap(&self) -> Option<D> {
        match self {
            Shape::Circle(c) => c.unit_circle_gap(),
            Shape::XYRR(e) => e.unit_circle_gap(),
            Shape::XYRRT(e) => e.unit_circle_gap(),
        }
    }
}

impl Shape<Dual> {
    pub fn step(&self, step_vec: &Vec<f64>) -> Shape<Dual> {
        match self {
            Shape::Circle(s) => {
                let [ dx, dy, dr ]: [f64; 3] = s.duals().map(|d| d.iter().zip(step_vec).map(|(mask, step)| mask * step).sum());
                let Circle { c, r } = s.clone();
                let c = R2 { x: c.x + dx, y: c.y + dy, };
                let r = r + dr;
                Shape::Circle(Circle { c, r })
            },
            Shape::XYRR(e) => {
                let [ dcx, dcy, drx, dry ]: [f64; 4] = e.duals().map(|d| d.iter().zip(step_vec).map(|(mask, step)| mask * step).sum());
                let XYRR { c, r } = e.clone();
                let c = R2 { x: c.x + dcx, y: c.y + dcy, };
                let r = R2 { x: r.x + drx, y: r.y + dry, };
                Shape::XYRR(XYRR { c, r })
            },
            Shape::XYRRT(e) => {
                let [ dcx, dcy, drx, dry, dt ]: [f64; 5] = e.duals().map(|d| d.iter().zip(step_vec).map(|(mask, step)| mask * step).sum());
                let XYRRT { c, r, t } = e.clone();
                let c = R2 { x: c.x + dcx, y: c.y + dcy, };
                let r = R2 { x: r.x + drx, y: r.y + dry, };
                let t = t + dt;
                Shape::XYRRT(XYRRT { c, r, t })
            },
        }
    }
    pub fn duals(&self) -> Duals {
        match self {
            Shape::Circle(s) => s.duals().to_vec(),
            Shape::XYRR(e) => e.duals().to_vec(),
            Shape::XYRRT(e) => e.duals().to_vec(),
        }
    }
}

impl<D: IntersectShapesArg> Shape<D>
where
    Shape<D>: CanTransform<D, Output = Shape<D>>,
    R2<D>: CanProject<D, Output = R2<D>>,
    f64
    : Add<D, Output = D>
    + Sub<D, Output = D>
    + Mul<D, Output = D>
    + Div<D, Output = D>,
{
    pub fn _intersect(&self, o: &Shape<D>) -> Vec<R2<D>> {
        debug!("Intersecting:");
        debug!("  self: {}", self);
        debug!("  other: {}", o);
        let projection = self.projection();
        debug!("  projection: {:?}", projection);
        let projected = o.apply(&projection);
        debug!("  projected: {}", projected);
        let rev = -projection.clone();
        // debug!("reverse projection: {:?}", rev);
        let points = projected.unit_circle_intersections().iter().map(|p| p.apply(&rev)).collect();
        // debug!("points: {:?}", points.clone().collect::<Vec<_>>());
        // debug!();
        points
    }
}