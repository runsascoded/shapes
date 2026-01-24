use std::ops::{Div, Mul};

use roots::FloatType;

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

#[cfg(test)]
mod tests {
    use roots::find_roots_quartic;

    use super::*;

    #[test]
    fn test_floatwrap() {
        let roots = find_roots_quartic(FloatWrap(1.), FloatWrap(4.), FloatWrap(6.), FloatWrap(4.), FloatWrap(1.));
        println!("{:?}", roots);
    }
}
