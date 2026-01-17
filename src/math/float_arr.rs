use std::ops::{Div, Mul, Neg, Add, Sub};

use roots::FloatType;

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
    fn test_floatarr() {
        let roots = find_roots_quartic(FloatArr([1.]), FloatArr([4.]), FloatArr([6.]), FloatArr([4.]), FloatArr([1.]));
        println!("{:?}", roots);
    }
}