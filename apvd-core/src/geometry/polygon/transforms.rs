use std::ops::Mul;

use crate::{
    dual::Dual,
    r2::R2,
    rotate::RotateArg,
    rotate::Rotate as _Rotate,
    shape::Shape,
    transform::{
        CanTransform,
        Transform::{self, Rotate, Scale, ScaleXY, Translate},
    },
};

use std::ops::Add;

use super::Polygon;

pub trait TransformD: Clone + Mul<Output = Self> + Mul<f64, Output = Self> + RotateArg {}
impl TransformD for f64 {}
impl TransformD for Dual {}

pub trait TransformR2<D>:
    Add<R2<D>, Output = R2<D>> + Mul<R2<D>, Output = R2<D>> + Mul<D, Output = R2<D>>
{
}
impl TransformR2<f64> for R2<f64> {}
impl TransformR2<Dual> for R2<Dual> {}

impl<D: TransformD> CanTransform<D> for Polygon<D>
where
    R2<D>: TransformR2<D>,
{
    type Output = Shape<D>;
    fn transform(&self, transform: &Transform<D>) -> Shape<D> {
        let vertices = match transform {
            Translate(v) => self
                .vertices
                .iter()
                .map(|p| p.clone() + v.clone())
                .collect(),
            Scale(s) => self
                .vertices
                .iter()
                .map(|p| p.clone() * s.clone())
                .collect(),
            ScaleXY(s) => self
                .vertices
                .iter()
                .map(|p| p.clone() * s.clone())
                .collect(),
            Rotate(a) => self
                .vertices
                .iter()
                .map(|p| p.clone().rotate(a))
                .collect(),
        };
        Shape::Polygon(Polygon { vertices })
    }
}
