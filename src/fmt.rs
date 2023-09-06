use crate::dual::Dual;

pub trait Fmt {
    fn s(&self, n: usize) -> String;
}

impl Fmt for Dual {
    fn s(&self, n: usize) -> String {
        format!("{}, vec![{}]", Dual::fmt(&self.v(), n), self.d().iter().map(|d| Dual::fmt(d, n)).collect::<Vec<String>>().join(", "))
    }
}