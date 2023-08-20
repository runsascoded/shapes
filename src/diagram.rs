use core::num;
use std::collections::HashMap;

use crate::{circle::Circle, shapes::Shapes, dual::{D, Dual}, r2::R2};


struct Diagram {
    shapes: Shapes,
    targets: HashMap<String, f64>,
}

impl Diagram {
    pub fn new(shapes: Vec<Circle<f64>>, targets: HashMap<String, f64>) -> Diagram {
        let shapes = Shapes::new(shapes);
        Diagram { shapes, targets }
    }

    pub fn n(&self) -> usize {
        self.shapes.len()
    }

    pub fn key_contained_by(k0: &String, k1: &String) -> bool {
        k0.chars().zip(k1.chars()).all(|(a, b)| a == '-' || a == b || b == '*')
    }

    pub fn error(&self) -> D {
        let all_key = String::from_utf8(vec![b'*'; self.n()]).unwrap();
        let shapes = &self.shapes;
        let targets = &self.targets;
        let total_area = shapes.area(&all_key);
        let num_vars = total_area.d().len();
        let mut total_target_area = 0.;
        for (key, area) in targets.iter() {
            let mut is_contained = false;
            for key2 in targets.keys() {
                if key != key2 && Self::key_contained_by(key, key2) {
                    is_contained = true;
                    break;
                }
            }
            if !is_contained {
                total_target_area += area;
            }
        }
        let mut total_err: D = Dual::new(0., vec![0.; num_vars]);
        for (key, target_area) in targets.iter() {
            let actual_area = shapes.area(key);
            let target_frac = target_area / total_target_area;
            let actual_frac = actual_area.clone() / &total_area;
            let err = (actual_frac.clone() - target_frac).abs();
            println!(
                "{}: err {:.3}, {:.3} / {:.3} = {:.3}, {:.3} / {:.3} = {:.3}",
                key, err.v(),
                actual_area.v(), total_area.v(), actual_frac.v(),
                target_area, total_target_area, target_frac,
            );
            total_err += err;
        }
        total_err
    }

    pub fn step(&self, step_size: f64) -> Diagram {
        let grad_vec = (-self.error()).d();
        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>();
        let step_vec = grad_vec.iter().map(|d| d / magnitude * step_size).collect::<Vec<f64>>();
        let mut shapes = &self.shapes.shapes;
        let new_shapes = shapes.iter().map(|s| {
            let idx = s.idx * 3;
            let c = R2 {
                x: s.c.x + step_vec[idx],
                y: s.c.y + step_vec[idx + 1],
            };
            let r = s.r + step_vec[idx + 2];
            Circle { idx: s.idx, c, r }
        }).collect::<Vec<Circle<f64>>>();
        println!("Applying step_size {:.3} / {:?}:", step_size, step_vec);
        for (cur, nxt) in shapes.iter().zip(new_shapes.iter()) {
            println!("  {} -> {}", cur, nxt);
        }
        Diagram::new(new_shapes, self.targets.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::{r2::R2};

    use super::*;

    #[test]
    fn simple() {
        let c0 = Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. };
        let c1 = Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. };
        let circles = vec![c0, c1];
        let mut targets = HashMap::<String, f64>::new();
        targets.insert("0*".to_string(), 1. / 3.);
        targets.insert("*1".to_string(), 1. / 5.);
        targets.insert("01".to_string(), 1. / 15.);
        let mut diagram = Diagram::new(circles, targets);
        let expected_errs = [
            Dual::new(0.368, vec![ 0.426, 0.000, -1.030, -0.426, 0.000, 1.456]),
            Dual::new(0.334, vec![ 0.827,  0.000,  0.386, -0.827,  0.000,  0.469]),
            Dual::new(0.238, vec![ 0.746,  0.000,  0.398, -0.746,  0.000,  0.465]),
            Dual::new(0.163, vec![ 0.302,  0.000, -0.964, -0.302,  0.000,  1.458]),
            Dual::new(0.125, vec![ 0.000,  0.000, -0.000, -0.000,  0.000, -0.000]),
        ];

        for (idx, expected_err) in expected_errs.iter().enumerate() {
            let total_err = diagram.error();
            println!("Step {}, total_err {}", idx, total_err);
            assert_relative_eq!(total_err, expected_err, epsilon = 1e-3);
            diagram = diagram.step(0.1);
        }
    }
}