use std::{collections::{HashMap, HashSet}, fmt::Display};

use crate::{circle::{Circle, Split, Duals}, shapes::Shapes, dual::D, r2::R2};


pub struct Diagram {
    inputs: Vec<Split>,
    shapes: Shapes,
    targets: HashMap<String, f64>,
}

pub struct Error {
    key: String,
    actual_area: D,
    total_area: D,
    actual_frac: D,
    target_area: f64,
    total_target_area: f64,
    target_frac: f64,
    error: D,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{}: err {:.3}, {:.3} / {:.3} = {:.3}, {:.3} / {:.3} = {:.3}",
            self.key, self.error.v(),
            self.actual_area.v(), self.total_area.v(), self.actual_frac.v(),
            self.target_area, self.total_target_area, self.target_frac,
        )
    }
}

impl Diagram {
    pub fn new(inputs: Vec<Split>, targets: HashMap<String, f64>) -> Diagram {
        let shapes = Shapes::new(&inputs);
        Diagram { inputs, shapes, targets, }
    }

    pub fn n(&self) -> usize {
        self.shapes.len()
    }

    pub fn key_contained_by(k0: &String, k1: &String) -> bool {
        k0.chars().zip(k1.chars()).all(|(a, b)| a == '-' || a == b || b == '*')
    }

    pub fn errors(&self) -> HashMap<&String, Error> {
        let all_key = String::from_utf8(vec![b'*'; self.n()]).unwrap();
        let shapes = &self.shapes;
        let targets = &self.targets;
        let total_area = shapes.area(&all_key);
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
        targets.iter().map(|(key, target_area)| {
            let actual_area = shapes.area(key);
            let target_frac = target_area / total_target_area;
            let actual_frac = actual_area.clone() / &total_area;
            let error = (actual_frac.clone() - target_frac).abs();
            (key, Error {
                key: key.clone(),
                actual_area, total_area: total_area.clone(),
                target_area: target_area.clone(), total_target_area,
                actual_frac,
                target_frac,
                error,
            })
        }).collect()
    }
    pub fn error(&self) -> D {
        self.errors().values().into_iter().map(|e| &e.error).cloned().collect::<Vec<D>>().into_iter().sum()
    }

    pub fn duals(&self) -> Vec<Duals> {
        self.inputs.iter().map(|(_, duals)| duals.clone()).collect()
    }

    pub fn step(&self, step_size: f64) -> Diagram {
        let grad_vec = (-self.error()).d();
        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>();
        let step_vec = grad_vec.iter().map(|d| d / magnitude * step_size).collect::<Vec<f64>>();
        println!("step_vec {:?}", step_vec);
        let shapes = &self.shapes.shapes;
        let new_inputs = shapes.iter().zip(self.duals()).map(|(s, duals)| {
            let updates: [f64; 3] = duals.clone().map(|d| d.iter().zip(&step_vec).map(|(mask, step)| mask * step).sum());
            println!("  updates {:?}", updates);
            let c = R2 {
                x: s.c.x + updates[0],
                y: s.c.y + updates[1],
            };
            let r = s.r + updates[2];
            (Circle { idx: s.idx, c, r }, duals)
        }).collect::<Vec<Split>>();
        println!("Applying step_size {:.3} / {:?}:", step_size, step_vec);
        for (cur, (nxt, _)) in shapes.iter().zip(new_inputs.iter()) {
            println!("  {} -> {}", cur, nxt);
        }
        Diagram::new(new_inputs, self.targets.clone(), )
    }
}

#[cfg(test)]
mod tests {
    use crate::{r2::R2, dual::Dual};

    use super::*;

    #[test]
    fn simple() {
        // 2 Circles, only the 2nd circle's x and r can move
        let inputs = vec![
            (Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. }, [ vec![0., 0.], vec![0., 0.], vec![0., 0.], ]),
            (Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. }, [ vec![1., 0.], vec![0., 0.], vec![0., 1.], ]),
        ];
        let tgts = [
            ("0*", 1. /  3.),
            ("*1", 1. /  5.),
            ("01", 1. / 15.),
        ];
        let mut targets: HashMap::<_, _> = tgts.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut diagram = Diagram::new(inputs, targets);
        let expected_errs = [
            Dual::new(0.368, vec![ -0.426, 1.456, ]),
            Dual::new(0.317, vec![ -0.831, 0.476, ]),
            Dual::new(0.220, vec![ -0.744, 0.492, ]),
            Dual::new(0.125, vec![  0.000, 0.000, ]),
            // Dual::new(0.125, vec![ 0.000,  0.000,  0.000,  0.000,  0.000,  0.000]),
        ];
        for (idx, expected_err) in expected_errs.iter().enumerate() {
            println!("Step {}", idx);
            let errors = diagram.errors();
            for (target, _) in tgts {
                let err = errors.get(&target.to_string()).unwrap();
                println!("  {}", err);
            }
            let total_err = diagram.error();
            println!(
                "  total_err {} grads: [{}]",
                Dual::fmt(&total_err.clone().v(), 3),
                (-total_err.clone()).d().iter().map(|d| Dual::fmt(d, 3)).collect::<Vec<String>>().join(", "),
            );
            assert_relative_eq!(total_err, expected_err, epsilon = 1e-3);
            println!();
            diagram = diagram.step(0.1);
            println!();
        }
    }
}