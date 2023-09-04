use std::{ops::{Div, Mul}, f64::consts::{PI, FRAC_PI_3}, fmt::Display, fmt::Debug};

use derive_more::Deref;
use nalgebra::{ComplexField, Const, SMatrix, U5, U1};
use num_dual::{DualSVec64, Derivative};
use num_traits::FromPrimitive;
use roots::FloatType;

pub type DS5 = DualSVec64<5>;
pub type D5D = Derivative<f64, f64, Const<5>, Const<1>>;

pub fn d5d(d: [f64; 5]) -> D5D {
    D5D::new(Some(SMatrix::from(d)))
}

pub fn d5(v: f64, d: [f64; 5]) -> D5 {
    D5(DS5::new(v, d5d(d)))
}

/// "Dual" number with 5 gradient/derivative dimensions
///
/// This is a test of statically-sized duals, which allows implementing `roots::FloatType` and using the `roots` crate to find roots of polynomials.
/// `D5` wraps `num_traits::DualSVec64`, but the latter doesn't satisfy `roots::FloatType`'s bounds.
#[derive(
    Clone,
    Copy,
    Deref,
    PartialEq,
    PartialOrd,
    derive_more::Neg,
    derive_more::Add,
    derive_more::Sub,
    derive_more::Mul,
    derive_more::Div,
)]
pub struct D5(pub DS5);

impl D5 {
    pub fn v(&self) -> f64 {
        self.0.re
    }
    pub fn d(&self) -> [f64; 5] {
        self.0.eps.unwrap_generic(U5, U1).into()
    }
    pub fn s(&self, n: usize) -> String {
        let f = |v: &f64| format!("{0}{1:.2$}", if v >= &0. { " " } else { "" }, v, n);
        format!("{} [{}]", f(&self.v()), self.d().iter().map(f).collect::<Vec<_>>().join(", "))
    }
}

impl From<i16> for D5 {
    fn from(i: i16) -> Self {
        D5(DS5::from_f64(i as f64).unwrap())
    }
}

impl Mul<D5> for DS5 {
    type Output = DS5;
    fn mul(self, rhs: D5) -> Self::Output {
        self * rhs.0
    }
}

impl Div<D5> for DS5 {
    type Output = DS5;
    fn div(self, rhs: D5) -> Self::Output {
        self / rhs.0
    }
}

impl FloatType for D5 {
    fn zero() -> Self {
        D5((0.).into())
    }
    fn one() -> Self {
        D5((1.).into())
    }
    fn one_third() -> Self {
        D5((1. / 3.).into())
    }
    fn pi() -> Self {
        D5(PI.into())
    }
    fn two_third_pi() -> Self {
        D5((2. * FRAC_PI_3).into())
    }
    fn sqrt(self) -> Self {
        D5(self.0.sqrt())
    }
    fn atan(self) -> Self {
        D5(self.0.atan())
    }
    fn acos(self) -> Self {
        D5(self.0.acos())
    }
    fn sin(self) -> Self {
        D5(self.0.sin())
    }
    fn cos(self) -> Self {
        D5(self.0.cos())
    }
    fn abs(self) -> Self {
        D5(self.0.abs())
    }
    fn powf(self, n: Self) -> Self {
        D5(self.0.powf(n.0))
    }
}

impl Debug for D5 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("D5").field(&self.0).finish()
    }
}

impl Display for D5 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.s(3))
    }
}

#[cfg(test)]
mod tests {
    use roots::find_roots_quartic;

    use super::*;

    fn compute(a4: f64, a3: f64, a2: f64, a1: f64, a0: f64) {
        let s = |f: f64, n: i32| -> String {
            if f == 0. {
                return String::new()
            }
            let coef = if f == 1. && n != 0 {
                String::new()
            } else {
                format!("{}", f.abs())
            };
            let sign_prefix = if n == 4 {
                if f < 0. { "-" } else { "" }
            } else {
                if f < 0. { "- " } else { "+ " }
            };
            let x_term = match n {
                4 => "x⁴",
                3 => "x³",
                2 => "x²",
                1 => "x",
                0 => "",
                _ => panic!("unexpected n: {}", n),
            };
            format!("{}{}{}", sign_prefix, coef, x_term)
        };
        println!("{} {} {} {} {}", s(a4, 4), s(a3, 3), s(a2, 2), s(a1, 1), s(a0, 0));
        let d4 = [ 1., 0., 0., 0., 0., ];
        let d3 = [ 0., 1., 0., 0., 0., ];
        let d2 = [ 0., 0., 1., 0., 0., ];
        let d1 = [ 0., 0., 0., 1., 0., ];
        let d0 = [ 0., 0., 0., 0., 1., ];
        let c4 = d5(a4, d4);
        let c3 = d5(a3, d3);
        let c2 = d5(a2, d2);
        let c1 = d5(a1, d1);
        let c0 = d5(a0, d0);
        let coeffs = [ c0, c1, c2, c3, c4, ];
        let roots = find_roots_quartic(c4, c3, c2, c1, c0);
        let roots_arr = roots.as_ref();
        for root in roots_arr {
            println!("{}", root.s(2));
        }
        println!();

        let ε = 1e-8;
        for idx in (0..5).rev() {
            let n4 = d5(a4 + if idx == 4 { ε } else { 0. }, d4);
            let n3 = d5(a3 + if idx == 3 { ε } else { 0. }, d3);
            let n2 = d5(a2 + if idx == 2 { ε } else { 0. }, d2);
            let n1 = d5(a1 + if idx == 1 { ε } else { 0. }, d1);
            let n0 = d5(a0 + if idx == 0 { ε } else { 0. }, d0);
            let new_coeffs = [ n0, n1, n2, n3, n4, ];
            let new_coef = new_coeffs[idx];
            let old_coef = coeffs[idx];
            let d_coef = new_coef.v() - old_coef.v();
            let new_roots = find_roots_quartic(n4, n3, n2, n1, n0);
            let new_roots_arr = new_roots.as_ref();
            println!("Bumped coeff {}: {} → {}, Δ {} (expected {}):", idx, old_coef.v(), new_coef.v(), d_coef, ε);
            for (jdx, (old, new)) in roots_arr.iter().zip(new_roots_arr).enumerate() {
                let d_root = new.v() - old.v();
                let old_d = old.d()[5 - 1 - idx];
                let new_d = new.d()[5 - 1 - idx];
                println!("  Root {}: {} → {}, Δ {} (expected {}):", jdx, old.v(), new.v(), d_root, old_d * ε);
                println!("       before gradient: {}", old_d);
                println!("        after gradient: {}", new_d);
                println!("       actual gradient: {}", d_root / ε);
                println!("    empirical gradient: {}", d_root / d_coef);
            }
            println!();
        };
    }

    #[test]
    fn test_floatarr() {
        // compute(1., 4., 6., 4., 1.,);
        compute(1., -6., 11., -6., 0.,);
   }
}