use std::{f64::consts::TAU, ops::{Div, Mul, Add, Sub}, fmt};

use crate::trig::Trig;

use super::{complex::{ComplexPair, Complex}, quadratic, abs::{Abs, AbsArg}, is_zero::IsZero, cbrt::Cbrt, recip::Recip};

#[derive(Debug, Clone, PartialEq)]
pub enum Roots<D> {
    Quadratic(quadratic::Roots<D>),
    Reals([ D; 3 ]),
    Mixed(D, ComplexPair<D>),
}

use Roots::{Quadratic, Reals, Mixed};
use ordered_float::OrderedFloat;

pub trait Arg
: fmt::Debug
+ Into<f64>
+ IsZero
+ Cbrt
+ AbsArg
+ Recip
+ Trig
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

pub fn cubic<D: Arg>(a3: D, a2: D, a1: D, a0: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
{
    if a3.is_zero() {
        Quadratic(quadratic::quadratic(a2, a1, a0))
    } else {
        cubic_scaled(a2 / a3.clone(), a1 / a3.clone(), a0 / a3)
    }
}

pub fn cubic_scaled<D: Arg>(a2: D, a1: D, a0: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>
{
    let b3 = a2.clone() / -3.;
    let p = a1.clone() + a2 * b3.clone();
    let q = b3.clone() * b3.clone() * b3.clone() * -2. - b3.clone() * a1 + a0;
    match cubic_depressed(p, q) {
        Reals(roots) => Reals(roots.map(|r| r + b3.clone())),
        Mixed(re, ims) => Mixed(re + b3.clone(), ims + b3),
        Quadratic(q) => panic!("cubic_depressed returned quadratic::Roots: {:?}", q),
    }
}

static TAU3: f64 = TAU / -3.;

pub fn cubic_depressed<D: Arg>(p: D, q: D) -> Roots<D>
where
    Complex<D>
    : Add<D, Output = Complex<D>>
    + Add<Complex<D>, Output = Complex<D>>
    + Mul<D, Output = Complex<D>>
    + Mul<Complex<f64>, Output = Complex<D>>,
{
    let sin_tau3: f64 = TAU3.sin();
    let u_1: Complex<f64> = Complex { re: -1. / 2., im:  sin_tau3 };
    // let u_2: Complex<f64> = Complex { re: -1. / 2., im: -sin_tau3 };
    if p.is_zero() {
        let re = -q.cbrt();
        let im = Complex::re(re.clone()) * u_1.clone();
        Mixed(re, im)
    } else if p.lt_zero() {
        let p3 = -p / 3.;
        let q2 = q / 2.;
        let p3sq = p3.sqrt();
        let u = q2 / p3 / p3sq.clone();
        if (u.abs() - 1.).lt_zero() {
            let r = p3sq.clone() * 2.;
            let theta = u.acos() / 3.;
            let mut roots = [
                r.clone() *  theta.clone().cos(),
                r.clone() * (theta.clone() + TAU3).cos(),
                r * (theta + TAU3 + TAU3).cos(),
            ];
            roots.sort_by_cached_key(|r| OrderedFloat::<f64>(r.clone().into()));
            Reals(roots)
        } else {
            let w = u.clone() + (u.clone() * u - 1.).sqrt();
            let m = w.cbrt();
            let re = (m.clone() + m.recip()) * p3sq.clone();
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
    // let q22 = q2 * q2;
    // let r = q22 - p33;
    // let theta = (q2 / p33.sqrt()).acos() / 3.;
    // let p32 = p3.sqrt() * 2.;
    // let theta_k = |rot: f64| p32 * (theta + rot).cos();
    // if r.lt_zero() {
    //     // Three distinct real roots: x-coordinates of equally spaced points on a circle of radius $2\sqrt{\frac{-p}/{3}}$.
    //     let mut roots = [
    //         p32 * theta.cos(),
    //         p32 * (theta + tau3).cos(),
    //         p32 * (theta + tau3 + tau3).cos(),
    //     ];
    //     roots.sort();
    //     Reals(roots)
    // } else if r.is_zero() {
    //     // Real roots, one single, one double. Not distinguished in the return type, currently.
    //     let double_root = -q2 / p3;
    //     let mut roots = [
    //         3 * q / p,
    //         double_root,
    //         double_root,
    //     ];
    //     roots.sort();
    //     Reals(roots)
    // } else {
    //     // One real root one complex conjugate pair.

    // }

}


