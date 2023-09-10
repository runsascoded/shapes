use std::{fmt::{Display, Formatter, self}, ops::{Mul, Sub, Add, Neg}};

use approx::{AbsDiffEq, RelativeEq};
use derive_more;

use crate::{zero::Zero, sqrt::Sqrt, dual::Dual};


#[derive(
    Clone, Copy, Debug, PartialEq,
    derive_more::Add,
    // derive_more::Sub,
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

impl<D: Clone + Neg<Output = D>> Complex<D> {
    pub fn conj(&self) -> Self {
        Complex {
            re: self.re.clone(),
            im: -self.im,
        }
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
    + Sub<Output = D>
> Sub for Complex<D> {
    type Output = Self;
    fn sub(self, rhs: Complex<D>) -> Self::Output {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

// Prevents the two Mul `impl`s below from conflicting
pub trait Numeric {}
impl Numeric for f64 {}
impl Numeric for Dual {}

impl<
    D
    : Clone
    + Numeric
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
        Self {
            re: a.clone() * c.clone() - b.clone() * d.clone(),
            im: a * d + b * c,
        }
    }
}

pub trait Norm
: Clone
+ Sized
+ Sqrt
+ Add<Output = Self>
+ Mul<Output = Self>
{}
impl Norm for f64 {}
impl Norm for Dual {}

impl<D: Norm> Complex<D> {
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

pub trait Eq
: Norm
+ Zero
+ Sub<Output = Self>
+ RelativeEq<Epsilon = f64>
{}
impl Eq for f64 {}
impl Eq for Dual {}

impl<D: Eq> AbsDiffEq for Complex<D>
where
    Complex<D>: Sub<Output = Complex<D>>
{
    type Epsilon = D::Epsilon;
    fn default_epsilon() -> Self::Epsilon {
        D::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        let norm = (self.clone() - other.clone()).norm();
        norm.abs_diff_eq(&norm.zero(), epsilon)
    }
}

impl<D: Eq> RelativeEq for Complex<D>
{
    fn default_max_relative() -> Self::Epsilon {
        D::default_max_relative()
    }
    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        self.re.relative_eq(&other.re, epsilon, max_relative) && self.im.relative_eq(&other.im, epsilon, max_relative)
    }
}
