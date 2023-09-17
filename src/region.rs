use std::{fmt::{Formatter, Display, self}, ops::Add, iter::Sum};

use crate::{segment::Segment, intersection::Intersection, edge::{Edge, EdgeArg}, math::abs::{Abs, AbsArg}, r2::R2, to::To, dual::Dual};

#[derive(Debug, Clone)]
pub struct Region<D> {
    pub key: String,
    pub segments: Vec<Segment<D>>,
    pub container_idxs: Vec<usize>,
    pub container_bmp: Vec<bool>,
}

pub trait RegionArg
: EdgeArg
+ AbsArg
+ Sum
+ Add<Output = Self>
{}
impl RegionArg for f64 {}
impl RegionArg for Dual {}

impl<D: RegionArg> Region<D>
where
    Intersection<D>: Display,
    R2<D>: To<R2<f64>>,
{
    pub fn len(&self) -> usize {
        self.segments.len()
    }
    pub fn polygon_area(&self) -> D {
        (self.segments.iter().map(|s| {
            let cur = s.start().borrow().p.clone();
            let nxt = s.end().borrow().p.clone();
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

impl<D: Display> Display for Region<D>
where
    Edge<D>: Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f, "R({})",
            self.segments.iter().map(|s| {
                format!("{}", s.edge.borrow())
            }).collect::<Vec<String>>().join(", ")
        )
    }
}