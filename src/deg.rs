use std::f64::consts::PI;

use crate::dual::Dual;

pub trait Deg {
    fn deg(&self) -> Self;
}

impl Deg for f64 {
    fn deg(&self) -> f64 {
        self * 180.0 / PI
    }
}

impl Deg for Dual {
    fn deg(&self) -> Dual {
        self * 180.0 / PI
    }
}