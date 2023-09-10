use std::fmt::Display;

use serde::{Deserialize, Serialize};
use tsify::Tsify;
use crate::{
    math::deg::Deg,
    dual::D,
    r2::R2, fmt::Fmt,
};

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Intersection<D> {
    pub x: D,
    pub y: D,
    pub c0idx: usize,
    pub c1idx: usize,
    pub t0: D,
    pub t1: D,
}

impl Intersection<D> {
    pub fn v(&self) -> Intersection<f64> {
        Intersection {
            x: self.x.v(),
            y: self.y.v(),
            c0idx: self.c0idx,
            c1idx: self.c1idx,
            t0: self.t0.v(),
            t1: self.t1.v(),
        }
    }
}

impl<D: Clone> Intersection<D> {
    pub fn p(&self) -> R2<D> {
        R2 { x: self.x.clone(), y: self.y.clone() }
    }
    pub fn other(&self, cidx: usize) -> usize {
        if cidx == self.c0idx {
            self.c1idx
        } else if cidx == self.c1idx {
            self.c0idx
        } else {
            panic!("Invalid circle index {} ({}, {})", cidx, self.c0idx, self.c1idx);
        }
    }
}

impl<D: Deg + Display + Fmt> Display for Intersection<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I({}, {}, C{}({})/C{}({}))", self.x.s(3), self.y.s(3), self.c0idx, self.t0.deg_str(), self.c1idx, self.t1.deg_str())
    }
}

