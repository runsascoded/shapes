use std::iter::repeat;

use num_traits;

use crate::dual::Dual;

pub trait Zero {
    fn zero(&self) -> Self;
}

impl<D: num_traits::Zero> Zero for D {
    fn zero(&self) -> D {
        D::zero()
    }
}

impl Zero for Dual {
    fn zero(&self) -> Dual {
        Dual::new(0., repeat(0.).take(self.d().len()).collect())
    }
}
