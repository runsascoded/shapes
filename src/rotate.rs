use std::ops::{Sub, Mul, Add, Neg};

use crate::trig::Trig;

pub trait RotateArg
: Clone
+ Trig
+ Add<Output = Self>
+ Sub<Output = Self>
+ Mul<Output = Self>
+ Neg<Output = Self>
{}

impl<
    D
    : Clone
    + Trig
    + Add<Output = D>
    + Sub<Output = D>
    + Mul<Output = D>
    + Neg<Output = Self>
> RotateArg for D
{}

pub trait Rotate<D> {
    fn rotate(&self, theta: &D) -> Self;
}
