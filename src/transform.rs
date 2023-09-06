use std::ops::{Neg, Div};

use crate::{r2::R2, shape::Shape};

#[derive(Debug, Clone)]
pub enum Transform<D> {
    Translate(R2<D>),
    Scale(R2<D>),
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
            Transform::Scale(v) => Transform::Scale(R2 { x: 1. / v.x, y: 1. / v.y }),
            // Transform::Rotate(a) => Transform::Rotate(-a),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Projection<D>(pub Vec<Transform<D>>);

impl<D: Neg<Output = D>> Neg for Projection<D>
where
R2<D>: Neg<Output = R2<D>>,
f64: Div<D, Output = D>,
{
    type Output = Projection<D>;
    fn neg(self) -> Self {
        Projection(self.0.into_iter().map(|t| -t).collect())
    }
}

pub trait HasProjection<D> {
    fn projection(&self) -> Projection<D>;
}

impl<D: Clone> HasProjection<D> for Shape<D>
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

impl<'a, D: 'a, T: 'a + Clone + CanTransform<'a, D, Output = T>> CanProject<D> for T {
    type Output = T;
    fn apply(self, projection: &Projection<D>) -> Self::Output {
        projection.0.iter().fold(self.clone(), |c, t| c.transform(&t))
    }
}

pub trait CanTransform<'a, D: 'a> {
    type Output;
    fn transform(&'a self, transform: &'a Transform<D>) -> Self::Output;
}