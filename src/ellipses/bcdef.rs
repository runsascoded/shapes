use core::f64;
use std::{ops::{Div, Sub, Mul, Add, Neg}, fmt::{Display, Debug}};

use approx::{RelativeEq, AbsDiffEq};
use log::debug;

use crate::{r2::R2, rotate::RotateArg, trig::Trig, math::{recip::Recip, deg::Deg, is_zero::IsZero}, zero::Zero};

use super::{xyrrt::XYRRT, cdef::{CDEF, self}};

/// "Unaligned" ellipse: x² + Bxy + Cy² + Dx + Ey + F = 0
#[derive(Debug, Clone, PartialEq)]
pub struct BCDEF<D> {
    pub b: D,
    pub c: D,
    pub d: D,
    pub e: D,
    pub f: D,
    pub t: D,
}

pub trait XyrrtArg: Display + LevelArg + cdef::XyrrArg + RotateArg {}
impl<D: Display + LevelArg + cdef::XyrrArg + RotateArg> XyrrtArg for D {}

impl<D: XyrrtArg> BCDEF<D> {
    pub fn xyrrt(&self) -> XYRRT<D> {
        let cdef = self.level();
        // debug!("BCDEF leveled: {}", cdef);
        let xyrr = cdef.xyrr();
        // debug!("BCDEF leveled xyrr: {}", xyrr);
        xyrr.rotate(&self.t)
    }
}
impl BCDEF<f64> {
    pub fn at_y(&self, y: f64) -> Vec<f64> {
        let b = (self.b * y + self.d) / -2.;
        let c = self.c * y * y + self.e * y + self.f;
        let d = b * b - c;
        if d < 0. {
            vec![]
        } else if d == 0. {
            vec![b]
        } else {
            let d = d.sqrt();
            vec![b - d, b + d]
        }
    }
}
impl<
    D
    : Clone
    + Debug
    + Display
    + Trig
    + IsZero
    + Zero
    + Add<f64, Output = D>
    + Sub<f64, Output = D>
    + Div<Output = D>
    + Div<f64, Output = D>
    + Neg<Output = D>
> BCDEF<D> {
    pub fn new(b: D, c: D, d: D, e: D, f: D) -> BCDEF<D> {
        // let t = (b.clone() / (c.clone() - 1.)).atan() / 2.;
        let c1 = -c.clone() + 1.;
        let t = if b.is_zero() { b.clone().zero() } else { b.clone().atan2(&c1) / 2. };
        // debug!("BCDEF::new: b {:?}", b);
        // debug!("BCDEF::new: c {:?}", c);
        // debug!("BCDEF::new: d {:?}", d);
        // debug!("BCDEF::new: e {:?}", e);
        // debug!("BCDEF::new: f {:?}", f);
        // debug!("BCDEF::new: t {:?}", t);
        BCDEF { b, c, d, e, f, t }
    }
}
impl<
    D
    : Clone
    + Debug
    + Display
    + IsZero
    + Zero
    + Add<f64, Output = D>
    + Sub<f64, Output = D>
    + Mul<Output = D>
    + Div<Output = D>
    + Div<f64, Output = D>
    + Neg<Output = D>
    + Trig
> BCDEF<D> {
    pub fn scale_xy(&self, xy: &R2<D>) -> BCDEF<D> {
        let R2 { x, y } = xy;
        let x2 = x.clone() * x.clone();
        BCDEF::new(
            self.b.clone() * x.clone() / y.clone(),
            self.c.clone() * x2.clone() / y.clone() / y.clone(),
            self.d.clone() * x.clone(),
            self.e.clone() * x2.clone() / y.clone(),
            self.f.clone() * x2.clone(),
        )
    }
}

pub trait LevelArg
: Clone
+ Deg
+ Display
+ IsZero
+ Recip
+ Trig
+ Add<Output = Self>
+ Sub<Output = Self>
+ Sub<f64, Output = Self>
+ Mul<Output = Self>
+ Div<Output = Self>
+ Div<f64, Output = Self>
+ Neg<Output = Self>
{}
impl<
    D
    : Clone
    + Deg
    + Display
    + IsZero
    + Recip
    + Trig
    + Add<Output = D>
    + Sub<Output = D>
    + Sub<f64, Output = D>
    + Mul<Output = D>
    + Div<Output = D>
    + Div<f64, Output = D>
    + Neg<Output = D>
> LevelArg for D {}

impl<D: LevelArg> BCDEF<D> {
    pub fn level(&self) -> CDEF<D> {
        let BCDEF { b, c, d, e, f, t } = self;
        // debug!("leveling {}", self);
        // debug!("leveling: t {}", t);
        let cos = t.cos();
        // debug!("leveling: cos {}", cos);
        let cos2 = cos.clone() * cos.clone();
        let sin = -t.sin();
        // debug!("leveling: sin {}", sin);
        let sin2 = sin.clone() * sin.clone();
        let bcs = b.clone() * cos.clone() * sin.clone();
        // debug!("leveling: cos2 {}", cos2);
        // debug!("leveling: bcs {}", bcs);
        // debug!("leveling: sin2 {}", sin2);
        let a = cos2.clone() - bcs.clone() + self.c.clone() * sin2.clone();
        let ra = a.recip();
        // debug!("leveling, a {} ra {}", a, ra);
        CDEF {
            c: ra.clone() * (sin2.clone() + bcs.clone() + c.clone() * cos2.clone()),
            d: ra.clone() * (d.clone() * cos.clone() - e.clone() * sin.clone()),
            e: ra.clone() * (d.clone() * sin.clone() + e.clone() * cos.clone()),
            f: ra * f.clone(),
        }
    }
}

impl<D: Clone + Deg + Display + Neg<Output = D>> Display for BCDEF<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x² + {}xy + {}y² + {}x + {}y = {} ({} radians, {}°)", self.b, self.c, self.d, self.e, -self.f.clone(), self.t, self.t.deg_str())
    }
}

impl<D: AbsDiffEq<Epsilon = f64> + Clone> AbsDiffEq for BCDEF<D> {
    type Epsilon = D::Epsilon;
    fn default_epsilon() -> Self::Epsilon {
        D::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.b.abs_diff_eq(&other.b, epsilon.clone())
        && self.c.abs_diff_eq(&other.c, epsilon.clone())
        && self.d.abs_diff_eq(&other.d, epsilon.clone())
        && self.e.abs_diff_eq(&other.e, epsilon.clone())
        && self.f.abs_diff_eq(&other.f, epsilon)
    }
}

impl<D: RelativeEq<Epsilon = f64> + Clone> RelativeEq for BCDEF<D> {
    fn default_max_relative() -> Self::Epsilon {
        D::default_max_relative()
    }
    fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool {
        self.b.relative_eq(&other.b, epsilon.clone(), max_relative.clone())
        && self.c.relative_eq(&other.c, epsilon.clone(), max_relative.clone())
        && self.d.relative_eq(&other.d, epsilon.clone(), max_relative.clone())
        && self.e.relative_eq(&other.e, epsilon.clone(), max_relative.clone())
        && self.f.relative_eq(&other.f, epsilon, max_relative)
    }
}
