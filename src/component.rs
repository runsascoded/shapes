use std::{collections::{BTreeSet, BTreeMap}, cell::RefCell, rc::Rc, f64::consts::TAU, fmt};

use log::{debug, error};

use crate::{node::{N, Node}, math::deg::Deg, edge::{E, self, EdgeArg, Edge}, contains::Contains, region::Region, segment::Segment, shape::S, theta_points::{ThetaPoints, ThetaPointsArg}, r2::R2, to::To, zero::Zero};

#[derive(Clone, Debug)]
pub struct Component<D> {
    pub shape_idxs: Vec<usize>,
    pub shapes: Vec<S<D>>,
    pub nodes: Vec<N<D>>,
    pub edges: Vec<E<D>>,
    pub container_idxs: BTreeSet<usize>,
    pub regions: Vec<Region<D>>,
    pub hull: Region<D>,
}

impl<D: Deg + EdgeArg + fmt::Debug + ThetaPointsArg + Zero> Component<D>
where R2<D>: To<R2<f64>>,
{
    pub fn new(
        shape_idxs: Vec<usize>,
        shape_containers: &Vec<BTreeSet<usize>>,
        nodes_by_shape: &Vec<Vec<N<D>>>,
        duals: &Vec<S<D>>,
        num_shapes: usize,
    ) -> Component<D> {
        let mut shape_idxs_iter = shape_idxs.iter().map(|i| shape_containers[*i].clone());
        let mut component_container_idxs = shape_idxs_iter.next().unwrap();
        for containers in shape_idxs_iter {
            if component_container_idxs.is_empty() {
                break;
            }
            component_container_idxs = component_container_idxs.intersection(&containers).cloned().collect();
        }

        if shape_idxs.len() == 1 {
            let shape_idx = shape_idxs[0];
            let shape = &duals[shape_idx];
            let zero = shape.borrow().zero();
            let tau = zero.clone() + TAU;
            let p = shape.borrow().point(zero.clone());
            let mut node = Node {
                idx: 0,
                p,
                intersections: Vec::new(),
                shape_thetas: BTreeMap::new(),
                edges: Vec::new(),
            };
            let n = Rc::new(RefCell::new(node.clone()));
            let edge = Edge {
                idx: 0,
                c: shape.clone(),
                n0: n.clone(),
                n1: n.clone(),
                t0: zero,
                t1: tau.clone(),
                container_idxs: component_container_idxs.clone(),
                is_component_boundary: false,
                visits: 0,
            };
            let e = Rc::new(RefCell::new(edge));
            let region = Region {
                key: String::new(),
                segments: vec![ Segment { edge: e.clone(), fwd: true } ],
                container_idxs: component_container_idxs.clone(),
            };
            node.add_edge(e.clone());
            let component = Component {
                shape_idxs: vec![shape_idx],
                shapes: duals.clone(),
                nodes: vec![n.clone()],
                edges: vec![e.clone()],
                container_idxs: component_container_idxs.clone(),
                regions: vec![ region ],
                hull: Region {
                    key: String::new(),
                    segments: vec![ Segment { edge: e, fwd: true } ],
                    container_idxs: BTreeSet::new(),
                },
            };
            return component;
        }

        let nodes: Vec<N<D>> = shape_idxs.iter().map(|i| nodes_by_shape[*i].clone()).flatten().collect();
        debug!("Connected component: {}", shape_idxs.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", "));
        let edges = Component::edges(&shape_idxs, &nodes_by_shape, &duals, &component_container_idxs);
        let total_expected_visits = edges.iter().map(|e| e.borrow().expected_visits()).sum::<usize>();
        let regions = Component::regions(&edges, num_shapes, total_expected_visits, &component_container_idxs);
        // let total_visits = edges.iter().map(|e| e.borrow().visits).sum::<usize>();

        let first_hull_edge = edges.iter().find(|e| e.borrow().is_component_boundary).unwrap();
        let first_hull_segment = Segment { edge: first_hull_edge.clone(), fwd: true };
        let mut hull_segments: Vec<Segment<D>> = vec![ first_hull_segment.clone() ];
        let start_idx = first_hull_segment.start().borrow().idx;
        loop {
            let last_hull_segment = hull_segments.last().unwrap();
            let end_idx = last_hull_segment.end().borrow().idx;
            if start_idx == end_idx {
                break;
            }
            let successors = last_hull_segment.successors().into_iter().filter(|s| s.edge.borrow().is_component_boundary).collect::<Vec<_>>();
            if successors.len() != 1 {
                panic!("Expected 1 boundary successor for hull segment {}, found {}: {:?}", last_hull_segment, successors.len(), successors);
            }
            let successor = successors[0].clone();
            hull_segments.push(successor);
        }
        let hull = Region {
            key: String::new(),
            segments: hull_segments,
            container_idxs: component_container_idxs.iter().cloned().collect(),
        };

        Component {
            shape_idxs: shape_idxs.clone(),
            shapes: duals.clone(),
            nodes,
            edges,
            container_idxs: component_container_idxs,
            regions,
            hull,
        }
    }
    pub fn edges(
        shape_idxs: &Vec<usize>,
        nodes_by_shape: &Vec<Vec<N<D>>>,
        duals: &Vec<S<D>>,
        component_container_idxs: &BTreeSet<usize>,
    ) -> Vec<E<D>> {
        // Construct edges
        let mut edges: Vec<E<D>> = Vec::new();
        for shape_idx in shape_idxs {
            let shape_idx = *shape_idx;
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
                let mut container_idxs: BTreeSet<usize> = component_container_idxs.clone();
                for cdx in shape_idxs {
                    let cdx = *cdx;
                    if cdx == shape_idx {
                        continue;
                    }
                    let container = duals[cdx].clone();
                    let contained = container.borrow().contains(&arc_midpoint);
                    if contained {
                        // Shape cdx contains this edge
                        container_idxs.insert(cdx);
                        is_component_boundary = false;
                    }
                }
                let expected_visits = if is_component_boundary { 1 } else { 2 };
                let edge = Rc::new(RefCell::new(edge::Edge {
                    idx: edges.len(),
                    c: c.clone(),
                    n0: cur_node, n1: nxt_node,
                    t0: cur_theta, t1: nxt_theta,
                    container_idxs,
                    is_component_boundary,
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

        edges
    }

    pub fn regions(
        edges: &Vec<E<D>>,
        num_shapes: usize,
        total_expected_visits: usize,
        component_container_idxs: &BTreeSet<usize>,
    ) -> Vec<Region<D>> {
        // Graph-traversal will accumulate Regions here
        let mut regions: Vec<Region<D>> = Vec::new();
        // Working list o Segments comprising partial Regions, as they are built up and verified by `traverse`
        let mut segments: Vec<Segment<D>> = Vec::new();
        let mut visited_nodes: BTreeSet<usize> = BTreeSet::new();
        // The first two Segments for each Region uniquely determine various properties of the Region, so we loop over and construct them explicitly below, before kicking off a recursive `traverse` process to complete each Region.
        for edge in edges.clone() {
            if edge.borrow().visits == edge.borrow().expected_visits() {
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
                let container_idxs = container_idxs.intersection(&nxt_idxs).cloned().collect::<BTreeSet<usize>>();
                let component_containers = container_idxs.difference(&component_container_idxs).cloned().collect::<BTreeSet<usize>>();
                if !component_containers.is_empty() {
                    // Recursively traverse the graph, trying to add each eligible Segment to the list we've seeded here, accumulating valid Regions in `regions` along the way.
                    Component::traverse(&start, num_shapes, &mut regions, &mut segments, &container_idxs, &mut visited_nodes, edges.len());
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

        regions
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
                Component::traverse(&start, num_shapes, regions, segments, container_idxs, visited_nodes, max_edges);
                segments.pop();
                visited_nodes.remove(&next_node_idx);
            }
        }
    }
}