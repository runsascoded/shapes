

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
    pub fn grad_size(&self) -> usize {
        self.steps[0].grad_size()
    }
}

#[cfg(test)]
mod tests {
    use std::{env, collections::HashMap};

    use crate::{dual::Dual, circle::Circle, fmt::Fmt, r2::R2, shape::Shape, ellipses::xyrr::XYRR};

    use super::*;
    use test_log::test;

    pub struct ExpectedStep {
        vals: Vec<f64>,
        err: f64,
        grads: Vec<f64>,
    }
    impl ExpectedStep {
        pub fn dual(&self) -> Dual {
            Dual::new(self.err, self.grads.clone())
        }
    }
    impl<const N: usize> From<([f64; N], f64, [f64; N])> for ExpectedStep {
        fn from((vals, err, grads): ([f64; N], f64, [f64; N])) -> Self {
            ExpectedStep { vals: vals.to_vec(), err, grads: grads.to_vec() }
        }
    }

    pub trait To<T1> {
        fn to(self) -> T1;
    }
    impl<T0, T1: From<T0>> To<Vec<T1>> for Vec<T0> {
        fn to(self: Self) -> Vec<T1> {
            self.into_iter().map(|x| x.into()).collect()
        }
    }

    fn get_actual(step: &Diagram, getters: &Vec<CoordGetter>) -> ExpectedStep {
        let error = step.error.clone();
        let err = error.v();
        let mut vals: Vec<f64> = Vec::new();
        let mut grads: Vec<f64> = Vec::new();
        getters.iter().enumerate().for_each(|(coord_idx, getter)| {
            let val: f64 = getter.0(step.clone());
            vals.push(val);
            let error_d = error.d();
            let err_grad = error_d[coord_idx];
            grads.push(-err_grad);
        });
        ExpectedStep { vals, err, grads }
    }

