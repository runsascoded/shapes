use std::{collections::{BTreeSet, BTreeMap}, cell::RefCell, rc::Rc, f64::consts::TAU, fmt, ops::Sub};
use anyhow::{Result, anyhow};

use log::{debug, error};
use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{node::{N, Node}, math::deg::Deg, edge::{E, self, EdgeArg, Edge}, contains::{Contains, ShapeContainsPoint}, region::{Region, RegionArg, R}, segment::Segment, set::S, theta_points::{ThetaPoints, ThetaPointsArg}, r2::R2, to::To, zero::Zero, fmt::Fmt, hull::Hull, shape::AreaArg};

pub type C<D> = Rc<RefCell<Component<D>>>;

#[derive(Clone, Debug, derive_more::Deref, derive_more::Display, Eq, derive_more::From, Ord, PartialEq, PartialOrd, Serialize, Deserialize, Tsify)]
pub struct Key(pub String);

/// Connected component within a [`Scene`] of [`Set`]s
#[derive(Clone, Debug)]
pub struct Component<D> {
    pub key: Key,
    pub set_idxs: Vec<usize>,
    pub sets: Vec<S<D>>,
    pub nodes: Vec<N<D>>,
    pub edges: Vec<E<D>>,
    pub container_set_idxs: BTreeSet<usize>,
    pub regions: Vec<Region<D>>,
    pub child_component_keys: BTreeSet<Key>,
    pub hull: Hull<D>,
}

