use super::{complex::ComplexPair, is_zero::IsZero};

pub enum Roots<D> {
    Single(D),
    Double(D),
    Reals([ D; 2 ]),
    Complex(ComplexPair<D>),
}

use Roots::{Single, Double, Reals, Complex};

pub fn quadratic<D: IsZero>(a2: D, a1: D, a0: D) -> Roots<D> {
    if a2.is_zero() {
        Single(-a0 / a1)
    } else {
        quadratic_scaled(a1 / a2, a0 / a1)
    }
}

pub fn quadratic_scaled<D: IsZero>(a1: D, a0: D) -> Roots<D> {
    let b2 = a1 / -2.;
    let d = b2 * b2 - a0;
    if d.lt_zero() {
        let d = d.abs();
        let b2 = b2.abs();
        let d = d.sqrt();
        let b2 = b2.sqrt();
        Complex(ComplexPair::new(b2, d.sqrt()))
    } else if d.is_zero() {
        Double(b2)
    } else {
        let d = d.sqrt();
        let b2 = b2.sqrt();
        Reals([ b2 + d, b2 - d ])
    }
}