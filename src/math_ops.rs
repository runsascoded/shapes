use std::ops::{Div, Mul, Neg, Add, Sub};

use crate::dual::Dual;
// use derive_more::{Neg, Add, Sub, Mul, Div, From};
use nalgebra::{ComplexField, RealField};
use roots::FloatType;

pub trait Trig {
    fn sin(&self) -> Self;
    fn cos(&self) -> Self;
    fn atan(&self) -> Self;
    fn atan2(&self, o: Self) -> Self;
}

impl Trig for Dual {
    fn sin(&self) -> Dual {
        Dual(self.0.clone().sin(), self.1)
    }
    fn cos(&self) -> Dual {
        Dual(self.0.clone().cos(), self.1)
    }
    fn atan(&self) -> Dual {
        Dual(self.0.clone().atan(), self.1)
    }
    fn atan2(&self, o: Dual) -> Dual {
        assert!(self.1 == o.1);
        let x = self.0.clone();
        let y = o.0;
        let z = x.atan2(y);
        Dual(z, self.1)
    }
}

impl Trig for f64 {
    fn sin(&self) -> f64 {
        ComplexField::sin(*self)
    }
    fn cos(&self) -> f64 {
        ComplexField::cos(*self)
    }
    fn atan(&self) -> f64 {
        ComplexField::atan(*self)
    }
    fn atan2(&self, o: f64) -> f64 {
        RealField::atan2(*self, o)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    PartialOrd,
    derive_more::Neg,
    derive_more::Add,
    derive_more::Sub,
    derive_more::Mul,
    derive_more::Div,
)]
pub struct FloatWrap(f64);

impl From<i16> for FloatWrap {
    fn from(i: i16) -> Self {
        FloatWrap(i as f64)
    }
}

impl Mul<FloatWrap> for f64 {
    type Output = f64;
    fn mul(self, rhs: FloatWrap) -> Self::Output {
        self * rhs.0
    }
}

impl Div<FloatWrap> for f64 {
    type Output = f64;
    fn div(self, rhs: FloatWrap) -> Self::Output {
        self / rhs.0
    }
}

impl FloatType for FloatWrap
// where f64: Mul<FloatVec, Output = FloatVec> + Div<FloatVec, Output = FloatVec>
{
    fn zero() -> Self {
        FloatWrap(0.)
    }
    fn one() -> Self {
        FloatWrap(1.)
    }
    fn one_third() -> Self {
        FloatWrap(1. / 3.)
    }
    fn pi() -> Self {
        FloatWrap(std::f64::consts::PI)
    }
    fn two_third_pi() -> Self {
        FloatWrap(2. * std::f64::consts::FRAC_PI_3)
    }
    fn sqrt(self) -> Self {
        FloatWrap(self.0.sqrt())
    }
    fn atan(self) -> Self {
        FloatWrap(self.0.atan())
    }
    fn acos(self) -> Self {
        FloatWrap(self.0.acos())
    }
    fn sin(self) -> Self {
        FloatWrap(self.0.sin())
    }
    fn cos(self) -> Self {
        FloatWrap(self.0.cos())
    }
    fn abs(self) -> Self {
        FloatWrap(self.0.abs())
    }
    fn powf(self, n: Self) -> Self {
        FloatWrap(self.0.powf(n.0))
    }
}


#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    PartialOrd,
    // derive_more::Neg,
    // derive_more::Add,
    // derive_more::Sub,
    derive_more::Mul,
    derive_more::Div,
)]
pub struct FloatArr([f64; 1]);

impl From<i16> for FloatArr {
    fn from(i: i16) -> Self {
        FloatArr([i as f64])
    }
}

impl Neg for FloatArr {
    type Output = FloatArr;
    fn neg(self) -> Self::Output {
        FloatArr([-self.0[0]])
    }
}

impl Add<FloatArr> for FloatArr {
    type Output = FloatArr;
    fn add(self, rhs: FloatArr) -> Self::Output {
        FloatArr([self.0[0] + rhs.0[0]])
    }
}

impl Sub<FloatArr> for FloatArr {
    type Output = FloatArr;
    fn sub(self, rhs: FloatArr) -> Self::Output {
        FloatArr([self.0[0] - rhs.0[0]])
    }
}

impl Mul<FloatArr> for f64 {
    type Output = f64;
    fn mul(self, rhs: FloatArr) -> Self::Output {
        self * rhs.0[0]
    }
}

impl Div<FloatArr> for f64 {
    type Output = f64;
    fn div(self, rhs: FloatArr) -> Self::Output {
        self / rhs.0[0]
    }
}

impl Mul<FloatArr> for [f64; 1] {
    type Output = [f64; 1];
    fn mul(self, rhs: FloatArr) -> Self::Output {
        [self[0] * rhs.0[0]]
    }
}

impl Div<FloatArr> for [f64; 1] {
    type Output = [f64; 1];
    fn div(self, rhs: FloatArr) -> Self::Output {
        [self[0] / rhs.0[0]]
    }
}

impl FloatType for FloatArr {
    fn zero() -> Self {
        FloatArr([0.])
    }
    fn one() -> Self {
        FloatArr([1.])
    }
    fn one_third() -> Self {
        FloatArr([1. / 3.])
    }
    fn pi() -> Self {
        FloatArr([std::f64::consts::PI])
    }
    fn two_third_pi() -> Self {
        FloatArr([2. * std::f64::consts::FRAC_PI_3])
    }
    fn sqrt(self) -> Self {
        FloatArr([self.0[0].sqrt()])
    }
    fn atan(self) -> Self {
        FloatArr([self.0[0].atan()])
    }
    fn acos(self) -> Self {
        FloatArr([self.0[0].acos()])
    }
    fn sin(self) -> Self {
        FloatArr([self.0[0].sin()])
    }
    fn cos(self) -> Self {
        FloatArr([self.0[0].cos()])
    }
    fn abs(self) -> Self {
        FloatArr([self.0[0].abs()])
    }
    fn powf(self, n: Self) -> Self {
        FloatArr([self.0[0].powf(n.0[0])])
    }
}

#[cfg(test)]
mod tests {
    use roots::find_roots_quartic;

    use super::*;

    #[test]
    fn test_floatwrap() {
        let roots = find_roots_quartic(FloatWrap(1.), FloatWrap(4.), FloatWrap(6.), FloatWrap(4.), FloatWrap(1.));
        println!("{:?}", roots);
    }

    #[test]
    fn test_floatarr() {
        let roots = find_roots_quartic(FloatArr([1.]), FloatArr([4.]), FloatArr([6.]), FloatArr([4.]), FloatArr([1.]));
        println!("{:?}", roots);
    }
}