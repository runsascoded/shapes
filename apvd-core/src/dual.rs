use std::{fmt::{Debug, Display}, iter::Sum, ops::{Add, AddAssign, Deref, Div, Mul, Neg, Sub, SubAssign}};

use approx::{AbsDiffEq, RelativeEq};
use crate::{fmt::Fmt, to::To};
use nalgebra::{ComplexField, Dyn, Matrix, RealField, U1};
use num_dual::{Derivative, DualDVec64};
use num_traits::Zero;
use serde::{Deserialize, Deserializer, Serialize};
use serde::ser::SerializeStruct;
use tsify::declare;

#[declare]
pub type D = Dual;

#[derive(Clone, PartialEq)]
pub struct Dual(
    pub DualDVec64,
    pub usize
);

impl PartialOrd for Dual {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for Dual {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        let mut ser = serializer.serialize_struct("Dual", 2)?;
        ser.serialize_field("v", &self.v())?;
        ser.serialize_field("d", &self.d())?;
        ser.end()
    }
}

impl<'de> Deserialize<'de> for Dual {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { V, D }
        struct DualVisitor;
        impl<'de> serde::de::Visitor<'de> for DualVisitor {
            type Value = Dual;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Dual")
            }
            fn visit_seq<V: serde::de::SeqAccess<'de>>(self, mut seq: V) -> Result<Self::Value, V::Error> {
                let v = seq.next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let d = seq.next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                Ok(Dual::new(v, d))
            }
            fn visit_map<V: serde::de::MapAccess<'de>>(self, mut map: V) -> Result<Self::Value, V::Error> {
                let mut v = None;
                let mut d = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::V => {
                            if v.is_some() {
                                return Err(serde::de::Error::duplicate_field("v"));
                            }
                            v = Some(map.next_value()?);
                        }
                        Field::D => {
                            if d.is_some() {
                                return Err(serde::de::Error::duplicate_field("d"));
                            }
                            d = Some(map.next_value()?);
                        }
                    }
                }
                let v = v.ok_or_else(|| serde::de::Error::missing_field("v"))?;
                let d = d.ok_or_else(|| serde::de::Error::missing_field("d"))?;
                Ok(Dual::new(v, d))
            }
        }
        const FIELDS: &[&str] = &["v", "d"];
        deserializer.deserialize_struct("Dual", FIELDS, DualVisitor)
    }
}

impl Dual {
    pub fn is_normal(&self) -> bool {
        let v = self.v();
        (v.is_normal() || v.is_zero()) && self.d().iter().all(|d| d.is_normal() || d.is_zero())
    }
    pub fn new(v: f64, d: Vec<f64>) -> Self {
        let n = d.len();
        Dual(
            DualDVec64::new(v, Derivative::some(Matrix::from(d))),
            n
        )
    }
    pub fn scalar(v: f64, n: usize) -> Self {
        Dual::new(v, std::iter::repeat_n(0., n).collect())
    }
    pub fn v(&self) -> f64 {
        self.0.re
    }
    pub fn d(&self) -> Vec<f64> {
        let d = self.0.clone();
        let eps = d.eps;
        let unwrapped = eps.unwrap_generic(Dyn(self.1), U1);
        let sliced = unwrapped.as_slice();
        sliced.to_vec()
    }
    pub fn zero(n: usize) -> Self {
        Dual::new(0., std::iter::repeat_n(0., n).collect())
    }
    pub fn sqrt(&self) -> Self {
        Dual(self.0.clone().sqrt(), self.1)
    }
    pub fn abs(&self) -> Self {
        Dual(self.0.clone().abs(), self.1)
    }
    #[inline]
    pub fn sin(self) -> Self {
        Dual(self.0.clone().sin(), self.1)
    }
    #[inline]
    pub fn cos(self) -> Self {
        Dual(self.0.clone().cos(), self.1)
    }
    #[inline]
    pub fn atan(self) -> Self {
        Dual(self.0.clone().atan(), self.1)
    }
    pub fn atan2(self, o: Self) -> Self {
        assert_eq!(self.1, o.1);
        Dual(self.0.clone().atan2(o.0), self.1)
    }
}

