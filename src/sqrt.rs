use crate::dual::Dual;

use nalgebra::ComplexField;

pub trait Sqrt {
    fn sqrt(&self) -> Self;
}

impl Sqrt for f64 {
    fn sqrt(&self) -> f64 {
        self.sqrt()
    }
}

impl Sqrt for Dual {
    fn sqrt(&self) -> Dual {
        Dual(self.0.clone().sqrt(), self.1)
    }
}
