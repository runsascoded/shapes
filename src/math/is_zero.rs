use crate::dual::Dual;

pub trait IsZero {
    fn is_zero(&self) -> bool;
    fn lt_zero(&self) -> bool;
}

impl IsZero for f64 {
    fn is_zero(&self) -> bool {
        let f = *self;
        f == 0. || f == -0.
    }

    fn lt_zero(&self) -> bool {
        let f = *self;
        f < 0.
    }
}

impl IsZero for Dual {
    fn is_zero(&self) -> bool {
        IsZero::is_zero(&self.v())
    }

    fn lt_zero(&self) -> bool {
        IsZero::lt_zero(&self.v())
    }
}