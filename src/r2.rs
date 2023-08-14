use std::ops::{Sub, Mul, Add};
use derive_more::{From};

use nalgebra::Const;
use num_dual::DualVec64;
use serde::{Deserialize, Serialize};

use crate::dual::Dual;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct R2<D> {
    pub x: D,
    pub y: D,
}

impl<'a> From<R2<DualVec64<Const<3>>>> for R2<Dual> {
    fn from(dv: R2<DualVec64<Const<3>>>) -> Self {
        R2 {
            x: dv.x.into(),
            y: dv.y.into(),
        }
    }
}

impl<D: Add<Output = D>> Add for R2<D> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        R2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<D: Sub<Output = D>> Sub for R2<D> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        R2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<D: Mul<Output = D>> Mul for R2<D> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        R2 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl<D: Mul<D, Output = D>> Mul<D> for R2<D> {
    type Output = Self;
    fn mul(self, rhs: D) -> Self::Output {
        R2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}
