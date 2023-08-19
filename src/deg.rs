use std::f64::consts::PI;

use crate::{dual::Dual, math::round};

pub trait Deg {
    fn deg(&self) -> Self;
    fn deg_str(&self) -> String;
}

impl Deg for f64 {
    fn deg(&self) -> f64 {
        self * 180.0 / PI
    }
    fn deg_str(&self) -> String {
        let deg = round(&self.deg());
        format!("{:4}", deg)
    }
}

impl Deg for Dual {
    fn deg(&self) -> Dual {
        self * 180.0 / PI
    }
    fn deg_str(&self) -> String {
        let v = round(&self.deg().v());
        let d = self.deg().d().iter().map(|t| t.deg()).map(|d| round(&d)).map(|i| format!("{:4}", i)).collect::<Vec<String>>().join(", ");
        format!("{:4} [{}]", v, d)
    }
}