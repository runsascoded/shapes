use crate::dual::Dual;

use nalgebra::ComplexField;

pub trait Cbrt {
    fn cbrt(&self) -> Self;
}

impl Cbrt for f64 {
    fn cbrt(&self) -> f64 {
        f64::cbrt(*self)
    }
}

impl Cbrt for Dual {
    fn cbrt(&self) -> Dual {
        Dual(self.0.clone().cbrt(), self.1)
    }
}
