use std::{fmt::{Debug, Display}, iter::{Sum, repeat}, ops::{Add, AddAssign, Deref, Div, Mul, Neg, Sub, SubAssign}};

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

#[derive(Clone, PartialEq, PartialOrd)]
pub struct Dual(
    pub DualDVec64,
    pub usize
);

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
        const FIELDS: &'static [&'static str] = &["v", "d"];
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
        Dual::new(v, repeat(0.).take(n).collect())
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
        Dual::new(0., repeat(0.).take(n).collect())
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
        Dual(self.0 * rhs.clone(), self.1)
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

impl<'a> Add<&Dual> for &'a Dual {
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
    use super::*;
    use test_log::test;

    #[test]
    fn cos0() {
        let x = Dual::new(0., vec![0.]);
        let cos = x.cos();
        assert_eq!(cos.v(), 1.);
        assert_eq!(cos.d(), vec![0.]);
    }
}