use std::{iter::Sum, ops::{Div, Mul, Sub, Add}, fmt::{Formatter, Display, self}};

use crate::{edge::{Edge, E}, intersection::Intersection, dual::Dual, r2::R2};

#[derive(Debug, Clone)]
pub struct Segment {
    pub edge: E,
    pub fwd: bool,
}

impl Segment {
    pub fn sgn(&self) -> f64 {
        if self.fwd { 1. } else { -1. }
    }
    pub fn secant_area(&self) -> D {
        self.edge.borrow().secant_area() * self.sgn()
    }
    pub fn start(&self) -> R2<D> {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.i0 } else { &e.i1 };
        i.clone().borrow().p()
    }
    pub fn end(&self) -> R2<D> {
        let e = self.edge.borrow();
        let i = if self.fwd { &e.i1 } else { &e.i0 };
        i.clone().borrow().p()
    }
    // pub fn sector_area(&self) -> D {
    //     let e = self.edge.borrow();
    //     let r = e.c.borrow().r.clone();
    //     let theta = e.theta();
    //     r * r * theta / 2.
    // }
    // pub fn triangle_area(&self) -> D {
    //     let e = self.edge.borrow();
    //     let r = e.c.borrow().r.clone();
    //     let theta = e.theta();
    //     r * r * theta.sin() / 2.
    // }
}

#[derive(Debug, Clone)]
pub struct Region {
    pub segments: Vec<Segment>,
}

type D = Dual;

impl Region {
    pub fn len(&self) -> usize {
        self.segments.len()
    }
    pub fn polygon_area(&self, intersections: &Vec<Intersection>) -> D {
        self.segments.iter().map(|s| {
            let cur = s.start();
            let nxt = s.end();
            cur.x * nxt.y - cur.y * nxt.x
        }).sum::<D>() / 2.
    }
    pub fn secant_area(&self) -> D {
        self.segments.iter().map(|s| s.secant_area()).sum::<D>()
    }
    pub fn area(&self, intersections: &Vec<Intersection>) -> D {
        self.polygon_area(intersections) + self.secant_area()
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // let mut s: String = "Region(".to_owned();
        write!(
            f, "R({})",
            self.segments.iter().map(|s| {
                format!("{}", s.edge.borrow())
            }).collect::<Vec<String>>().join(", ")
        )
    }
}