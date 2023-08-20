use std::{cell::RefCell, rc::Rc, f64::consts::PI, collections::HashSet};

use crate::{circle::{Circle, C}, intersection::Node, edge::{self, E}, region::{Region, Segment}, dual::D};

pub struct Shapes {
    pub shapes: Vec<Circle<f64>>,
    pub duals: Vec<C>,
    pub nodes: Vec<Node>,
    pub nodes_by_shape: Vec<Vec<Node>>,
    pub nodes_by_shapes: Vec<Vec<Vec<Node>>>,
    pub edges: Vec<E>,
    pub is_connected: Vec<Vec<bool>>,
    pub regions: Vec<Region>,
    pub total_visits: usize,
    pub total_expected_visits: usize,
}

impl Shapes {
    pub fn new(shapes: Vec<Circle<f64>>) -> Shapes {
        let n = shapes.len();
        let duals: Vec<C> = shapes.iter().enumerate().map(|(idx, c)| Rc::new(RefCell::new(c.dual(idx * 3, 3 * (n - 1 - idx))))).collect();
        let mut nodes: Vec<Node> = Vec::new();
        let mut nodes_by_shape: Vec<Vec<Node>> = Vec::new();
        let mut nodes_by_shapes: Vec<Vec<Vec<Node>>> = Vec::new();
        for _idx in 0..n {
            nodes_by_shape.push(Vec::new());
            let mut shapes_nodes: Vec<Vec<Node>> = Vec::new();
            for _jdx in 0..n {
                shapes_nodes.push(Vec::new());
            }
            nodes_by_shapes.push(shapes_nodes);
        }
        for (idx, dual) in duals.iter().enumerate() {
            for jdx in (idx + 1)..n {
                let intersections = dual.borrow().intersect(&duals[jdx].borrow());
                let ns = intersections.iter().map(|i| Rc::new(RefCell::new(i.clone())));
                for n in ns {
                    nodes.push(n.clone());
                    nodes_by_shape[idx].push(n.clone());
                    nodes_by_shapes[idx][jdx].push(n.clone());
                    nodes_by_shape[jdx].push(n.clone());
                    nodes_by_shapes[jdx][idx].push(n.clone());
                }
            }
        }

        // Sort each circle's nodes in order of where they appear on the circle (from -PI to PI)
        for (idx, nodes) in nodes_by_shape.iter_mut().enumerate() {
            nodes.sort_by_cached_key(|n| n.borrow().theta(idx))
        }

        // Compute connected components (shape -> shape -> bool)
        let mut is_connected: Vec<Vec<bool>> = Vec::new();
        for idx in 0..n {
            let mut connected: Vec<bool> = Vec::new();
            for jdx in 0..n {
                connected.push(idx == jdx);
            }
            is_connected.push(connected);
        }
        for node in &nodes {
            let i0 = node.borrow().c0idx;
            let i1 = node.borrow().c1idx;
            if !is_connected[i0][i1] {
                for i2 in 0..n {
                    if is_connected[i0][i2] && !is_connected[i1][i2] {
                        for i3 in 0..n {
                            if is_connected[i1][i3] && !is_connected[i2][i3] {
                                is_connected[i2][i3] = true;
                                is_connected[i3][i2] = true;
                            }
                        };
                    }
                };
            }
        }

        let mut edges: Vec<E> = Vec::new();
        let mut edges_by_shape: Vec<Vec<E>> = Vec::new();
        let mut total_expected_visits = 0;
        for idx in 0..n {
            let nodes = &nodes_by_shape[idx];
            let mut shape_edges: Vec<E> = Vec::new();
            let m = n;
            let n = nodes.len();
            let c = duals[idx].clone();
            for jdx in 0..n {
                let i0 = nodes[jdx].clone();
                let i1 = nodes[(jdx + 1) % n].clone();
                let t0 = i0.borrow().theta(idx);
                let mut t1 = i1.borrow().theta(idx);
                if t1 < t0 {
                    t1 += 2. * PI;
                }
                let arc_midpoint = shapes[idx].arc_midpoint(t0.v(), t1.v());
                let mut is_component_boundary = true;
                let mut containers: Vec<C> = Vec::new();
                let mut containments: Vec<bool> = Vec::new();
                for cdx in 0..m {
                    if cdx == idx {
                        continue;
                    }
                    let container = duals[cdx].clone();
                    let contained = container.borrow().v().contains(&arc_midpoint);
                    if contained {
                        // Shape cdx contains this edge
                        containers.push(container);
                        if is_connected[idx][cdx] {
                            is_component_boundary = false;
                        }
                    }
                    containments.push(contained);
                }
                let c0idx = i0.borrow().other(idx);
                let c1idx = i1.borrow().other(idx);
                let expected_visits = if is_component_boundary { 1 } else { 2 };
                total_expected_visits += expected_visits;
                let edge = Rc::new(RefCell::new(edge::Edge {
                    c: c.clone(),
                    c0: duals[c0idx].clone(),
                    c1: duals[c1idx].clone(),
                    i0, i1,
                    t0, t1,
                    containers,
                    containments,
                    expected_visits,
                    visits: 0,
                 }));
                edges.push(edge.clone());
                shape_edges.push(edge.clone());
                edge.borrow_mut().i0.borrow_mut().add_edge(edge.clone());
                edge.borrow_mut().i1.borrow_mut().add_edge(edge.clone());
            }
            edges_by_shape.push(shape_edges);
        }

        fn traverse(
            start: &Node,
            num_shapes: usize,
            regions: &mut Vec<Region>,
            segments: &mut Vec<Segment>,
            container_idxs: &mut HashSet<usize>,
        ) {
            let last_segment = segments.last().unwrap();
            let end = last_segment.end();
            if start.borrow().p() == end.borrow().p() {
                // Back where we started; check whether this is a valid region, push it if so, and return
                let first_segment = segments.first().unwrap();
                let first_edge = first_segment.edge.clone();
                let cidx0 = first_edge.borrow().c.borrow().idx;
                let cidx_end = last_segment.edge.borrow().c.borrow().idx;
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
                for successor in successors {
                    // The new Segment should be contained by (or run along the border of) the same shapes as the previous segments, with one exception: the new Segment can run along the border of a shape that doesn't contain the in-progress Region.
                    let nxt = successor.edge.clone();
                    let nxt_idxs = nxt.borrow().all_idxs();
                    // First, verify existing containers are preserved by the new Segment:
                    let mut both = container_idxs.intersection(&nxt_idxs).cloned().collect::<HashSet<usize>>();
                    if both.len() < container_idxs.len() {
                        // This edge candidate isn't contained by (or on the border of) all the shapes that the previous segments are.
                        continue;
                    }
                    // Next, verify that the only additional container, if any, is the Segment's border shape:
                    let num_extra = nxt_idxs.len() - both.len();
                    if num_extra > 1 {
                        // This Segment can't join a Region with the existing Segments, as it is contained by at least one shape that doesn't contain the existing edges.
                        continue;
                    } else if num_extra == 1 {
                        let extra = nxt_idxs.difference(&container_idxs).cloned().collect::<HashSet<usize>>();
                        let extra_idx = extra.iter().next().unwrap();
                        let nxt_edge_idx = successor.edge.borrow().c.borrow().idx;
                        if nxt_edge_idx != *extra_idx {
                            // The only admissible extra containing shape is the one the new edge traverses
                            continue;
                        } else {
                            // OK to proceed with this edge; it is contained by all the shapes that the previous segments are (and outer-borders one additional shape that's not included in the bounded region)
                        }
                    }
                    segments.push(successor.clone());
                    traverse(&start, num_shapes, regions, segments, &mut both);
                    segments.pop();
                }
            }
        }

        // Graph-traversal will accumulate Regions here
        let mut regions: Vec<Region> = Vec::new();
        // Working list o Segments comprising partial Regions, as they are built up and verified by `traverse`
        let mut segments: Vec<Segment> = Vec::new();
        // The first two Segments for each Region uniquely determine various properties of the Region, so we loop over and construct them explicitly below, before kicking off a recursive `traverse` process to complete each Region.
        for edge in edges.clone() {
            if edge.borrow().visits == edge.borrow().expected_visits {
                // All of the Regions we expect this Edge to be a part of have already been computed and saved, nothing further to do with this Edge.
                continue;
            }
            let segment = Segment { edge: edge.clone(), fwd: true };  // Each Region's first edge can be traversed in the forward direction, WLOG
            let start = &segment.start();
            let successors = segment.successors();
            segments.push(segment);
            let container_idxs = &mut edge.borrow().all_idxs().clone();  // Shape indices that contain the first Edge, will be intersected with the second Edge below to obtain the set of shapes for the Region under construction.
            for successor in successors {
                segments.push(successor.clone());
                let nxt = successor.edge.clone();
                let nxt_idxs = nxt.borrow().all_idxs();
                let mut both = container_idxs.intersection(&nxt_idxs).cloned().collect::<HashSet<usize>>();
                // Recursively traverse the graph, trying to add each eligible Segment to the list we've seeded here, accumulating valid Regions in `regions` along the way.
                traverse(&start, n, &mut regions, &mut segments, &mut both);
                assert_eq!(segments.len(), 2);
                segments.pop();
            }
            segments.pop();
        }

        // Verify that all Edges have been visited the expected number of times
        let total_visits = edges.iter().map(|e| e.borrow().visits).sum::<usize>();
        if total_visits != total_expected_visits {
            panic!("total_visits ({}) != total_expected_visits ({})", total_visits, total_expected_visits);
        }

        Shapes { shapes, duals, nodes, nodes_by_shape, nodes_by_shapes, edges, is_connected, regions, total_visits, total_expected_visits, }
    }

