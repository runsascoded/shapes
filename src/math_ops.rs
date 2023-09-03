use crate::dual::Dual;
use nalgebra::{ComplexField, RealField};

pub trait Trig {
    fn sin(&self) -> Self;
    fn cos(&self) -> Self;
    fn atan(&self) -> Self;
    fn atan2(&self, o: Self) -> Self;
}

impl Trig for Dual {
    fn sin(&self) -> Dual {
        Dual(self.0.clone().sin(), self.1)
    }
    fn cos(&self) -> Dual {
        Dual(self.0.clone().cos(), self.1)
    }
    fn atan(&self) -> Dual {
        Dual(self.0.clone().atan(), self.1)
    }
    fn atan2(&self, o: Dual) -> Dual {
        assert!(self.1 == o.1);
        let x = self.0.clone();
        let y = o.0;
        let z = x.atan2(y);
        Dual(z, self.1)
    }
}

impl Trig for f64 {
    fn sin(&self) -> f64 {
        ComplexField::sin(*self)
    }
    fn cos(&self) -> f64 {
        ComplexField::cos(*self)
    }
    fn atan(&self) -> f64 {
        ComplexField::atan(*self)
    }
    fn atan2(&self, o: f64) -> f64 {
        RealField::atan2(*self, o)
    }
}
