use std::{ops::{Sub, Mul, Add, Div}, fmt::{Display, Formatter, self}};
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

impl<D: Display> Display for R2<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "R2 {{ x: {}, y: {} }}", self.x, self.y)
    }
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

impl<D: Mul<D, Output = D> + Clone> Mul<D> for R2<D> {
    type Output = Self;
    fn mul(self, rhs: D) -> Self::Output {
        R2 {
            x: self.x * rhs.clone(),
            y: self.y * rhs.clone(),
        }
    }
}

impl<D: Div<Output = D>> Div for R2<D> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        R2 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

impl<D: Div<D, Output = D> + Clone> Div<D> for R2<D> {
    type Output = Self;
    fn div(self, rhs: D) -> Self::Output {
        R2 {
            x: self.x / rhs.clone(),
            y: self.y / rhs.clone(),
        }
    }
}

