use std::{ops::{Neg, Div, Add, Sub, Mul}, fmt};

use crate::{sqrt::Sqrt, dual::Dual, zero::Zero};

use super::{complex::{self, Complex as C, ComplexPair}, is_zero::IsZero, abs::AbsArg};

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

impl<D: Clone + fmt::Debug + Neg<Output = D> + Zero> Roots<D> {
    pub fn all(&self) -> Vec<C<D>> {
        match self {
            Single(r) => vec![ C::re(r.clone()) ],
            Double(r) => vec![ C::re(r.clone()) ],
            Reals(rs) => rs.iter().map(|r| C::re(r.clone())).collect(),
            Complex(c) => vec![ c.clone(), c.conj() ],
        }
    }

    pub fn two_roots(&self) -> [ C<D>; 2 ] {
        match self {
            Single(r) => panic!("single root: {:?}", r),
            Double(r) => [ C::re(r.clone()), C::re(r.clone()) ],
            Reals(rs) => [ C::re(rs[0].clone()), C::re(rs[1].clone()) ],
            Complex(c) => [ c.clone(), c.conj() ],
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
+ fmt::Debug
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
    // debug!("quadratic: {:?}x^2 + {:?}x + {:?}", a2, a1, a0);
    if a2.is_zero() {
        Single(-a0 / a1)
    } else {
        quadratic_scaled(a1 / a2.clone(), a0 / a2)
    }
}

pub fn quadratic_scaled<D: Arg>(a1: D, a0: D) -> Roots<D> {
    // debug!("quadratic_scaled: x^2 + {:?}x + {:?}", a1, a0);
    let b2 = a1 / -2.;
    let d = b2.clone() * b2.clone() - a0;
    if d.lt_zero() {
        Complex(complex::Complex { re: b2, im: (-d).sqrt() })
    } else if d.is_zero() {
        Double(b2)
    } else {
        let d = d.sqrt();
        // let b2 = b2.sqrt();
        Reals([ b2.clone() + d.clone(), b2 - d ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify roots satisfy the quadratic equation ax² + bx + c = 0
    fn verify_roots(a: f64, b: f64, c: f64, roots: &Roots<f64>, epsilon: f64) {
        let f = |x: f64| a * x * x + b * x + c;
        for root in roots.reals() {
            let val = f(root);
            assert!(
                val.abs() < epsilon,
                "Root {} doesn't satisfy equation (f(x) = {})",
                root, val
            );
        }
        if let Complex(pair) = roots {
            let fc = |x: C<f64>| x.clone() * x.clone() * a + x.clone() * b + c;
            let val = fc(pair.clone());
            assert!(
                val.norm() < epsilon,
                "Complex root {} doesn't satisfy equation (f(x) = {})",
                pair, val.norm()
            );
        }
    }

    #[test]
    fn two_distinct_real_roots() {
        // x² - 5x + 6 = 0 → (x-2)(x-3) = 0 → roots: 2, 3
        let roots = quadratic(1., -5., 6.);
        assert!(matches!(roots, Reals(_)));
        if let Reals([r0, r1]) = roots {
            assert!((r0 - 3.).abs() < 1e-10 || (r0 - 2.).abs() < 1e-10);
            assert!((r1 - 3.).abs() < 1e-10 || (r1 - 2.).abs() < 1e-10);
            assert!((r0 - r1).abs() > 0.5); // distinct
        }
        verify_roots(1., -5., 6., &roots, 1e-10);
    }

    #[test]
    fn repeated_root() {
        // x² - 4x + 4 = 0 → (x-2)² = 0 → root: 2 (double)
        let roots = quadratic(1., -4., 4.);
        assert!(matches!(roots, Double(_)));
        if let Double(r) = roots {
            assert!((r - 2.).abs() < 1e-10);
        }
        verify_roots(1., -4., 4., &roots, 1e-10);
    }

    #[test]
    fn complex_roots() {
        // x² + 1 = 0 → roots: ±i
        let roots = quadratic(1., 0., 1.);
        assert!(matches!(roots, Complex(_)));
        if let Complex(c) = &roots {
            assert!(c.re.abs() < 1e-10);
            assert!((c.im.abs() - 1.).abs() < 1e-10);
        }
        verify_roots(1., 0., 1., &roots, 1e-10);
    }

    #[test]
    fn complex_roots_general() {
        // x² - 2x + 5 = 0 → roots: 1 ± 2i
        let roots = quadratic(1., -2., 5.);
        assert!(matches!(roots, Complex(_)));
        if let Complex(c) = &roots {
            assert!((c.re - 1.).abs() < 1e-10);
            assert!((c.im.abs() - 2.).abs() < 1e-10);
        }
        verify_roots(1., -2., 5., &roots, 1e-10);
    }

    #[test]
    fn linear_equation() {
        // 0x² + 2x + 4 = 0 → x = -2
        let roots = quadratic(0., 2., 4.);
        assert!(matches!(roots, Single(_)));
        if let Single(r) = roots {
            assert!((r - (-2.)).abs() < 1e-10);
        }
    }

    #[test]
    fn scaled_coefficients() {
        // 2x² - 10x + 12 = 0 → same as x² - 5x + 6 = 0 → roots: 2, 3
        let roots = quadratic(2., -10., 12.);
        assert!(matches!(roots, Reals(_)));
        verify_roots(2., -10., 12., &roots, 1e-10);
    }

    #[test]
    fn roots_at_zero() {
        // x² - x = 0 → x(x-1) = 0 → roots: 0, 1
        let roots = quadratic(1., -1., 0.);
        assert!(matches!(roots, Reals(_)));
        if let Reals([r0, r1]) = roots {
            assert!((r0 * r1).abs() < 1e-10); // one root is 0
            assert!((r0 + r1 - 1.).abs() < 1e-10); // sum is 1
        }
        verify_roots(1., -1., 0., &roots, 1e-10);
    }

    #[test]
    fn negative_roots() {
        // x² + 5x + 6 = 0 → (x+2)(x+3) = 0 → roots: -2, -3
        let roots = quadratic(1., 5., 6.);
        assert!(matches!(roots, Reals(_)));
        if let Reals([r0, r1]) = roots {
            assert!((r0 + 2.).abs() < 1e-10 || (r0 + 3.).abs() < 1e-10);
            assert!((r1 + 2.).abs() < 1e-10 || (r1 + 3.).abs() < 1e-10);
        }
        verify_roots(1., 5., 6., &roots, 1e-10);
    }
}