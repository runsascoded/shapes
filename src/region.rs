use std::fmt::{Formatter, Display, self};

use crate::{dual::D, segment::Segment};

#[derive(Debug, Clone)]
pub struct Region {
    pub key: String,
    pub segments: Vec<Segment>,
    pub container_idxs: Vec<usize>,
    pub container_bmp: Vec<bool>,
}

impl Region {
    pub fn len(&self) -> usize {
        self.segments.len()
    }
    pub fn polygon_area(&self) -> D {
        (self.segments.iter().map(|s| {
            let cur = s.start().borrow().p();
            let nxt = s.end().borrow().p();
            cur.x * nxt.y - cur.y * nxt.x
        }).sum::<D>() / 2.).abs()
    }
    pub fn secant_area(&self) -> D {
        self.segments.iter().map(|s| {
            let area = s.secant_area();
            let idx = s.edge.borrow().c.borrow().idx();
            if self.container_idxs.contains(&idx) { area } else { -area }
        }).sum::<D>()
    }
    pub fn area(&self) -> D {
        self.polygon_area() + self.secant_area()
    }
    pub fn matches(&self, key: &String) -> bool {
        for (idx, ch) in (&key).chars().enumerate() {
            if ch == '-' && self.container_bmp[idx] {
                return false;
            }
            if ch != '*' && !self.container_bmp[idx] {
                return false
            }
        }
        true
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f, "R({})",
            self.segments.iter().map(|s| {
                format!("{}", s.edge.borrow())
            }).collect::<Vec<String>>().join(", ")
        )
    }
}