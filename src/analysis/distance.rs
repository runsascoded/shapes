use std::ops::{Sub, Add, Mul};

use crate::{r2::R2, sqrt::Sqrt, shape::Shape};

pub trait Distance<O> {
    type Output;
    fn distance(&self, o: &O) -> Self::Output;
}

pub trait DistanceArg: Clone + Add<Output = Self> + Mul<Output = Self> + Sqrt {}
impl<D: Clone + Add<Output = D> + Mul<Output = D> + Sqrt> DistanceArg for D {}

impl<D: DistanceArg> Distance<R2<D>> for R2<D>
where R2<D>: Sub<Output = R2<D>>
{
    type Output = D;
    fn distance(&self, o: &R2<D>) -> D {
        (self.clone() - o.clone()).r()
    }
}

impl<D: DistanceArg> Distance<Shape<D>> for Shape<D>
where R2<D>: Sub<Output = R2<D>>
{
    type Output = D;
    fn distance(&self, o: &Shape<D>) -> D {
        self.center().distance(&o.center())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r2_distance_origin_to_point() {
        let origin = R2 { x: 0., y: 0. };
        let p = R2 { x: 3., y: 4. };
        let d = origin.distance(&p);
        assert!((d - 5.).abs() < 1e-10); // 3-4-5 triangle
    }

    #[test]
    fn r2_distance_same_point() {
        let p = R2 { x: 3., y: 4. };
        let d = p.distance(&p);
        assert!(d.abs() < 1e-10);
    }

    #[test]
    fn r2_distance_horizontal() {
        let p1 = R2 { x: 1., y: 0. };
        let p2 = R2 { x: 5., y: 0. };
        let d = p1.distance(&p2);
        assert!((d - 4.).abs() < 1e-10);
    }

    #[test]
    fn r2_distance_vertical() {
        let p1 = R2 { x: 0., y: 1. };
        let p2 = R2 { x: 0., y: 7. };
        let d = p1.distance(&p2);
        assert!((d - 6.).abs() < 1e-10);
    }

    #[test]
    fn r2_distance_symmetry() {
        let p1 = R2 { x: 1., y: 2. };
        let p2 = R2 { x: 4., y: 6. };
        let d1 = p1.distance(&p2);
        let d2 = p2.distance(&p1);
        assert!((d1 - d2).abs() < 1e-10);
    }

    #[test]
    fn r2_distance_negative_coords() {
        let p1 = R2 { x: -3., y: -4. };
        let p2 = R2 { x: 0., y: 0. };
        let d = p1.distance(&p2);
        assert!((d - 5.).abs() < 1e-10);
    }
}
