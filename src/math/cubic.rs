use std::{f64::consts::TAU, ops::{Div, Mul, Add, Sub, Neg}, fmt};

use crate::{trig::Trig, dual::Dual, zero::Zero};

use super::{complex::{ComplexPair, Complex, self}, quadratic, abs::{Abs, AbsArg}, is_zero::IsZero, cbrt::Cbrt, recip::Recip, deg::Deg};

#[derive(Debug, Clone, PartialEq)]
pub enum Roots<D> {
    Quadratic(quadratic::Roots<D>),
    Reals([ D; 3 ]),
    Mixed(D, ComplexPair<D>),
}

use Roots::{Quadratic, Reals, Mixed};
use approx::{AbsDiffEq, RelativeEq};
use log::{debug, error};
use ordered_float::OrderedFloat;

impl<D: Clone> Roots<D> {
    pub fn reals(&self) -> Vec<D> {
        match self {
            Quadratic(q) => q.reals(),
            Reals(rs) => rs.to_vec(),
            Mixed(re, _) => vec![ re.clone() ],
        }
    }
}
impl<D: Clone + Neg<Output = D> + Zero> Roots<D> {
    pub fn all(&self) -> Vec<Complex<D>> {
        match self {
            Quadratic(q) => q.all(),
            Reals(rs) => rs.iter().map(|r| Complex::re(r.clone())).collect(),
            Mixed(re, im) => vec![ Complex::re(re.clone()), im.clone(), im.conj() ],
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
            (Quadratic(q0), Quadratic(q1)) => q0.abs_diff_eq(q1, epsilon),
            (Reals([ l0, l1, l2 ]), Reals([ r0, r1, r2 ])) => l0.abs_diff_eq(r0, epsilon) && l1.abs_diff_eq(r1, epsilon) && l2.abs_diff_eq(r2, epsilon),
            (Mixed(re0, im0), Mixed(re1, im1)) => re0.abs_diff_eq(re1, epsilon) && im0.abs_diff_eq(im1, epsilon),
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
            (Quadratic(q0), Quadratic(q1)) => q0.relative_eq(q1, epsilon, max_relative),
            (Reals([ l0, l1, l2 ]), Reals([ r0, r1, r2 ])) => l0.relative_eq(r0, epsilon, max_relative) && l1.relative_eq(r1, epsilon, max_relative) && l2.relative_eq(r2, epsilon, max_relative),
            (Mixed(re0, im0), Mixed(re1, im1)) => re0.relative_eq(re1, epsilon, max_relative) && im0.relative_eq(im1, epsilon, max_relative),
            _ => false,
        }
    }
}

pub trait Arg
: fmt::Debug
+ Into<f64>
+ IsZero
+ Cbrt
+ Deg
+ AbsArg
+ Recip
+ Trig
+ complex::Norm
+ quadratic::Arg
+ Add<Output = Self>
+ Add<f64, Output = Self>
+ Sub<Output = Self>
+ Sub<f64, Output = Self>
+ Mul<Output = Self>
+ Mul<f64, Output = Self>
+ Div<Output = Self>
+ Div<f64, Output = Self>
{}

impl Arg for f64 {}
impl Arg for Dual {}

