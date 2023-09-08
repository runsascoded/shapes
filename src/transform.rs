use std::{ops::{Neg, Div}, fmt};

use derive_more::Display;

use crate::{r2::R2, shape::Shape};

#[derive(Debug, Display, Clone)]
pub enum Transform<D> {
    Translate(R2<D>),
    Scale(D),
    ScaleXY(R2<D>),
    // Rotate(D),
}

impl<D: Neg<Output = D>> Neg for Transform<D>
where
R2<D>: Neg<Output = R2<D>>,
f64: Div<D, Output = D>,
{
    type Output = Transform<D>;
    fn neg(self) -> Self {
        match self {
            Transform::Translate(v) => Transform::Translate(-v),
            Transform::Scale(v) => Transform::Scale(1. / v),
            Transform::ScaleXY(v) => Transform::ScaleXY(R2 { x: 1. / v.x, y: 1. / v.y }),
            // Transform::Rotate(a) => Transform::Rotate(-a),
        }
    }
}

#[derive(Debug, Display, Clone)]
pub struct Projection<D>(pub Vec<Transform<D>>);

impl<D: Neg<Output = D>> Neg for Projection<D>
where
R2<D>: Neg<Output = R2<D>>,
f64: Div<D, Output = D>,
{
    type Output = Projection<D>;
    fn neg(self) -> Self {
        Projection(self.0.into_iter().rev().map(|t| -t).collect())
    }
}

pub trait HasProjection<D> {
    fn projection(&self) -> Projection<D>;
}

impl<D: Clone + fmt::Display> HasProjection<D> for Shape<D>
where
    R2<D>: Neg<Output = R2<D>>,
    f64: Div<D, Output = D>,
{
    fn projection(&self) -> Projection<D> {
        match self {
            Shape::Circle(c) => c.projection(),
            Shape::XYRR(e) => e.projection(),
        }
    }
}

pub trait CanProject<D> {
    type Output;
    fn apply(&self, projection: &Projection<D>) -> Self::Output;
}

impl<D, T: Clone + CanTransform<D, Output = T>> CanProject<D> for T {
    type Output = T;
    fn apply(&self, projection: &Projection<D>) -> Self::Output {
        projection.0.iter().fold(self.clone(), |c, t| c.transform(&t))
    }
}

pub trait CanTransform<D> {
    type Output;
    fn transform(&self, transform: &Transform<D>) -> Self::Output;
}
