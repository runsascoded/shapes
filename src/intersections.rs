use core::f64;
use std::{cell::RefCell, rc::Rc, f64::consts::TAU, collections::BTreeSet, ops::{Neg, Add, Sub, Mul, Div}, fmt::Display};

use log::{error, debug};
use ordered_float::OrderedFloat;

use crate::{node::{N, Node}, edge::{self, E}, distance::Distance, region::{Region, RegionArg}, shape::{S, Shape}, segment::Segment, theta_points::ThetaPoints, intersect::{Intersect, IntersectShapesArg}, r2::R2, transform::CanTransform, intersection::Intersection, dual::Dual, to::To, math::deg::Deg, fmt::Fmt};

#[derive(Clone, Debug)]
pub struct Intersections<D> {
    pub shapes: Vec<Shape<D>>,
    pub nodes: Vec<N<D>>,
    // pub nodes_by_shape: Vec<Vec<N<D>>>,
    // pub nodes_by_shapes: Vec<Vec<Vec<N<D>>>>,
    pub edges: Vec<E<D>>,
    pub is_connected: Vec<Vec<bool>>,
    pub regions: Vec<Region<D>>,
    pub total_visits: usize,
    pub total_expected_visits: usize,
}

pub trait IntersectionsD
: IntersectShapesArg
+ Add<Output = Self>
+ Mul<f64, Output = Self>
+ Div<f64, Output = Self>
+ Deg
+ Fmt
+ RegionArg
{}
impl IntersectionsD for f64 {}
impl IntersectionsD for Dual {}
pub trait IntersectionsR2<D>
: Neg<Output = Self>
// TODO: can't get this to derive for R2<Dual>
// + Mul<f64, Output = Self>
// + Div<f64, Output = Self>
+ CanTransform<D, Output = Self>
+ To<R2<f64>>
{}
impl IntersectionsR2<f64> for R2<f64> {}
impl IntersectionsR2<Dual> for R2<Dual> {}
pub trait IntersectionsFloat<D>
: Add<D, Output = D>
+ Sub<D, Output = D>
+ Mul<D, Output = D>
+ Div<D, Output = D>
{}
impl IntersectionsFloat<f64> for f64 {}
impl IntersectionsFloat<Dual> for f64 {}

