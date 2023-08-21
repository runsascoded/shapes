use std::{collections::HashMap, fmt::Display};

use crate::{circle::{Circle, Split, Duals}, shapes::Shapes, dual::D, r2::R2, areas::Areas};


type Targets = HashMap<String, f64>;
type Errors = HashMap<String, Error>;

pub struct Diagram {
    inputs: Vec<Split>,
    shapes: Shapes,
    targets: Targets,
    total_target_area: f64,
    errors: Errors,
    error: D,
}

#[derive(Clone, Debug)]
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
            self.target_area, self.total_target_area, self.target_frac,
            self.actual_area.v(), self.total_area.v(), self.actual_frac.v(),
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
        let errors = Self::compute_errors(&shapes, &targets, total_target_area);
        let error = errors.values().into_iter().map(|e| &e.error).cloned().collect::<Vec<D>>().into_iter().sum();
        Diagram { inputs, shapes, targets, total_target_area: total_target_area.clone(), errors, error }
    }

    pub fn n(&self) -> usize {
        self.shapes.len()
    }

    pub fn key_contained_by(k0: &String, k1: &String) -> bool {
        k0.chars().zip(k1.chars()).all(|(a, b)| a == '-' || a == b || b == '*')
    }

    pub fn compute_errors(shapes: &Shapes, targets: &Targets, total_target_area: f64) -> Errors {
        let n = shapes.len();
        let all_key = String::from_utf8(vec![b'*'; n]).unwrap();
        let none_key = String::from_utf8(vec![b'-'; n]).unwrap();
        let total_area = shapes.area(&all_key);
        targets.iter().filter_map(|(key, target_area)| {
            if key == &none_key {
                None
            } else {
                let actual_area = shapes.area(key);
                let target_frac = target_area / total_target_area;
                let actual_frac = actual_area.clone() / &total_area;
                let error = (actual_frac.clone() - target_frac).abs();
                Some((
                    key.clone(),
                    Error {
                        key: key.clone(),
                        actual_area, total_area: total_area.clone(),
                        target_area: target_area.clone(), total_target_area: total_target_area.clone(),
                        actual_frac,
                        target_frac,
                        error,
                    }
                ))
            }
        }).collect()
    }

    pub fn duals(&self) -> Vec<Duals> {
        self.inputs.iter().map(|(_, duals)| duals.clone()).collect()
    }

    pub fn step(&self, step_size: f64) -> Diagram {
        let error = self.error.clone();
        let error_size = error.v();
        let grad_vec = (-error).d();
        // let max_error = grad_vec.iter().map(|(_, e)| e.error.v()).unwrap().1.error.v();
        let clamped_step_size = f64::min(error_size, step_size);
        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>();
        let step_vec = grad_vec.iter().map(|d| d / magnitude * clamped_step_size).collect::<Vec<f64>>();
        // println!("step_vec {:?}", step_vec);
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
        println!("Applying: step_size {:.3}, updates [{}]:", clamped_step_size, step_vec.iter().map(|x| format!("{:.3}", x)).collect::<Vec<String>>().join(", "));
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
        // 2 Circles, only the 2nd circle's x and r can move:
        // - 1st circle is fixed unit circle at origin
        // - 2nd circle's center is fixed on x-axis (y=0)
        // This is the minimal degrees of freedom that can reach any target (relative) distribution between {"0*", "*1", and "01"} (1st circle size, 2nd circle size, intersection size).
        let inputs = vec![
            (Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. }, [ vec![0., 0.], vec![0., 0.], vec![0., 0.], ]),
            (Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. }, [ vec![1., 0.], vec![0., 0.], vec![0., 1.], ]),
        ];
        // FizzBuzz example:
        let tgts = [
            ("0*", 1. /  3.),  // Fizz (multiples of 3)
            ("*1", 1. /  5.),  // Buzz (multiples of 5)
            ("01", 1. / 15.),  // FizzBuzz (multiples of both 3 and 5)
        ];
        let targets: HashMap::<_, _> = tgts.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut diagram = Diagram::new(inputs, targets, None);
        let expected_errs = [
            (( 1.000, 1.000 ), 0.386,     ( 0.426, -1.456 )),  // Step 0
            (( 1.019, 0.937 ), 0.284,     ( 0.388, -1.520 )),  // Step 1
            (( 1.034, 0.875 ), 0.1828,    ( 0.349, -1.570 )),  // Step 2
            (( 1.048, 0.814 ), 0.0819,    ( 0.309, -1.606 )),  // Step 3
            (( 1.057, 0.765 ), 0.03516,   ( 0.470,  1.054 )),  // Step 4
            (( 1.070, 0.793 ), 0.0412,    ( 0.747, -0.551 )),  // Step 5
            (( 1.105, 0.767 ), 0.0116,    ( 0.445,  1.047 )),  // Step 6
            (( 1.109, 0.776 ), 0.00307,   ( 0.707, -0.541 )),  // Step 7
            (( 1.112, 0.774 ), 9.94e-4,   ( 0.440,  1.046 )),  // Step 8
            (( 1.112, 0.775 ), 2.018e-4,  ( 0.704, -0.540 )),  // Step 9
            (( 1.113, 0.775 ), 6.55e-5,   ( 0.440,  1.046 )),  // Step 10
            (( 1.113, 0.775 ), 1.30e-5,   ( 0.704, -0.540 )),  // Step 11
            (( 1.113, 0.775 ), 4.22e-6,   ( 0.440,  1.046 )),  // Step 12
            (( 1.113, 0.775 ), 8.37e-7,   ( 0.704, -0.540 )),  // Step 13
            (( 1.113, 0.775 ), 2.714e-7,  ( 0.440,  1.046 )),  // Step 14
            (( 1.113, 0.775 ), 5.380e-8,  ( 0.704, -0.540 )),  // Step 15
            (( 1.113, 0.775 ), 1.746e-8,  ( 0.440,  1.046 )),  // Step 16
            (( 1.113, 0.775 ), 3.460e-9,  ( 0.704, -0.540 )),  // Step 17
            (( 1.113, 0.775 ), 1.123e-9,  ( 0.440,  1.046 )),  // Step 18
            (( 1.113, 0.775 ), 2.226e-10, ( 0.264, -1.586 )),  // Step 19
            (( 1.113, 0.775 ), 1.328e-10, ( 0.704, -0.540 )),  // Step 20
            (( 1.113, 0.775 ), 1.759e-10, ( 0.440,  1.046 )),  // Step 21
            (( 1.113, 0.775 ), 3.488e-11, ( 0.704, -0.540 )),  // Step 22
            (( 1.113, 0.775 ), 1.132e-11, (-0.264,  1.586 )),  // Step 23
            (( 1.113, 0.775 ), 4.562e-12, ( 0.440,  1.046 )),  // Step 24
            (( 1.113, 0.775 ), 5.466e-12, ( 0.704, -0.540 )),  // Step 25
            (( 1.113, 0.775 ), 1.774e-12, (-0.264,  1.586 )),  // Step 26
            (( 1.113, 0.775 ), 7.147e-13, ( 0.704, -0.540 )),  // Step 27
            (( 1.113, 0.775 ), 9.467e-13, ( 0.440,  1.046 )),  // Step 28
            (( 1.113, 0.775 ), 1.879e-13, ( 0.704, -0.540 )),  // Step 29
            (( 1.113, 0.775 ), 6.090e-14, (-0.264,  1.586 )),  // Step 30
            (( 1.113, 0.775 ), 2.459e-14, ( 0.440,  1.046 )),  // Step 31
            (( 1.113, 0.775 ), 2.928e-14, ( 0.704, -0.540 )),  // Step 32
            (( 1.113, 0.775 ), 9.465e-15, (-0.264,  1.586 )),  // Step 33
            (( 1.113, 0.775 ), 3.719e-15, ( 0.704, -0.540 )),  // Step 34
            (( 1.113, 0.775 ), 4.968e-15, ( 0.440,  1.046 )),  // Step 35
            (( 1.113, 0.775 ), 1.027e-15, ( 0.704, -0.540 )),  // Step 36
            (( 1.113, 0.775 ), 6.106e-16, (-0.264,  1.586 )),  // Step 37
            (( 1.113, 0.775 ), 2.220e-16, (-0.704,  0.540 )),  // Step 38
            (( 1.113, 0.775 ), 4.996e-16, ( 0.264, -1.586 )),  // Step 39
            (( 1.113, 0.775 ), 1.388e-16, ( 0.704, -0.540 )),  // Step 40
            (( 1.113, 0.775 ), 6.106e-16, (-0.264,  1.586 )),  // Step 41
            (( 1.113, 0.775 ), 2.220e-16, (-0.704,  0.540 )),  // Step 42
            (( 1.113, 0.775 ), 4.996e-16, ( 0.264, -1.586 )),  // Step 43
            (( 1.113, 0.775 ), 1.388e-16, ( 0.704, -0.540 )),  // Step 44
        ];
        let print_step = |diagram: &Diagram, idx: usize| {
            println!("Step {}", idx);
            let errors = &diagram.errors;
            for (target, _) in tgts {
                let err = errors.get(&target.to_string()).unwrap();
                println!("  {}", err);
            }
            let total_err = diagram.error.clone();
            println!("Err: {:?}", total_err);
            let c1 = diagram.shapes.shapes[1];
            let grads = (-total_err.clone()).d();
            println!("Actual: (( {:.3}, {:.3} ), {:.3e}, ({}, {} )),  // Step {}", c1.c.x, c1.r, total_err.v(), Dual::fmt(&grads[0], 3), Dual::fmt(&grads[1], 3), idx);
            total_err
        };
        for (idx, ((e_cx, e_cr), e_err, (e_grad0, e_grad1))) in expected_errs.iter().enumerate() {
            let total_err = print_step(&diagram, idx);
            let c1 = diagram.shapes.shapes[1];
            assert_relative_eq!(c1.c.x, *e_cx, epsilon = 1e-3);
            assert_relative_eq!(c1.r, *e_cr, epsilon = 1e-3);
            let expected_err = Dual::new(*e_err, vec![-*e_grad0, -*e_grad1]);
            assert_relative_eq!(total_err, expected_err, epsilon = 1e-3);
            let actual_err = total_err.v();
            let abs_err_diff = (*e_err - actual_err).abs();
            let relative_err = abs_err_diff / *e_err;
            assert!(relative_err < 1e-3, "relative_err {} >= 1e-3: actual err {}, expected {}", relative_err, actual_err, *e_err);
            println!();
            diagram = diagram.step(0.1);
            println!();
        }
        print_step(&diagram, expected_errs.len());
    }
}