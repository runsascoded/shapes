use std::{collections::HashMap, fmt::Display};

use crate::{circle::{Circle, Split, Duals}, shapes::Shapes, dual::D, r2::R2, areas::Areas};


pub struct Diagram {
    inputs: Vec<Split>,
    shapes: Shapes,
    targets: HashMap<String, f64>,
    total_target_area: f64,
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
    pub fn new(inputs: Vec<Split>, targets: HashMap<String, f64>, total_target_area: Option<f64>) -> Diagram {
        let shapes = Shapes::new(&inputs);
        let all_key = String::from_utf8(vec![b'*'; shapes.len()]).unwrap();
        let total_target_area = total_target_area.unwrap_or_else(|| {
            let mut expanded_target = targets.clone();
            Areas::expand(&mut expanded_target);
            expanded_target.get(&all_key).unwrap().clone()
        });
        Diagram { inputs, shapes, targets, total_target_area: total_target_area.clone() }
    }

    pub fn n(&self) -> usize {
        self.shapes.len()
    }

    pub fn key_contained_by(k0: &String, k1: &String) -> bool {
        k0.chars().zip(k1.chars()).all(|(a, b)| a == '-' || a == b || b == '*')
    }

    pub fn errors(&self) -> HashMap<&String, Error> {
        let all_key = String::from_utf8(vec![b'*'; self.n()]).unwrap();
        let none_key = String::from_utf8(vec![b'-'; self.n()]).unwrap();
        let shapes = &self.shapes;
        let targets = &self.targets;
        let total_area = shapes.area(&all_key);
        let total_target_area = self.total_target_area;
        targets.iter().filter_map(|(key, target_area)| {
            if key == &none_key {
                None
            } else {
                let actual_area = shapes.area(key);
                let target_frac = target_area / total_target_area;
                let actual_frac = actual_area.clone() / &total_area;
                let error = (actual_frac.clone() - target_frac).abs();
                Some((key, Error {
                    key: key.clone(),
                    actual_area, total_area: total_area.clone(),
                    target_area: target_area.clone(), total_target_area: total_target_area.clone(),
                    actual_frac,
                    target_frac,
                    error,
                }))
            }
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
            // println!("  updates {:?}", updates);
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
        Diagram::new(new_inputs, self.targets.clone(), Some(self.total_target_area))
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
        let targets: HashMap::<_, _> = tgts.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        // Areas::expand(&mut targets);
        let mut diagram = Diagram::new(inputs, targets, None);
        let expected_errs = [
            Dual::new(0.386, vec![ -0.426,  1.456, ]),
            Dual::new(0.284, vec![ -0.388,  1.520, ]),
            Dual::new(0.183, vec![ -0.349,  1.570, ]),
            Dual::new(0.082, vec![ -0.309,  1.606, ]),
            Dual::new(0.046, vec![ -0.470, -1.053, ]),
            // Dual::new(0.046, vec![ -0.470, -1.053, ]),
        ];
        let print_step = |diagram: &Diagram, idx: usize| {
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
            total_err
        };
        for (idx, expected_err) in expected_errs.iter().enumerate() {
            let total_err = print_step(&diagram, idx);
            assert_relative_eq!(total_err, expected_err, epsilon = 1e-3);
            println!();
            diagram = diagram.step(0.1);
            println!();
        }
        print_step(&diagram, expected_errs.len());
    }
}