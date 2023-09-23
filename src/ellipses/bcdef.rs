use std::ops::Div;

use crate::{r2::R2, rotate::RotateArg};

use super::{xyrrt::XYRRT, cdef::CDEF};

/// Ax² + Bxy + Cy² + Dx + Ey + F = 0
pub struct BCDEF<D> {
    pub b: D,
    pub c: D,
    pub d: D,
    pub e: D,
    pub f: D,
}

impl<D: RotateArg> BCDEF<D> {
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

impl<D> BCDEF<D> {
    pub fn t(&self) -> D {
        todo!()
    }
    pub fn level(&self) -> (CDEF<D>, D) {
        todo!()
    }
    pub fn rotate(&self, t: &D) -> BCDEF<D> {
        todo!()
    }
}