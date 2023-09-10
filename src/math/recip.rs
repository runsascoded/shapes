use std::ops::{Neg, Div};

use nalgebra::ComplexField;

use crate::dual::Dual;

use super::complex::{Complex, self};

pub trait Recip {
    fn recip(&self) -> Self;
}

impl Recip for f64 {
    fn recip(&self) -> f64 {
        1. / self
    }
}

impl Recip for Dual {
    fn recip(&self) -> Dual {
        Dual(self.0.clone().recip(), self.1)
    }
}

impl<
    D
    : complex::Norm
    + Div<Output = D>
    + Neg<Output = D>
> Recip for Complex<D> {
    fn recip(&self) -> Complex<D> {
        let norm = self.norm();
        Complex {
            re: self.re.clone() / norm.clone(),
            im: -self.im.clone() / norm,
        }
    }
}