use std::collections::BTreeSet;

use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{dual::{Dual, D}, r2::R2, component, set::Set};


#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Point {
    pub p: R2<D>,
    pub edge_idxs: Vec<usize>,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Edge {
    pub set_idx: usize,
    pub node0_idx: usize,
    pub node1_idx: usize,
    pub theta0: f64,
    pub theta1: f64,
    pub container_idxs: BTreeSet<usize>,
    pub is_component_boundary: bool,
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
    pub sets: Vec<Set<f64>>,
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
    pub regions: Vec<Region>,
}

impl Component {
    pub fn new(component: &component::Component<D>) -> Self {
        let sets: Vec<Set<f64>> = component.sets.iter().map(|set| set.borrow().v()).collect();
        let points = component.nodes.iter().map(|n| Point {
            p: n.borrow().p.clone(),
            edge_idxs: n.borrow().edges.iter().map(|e| e.borrow().idx).collect(),
        }).collect();
        let edges = component.edges.iter().map(|e| Edge {
            set_idx: e.borrow().set.borrow().idx,
            node0_idx: e.borrow().node0.borrow().idx,
            node1_idx: e.borrow().node1.borrow().idx,
            theta0: e.borrow().theta0.v(),
            theta1: e.borrow().theta1.v(),
            container_idxs: e.borrow().container_idxs.clone(),
            is_component_boundary: e.borrow().is_component_boundary,
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
            sets,
            points,
            edges,
            regions,
        }
    }
}