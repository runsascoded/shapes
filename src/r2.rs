use std::{ops::{Sub, Mul, Add, Div}, fmt::{Display, Formatter, self}};
use approx::{AbsDiffEq, RelativeEq};

use serde::{Deserialize, Serialize};

use crate::dual::Dual;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct R2<D> {
    pub x: D,
    pub y: D,
}

impl<D: Display> Display for R2<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({:.3}, {:.3})", self.x, self.y)
    }
}

impl R2<Dual> {
    pub fn v(&self) -> R2<f64> {
        R2 { x: self.x.v(), y: self.y.v() }
    }
}

impl AbsDiffEq for R2<Dual> {
    type Epsilon = f64;
    fn default_epsilon() -> Self::Epsilon {
        Dual::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.x.abs_diff_eq(&other.x, epsilon.clone()) && self.y.abs_diff_eq(&other.y, epsilon)
    }
}

impl RelativeEq for R2<Dual> {
    fn default_max_relative() -> Self::Epsilon {
        Dual::default_max_relative()
    }
    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        self.x.relative_eq(&other.x, epsilon.clone(), max_relative.clone()) && self.y.relative_eq(&other.y, epsilon, max_relative)
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