    fn print_step(diagram: &Diagram, idx: usize, getters: &Vec<CoordGetter>) {
        let actual = get_actual(diagram, getters);
        let vals_str = actual.vals.iter().map(|g| g.s(3)).collect::<Vec<_>>().join(",");
        let grads_str = actual.grads.iter().map(|g| g.s(3)).collect::<Vec<_>>().join(", ");

        let total_err = diagram.error.clone();
        let err = total_err.v();
        let err_str = if err < 0.001 {
            format!("{:.3e}", err)
        } else {
            format!("{:.5}", err)
        };
        println!("([{} ], {: <9}, [{} ]),  // Step {}", vals_str, err_str, grads_str, idx);
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

    fn test(inputs: Vec<Input>, targets: Vec<(&str, f64)>, expecteds: Vec<ExpectedStep>) {
        let targets: HashMap<_, _> = targets.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut model = Model::new(inputs.clone(), targets);
        model.train(0.8, 100);

        let mut coord_getters: Vec<(usize, CoordGetter)> = inputs.iter().enumerate().flat_map(
            |(shape_idx, (shape, duals))| match shape {
                Shape::Circle(_) => {
                    let getters = [
                        |c: Circle<f64>| c.c.x,
                        |c: Circle<f64>| c.c.y,
                        |c: Circle<f64>| c.r,
                    ];
                    getters.into_iter().zip(duals).filter_map(|(getter, dual)| {
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
                Shape::XYRR(_) => {
                    let getters = [
                        |e: XYRR<f64>| e.c.x,
                        |e: XYRR<f64>| e.c.y,
                        |e: XYRR<f64>| e.r.x,
                        |e: XYRR<f64>| e.r.y,
                    ];
                    getters.into_iter().zip(duals).filter_map(|(getter, dual)| {
                        is_one_hot(dual).map(|grad_idx| (
                            grad_idx,
                            CoordGetter(
                                Box::new(move |step: Diagram| match step.shapes()[shape_idx].clone() {
                                    Shape::XYRR(e) => getter(e),
                                    _ => panic!("Expected XYRR at idx {}", shape_idx),
                                })
                            )
                        )
                    )
                    }).collect::<Vec<_>>()
                },
        }).collect();
        coord_getters.sort_by(|(a, _), (b, _)| a.cmp(b));
        assert_eq!(model.grad_size(), coord_getters.len());
        let coord_getters: Vec<_> = coord_getters.into_iter().map(|(_, getter)| getter).collect();
        // println!("coord_getters: {:?}", coord_getters.iter().map(|(idx, _)| idx).collect::<Vec<_>>());

        let steps = model.steps;
        let generate_vals = env::var("GENERATE_VALS").map(|s| s.parse::<usize>().unwrap()).ok();
        match generate_vals {
            Some(_) => {
                for (idx, step) in steps.iter().enumerate() {
                    print_step(&step, idx, &coord_getters);
                }
            }
            None => {
                assert_eq!(steps.len(), expecteds.len());
                for (idx, (step, expected)) in steps.iter().zip(expecteds.iter()).enumerate() {
                    let actual = get_actual(step, &coord_getters);
                    assert_eq!(actual.vals.len(), expected.vals.len());
                    for (a_val, e_val) in actual.vals.iter().zip(expected.vals.iter()) {
                        assert_relative_eq!(a_val, e_val, epsilon = 1e-3);
                    }
                    assert_relative_eq!(actual.dual(), expected.dual(), epsilon = 1e-3);

                    // assert_relative_eq trivially false-positives when the provided "expected" value is larger than the "actual" value, because it checks |A - B| / max(A, B).
                    // TODO: factor out and use a better relative-equality macro.
                    let a_err = actual.err;
                    let e_err = expected.err;
                    let abs_err_diff = (e_err - a_err).abs();
                    let relative_err = abs_err_diff / e_err;
                    assert!(relative_err < 1e-3, "relative_err {} >= 1e-3: actual err {}, expected {}", relative_err, a_err, e_err);

                    print_step(&step, idx, &coord_getters);
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
        let targets = vec![
            ("0*", 1. /  3.),  // Fizz (multiples of 3)
            ("*1", 1. /  5.),  // Buzz (multiples of 5)
            ("01", 1. / 15.),  // Fizz Buzz (multiples of both 3 and 5)
        ];

        let os = env::consts::OS;
        let macos = vec![
            ([ 1.000, 1.000 ], 0.38587  , [ 0.426, -1.456 ]),  // Step 0
            ([ 1.087, 0.704 ], 0.10719  , [-0.228,  1.615 ]),  // Step 1
            ([ 1.075, 0.789 ], 0.03504  , [ 0.741, -0.551 ]),  // Step 2
            ([ 1.097, 0.772 ], 0.00966  , [ 0.448,  1.048 ]),  // Step 3
            ([ 1.100, 0.779 ], 0.01118  , [ 0.716, -0.544 ]),  // Step 4
            ([ 1.107, 0.774 ], 0.00336  , [ 0.443,  1.047 ]),  // Step 5
            ([ 1.108, 0.776 ], 0.00378  , [ 0.708, -0.541 ]),  // Step 6
            ([ 1.111, 0.774 ], 0.001167 , [ 0.441,  1.046 ]),  // Step 7
            ([ 1.111, 0.775 ], 0.001298 , [ 0.705, -0.540 ]),  // Step 8
            ([ 1.112, 0.774 ], 4.052e-4 , [ 0.440,  1.046 ]),  // Step 9
            ([ 1.112, 0.775 ], 4.491e-4 , [ 0.704, -0.540 ]),  // Step 10
            ([ 1.112, 0.775 ], 1.406e-4 , [ 0.440,  1.046 ]),  // Step 11
            ([ 1.112, 0.775 ], 1.557e-4 , [ 0.704, -0.540 ]),  // Step 12
            ([ 1.113, 0.775 ], 4.880e-5 , [ 0.440,  1.046 ]),  // Step 13
            ([ 1.113, 0.775 ], 5.400e-5 , [ 0.704, -0.540 ]),  // Step 14
            ([ 1.113, 0.775 ], 1.693e-5 , [ 0.440,  1.046 ]),  // Step 15
            ([ 1.113, 0.775 ], 1.874e-5 , [ 0.704, -0.540 ]),  // Step 16
            ([ 1.113, 0.775 ], 5.877e-6 , [ 0.440,  1.046 ]),  // Step 17
            ([ 1.113, 0.775 ], 6.501e-6 , [ 0.704, -0.540 ]),  // Step 18
            ([ 1.113, 0.775 ], 2.039e-6 , [ 0.440,  1.046 ]),  // Step 19
            ([ 1.113, 0.775 ], 2.256e-6 , [ 0.704, -0.540 ]),  // Step 20
            ([ 1.113, 0.775 ], 7.076e-7 , [ 0.440,  1.046 ]),  // Step 21
            ([ 1.113, 0.775 ], 7.828e-7 , [ 0.704, -0.540 ]),  // Step 22
            ([ 1.113, 0.775 ], 2.456e-7 , [ 0.440,  1.046 ]),  // Step 23
            ([ 1.113, 0.775 ], 2.716e-7 , [ 0.704, -0.540 ]),  // Step 24
            ([ 1.113, 0.775 ], 8.521e-8 , [ 0.440,  1.046 ]),  // Step 25
            ([ 1.113, 0.775 ], 9.427e-8 , [ 0.704, -0.540 ]),  // Step 26
            ([ 1.113, 0.775 ], 2.957e-8 , [ 0.440,  1.046 ]),  // Step 27
            ([ 1.113, 0.775 ], 3.271e-8 , [ 0.704, -0.540 ]),  // Step 28
            ([ 1.113, 0.775 ], 1.026e-8 , [ 0.440,  1.046 ]),  // Step 29
            ([ 1.113, 0.775 ], 1.135e-8 , [ 0.704, -0.540 ]),  // Step 30
            ([ 1.113, 0.775 ], 3.561e-9 , [ 0.440,  1.046 ]),  // Step 31
            ([ 1.113, 0.775 ], 3.939e-9 , [ 0.704, -0.540 ]),  // Step 32
            ([ 1.113, 0.775 ], 1.236e-9 , [ 0.440,  1.046 ]),  // Step 33
            ([ 1.113, 0.775 ], 1.367e-9 , [ 0.704, -0.540 ]),  // Step 34
            ([ 1.113, 0.775 ], 4.288e-10, [ 0.440,  1.046 ]),  // Step 35
            ([ 1.113, 0.775 ], 4.743e-10, [ 0.704, -0.540 ]),  // Step 36
            ([ 1.113, 0.775 ], 1.488e-10, [ 0.440,  1.046 ]),  // Step 37
            ([ 1.113, 0.775 ], 1.646e-10, [ 0.704, -0.540 ]),  // Step 38
            ([ 1.113, 0.775 ], 5.163e-11, [ 0.440,  1.046 ]),  // Step 39
            ([ 1.113, 0.775 ], 5.712e-11, [ 0.704, -0.540 ]),  // Step 40
            ([ 1.113, 0.775 ], 1.792e-11, [ 0.440,  1.046 ]),  // Step 41
            ([ 1.113, 0.775 ], 1.982e-11, [ 0.704, -0.540 ]),  // Step 42
            ([ 1.113, 0.775 ], 6.217e-12, [ 0.440,  1.046 ]),  // Step 43
            ([ 1.113, 0.775 ], 6.878e-12, [ 0.704, -0.540 ]),  // Step 44
            ([ 1.113, 0.775 ], 2.157e-12, [ 0.440,  1.046 ]),  // Step 45
            ([ 1.113, 0.775 ], 2.387e-12, [ 0.704, -0.540 ]),  // Step 46
            ([ 1.113, 0.775 ], 7.487e-13, [ 0.440,  1.046 ]),  // Step 47
            ([ 1.113, 0.775 ], 8.283e-13, [ 0.704, -0.540 ]),  // Step 48
            ([ 1.113, 0.775 ], 2.598e-13, [ 0.440,  1.046 ]),  // Step 49
            ([ 1.113, 0.775 ], 2.872e-13, [ 0.704, -0.540 ]),  // Step 50
            ([ 1.113, 0.775 ], 9.023e-14, [ 0.440,  1.046 ]),  // Step 51
            ([ 1.113, 0.775 ], 9.948e-14, [ 0.704, -0.540 ]),  // Step 52
            ([ 1.113, 0.775 ], 3.120e-14, [ 0.440,  1.046 ]),  // Step 53
            ([ 1.113, 0.775 ], 3.486e-14, [ 0.704, -0.540 ]),  // Step 54
            ([ 1.113, 0.775 ], 1.105e-14, [ 0.440,  1.046 ]),  // Step 55
            ([ 1.113, 0.775 ], 1.199e-14, [ 0.704, -0.540 ]),  // Step 56
            ([ 1.113, 0.775 ], 3.691e-15, [ 0.440,  1.046 ]),  // Step 57
            ([ 1.113, 0.775 ], 4.219e-15, [ 0.704, -0.540 ]),  // Step 58
            ([ 1.113, 0.775 ], 1.360e-15, [ 0.440,  1.046 ]),  // Step 59
            ([ 1.113, 0.775 ], 1.499e-15, [ 0.704, -0.540 ]),  // Step 60
            ([ 1.113, 0.775 ], 6.661e-16, [ 0.440,  1.046 ]),  // Step 61
            ([ 1.113, 0.775 ], 6.939e-16, [ 0.704, -0.540 ]),  // Step 62
            ([ 1.113, 0.775 ], 5.551e-17, [ 0.440,  1.046 ]),  // Step 63
            ([ 1.113, 0.775 ], 5.551e-17, [ 0.440,  1.046 ]),  // Step 64
        ];
        let mut linux = macos.clone();
        linux[53] = ([ 1.113, 0.775 ], 3.114e-14, [ 0.440,  1.046 ]);  // Step 53

        let expecteds = if os == "macos" { macos } else { linux };

        test(inputs, targets, expecteds.to());
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
        let targets = vec![
            ("0*", 1. /  3.),  // Fizz (multiples of 3)
            ("*1", 1. /  5.),  // Buzz (multiples of 5)
            ("01", 1. / 15.),  // Fizz Buzz (multiples of both 3 and 5)
        ];

        let expecteds: Vec<ExpectedStep> = vec![
            ([ 1.000, 1.000, 1.000 ], 0.38587  , [ 0.426, -0.675, -0.781 ]),  // Step 0
            ([ 1.118, 0.813, 0.784 ], 0.03856  , [ 0.275, -0.842, -0.736 ]),  // Step 1
            ([ 1.125, 0.791, 0.764 ], 0.00566  , [-0.431, -0.407, -0.637 ]),  // Step 2
            ([ 1.123, 0.789, 0.761 ], 0.002334 , [-0.692,  0.435,  0.106 ]),  // Step 3
            ([ 1.121, 0.790, 0.761 ], 0.001593 , [-0.433, -0.407, -0.637 ]),  // Step 4
            ([ 1.121, 0.789, 0.760 ], 7.121e-4 , [-0.694,  0.435,  0.107 ]),  // Step 5
            ([ 1.120, 0.789, 0.760 ], 4.439e-4 , [-0.434, -0.407, -0.637 ]),  // Step 6
            ([ 1.120, 0.789, 0.760 ], 2.187e-4 , [-0.694,  0.435,  0.108 ]),  // Step 7
            ([ 1.120, 0.789, 0.760 ], 1.225e-4 , [-0.434, -0.407, -0.637 ]),  // Step 8
            ([ 1.120, 0.789, 0.760 ], 6.783e-5 , [-0.695,  0.435,  0.108 ]),  // Step 9
            ([ 1.120, 0.789, 0.760 ], 3.337e-5 , [-0.434, -0.407, -0.637 ]),  // Step 10
            ([ 1.120, 0.789, 0.760 ], 2.127e-5 , [-0.695,  0.435,  0.108 ]),  // Step 11
            ([ 1.120, 0.789, 0.760 ], 8.940e-6 , [-0.434, -0.407, -0.637 ]),  // Step 12
            ([ 1.120, 0.789, 0.760 ], 6.746e-6 , [-0.695,  0.435,  0.108 ]),  // Step 13
            ([ 1.120, 0.789, 0.760 ], 2.338e-6 , [-0.434, -0.407, -0.637 ]),  // Step 14
            ([ 1.120, 0.789, 0.760 ], 2.165e-6 , [-0.695,  0.435,  0.108 ]),  // Step 15
            ([ 1.120, 0.789, 0.760 ], 7.335e-7 , [-0.695,  0.435,  0.108 ]),  // Step 16
            ([ 1.120, 0.789, 0.760 ], 5.502e-7 , [-0.434, -0.407, -0.637 ]),  // Step 17
            ([ 1.120, 0.789, 0.760 ], 2.203e-7 , [-0.695,  0.435,  0.108 ]),  // Step 18
            ([ 1.120, 0.789, 0.760 ], 1.545e-7 , [-0.434, -0.407, -0.637 ]),  // Step 19
            ([ 1.120, 0.789, 0.760 ], 6.668e-8 , [-0.695,  0.435,  0.108 ]),  // Step 20
            ([ 1.120, 0.789, 0.760 ], 4.311e-8 , [-0.434, -0.407, -0.637 ]),  // Step 21
            ([ 1.120, 0.789, 0.760 ], 2.038e-8 , [-0.695,  0.435,  0.108 ]),  // Step 22
            ([ 1.120, 0.789, 0.760 ], 1.193e-8 , [-0.434, -0.407, -0.637 ]),  // Step 23
            ([ 1.120, 0.789, 0.760 ], 6.290e-9 , [-0.695,  0.435,  0.108 ]),  // Step 24
            ([ 1.120, 0.789, 0.760 ], 3.268e-9 , [-0.434, -0.407, -0.637 ]),  // Step 25
            ([ 1.120, 0.789, 0.760 ], 1.963e-9 , [-0.695,  0.435,  0.108 ]),  // Step 26
            ([ 1.120, 0.789, 0.760 ], 8.820e-10, [-0.434, -0.407, -0.637 ]),  // Step 27
            ([ 1.120, 0.789, 0.760 ], 6.198e-10, [-0.695,  0.435,  0.108 ]),  // Step 28
            ([ 1.120, 0.789, 0.760 ], 2.332e-10, [-0.434, -0.407, -0.637 ]),  // Step 29
            ([ 1.120, 0.789, 0.760 ], 1.980e-10, [-0.695,  0.435,  0.108 ]),  // Step 30
            ([ 1.120, 0.789, 0.760 ], 6.707e-11, [-0.695,  0.435,  0.108 ]),  // Step 31
            ([ 1.120, 0.789, 0.760 ], 5.617e-11, [-0.434, -0.407, -0.637 ]),  // Step 32
            ([ 1.120, 0.789, 0.760 ], 1.984e-11, [-0.695,  0.435,  0.108 ]),  // Step 33
            ([ 1.120, 0.789, 0.760 ], 1.591e-11, [-0.434, -0.407, -0.637 ]),  // Step 34
            ([ 1.120, 0.789, 0.760 ], 5.903e-12, [-0.695,  0.435,  0.108 ]),  // Step 35
            ([ 1.120, 0.789, 0.760 ], 4.492e-12, [-0.434, -0.407, -0.637 ]),  // Step 36
            ([ 1.120, 0.789, 0.760 ], 1.769e-12, [-0.695,  0.435,  0.108 ]),  // Step 37
            ([ 1.120, 0.789, 0.760 ], 1.263e-12, [-0.434, -0.407, -0.637 ]),  // Step 38
            ([ 1.120, 0.789, 0.760 ], 5.343e-13, [-0.695,  0.435,  0.108 ]),  // Step 39
            ([ 1.120, 0.789, 0.760 ], 3.529e-13, [-0.434, -0.407, -0.637 ]),  // Step 40
            ([ 1.120, 0.789, 0.760 ], 1.630e-13, [-0.695,  0.435,  0.108 ]),  // Step 41
            ([ 1.120, 0.789, 0.760 ], 9.798e-14, [-0.434, -0.407, -0.637 ]),  // Step 42
            ([ 1.120, 0.789, 0.760 ], 4.999e-14, [-0.695,  0.435,  0.108 ]),  // Step 43
            ([ 1.120, 0.789, 0.760 ], 2.676e-14, [-0.434, -0.407, -0.637 ]),  // Step 44
            ([ 1.120, 0.789, 0.760 ], 1.579e-14, [-0.695,  0.435,  0.108 ]),  // Step 45
            ([ 1.120, 0.789, 0.760 ], 7.494e-15, [-0.434, -0.407, -0.637 ]),  // Step 46
            ([ 1.120, 0.789, 0.760 ], 4.802e-15, [-0.695,  0.435,  0.108 ]),  // Step 47
            ([ 1.120, 0.789, 0.760 ], 2.026e-15, [-0.434, -0.407, -0.637 ]),  // Step 48
            ([ 1.120, 0.789, 0.760 ], 1.665e-15, [-0.695,  0.435,  0.108 ]),  // Step 49
            ([ 1.120, 0.789, 0.760 ], 6.384e-16, [-0.695,  0.435,  0.108 ]),  // Step 50
            ([ 1.120, 0.789, 0.760 ], 2.498e-16, [-0.434, -0.407, -0.637 ]),  // Step 51
            ([ 1.120, 0.789, 0.760 ], 1.388e-16, [ 0.000,  0.000, -0.000 ]),  // Step 52
            ([ 1.120, 0.789, 0.760 ], 3.886e-16, [ 0.695, -0.435, -0.108 ]),  // Step 53
            ([ 1.120, 0.789, 0.760 ], 6.384e-16, [-0.695,  0.435,  0.108 ]),  // Step 54
            ([ 1.120, 0.789, 0.760 ], 4.996e-16, [ 0.261, -0.842, -0.745 ]),  // Step 55
            ([ 1.120, 0.789, 0.760 ], 2.220e-16, [ 0.695, -0.435, -0.108 ]),  // Step 56
            ([ 1.120, 0.789, 0.760 ], 3.608e-16, [-0.695,  0.435,  0.108 ]),  // Step 57
            ([ 1.120, 0.789, 0.760 ], 2.220e-16, [ 0.695, -0.435, -0.108 ]),  // Step 58
        ].to();
        test(inputs, targets, expecteds)
    }
}
