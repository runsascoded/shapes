// use std::ops::{Div, Mul, Neg, Add, Sub};

// use roots::FloatType;

// #[derive(
//     Clone,
//     // Copy,
//     Debug,
//     PartialEq,
//     PartialOrd,
//     // derive_more::Neg,
//     // derive_more::Add,
//     // derive_more::Sub,
//     derive_more::Mul,
//     derive_more::Div,
// )]
// pub struct FloatVec(Vec<f64>);

// impl From<i16> for FloatVec {
//     fn from(i: i16) -> Self {
//         FloatVec([i as f64])
//     }
// }

// impl Neg for FloatVec {
//     type Output = FloatVec;
//     fn neg(self) -> Self::Output {
//         FloatVec([-self.0[0]])
//     }
// }

// impl Add<FloatVec> for FloatVec {
//     type Output = FloatVec;
//     fn add(self, rhs: FloatVec) -> Self::Output {
//         FloatVec([self.0[0] + rhs.0[0]])
//     }
// }

// impl Sub<FloatVec> for FloatVec {
//     type Output = FloatVec;
//     fn sub(self, rhs: FloatVec) -> Self::Output {
//         FloatVec([self.0[0] - rhs.0[0]])
//     }
// }

// impl Mul<FloatVec> for f64 {
//     type Output = f64;
//     fn mul(self, rhs: FloatVec) -> Self::Output {
//         self * rhs.0[0]
//     }
// }

// impl Div<FloatVec> for f64 {
//     type Output = f64;
//     fn div(self, rhs: FloatVec) -> Self::Output {
//         self / rhs.0[0]
//     }
// }

// impl Mul<FloatVec> for [f64; 1] {
//     type Output = [f64; 1];
//     fn mul(self, rhs: FloatVec) -> Self::Output {
//         [self[0] * rhs.0[0]]
//     }
// }

// impl Div<FloatVec> for [f64; 1] {
//     type Output = [f64; 1];
//     fn div(self, rhs: FloatVec) -> Self::Output {
//         [self[0] / rhs.0[0]]
//     }
// }

// impl FloatType for FloatVec {
//     fn zero() -> Self {
//         FloatVec([0.])
//     }
//     fn one() -> Self {
//         FloatVec([1.])
//     }
//     fn one_third() -> Self {
//         FloatVec([1. / 3.])
//     }
//     fn pi() -> Self {
//         FloatVec([std::f64::consts::PI])
//     }
//     fn two_third_pi() -> Self {
//         FloatVec([2. * std::f64::consts::FRAC_PI_3])
//     }
//     fn sqrt(self) -> Self {
//         FloatVec([self.0[0].sqrt()])
//     }
//     fn atan(self) -> Self {
//         FloatVec([self.0[0].atan()])
//     }
//     fn acos(self) -> Self {
//         FloatVec([self.0[0].acos()])
//     }
//     fn sin(self) -> Self {
//         FloatVec([self.0[0].sin()])
//     }
//     fn cos(self) -> Self {
//         FloatVec([self.0[0].cos()])
//     }
//     fn abs(self) -> Self {
//         FloatVec([self.0[0].abs()])
//     }
//     fn powf(self, n: Self) -> Self {
//         FloatVec([self.0[0].powf(n.0[0])])
//     }
// }

// #[cfg(test)]
// mod tests {
//     use roots::find_roots_quartic;

//     use super::*;

//     #[test]
//     fn test_floatvec() {
//         let roots = find_roots_quartic(FloatVec([1.]), FloatVec([4.]), FloatVec([6.]), FloatVec([4.]), FloatVec([1.]));
//         println!("{:?}", roots);
//     }
// }