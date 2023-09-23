use core::f64;
use std::ops::{Div, Sub, Mul, Add};

use crate::{r2::R2, rotate::RotateArg, trig::Trig, math::recip::Recip};

use super::{xyrrt::XYRRT, cdef::{CDEF, self}};

/// "Unaligned" ellipse: x² + Bxy + Cy² + Dx + Ey + F = 0
pub struct BCDEF<D> {
    pub b: D,
    pub c: D,
    pub d: D,
    pub e: D,
    pub f: D,
}

pub trait XyrrtArg: LevelArg + cdef::XyrrArg + RotateArg {}
impl<D: LevelArg + cdef::XyrrArg + RotateArg> XyrrtArg for D {}

impl<D: XyrrtArg> BCDEF<D> {
    pub fn xyrrt(&self) -> XYRRT<D> {
        let (cdef, t) = self.level();
        cdef.xyrr().rotate(&t)
    }
}
impl<D: Clone + Div<Output = D>> BCDEF<D> {
    pub fn scale_xy(&self, xy: &R2<D>) -> BCDEF<D> {
        BCDEF {
            b: self.b.clone() / xy.x.clone() / xy.y.clone(),
            c: self.c.clone() / xy.x.clone() / xy.x.clone(),
            d: self.d.clone() / xy.x.clone(),
            e: self.e.clone() / xy.y.clone(),
            f: self.f.clone(),
        }
    }
}

pub trait LevelArg
: Clone
+ Recip
+ Trig
+ Add<Output = Self>
+ Sub<Output = Self>
+ Sub<f64, Output = Self>
+ Mul<Output = Self>
+ Div<Output = Self>
+ Div<f64, Output = Self>
{}
impl<
    D
    : Clone
    + Recip
    + Trig
    + Add<Output = D>
    + Sub<Output = D>
    + Sub<f64, Output = D>
    + Mul<Output = D>
    + Div<Output = D>
    + Div<f64, Output = D>
> LevelArg for D {}

impl<D: LevelArg> BCDEF<D> {
    pub fn t(&self) -> D {
        let BCDEF { b, c, .. } = self;
        (c.clone() - 1.).atan2(b) / 2.
    }
    pub fn level(&self) -> (CDEF<D>, D) {
        let t = self.t();
        let cos = t.cos();
        let cos2 = cos.clone() * cos.clone();
        let sin = t.sin();
        let sin2 = sin.clone() * sin.clone();
        let BCDEF { b, c, d, e, f } = self;
        let bcs = b.clone() * cos.clone() * sin.clone();
        let a = cos2.clone() - bcs.clone() + self.c.clone() * sin2.clone();
        let ra = a.recip();
        (
            CDEF {
                c: ra.clone() * (sin2.clone() - bcs.clone() + c.clone() * cos2.clone()),
                d: ra.clone() * (d.clone() * cos.clone() - e.clone() * sin.clone()),
                e: ra.clone() * (d.clone() * sin.clone() + e.clone() * cos.clone()),
                f: ra * f.clone(),
            },
            t
        )
    }
}