pub fn cubic<D: Arg>(a: D, b: D, c: D, d: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Sub<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
{
    if a.is_zero() {
        Quadratic(quadratic::quadratic(b, c, d))
    } else {
        cubic_scaled(b / a.clone(), c / a.clone(), d / a)
    }
}

pub fn cubic_scaled<D: Arg>(b: D, c: D, d: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Sub<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
{
    debug!("cubic_scaled({:?}, {:?}, {:?})", b, c, d);
    let b3 = b.clone() / 3.;
    let p = c.clone() - b * b3.clone();
    let q = b3.clone() * b3.clone() * b3.clone() * 2. - b3.clone() * c + d.clone();
    if p.is_zero() && q.is_zero() {
        let re = -d.cbrt();
          // TODO: factor / make these static
        let sin_tau3: f64 = TAU3.sin();
        let u_1: Complex<f64> = Complex { re: -0.5, im: sin_tau3 };
        Mixed(re.clone(), Complex::re(re) * u_1)
    } else {
        match cubic_depressed(p, q) {
            Reals(roots) => Reals(roots.map(|r| r - b3.clone())),
            Mixed(re, ims) => Mixed(re - b3.clone(), ims - b3),
            Quadratic(q) => panic!("cubic_depressed returned quadratic::Roots: {:?}", q),
        }
    }
}

static TAU3: f64 = TAU / 3.;

pub fn cubic_depressed<D: Arg>(p: D, q: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Add<Complex<D>, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>,
{
    // TODO: factor / make these static
    let sin_tau3: f64 = TAU3.sin();
    let u_1: Complex<f64> = Complex { re: -0.5, im: sin_tau3 };
    // let u_2: Complex<f64> = Complex { re: -1. / 2., im: -sin_tau3 };

    debug!("p: {:?}, q: {:?}", p, q);
    if p.is_zero() {
        if q.is_zero() {
            error!("Can't infer complex roots from depressed cubic with p == 0 and q == 0");
        }
        let re = -q.cbrt();
        let re2 = Complex::re(re.clone());
        let im = re2.clone() * u_1.clone();
        Mixed(re, im)
    } else if p.lt_zero() {
        let p3 = p / 3.;
        let q2 = q / 2.;
        let p3sq = (-p3.clone()).sqrt();
        let u = q2 / p3 / p3sq.clone();
        let d = u.abs() - 1.;
        if d.le_zero() {
            let r = p3sq.clone() * 2.;
            let θ = u.acos() / 3.;
            debug!("u {:?}, d {:?}, r {:?}, θ {:?}", u, d, r, θ.deg_str());
            let mut roots = [
                r.clone() *  θ.clone().cos(),
                r.clone() * (θ.clone() + TAU3).cos(),
                r * (θ + TAU3 + TAU3).cos(),
            ];
            roots.sort_by_cached_key(|r| OrderedFloat::<f64>(r.clone().into()));
            debug!("depressed roots: {:?}", roots);
            Reals(roots)
        } else {
            let w = u.clone() + (u.clone() * u.clone() - 1.).sqrt();
            let m = w.cbrt();
            let re = (m.clone() + m.recip()) * p3sq.clone();
            debug!("u {:?}, w {:?}, m {:?}, re {:?}", u, w, m, re);
            let ru = Complex::re(m) * u_1;
            let im = (ru.clone() + ru.recip()) * p3sq;
            Mixed(re, im)
        }
    } else {
        let p3 = p / 3.;
        let q2 = q / 2.;
        let p3sq = p3.sqrt();
        let u = q2 / p3 / p3sq.clone();
        let w = u.clone() + (u.clone() * u + 1.).sqrt();
        let m = w.cbrt();
        let re = (m.clone() + m.recip()) * p3sq.clone();
        let ru = Complex::re(m) * u_1;
        let im = (ru.clone() + ru.recip()) * p3sq;
        Mixed(re, im)
    }
}

#[cfg(test)]
mod tests {
    use crate::sqrt::Sqrt;

    use super::*;
    use test_log::test;

    fn check(r0: f64, r1: f64, r2: f64, scale: f64) {
        let unscaled_coeffs = [
            1.,
            -(r0 + r1 + r2),
            r0 * r1 + r0 * r2 + r1 * r2,
            -(r0 * r1 * r2),
        ];
        let coeffs = unscaled_coeffs.map(|c| c * scale);
        let [ a3, a2, a1, a0 ] = coeffs;
        let f = |x: f64| a3 * x * x * x + a2 * x * x + a1 * x + a0;
        let roots = cubic::<f64>(a3, a2, a1, a0);
        let ε = 1e-6;
        if r0 == r1 && r1 == r2 {
            let expected = Mixed( r0, Complex { re: r0 / -2., im: r0 * Sqrt::sqrt(&3.) / 2. });
            assert_relative_eq!(expected, roots, max_relative = ε);
        } else {
            let expected_roots = [ r0, r1, r2 ];
            assert_relative_eq!(Reals(expected_roots), roots, max_relative = ε);
        }
    }

    #[test]
    fn sweep() {
        // let vals = [-10., -1., -0.1, 0., 0.1, 1., 10., ];
        check(-1., -1., -0.1, 1.);
        // let vals = [ -10., -1., -0.1, ];
        // let n = vals.len();
        // for i0 in 0..n {
        //     let r0 = vals[i0];
        //     for i1 in i0..n {
        //         let r1 = vals[i1];
        //         for i2 in i1..n {
        //             let r2 = vals[i2];
        //             let scale = 1.;
        //             check(r0, r1, r2, scale);
        //         }
        //     }
        // }
    }
}
