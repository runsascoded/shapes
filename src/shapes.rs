use crate::{dual::Dual, circle::Circle, intersection::Intersection, edge::Edge, region::Region};


type D = Dual;

struct Shapes {
    circles: Vec<Circle<f64>>,
    duals: Vec<Circle<D>>,
    nodes: Vec<Intersection<D>>,
    // edges: Vec<Edge<D>>,
    // regions: Vec<Region>,
}

impl Shapes {
    pub fn new(circles: Vec<Circle<f64>>) -> Shapes {
        let n = circles.len();
        let duals: Vec<Circle<D>> = circles.iter().enumerate().map(|(idx, c)| c.dual(idx * 3, 3 * (n - 1 - idx))).collect();
        let mut nodes: Vec<Intersection<D>> = Vec::new();
        for (idx, dual) in duals.iter().enumerate() {
            for jdx in (idx + 1)..n {
                let intersections = dual.intersect(&duals[jdx]);
                nodes.extend(intersections);
            }
        }

        Shapes { circles, duals, nodes }
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
        for node in shapes.nodes.iter() {
            println!("{}", node);
        }
        assert_eq!(shapes.nodes.len(), 6);
    }
}