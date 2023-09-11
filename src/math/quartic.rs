use std::{f64::consts::TAU, ops::{Div, Mul, Add, Sub, Neg}, fmt};

use crate::{trig::Trig, dual::Dual, sqrt::Sqrt, zero::Zero};

use super::{complex::{ComplexPair, Complex, self, SqrtArg, Numeric}, quadratic, abs::{Abs, AbsArg}, is_zero::IsZero, cbrt::Cbrt, recip::Recip, deg::Deg};

use super::cubic;

pub enum Roots<D> {
    Cubic(cubic::Roots<D>),
    Reals([ D; 4 ]),
    Mixed(D, D, ComplexPair<D>),
    Imags(ComplexPair<D>, ComplexPair<D>),
}

use Roots::*;
use log::debug;

impl<D: Clone + fmt::Debug + Zero + Neg<Output = D> + Zero> Roots<D> {
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
    + Sub<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
    + Mul<f64, Output = Complex<D>>
    + Div<f64, Output = Complex<D>>
    + Neg<Output = Complex<D>>
{
    debug!("quartic_scaled({:?}, {:?}, {:?}, {:?})", b, c, d, e);
    let b4 = b / 4.;
    let b4sq = b4.clone() * b4.clone();
    let c2 = c.clone() - b4sq.clone() * 6.;
    let d2 = b4sq.clone() * b4.clone() * 8. - b4.clone() * c.clone() * 2. + d.clone();
    let e2 = b4sq.clone() * b4sq.clone() * -3. + b4sq * c - b4.clone() * d + e;
    match quartic_depressed(c2, d2, e2) {
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
    }
}

pub fn quartic_depressed<D: Arg>(c: D, d: D, e: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Add<Complex<D>, Output = Complex<D>>
    + Sub<Complex<D>, Output = Complex<D>>
    + Sub<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
    + Mul<f64, Output = Complex<D>>
    + Div<f64, Output = Complex<D>>
    + Neg<Output = Complex<D>>
{
    debug!("quartic_depressed({:?}, {:?}, {:?})", c, d, e);
    let roots = if d.is_zero() {
        quartic_biquadratic(c, e)
    } else {
        let a_2 = c.clone() * 2.;
        let a_1 = c.clone() * c.clone() - e * 4.;
        let a_0 = -d.clone() * d.clone();
        let cubic_roots = cubic::cubic(a_2.zero() + 1., a_2, a_1, a_0).reals();
        debug!("cubic_roots: {:?}", cubic_roots);
        let u = cubic_roots.iter().rev().next().unwrap();
        let usq = Sqrt::sqrt(&Complex::re(u.clone())) / 2.;
        let usqr = usq.recip();
        debug!("u {:?}, usq {:?}, usqr {:?}", u, usq, usqr);
        let c2 = c * 2.;
        let usqrd = usqr.clone() * d;
        let c = Complex::re(-u.clone()) - c2;
        let d0 = c.clone() - usqrd.clone();
        let d1 = c + usqrd;
        let d0 = Sqrt::sqrt(&d0) / 2.;
        let d1 = Sqrt::sqrt(&d1) / 2.;
        let roots = [
             usq.clone() + d0.clone(),
             usq.clone() - d0.clone(),
            -usq.clone() + d1.clone(),
            -usq.clone() - d1.clone(),
        ];
        roots
    };
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
        Imags(imags[0].clone(), imags[2].clone())
    } else {
        Mixed(reals[0].clone(), reals[1].clone(), imags[0].clone())
    }
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
{
    let [ r0, r1 ] = quadratic::quadratic(c.zero() + 1., c.clone(), e.clone()).two_roots();
    let sq0 = Sqrt::sqrt(&r0);
    let sq1 = Sqrt::sqrt(&r1);
    let roots = [
        sq0.clone(),
        -sq0.clone(),
        sq1.clone(),
        -sq1.clone(),
    ];
    debug!("quartic_biquadratic({:?}, {:?}) = {:?}", c, e, roots);
    roots
}

#[cfg(test)]
mod tests {

    use super::*;

    use test_log::test;

    fn check(r0: f64, r1: f64, r2: f64, r3: f64, scale: f64) {
        let unscaled_coeffs = [
            1.,
            -(r0 + r1 + r2 + r3),
            r0 * r1 + r0 * r2 + r0 * r3 + r1 * r2 + r1 * r3 + r2 * r3,
            -(r0 * r1 * r2 + r0 * r1 * r3 + r0 * r2 * r3 + r1 * r2 * r3),
            r0 * r1 * r2 * r3,
        ];
        let coeffs = unscaled_coeffs.map(|c| c * scale);
        let [ a4, a3, a2, a1, a0 ] = coeffs;
        // let f = |x: f64| a4 * x * x * x * x + a3 * x * x * x + a2 * x * x + a1 * x + a0;
        let roots = quartic::<f64>(a4, a3, a2, a1, a0);
        let ε = 1e-5;
        let actual = crate::math::roots::Roots(roots.all());
        let expected_reals = crate::math::roots::Roots([ r0, r1, r2, r3 ].into_iter().map(Complex::re).collect());
        assert_relative_eq!(actual, expected_reals, max_relative = ε, epsilon = ε);
    }

    #[test]
    fn sweep() {
        check(-10., -10., -10., -1., 1.);
        // let vals = [ -10., -1., -0.1, 0., 0.1, 1., 10., ];
        // let n = vals.len();
        // for i0 in 0..n {
        //     let r0 = vals[i0];
        //     for i1 in i0..n {
        //         let r1 = vals[i1];
        //         for i2 in i1..n {
        //             let r2 = vals[i2];
        //             for i3 in i2..n {
        //                 let r3 = vals[i3];
        //                 let scale = 1.;
        //                 check(r0, r1, r2, r3, scale);
        //             }
        //         }
        //     }
        // }
    }
}