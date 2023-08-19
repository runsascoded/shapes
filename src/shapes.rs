use std::{cell::RefCell, rc::Rc};

use crate::{circle::{Circle, C}, intersection::Node, edge::{self, E}, r2::R2};

pub struct Shapes {
    shapes: Vec<Circle<f64>>,
    duals: Vec<C>,
    nodes: Vec<Node>,
    nodes_by_shape: Vec<Vec<Node>>,
    nodes_by_shapes: Vec<Vec<Vec<Node>>>,
    edges: Vec<E>,
    // regions: Vec<Region>,
}

impl Shapes {
    pub fn new(shapes: Vec<Circle<f64>>) -> Shapes {
        let n = shapes.len();
        let duals: Vec<C> = shapes.iter().enumerate().map(|(idx, c)| Rc::new(RefCell::new(c.dual(idx * 3, 3 * (n - 1 - idx))))).collect();
        let mut nodes: Vec<Node> = Vec::new();
        let mut nodes_by_shape: Vec<Vec<Node>> = Vec::new();
        let mut nodes_by_shapes: Vec<Vec<Vec<Node>>> = Vec::new();
        for idx in 0..n {
            nodes_by_shape.push(Vec::new());
            let mut shapes_nodes: Vec<Vec<Node>> = Vec::new();
            for jdx in 0..n {
                shapes_nodes.push(Vec::new());
            }
            nodes_by_shapes.push(shapes_nodes);
        }
        for (idx, dual) in duals.iter().enumerate() {
            for jdx in (idx + 1)..n {
                let intersections = dual.borrow().intersect(&duals[jdx].borrow());
                let ns = intersections.map(|i| Rc::new(RefCell::new(i)));
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

        let mut edges: Vec<E> = Vec::new();
        let mut edges_by_shape: Vec<Vec<E>> = Vec::new();
        for idx in 0..n {
            let nodes = &nodes_by_shape[idx];
            let mut shape_edges: Vec<E> = Vec::new();
            let m = n;
            let n = nodes.len();
            let c = duals[idx].clone();
            for jdx in 0..n {
                let i0 = nodes[jdx].clone();
                let i1 = nodes[(jdx + 1) % n].clone();
                let midpoint = R2 {
                    x: (&i0.borrow().x + &i1.borrow().x) / 2.,
                    y: (&i0.borrow().y + &i1.borrow().y) / 2.,
                };
                let mut containers: Vec<C> = Vec::new();
                let mut containments: Vec<bool> = Vec::new();
                for cdx in 0..m {
                   if cdx == idx {
                       continue;
                   }
                   let container = duals[cdx].clone();
                   let contained = container.borrow().contains(&midpoint);
                   if contained {
                       containers.push(container);
                   }
                   containments.push(contained);
                }
                let c0idx = i0.borrow().other(idx);
                let c1idx = i1.borrow().other(idx);
                let edge = Rc::new(RefCell::new(edge::Edge {
                    c: c.clone(),
                    c0: duals[c0idx].clone(),
                    c1: duals[c1idx].clone(),
                    i0,
                    i1,
                    containers,
                    containments,
                 }));
                edges.push(edge.clone());
                shape_edges.push(edge.clone());
                edge.borrow_mut().i0.borrow_mut().add_edge(edge.clone());
                edge.borrow_mut().i1.borrow_mut().add_edge(edge.clone());
            }
            edges_by_shape.push(shape_edges);
        }

        Shapes { shapes, duals, nodes, nodes_by_shape, nodes_by_shapes, edges, }
    }
}

#[cfg(test)]
mod tests {
    use crate::deg::Deg;
    use crate::dual::Dual;
    use crate::math::round;
    use crate::r2::R2;

    use super::*;

    #[test]
    fn test_00_10_01() {
        let c0 = Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. };
        let c2 = Circle { idx: 2, c: R2 { x: 0., y: 1. }, r: 1. };
        let circles = vec![c0, c1, c2];
        let shapes = Shapes::new(circles);

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

        // for node in shapes.nodes.iter() {
        //     println!("{}", node.borrow());
        // }
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

        assert_eq!(shapes.edges.len(), 12);
        println!("edges:");
        for edge in shapes.edges.iter() {
            let edge = edge.borrow();
            println!("C{}: {}({}) → {}({})", edge.c.borrow().idx, edge.t0().v().deg_str(), edge.c0.borrow().idx, edge.t1().v().deg_str(), edge.c1.borrow().idx);
        }
        println!();
    }
}