// use std::ops::Add;

use std::{fmt::{Display, Formatter, self}, ops};

use derive_more::{Add, Sub, Mul, Div, Neg};

use crate::{zero::Zero, sqrt::Sqrt};


#[derive(Clone, Debug, PartialEq, Add, Sub, Mul, Div, Neg)]
pub struct Complex<D> {
    pub re: D,
    pub im: D,
}

impl<D: Clone + Zero> Complex<D> {
    pub fn re(re: D) -> Self {
        Self { re: re.clone(), im: re.zero() }
    }
}

// pub trait Norm
// : Sqrt
// + ops::Add<Output = Self>
// + ops::Mul<Output = Self>
// {}
// impl Norm for f64 {}
// impl Norm for Dual {}

impl<
    D
    : Sqrt
    + Clone
    + ops::Add<Output = D>
    + ops::Mul<Output = D>
> Complex<D> {
    pub fn norm(&self) -> D {
        let re = self.re.clone();
        let im = self.im.clone();
        // let Complex { re, im } = self.clone();
        let re2 = re.clone() * re;
        let im2 = im.clone() * im;
        (re2 + im2).sqrt()
    }
}

impl<D: Display> Display for Complex<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6} + {:.6}i", self.re, self.im)
    }
}

pub type ComplexPair<D> = Complex<D>;

// impl<D: Add<Output = D>> Add for Complex<D> {
//     type Output = Self;
//     fn add(self, rhs: Self) -> Self::Output {
//         Complex {
//             re: self.re + rhs.re,
//             im: self.im + rhs.im,
//         }
//     }
// }