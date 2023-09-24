use std::{ops::{Sub, Mul, Add, Div}, fmt::{Display, Formatter, self}, iter::Sum};
use approx::{AbsDiffEq, RelativeEq};

use derive_more::{Neg, From};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{dual::Dual, rotate::{self, Rotate as _Rotate, RotateArg}, transform::{Transform::{self, Rotate, Translate, Scale, ScaleXY}, CanTransform}, sqrt::Sqrt, trig::Trig, to::To, math::recip::Recip};

#[derive(Debug, Copy, Clone, From, Neg, PartialEq, Tsify, Serialize, Deserialize)]
pub struct R2<D> {
    pub x: D,
    pub y: D,
}

impl<D: Display> Display for R2<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({:.6}, {:.6})", self.x, self.y)
    }
}

impl<D: RotateArg> rotate::Rotate<D> for R2<D> {
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

impl<
    D
    : Clone
    + Add<Output = D>
    + Mul<Output = D>
    + RotateArg
> CanTransform<D> for R2<D> {
    type Output = R2<D>;
    fn transform(&self, transform: &Transform<D>) -> Self::Output {
        match transform {
            Translate(v) => self.clone() + v,
            Scale(v) => self.clone() * v,
            ScaleXY(v) => self.clone() * v,
            Rotate(a) => self.rotate(&a),
        }
    }
}

impl<
    D
    : Clone
    + Add<Output = D>
    + Mul<Output = D>
> R2<D> {
    pub fn norm2(&self) -> D {
        self.x.clone() * self.x.clone() + self.y.clone() * self.y.clone()
    }
}

impl<
    D
    : Clone
    + Sqrt
    + Add<Output = D>
    + Mul<Output = D>
> R2<D> {
    pub fn norm(&self) -> D {
        self.norm2().sqrt()
    }
}

impl<D: Recip> Recip for R2<D> {
    fn recip(&self) -> Self {
        R2 {
            x: self.x.recip(),
            y: self.y.recip(),
        }
    }
}

impl R2<Dual> {
    pub fn v(&self) -> R2<f64> {
        R2 { x: self.x.v(), y: self.y.v() }
    }
}

impl<O, D: To<O>> To<R2<O>> for R2<D> {
    fn to(self) -> R2<O> {
        R2 { x: self.x.to(), y: self.y.to() }
    }
}

impl<D: Trig> R2<D> {
    pub fn atan2(&self) -> D {
        self.y.atan2(&self.x)
    }
}

impl<D: Clone + Add<Output = D> + Mul<Output = D> + Sqrt> R2<D>
{
    pub fn r(&self) -> D {
        (self.x.clone() * self.x.clone() + self.y.clone() * self.y.clone()).sqrt()
    }
}

impl<D: AbsDiffEq<Epsilon = f64>> AbsDiffEq for R2<D> {
    type Epsilon = f64;
    fn default_epsilon() -> Self::Epsilon {
        Dual::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.x.abs_diff_eq(&other.x, epsilon.clone()) && self.y.abs_diff_eq(&other.y, epsilon)
    }
}

impl<D: RelativeEq<Epsilon = f64>> RelativeEq for R2<D> {
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

// Want to be able to add R2<D> + R2<f64>, but this gives an infinite recursion "overflow evaluating the requirement" error at compile time.
// impl<D: Add<R, Output = D>, R> Add<R2<R>> for R2<D> {
//     type Output = R2<D>;
//     fn add(self, rhs: R2<R>) -> Self::Output {
//         R2 {
//             x: self.x + rhs.x,
//             y: self.y + rhs.y,
//         }
//     }
// }

impl<'a, D: 'a + Clone + Add<Output = D>> Add<&'a R2<D>> for R2<D> {
    type Output = Self;
    fn add(self, rhs: &'a R2<D>) -> Self::Output {
        R2 {
            x: self.x + rhs.x.clone(),
            y: self.y + rhs.y.clone(),
        }
    }
}

impl<D: Clone + Add<Output = D>> Add<D> for R2<D> {
    type Output = Self;
    fn add(self, rhs: D) -> Self::Output {
        R2 {
            x: self.x.clone() + rhs.clone(),
            y: self.y.clone() + rhs.clone(),
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

impl<D: Clone + Sub<Output = D>> Sub<&R2<D>> for R2<D> {
    type Output = Self;
    fn sub(self, rhs: &R2<D>) -> Self::Output {
        R2 {
            x: self.x - rhs.x.clone(),
            y: self.y - rhs.y.clone(),
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

impl<D: Clone + Mul<Output = D>> Mul<D> for R2<D> {
    type Output = R2<D>;
    fn mul(self, rhs: D) -> Self::Output {
        R2 {
            x: self.x.clone() * rhs.clone(),
            y: self.y.clone() * rhs.clone(),
        }
    }
}

impl<D: Clone + Mul<Output = D>> Mul<D> for &R2<D> {
    type Output = R2<D>;
    fn mul(self, rhs: D) -> Self::Output {
        R2 {
            x: self.x.clone() * rhs.clone(),
            y: self.y.clone() * rhs.clone(),
        }
    }
}

impl<'a, D: 'a + Clone + Mul<Output = D>> Mul<&'a R2<D>> for R2<D> {
    type Output = R2<D>;
    fn mul(self, rhs: &'a R2<D>) -> Self::Output {
        R2 {
            x: self.x.clone() * rhs.x.clone(),
            y: self.y.clone() * rhs.y.clone(),
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

impl<D: Div<D> + Clone> Div<D> for R2<D> {
    type Output = R2<D::Output>;
    fn div(self, rhs: D) -> Self::Output {
        R2 {
            x: self.x / rhs.clone(),
            y: self.y / rhs.clone(),
        }
    }
}

// impl<D: Div<f64, Output = D> + Clone> Div<f64> for R2<D> {
//     type Output = Self;
//     fn div(self, rhs: f64) -> Self::Output {
//         R2 {
//             x: self.x / rhs.clone(),
//             y: self.y / rhs.clone(),
//         }
//     }
// }

impl<D> Div<R2<D>> for f64
where
    f64: Div<D, Output = D>,
{
    type Output = R2<D>;
    fn div(self, rhs: R2<D>) -> Self::Output {
        R2 {
            x: self / rhs.x,
            y: self / rhs.y,
        }
    }
}

impl<D: Sum> Sum for R2<D>
where R2<D>: Add<Output = R2<D>>
{
    fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        let first = iter.next().unwrap();
        iter.fold(first, |a, b| a + b)
        // iter.fold(R2 { x: D::zero(), y: D::zero() }, |a, b| a + b)
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