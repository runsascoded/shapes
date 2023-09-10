use std::ops::{Neg, Div, Add, Sub, Mul};

use crate::{sqrt::Sqrt, dual::Dual};

use super::{complex::{self, ComplexPair}, is_zero::IsZero, abs::{Abs, AbsArg}};

#[derive(Debug, Clone, PartialEq)]
pub enum Roots<D> {
    Single(D),
    Double(D),
    Reals([ D; 2 ]),
    Complex(ComplexPair<D>),
}

use Roots::{Single, Double, Reals, Complex};
use approx::{AbsDiffEq, RelativeEq};

impl<D: Clone> Roots<D> {
    pub fn reals(&self) -> Vec<D> {
        match self {
            Single(r) => vec![ r.clone() ],
            Double(r) => vec![ r.clone() ],
            Reals(rs) => rs.clone().to_vec(),
            Complex(_) => vec![],
        }
    }
}

impl<D: complex::Eq> AbsDiffEq for Roots<D> {
    type Epsilon = D::Epsilon;
    fn default_epsilon() -> Self::Epsilon {
        D::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        match (self, other) {
            (Single(r0), Single(r1)) => r0.abs_diff_eq(r1, epsilon),
            (Double(r0), Double(r1)) => r0.abs_diff_eq(r1, epsilon),
            (Reals([ l0, l1 ]), Reals([ r0, r1 ])) => l0.abs_diff_eq(r0, epsilon) && l1.abs_diff_eq(r1, epsilon),
            (Complex(c0), Complex(c1)) => c0.abs_diff_eq(c1, epsilon),
            _ => false,
        }
    }
}

impl<D: complex::Eq> RelativeEq for Roots<D> {
    fn default_max_relative() -> Self::Epsilon {
        D::default_max_relative()
    }
    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        match (self, other) {
            (Single(r0), Single(r1)) => r0.relative_eq(r1, epsilon, max_relative),
            (Double(r0), Double(r1)) => r0.relative_eq(r1, epsilon, max_relative),
            (Reals([ l0, l1 ]), Reals([ r0, r1 ])) => l0.relative_eq(r0, epsilon, max_relative) && l1.relative_eq(r1, epsilon, max_relative),
            (Complex(c0), Complex(c1)) => c0.relative_eq(c1, epsilon, max_relative),
            _ => false,
        }
    }
}

pub trait Arg
: Clone
+ IsZero
+ AbsArg
+ Sqrt
+ Neg<Output = Self>
+ Add<Output = Self>
+ Sub<Output = Self>
+ Mul<Output = Self>
+ Div<Output = Self>
+ Div<f64, Output = Self>
{}

impl Arg for f64 {}
impl Arg for Dual {}

pub fn quadratic<D: Arg>(a2: D, a1: D, a0: D) -> Roots<D> {
    if a2.is_zero() {
        Single(-a0 / a1)
    } else {
        quadratic_scaled(a1 / a2.clone(), a0 / a2)
    }
}

pub fn quadratic_scaled<D: Arg>(a1: D, a0: D) -> Roots<D> {
    let b2 = a1 / -2.;
    let d = b2.clone() * b2.clone() - a0;
    if d.lt_zero() {
        let d = d.abs();
        let b2 = b2.abs();
        let d = d.sqrt();
        let b2 = b2.sqrt();
        Complex(complex::Complex { re: b2, im: d.sqrt() })
    } else if d.is_zero() {
        Double(b2)
    } else {
        let d = d.sqrt();
        // let b2 = b2.sqrt();
        Reals([ b2.clone() + d.clone(), b2 - d ])
    }
}