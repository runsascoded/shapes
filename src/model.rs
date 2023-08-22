use std::{cell::RefCell, rc::Rc, collections::HashMap};

use log::{info, debug};

use crate::{diagram::{Diagram, Targets}, circle::Split};


type Step = Rc<RefCell<Diagram>>;
pub struct Model {
    pub steps: Vec<Step>,
    pub repeat_idx: Option<usize>,
    pub min_idx: usize,
    pub min_step: Step,
    pub error: f64,
}

impl Model {
    pub fn new(inputs: Vec<Split>, targets: Targets, step_size: f64, max_steps: usize) -> Model {
        let mut diagram = Rc::new(RefCell::new(Diagram::new(inputs, targets, None)));
        let mut steps = Vec::<Step>::new();
        let mut min_step: Option<(usize, Step)> = None;
        let mut repeat_idx: Option<usize> = None;
        for idx in 0..max_steps {
            steps.push(diagram.clone());
            debug!("Step {}:", idx);
            let nxt_diagram = diagram.borrow_mut().step(step_size);
            let nxt = Rc::new(RefCell::new(nxt_diagram));
            let nxt_err = nxt.borrow().error.re;
            if min_step.clone().map(|(_, cur_min)| nxt_err < cur_min.borrow().error.re).unwrap_or(true) {
                min_step = Some((idx, nxt.clone()));
            }
            for (prv_idx, prv) in steps.iter().enumerate().rev() {
                let prv_err = prv.borrow().error.re;
                if prv_err == nxt_err &&
                    prv
                    .borrow()
                    .shapes
                    .shapes
                    .iter()
                    .zip(nxt.borrow().shapes.shapes.iter())
                    .all(|(a, b)| {
                        //println!("Checking {} vs {}", a, b);
                        a == b
                    })
                {
                    info!("  Step {} matches step {}: {}", idx + 1, prv_idx, prv_err);
                    repeat_idx = Some(prv_idx);
                    steps.push(nxt.clone());
                    break;
                }
            }
            if repeat_idx.is_some() {
                break;
            }
            diagram = nxt;
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
    use test_log::test;

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
        let macos = vec![
            (( 1.000, 1.000 ), 0.38587  , ( 0.426, -1.456 )),  // Step 0
            (( 1.028, 0.904 ), 0.23030  , ( 0.367, -1.548 )),  // Step 1
            (( 1.051, 0.807 ), 0.06871  , ( 0.303, -1.608 )),  // Step 2
            (( 1.064, 0.739 ), 0.05927  , ( 0.470,  1.050 )),  // Step 3
            (( 1.088, 0.793 ), 0.03644  , ( 0.283, -1.592 )),  // Step 4
            (( 1.094, 0.757 ), 0.02606  , ( 0.452,  1.048 )),  // Step 5
            (( 1.105, 0.781 ), 0.01283  , ( 0.270, -1.588 )),  // Step 6
            (( 1.107, 0.769 ), 0.00866  , ( 0.444,  1.047 )),  // Step 7
            (( 1.110, 0.777 ), 0.00393  , ( 0.266, -1.586 )),  // Step 8
            (( 1.111, 0.773 ), 0.00259  , ( 0.441,  1.046 )),  // Step 9
            (( 1.112, 0.775 ), 0.00113  , ( 0.264, -1.586 )),  // Step 10
            (( 1.112, 0.774 ), 7.350e-4 , ( 0.440,  1.046 )),  // Step 11
            (( 1.112, 0.775 ), 3.120e-4 , ( 0.264, -1.586 )),  // Step 12
            (( 1.112, 0.774 ), 2.003e-4 , ( 0.440,  1.046 )),  // Step 13
            (( 1.113, 0.775 ), 8.283e-5 , ( 0.264, -1.586 )),  // Step 14
            (( 1.113, 0.775 ), 5.252e-5 , ( 0.440,  1.046 )),  // Step 15
            (( 1.113, 0.775 ), 2.107e-5 , ( 0.264, -1.586 )),  // Step 16
            (( 1.113, 0.775 ), 1.316e-5 , ( 0.440,  1.046 )),  // Step 17
            (( 1.113, 0.775 ), 5.083e-6 , ( 0.264, -1.586 )),  // Step 18
            (( 1.113, 0.775 ), 3.107e-6 , ( 0.440,  1.046 )),  // Step 19
            (( 1.113, 0.775 ), 1.136e-6 , ( 0.264, -1.586 )),  // Step 20
            (( 1.113, 0.775 ), 6.901e-7 , (-0.264,  1.586 )),  // Step 21
            (( 1.113, 0.775 ), 4.292e-7 , ( 0.704, -0.540 )),  // Step 22
            (( 1.113, 0.775 ), 1.335e-7 , ( 0.440,  1.046 )),  // Step 23
            (( 1.113, 0.775 ), 9.656e-8 , ( 0.264, -1.586 )),  // Step 24
            (( 1.113, 0.775 ), 7.470e-8 , ( 0.440,  1.046 )),  // Step 25
            (( 1.113, 0.775 ), 4.289e-8 , ( 0.264, -1.586 )),  // Step 26
            (( 1.113, 0.775 ), 3.110e-8 , ( 0.440,  1.046 )),  // Step 27
            (( 1.113, 0.775 ), 1.623e-8 , ( 0.264, -1.586 )),  // Step 28
            (( 1.113, 0.775 ), 1.139e-8 , ( 0.440,  1.046 )),  // Step 29
            (( 1.113, 0.775 ), 5.622e-9 , ( 0.264, -1.586 )),  // Step 30
            (( 1.113, 0.775 ), 3.862e-9 , ( 0.440,  1.046 )),  // Step 31
            (( 1.113, 0.775 ), 1.836e-9 , ( 0.264, -1.586 )),  // Step 32
            (( 1.113, 0.775 ), 1.242e-9 , ( 0.440,  1.046 )),  // Step 33
            (( 1.113, 0.775 ), 5.729e-10, ( 0.264, -1.586 )),  // Step 34
            (( 1.113, 0.775 ), 3.826e-10, ( 0.440,  1.046 )),  // Step 35
            (( 1.113, 0.775 ), 1.722e-10, ( 0.264, -1.586 )),  // Step 36
            (( 1.113, 0.775 ), 1.137e-10, ( 0.440,  1.046 )),  // Step 37
            (( 1.113, 0.775 ), 5.002e-11, ( 0.264, -1.586 )),  // Step 38
            (( 1.113, 0.775 ), 3.270e-11, ( 0.440,  1.046 )),  // Step 39
            (( 1.113, 0.775 ), 1.406e-11, ( 0.264, -1.586 )),  // Step 40
            (( 1.113, 0.775 ), 9.093e-12, ( 0.440,  1.046 )),  // Step 41
            (( 1.113, 0.775 ), 3.820e-12, ( 0.264, -1.586 )),  // Step 42
            (( 1.113, 0.775 ), 2.442e-12, ( 0.440,  1.046 )),  // Step 43
            (( 1.113, 0.775 ), 9.995e-13, ( 0.264, -1.586 )),  // Step 44
            (( 1.113, 0.775 ), 6.305e-13, ( 0.440,  1.046 )),  // Step 45
            (( 1.113, 0.775 ), 2.497e-13, ( 0.264, -1.586 )),  // Step 46
            (( 1.113, 0.775 ), 1.547e-13, ( 0.440,  1.046 )),  // Step 47
            (( 1.113, 0.775 ), 5.873e-14, ( 0.264, -1.586 )),  // Step 48
            (( 1.113, 0.775 ), 3.575e-14, (-0.264,  1.586 )),  // Step 49
            (( 1.113, 0.775 ), 2.315e-14, ( 0.704, -0.540 )),  // Step 50
            (( 1.113, 0.775 ), 7.994e-15, ( 0.440,  1.046 )),  // Step 51
            (( 1.113, 0.775 ), 5.163e-15, ( 0.264, -1.586 )),  // Step 52
            (( 1.113, 0.775 ), 3.914e-15, ( 0.440,  1.046 )),  // Step 53
            (( 1.113, 0.775 ), 2.220e-15, ( 0.264, -1.586 )),  // Step 54
            (( 1.113, 0.775 ), 1.499e-15, (-0.264,  1.586 )),  // Step 55
            (( 1.113, 0.775 ), 1.027e-15, ( 0.704, -0.540 )),  // Step 56
            (( 1.113, 0.775 ), 3.608e-16, ( 0.440,  1.046 )),  // Step 57
            (( 1.113, 0.775 ), 2.220e-16, (-0.704,  0.540 )),  // Step 58
            (( 1.113, 0.775 ), 4.996e-16, ( 0.264, -1.586 )),  // Step 59
            (( 1.113, 0.775 ), 3.608e-16, ( 0.440,  1.046 )),  // Step 60 == Step 57, break
        ];
        let linux = macos;

        let expected_errs = if os == "macos" { macos } else { linux };

        let steps = model.steps;

        let print_step = |diagram: &Diagram, idx: usize| {
            let total_err = diagram.error.clone();
            let c1 = diagram.shapes.shapes[1];
            let grads = (-total_err.clone()).d();
            let err = total_err.v();
            let err_str = if err < 0.001 {
                format!("{:.3e}", err)
            } else {
                format!("{:.5}", err)
            };
            println!("(( {:.3}, {:.3} ), {: <9}, ({}, {} )),  // Step {}", c1.c.x, c1.r, err_str, Dual::fmt(&grads[0], 3), Dual::fmt(&grads[1], 3), idx);
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
                    let c1 = step.borrow().shapes.shapes[1];
                    assert_relative_eq!(c1.c.x, *e_cx, epsilon = 1e-3);
                    assert_relative_eq!(c1.r, *e_cr, epsilon = 1e-3);

                    let total_err = step.borrow().error.clone();
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