use std::{ops::{Deref, Mul, Sub, Neg, Div, AddAssign, SubAssign, Add}, fmt::{Display, Debug}};

use approx::{RelativeEq, AbsDiffEq};
use nalgebra::{Dyn, U1, allocator::Allocator, DefaultAllocator, Matrix, OMatrix, ComplexField};
use num_dual::{DualVec64, DualDVec64, DualNum, Derivative, DualVec};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, PartialOrd)]
pub struct Dual(
    DualDVec64,
    usize
);

impl Dual {
    pub fn fmt(f: &f64) -> String {
        format!("{}{:.3}", if f < &0. {""} else {" "}, f)
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
}

impl Deref for Dual {
    type Target = DualDVec64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Dual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} + [{}]ε", Dual::fmt(&self.v()), self.d().iter().map(Dual::fmt).collect::<Vec<String>>().join(" "))
    }
}

impl Debug for Dual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} + [{}]ε", self.v(), self.d().iter().map(|x| format!("{}", x)).collect::<Vec<String>>().join(", "))
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

    // fn default_max_ulps() -> u32 {
    //     10u32
    // }

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

impl Mul<f64> for Dual {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Dual(self.0 * rhs, self.1)
    }
}

impl Sub for Dual {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        assert_eq!(self.1, rhs.1);
        Dual(self.0 - rhs.0, self.1)
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

impl AddAssign for Dual {
    fn add_assign(&mut self, rhs: Self) {
        assert_eq!(self.1, rhs.1);
        self.0 += rhs.0;
    }
}

// impl Sqrt for Dual {
//     type Output = Self;
//     fn sqrt(self) -> Self::Output {
//         Dual(self.0.sqrt(), self.1)
//     }
// }

// fn dual(re: f64, eps: Vec<f64>) -> DualDVec64 {
//     DualDVec64::new(re, Derivative::new(Some(Matrix::from(eps))))
// }



// impl From<DualDVec64> for Dual
// where
//     DefaultAllocator: Allocator<DualNum<f64>, Dyn>,
// {
//     fn from(dv: DualDVec64) -> Self {
//         let eps = dv.eps
//         Dual {
//             v: dv.re,
//             d: dv.eps.unwrap_generic(Dyn(6), U1).as_slice().to_vec()
//         }
//     }
// }