impl<D: Deg + EdgeArg + fmt::Debug + Fmt + RegionArg + ShapeContainsPoint + ThetaPointsArg + Zero> Component<D>
where R2<D>: To<R2<f64>>,
{
    pub fn new(
        set_idxs: Vec<usize>,
        unconnected_containers: &Vec<BTreeSet<usize>>,
        nodes_by_set: &Vec<Vec<N<D>>>,
        sets: &Vec<S<D>>,
        num_sets: usize,
    ) -> Component<D> {
        debug!("Making component: {:?}", set_idxs);
        let component_sets = sets.into_iter().filter(|set| set_idxs.contains(&set.borrow().idx)).cloned().collect();
        let key: Key = set_idxs.iter().map(|i| i.to_string()).collect::<Vec<String>>().join("").into();
        let mut set_idxs_iter = set_idxs.iter().map(|i| unconnected_containers[*i].clone()).enumerate();
        let container_set_idxs = set_idxs_iter.next().unwrap().1;
        for (jdx, containers) in set_idxs_iter {
            if container_set_idxs != containers {
                panic!("Expected all sets to share the same container_set_idxs, but found {}: {:?} vs. {}: {:?}", 0, container_set_idxs, jdx, containers);
            }
        }

        if set_idxs.len() == 1 {
            // Singleton component, set is on its own, doesn't intersect any others
            let set_idx = set_idxs[0];
            let set = &sets[set_idx];
            let zero = set.borrow().zero();
            let tau = zero.clone() + TAU;
            let p = set.borrow().shape.point(zero.clone());
            let mut node = Node {
                idx: 0,
                p,
                n: 0,
                shape_thetas: BTreeMap::new(),
                edges: Vec::new(),
            };
            let n = Rc::new(RefCell::new(node.clone()));
            let edge = Edge {
                idx: 0,
                set: set.clone(),
                node0: n.clone(),
                node1: n.clone(),
                theta0: zero,
                theta1: tau.clone(),
                container_set_idxs: container_set_idxs.clone(),
                is_component_boundary: true,
                visits: 0,
            };
            let e = Rc::new(RefCell::new(edge));
            let region = {
                let mut key = String::new();
                for i in 0..num_sets {
                    if i == set_idx || container_set_idxs.contains(&i) {
                        key += &i.to_string();
                    } else {
                        key += "-";
                    }
                }
                let mut region_container_set_idxs = container_set_idxs.clone();  // TODO: shadow?
                region_container_set_idxs.insert(set_idx);
                Region::new(
                    key,
                    vec![ Segment { edge: e.clone(), fwd: true } ],
                    region_container_set_idxs.clone()
                )
            };
            node.add_edge(e.clone());
            let component = Component {
                key,
                set_idxs: vec![set_idx],
                sets: component_sets,
                nodes: vec![n.clone()],
                edges: vec![e.clone()],
                container_set_idxs: container_set_idxs.clone(),
                child_component_keys: BTreeSet::new(),  // populated by `Scene`, once all `Component`s have been created
                regions: vec![ region.clone() ],
                hull: Hull(vec![ Segment { edge: e.clone(), fwd: true } ]),
            };
            debug!("singleton component: {}", set_idx);
            debug!("  node: {}", node);
            debug!("  edge: {}", e.borrow());
            debug!("  region: {}", region);
            return component;
        }

        let mut nodes: Vec<N<D>> =
            set_idxs
            .iter()
            .flat_map(|i|
                nodes_by_set[*i]
                .iter()
                .filter(|n|
                    n.borrow().shape_thetas.keys().min() == Some(i)
                )
                .cloned()
            )
            .collect();
        nodes.sort_by_cached_key(|n| n.borrow().idx);
        debug!("Connected component: {}", set_idxs.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", "));
        let edges = Component::edges(&set_idxs, &nodes_by_set, &sets, &container_set_idxs);

        let first_hull_edge = edges.iter().find(|e| e.borrow().is_component_boundary).expect(
            format!(
                "Component {} has no boundary edges:\n\t{}\nshapes:\n\t{}\nnodes:\n\t{}",
                set_idxs.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", "),
                edges.iter().map(|e| format!("{}", e.borrow())).collect::<Vec<String>>().join(",\n\t"),
                sets.iter().map(|s| format!("{}", s.borrow().shape)).collect::<Vec<String>>().join(",\n\t"),
                nodes.iter().map(|n| format!("{}", n.borrow())).collect::<Vec<String>>().join(",\n\t"),
            ).as_str()
        );
        let first_hull_segment = Segment { edge: first_hull_edge.clone(), fwd: true };
        let mut hull_segments: Vec<Segment<D>> = vec![ first_hull_segment.clone() ];
        let start_idx = first_hull_segment.start().borrow().idx;
        loop {
            let last_hull_segment = hull_segments.last().unwrap();
            debug!("computing hull, last segment: {}", last_hull_segment);
            let end_idx = last_hull_segment.end().borrow().idx;
            if start_idx == end_idx {
                break;
            }
            let all_successors = last_hull_segment.successors();
            let num_successors = all_successors.len();
            let successors = all_successors.clone().into_iter().filter(|s| s.edge.borrow().is_component_boundary).collect::<Vec<_>>();
            if successors.len() != 1 {
                panic!("Expected 1 boundary successor among {} for hull segment {}, found {}, all: {}", num_successors, last_hull_segment, successors.len(), all_successors.iter().map(|s| format!("{}", s)).collect::<Vec<_>>().join(", "));
            }
            let successor = successors[0].clone();
            hull_segments.push(successor);
        }
        let hull = Hull(hull_segments);

        let total_expected_visits = edges.iter().map(|e| e.borrow().expected_visits()).sum::<usize>();
        let regions = Component::regions(&edges, num_sets, total_expected_visits, &container_set_idxs);
        // let total_visits = edges.iter().map(|e| e.borrow().visits).sum::<usize>();

        // debug!("Made component: {:?}, {:?}", set_idxs, sets);
        Component {
            key,
            set_idxs: set_idxs.clone(),
            sets: component_sets,
            nodes,
            edges,
            container_set_idxs,
            child_component_keys: BTreeSet::new(),  // populated by `Scene`, once all `Component`s have been created
            regions,
            hull,
        }
    }
    pub fn edges(
        set_idxs: &Vec<usize>,
        nodes_by_set: &Vec<Vec<N<D>>>,
        sets: &Vec<S<D>>,
        component_container_idxs: &BTreeSet<usize>,
    ) -> Vec<E<D>> {
        // Construct edges
        let mut edges: Vec<E<D>> = Vec::new();
        for set_idx in set_idxs {
            let set_idx = *set_idx;
            let nodes = &nodes_by_set[set_idx];
            debug!("{} nodes for set {}: {}", nodes.len(), set_idx, nodes.iter().map(|n| n.borrow().theta(set_idx).deg_str()).collect::<Vec<String>>().join(", "));
            let num_set_nodes = nodes.len();
            let set = sets[set_idx].clone();
            for node_idx in 0..num_set_nodes {
                let cur_node = nodes[node_idx].clone();
                let nxt_node = nodes[(node_idx + 1) % num_set_nodes].clone();
                let cur_theta = cur_node.borrow().theta(set_idx);
                let nxt_theta = nxt_node.borrow().theta(set_idx);
                let nxt_theta = if &nxt_theta <= &cur_theta { nxt_theta + TAU } else { nxt_theta };
                let arc_midpoint = sets[set_idx].borrow().shape.arc_midpoint(cur_theta.clone(), nxt_theta.clone());
                let mut is_component_boundary = true;
                let mut container_idxs: BTreeSet<usize> = component_container_idxs.clone();
                for cdx in set_idxs {
                    let cdx = *cdx;
                    if cdx == set_idx {
                        continue;
                    }
                    let container = sets[cdx].clone();
                    let contained = container.borrow().shape.contains(&arc_midpoint);
                    if contained {
                        // Set cdx contains this edge
                        container_idxs.insert(cdx);
                        is_component_boundary = false;
                    }
                }
                let edge = Rc::new(RefCell::new(edge::Edge {
                    idx: edges.len(),
                    set: set.clone(),
                    node0: cur_node, node1: nxt_node,
                    theta0: cur_theta, theta1: nxt_theta,
                    container_set_idxs: container_idxs,
                    is_component_boundary,
                    visits: 0,
                }));
                edges.push(edge.clone());
                edge.borrow_mut().node0.borrow_mut().add_edge(edge.clone());
                edge.borrow_mut().node1.borrow_mut().add_edge(edge.clone());
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
        num_sets: usize,
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
            let container_idxs = &mut edge.borrow().all_idxs().clone();  // Set indices that contain the first Edge, will be intersected with the second Edge below to obtain the set of sets for the Region under construction.
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
                    Component::traverse(&start, num_sets, &mut regions, &mut segments, &container_idxs, &mut visited_nodes, edges.len());
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
        num_sets: usize,
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
            let cidx0 = first_edge.borrow().set.borrow().idx;
            let cidx_end = last_segment.edge.borrow().set.borrow().idx;
            if cidx0 == cidx_end {
                // Can't start and end on same set. Adjacent segments are checked for this as each segment is pushed, but "closing the loop" requires this extra check of the first and last segments.
                return
            } else {
                // We've found a valid Region; increment each Edge's visit count, and save the Region
                for segment in segments.clone() {
                    let mut edge = segment.edge.borrow_mut();
                    edge.visits += 1;
                }
                let mut container_bmp: Vec<bool> = vec![false; num_sets];
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
                let region = Region::new(
                    key,
                    segments.clone(),
                    container_idxs.iter().cloned().collect(),
                );
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
                // The new Segment should be contained by (or run along the border of) the same sets as the previous segments, with one exception: the new Segment can run along the border of a set that doesn't contain the in-progress Region.
                let nxt = successor.edge.clone();
                let nxt_idxs = nxt.borrow().all_idxs();
                // First, verify existing containers are preserved by the new Segment:
                let missing: BTreeSet<usize> = container_idxs.difference(&nxt_idxs).cloned().collect();
                if !missing.is_empty() {
                // let mut both = container_idxs.intersection(&nxt_idxs).cloned().collect::<BTreeSet<usize>>();
                // if both.len() < container_idxs.len() {
                    // This edge candidate isn't contained by (or on the border of) all the sets that the previous segments are.
                    continue;
                }
                let extra: BTreeSet<usize> = nxt_idxs.difference(&container_idxs).cloned().collect();
                // Next, verify that the only additional container, if any, is the Segment's border set:
                let num_extra = extra.len();
                if num_extra > 1 {
                    // This Segment can't join a Region with the existing Segments, as it is contained by at least one set that doesn't contain the existing edges.
                    continue;
                } else if num_extra == 1 {
                    // let extra = nxt_idxs.difference(&container_idxs).cloned().collect::<BTreeSet<usize>>();
                    let extra_idx = extra.iter().next().unwrap();
                    let nxt_edge_idx = successor.edge.borrow().set_idx();
                    if nxt_edge_idx != *extra_idx {
                        // The only admissible extra containing set is the one the new edge traverses
                        continue;
                    } else {
                        // OK to proceed with this edge; it is contained by all the sets that the previous segments are (and outer-borders one additional set that's not included in the bounded region)
                    }
                }
                visited_nodes.insert(next_node_idx);
                segments.push(successor.clone());
                Component::traverse(&start, num_sets, regions, segments, container_idxs, visited_nodes, max_edges);
                segments.pop();
                visited_nodes.remove(&next_node_idx);
            }
        }
    }
}

