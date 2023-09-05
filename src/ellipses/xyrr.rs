use std::{ops::{Mul, Div, Add, Sub}, fmt::Display};

use derive_more::From;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{r2::R2, rotate::{Rotate, RotateArg}, dual::{D, Dual}, shape::Duals};

use super::{xyrrt::XYRRT, cdef::CDEF};


pub trait UnitIntersectionsArg: Clone + Add<Output = Self> + Add<f64, Output = Self> + Sub<Output = Self> + Sub<f64, Output = Self> + Mul<Output = Self> + Div<Output = Self> {}
impl<D: Clone + Add<Output = D> + Add<f64, Output = D> + Sub<Output = D> + Sub<f64, Output = D> + Mul<Output = D> + Div<Output = D>> UnitIntersectionsArg for D {}

#[derive(Debug, Clone, From, Serialize, Deserialize, Tsify)]
pub struct XYRR<D> {
    pub idx: usize,
    pub c: R2<D>,
    pub r: R2<D>,
}

impl XYRR<f64> {
    pub fn dual(&self, duals: &Duals) -> XYRR<D> {
        let cx = Dual::new(self.c.x, duals[0].clone());
        let cy = Dual::new(self.c.y, duals[1].clone());
        let rx = Dual::new(self.r.x, duals[2].clone());
        let ry = Dual::new(self.r.y, duals[3].clone());
        let c = R2 { x: cx, y: cy };
        let r = R2 { x: rx, y: ry };
        XYRR::from((self.idx, c, r))
    }
}

impl<D: RotateArg> XYRR<D> {
    pub fn rotate(&self, t: &D) -> XYRRT<D> {
        XYRRT {
            c: self.c.clone().rotate(t),
            r: self.r.clone(),
            t: t.clone(),
        }
    }
}

impl<D: Display + UnitIntersectionsArg> XYRR<D>
where
    f64: Mul<D, Output = D> + Div<D, Output = D>,
{
    pub fn cdef(&self) -> CDEF<D> {
        let rx2 = self.r.x.clone() * self.r.x.clone();
        let rr = self.r.x.clone() / self.r.y.clone();
        let rr2 = rr.clone() * rr.clone();
        CDEF {
            c: rr2.clone(),
            d: -2. * self.c.x.clone(),
            e: -2. * self.c.y.clone() * rr2.clone(),
            f: self.c.x.clone() * self.c.x.clone() + self.c.y.clone() * self.c.y.clone() * rr2.clone() - rx2.clone(),
        }
    }
}
impl XYRR<D> {
    pub fn n(&self) -> usize {
        self.c.x.d().len()
    }
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        self.cdef().unit_intersections()
    }
}

#[cfg(test)]
mod tests {
    use crate::dual::Dual;

    use super::*;

    #[test]
    fn test_unit_intersections() {
        let e = XYRR {
            c: R2 { x: Dual::new(1., vec![1.,0.,0.,0.]),
                    y: Dual::new(1., vec![0.,1.,0.,0.]), },
            r: R2 { x: Dual::new(2., vec![0.,0.,1.,0.]),
                    y: Dual::new(3., vec![0.,0.,0.,1.]), },
        };
        let us = e.unit_intersections();
        for p in us {
            println!("{}", p);
        }
    }
}
