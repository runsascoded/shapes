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