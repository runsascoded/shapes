use std::{f64::consts::TAU, ops::{Div, Mul, Add, Sub, Neg}, fmt};

use crate::{trig::Trig, dual::Dual, sqrt::Sqrt, zero::Zero, math::cubic::cubic_depressed};

use super::{complex::{ComplexPair, Complex, self, SqrtArg, Numeric}, quadratic, abs::{Abs, AbsArg}, is_zero::IsZero, cbrt::Cbrt, recip::Recip, deg::Deg};

use super::cubic;

#[derive(Debug, Clone)]
pub enum Roots<D> {
    Cubic(cubic::Roots<D>),
    Reals([ D; 4 ]),
    Mixed(D, D, ComplexPair<D>),
    Imags(ComplexPair<D>, ComplexPair<D>),
}

use Roots::*;
use log::debug;
use ordered_float::OrderedFloat;

impl<D: Clone + Into<f64> + IsZero + PartialEq + Neg<Output = D>> Roots<D> {
    pub fn new(roots: [ Complex<D>; 4 ]) -> Self {
        let mut reals: Vec<D> = Vec::new();
        let mut imags: Vec<Complex<D>> = Vec::new();
        for root in &roots {
            if root.im.is_zero() {
                reals.push(root.re.clone());
            } else {
                imags.push(root.clone());
            }
        }
        if imags.len() == 0 {
            Reals(roots.map(|r| r.re))
        } else if reals.len() == 0 {
            let c00 = imags[0].clone();
            let c01 = c00.clone().conj();
            let c0 = if c00.im.clone().into() > 0. { c00.clone() } else { c01.clone() };
            match imags.into_iter().skip(1).filter(|r| (*r) != c00 && (*r) != c01 && r.clone().im.into() > 0.).next() {
                Some(c1) => Imags(c0, c1),
                None => Imags(c0.clone(), c0),
            }
        } else {
            Mixed(reals[0].clone(), reals[1].clone(), imags[0].clone())
        }
    }
    pub fn reals(&self) -> Vec<D> {
        match self {
            Cubic(roots) => roots.reals(),
            Reals(rs) => rs.clone().to_vec(),
            Mixed(r0, r1, _) => vec![ r0.clone(), r1.clone() ],
            Imags(_, _) => vec![],
        }
    }
}

impl<D: Clone + IsZero + fmt::Debug + Zero + Neg<Output = D> + Zero> Roots<D> {
    pub fn all(&self) -> Vec<Complex<D>> {
        match self {
            Roots::Cubic(roots) => roots.all(),
            Roots::Reals(roots) => roots.iter().map(|r| Complex::re(r.clone())).collect(),
            Roots::Mixed(r0, r1, c) => vec![
                Complex::re(r0.clone()),
                Complex::re(r1.clone()),
                c.clone(),
                c.conj(),
            ],
            Roots::Imags(c0, c1) => vec![
                c0.clone(),
                c0.conj(),
                c1.clone(),
                c1.conj(),
            ],
        }
    }
}


pub trait Arg
: cubic::Arg
+ PartialEq
+ Neg<Output = Self>
+ complex::SqrtArg
+ Numeric
{}

impl Arg for f64 {}
impl Arg for Dual {}

