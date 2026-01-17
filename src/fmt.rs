use std::fmt::Display;
use crate::{dual::Dual, math::deg::Deg};

pub trait Fmt {
    fn s(&self, n: usize) -> String;
}

/// Trait alias for types that can be formatted with degree notation.
/// Used in Display implementations for geometric types.
pub trait DisplayNum: Deg + Display + Fmt {}
impl DisplayNum for f64 {}
impl DisplayNum for Dual {}

impl Fmt for f64 {
    fn s(&self, n: usize) -> String {
        let rendered = format!("{:.1$}", self, n);
        format!("{}{}", if rendered.starts_with('-') { "" } else { " " }, rendered)
    }
}

impl Fmt for Dual {
    fn s(&self, n: usize) -> String {
        format!("{}, vec![{}]", self.v().s(n), self.d().iter().map(|d| d.s(n)).collect::<Vec<String>>().join(", "))
    }
}