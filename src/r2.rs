use std::{ops::{Sub, Mul, Add, Div}, fmt::{Display, Formatter, self}};
use approx::{AbsDiffEq, RelativeEq};

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{dual::Dual, rotate::{Rotate, RotateArg}};

#[derive(Debug, Copy, Clone, PartialEq, Tsify, Serialize, Deserialize)]
pub struct R2<D> {
    pub x: D,
    pub y: D,
}

impl<D: Display> Display for R2<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({:.3}, {:.3})", self.x, self.y)
    }
}

impl<D: RotateArg> Rotate<D> for R2<D> {
    fn rotate(&self, theta: &D) -> Self {
        let c = (*theta).cos();
        let s = (*theta).sin();
        let x = &self.x;
        let y = &self.y;
        R2 {
            x: x.clone() * c.clone() - y.clone() * s.clone(),
            y: x.clone() * s + y.clone() * c,
        }
    }
}

// impl<D: Clone + Trig + Add<Output = D> + Sub<Output = D> + Mul<Output = D>> R2<D>
// impl<D: CanRotate> R2<D>
// {
//     pub fn rotate(&self, theta: &D) -> R2<D> {
//         let c = theta.cos();
//         let s = theta.sin();
//         let x = &self.x;
//         let y = &self.y;
//         R2 {
//             x: x.clone() * c.clone() - y.clone() * s.clone(),
//             y: x.clone() * s + y.clone() * c,
//         }
//     }
// }

impl R2<Dual> {
    pub fn v(&self) -> R2<f64> {
        R2 { x: self.x.v(), y: self.y.v() }
    }
    pub fn norm(&self) -> Dual {
        (self.x.clone() * &self.x + self.y.clone() * &self.y).sqrt()
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

impl<D: Mul<D, Output = D> + Clone> Mul<&D> for R2<D> {
    type Output = Self;
    fn mul(self, rhs: &D) -> Self::Output {
        R2 {
            x: self.x * rhs.clone(),
            y: self.y * rhs.clone(),
        }
    }
}

impl<D: Mul<D, Output = D> + Clone> Mul<D> for &R2<D> {
    type Output = R2<D>;
    fn mul(self, rhs: D) -> Self::Output {
        R2 {
            x: self.x.clone() * rhs.clone(),
            y: self.y.clone() * rhs.clone(),
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

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    #[test]
    fn test_rotate() {
        let p = R2 { x: 1., y: 1. };

        let r = p.rotate(&(PI / 4.));
        assert_relative_eq!(r.x, 0.);
        assert_relative_eq!(r.y, 2_f64.sqrt());

        let r = p.rotate(&(3. * PI / 4.));
        assert_relative_eq!(r.x, -2_f64.sqrt());
        assert_relative_eq!(r.y, 0.);

        let r = p.rotate(&(5. * PI / 4.));
        assert_relative_eq!(r.x, 0.);
        assert_relative_eq!(r.y, -2_f64.sqrt());

        let r = p.rotate(&(7. * PI / 4.));
        assert_relative_eq!(r.x, 2_f64.sqrt());
        assert_relative_eq!(r.y, 0., epsilon = 1e-15);

        let r = p.rotate(&(-PI / 4.));
        assert_relative_eq!(r.x, 2_f64.sqrt());
        assert_relative_eq!(r.y, 0., epsilon = 1e-15);

        let r = p.rotate(&(PI / 2.));
        assert_relative_eq!(r.x, -1.);
        assert_relative_eq!(r.y,  1.);

        let r = p.rotate(&(PI));
        assert_relative_eq!(r.x, -1.);
        assert_relative_eq!(r.y, -1.);

        let r = p.rotate(&(3. * PI / 2.));
        assert_relative_eq!(r.x,  1.);
        assert_relative_eq!(r.y, -1.);
    }
}