use std::{fmt::{Debug, Display}, iter::Sum, ops::{Add, AddAssign, Deref, Div, Mul, Neg, Sub, SubAssign}};

use approx::{AbsDiffEq, RelativeEq};
use nalgebra::{ComplexField, Dyn, Matrix, RealField, U1};
use num_dual::{Derivative, DualDVec64};
use num_traits::Zero;
use serde::{Serialize, Serializer};
use serde::ser::{SerializeSeq, SerializeStruct};

pub type D = Dual;

#[derive(Clone, PartialEq, PartialOrd)]
pub struct Dual(
    DualDVec64,
    usize
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

impl Dual {
    pub fn fmt(f: &f64, n: usize) -> String {
        format!("{}{}", if f < &0. {""} else {" "}, format!("{:.1$}", f, n))
    }
    pub fn s(&self, n: usize) -> String {
        format!("{}, vec![{}]", Dual::fmt(&self.v(), n), self.d().iter().map(|d| Dual::fmt(d, n)).collect::<Vec<String>>().join(", "))
    }

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
        write!(f, "{} + [{}]ε", self.v(), self.d().iter().map(|x| format!("{}", x)).collect::<Vec<String>>().join(", "))
    }
}

impl Eq for Dual {

}

impl Ord for Dual {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.v().partial_cmp(&other.v()).unwrap()
    }
}

impl AbsDiffEq for Dual {
    type Epsilon = f64;
    fn default_epsilon() -> Self::Epsilon {
        1e-6
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.v().abs_diff_eq(&other.v(), epsilon) && self.d().abs_diff_eq(&other.d(), epsilon)
    }
}

impl RelativeEq for Dual {
    fn default_max_relative() -> Self::Epsilon {
        1e-3
    }

    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        self.v().relative_eq(&other.v(), epsilon, max_relative) && self.d().relative_eq(&other.d(), epsilon, max_relative)
    }
}

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

impl Mul<f64> for &Dual {
    type Output = Dual;
    fn mul(self, rhs: f64) -> Self::Output {
        Dual(self.0.clone() * rhs, self.1)
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
