use std::ops::{Sub, Mul, Add};

use crate::math_ops::Trig;

pub trait RotateArg: Clone + Trig + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> {}
impl<D: Clone + Trig + Add<Output = D> + Sub<Output = D> + Mul<Output = D>> RotateArg for D {}

pub trait Rotate<D> {
    fn rotate(&self, theta: &D) -> Self;
}