impl<D: IntersectionsD> Intersections<D>
where
    R2<D>: IntersectionsR2<D>,
    Shape<D>: CanTransform<D, Output = Shape<D>>,
    Intersection<D>: Display,
    f64: IntersectionsFloat<D>,
{
    pub fn new(shapes: Vec<Shape<D>>) -> Intersections<D> {
        let num_shapes = (&shapes).len();
        let duals: Vec<S<D>> = shapes.clone().into_iter().map(|s| Rc::new(RefCell::new(s))).collect();
        let mut nodes: Vec<N<D>> = Vec::new();
        let merge_threshold = 1e-7;

        // Intersect all shapes, pair-wise
        for (idx, dual) in duals.iter().enumerate() {
            for jdx in (idx + 1)..num_shapes {
                let intersections = dual.borrow().intersect(&duals[jdx].borrow());
                for i in intersections {
                    let mut merged = false;
                    for node in &nodes {
                        let d = node.borrow().p.distance(&i.p()).unwrap();
                        if d.into() < merge_threshold {
                            // This intersection is close enough to an existing node; merge them
                            let mut node = node.borrow_mut();
                            node.merge(i.clone());
                            debug!("Merged: {} into {}", i, node);
                            merged = true;
                            break;
                        }
                    }
                    if merged {
                        continue;
                    }
                    let node = Node {
                        idx: nodes.len(),
                        p: i.p(),
                        intersections: vec![i.clone()],
                        shape_thetas: i.thetas(),
                        edges: Vec::new(),
                    };
                    let n = Rc::new(RefCell::new(node));
                    nodes.push(n.clone());
                }
            }
        }

        debug!("{} nodes", (&nodes).len());
        for (idx, node) in nodes.iter().enumerate() {
            debug!("  Node {}: {}", idx, node.borrow());
        }
        debug!("");

        let mut nodes_by_shape: Vec<Vec<N<D>>> = Vec::new();
        for _idx in 0..num_shapes {
            nodes_by_shape.push(Vec::new());
        }
        // Compute connected components (shape -> shape -> bool)
        let mut is_connected: Vec<Vec<bool>> = Vec::new();
        for idx in 0..num_shapes {
            let mut connected: Vec<bool> = Vec::new();
            for jdx in 0..num_shapes {
                connected.push(idx == jdx);
            }
            is_connected.push(connected);
        }
        for node in &nodes {
            node.borrow().shape_thetas.keys().for_each(|i0| {
                nodes_by_shape[*i0].push(node.clone());
                node.borrow().shape_thetas.keys().for_each(|i1| {
                    is_connected[*i0][*i1] = true;
                    is_connected[*i1][*i0] = true;
                    if !is_connected[*i0][*i1] {
                        for i2 in 0..num_shapes {
                            if is_connected[*i0][i2] && !is_connected[*i1][i2] {
                                for i3 in 0..num_shapes {
                                    if is_connected[*i1][i3] && !is_connected[i2][i3] {
                                        is_connected[i2][i3] = true;
                                        is_connected[i3][i2] = true;
                                    }
                                };
                            }
                        };
                    }
                })
            });
        }
        // Sort each circle's nodes in order of where they appear on the circle (from -PI to PI)
        for (idx, nodes) in nodes_by_shape.iter_mut().enumerate() {
            nodes.sort_by_cached_key(|n| OrderedFloat(n.borrow().theta(idx).into()))
        }

        // Construct edges
        let mut edges: Vec<E<D>> = Vec::new();
        let mut total_expected_visits = 0;
        for shape_idx in 0..num_shapes {
            let nodes = &nodes_by_shape[shape_idx];
            debug!("{} nodes for shape {}: {}", nodes.len(), shape_idx, nodes.iter().map(|n| n.borrow().theta(shape_idx).deg_str()).collect::<Vec<String>>().join(", "));
            let num_shape_nodes = nodes.len();
            let c = duals[shape_idx].clone();
            for node_idx in 0..num_shape_nodes {
                let cur_node = nodes[node_idx].clone();
                let nxt_node = nodes[(node_idx + 1) % num_shape_nodes].clone();
                let cur_theta = cur_node.borrow().theta(shape_idx);
                let nxt_theta = nxt_node.borrow().theta(shape_idx);
                let nxt_theta = if &nxt_theta < &cur_theta { nxt_theta + TAU } else { nxt_theta };
                let arc_midpoint = duals[shape_idx].borrow().arc_midpoint(cur_theta.clone(), nxt_theta.clone());
                let mut is_component_boundary = true;
                let mut containers: Vec<S<D>> = Vec::new();
                let mut containments: Vec<bool> = Vec::new();
                for cdx in 0..num_shapes {
                    if cdx == shape_idx {
                        continue;
                    }
                    let container = duals[cdx].clone();
                    let contained = container.borrow().contains(&arc_midpoint);
                    if contained {
                        // Shape cdx contains this edge
                        containers.push(container.clone());
                        if is_connected[shape_idx][cdx] {
                            is_component_boundary = false;
                        }
                    }
                    containments.push(contained);
                }
                let expected_visits = if is_component_boundary { 1 } else { 2 };
                total_expected_visits += expected_visits;
                let edge = Rc::new(RefCell::new(edge::Edge {
                    idx: edges.len(),
                    c: c.clone(),
                    n0: cur_node, n1: nxt_node,
                    t0: cur_theta, t1: nxt_theta,
                    containers,
                    containments,
                    expected_visits,
                    visits: 0,
                }));
                edges.push(edge.clone());
                edge.borrow_mut().n0.borrow_mut().add_edge(edge.clone());
                edge.borrow_mut().n1.borrow_mut().add_edge(edge.clone());
            }
        }

        debug!("{} edges", (&edges).len());
        for edge in &edges {
            debug!("  {}", edge.borrow());
        }
        debug!("");

        // Graph-traversal will accumulate Regions here
        let mut regions: Vec<Region<D>> = Vec::new();
        // Working list o Segments comprising partial Regions, as they are built up and verified by `traverse`
        let mut segments: Vec<Segment<D>> = Vec::new();
        let mut visited_nodes: BTreeSet<usize> = BTreeSet::new();
        // The first two Segments for each Region uniquely determine various properties of the Region, so we loop over and construct them explicitly below, before kicking off a recursive `traverse` process to complete each Region.
        for edge in edges.clone() {
            if edge.borrow().visits == edge.borrow().expected_visits {
                // All of the Regions we expect this Edge to be a part of have already been computed and saved, nothing further to do with this Edge.
                continue;
            }
            let segment = Segment { edge: edge.clone(), fwd: true };  // Each Region's first edge can be traversed in the forward direction, WLOG
            let start = &segment.start();
            let end = &segment.end();
            let segment_end_idx = end.borrow().idx;
            let successors = segment.successors();
            segments.push(segment);
            visited_nodes.insert(segment_end_idx);
            let container_idxs = &mut edge.borrow().all_idxs().clone();  // Shape indices that contain the first Edge, will be intersected with the second Edge below to obtain the set of shapes for the Region under construction.
            for successor in successors {
                let segment_end_idx = successor.end().borrow().idx;
                visited_nodes.insert(segment_end_idx);
                segments.push(successor.clone());
                let nxt = successor.edge.clone();
                let nxt_idxs = nxt.borrow().all_idxs();
                let both = container_idxs.intersection(&nxt_idxs).cloned().collect::<BTreeSet<usize>>();
                if !both.is_empty() {
                    // Recursively traverse the graph, trying to add each eligible Segment to the list we've seeded here, accumulating valid Regions in `regions` along the way.
                    Intersections::traverse(&start, num_shapes, &mut regions, &mut segments, &both, &mut visited_nodes, edges.len());
                    assert_eq!(segments.len(), 2);
                }
                segments.pop();
                visited_nodes.remove(&segment_end_idx);
            }
            segments.pop();
            visited_nodes.remove(&segment_end_idx);
        }

        debug!("{} regions", (&regions).len());
        for region in &regions {
            debug!("  {}", region);
        }
        debug!("");

        // Verify that all Edges have been visited the expected number of times
        let total_visits = edges.iter().map(|e| e.borrow().visits).sum::<usize>();
        if total_visits != total_expected_visits {
            error!("total_visits ({}) != total_expected_visits ({})", total_visits, total_expected_visits);
        }

        Intersections { shapes, nodes, edges, is_connected, regions, total_visits, total_expected_visits, }
    }

    pub fn area(&self, key: &String) -> Option<D> {
        match key.split_once('*') {
            Some((prefix, suffix)) => {
                let k0 = format!("{}-{}", prefix, suffix);
                let k1 = format!("{}{}{}", prefix, prefix.len(), suffix);
                if k0.chars().all(|ch| ch == '-') {
                    return self.area(&k1)
                } else {
                    let a0 = self.area(&k0).unwrap_or_else(|| self.zero());
                    let a1 = self.area(&k1).unwrap_or_else(|| self.zero());
                    Some(a0 + a1)
                }
            }
            None => {
                let regions = &self.regions;
                let areas =
                regions
                .iter()
                .filter(|r| &r.key == key)
                .map(|r| r.area());
                let mut sum = None;
                for area in areas {
                    match sum {
                        None => sum = Some(area),
                        Some(s) => sum = Some(s + area),
                    }
                }
                sum
            }
        }
    }

    /// Recursively traverse the graph, accumulating valid Regions in `regions` along the way.
    fn traverse(
        start: &N<D>,
        num_shapes: usize,
        regions: &mut Vec<Region<D>>,
        segments: &mut Vec<Segment<D>>,
        container_idxs: &BTreeSet<usize>,
        visited_nodes: &mut BTreeSet<usize>,
        max_edges: usize,
    ) {
        debug!("traverse, segments:");
        for segment in segments.clone() {
            debug!("  {}", segment);
        }
        debug!("");
        if segments.len() > max_edges {
            panic!("segments.len() ({}) > edges.len() ({})", segments.len(), max_edges);
        }
        let last_segment = segments.last().unwrap();
        let end = last_segment.end();
        let indent = String::from_utf8(vec![b' '; 4 * (segments.len() - 2)]).unwrap();
        let idxs_str = segments.iter().fold(start.borrow().idx.to_string(), |acc, s| {
            format!("{}→{}", acc, s.end().borrow().idx)
        });
        let containers_str = container_idxs.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(" ");
        debug!(
            "{}traverse: {}, {} segments, containers: [{}], {} regions",
            indent,
            idxs_str,
            segments.len(),
            containers_str,
            regions.len(),
        );
        if start.borrow().idx == end.borrow().idx {
            // Back where we started; check whether this is a valid region, push it if so, and return
            let first_segment = segments.first().unwrap();
            let first_edge = first_segment.edge.clone();
            let cidx0 = first_edge.borrow().c.borrow().idx();
            let cidx_end = last_segment.edge.borrow().c.borrow().idx();
            if cidx0 == cidx_end {
                // Can't start and end on same shape. Adjacent segments are checked for this as each segment is pushed, but "closing the loop" requires this extra check of the first and last segments.
                return
            } else {
                // We've found a valid Region; increment each Edge's visit count, and save the Region
                for segment in segments.clone() {
                    let mut edge = segment.edge.borrow_mut();
                    edge.visits += 1;
                }
                let mut container_bmp: Vec<bool> = vec![false; num_shapes];
                for idx in container_idxs.iter() {
                    container_bmp[*idx] = true;
                }
                let mut key = String::new();
                for (idx, b) in container_bmp.iter().enumerate() {
                    if *b {
                        key += &idx.to_string();
                    } else {
                        key += "-";
                    }
                }
                let region = Region {
                    key,
                    segments: segments.clone(),
                    container_idxs: container_idxs.iter().cloned().collect(),
                    container_bmp,
                    };
                regions.push(region);
            }
        } else {
            // Attempt to add another Segment to this Region (from among eligible successors of the last Segment)
            let successors = last_segment.successors();
            // for successor in successors.clone() {
            //     println!("{}  {}", indent, successor);
            // }
            for successor in successors {
                let next_node = successor.end();
                let next_node_idx = next_node.borrow().idx;
                if visited_nodes.contains(&next_node_idx) {
                    // This Segment would revisit a Node that's already been visited (and isn't the start Node, which we're allowed to revisit, to complete a Region)
                    continue;
                }
                // The new Segment should be contained by (or run along the border of) the same shapes as the previous segments, with one exception: the new Segment can run along the border of a shape that doesn't contain the in-progress Region.
                let nxt = successor.edge.clone();
                let nxt_idxs = nxt.borrow().all_idxs();
                // First, verify existing containers are preserved by the new Segment:
                let missing: BTreeSet<usize> = container_idxs.difference(&nxt_idxs).cloned().collect();
                if !missing.is_empty() {
                // let mut both = container_idxs.intersection(&nxt_idxs).cloned().collect::<BTreeSet<usize>>();
                // if both.len() < container_idxs.len() {
                    // This edge candidate isn't contained by (or on the border of) all the shapes that the previous segments are.
                    continue;
                }
                let extra: BTreeSet<usize> = nxt_idxs.difference(&container_idxs).cloned().collect();
                // Next, verify that the only additional container, if any, is the Segment's border shape:
                let num_extra = extra.len();
                if num_extra > 1 {
                    // This Segment can't join a Region with the existing Segments, as it is contained by at least one shape that doesn't contain the existing edges.
                    continue;
                } else if num_extra == 1 {
                    // let extra = nxt_idxs.difference(&container_idxs).cloned().collect::<BTreeSet<usize>>();
                    let extra_idx = extra.iter().next().unwrap();
                    let nxt_edge_idx = successor.edge.borrow().c.borrow().idx();
                    if nxt_edge_idx != *extra_idx {
                        // The only admissible extra containing shape is the one the new edge traverses
                        continue;
                    } else {
                        // OK to proceed with this edge; it is contained by all the shapes that the previous segments are (and outer-borders one additional shape that's not included in the bounded region)
                    }
                }
                visited_nodes.insert(next_node_idx);
                segments.push(successor.clone());
                Intersections::traverse(&start, num_shapes, regions, segments, container_idxs, visited_nodes, max_edges);
                segments.pop();
                visited_nodes.remove(&next_node_idx);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    // pub fn num_vars(&self) -> usize {
    //     self.shapes[0].borrow().n()
    // }

    pub fn zero(&self) -> D {
        self.shapes[0].zero()
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::BTreeMap;
    use std::env;

    use log::debug;

    use crate::circle::Circle;
    use crate::math::{deg::Deg, round::round};
    use crate::dual::Dual;
    use crate::ellipses::xyrr::XYRR;
    use crate::fmt::Fmt;
    use crate::r2::R2;

    use super::*;
    use test_log::test;

    fn duals(idx: usize, n: usize) -> Vec<Vec<f64>> {
        let mut duals = [ vec![ 0.; 3 * n ], vec![ 0.; 3 * n ], vec![ 0.; 3 * n ] ];
        duals[0][3 * idx + 0] = 1.;
        duals[1][3 * idx + 1] = 1.;
        duals[2][3 * idx + 2] = 1.;
        duals.into()
    }

    #[test]
    fn test_00_10_01() {
        let c0 = Shape::Circle(Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. });
        let c1 = Shape::Circle(Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. });
        let c2 = Shape::Circle(Circle { idx: 2, c: R2 { x: 0., y: 1. }, r: 1. });
        let inputs = vec![
            (c0, duals(0, 3)),
            (c1, duals(1, 3)),
            (c2, duals(2, 3)),
        ];
        let shapes: Vec<_> = inputs.iter().map(|(c, duals)| c.dual(duals)).collect();
        let intersections = Intersections::new(shapes);

        // for node in shapes.nodes.iter() {
        //     println!("{}", node.borrow());
        // }

        let check = |idx: usize, x: Dual, y: Dual, c0idx: usize, deg0v: i64, deg0d: [i64; 9], c1idx: usize, deg1v: i64, deg1d: [i64; 9]| {
            let n = intersections.nodes[idx].borrow();
            assert_relative_eq!(n.p.x, x, epsilon = 1e-3);
            assert_relative_eq!(n.p.y, y, epsilon = 1e-3);
            let shape_idxs: Vec<usize> = Vec::from_iter(n.shape_thetas.keys().into_iter().map(|i| *i));
            assert_eq!(
                shape_idxs,
                vec![c0idx, c1idx],
            );
            // assert_eq!(n.i.c0idx, c0idx);
            // assert_eq!(n.i.c1idx, c1idx);
            let thetas: Vec<_> = n.shape_thetas.values().into_iter().map(|t| t.deg()).map(|d| (round(&d.v()), d.d().iter().map(round).collect::<Vec<_>>())).collect();
            assert_eq!(
                thetas,
                vec![
                    (deg0v, deg0d.into()),
                    (deg1v, deg1d.into()),
                ],
            );
            // let d0 = n.i.t0.deg();
            // let a0: (i64, Vec<i64>) = (round(&d0.v()), d0.d().iter().map(round).collect());
            // assert_eq!(a0, (deg0v, deg0d.into()));
            // let d1 = n.i.t1.deg();
            // let a1: (i64, Vec<i64>) = (round(&d1.v()), d1.d().iter().map(round).collect());
            // assert_eq!(a1, (deg1v, deg1d.into()));
        };

        let expected = [
            //    x                                                                     dx         y                                                                    dy c0idx deg0                                       deg0.d c1idx  deg1                                       deg1.d
            ( 0.500, [ 0.500,  0.866,  1.000, 0.500, -0.866, -1.000, 0.000,  0.000,  0.000 ],  0.866, [ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577,  0.000, 0.000,  0.000 ], 0,  60, [  33, -57, -33, -33, 57,  66,   0,   0,   0 ], 1,  120, [ -33, -57, -66,  33, 57,  33,   0,   0,   0 ]),
            ( 0.500, [ 0.500, -0.866,  1.000, 0.500,  0.866, -1.000, 0.000,  0.000,  0.000 ], -0.866, [-0.289, 0.500, -0.577,  0.289, 0.500, -0.577,  0.000, 0.000,  0.000 ], 0, -60, [ -33, -57,  33,  33, 57, -66,   0,   0,   0 ], 1, -120, [  33, -57,  66, -33, 57, -33,   0,   0,   0 ]),
            ( 0.866, [ 0.500,  0.289,  0.577, 0.000,  0.000,  0.000, 0.500, -0.289,  0.577 ],  0.500, [ 0.866, 0.500,  1.000,  0.000, 0.000,  0.000, -0.866, 0.500, -1.000 ], 0,  30, [  57, -33,  33,   0,  0,   0, -57,  33, -66 ], 2,  -30, [  57,  33,  66,   0,  0,   0, -57, -33, -33 ]),
            (-0.866, [ 0.500, -0.289, -0.577, 0.000,  0.000,  0.000, 0.500,  0.289, -0.577 ],  0.500, [-0.866, 0.500,  1.000,  0.000, 0.000,  0.000,  0.866, 0.500, -1.000 ], 0, 150, [  57,  33, -33,   0,  0,   0, -57, -33,  66 ], 2, -150, [  57, -33, -66,   0,  0,   0, -57,  33,  33 ]),
            ( 1.000, [ 0.000,  0.000,  0.000, 0.000,  0.000,  0.000, 1.000,  0.000,  1.000 ],  1.000, [ 0.000, 0.000,  0.000,  0.000, 1.000,  1.000,  0.000, 0.000,  0.000 ], 1,  90, [   0,   0,   0,  57,  0,   0, -57,   0, -57 ], 2,    0, [   0,   0,   0,   0, 57,  57,   0, -57,   0 ]),
            ( 0.000, [ 0.000,  0.000,  0.000, 1.000,  0.000, -1.000, 0.000,  0.000,  0.000 ],  0.000, [ 0.000, 0.000,  0.000,  0.000, 0.000,  0.000,  0.000, 1.000, -1.000 ], 1, 180, [   0,   0,   0,   0, 57,   0,   0, -57,  57 ], 2,  -90, [   0,   0,   0,  57,  0, -57, -57,   0,   0 ]),
        ];
        assert_eq!(intersections.nodes.len(), expected.len());
        for (idx, (x, dx, y, dy, c0idx, deg0v, deg0d, c1idx, deg1v, deg1d)) in expected.iter().enumerate() {
            let x = Dual::new(*x, dx.iter().map(|d| *d as f64).collect());
            let y = Dual::new(*y, dy.iter().map(|d| *d as f64).collect());
            check(idx, x, y, *c0idx, *deg0v, *deg0d, *c1idx, *deg1v, *deg1d);
        }

        assert_eq!(intersections.edges.len(), 12);
        // println!("edges:");
        // for edge in shapes.edges.iter() {
        //     println!("{}", edge.borrow());
        // }
        // println!();

        // let is_connected = intersections.is_connected;
        // println!("is_connected:");
        // for row in is_connected {
        //     for col in row {
        //         print!("{}", if col { "1" } else { "0" });
        //     }
        //     println!();
        // }
        // println!();

        // let segment = Segment { edge: intersections.edges[0].clone(), fwd: true };
        // let successors = segment.successors();
        // println!("successors:");
        // for successor in successors {
        //     println!("  {}", successor);
        // }
        // println!();

        assert_eq!(intersections.regions.len(), 7);
        assert_eq!(intersections.total_expected_visits, 21);
        assert_eq!(intersections.total_visits, 21);
        let expected = [
            "01- 0( -60) 2( -30) 1( 180): 0.500 + 0.285 =  0.785, vec![ 1.366, -0.366,  1.571, -0.866, -0.500,  1.047, -0.500,  0.866, -1.047]",
            "-1- 0( -60) 2( -30) 1(  90): 0.000 + 1.785 =  1.785, vec![-1.366,  0.366, -1.571,  1.866, -0.500,  3.665, -0.500,  0.134, -0.524]",
            "-12 0(  30) 1( 120) 2(   0): 0.116 + 0.012 =  0.128, vec![-0.366, -0.366, -0.524, -0.134,  0.500,  0.524,  0.500, -0.134,  0.524]",
            "012 0(  30) 1( 120) 2( -90): 0.250 + 0.193 =  0.443, vec![ 0.366,  0.366,  0.524, -0.866,  0.500,  1.047,  0.500, -0.866,  1.047]",
            "0-2 0(  60) 2(-150) 1( 180): 0.500 + 0.285 =  0.785, vec![-0.366,  1.366,  1.571,  0.866, -0.500, -1.047, -0.500, -0.866,  1.047]",
            "--2 0(  60) 2(-150) 1(  90): 0.000 + 1.785 =  1.785, vec![ 0.366, -1.366, -1.571,  0.134, -0.500, -0.524, -0.500,  1.866,  3.665]",
            "0-- 0( 150) 1(-120) 2( -90): 0.250 + 0.878 =  1.128, vec![-1.366, -1.366,  2.618,  0.866,  0.500, -1.047,  0.500,  0.866, -1.047]",
        ];
        let actual = intersections.regions.iter().map(|region| {
            let segments = &region.segments;
            let path_str = segments.iter().map(|segment| {
                let start = segment.start();
                let edge = segment.edge.clone();
                let cidx = edge.borrow().c.borrow().idx();
                format!("{}({})", cidx, start.borrow().theta(cidx).v().deg_str())
            }).collect::<Vec<String>>().join(" ");
            format!("{} {}: {:.3} + {:.3} = {}", region.key, path_str, region.polygon_area().v(), region.secant_area().v(), region.area().s(3))
        }).collect::<Vec<String>>();
        // println!("regions:");
        // for a in actual.iter() {
        //     println!("{}", a);
        // }
        // println!();
        actual.iter().zip(expected.iter()).enumerate().for_each(|(idx, (a, b))| assert_eq!(&a, b, "idx: {}, {} != {}", idx, a, b));
    }

    #[test]
    fn test_components() {
        let c0 = Shape::Circle(Circle { idx: 0, c: R2 { x: 0. , y: 0. }, r: 1. });
        let c1 = Shape::Circle(Circle { idx: 1, c: R2 { x: 1. , y: 0. }, r: 1. });
        let c2 = Shape::Circle(Circle { idx: 2, c: R2 { x: 0.5, y: 0. }, r: 3. });
        let c3 = Shape::Circle(Circle { idx: 3, c: R2 { x: 0. , y: 3. }, r: 1. });
        let inputs = vec![
            (c0, duals(0, 4)),
            (c1, duals(1, 4)),
            (c2, duals(2, 4)),
            (c3, duals(3, 4)),
        ];
        let shapes: Vec<_> = inputs.iter().map(|(c, duals)| c.dual(duals)).collect();
        let intersections = Intersections::new(shapes);

        for node in intersections.nodes.iter() {
            println!("{}", node.borrow());
        }
        println!();

        let is_connected = intersections.is_connected;
        println!("is_connected:");
        for row in is_connected {
            for col in row {
                print!("{}", if col { "1" } else { "0" });
            }
            println!();
        }
        println!();
    }

    pub fn ellipses4(r: f64) -> [Shape<f64>; 4] {
        ellipses4_select(r, [ 0, 1, 2, 3 ])
    }

    pub fn ellipses4_select<const N: usize>(r: f64, mask: [ usize; N ]) -> [ Shape<f64>; N ] {
        let r2 = r * r;
        let r2sqrt = (1. + r2).sqrt();
        let c0 = 1. / r2sqrt;
        let c1 = r2 * c0;
        let ellipses = [
            XYRR { idx: 0, c: R2 { x:      c0, y:      c1, }, r: R2 { x: 1., y: r , }, },
            XYRR { idx: 1, c: R2 { x: 1. + c0, y:      c1, }, r: R2 { x: 1., y: r , }, },
            XYRR { idx: 2, c: R2 { x:      c1, y: 1. + c0, }, r: R2 { x: r , y: 1., }, },
            XYRR { idx: 3, c: R2 { x:      c1, y:      c0, }, r: R2 { x: r , y: 1., }, },
        ];
        let mut ellipses = mask.map(|i| ellipses[i].clone());
        for i in 0..N {
            let mut e = ellipses[i].clone();
            e.idx = i;
            ellipses[i] = e;
        }
        ellipses.map(|e| Shape::XYRR(e))
    }

    #[test]
    fn ellipses4_0_2() {
        let shapes = ellipses4_select(2., [0, 2]).to_vec();
        debug!("shapes: {:?}", shapes);
        let intersections = Intersections::new(shapes);
        assert_eq!(intersections.nodes.len(), 2);
        assert_eq!(intersections.edges.len(), 4);
    }

    #[test]
    fn test_4_ellipses() {
        let intersections = Intersections::new(ellipses4(2.).into());
        assert_eq!(intersections.nodes.len(), 14);
        assert_eq!(intersections.edges.len(), 28);
        let mut regions = intersections.regions;
        regions.sort_by_cached_key(|r| OrderedFloat(r.area()));

        let expected: BTreeMap<&str, f64> = [
            ("0-23", 0.1574458339117841),
            ("01-3", 0.15744583391178563),
            ("-1-3", 0.5830467212121343),
            ("0-2-", 0.5830467212121406),
            ("0123", 0.6327112800110606),
            ("--23", 0.6911159327218127),
            ("01--", 0.6911159327218175),
            ("0--3", 0.8415779241718198),
            ("-123", 0.9754663505728485),
            ("012-", 0.9754663505728591),
            ("-12-", 1.0010372353827428),
            ("-1--", 1.2668956027943292),
            ("--2-", 1.2668956027943392),
            ("0---", 2.9962311616537862),
            ("---3", 2.996264719973729),
        ].into();
        assert_eq!(regions.len(), expected.len());
        match env::var("GEN_VALS").map(|s| s.parse::<usize>().unwrap()).ok() {
            Some(_) => {
                let mut actual: BTreeMap<&str, f64> = BTreeMap::new();
                for region in &regions {
                    let area = region.area();
                    let key = region.key.as_str();
                    actual.insert(key, area);
                }
                let mut actual: Vec<_> = actual.into_iter().collect();
                actual.sort_by_cached_key(|(_, a)| OrderedFloat(*a));
                for (key, area) in actual {
                    println!("(\"{}\", {}),", key, area);
                }
            }
            None => {
                for region in &regions {
                    let area = region.area();
                    let key = region.key.as_str();
                    assert_relative_eq!(area, *expected.get(key).unwrap(), max_relative = 1e-10);
                }
            }
        }
    }

    #[test]
    fn test_4_circles_lattice_0_1() {
        let ellipses = [
            XYRR { idx: 0, c: R2 { x: 0., y: 0. }, r: R2 { x: 1., y: 1., }, },
            XYRR { idx: 1, c: R2 { x: 0., y: 1. }, r: R2 { x: 1., y: 1., }, },
            XYRR { idx: 2, c: R2 { x: 1., y: 0. }, r: R2 { x: 1., y: 1., }, },
            XYRR { idx: 3, c: R2 { x: 1., y: 1. }, r: R2 { x: 1., y: 1., }, },
        ];
        let shapes = ellipses.map(Shape::XYRR);
        let intersections = Intersections::new(shapes.to_vec());
        assert_node_strs(
            &intersections,
            vec![
                "N0( 0.866,  0.500: C0(  30), C1( -30))",
                "N1(-0.866,  0.500: C0( 150), C1(-150))",
                "N2( 0.500,  0.866: C0(  60), C2( 120))",
                "N3( 0.500, -0.866: C0( -60), C2(-120))",
                "N4( 0.000,  1.000: C0(  90), C3( 180))",
                "N5( 1.000,  0.000: C0(   0), C3( -90))",
                "N6( 1.000,  1.000: C1(   0), C2(  90))",
                "N7( 0.000,  0.000: C1( -90), C2( 180))",
                "N8( 0.500,  1.866: C1(  60), C3( 120))",
                "N9( 0.500,  0.134: C1( -60), C3(-120))",
                "N10( 1.866,  0.500: C2(  30), C3( -30))",
                "N11( 0.134,  0.500: C2( 150), C3(-150))",
            ]
        );
    }

    #[test]
    fn test_4_circles_diamond() {
        let sq3 = 3_f64.sqrt();
        let ellipses = [
            XYRR { idx: 0, c: R2 { x: 0. , y:  0.       }, r: R2 { x: 1., y: 1., }, },
            XYRR { idx: 1, c: R2 { x: 1. , y:  0.       }, r: R2 { x: 1., y: 1., }, },
            XYRR { idx: 2, c: R2 { x: 0.5, y:  sq3 / 2. }, r: R2 { x: 1., y: 1., }, },
            XYRR { idx: 3, c: R2 { x: 0.5, y: -sq3 / 2. }, r: R2 { x: 1., y: 1., }, },
        ];
        let shapes = ellipses.map(Shape::XYRR);
        let intersections = Intersections::new(shapes.to_vec());
        assert_node_strs(
            &intersections,
            vec![
                "N0( 0.500,  0.866: C0(  60), C1( 120))",
                "N1( 0.500, -0.866: C0( -60), C1(-120))",
                "N2( 1.000,  0.000: C0(   0), C2( -60), C3(  60))",
                "N3(-0.500,  0.866: C0( 120), C2( 180))",
                "N4(-0.500, -0.866: C0(-120), C3(-180))",
                "N5( 1.500,  0.866: C1(  60), C2(   0))",
                "N6(-0.000,  0.000: C1( 180), C2(-120), C3( 120))",
                "N7( 1.500, -0.866: C1( -60), C3(   0))",
            ]
        );
        assert_eq!(intersections.regions.len(), 11);
    }

    fn assert_node_strs<D: Display + Deg + Fmt>(intersections: &Intersections<D>, expected: Vec<&str>) {
        let gen_vals = env::var("GEN_VALS").map(|s| s.parse::<usize>().unwrap()).ok();
        match gen_vals {
            Some(_) => {
                println!("Nodes:");
                for node in &intersections.nodes {
                    println!("  {:?},", format!("{}", node.borrow()));
                }
            },
            None => {
                let actual = intersections.nodes.iter().map(|n| format!("{}", n.borrow())).collect::<Vec<String>>();
                assert_eq!(actual.len(), expected.len());
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn tweaked_2_ellipses_f64() {
        // find_roots_quartic bug?
        // a_4: 0.000000030743755847066437
        // a_3: 0.000000003666731306801131
        // a_2: 1.0001928389119579
        // a_1: 0.000011499702220469921
        // a_0: -0.6976068572771268
        // Roots: Two([-5.106021787341158, 5.106010194296296])
        //   x: -5.106021787341158, f(x): 25.37884091853862 ❌ should be 0!
        //   x: 5.106010194296296, f(x): 25.37884091853862  ❌ should be 0!
        // Actual roots ≈ ±0.835153846196954, per:
        // https://www.wolframalpha.com/input?i=0.000000030743755847066437+x%5E4++%2B0.000000003666731306801131+x%5E3+%2B+1.0001928389119579+x%5E2++%2B0.000011499702220469921+x+-+0.6976068572771268
        let ellipses = [
            // XYRR { idx: 0, c: R2 { x: 0.347, y: 1.789 }, r: R2 { x: 1.000, y: 2.000 } },
            // XYRR { idx: 1, c: R2 { x: 1.447, y: 1.789 }, r: R2 { x: 1.000, y: 2.000 } },
            // XYRR { idx: 2, c: R2 { x: 1.789, y: 1.447 }, r: R2 { x: 2.000, y: 0.999 } },
            XYRR { idx: 0, c: R2 { x: 0.3472135954999579, y: 1.7888543819998317 }, r: R2 { x: 1.0,                y: 2.0                } },
            XYRR { idx: 1, c: R2 { x: 1.4472087032327248, y: 1.7888773809286864 }, r: R2 { x: 0.9997362494738584, y: 1.9998582057729295 } },
            // XYRR { idx: 2, c: R2 { x: 1.7890795512191124, y: 1.4471922162722848 }, r: R2 { x: 1.9998252659224116, y: 0.9994675708661026 } },
        ];
        let shapes: Vec<_> = ellipses.iter().map(|e| Shape::XYRR(e.clone())).collect();
        let intersections = Intersections::new(shapes);
        assert_node_strs(
            &intersections,
            vec![
                "N0( 0.897,  0.119: C0( -57), C1(-123))",
                "N1( 0.897,  3.459: C0(  57), C1( 123))",
            ]
        );
    }

    #[test]
    fn tweaked_3_ellipses_f64() {
        let shapes = vec![
            Shape::XYRR(XYRR { idx: 0, c: R2 { x: 0.3472135954999579, y: 1.7888543819998317 }, r: R2 { x: 1.0,                y: 2.0                } }),
            Shape::XYRR(XYRR { idx: 1, c: R2 { x: 1.4472087032327248, y: 1.7888773809286864 }, r: R2 { x: 0.9997362494738584, y: 1.9998582057729295 } }),
            Shape::XYRR(XYRR { idx: 2, c: R2 { x: 1.7890795512191124, y: 1.4471922162722848 }, r: R2 { x: 1.9998252659224116, y: 0.9994675708661026 } }),
        ];
        let intersections = Intersections::new(shapes);
        assert_node_strs(
            &intersections,
            vec![
                "N0( 0.897,  0.119: C0( -57), C1(-123))",
                "N1( 0.897,  3.459: C0(  57), C1( 123))",
                "N2( 1.297,  2.416: C0(  18), C2( 104))",
                "N3( 1.115,  0.506: C0( -40), C2(-110))",
                "N4( 2.399,  2.399: C1(  18), C2(  72))",
                "N5( 0.469,  2.198: C1( 168), C2( 131))",
                "N6( 0.632,  0.632: C1(-145), C2(-125))",
                "N7( 2.198,  0.469: C1( -41), C2( -78))",
            ]
        );
    }

    #[test]
    fn fizz_bazz_buzz_qux_error() {
        let ellipses = [
            XYRR { idx: 0, c: R2 { x:  -2.0547374714862916, y:  0.7979432881804286   }, r: R2 { x: 15.303664487498873, y: 17.53077114567813  } },
            XYRR { idx: 1, c: R2 { x: -11.526407092112622 , y:  3.0882189920409058   }, r: R2 { x: 22.75383340199038 , y:  5.964648612528639 } },
            XYRR { idx: 2, c: R2 { x:  10.550418544451459 , y:  0.029458342547552023 }, r: R2 { x:  6.102407875525676, y: 11.431493472697646 } },
            XYRR { idx: 3, c: R2 { x:   4.271631577807546 , y: -5.4473446956862155   }, r: R2 { x:  2.652054463066812, y: 10.753963707585315 } },
        ];
        let shapes = ellipses.map(Shape::XYRR);
        let intersections = Intersections::new(shapes.to_vec());
        assert_node_strs(
            &intersections,
            vec![
                "N0(-15.600,  8.956: C0( 152), C1( 100))",
                "N1(-17.051, -2.698: C0(-168), C1(-104))",
                "N2( 10.112,  11.431: C0(  37), C2(  94))",
                "N3( 9.177, -11.109: C0( -43), C2(-103))",
                "N4( 5.795, -14.251: C0( -59), C3( -55))",
                "N5( 3.388, -15.587: C0( -69), C3(-109))",
                "N6( 5.707,  6.983: C1(  41), C2( 143))",
                "N7( 4.481, -1.151: C1( -45), C2(-174))",
                "N8( 6.627, -0.508: C1( -37), C3(  27))",
                "N9( 1.781, -1.750: C1( -54), C3( 160))",
                "N10( 5.022,  4.868: C2( 155), C3(  74))",
                "N11( 6.778, -8.957: C2(-128), C3( -19))",
            ]
        );
        assert_eq!(intersections.regions.len(), 11);
    }
}
