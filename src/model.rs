use std::{cell::RefCell, rc::Rc, collections::HashMap};

use crate::{diagram::Diagram, circle::Split};


type Step = Rc<RefCell<Diagram>>;
struct Model {
    steps: Vec<Step>,
    repeat_idx: Option<usize>,
    min_idx: usize,
    min_step: Step,
    error: f64,
}

impl Model {
    fn new(inputs: Vec<Split>, targets: HashMap<String, f64>, step_size: f64, max_steps: usize) -> Model {
        let mut diagram = Diagram::new(inputs, targets, None);
        let mut steps = Vec::<Step>::new();
        let mut min_step: Option<(usize, Step)> = None;
        let mut repeat_idx: Option<usize> = None;
        for idx in 0..max_steps {
            let nxt = diagram.step(step_size);
            let nxt = Rc::new(RefCell::new(nxt));
            let nxt_err = nxt.borrow().error.re;
            if min_step.clone().map(|(_, cur_min)| nxt_err < cur_min.borrow().error.re).unwrap_or(true) {
                min_step = Some((idx, nxt.clone()));
            }
            for (prv_idx, prv) in steps.iter().enumerate().rev() {
                if prv.borrow().error.re == nxt_err &&
                    prv
                    .borrow()
                    .shapes
                    .shapes
                    .iter()
                    .zip(nxt.borrow().shapes.shapes.iter())
                    .all(|(a, b)| a == b)
                {
                    println!("  (repeated)");
                    repeat_idx = Some(prv_idx);
                    break;
                }
            }
            if repeat_idx.is_some() {
                break;
            }
            steps.push(nxt);
        }
        let (min_idx, min_step) = min_step.unwrap();
        let error = min_step.borrow().error.re;

        Model { steps, min_idx, min_step, repeat_idx, error }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::{dual::Dual, circle::Circle, r2::R2};

    use super::*;

    fn simple() {
        // 2 Circles, only the 2nd circle's x and r can move:
        // - 1st circle is fixed unit circle at origin
        // - 2nd circle's center is fixed on x-axis (y=0)
        // This is the minimal degrees of freedom that can reach any target (relative) distribution between {"0*", "*1", and "01"} (1st circle size, 2nd circle size, intersection size).
        let inputs = vec![
            (Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. }, [ vec![0., 0.], vec![0., 0.], vec![0., 0.], ]),
            (Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. }, [ vec![1., 0.], vec![0., 0.], vec![0., 1.], ]),
        ];
        // Fizz Buzz example:
        let tgts = [
            ("0*", 1. /  3.),  // Fizz (multiples of 3)
            ("*1", 1. /  5.),  // Buzz (multiples of 5)
            ("01", 1. / 15.),  // Fizz Buzz (multiples of both 3 and 5)
        ];
        let targets: HashMap::<_, _> = tgts.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let model = Model::new(inputs, targets, 0.1, 100);
        // let mut diagram = Diagram::new(inputs, targets, None);
        let os = env::consts::OS;
        let expected_errs = vec![
            (( 1.000, 1.000 ), 0.386    , ( 0.426, -1.456 )),  // Step 0
            (( 1.019, 0.937 ), 0.284    , ( 0.388, -1.520 )),  // Step 1
            (( 1.034, 0.875 ), 0.1828   , ( 0.349, -1.570 )),  // Step 2
            (( 1.048, 0.814 ), 0.0819   , ( 0.309, -1.606 )),  // Step 3
            (( 1.057, 0.765 ), 0.03516  , ( 0.470,  1.054 )),  // Step 4
            (( 1.070, 0.793 ), 0.0412   , ( 0.747, -0.551 )),  // Step 5
            (( 1.105, 0.767 ), 0.0116   , ( 0.445,  1.047 )),  // Step 6
            (( 1.109, 0.776 ), 0.00307  , ( 0.707, -0.541 )),  // Step 7
            (( 1.112, 0.774 ), 9.942e-4 , ( 0.440,  1.046 )),  // Step 8
            (( 1.112, 0.775 ), 2.018e-4 , ( 0.704, -0.540 )),  // Step 9
            (( 1.113, 0.775 ), 6.550e-5 , ( 0.440,  1.046 )),  // Step 10
            (( 1.113, 0.775 ), 1.301e-5 , ( 0.704, -0.540 )),  // Step 11
            (( 1.113, 0.775 ), 4.220e-6 , ( 0.440,  1.046 )),  // Step 12
            (( 1.113, 0.775 ), 8.365e-7 , ( 0.704, -0.540 )),  // Step 13
            (( 1.113, 0.775 ), 2.714e-7 , ( 0.440,  1.046 )),  // Step 14
            (( 1.113, 0.775 ), 5.380e-8 , ( 0.704, -0.540 )),  // Step 15
            (( 1.113, 0.775 ), 1.746e-8 , ( 0.440,  1.046 )),  // Step 16
            (( 1.113, 0.775 ), 3.460e-9 , ( 0.704, -0.540 )),  // Step 17
            (( 1.113, 0.775 ), 1.123e-9 , ( 0.440,  1.046 )),  // Step 18
            (( 1.113, 0.775 ), 2.226e-10, ( 0.264, -1.586 )),  // Step 19
            (( 1.113, 0.775 ), 1.328e-10, ( 0.704, -0.540 )),  // Step 20
            (( 1.113, 0.775 ), 1.759e-10, ( 0.440,  1.046 )),  // Step 21
            (( 1.113, 0.775 ), 3.488e-11, ( 0.704, -0.540 )),  // Step 22
            (( 1.113, 0.775 ), 1.132e-11, (-0.264,  1.586 )),  // Step 23
            (( 1.113, 0.775 ), 4.562e-12, ( 0.440,  1.046 )),  // Step 24
            (( 1.113, 0.775 ), 5.466e-12, ( 0.704, -0.540 )),  // Step 25
            (( 1.113, 0.775 ), 1.774e-12, (-0.264,  1.586 )),  // Step 26
        ];
        let linux = vec![
            (( 1.113, 0.775 ), 7.149e-13, ( 0.704, -0.540 )),  // Step 27
            (( 1.113, 0.775 ), 9.468e-13, (-0.264,  1.586 )),  // Step 28
            (( 1.113, 0.775 ), 3.816e-13, ( 0.704, -0.540 )),  // Step 29
            (( 1.113, 0.775 ), 5.055e-13, ( 0.440,  1.046 )),  // Step 30
            (( 1.113, 0.775 ), 1.003e-13, ( 0.264, -1.586 )),  // Step 31
            (( 1.113, 0.775 ), 5.990e-14, ( 0.440,  1.046 )),  // Step 32
            (( 1.113, 0.775 ), 7.155e-14, ( 0.704, -0.540 )),  // Step 33
            (( 1.113, 0.775 ), 2.318e-14, ( 0.440,  1.046 )),  // Step 34
            (( 1.113, 0.775 ), 4.829e-15, ( 0.264, -1.586 )),  // Step 35
            (( 1.113, 0.775 ), 3.081e-15, ( 0.704, -0.540 )),  // Step 36
            (( 1.113, 0.775 ), 4.247e-15, (-0.264,  1.586 )),  // Step 37
            (( 1.113, 0.775 ), 1.582e-15, ( 0.704, -0.540 )),  // Step 38
            (( 1.113, 0.775 ), 2.137e-15, (-0.264,  1.586 )),  // Step 39
            (( 1.113, 0.775 ), 8.882e-16, ( 0.704, -0.540 )),  // Step 40
            (( 1.113, 0.775 ), 1.166e-15, (-0.264,  1.586 )),  // Step 41
            (( 1.113, 0.775 ), 4.718e-16, ( 0.440,  1.046 )),  // Step 42
            (( 1.113, 0.775 ), 4.996e-16, ( 0.704, -0.540 )),  // Step 43
            (( 1.113, 0.775 ), 2.498e-16, ( 0.704, -0.540 )),  // Step 44
            (( 1.113, 0.775 ), 7.494e-16, (-0.264,  1.586 )),  // Step 45
            (( 1.113, 0.775 ), 2.498e-16, (-0.440, -1.046 )),  // Step 46
            (( 1.113, 0.775 ), 1.388e-16, (-0.264,  1.586 )),  // Step 47
            (( 1.113, 0.775 ), 1.943e-16, (-0.440, -1.046 )),  // Step 48
            (( 1.113, 0.775 ), 1.388e-16, (-0.264,  1.586 )),  // Step 49
            (( 1.113, 0.775 ), 1.943e-16, (-0.440, -1.046 )),  // Step 50
            (( 1.113, 0.775 ), 1.388e-16, (-0.264,  1.586 )),  // Step 51
            (( 1.113, 0.775 ), 1.943e-16, (-0.440, -1.046 )),  // Step 52
            (( 1.113, 0.775 ), 1.388e-16, (-0.264,  1.586 )),  // Step 53
        ];
        let macos = vec![
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
            (( 1.113, 0.775 ), 6.106e-16, (-0.264,  1.586 )),  // Step 45
            (( 1.113, 0.775 ), 2.220e-16, (-0.704,  0.540 )),  // Step 46
            (( 1.113, 0.775 ), 4.996e-16, ( 0.264, -1.586 )),  // Step 47
            (( 1.113, 0.775 ), 1.388e-16, ( 0.704, -0.540 )),  // Step 48
            (( 1.113, 0.775 ), 6.106e-16, (-0.264,  1.586 )),  // Step 49
        ];
        let expected_errs = [
            expected_errs,
            if os == "macos" { macos } else { linux },
        ].concat();

