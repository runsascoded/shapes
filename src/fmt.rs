use crate::dual::Dual;

pub trait Fmt {
    fn s(&self, n: usize) -> String;
}

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