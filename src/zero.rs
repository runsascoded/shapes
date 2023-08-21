use std::iter::repeat;

use num_traits;

use crate::dual::Dual;

pub trait Zero<D> {
    fn zero(x: D) -> D;
}

impl<D: num_traits::Zero> Zero<D> for D {
    fn zero(x: D) -> D {
        D::zero()
    }
}

impl Zero<Dual> for Dual {
    fn zero(x: Dual) -> Dual {
        Dual::new(0., repeat(0.).take(x.d().len()).collect())
    }
}