impl Deref for Dual {
    type Target = DualDVec64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Display for Dual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.s(3))
    }
}
impl Debug for Dual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dual::new({:?}, vec!{:?})", self.v(), self.d())
    }
}

impl Eq for Dual {}
impl Ord for Dual {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.v().partial_cmp(&other.v()).unwrap()
    }
}

impl AbsDiffEq for Dual {
    type Epsilon = f64;
    fn default_epsilon() -> Self::Epsilon {
        1e-16
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.v().abs_diff_eq(&other.v(), epsilon) && self.d().abs_diff_eq(&other.d(), epsilon)
    }
}
impl RelativeEq for Dual {
    fn default_max_relative() -> Self::Epsilon {
        1e-16
    }

    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        self.v().relative_eq(&other.v(), epsilon, max_relative) && self.d().relative_eq(&other.d(), epsilon, max_relative)
    }
}

impl To<f64> for Dual {
    fn to(self) -> f64 {
        self.v()
    }
}

impl From<Dual> for f64 {
    fn from(d: Dual) -> Self {
        d.v()
    }
}

impl From<&Dual> for f64 {
    fn from(d: &Dual) -> Self {
        d.v()
    }
}

// impl Into<f64> for Dual {
//     fn into(self) -> f64 {
//         self.v()
//     }
// }

impl Mul for Dual {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0 * rhs.0, self.1)
    }
}

impl Mul<&Dual> for Dual {
    type Output = Self;
    fn mul(self, rhs: &Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0 * &rhs.0, self.1)
    }
}

impl Mul for &Dual {
    type Output = Dual;
    fn mul(self, rhs: Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0.clone() * &rhs.0, self.1)
    }
}

impl Mul<f64> for Dual {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Dual(self.0 * rhs, self.1)
    }
}

impl Mul<&f64> for Dual {
    type Output = Self;
    fn mul(self, rhs: &f64) -> Self::Output {
        Dual(self.0 * *rhs, self.1)
    }
}

impl Mul<f64> for &Dual {
    type Output = Dual;
    fn mul(self, rhs: f64) -> Self::Output {
        Dual(self.0.clone() * rhs, self.1)
    }
}

impl Mul<Dual> for f64 {
    type Output = Dual;
    fn mul(self, rhs: Dual) -> Self::Output {
        Dual(rhs.0 * self, rhs.1)
    }
}

impl Sub for Dual {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0 - rhs.0, self.1)
    }
}

impl Sub<&Dual> for Dual {
    type Output = Self;
    fn sub(self, rhs: &Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0 - &rhs.0, self.1)
    }
}

impl Sub<Dual> for &Dual {
    type Output = Dual;
    fn sub(self, rhs: Dual) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0.clone() - rhs.0, self.1)
    }
}

impl Sub<f64> for Dual {
    type Output = Self;
    fn sub(self, rhs: f64) -> Self::Output {
        Dual(self.0 - rhs, self.1)
    }
}

impl Sub<Dual> for f64 {
    type Output = Dual;
    fn sub(self, rhs: Dual) -> Self::Output {
        Dual(-rhs.0 + self, rhs.1)
    }
}

impl SubAssign for Dual {
    fn sub_assign(&mut self, rhs: Self) {
        assert_eq!(self.1, rhs.1);
        self.0 -= rhs.0;
    }
}

impl Neg for Dual {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Dual(-self.0, self.1)
    }
}

impl Div for Dual {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0 / rhs.0, self.1)
    }
}

impl Div<&Dual> for Dual {
    type Output = Self;
    fn div(self, rhs: &Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0 / &rhs.0, self.1)
    }
}

