use std::fmt::Display;

use serde::{Deserialize, Serialize};
use tsify::Tsify;
use crate::{
    deg::Deg,
    dual::D,
    r2::R2,
};

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Intersection {
    pub x: D,
    pub y: D,
    pub c0idx: usize,
    pub c1idx: usize,
    pub t0: D,
    pub t1: D,
}

impl Intersection {
    pub fn p(&self) -> R2<D> {
        R2 { x: self.x.clone(), y: self.y.clone() }
    }
    pub fn v(&self) -> R2<f64> {
        R2 { x: self.x.v(), y: self.y.v() }
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

impl Display for Intersection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I({:.3}, {:.3}, C{}({})/C{}({}))", self.x, self.y, self.c0idx, self.t0.deg().s(0), self.c1idx, self.t1.deg().s(0))
    }
}

