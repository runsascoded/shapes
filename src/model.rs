

use log::{info, debug};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{diagram::{Diagram, Targets}, shape::Input};

#[derive(Debug, Clone, Tsify, Serialize, Deserialize)]
pub struct Model {
    pub steps: Vec<Diagram>,
    pub repeat_idx: Option<usize>,
    pub min_idx: usize,
    pub min_error: f64,
}

impl Model {
    pub fn new(inputs: Vec<Input>, targets: Targets) -> Model {
        let diagram = Diagram::new(inputs, targets, None);
        let min_error = (&diagram).error.re.clone();
        let mut steps = Vec::<Diagram>::new();
        steps.push(diagram);
        let repeat_idx: Option<usize> = None;
        Model { steps, min_idx: 0, repeat_idx, min_error }
    }
    pub fn train(&mut self, max_step_error_ratio: f64, max_steps: usize) {
        let num_steps = self.steps.len().clone();
        let mut diagram = self.steps[num_steps - 1].clone();
        for idx in 0..max_steps {
            let step_idx = idx + num_steps;
            debug!("Step {}:", step_idx);
            let nxt = diagram.step(max_step_error_ratio);
            let nxt_err = nxt.error.re;
            let min_step = &self.steps[self.min_idx];
            if nxt_err < min_step.error.re {
                self.min_idx = step_idx;
                self.min_error = nxt_err;
            }
            self.steps.push(nxt.clone());

            // Check whether the newest step (`nxt`) is a repeat of a previous step:
            for (prv_idx, prv) in self.steps.iter().enumerate().rev().skip(1) {
                let prv_err = prv.error.re;
                if prv_err == nxt_err &&
                    prv
                    .shapes()
                    .iter()
                    .zip(nxt.shapes().iter())
                    .all(|(a, b)| {
                        //println!("Checking {} vs {}", a, b);
                        a == b
                    })
                {
                    info!("  Step {} matches step {}: {}", step_idx, prv_idx, prv_err);
                    self.repeat_idx = Some(prv_idx);
                    break;
                }
            }
            // If so, break
            if self.repeat_idx.is_some() {
                break;
            }
            diagram = nxt;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, collections::HashMap};

    use crate::{dual::Dual, circle::Circle, fmt::Fmt, r2::R2, shape::Shape, ellipses::xyrr::XYRR};

    use super::*;
    use test_log::test;

    fn print_step(diagram: &Diagram, idx: usize) {
        let total_err = diagram.error.clone();
        let c1 = match diagram.shapes()[1] {
            Shape::Circle(c) => c,
            _ => panic!("Expected Circle"),
        };
        let grads = (-total_err.clone()).d();
        let err = total_err.v();
        let err_str = if err < 0.001 {
            format!("{:.3e}", err)
        } else {
            format!("{:.5}", err)
        };
        println!("(( {:.3}, {:.3} ), {: <9}, ({}, {} )),  // Step {}", c1.c.x, c1.r, err_str, grads[0].s(3), grads[1].s(3), idx);
    }

    fn is_one_hot(v: &Vec<f64>) -> Option<usize> {
        let mut idx = None;
        for (i, x) in v.iter().enumerate() {
            if *x == 1. {
                if idx.is_some() {
                    return None;
                }
                idx = Some(i);
            } else if *x != 0. {
                return None;
            }
        }
        idx
    }

    pub struct CoordGetter(pub Box<dyn Fn(Diagram) -> f64>);

    fn test<Expected>(inputs: Vec<Input>, targets: Vec<(&str, f64)>, expecteds: Vec<Expected>) {
        let targets: HashMap<_, _> = targets.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut model = Model::new(inputs, targets);
        model.train(0.8, 100);

        let steps = model.steps;

        let filtered: Vec<(usize, CoordGetter)> = inputs.iter().enumerate().flat_map(
            |(shape_idx, (shape, duals))| match shape {
                Shape::Circle(Circle { idx, c: R2 { x, y }, r }) => {
                    let getters = [
                        |c: Circle<f64>| c.c.x,
                        |c: Circle<f64>| c.c.y,
                        |c: Circle<f64>| c.r,
                    ];
                    getters.iter().zip(duals).filter_map(|(getter, dual)| {
                        is_one_hot(dual).map(|grad_idx| (
                            grad_idx,
                            CoordGetter(
                                Box::new(move |step: Diagram| match step.shapes()[shape_idx] {
                                    Shape::Circle(c) => getter(c),
                                    _ => panic!("Expected Circle at idx {}", shape_idx),
                                })
                            )
                        )
                    )
                    }).collect::<Vec<_>>()
                },
                Shape::XYRR(e) => {
                    let getters = [
                        |e: XYRR<f64>| e.c.x,
                        |e: XYRR<f64>| e.c.y,
                        |e: XYRR<f64>| e.r.x,
                        |e: XYRR<f64>| e.r.y,
                    ];
                    getters.iter().zip(duals).filter_map(|(getter, dual)| {
                        is_one_hot(dual).map(|grad_idx| (
                            grad_idx,
                            CoordGetter(
                                Box::new(move |step: Diagram| match step.shapes()[shape_idx] {
                                    Shape::XYRR(e) => getter(e),
                                    _ => panic!("Expected XYRR at idx {}", shape_idx),
                                })
                            )
                        )
                    )
                    }).collect::<Vec<_>>()
                },
        }).collect();

        let generate_vals = env::var("GENERATE_VALS").map(|s| s.parse::<usize>().unwrap()).ok();
        match generate_vals {
            Some(_) => {
                for (idx, step) in steps.iter().enumerate() {
                    print_step(&step, idx);
                }
            }
            None => {
                assert_eq!(steps.len(), expecteds.len());
                for (idx, (step, expected)) in steps.iter().zip(expecteds.iter()).enumerate() {
                    let c1 = match step.shapes()[1] {
                        Shape::Circle(c) => c,
                        _ => panic!("Expected Circle"),
                    };
                    assert_relative_eq!(c1.c.x, *e_cx, epsilon = 1e-3);
                    assert_relative_eq!(c1.r, *e_cr, epsilon = 1e-3);

                    let total_err = (&step).error.clone();
                    let expected_err = Dual::new(*e_err, vec![-*e_grad0, -*e_grad1]);
                    assert_relative_eq!(total_err, expected_err, epsilon = 1e-3);

                    let actual_err = total_err.v();
                    let abs_err_diff = (*e_err - actual_err).abs();
                    let relative_err = abs_err_diff / *e_err;
                    assert!(relative_err < 1e-3, "relative_err {} >= 1e-3: actual err {}, expected {}", relative_err, actual_err, *e_err);

                    print_step(&step, idx);
                }
            }
        }
    }

    #[test]
    fn fizz_buzz_circles() {
        // 2 Circles, only the 2nd circle's x and r can move:
        // - 1st circle is fixed unit circle at origin
        // - 2nd circle's center is fixed on x-axis (y=0)
        // This is the minimal degrees of freedom that can reach any target (relative) distribution between {"0*", "*1", and "01"} (1st circle size, 2nd circle size, intersection size).
        let inputs: Vec<Input> = vec![
            (Shape::Circle(Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. }), vec![ vec![0., 0.], vec![0., 0.], vec![0., 0.], ]),
            (Shape::Circle(Circle { idx: 1, c: R2 { x: 1., y: 0. }, r: 1. }), vec![ vec![1., 0.], vec![0., 0.], vec![0., 1.], ]),
        ];
        // Fizz Buzz example:
        let targets = [
            ("0*", 1. /  3.),  // Fizz (multiples of 3)
            ("*1", 1. /  5.),  // Buzz (multiples of 5)
            ("01", 1. / 15.),  // Fizz Buzz (multiples of both 3 and 5)
        ];
        let targets: HashMap<_, _> = targets.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut model = Model::new(inputs, targets);
        model.train(0.8, 100);

        let os = env::consts::OS;
        let macos = vec![
            (( 1.000, 1.000 ), 0.38587  , ( 0.426, -1.456 )),  // Step 0
            (( 1.087, 0.704 ), 0.10719  , (-0.228,  1.615 )),  // Step 1
            (( 1.075, 0.789 ), 0.03504  , ( 0.741, -0.551 )),  // Step 2
            (( 1.097, 0.772 ), 0.00966  , ( 0.448,  1.048 )),  // Step 3
            (( 1.100, 0.779 ), 0.01118  , ( 0.716, -0.544 )),  // Step 4
            (( 1.107, 0.774 ), 0.00336  , ( 0.443,  1.047 )),  // Step 5
            (( 1.108, 0.776 ), 0.00378  , ( 0.708, -0.541 )),  // Step 6
            (( 1.111, 0.774 ), 0.001167 , ( 0.441,  1.046 )),  // Step 7
            (( 1.111, 0.775 ), 0.001298 , ( 0.705, -0.540 )),  // Step 8
            (( 1.112, 0.774 ), 4.052e-4 , ( 0.440,  1.046 )),  // Step 9
            (( 1.112, 0.775 ), 4.491e-4 , ( 0.704, -0.540 )),  // Step 10
            (( 1.112, 0.775 ), 1.406e-4 , ( 0.440,  1.046 )),  // Step 11
            (( 1.112, 0.775 ), 1.557e-4 , ( 0.704, -0.540 )),  // Step 12
            (( 1.113, 0.775 ), 4.880e-5 , ( 0.440,  1.046 )),  // Step 13
            (( 1.113, 0.775 ), 5.400e-5 , ( 0.704, -0.540 )),  // Step 14
            (( 1.113, 0.775 ), 1.693e-5 , ( 0.440,  1.046 )),  // Step 15
            (( 1.113, 0.775 ), 1.874e-5 , ( 0.704, -0.540 )),  // Step 16
            (( 1.113, 0.775 ), 5.877e-6 , ( 0.440,  1.046 )),  // Step 17
            (( 1.113, 0.775 ), 6.501e-6 , ( 0.704, -0.540 )),  // Step 18
            (( 1.113, 0.775 ), 2.039e-6 , ( 0.440,  1.046 )),  // Step 19
            (( 1.113, 0.775 ), 2.256e-6 , ( 0.704, -0.540 )),  // Step 20
            (( 1.113, 0.775 ), 7.076e-7 , ( 0.440,  1.046 )),  // Step 21
            (( 1.113, 0.775 ), 7.828e-7 , ( 0.704, -0.540 )),  // Step 22
            (( 1.113, 0.775 ), 2.456e-7 , ( 0.440,  1.046 )),  // Step 23
            (( 1.113, 0.775 ), 2.716e-7 , ( 0.704, -0.540 )),  // Step 24
            (( 1.113, 0.775 ), 8.521e-8 , ( 0.440,  1.046 )),  // Step 25
            (( 1.113, 0.775 ), 9.427e-8 , ( 0.704, -0.540 )),  // Step 26
            (( 1.113, 0.775 ), 2.957e-8 , ( 0.440,  1.046 )),  // Step 27
            (( 1.113, 0.775 ), 3.271e-8 , ( 0.704, -0.540 )),  // Step 28
            (( 1.113, 0.775 ), 1.026e-8 , ( 0.440,  1.046 )),  // Step 29
            (( 1.113, 0.775 ), 1.135e-8 , ( 0.704, -0.540 )),  // Step 30
            (( 1.113, 0.775 ), 3.561e-9 , ( 0.440,  1.046 )),  // Step 31
            (( 1.113, 0.775 ), 3.939e-9 , ( 0.704, -0.540 )),  // Step 32
            (( 1.113, 0.775 ), 1.236e-9 , ( 0.440,  1.046 )),  // Step 33
            (( 1.113, 0.775 ), 1.367e-9 , ( 0.704, -0.540 )),  // Step 34
            (( 1.113, 0.775 ), 4.288e-10, ( 0.440,  1.046 )),  // Step 35
            (( 1.113, 0.775 ), 4.743e-10, ( 0.704, -0.540 )),  // Step 36
            (( 1.113, 0.775 ), 1.488e-10, ( 0.440,  1.046 )),  // Step 37
            (( 1.113, 0.775 ), 1.646e-10, ( 0.704, -0.540 )),  // Step 38
            (( 1.113, 0.775 ), 5.163e-11, ( 0.440,  1.046 )),  // Step 39
            (( 1.113, 0.775 ), 5.712e-11, ( 0.704, -0.540 )),  // Step 40
            (( 1.113, 0.775 ), 1.792e-11, ( 0.440,  1.046 )),  // Step 41
            (( 1.113, 0.775 ), 1.982e-11, ( 0.704, -0.540 )),  // Step 42
            (( 1.113, 0.775 ), 6.217e-12, ( 0.440,  1.046 )),  // Step 43
            (( 1.113, 0.775 ), 6.878e-12, ( 0.704, -0.540 )),  // Step 44
            (( 1.113, 0.775 ), 2.157e-12, ( 0.440,  1.046 )),  // Step 45
            (( 1.113, 0.775 ), 2.387e-12, ( 0.704, -0.540 )),  // Step 46
            (( 1.113, 0.775 ), 7.487e-13, ( 0.440,  1.046 )),  // Step 47
            (( 1.113, 0.775 ), 8.283e-13, ( 0.704, -0.540 )),  // Step 48
            (( 1.113, 0.775 ), 2.598e-13, ( 0.440,  1.046 )),  // Step 49
            (( 1.113, 0.775 ), 2.872e-13, ( 0.704, -0.540 )),  // Step 50
            (( 1.113, 0.775 ), 9.023e-14, ( 0.440,  1.046 )),  // Step 51
            (( 1.113, 0.775 ), 9.948e-14, ( 0.704, -0.540 )),  // Step 52
            (( 1.113, 0.775 ), 3.120e-14, ( 0.440,  1.046 )),  // Step 53
            (( 1.113, 0.775 ), 3.486e-14, ( 0.704, -0.540 )),  // Step 54
            (( 1.113, 0.775 ), 1.105e-14, ( 0.440,  1.046 )),  // Step 55
            (( 1.113, 0.775 ), 1.199e-14, ( 0.704, -0.540 )),  // Step 56
            (( 1.113, 0.775 ), 3.691e-15, ( 0.440,  1.046 )),  // Step 57
            (( 1.113, 0.775 ), 4.219e-15, ( 0.704, -0.540 )),  // Step 58
            (( 1.113, 0.775 ), 1.360e-15, ( 0.440,  1.046 )),  // Step 59
            (( 1.113, 0.775 ), 1.499e-15, ( 0.704, -0.540 )),  // Step 60
            (( 1.113, 0.775 ), 6.661e-16, ( 0.440,  1.046 )),  // Step 61
            (( 1.113, 0.775 ), 6.939e-16, ( 0.704, -0.540 )),  // Step 62
            (( 1.113, 0.775 ), 5.551e-17, ( 0.440,  1.046 )),  // Step 63
            (( 1.113, 0.775 ), 5.551e-17, ( 0.440,  1.046 )),  // Step 64
        ];
        let mut linux = macos.clone();
        linux[53] = (( 1.113, 0.775 ), 3.114e-14, ( 0.440,  1.046 ));  // Step 53

        let expected_errs = if os == "macos" { macos } else { linux };

        let steps = model.steps;

        let generate_vals = env::var("GENERATE_VALS").map(|s| s.parse::<usize>().unwrap()).ok();
        match generate_vals {
            Some(_) => {
                for (idx, step) in steps.iter().enumerate() {
                    print_step(&step, idx);
                }
            }
            None => {
                assert_eq!(steps.len(), expected_errs.len());
                for (idx, (step, ((e_cx, e_cr), e_err, (e_grad0, e_grad1)))) in steps.iter().zip(expected_errs.iter()).enumerate() {
                    let c1 = match step.shapes()[1] {
                        Shape::Circle(c) => c,
                        _ => panic!("Expected Circle"),
                    };
                    assert_relative_eq!(c1.c.x, *e_cx, epsilon = 1e-3);
                    assert_relative_eq!(c1.r, *e_cr, epsilon = 1e-3);

                    let total_err = (&step).error.clone();
                    let expected_err = Dual::new(*e_err, vec![-*e_grad0, -*e_grad1]);
                    assert_relative_eq!(total_err, expected_err, epsilon = 1e-3);

                    let actual_err = total_err.v();
                    let abs_err_diff = (*e_err - actual_err).abs();
                    let relative_err = abs_err_diff / *e_err;
                    assert!(relative_err < 1e-3, "relative_err {} >= 1e-3: actual err {}, expected {}", relative_err, actual_err, *e_err);

                    print_step(&step, idx);
                }
            }
        }
    }

    #[test]
    fn fizz_buzz_circle_ellipse() {
        let inputs: Vec<Input> = vec![
            (
                Shape::Circle(Circle { idx: 0, c: R2 { x: 0., y: 0. }, r: 1. }),
                vec![
                    vec![0., 0., 0.],
                    vec![0., 0., 0.],
                    vec![0., 0., 0.],
                ]),
            (
                Shape::XYRR(XYRR { idx: 1, c: R2 { x: 1., y: 0. }, r: R2 { x: 1., y: 1. } }),
                vec![
                    vec![1., 0., 0.],
                    vec![0., 0., 0.],
                    vec![0., 1., 0.],
                    vec![0., 0., 1.],
                ]),
        ];
        // Fizz Buzz example:
        let targets = [
            ("0*", 1. /  3.),  // Fizz (multiples of 3)
            ("*1", 1. /  5.),  // Buzz (multiples of 5)
            ("01", 1. / 15.),  // Fizz Buzz (multiples of both 3 and 5)
        ];
        let targets: HashMap<_, _> = targets.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut model = Model::new(inputs, targets);
        model.train(0.7, 100);

    }
}
