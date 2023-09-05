use std::ops::Neg;

use crate::{r2::R2, rotate::{Rotate, RotateArg}, dual::D};

use super::xyrr::XYRR;

pub struct XYRRT<D> {
    pub idx: usize,
    pub c: R2<D>,
    pub r: R2<D>,
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
            idx: self.idx,
            c: self.c.clone().rotate(&-self.t.clone()),
            r: self.r.clone(),
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
            idx: 0,
            c: R2 { x: 1., y: 1. },
            r: R2 { x: 2., y: 3. },
            t: PI / 4.,
        };

        let l = e.level();

        assert_relative_eq!(l.c.x, 2_f64.sqrt());
        assert_relative_eq!(l.c.y, 0.);
        assert_relative_eq!(l.r.x, 2.);
        assert_relative_eq!(l.r.y, 3.);
    }
}