        let steps = model.steps;

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
            let err = total_err.v();
            let err_str = if err < 0.001 {
                format!("{:.3e}", err)
            } else {
                format!("{:.5}", err)
            };
            println!("Actual: (( {:.3}, {:.3} ), {: <9}, ({}, {} )),  // Step {}", c1.c.x, c1.r, err_str, Dual::fmt(&grads[0], 3), Dual::fmt(&grads[1], 3), idx);
            println!();
        };

        let generate_vals = env::var("GENERATE_VALS").map(|s| s.parse::<usize>().unwrap()).ok();
        match generate_vals {
            Some(n) => {
                for (idx, step) in steps.iter().enumerate() {
                    print_step(&step.borrow(), idx);
                }
            }
            None => {
                assert_eq!(steps.len(), expected_errs.len());
                for (idx, (step, ((e_cx, e_cr), e_err, (e_grad0, e_grad1)))) in steps.iter().zip(expected_errs.iter()).enumerate() {
                    println!("Step {}", idx);
                    let errors = &step.borrow().errors;
                    for (target, _) in tgts {
                        let err = errors.get(&target.to_string()).unwrap();
                        println!("  {}", err);
                    }
                    let total_err = step.borrow().error.clone();
                    println!("Err: {:?}", total_err);
                    let c1 = step.borrow().shapes.shapes[1];

                    assert_relative_eq!(c1.c.x, *e_cx, epsilon = 1e-3);
                    assert_relative_eq!(c1.r, *e_cr, epsilon = 1e-3);

                    let expected_err = Dual::new(*e_err, vec![-*e_grad0, -*e_grad1]);
                    assert_relative_eq!(total_err, expected_err, epsilon = 1e-3);

                    let actual_err = total_err.v();
                    let abs_err_diff = (*e_err - actual_err).abs();
                    let relative_err = abs_err_diff / *e_err;
                    assert!(relative_err < 1e-3, "relative_err {} >= 1e-3: actual err {}, expected {}", relative_err, actual_err, *e_err);

                    print_step(&step.borrow(), idx);
                }
            }
        }
    }
}