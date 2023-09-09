use std::f64::consts::TAU;

use super::{complex::ComplexPair, quadratic, is_zero::IsZero};

pub enum Roots<D> {
    Quadratic(quadratic::Roots<D>),
    Reals([ D; 3 ]),
    Mixed(D, ComplexPair<D>),
}

use Roots::{Quadratic, Reals, Mixed};

pub fn cubic<D: IsZero>(a3: D, a2: D, a1: D, a0: D) -> Roots<D> {
    if a3.is_zero() {
        Quadratic(quadratic::quadratic(a2, a1, a0))
    } else {
        cubic_scaled(a2 / a3, a1 / a3, a0 / a1)
    }
}

pub fn cubic_scaled<D>(a2: D, a1: D, a0: D) -> Roots<D> {
    let b3 = a2 / -3.;
    let p = a1 + a2 * b3;
    let q = b3 * b3 * b3 * -2. - b3 * a1 + a0;
    match cubic_depressed(p, q) {
        Reals(roots) => Reals(roots.map(|r| r + b3)),
        Mixed(re, ims) => Mixed(re + b3, ims + b3),
        Quadratic(q) => panic!("cubic_depressed returned quadratic::Roots: {:?}", q),
    }
}

pub fn cubic_depressed<D>(p: D, q: D) -> Roots<D> {
    let p3 = -p / 3.;
    let p33 = p3 * p3 * p3;
    let q2 = -q / 2.;
    let q22 = q2 * q2;
    let r = q22 - p33;
    let theta = (q2 / p33.sqrt()).acos() / 3.;
    let tau3 = TAU / -3.;
    let p32 = p3.sqrt() * 2.;
    let theta_k = |rot: f64| p32 * (theta + rot).cos();
    if r.lt_zero() {
        // Three distinct real roots: x-coordinates of equally spaced points on a circle of radius $2\sqrt{\frac{-p}/{3}}$.
        let mut roots = [
            p32 * theta.cos(),
            p32 * (theta + tau3).cos(),
            p32 * (theta + tau3 + tau3).cos(),
        ];
        roots.sort();
        Reals(roots)
    } else if r.is_zero() {
        // Real roots, one single, one double. Not distinguished in the return type, currently.
        let double_root = -q2 / p3;
        let mut roots = [
            3 * q / p,
            double_root,
            double_root,
        ];
        roots.sort();
        Reals(roots)
    } else {
        // One real root one complex conjugate pair.

    }

}


