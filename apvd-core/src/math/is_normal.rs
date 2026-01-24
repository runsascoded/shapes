use crate::dual::Dual;

pub trait IsNormal {
    fn is_normal(&self) -> bool;
}

impl IsNormal for f64 {
    fn is_normal(&self) -> bool {
        let f = *self;
        f64::is_normal(f) || f == 0. || f == -0.
    }
}

impl IsNormal for Dual {
    fn is_normal(&self) -> bool {
        IsNormal::is_normal(&self.v())
    }
}