impl Div<f64> for Dual {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        Dual(self.0 / rhs, self.1)
    }
}

impl Div<Dual> for f64 {
    type Output = Dual;
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Dual) -> Self::Output {
        Dual(rhs.0.recip() * self, rhs.1)
    }
}

impl Add for Dual {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0 + rhs.0, self.1)
    }
}

impl Add<Dual> for f64 {
    type Output = Dual;
    fn add(self, rhs: Dual) -> Self::Output {
        Dual(rhs.0 + self, rhs.1)
    }
}

impl Add<&Dual> for f64 {
    type Output = Dual;
    fn add(self, rhs: &Dual) -> Self::Output {
        Dual(rhs.0.clone() + self, rhs.1)
    }
}

impl Add<f64> for Dual {
    type Output = Self;
    fn add(self, rhs: f64) -> Self::Output {
        Dual(self.0 + rhs, self.1)
    }
}

impl Add<&Dual> for Dual {
    type Output = Self;
    fn add(self, rhs: &Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0 + &rhs.0, self.1)
    }
}

impl Add<&Dual> for &Dual {
    type Output = Dual;
    fn add(self, rhs: &Dual) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0.clone() + rhs.0.clone(), self.1)
    }
}

impl AddAssign for Dual {
    fn add_assign(&mut self, rhs: Self) {
        assert_eq!(self.1, rhs.1);
        self.0 += rhs.0;
    }
}

impl AddAssign<f64> for Dual {
    fn add_assign(&mut self, rhs: f64) {
        self.0 += rhs;
    }
}

