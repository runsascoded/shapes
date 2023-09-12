use std::ops::Mul;

use derive_more::Display;
use log::{warn, debug};
use roots::{find_roots_quartic, find_roots_sturm, find_roots_eigen};

use crate::{dual::{D, Dual}, fmt::Fmt, zero::Zero, math::quartic::quartic};

#[derive(Debug, Clone)]
pub struct Root<D>(pub D, pub bool);  // (root, double_root

pub trait Quartic
where
    Self: Sized
{
    fn quartic_roots(c_4: Self, c_3: Self, c_2: Self, c_1: Self, c_0: Self) -> Vec<Root<Self>>;
}

impl Quartic for f64 {
    fn quartic_roots(a_4: f64, a_3: f64, a_2: f64, a_1: f64, a_0: f64) -> Vec<Root<f64>> {
        // roots::find_roots_quartic
        // let reals = find_roots_quartic(a_4, a_3, a_2, a_1, a_0).as_ref().to_vec();

        // crate::math::quartic
        debug!("{}x^4 + {}x^3 + {}x^2 + {}x + {}", a_4, a_3, a_2, a_1, a_0);
        let roots0 = quartic(a_4, a_3, a_2, a_1, a_0);
        debug!("roots0: {:?}", roots0);
        let reals = roots0.reals();

        // roots::{find_roots_stur, find_roots_eigen}
        // let mut first_nonzero: Option<f64> = None;
        // let mut rest: Vec<f64> = vec![];
        // for coeff in &[ a_4, a_3, a_2, a_1, a_0 ] {
        //     match first_nonzero {
        //         None => if *coeff != 0. {
        //             first_nonzero = Some(*coeff);
        //         },
        //         Some(first_nonzero) => rest.push(*coeff / first_nonzero),
        //     }
        // }
        // Sturm
        // let results = find_roots_sturm(&rest, &mut 1e-6);
        // let reals: Vec<f64> = results.into_iter().map(|r| r.unwrap()).collect();
        // Eigen
        // let reals = find_roots_eigen(rest.into_iter().rev().collect());
        let d_3: f64 = f64::mul(a_4, 4.);
        let d_2: f64 = f64::mul(a_3, 3.);
        let d_1: f64 = f64::mul(a_2, 2.);
        let d_0: f64 = a_1;
        let fp = |x: f64| {
            let x2 = x * x;
            d_3 * x2 * x + d_2 * x2 + d_1 * x + d_0
        };
        let mut roots: Vec<Root<f64>> = vec![];
        let f = |x: f64| {
            let x2 = x * x;
            (a_4 * x2 * x2) + (a_3 * x2 * x) + (a_2 * x2) + (a_1 * x) + a_0
        };
        println!("Roots: {:?}", reals);
        for root in reals {
            println!("  x: {}, f(x): {}", root, f(root));
            let fd = fp(root);
            let mut double_root = false;
            if fd == 0. {
                // Multiple root
                let e_2: f64 = 3. * d_3;
                let e_1: f64 = 2. * d_2;
                let e_0: f64 = d_1;
                let fpp = |x: f64| e_2 * x * x + e_1 * x + e_0;
                let fdd = fpp(root);
                if fdd == 0. {
                    let f_1 = 2. * e_2;
                    let f_0 = e_1;
                    let fppp = |x: f64| f_1 * x + f_0;
                    let fddd = fppp(root);
                    let order = if fddd == 0. { 4 } else { 3 };
                    warn!("Skipping multiple root {} ({})", root, order);
                    continue;
                } else {
                    double_root = true;
                }
            }
            roots.push(Root(root, double_root));
        }
        roots
    }
}

impl Quartic for Dual {
    fn quartic_roots(c_4: D, c_3: D, c_2: D, c_1: D, c_0: D) -> Vec<Root<D>> {
        let coeff_duals: [ &D; 5 ] = [
            &c_0,
            &c_1,
            &c_2,
            &c_3,
            &c_4,
        ];
        let coeffs: [ f64; 5 ] = [
            c_0.clone().into(),
            c_1.clone().into(),
            c_2.clone().into(),
            c_3.clone().into(),
            c_4.clone().into(),
        ];
        let [ a_0, a_1, a_2, a_3, a_4 ] = coeffs;
        let roots = Quartic::quartic_roots(a_4, a_3, a_2, a_1, a_0);

        let d_3: f64 = f64::mul(a_4, 4.);
        let d_2: f64 = f64::mul(a_3, 3.);
        let d_1: f64 = f64::mul(a_2, 2.);
        let d_0: f64 = a_1;
        let fp = |x: f64| {
            let x2 = x * x;
            d_3 * x2 * x + d_2 * x2 + d_1 * x + d_0
        };
        let mut dual_roots: Vec<Root<D>> = vec![];
        for Root(root, double_root) in roots {
            let fd = fp(root);
            let root2 = root * root;
            let root_pows = [ 1., root, root2, root2 * root, root2 * root2 ];
            // let mut dual_root_d: Vec<f64> = repeat(0.).take(n).collect();
            let mut y: D = Zero::zero(&c_4) + root;
            let mut y_d = y.0.eps;
            for (coeff, root_pow) in coeff_duals.into_iter().zip(root_pows.iter()).rev() {
                y_d += coeff.0.clone().eps.clone() * -root_pow / fd;
            }
            y.0.eps = y_d;
            // println!("root: {}", y.clone());
            dual_roots.push(Root(y, double_root));
        }
        // println!("Roots:");
        // for root in &dual_roots {
        //     println!("  Root: {}, double? {}", root.0.s(2), root.1);
        // }
        dual_roots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quartic_roots() {
        let d_4 = vec![ 1., 0., 0., 0., 0., ];
        let d_3 = vec![ 0., 1., 0., 0., 0., ];
        let d_2 = vec![ 0., 0., 1., 0., 0., ];
        let d_1 = vec![ 0., 0., 0., 1., 0., ];
        let d_0 = vec![ 0., 0., 0., 0., 1., ];
        let c_4 = Dual::new( 1., d_4);
        let c_3 = Dual::new(-6., d_3);
        let c_2 = Dual::new(11., d_2);
        let c_1 = Dual::new(-6., d_1);
        let c_0 = Dual::new( 0., d_0);
        let roots = Quartic::quartic_roots(c_4, c_3, c_2, c_1, c_0);
        // assert_eq!(roots.len(), 4);
        for root in roots {
            println!("{:?}", root);
        }
    }
}
