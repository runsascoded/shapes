use std::ops::{Neg, Div};

use crate::r2::R2;

pub enum Transform<D> {
    Translate(R2<D>),
    Scale(R2<D>),
    Rotate(D),
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
            Transform::Rotate(a) => Transform::Rotate(-a),
        }
    }
}

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

