use std::{f64::consts::TAU, ops::{Div, Mul, Add, Sub, Neg}, fmt};

use crate::{trig::Trig, dual::Dual, zero::Zero};

use super::{complex::{ComplexPair, Complex, self}, quadratic, abs::{Abs, AbsArg}, is_zero::IsZero, cbrt::Cbrt, recip::Recip, deg::Deg};

#[derive(Debug, Clone, PartialEq)]
pub enum DepressedRoots<D> {
    Reals([ D; 3 ]),
    Mixed(D, ComplexPair<D>),
}

impl<D> From<DepressedRoots<D>> for Roots<D> {
    fn from(val: DepressedRoots<D>) -> Self {
        match val {
            DepressedRoots::Reals(rs) => Roots::Reals(rs),
            DepressedRoots::Mixed(re, ims) => Roots::Mixed(re, ims),
        }
    }
}

impl<D: Clone + IsZero + fmt::Debug + Neg<Output = D> + Zero> DepressedRoots<D> {
    pub fn all(&self) -> Vec<Complex<D>> {
        match self {
            DepressedRoots::Reals(rs) => rs.iter().map(|r| Complex::re(r.clone())).collect(),
            DepressedRoots::Mixed(r, im_pair) => {
                // Return the complex pair in sorted order
                let orig = im_pair.clone();
                let conj = im_pair.conj();
                let (neg, pos) = if im_pair.im.lt_zero() {
                    (orig, conj)
                } else {
                    (conj, orig)
                };
                vec![ Complex::re(r.clone()), neg, pos ]
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Roots<D> {
    Quadratic(quadratic::Roots<D>),
    Reals([ D; 3 ]),
    Mixed(D, ComplexPair<D>),
}

use Roots::{Quadratic, Reals, Mixed};
use approx::{AbsDiffEq, RelativeEq};
use log::debug;
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
impl<D: Clone + IsZero + fmt::Debug + Neg<Output = D> + Zero> Roots<D> {
    pub fn all(&self) -> Vec<Complex<D>> {
        match self {
            Quadratic(q) => q.all(),
            Reals(rs) => DepressedRoots::Reals(rs.clone()).all(),
            Mixed(r, im_pair) => DepressedRoots::Mixed(r.clone(), im_pair.clone()).all(),
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
    + Mul<Complex<D>, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
{
    if a.is_zero() {
        Quadratic(quadratic::quadratic(b, c, d))
    } else {
        cubic_normalized(b / a.clone(), c / a.clone(), d / a)
    }
}

pub fn cubic_normalized<D: Arg>(b: D, c: D, d: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Add<Complex<D>, Output = Complex<D>>
    + Sub<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<D>, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
{
    debug!("cubic_normalized: x^3 + {:?} x^2 + {:?} x + {:?}", b, c, d);
    let b3 = b.clone() / 3.;
    let p = c.clone() - b.clone() * b3.clone();
    let q = b3.clone() * b3.clone() * b3.clone() * 2. - b3.clone() * c.clone() + d.clone();
    let rv = if p.is_zero() && q.is_zero() {
        let re = -d.cbrt();
        Reals([ re.clone(), re.clone(), re, ])
    } else {
        match cubic_depressed(p, q) {
            DepressedRoots::Reals(roots) => Reals(roots.map(|r| r - b3.clone())),
            DepressedRoots::Mixed(re, ims) => Mixed(re - b3.clone(), ims - b3),
        }
    };
    debug!("cubic_normalized roots:");
    for x in &rv.all() {
        let x2 = x.clone() * x.clone();
        let y = x2.clone() * x.clone() + x2 * b.clone() + x.clone() * c.clone() + d.clone();
        debug!("  x {:?}, f(x) {:?} ({:?})", x, y, y.norm());
    }
    rv
}

static TAU3: f64 = TAU / 3.;
/// sin(2π/3) = sin(120°) = √3/2
static SIN_TAU3: f64 = 0.8660254037844387; // √3/2
/// Primitive cube root of unity: e^(2πi/3) = -1/2 + i√3/2
static U_1: Complex<f64> = Complex { re: -0.5, im: SIN_TAU3 };

pub fn cubic_depressed<D: Arg>(p: D, q: D) -> DepressedRoots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Add<Complex<D>, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<D>, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
{

    debug!("cubic_depressed: x^3 + {:?}x + {:?}", p, q);
    let rv = if p.is_zero() {
        let re = -q.cbrt();
        let re2 = Complex::re(re.clone());
        let im = re2.clone() * U_1;
        DepressedRoots::Mixed(re, im)
    } else if p.lt_zero() {
        let p3 = p.clone() / 3.;
        let q2 = q.clone() / 2.;
        let p3sq = (-p3.clone()).sqrt();
        let u = q2 / p3 / p3sq.clone();
        let d = u.abs() - 1.;
        if d.le_zero() {
            let r = p3sq.clone() * 2.;
            let θ = u.acos() / 3.;
            // debug!("u {:?}, d {:?}, r {:?}, θ {:?}", u, d, r, θ.deg_str());
            let mut roots = [
                r.clone() *  θ.clone().cos(),
                r.clone() * (θ.clone() + TAU3).cos(),
                r * (θ + TAU3 + TAU3).cos(),
            ];
            roots.sort_by_cached_key(|r| OrderedFloat::<f64>(r.clone().into()));
            // debug!("depressed roots: {:?}", roots);
            DepressedRoots::Reals(roots)
        } else {
            let w = u.clone() + (u.clone() * u.clone() - 1.).sqrt();
            let m = w.cbrt();
            let re = (m.clone() + m.recip()) * p3sq.clone();
            // debug!("u {:?}, w {:?}, m {:?}, re {:?}", u, w, m, re);
            let mu = Complex::re(m) * U_1;
            let im = (mu.clone() + mu.recip()) * p3sq;
            DepressedRoots::Mixed(re, im)
        }
    } else {
        let p3 = p.clone() / 3.;
        let q2 = q.clone() / 2.;
        let p3sq = p3.sqrt();
        let u = -q2 / p3 / p3sq.clone();

        let use_asinh = true;
        let m = if use_asinh {
            // More numerically stable in some cases, using asinh(x) = ln(x + sqrt(x² + 1))
            let a = u.asinh();
            
            // debug!("u {:?}, a {:?}, m {:?}", u, a, m.clone());
            (a.clone() / 3.).exp()
        } else {
            // Naive impl, $w$ can end up as 0. with large negative $u$
            let w = u.clone() + (u.clone() * u.clone() + 1.).sqrt();
            
            // debug!("u {:?}, w {:?}, m {:?}", u, w, m.clone());
            w.cbrt()
        };
        // let w = u.clone() + (u.clone() * u.clone() + 1.).sqrt();
        // if w.is_zero() {
            // u is very large, and negative (e.g. -104023284.33940886); p is so close to 0 that we don't have enough precision to complete this path; treat it like the $p = 0$ / $x³ + q = 0$ code path above.
        //     let re = -q.cbrt();
        //     let re2 = Complex::re(re.clone());
        //     let im = re2.clone() * U_1;
        //     Mixed(re, im)
        // } else {
            // let m = w.cbrt();
            // w.clone().cbrt();
        // };
        // debug!("u {:?}, w {:?}, m {:?}", u, w, m.clone());
        let re = (m.clone() - m.recip()) * p3sq.clone();
        let mu = Complex::re(m) * U_1;
        let im = (mu.clone() - mu.recip()) * p3sq;
        DepressedRoots::Mixed(re, im)
    };
    debug!("cubic_depressed roots:");
    let f = |x: &Complex<D>| x.clone() * x.clone() * x.clone() + x.clone() * p.clone() + q.clone();
    for x in &rv.all() {
        let y = f(x);
        debug!("  x {:?}, f(x) {:?} ({:?})", x, y, y.norm());
    }
    rv
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    fn check(r0: Complex<f64>, r1: Complex<f64>, r2: Complex<f64>, scale: f64) {
        let unscaled_coeffs = [
            Complex::re(1.),
            -(r0 + r1 + r2),
            r0 * r1 + r0 * r2 + r1 * r2,
            -(r0 * r1 * r2),
        ];
        let coeffs = unscaled_coeffs.map(|c| c * scale);
        let [ a3, a2, a1, a0 ] = coeffs.map(|c| {
            assert_abs_diff_eq!(c.im, 0., epsilon = 2e-17);
            c.re
        });
        let roots = cubic::<f64>(a3, a2, a1, a0);
        let ε = 7e-7;
        let actual = crate::math::roots::Roots(roots.all());
        let expected_reals = crate::math::roots::Roots(vec![ r0, r1, r2 ]);
        assert_relative_eq!(actual, expected_reals, max_relative = ε, epsilon = ε);
    }

    #[test]
    fn sweep_reals() {
        // For every nondecreasing triplet of the following values:
        // - synthesize the corresponding cubic polynomial
        // - ask cubic() for the roots
        // - check that the roots are close to the expected values
        //
        // Expected and actual roots are assessed to be "relatively equal" if either:
        // - |a-b| / max(|a|,|b|) <= ε$, or
        // - |a-b| <= ε
        //
        // The algorithm operates on complex numbers, and small imprecisions can manifest in both the real and
        // imaginary parts. Sometimes this means an expected real root will show up as a complex number (with a
        // vanishingly small imaginary part). Comparing values via complex vector lengths and distances is the simplest
        // way to verify that the algorithm is working basically correctly.
        //
        // THe roots::Roots(Vec<Complex<f64>>)  wrapper also implements relative equality checks by "aligning" the
        // "expected" roots against the "actual" roots in such a way that the sum of the element-wise distances is
        // minimized.
        let vals = [ -10., -1., -0.1, 0., 0.1, 1., 10., ];
        let n = vals.len();
        for i0 in 0..n {
            let r0 = Complex::re(vals[i0]);
            for i1 in i0..n {
                let r1 = Complex::re(vals[i1]);
                for i2 in i1..n {
                    let r2 = Complex::re(vals[i2]);
                    let scale = 1.;
                    check(r0, r1, r2, scale);
                }
            }
        }
    }

    #[test]
    fn sweep_mixed() {
        let vals = [ -10., -1., -0.1, 0., 0.1, 1., 10., ];
        let n = vals.len();
        for i0 in 0..n {
            let r0 = Complex::re(vals[i0]);
            for i1 in 0..n {
                let re = vals[i1];
                for i2 in 0..n {
                    let im = vals[i2];
                    let im0 = Complex { re, im };
                    let im1 = im0.conj();
                    let scale = 1.;
                    check(r0, im0, im1, scale);
                }
            }
        }
    }

    #[test]
    fn depressed_ellipses4_0_2_crate() {
        let p = 1.4557437846748906;
        let q = 0.5480639588360245;
        let f = |x: Complex<f64>| x*x*x + x*p + q;
        let e_re = -0.347626610828303;
        let e_im = Complex { re: 0.17381330541415, im: 1.24353406872987 };
        let roots: Roots<f64> = cubic_depressed(p, q).into();

        let expected = Mixed(e_re, e_im);
        debug!("Check expected roots:");
        for x in &expected.all() {
            let y = f(*x);
            debug!("  x {:?}, f(x) {:?} ({:?})", x, y, y.norm());
        }

        assert_relative_eq!(roots, expected, max_relative = 1e-10);
    }

    #[test]
    fn depressed_ellipses4_0_2_roots_crate() {
        let p = 1.4557437846748906;
        let q = 0.5480639588360245;
        let f = |x: Complex<f64>| x*x*x + x*p + q;
        let e_re = -0.347626610828303;
        let e_im = Complex { re: 0.17381330541415, im: -1.24353406872987 };
        // let roots = cubic_depressed(p, q);
        let roots = roots::find_roots_cubic_depressed(p, q).as_ref().to_vec();

        let expected = Mixed(e_re, e_im);
        debug!("Check expected roots:");
        for x in &expected.all() {
            let y = f(*x);
            debug!("  x {:?}, f(x) {:?} ({:?})", x, y, y.norm());
        }
        assert_eq!(roots.len(), 1);
        assert_relative_eq!(roots[0], e_re, max_relative = 1e-10);
    }
}
