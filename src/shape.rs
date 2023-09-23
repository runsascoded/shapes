use std::{ops::{Neg, Add, Sub, Mul, Div}, fmt};

use derive_more::{From, Display};
use log::debug;
use serde::{Deserialize, Serialize};
use tsify::{declare, Tsify};

use crate::{dual::D, circle::{self, Circle}, ellipses::{xyrr::{self, XYRR, UnitCircleGap}, xyrrt::{self, XYRRT, LevelArg}}, zero::Zero, transform::{Transform, CanProject, CanTransform, HasProjection, Projection}, r2::R2, math::recip::Recip, intersect::{IntersectShapesArg, UnitCircleIntersections}};

#[declare]
pub type Duals = Vec<Vec<f64>>;
#[declare]
pub type Input = (Shape<f64>, Duals);

#[derive(Debug, Display, Clone, From, PartialEq, Serialize, Deserialize, Tsify)]
pub enum Shape<D> {
    Circle(circle::Circle<D>),
    XYRR(xyrr::XYRR<D>),
    XYRRT(xyrrt::XYRRT<D>),
}

pub struct Shapes {}
impl Shapes {
    pub fn from(inputs: &Vec<Input>) -> Vec<Shape<D>> {
        inputs.iter().map(|(c, duals)| c.dual(duals)).collect()
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

impl Shape<f64> {
    pub fn step(&self, duals: &Duals, step_vec: &Vec<f64>) -> Input {
        (
                match self {
                Shape::Circle(s) => {
                    if duals.len() != 3 {
                        panic!("expected 3 duals for Circle, got {}: {:?}", duals.len(), duals);
                    }
                    let duals = [ duals[0].clone(), duals[1].clone(), duals[2].clone() ];
                    let [ dx, dy, dr ]: [f64; 3] = duals.map(|d| d.iter().zip(step_vec).map(|(mask, step)| mask * step).sum());
                    let c = R2 {
                        x: s.c.x + dx,
                        y: s.c.y + dy,
                    };
                    Shape::Circle(Circle { c, r: s.r + dr })
                },
                Shape::XYRR(e) => {
                    if duals.len() != 4 {
                        panic!("expected 4 duals for XYRR ellipse, got {}: {:?}", duals.len(), duals);
                    }
                    let duals = [
                        duals[0].clone(),
                        duals[1].clone(),
                        duals[2].clone(),
                        duals[3].clone(),
                    ];
                    let [ dcx, dcy, drx, dry ]: [f64; 4] = duals.map(|d| d.iter().zip(step_vec).map(|(mask, step)| mask * step).sum());
                    let c = e.c + R2 { x: dcx, y: dcy, };
                    let r = e.r + R2 { x: drx, y: dry };
                    Shape::XYRR(XYRR { c, r })
                },
                Shape::XYRRT(e) => {
                    if duals.len() != 5 {
                        panic!("expected 5 duals for XYRRT ellipse, got {}: {:?}", duals.len(), duals);
                    }
                    let duals = [
                        duals[0].clone(),
                        duals[1].clone(),
                        duals[2].clone(),
                        duals[3].clone(),
                        duals[4].clone(),
                    ];
                    let [ dcx, dcy, drx, dry, dt ]: [f64; 5] = duals.map(|d| d.iter().zip(step_vec).map(|(mask, step)| mask * step).sum());
                    let c = e.c + R2 { x: dcx, y: dcy, };
                    let r = e.r + R2 { x: drx, y: dry };
                    let t = e.t + dt;
                    Shape::XYRRT(XYRRT { c, r, t })
                },
            },
            duals.clone(),
        )
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
        debug!("  self: {:?}", self);
        debug!("  other: {:?}", o);
        let projection = self.projection();
        let projected = o.apply(&projection);
        let rev = -projection.clone();
        debug!("  projection: {:?}", projection);
        debug!("  projected: {:?}", projected);
        // debug!("reverse projection: {:?}", rev);
        let points = projected.unit_circle_intersections().iter().map(|p| p.apply(&rev)).collect();
        // debug!("points: {:?}", points.clone().collect::<Vec<_>>());
        // debug!();
        points
    }
}