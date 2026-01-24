use std::collections::BTreeSet;

use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{dual::Dual, r2::R2, component, set::Set, region, segment};

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Point {
    pub p: R2<f64>,
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

impl From<&segment::Segment<Dual>> for Segment {
    fn from(s: &segment::Segment<Dual>) -> Self {
        Segment {
            edge_idx: s.edge.borrow().idx,
            fwd: s.fwd,
        }
    }
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Region {
    pub key: String,
    pub segments: Vec<Segment>,
    pub area: f64,
    pub container_set_idxs: Vec<usize>,
    pub child_component_keys: Vec<String>,
}

impl From<&region::Region<Dual>> for Region {
    fn from(region: &region::Region<Dual>) -> Self {
        Region {
            key: region.key.clone(),
            segments: region.segments.iter().map(|s| s.into()).collect(),
            area: region.area().v(),
            container_set_idxs: region.container_set_idxs.clone().into_iter().collect(),
            child_component_keys: region.child_components.iter().map(|c| c.borrow().key.0.clone()).collect(),
        }
    }
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Component {
    pub key: String,
    #[serde(skip)]
    pub sets: Vec<Set<f64>>,
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
    pub regions: Vec<Region>,
    #[serde(skip)]
    pub container_idxs: Vec<usize>,
    pub hull: Vec<Segment>,
}

impl Component {
    pub fn new(component: &component::Component<Dual>) -> Self {
        let sets: Vec<Set<f64>> = component.sets.iter().map(|set| set.borrow().v()).collect();
        let points = component.nodes.iter().map(|n| Point {
            p: n.borrow().p.v(),
            edge_idxs: n.borrow().edges.iter().map(|e| e.borrow().idx).collect(),
        }).collect();
        let edges = component.edges.iter().map(|e| Edge {
            set_idx: e.borrow().set.borrow().idx,
            node0_idx: e.borrow().node0.borrow().idx,
            node1_idx: e.borrow().node1.borrow().idx,
            theta0: e.borrow().coord0,
            theta1: e.borrow().coord1,
            container_idxs: e.borrow().container_set_idxs.clone(),
            is_component_boundary: e.borrow().is_component_boundary,
        }).collect();
        let regions: Vec<Region> = component.regions.iter().map(|r| {
            let region: Region = r.into();
            region
        }).collect();
        let hull: Vec<Segment> = component.hull.0.iter().map(|s| s.into()).collect();
        Component {
            key: component.key.0.clone(),
            sets,
            points,
            edges,
            regions,
            container_idxs: component.container_set_idxs.clone().into_iter().collect(),
            hull,
        }
    }
}