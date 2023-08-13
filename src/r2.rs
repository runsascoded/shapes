use std::ops::{Sub, Mul, Add};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct R2<D> {
    pub x: D,
    pub y: D,
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

impl<D: Mul<D, Output = D> + Copy> Mul<D> for R2<D> {
    type Output = Self;
    fn mul(self, rhs: D) -> Self::Output {
        R2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}
