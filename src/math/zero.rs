
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
        Dual::new(0., std::iter::repeat_n(0., self.d().len()).collect())
    }
}
