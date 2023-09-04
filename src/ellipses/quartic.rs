use std::ops::Mul;

use log::warn;
use roots::find_roots_quartic;

use crate::dual::{D, Dual};

pub struct Root(pub D, pub bool);  // (root, double_root

pub fn quartic_roots(c_4: D, c_3: D, c_2: D, c_1: D, c_0: D) -> Vec<Root> {
    let coeff_duals: [ &D; 5 ] = [
        &c_0,
        &c_1,
        &c_2,
        &c_3,
        &c_4,
    ];
    let coeffs: [ f64; 5 ] = [
        c_0.v(),
        c_1.v(),
        c_2.v(),
        c_3.v(),
        c_4.v(),
    ];
    let [ a_0, a_1, a_2, a_3, a_4 ] = coeffs;
    let roots = find_roots_quartic(a_4, a_3, a_2, a_1, a_0);
    let d_3: f64 = f64::mul(a_4, 4.);
    let d_2: f64 = f64::mul(a_3, 3.);
    let d_1: f64 = f64::mul(a_2, 2.);
    let d_0: f64 = a_1;
    let fp = |x: f64| d_3 * x * x * x + d_2 * x * x + d_1 * x + d_0;
    let n = c_4.1;
    let mut ys: Vec<Root> = vec![];
    for root in roots.as_ref() {
        let fd = fp(*root);
        let mut double_root = false;
        if fd == 0. {
            // Multiple root
            let e_2: f64 = 3. * d_3;
            let e_1: f64 = 2. * d_2;
            let e_0: f64 = d_1;
            let fpp = |x: f64| e_2 * x * x + e_1 * x + e_0;
            let fdd = fpp(*root);
            if fdd == 0. {
                let f_1 = 2. * e_2;
                let f_0 = e_1;
                let fppp = |x: f64| f_1 * x + f_0;
                let fddd = fppp(*root);
                let order = if fddd == 0. { 4 } else { 3 };
                warn!("Skipping multiple root {} ({})", root, order);
                continue;
            } else {
                double_root = true;
            }
        }
        let root2 = root * root;
        let root_pows = [ 1., *root, root2, root2 * root, root2 * root2 ];
        // let mut dual_root_d: Vec<f64> = repeat(0.).take(n).collect();
        let mut y: D = Dual::scalar(*root, n);
        let mut y_d = y.0.eps;
        for (coeff, root_pow) in coeff_duals.into_iter().zip(root_pows.iter()).rev() {
            y_d += coeff.0.clone().eps.clone() * -root_pow / fd;
        }
        y.0.eps = y_d;
        println!("root: {}", y.clone());
        ys.push(Root(y, double_root));
    }
    ys
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
        let roots = quartic_roots(c_4, c_3, c_2, c_1, c_0);
        assert_eq!(roots.len(), 4);
        // for root in roots {
        //     println!("{}", root);
        // }
   }
}