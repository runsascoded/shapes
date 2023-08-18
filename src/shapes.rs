use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

use crate::{dual::Dual, circle::Circle, intersection::Intersection, edge::Edge, region::Region};


type D = Dual;
type Node = Rc<RefCell<Intersection<D>>>;

struct Shapes {
    shapes: Vec<Circle<f64>>,
    duals: Vec<Circle<D>>,
    nodes: Vec<Node>,
    nodes_by_shape: Vec<Vec<Node>>,
    nodes_by_shapes: Vec<Vec<Vec<Node>>>,
    // edges: Vec<Edge<D>>,
    // regions: Vec<Region>,
}

impl Shapes {
    pub fn new(shapes: Vec<Circle<f64>>) -> Shapes {
        let n = shapes.len();
        let duals: Vec<Circle<D>> = shapes.iter().enumerate().map(|(idx, c)| c.dual(idx * 3, 3 * (n - 1 - idx))).collect();
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
                let intersections = dual.intersect(&duals[jdx]);
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

        Shapes { shapes, duals, nodes, nodes_by_shape, nodes_by_shapes }
    }
}

#[cfg(test)]
mod tests {
    use crate::r2::R2;

    use super::*;

    #[test]
    fn test_00_10_01() {
        let c0 = Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. };
        let c2 = Circle { idx: 2, c: R2 { x: 0., y: 1. }, r: 1. };
        let circles = vec![c0, c1, c2];
        let shapes = Shapes::new(circles);
        assert_eq!(shapes.nodes.len(), 6);
        for node in shapes.nodes.iter() {
            println!("{}", node.borrow());
        }
        println!();
        for (idx, nodes) in shapes.nodes_by_shape.iter().enumerate() {
            println!("nodes_by_shape[{}]: {}", idx, nodes.len());
            for node in nodes.iter() {
                println!("  {}", node.borrow());
            }
        }
        println!();
        for (idx, shapes_nodes) in shapes.nodes_by_shapes.iter().enumerate() {
            println!("nodes_by_shapes[{}]:", idx);
            for (jdx, nodes) in shapes_nodes.iter().enumerate() {
                println!("  nodes_by_shapes[{}][{}]: {}", idx, jdx, nodes.len());
                for node in nodes.iter() {
                    println!("    {}", node.borrow());
                }
            }
        }
    }
}