use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{intersection::Intersection, intersections::Intersections, dual::{Dual, D}, shape::Shape};


#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Point {
    pub i: Intersection<D>,
    pub edge_idxs: Vec<usize>,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Edge {
    // pub idx: usize,
    pub cidx: usize,
    // pub c0: usize,
    // pub c1: usize,
    pub i0: usize,
    pub i1: usize,
    pub t0: f64,
    pub t1: f64,
    pub containers: Vec<usize>,
    pub containments: Vec<bool>,
    // pub expected_visits: usize,
    // pub visits: usize,
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
    pub container_bmp: Vec<bool>,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Regions {
    pub shapes: Vec<Shape<f64>>,
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
    pub regions: Vec<Region>,
}

impl Regions {
    pub fn new(intersections: &Intersections<D>) -> Self {
        let shapes = intersections.shapes.clone();
        let points = intersections.nodes.iter().map(|n| Point {
            i: n.borrow().i.clone(),
            edge_idxs: n.borrow().edges.iter().map(|e| e.borrow().idx).collect(),
        }).collect();
        let edges = intersections.edges.iter().map(|e| Edge {
            cidx: e.borrow().c.borrow().idx(),
            i0: e.borrow().i0.borrow().idx,
            i1: e.borrow().i1.borrow().idx,
            t0: e.borrow().t0.v(),
            t1: e.borrow().t1.v(),
            containers: e.borrow().containers.iter().map(|c| c.borrow().idx()).collect(),
            containments: e.borrow().containments.clone(),
            // expected_visits: e.borrow().expected_visits,
            // visits: e.visits,
        }).collect();
        let regions = intersections.regions.iter().map(|r| Region {
            key: r.key.clone(),
            segments: r.segments.iter().map(|s| Segment {
                edge_idx: s.edge.borrow().idx,
                fwd: s.fwd,
            }).collect(),
            area: r.area(),
            container_idxs: r.container_idxs.clone(),
            container_bmp: r.container_bmp.clone(),
        }).collect();
        Regions {
            shapes: shapes.iter().map(|s| s.v()).collect(),
            points,
            edges,
            regions,
        }
    }
}