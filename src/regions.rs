use std::collections::BTreeSet;

use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{dual::{Dual, D}, shape::Shape, r2::R2, component};


#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Point {
    pub p: R2<D>,
    pub edge_idxs: Vec<usize>,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Edge {
    pub cidx: usize,
    pub i0: usize,
    pub i1: usize,
    pub t0: f64,
    pub t1: f64,
    pub container_idxs: BTreeSet<usize>,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Segment {
    pub edge_idx: usize,
    pub fwd: bool,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Region {
    pub key: String,
    pub segments: Vec<Segment>,
    pub area: Dual,
    pub container_idxs: Vec<usize>,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Component {
    pub shapes: Vec<Shape<f64>>,
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
    pub regions: Vec<Region>,
}

impl Component {
    pub fn new(component: &component::Component<D>) -> Self {
        let shapes = component.shapes.clone();
        let points = component.nodes.iter().map(|n| Point {
            p: n.borrow().p.clone(),
            edge_idxs: n.borrow().edges.iter().map(|e| e.borrow().idx).collect(),
        }).collect();
        let edges = component.edges.iter().map(|e| Edge {
            cidx: e.borrow().c.borrow().idx(),
            i0: e.borrow().n0.borrow().idx,
            i1: e.borrow().n1.borrow().idx,
            t0: e.borrow().t0.v(),
            t1: e.borrow().t1.v(),
            container_idxs: e.borrow().container_idxs.clone(),
        }).collect();
        let regions = component.regions.iter().map(|r| Region {
            key: r.key.clone(),
            segments: r.segments.iter().map(|s| Segment {
                edge_idx: s.edge.borrow().idx,
                fwd: s.fwd,
            }).collect(),
            area: r.area(),
            container_idxs: r.container_idxs.clone().into_iter().collect(),
        }).collect();
        Component {
            shapes: shapes.iter().map(|s| s.borrow().v()).collect(),
            points,
            edges,
            regions,
        }
    }
}