pub fn quartic<D: Arg>(a: D, b: D, c: D, d: D, e: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Sub<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<D>, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
    + Mul<f64, Output = Complex<D>>
    + Div<f64, Output = Complex<D>>
    + Neg<Output = Complex<D>>
{
    if a.is_zero() {
        Cubic(cubic::cubic(b, c, d, e))
    } else {
        quartic_scaled(b / a.clone(), c / a.clone(), d / a.clone(), e / a)
    }
}

pub fn quartic_scaled<D: Arg>(b: D, c: D, d: D, e: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Add<Complex<D>, Output = Complex<D>>
    + Sub<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<D>, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
    + Mul<f64, Output = Complex<D>>
    + Div<f64, Output = Complex<D>>
    + Neg<Output = Complex<D>>
{
    debug!("quartic_scaled({:?}, {:?}, {:?}, {:?})", b, c, d, e);
    debug!("x^4 + {:?} x^3 + {:?} x^2 + {:?} x + {:?}", b, c, d, e);
    let b4 = b.clone() / 4.;
    let b4sq = b4.clone() * b4.clone();
    let c2 = c.clone() - b4sq.clone() * 6.;
    let d2 = b4sq.clone() * b4.clone() * 8. - b4.clone() * c.clone() * 2. + d.clone();
    let e2 = b4sq.clone() * b4sq.clone() * -3. + b4sq * c.clone() - b4.clone() * d.clone() + e.clone();
    let rv = match quartic_depressed(c2, d2, e2) {
        Reals([ r0, r1, r2, r3 ]) => {
            let r0 = r0 - b4.clone();
            let r1 = r1 - b4.clone();
            let r2 = r2 - b4.clone();
            let r3 = r3 - b4;
            Reals([ r0, r1, r2, r3 ])
        },
        Mixed(r0, r1, c) => {
            let r0 = r0 - b4.clone();
            let r1 = r1 - b4.clone();
            let c = c - b4;
            Mixed(r0, r1, c)
        },
        Imags(c0, c1) => {
            let c0 = c0 - b4.clone();
            let c1 = c1 - b4;
            Imags(c0, c1)
        },
        Cubic(c) => panic!("quartic_depressed returned cubic::Roots: {:?}", c)
    };
    debug!("quartic_scaled roots:");
    for x in &rv.all() {
        let x2 = x.clone() * x.clone();
        let y = x2.clone() * x2.clone() + x2.clone() * x.clone() * b.clone() + x2 * c.clone() + x.clone() * d.clone() + e.clone();
        debug!("  x: {:?}, y: {:?}, r: {:?}", x, y, y.norm());
    }
    rv
}

pub fn quartic_depressed<D: Arg>(c: D, d: D, e: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Add<Complex<D>, Output = Complex<D>>
    + Sub<Complex<D>, Output = Complex<D>>
    + Sub<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<D>, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
    + Mul<f64, Output = Complex<D>>
    + Div<f64, Output = Complex<D>>
    + Neg<Output = Complex<D>>
{
    debug!("quartic_depressed({:?}, {:?}, {:?})", c, d, e);
    debug!("x^4 + {:?} x^2 + {:?} x + {:?}", c, d, e);
    let rv = if e.is_zero() {
        match cubic_depressed(c.clone(), d.clone()) {
            cubic::DepressedRoots::Reals([ r0, r1, r2 ]) => {
                let r0 = r0.clone();
                let r1 = r1.clone();
                let r2 = r2.clone();
                let mut roots = [ r0.clone(), r1, r2, r0.zero() ];
                roots.sort_by_key(|r| {
                    OrderedFloat(r.clone().into())
                });
                Reals(roots)
            },
            cubic::DepressedRoots::Mixed(r0, c) => {
                let mut reals = [ r0.clone(), r0.zero() ];
                reals.sort_by_key(|r| OrderedFloat(r.clone().into()));
                let [ r0, r1 ] = reals;
                Mixed(r0, r1, c)
            },
        }
    } else {
        let d64: f64 = d.clone().into();
        let c64: f64 = c.clone().into().abs();
        let roots = if d64.abs() / f64::max(1., c64) < 1e-14 {
            // Roots: -0.1, -0.1, -0.1 ± -0.1i:
            //   x⁴ - 0.4x³ + 0.07x² + 0.006x + 0.0002
            // f64 math turns this into:
            //   x⁴ + 0.4x³ + 0.07x² + 0.006000000000000002x + 0.00020000000000000006
            // which depresses to:
            //   x⁴ + 0.009999999999999995x² + 1.734723475976807e-18x + -1.3552527156068805e-19
            //
            // Factor of ≈1.7e-16 multiple between "c" and "d" is about at the edge of f64 precision, and ratio between
            // "a" and "d" is beyond it, so we treat "d" as 0, otherwise we can end up with some junk math later,
            // similar to dividing by zero, but instead of NaNs we just a cubic root of 0, and other assumptions about
            // completing the two polynomial squares break down. I believe taking one of the other cubic roots would
            // work, but this seems like an easier work-around for now.
            //
            // Bound pushed above 9e-15 by: x^4 + 40.0 x^3 + 700.01 x^2 + 6000.2 x + 20002.0
            // Depressed: x^4 + 100.00999999999999 x^2 + -9.094947017729282e-13 x + 1.0
            // Synthesized from roots:
            //   Complex { re: -10.0, im: -10.0 },
            //   Complex { re: -10.0, im:  10.0 },
            //   Complex { re: -10.0, im:  -0.1 },
            //   Complex { re: -10.0, im:   0.1 },
            quartic_biquadratic(c.clone(), e.clone())
        } else {
            let a_2 = c.clone() * 2.;
            let a_1 = c.clone() * c.clone() - e.clone() * 4.;
            let a_0 = -d.clone() * d.clone();
            let cubic_roots = cubic::cubic(a_2.zero() + 1., a_2, a_1, a_0);
            debug!("cubic_roots: {:?}", cubic_roots);
            let cubic_reals = cubic_roots.reals();
            let u = cubic_reals.iter().rev().next().unwrap();
            let usq2 = if u.lt_zero() {
                Complex { re: u.zero(), im: u.zero() + (-u.clone()).sqrt() } / 2.
            } else {
                Complex { re: u.zero() + u.sqrt(), im: u.zero() } / 2.
            };
            let d_usq2r = usq2.recip() * d.clone();
            let uc2 = Complex::re(-u.clone()) - c.clone() * 2.;
            let d0 = uc2.clone() - d_usq2r.clone();
            let d1 = uc2.clone() + d_usq2r.clone();
            let d0sq2 = Sqrt::sqrt(&d0) / 2.;
            let d1sq2 = Sqrt::sqrt(&d1) / 2.;
            debug!("u {:?}", u);
            debug!("usq2 {:?}", usq2);
            debug!("d_usq2r {:?}", d_usq2r);
            debug!("uc2 {:?}", uc2);
            debug!("d0 {:?}", d0);
            debug!("d1 {:?}", d1);
            debug!("d0sq2 {:?}", d0sq2);
            debug!("d1sq2 {:?}", d1sq2);
            let roots = [
                 usq2.clone() + d0sq2.clone(),
                 usq2.clone() - d0sq2.clone(),
                -usq2.clone() + d1sq2.clone(),
                -usq2.clone() - d1sq2.clone(),
            ];
            roots
        };
        Roots::new(roots)
    };
    debug!("quartic_depressed roots:");
    for x in &rv.all() {
        let x2 = x.clone() * x.clone();
        let y = x2.clone() * x2.clone() + x2 * c.clone() + x.clone() * d.clone() + e.clone();
        debug!("  x: {:?}, y: {:?}, r: {:?}", x, y, y.norm());
    }
    rv
}

pub fn quartic_biquadratic<
    D
    : quadratic::Arg
    + fmt::Debug
    + Add<f64, Output = D>
    + Numeric
    + SqrtArg
>(c: D, e: D) -> [ Complex<D>; 4 ]
where
    Complex<D>
    : Neg<Output = Complex<D>>
    + Add<Complex<D>, Output = Complex<D>>
    + Add<D, Output = Complex<D>>
    + Mul<Complex<D>, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
{
    let roots = quadratic::quadratic(c.zero() + 1., c.clone(), e.clone()).two_roots();
    debug!("quadratic roots:");
    for x in &roots {
        let y = x.clone() * x.clone() + x.clone() * c.clone() + e.clone();
        debug!("  x {:?}, y {:?}, r {:?}", x, y, y.norm());
    }
    let [ sq0, sq1 ] = roots.map(|r| r.sqrt());
    let roots = [
         sq0.clone(),
         sq1.clone(),
         -sq0.clone(),
         -sq1.clone(),
    ];
    debug!("quartic_biquadratic roots:");
    for x in &roots {
        let x2 = x.clone() * x.clone();
        let y = x2.clone() * x2.clone() + x2.clone() * c.clone() + e.clone();
        debug!("  x {:?}, y {:?}, r {:?}", x, y, y.norm());
    }
    roots
}

#[cfg(test)]
mod tests {

    use super::*;

    use test_log::test;

    fn coeffs_from_roots(roots: Roots<f64>) -> [ f64; 4 ] {
        match roots {
            Reals([ r0, r1, r2, r3 ]) => {
                let a3 = -(r0 + r1 + r2 + r3);
                let a2 = r0 * r1 + r0 * r2 + r0 * r3 + r1 * r2 + r1 * r3 + r2 * r3;
                let a1 = -(r0 * r1 * r2 + r0 * r1 * r3 + r0 * r2 * r3 + r1 * r2 * r3);
                let a0 = r0 * r1 * r2 * r3;
                [ a3, a2, a1, a0 ]
            },
            Mixed(r0, r1, c) => {
                let re = c.re;
                let n2 = c.norm2();
                let a3 = -(r0 + r1 + re + re);
                let a2 = r0 * r1 + 2. * re*(r0 + r1) + n2;
                let a1 = -(r0 * r1 * re * 2. + (r0 + r1) * n2);
                let a0 = r0 * r1 * n2;
                [ a3, a2, a1, a0 ]
            },
            Imags(c0, c1) => {
                let re0 = c0.re;
                let re1 = c1.re;
                let n20 = c0.norm2();
                let n21 = c1.norm2();
                let a3 = -2. * (re0 + re1);
                let a2 = 4. * re0 * re1 + n20 + n21;
                let a1 = -2. * (n20 * re1 + n21 * re0);
                let a0 = n20 * n21;
                [ a3, a2, a1, a0 ]
            }
            Cubic(_) => panic!("Unexpected cubic roots: {:?}", roots),
        }
    }

    fn check(r0: Complex<f64>, r1: Complex<f64>, r2: Complex<f64>, r3: Complex<f64>, scale: f64) {
        let roots = Roots::new([ r0, r1, r2, r3 ]);
        let a4 = scale;
        let [ a3, a2, a1, a0 ] = coeffs_from_roots(roots.clone()).map(|c| c * scale);
        let [ e3, e2, e1, e0 ] = [
            -(r0 + r1 + r2 + r3),
            r0 * r1 + r0 * r2 + r0 * r3 + r1 * r2 + r1 * r3 + r2 * r3,
            -(r0 * r1 * r2 + r0 * r1 * r3 + r0 * r2 * r3 + r1 * r2 * r3),
            r0 * r1 * r2 * r3,
        ].map(|c| c.re * scale);
        // debug!("Expected coeffs:");
        // for e in &[ e3, e2, e1, e0 ] {
        //     debug!("  {:?}", e);
        // }
        // debug!("Actual coeffs:");
        // for a in &[ a3, a2, a1, a0 ] {
        //     debug!("  {:?}", a);
        // }
        // debug!("roots:");
        // debug!("  {:?}", r0);
        // debug!("  {:?}", r1);
        // debug!("  {:?}", r2);
        // debug!("  {:?}", r3);
        assert_relative_eq!(a3, e3, max_relative = 1e-5, epsilon = 1e-14);
        assert_relative_eq!(a2, e2, max_relative = 1e-5, epsilon = 1e-14);
        assert_relative_eq!(a1, e1, max_relative = 1e-5, epsilon = 1e-14);
        assert_relative_eq!(a0, e0, max_relative = 1e-5, epsilon = 1e-14);
        let roots = quartic(a4, a3, a2, a1, a0);
        let ε = 2e-5;
        let actual = crate::math::roots::Roots(roots.all());
        let expected = crate::math::roots::Roots([ r0, r1, r2, r3 ].to_vec());
        assert_relative_eq!(actual, expected, max_relative = ε, epsilon = ε);
    }

    #[test]
    fn sweep_reals() {
        let vals = [ -10., -1., -0.1, 0., 0.1, 1., 10., ];
        let n = vals.len();
        for i0 in 0..n {
            let r0 = Complex::re(vals[i0]);
            for i1 in i0..n {
                let r1 = Complex::re(vals[i1]);
                for i2 in i1..n {
                    let r2 = Complex::re(vals[i2]);
                    for i3 in i2..n {
                        let r3 = Complex::re(vals[i3]);
                        let scale = 1.;
                        check(r0, r1, r2, r3, scale);
                    }
                }
            }
        }
    }

    #[test]
    fn sweep_mixed() {
        // This depressed quartic ends up with c≈0.01, and "d" and "e" below 2e-17; fuzzy zero-cmp on "d" treats it as biquadratic
        // check(Complex::re(-0.1), Complex::re(-0.1), Complex { re: -0.1, im: -0.1, }, Complex { re: -0.1, im:  0.1, }, 1.,);
        // This one is similar, but motivates comparing "d" (≈1e-15) with "c" (≈100). Increasing the imaginary component of the latter pair below can make `d`'s value be larger in absolute terms, but the math relative to "c" is more important.
        // check(Complex::re(-0.1), Complex::re(-0.1), Complex { re: -0.1, im: -10., }, Complex { re: -0.1, im: 10., }, 1.,);
        let vals = [ -10., -1., -0.1, 0., 0.1, 1., 10., ];
        let n = vals.len();
        for i0 in 0..n {
            let r0 = Complex::re(vals[i0]);
            for i1 in i0..n {
                let r1 = Complex::re(vals[i1]);
                for i2 in i1..n {
                    let re = vals[i2];
                    for i3 in i2..n {
                        let im = vals[i3];
                        let im0 = Complex { re, im };
                        let im1 = im0.conj();
                        let scale = 1.;
                        check(r0, r1, im0, im1, scale);
                    }
                }
            }
        }
    }

    #[test]
    fn sweep_imags() {
        // check(
        //     Complex { re: -10.0, im: -10.0 },
        //     Complex { re: -10.0, im:  10.0 },
        //     Complex { re: -10.0, im:  -0.1 },
        //     Complex { re: -10.0, im:   0.1 },
        //     1.
        // );
        let vals = [ -10., -1., -0.1, 0., 0.1, 1., 10., ];
        let n = vals.len();
        for i0 in 0..n {
            let re0 = vals[i0];
            for i1 in i0..n {
                let im0 = vals[i1];
                let c00 = Complex { re: re0, im: im0 };
                let c01 = c00.conj();
                for i2 in i1..n {
                    let re1 = vals[i2];
                    for i3 in i2..n {
                        let im1 = vals[i3];
                        let c10 = Complex { re: re1, im: im1 };
                        let c11 = c10.conj();
                        let scale = 1.;
                        check(c00, c01, c10, c11, scale);
                    }
                }
            }
        }
    }

    // Factored out of unit-intersection calculation for the ellipse:
    //
    // XYRR {
    //     c: R2 { x: -1.100285308561806, y: -1.1500279763995946e-5 },
    //     r: R2 { x:  1.000263820108834, y:  1.0000709021402923 }
    // }
    //
    // which is nearly a unit circle centered at (-1.1, 0), but with all 4 coordinates perturbed slightly.
    // See also: https://github.com/vorot/roots/issues/30, intersections::tests::test_perturbed_unit_circle.
    static A4: f64 = 0.000000030743755847066437;
    static A3: f64 = 0.000000003666731306801131;
    static A2: f64 = 1.0001928389119579;
    static A1: f64 = 0.000011499702220469921;
    static A0: f64 = -0.6976068572771268;

    // #[test]
    // fn almost_quadratic() {
    //     let roots = quartic(A4, A3, A2, A1, A0);
    //     let reals = roots.reals();
    //     assert_eq!(reals.len(), 2);
    //     let expected = vec![
    //         -0.835153846196954,
    //          0.835142346155438,
    //     ];
    //     assert_relative_eq!(reals[0], expected[0], max_relative = 1e-5, epsilon = 1e-5);
    //     assert_relative_eq!(reals[1], expected[1], max_relative = 1e-5, epsilon = 1e-5);
    // }

    #[test]
    fn almost_quadratic_sturm() {
        let results = roots::find_roots_sturm(&[A3 / A4, A2 / A4, A1 / A4, A0 / A4], &mut 1e-6);
        let roots: Vec<f64> = results.into_iter().map(|r| r.unwrap()).collect();
        debug!("roots: {:?}", roots);
        assert_eq!(
            roots,
            [
                -0.8351538461969557,
                 0.8351423461554403,
            ]
        )
    }

    #[test]
    fn unit_circle_l_sturm() {

    }
}