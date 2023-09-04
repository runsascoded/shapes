use std::ops::{Mul, Div, Add, Sub};

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{r2::R2, rotate::{Rotate, RotateArg}, dual::D};

use super::{xyrrt::XYRRT, acdef::ACDEF};


pub trait UnitIntersectionsArg: Clone + Add<Output = Self> + Add<f64, Output = Self> + Sub<Output = Self> + Sub<f64, Output = Self> + Mul<Output = Self> + Div<Output = Self> {}
impl<D: Clone + Add<Output = D> + Add<f64, Output = D> + Sub<Output = D> + Sub<f64, Output = D> + Mul<Output = D> + Div<Output = D>> UnitIntersectionsArg for D {}

#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
pub struct XYRR<D> {
    pub c: R2<D>,
    pub r: R2<D>,
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

impl<D: UnitIntersectionsArg> XYRR<D>
where
    f64: Mul<D, Output = D> + Div<D, Output = D>,
{
    pub fn acdef(&self) -> ACDEF<D> {
        let rxr = 1. / self.r.x.clone();
        let ryr = 1. / self.r.y.clone();
        let r_x = self.c.x.clone() * rxr.clone();
        let r_y = self.c.y.clone() * ryr.clone();
        ACDEF {
            a: rxr.clone() * rxr,
            c: ryr.clone() * ryr,
            d: -2. * r_x.clone(),
            e: -2. * r_y.clone(),
            f: r_x.clone() * r_x + r_y.clone() * r_y - 1.,
        }
    }
}
impl XYRR<D> {
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        self.acdef().unit_intersections()
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