    pub fn area(&self, key: &String) -> D {
        self.regions.iter().filter(|r| r.matches(key)).map(|r| r.area()).sum()
    }

    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    pub fn num_vars(&self) -> usize {
        self.duals[0].borrow().r.d().len()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Ref;

    use crate::deg::Deg;
    use crate::dual::Dual;
    use crate::edge::Edge;
    use crate::math::round;
    use crate::r2::R2;
    use crate::region::Segment;

    use super::*;

    #[test]
    fn test_00_10_01() {
        let c0 = Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. };
        let c2 = Circle { idx: 2, c: R2 { x: 0., y: 1. }, r: 1. };
        let circles = vec![c0, c1, c2];
        let shapes = Shapes::new(circles);

        for node in shapes.nodes.iter() {
            println!("{}", node.borrow());
        }

        let check = |idx: usize, x: Dual, y: Dual, c0idx: usize, deg0v: i64, deg0d: [i64; 9], c1idx: usize, deg1v: i64, deg1d: [i64; 9]| {
            let n = shapes.nodes[idx].borrow();
            assert_relative_eq!(n.x, x, epsilon = 1e-3);
            assert_relative_eq!(n.y, y, epsilon = 1e-3);
            assert_eq!(n.c0idx, c0idx);
            assert_eq!(n.c1idx, c1idx);
            let d0 = n.t0.deg();
            let a0: (i64, Vec<i64>) = (round(&d0.v()), d0.d().iter().map(round).collect());
            assert_eq!(a0, (deg0v, deg0d.into()));
            let d1 = n.t1.deg();
            let a1: (i64, Vec<i64>) = (round(&d1.v()), d1.d().iter().map(round).collect());
            assert_eq!(a1, (deg1v, deg1d.into()));
        };

        let intersections = [
            //    x                                                                     dx         y                                                                    dy c0idx deg0                                       deg0.d c1idx  deg1                                       deg1.d
            ( 0.500, [ 0.500,  0.866,  1.000, 0.500, -0.866, -1.000, 0.000,  0.000,  0.000 ],  0.866, [ 0.289, 0.500,  0.577, -0.289, 0.500,  0.577,  0.000, 0.000,  0.000 ], 0,  60, [  33, -57, -33, -33, 57,  66,   0,   0,   0 ], 1,  120, [ -33, -57, -66,  33, 57,  33,   0,   0,   0 ]),
            ( 0.500, [ 0.500, -0.866,  1.000, 0.500,  0.866, -1.000, 0.000,  0.000,  0.000 ], -0.866, [-0.289, 0.500, -0.577,  0.289, 0.500, -0.577,  0.000, 0.000,  0.000 ], 0, -60, [ -33, -57,  33,  33, 57, -66,   0,   0,   0 ], 1, -120, [  33, -57,  66, -33, 57, -33,   0,   0,   0 ]),
            ( 0.866, [ 0.500,  0.289,  0.577, 0.000,  0.000,  0.000, 0.500, -0.289,  0.577 ],  0.500, [ 0.866, 0.500,  1.000,  0.000, 0.000,  0.000, -0.866, 0.500, -1.000 ], 0,  30, [  57, -33,  33,   0,  0,   0, -57,  33, -66 ], 2,  -30, [  57,  33,  66,   0,  0,   0, -57, -33, -33 ]),
            (-0.866, [ 0.500, -0.289, -0.577, 0.000,  0.000,  0.000, 0.500,  0.289, -0.577 ],  0.500, [-0.866, 0.500,  1.000,  0.000, 0.000,  0.000,  0.866, 0.500, -1.000 ], 0, 150, [  57,  33, -33,   0,  0,   0, -57, -33,  66 ], 2, -150, [  57, -33, -66,   0,  0,   0, -57,  33,  33 ]),
            ( 1.000, [ 0.000,  0.000,  0.000, 0.000,  0.000,  0.000, 1.000,  0.000,  1.000 ],  1.000, [ 0.000, 0.000,  0.000,  0.000, 1.000,  1.000,  0.000, 0.000,  0.000 ], 1,  90, [   0,   0,   0,  57,  0,   0, -57,   0, -57 ], 2,    0, [   0,   0,   0,   0, 57,  57,   0, -57,   0 ]),
            ( 0.000, [ 0.000,  0.000,  0.000, 1.000,  0.000, -1.000, 0.000,  0.000,  0.000 ],  0.000, [ 0.000, 0.000,  0.000,  0.000, 0.000,  0.000,  0.000, 1.000, -1.000 ], 1, 180, [   0,   0,   0,   0, 57,   0,   0, -57,  57 ], 2,  -90, [   0,   0,   0,  57,  0, -57, -57,   0,   0 ]),
        ];
        assert_eq!(shapes.nodes.len(), intersections.len());
        for (idx, (x, dx, y, dy, c0idx, deg0v, deg0d, c1idx, deg1v, deg1d)) in intersections.iter().enumerate() {
            let x = Dual::new(*x, dx.iter().map(|d| *d as f64).collect());
            let y = Dual::new(*y, dy.iter().map(|d| *d as f64).collect());
            check(idx, x, y, *c0idx, *deg0v, *deg0d, *c1idx, *deg1v, *deg1d);
        }

        // println!();
        // for (idx, nodes) in shapes.nodes_by_shape.iter().enumerate() {
        //     println!("nodes_by_shape[{}]: {}", idx, nodes.len());
        //     for node in nodes.iter() {
        //         println!("  {}", node.borrow());
        //     }
        // }
        // println!();
        // for (idx, shapes_nodes) in shapes.nodes_by_shapes.iter().enumerate() {
        //     println!("nodes_by_shapes[{}]:", idx);
        //     for (jdx, nodes) in shapes_nodes.iter().enumerate() {
        //         println!("  nodes_by_shapes[{}][{}]: {}", idx, jdx, nodes.len());
        //         for node in nodes.iter() {
        //             println!("    {}", node.borrow());
        //         }
        //     }
        // }

        fn edge_str(edge: Ref<Edge>) -> String {
            let containers: Vec<String> = edge.containers.iter().map(|c| format!("{}", c.borrow().idx)).collect();
            format!(
                "C{}: {}({}) → {}({}), containers: [{}], expected_visits: {}",
                edge.c.borrow().idx,
                edge.t0.v().deg_str(), edge.c0.borrow().idx,
                edge.t1.v().deg_str(), edge.c1.borrow().idx,
                containers.join(","),
                edge.expected_visits,
            )
        }

        assert_eq!(shapes.edges.len(), 12);
        println!("edges:");
        for edge in shapes.edges.iter() {
            println!("{}", edge_str(edge.borrow()));
        }
        println!();

        let is_connected = shapes.is_connected;
        println!("is_connected:");
        for row in is_connected {
            for col in row {
                print!("{}", if col { "1" } else { "0" });
            }
            println!();
        }
        println!();

        let segment = Segment { edge: shapes.edges[0].clone(), fwd: true };
        let successors = segment.successors();
        println!("successors:");
        for successor in successors {
            println!("  {} (fwd: {})", edge_str(successor.edge.borrow()), successor.fwd);
        }
        println!();

        assert_eq!(shapes.regions.len(), 7);
        assert_eq!(shapes.total_expected_visits, 21);
        assert_eq!(shapes.total_visits, 21);
        let expected = [
            "01- 0( -60) 2( -30) 1( 180): 0.500 + 0.285 =  0.785 + [ 1.366 -0.366  1.571 -0.866 -0.500  1.047 -0.500  0.866 -1.047]ε",
            "-1- 0( -60) 2( -30) 1(  90): 0.000 + 1.785 =  1.785 + [-1.366  0.366 -1.571  1.866 -0.500  3.665 -0.500  0.134 -0.524]ε",
            "-12 0(  30) 1( 120) 2(   0): 0.116 + 0.012 =  0.128 + [-0.366 -0.366 -0.524 -0.134  0.500  0.524  0.500 -0.134  0.524]ε",
            "012 0(  30) 1( 120) 2( -90): 0.250 + 0.193 =  0.443 + [ 0.366  0.366  0.524 -0.866  0.500  1.047  0.500 -0.866  1.047]ε",
            "0-2 0(  60) 2(-150) 1( 180): 0.500 + 0.285 =  0.785 + [-0.366  1.366  1.571  0.866 -0.500 -1.047 -0.500 -0.866  1.047]ε",
            "--2 0(  60) 2(-150) 1(  90): 0.000 + 1.785 =  1.785 + [ 0.366 -1.366 -1.571  0.134 -0.500 -0.524 -0.500  1.866  3.665]ε",
            "0-- 0( 150) 1(-120) 2( -90): 0.250 + 0.878 =  1.128 + [-1.366 -1.366  2.618  0.866  0.500 -1.047  0.500  0.866 -1.047]ε",
        ];
        let actual = shapes.regions.iter().map(|region| {
            let segments = &region.segments;
            let path_str = segments.iter().map(|segment| {
                let start = segment.start();
                let edge = segment.edge.clone();
                let cidx = edge.borrow().c.borrow().idx;
                format!("{}({})", cidx, start.borrow().theta(cidx).v().deg_str())
            }).collect::<Vec<String>>().join(" ");
            format!("{} {}: {:.3} + {:.3} = {}", region.key, path_str, region.polygon_area().v(), region.secant_area().v(), region.area().s(3))
        }).collect::<Vec<String>>();
        println!("regions:");
        for a in actual.iter() {
            println!("{}", a);
        }
        println!();
        actual.iter().zip(expected.iter()).enumerate().for_each(|(idx, (a, b))| assert_eq!(&a, b, "idx: {}, {} != {}", idx, a, b));
    }
    #[test]
    fn test_components() {
        let c0 = Circle { idx: 0, c: R2 { x: 0. , y: 0. }, r: 1. };
        let c1 = Circle { idx: 1, c: R2 { x: 1. , y: 0. }, r: 1. };
        let c2 = Circle { idx: 2, c: R2 { x: 0.5, y: 0. }, r: 3. };
        let c3 = Circle { idx: 3, c: R2 { x: 0. , y: 3. }, r: 1. };
        let circles = vec![c0, c1, c2, c3];
        let shapes = Shapes::new(circles);

        for node in shapes.nodes.iter() {
            println!("{}", node.borrow());
        }
        println!();

        let is_connected = shapes.is_connected;
        println!("is_connected:");
        for row in is_connected {
            for col in row {
                print!("{}", if col { "1" } else { "0" });
            }
            println!();
        }
        println!();
    }
}