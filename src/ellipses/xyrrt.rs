use std::ops::Neg;

use crate::{r2::R2, rotate::{Rotate, RotateArg}, dual::D};

use super::xyrr::XYRR;

pub struct XYRRT<D> {
    pub c: R2<D>,
    pub rx: D,
    pub ry: D,
    pub t: D,
}

// impl<D> XYRRT<D> {
//     pub fn abcdef(&self) -> ABCDEF<D> {
//         ABCDEF {}
//     }
// }

impl<D: RotateArg + Neg<Output = D>> XYRRT<D> {
    /// Rotate the plane so that this ellipse ends up aligned with the x- and y-axes (i.e. θ == B == 0)
    pub fn level(&self) -> XYRR<D> {
        XYRR {
            c: self.c.clone().rotate(&-self.t.clone()),
            rx: self.rx.clone(),
            ry: self.ry.clone(),
        }
    }
}

// impl<D: RotateArg + Neg<Output = D> + UnitIntersectionsArg> XYRRT<D>
// where
//     f64: Mul<D, Output = D> + Div<D, Output = D>,
impl XYRRT<D>
{
    pub fn unit_intersections(&self) -> Vec<R2<D>> {
        self.level().unit_intersections().iter().map(|p| p.rotate(&self.t)).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    #[test]
    fn test_level() {
        let e = XYRRT {
            c: R2 { x: 1., y: 1. },
            rx: 2.,
            ry: 3.,
            t: PI / 4.,
        };

        let l = e.level();

        assert_relative_eq!(l.c.x, 2_f64.sqrt());
        assert_relative_eq!(l.c.y, 0.);
        assert_relative_eq!(l.rx, 2.);
        assert_relative_eq!(l.ry, 3.);
    }
}