impl<D: RegionArg> Component<D>
where R2<D>: To<R2<f64>>,
{
    pub fn area(&self) -> D {
        self.regions.iter().map(|r| r.total_area.clone()).sum::<D>()
    }
}

impl<D> Component<D> {
    pub fn contains(&self, key: &Key) -> bool {
        self.sets.iter().any(|set| set.borrow().child_component_keys.contains(key))
    }
}

impl<D: AreaArg + Into<f64>> Component<D>
// where f64: From<&'a D>  `From<&f64> for f64` doesn't exist…?
{
    pub fn verify_areas(&self, ε: f64) -> Result<()> {
        let mut shape_region_areas: BTreeMap<usize, f64> = BTreeMap::new();
        for region in &self.regions {
            let region_area: f64 = region.total_area.clone().into();
            for set_idx in &region.container_set_idxs {
                let shape_region_area = shape_region_areas.entry(*set_idx).or_insert(0.);
                *shape_region_area += region_area;
            }
        }
        for set_idx in &self.set_idxs {
            let shape_region_area = *shape_region_areas.get(set_idx).unwrap();
            let shape = &self.sets[*set_idx].borrow().shape;
            let shape_area: f64 = shape.area().into();
            let diff = (shape_region_area / shape_area - 1.).abs();
            if diff > ε {
                error!(
                // return Err(anyhow!(
                    "shape {} area {} != sum of regions area {}, half-diff {}",
                    set_idx, shape_area, shape_region_area, (shape_region_area - shape_area) / 2.
                // ));
                )
            }
        }
        Ok(())
    }
}