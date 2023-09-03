use std::iter::repeat;

use num_traits;

use crate::dual::Dual;

pub trait Zero {
    fn zero(x: &Self) -> Self;
}

impl<D: num_traits::Zero> Zero for D {
    fn zero(x: &D) -> D {
        D::zero()
    }
}

impl Zero for Dual {
    fn zero(x: &Dual) -> Dual {
        Dual::new(0., repeat(0.).take(x.d().len()).collect())
    }
}
