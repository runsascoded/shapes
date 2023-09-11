use crate::dual::Dual;
use nalgebra::{ComplexField, RealField};

pub trait Trig {
    fn sin(&self) -> Self;
    fn cos(&self) -> Self;
    fn acos(&self) -> Self;
    fn atan(&self) -> Self;
    fn atan2(&self, o: &Self) -> Self;
    fn asinh(&self) -> Self;
    fn exp(&self) -> Self;
}

impl Trig for Dual {
    fn sin(&self) -> Dual {
        Dual(self.0.clone().sin(), self.1)
    }
    fn cos(&self) -> Dual {
        Dual(self.0.clone().cos(), self.1)
    }
    fn acos(&self) -> Dual {
        Dual(self.0.clone().acos(), self.1)
    }
    fn atan(&self) -> Dual {
        Dual(self.0.clone().atan(), self.1)
    }
    fn atan2(&self, o: &Dual) -> Dual {
        assert!(self.1 == o.1);
        let y = self.0.clone();
        let x = o.0.clone();
        let z = y.atan2(x);
        Dual(z, self.1)
    }
    fn asinh(&self) -> Dual {
        Dual(self.0.clone().asinh(), self.1)
    }
    fn exp(&self) -> Dual {
        Dual(self.0.clone().exp(), self.1)
    }
}

impl Trig for f64 {
    fn sin(&self) -> f64 {
        ComplexField::sin(*self)
    }
    fn cos(&self) -> f64 {
        ComplexField::cos(*self)
    }
    fn acos(&self) -> f64 {
        ComplexField::acos(*self)
    }
    fn atan(&self) -> f64 {
        ComplexField::atan(*self)
    }
    fn atan2(&self, o: &f64) -> f64 {
        RealField::atan2(*self, *o)
    }
    fn asinh(&self) -> f64 {
        ComplexField::asinh(*self)
    }
    fn exp(&self) -> f64 {
        ComplexField::exp(*self)
    }
}
