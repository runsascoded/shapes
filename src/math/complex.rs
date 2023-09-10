use std::{fmt::{Display, Formatter, self}, ops::{self, Mul, Sub, Add}};

use approx::{AbsDiffEq, RelativeEq};
use derive_more;

use crate::{zero::Zero, sqrt::Sqrt};


#[derive(
    Clone, Copy, Debug, PartialEq,
    derive_more::Add,
)]
pub struct Complex<D> {
    pub re: D,
    pub im: D,
}

impl<D: Clone + Zero> Complex<D> {
    pub fn re(re: D) -> Self {
        Self { re: re.clone(), im: re.zero() }
    }
}

impl<
    D
    : Clone
    + Add<Output = D>
> Add<D> for Complex<D> {
    type Output = Self;
    fn add(self, rhs: D) -> Self::Output {
        Self {
            re: self.re + rhs,
            im: self.im,
        }
    }
}

impl<
    D
    : Clone
    + Sub<Output = D>
> Sub<D> for Complex<D> {
    type Output = Self;
    fn sub(self, rhs: D) -> Self::Output {
        Self {
            re: self.re - rhs,
            im: self.im,
        }
    }
}

impl<
    D
    : Clone
    + Mul<Output = D>
> Mul<D> for Complex<D> {
    type Output = Self;
    fn mul(self, rhs: D) -> Self::Output {
        Self {
            re: self.re * rhs.clone(),
            im: self.im * rhs,
        }
    }
}

impl<
    D
    : Clone
    + Display
    + Add<Output = D>
    + Sub<Output = D>
    + Mul<Output = D>
    + Mul<f64, Output = D>
> Mul<Complex<f64>> for Complex<D>
{
    type Output = Self;
    fn mul(self, rhs: Complex<f64>) -> Self::Output {
        let Complex { re: a, im: b } = self.clone();
        let Complex { re: c, im: d } = rhs;
        let rv = Self {
            re: a.clone() * c.clone() - b.clone() * d.clone(),
            im: a * d + b * c,
        };
        println!("{} * {} = {}", self, rhs, rv);
        rv
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

impl<D: AbsDiffEq<Epsilon = f64>> AbsDiffEq for Complex<D> {
    type Epsilon = D::Epsilon;
    fn default_epsilon() -> Self::Epsilon {
        D::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.re.abs_diff_eq(&other.re, epsilon) && self.im.abs_diff_eq(&other.im, epsilon)
    }
}

impl<D: RelativeEq<Epsilon = f64>> RelativeEq for Complex<D>
{
    fn default_max_relative() -> Self::Epsilon {
        D::default_max_relative()
    }
    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        self.re.relative_eq(&other.re, epsilon, max_relative) && self.im.relative_eq(&other.im, epsilon, max_relative)
    }
}

// impl<D: Add<Output = D>> Add for Complex<D> {
//     type Output = Self;
//     fn add(self, rhs: Self) -> Self::Output {
//         Complex {
//             re: self.re + rhs.re,
//             im: self.im + rhs.im,
//         }
//     }
// }