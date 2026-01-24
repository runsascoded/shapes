use std::ops::Neg;

use crate::{zero::Zero, dual::Dual};

pub trait Abs {
    fn abs(&self) -> Self;
}

pub trait AbsArg
    : Clone
    + Neg<Output = Self>
    + PartialOrd
    + Zero
{}

impl AbsArg for f64 {}
impl AbsArg for Dual {}

impl<D: AbsArg> Abs for D {
    fn abs(&self) -> D {
        if *self >= Zero::zero(self) {
            self.clone()
        } else {
            -self.clone()
        }
    }
}