impl Sum for Dual {
    fn sum<I: Iterator<Item = Dual>>(mut iter: I) -> Self {
        let first = iter.next().unwrap();
        iter.fold(first, |a, b| a + b)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;
    use approx::assert_relative_eq;
    use super::*;
    use test_log::test;

    #[test]
    fn cos0() {
        let x = Dual::new(0., vec![0.]);
        let cos = x.cos();
        assert_eq!(cos.v(), 1.);
        assert_eq!(cos.d(), vec![0.]);
    }

    #[test]
    fn test_new_and_accessors() {
        let d = Dual::new(3.5, vec![1., 2., 3.]);
        assert_eq!(d.v(), 3.5);
        assert_eq!(d.d(), vec![1., 2., 3.]);
    }

    #[test]
    fn test_scalar() {
        let d = Dual::scalar(5., 3);
        assert_eq!(d.v(), 5.);
        assert_eq!(d.d(), vec![0., 0., 0.]);
    }

    #[test]
    fn test_zero() {
        let d = Dual::zero(4);
        assert_eq!(d.v(), 0.);
        assert_eq!(d.d(), vec![0., 0., 0., 0.]);
    }

    #[test]
    fn test_add() {
        let a = Dual::new(2., vec![1., 0.]);
        let b = Dual::new(3., vec![0., 1.]);
        let c = a + b;
        assert_eq!(c.v(), 5.);
        assert_eq!(c.d(), vec![1., 1.]);
    }

    #[test]
    fn test_add_f64() {
        let a = Dual::new(2., vec![1., 2.]);
        let c = a + 3.;
        assert_eq!(c.v(), 5.);
        assert_eq!(c.d(), vec![1., 2.]);

        let d = Dual::new(2., vec![1., 2.]);
        let e = 3. + d;
        assert_eq!(e.v(), 5.);
        assert_eq!(e.d(), vec![1., 2.]);
    }

    #[test]
    fn test_sub() {
        let a = Dual::new(5., vec![1., 0.]);
        let b = Dual::new(3., vec![0., 1.]);
        let c = a - b;
        assert_eq!(c.v(), 2.);
        assert_eq!(c.d(), vec![1., -1.]);
    }

    #[test]
    fn test_sub_f64() {
        let a = Dual::new(5., vec![1., 2.]);
        let c = a - 3.;
        assert_eq!(c.v(), 2.);
        assert_eq!(c.d(), vec![1., 2.]);

        let d = Dual::new(3., vec![1., 2.]);
        let e = 5. - d;
        assert_eq!(e.v(), 2.);
        assert_eq!(e.d(), vec![-1., -2.]);
    }

    #[test]
    fn test_mul() {
        // d/dx (x * y) = y, d/dy (x * y) = x
        let a = Dual::new(2., vec![1., 0.]);
        let b = Dual::new(3., vec![0., 1.]);
        let c = a * b;
        assert_eq!(c.v(), 6.);
        assert_eq!(c.d(), vec![3., 2.]);
    }

    #[test]
    fn test_mul_f64() {
        let a = Dual::new(2., vec![1., 2.]);
        let c = a * 3.;
        assert_eq!(c.v(), 6.);
        assert_eq!(c.d(), vec![3., 6.]);

        let d = Dual::new(2., vec![1., 2.]);
        let e = 3. * d;
        assert_eq!(e.v(), 6.);
        assert_eq!(e.d(), vec![3., 6.]);
    }

    #[test]
    fn test_div() {
        // d/dx (x / y) = 1/y, d/dy (x / y) = -x/y²
        let a = Dual::new(6., vec![1., 0.]);
        let b = Dual::new(2., vec![0., 1.]);
        let c = a / b;
        assert_eq!(c.v(), 3.);
        assert_eq!(c.d(), vec![0.5, -1.5]);
    }

    #[test]
    fn test_div_f64() {
        let a = Dual::new(6., vec![2., 4.]);
        let c = a / 2.;
        assert_eq!(c.v(), 3.);
        assert_eq!(c.d(), vec![1., 2.]);

        let d = Dual::new(2., vec![1., 0.]);
        let e = 6. / d;
        assert_eq!(e.v(), 3.);
        assert_eq!(e.d(), vec![-1.5, 0.]);
    }

    #[test]
    fn test_neg() {
        let a = Dual::new(3., vec![1., 2.]);
        let b = -a;
        assert_eq!(b.v(), -3.);
        assert_eq!(b.d(), vec![-1., -2.]);
    }

    #[test]
    fn test_sqrt() {
        // d/dx √x = 1/(2√x)
        let a = Dual::new(4., vec![1., 0.]);
        let b = a.sqrt();
        assert_eq!(b.v(), 2.);
        assert_eq!(b.d(), vec![0.25, 0.]);
    }

    #[test]
    fn test_sin_cos() {
        // d/dx sin(x) = cos(x)
        let a = Dual::new(0., vec![1.]);
        let sin_a = a.clone().sin();
        assert_relative_eq!(sin_a.v(), 0.);
        assert_relative_eq!(sin_a.d()[0], 1.); // cos(0) = 1

        let cos_a = a.cos();
        assert_relative_eq!(cos_a.v(), 1.);
        assert_relative_eq!(cos_a.d()[0], 0.); // -sin(0) = 0

        // sin(π/2) = 1, cos(π/2) = 0
        let b = Dual::new(PI / 2., vec![1.]);
        let sin_b = b.clone().sin();
        assert_relative_eq!(sin_b.v(), 1.);
        assert_relative_eq!(sin_b.d()[0], 0., epsilon = 1e-15); // cos(π/2) ≈ 0

        let cos_b = b.cos();
        assert_relative_eq!(cos_b.v(), 0., epsilon = 1e-15);
        assert_relative_eq!(cos_b.d()[0], -1.); // -sin(π/2) = -1
    }

    #[test]
    fn test_atan() {
        // d/dx atan(x) = 1/(1+x²)
        let a = Dual::new(0., vec![1.]);
        let atan_a = a.atan();
        assert_relative_eq!(atan_a.v(), 0.);
        assert_relative_eq!(atan_a.d()[0], 1.); // 1/(1+0) = 1

        let b = Dual::new(1., vec![1.]);
        let atan_b = b.atan();
        assert_relative_eq!(atan_b.v(), PI / 4.);
        assert_relative_eq!(atan_b.d()[0], 0.5); // 1/(1+1) = 0.5
    }

    #[test]
    fn test_atan2() {
        let y = Dual::new(1., vec![1., 0.]);
        let x = Dual::new(1., vec![0., 1.]);
        let theta = y.atan2(x);
        assert_relative_eq!(theta.v(), PI / 4.);
        // d/dy atan2(y,x) = x/(x²+y²), d/dx atan2(y,x) = -y/(x²+y²)
        assert_relative_eq!(theta.d()[0], 0.5); // 1/(1+1) = 0.5
        assert_relative_eq!(theta.d()[1], -0.5); // -1/(1+1) = -0.5
    }

    #[test]
    fn test_abs() {
        let a = Dual::new(-3., vec![1., 2.]);
        let b = a.abs();
        assert_eq!(b.v(), 3.);
        assert_eq!(b.d(), vec![-1., -2.]);

        let c = Dual::new(3., vec![1., 2.]);
        let d = c.abs();
        assert_eq!(d.v(), 3.);
        assert_eq!(d.d(), vec![1., 2.]);
    }

    #[test]
    fn test_is_normal() {
        let a = Dual::new(1., vec![2., 3.]);
        assert!(a.is_normal());

        let b = Dual::new(0., vec![0., 0.]);
        assert!(b.is_normal()); // zero is considered "normal" for this purpose

        let c = Dual::new(f64::NAN, vec![1.]);
        assert!(!c.is_normal());

        let d = Dual::new(1., vec![f64::NAN]);
        assert!(!d.is_normal());

        let e = Dual::new(f64::INFINITY, vec![1.]);
        assert!(!e.is_normal());
    }

    #[test]
    fn test_ordering() {
        let a = Dual::new(1., vec![1.]);
        let b = Dual::new(2., vec![1.]);
        let c = Dual::new(1., vec![2.]); // same value, different derivative
        assert!(a < b);
        assert!(b > a);
        assert!(a <= c);
        assert!(a >= c);
        assert!(a == c); // equality is by value only
    }

    #[test]
    fn test_sum() {
        let duals = vec![
            Dual::new(1., vec![1., 0.]),
            Dual::new(2., vec![0., 1.]),
            Dual::new(3., vec![1., 1.]),
        ];
        let sum: Dual = duals.into_iter().sum();
        assert_eq!(sum.v(), 6.);
        assert_eq!(sum.d(), vec![2., 2.]);
    }

    #[test]
    fn test_display_debug() {
        let d = Dual::new(1.5, vec![2., 3.]);
        let display = format!("{}", d);
        assert!(display.contains("1.5"));

        let debug = format!("{:?}", d);
        assert!(debug.contains("Dual::new"));
        assert!(debug.contains("1.5"));
    }

    #[test]
    fn test_add_assign() {
        let mut a = Dual::new(2., vec![1., 0.]);
        let b = Dual::new(3., vec![0., 1.]);
        a += b;
        assert_eq!(a.v(), 5.);
        assert_eq!(a.d(), vec![1., 1.]);
    }

    #[test]
    fn test_sub_assign() {
        let mut a = Dual::new(5., vec![1., 0.]);
        let b = Dual::new(3., vec![0., 1.]);
        a -= b;
        assert_eq!(a.v(), 2.);
        assert_eq!(a.d(), vec![1., -1.]);
    }

    #[test]
    fn test_from_dual_to_f64() {
        let d = Dual::new(3.5, vec![1., 2.]);
        let f: f64 = d.into();
        assert_eq!(f, 3.5);

        let d2 = Dual::new(4.5, vec![1., 2.]);
        let f2: f64 = (&d2).into();
        assert_eq!(f2, 4.5);
    }
}