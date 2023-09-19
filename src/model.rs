

use log::{info, debug, warn};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{step::{Step, Targets}, shape::Input};

#[derive(Debug, Clone, Tsify, Serialize, Deserialize)]
pub struct Model {
    pub steps: Vec<Step>,
    pub repeat_idx: Option<usize>,
    pub min_idx: usize,
    pub min_error: f64,
}

impl Model {
    pub fn new(inputs: Vec<Input>, targets: Targets) -> Model {
        let step = Step::new(inputs, targets, None);
        let min_error = (&step).error.re.clone();
        let mut steps = Vec::<Step>::new();
        steps.push(step);
        let repeat_idx: Option<usize> = None;
        Model { steps, min_idx: 0, repeat_idx, min_error }
    }
    pub fn train(&mut self, max_step_error_ratio: f64, max_steps: usize) {
        let num_steps = self.steps.len().clone();
        let mut step = self.steps[num_steps - 1].clone();
        for idx in 0..max_steps {
            let step_idx = idx + num_steps;
            debug!("Step {}:", step_idx);
            let nxt = step.step(max_step_error_ratio);
            let nxt_err = nxt.error.re;
            if nxt_err.is_nan() {
                warn!("NaN err at step {}: {:?}", step_idx, nxt);
                self.repeat_idx = Some(step_idx);
                break;
            }
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
            step = nxt;
        }
    }
    pub fn grad_size(&self) -> usize {
        self.steps[0].grad_size()
    }
}

#[cfg(test)]
mod tests {
    use std::{env, collections::BTreeMap, path::Path};
    use polars::prelude::*;

    use crate::{dual::{Dual, is_one_hot, d_fns}, circle::Circle, r2::R2, shape::Shape, to::To, ellipses::xyrr::XYRR};

    use super::*;
    use test_log::test;

    static FIZZ_BUZZ: [(&str, f64); 3] = [
        ("0*", 1. /  3.),  // Fizz (multiples of 3)
        ("*1", 1. /  5.),  // Buzz (multiples of 5)
        ("01", 1. / 15.),  // Fizz Buzz (multiples of both 3 and 5)
    ];

    static FIZZ_BUZZ_BAZZ: [(&str, f64); 7] = [
        ( "0**", 35. ),  // 1 / 3
        ( "*1*", 21. ),  // 1 / 5
        ( "**2", 15. ),  // 1 / 7
        ( "01*",  7. ),  // 1 / 15
        ( "0*2",  5. ),  // 1 / 21
        ( "*12",  3. ),  // 1 / 35
        ( "012",  1. ),  // 1 / 105
    ];

    static VARIANT_CALLERS: [ (&str, f64); 15 ] = [
        ( "0---", 633. ),
        ( "-1--", 618. ),
        ( "--2-", 187. ),
        ( "---3", 319. ),
        ( "01--", 112. ),
        ( "0-2-",   0. ),
        ( "0--3",  13. ),
        ( "-12-",  14. ),
        ( "-1-3",  55. ),
        ( "--23",  21. ),
        ( "012-",   1. ),
        ( "01-3",  17. ),
        ( "0-23",   0. ),
        ( "-123",   9. ),
        ( "0123",  36. ),
    ];

    /// Convenience struct for test cases, containing:
    /// - shape-coordinate values
    /// - overall error
    /// - error gradient (with respect to each shape-coordinate)
    #[derive(Clone, Debug, PartialEq)]
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

    fn get_actual(step: &Step, getters: &Vec<CoordGetter>) -> ExpectedStep {
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

    use AnyValue::Float64;

    fn load_expecteds(path: &str) -> (DataFrame, Vec<ExpectedStep>) {
        let mut df = CsvReader::from_path(path).unwrap().has_header(false).finish().unwrap();
        let num_columns = df.shape().1;
        if num_columns % 2 == 0 {
            panic!("Expected odd number of columns, got {}", num_columns);
        }
        let n = (num_columns - 1) / 2;
        df.as_single_chunk_par();
        let mut iters = df.iter().map(|s| s.iter()).collect::<Vec<_>>();

        let mut expecteds: Vec<ExpectedStep> = Vec::new();
        for _ in 0..df.height() {
            let mut vals: Vec<f64> = Vec::new();
            let mut grads: Vec<f64> = Vec::new();
            let mut err: Option<f64> = None;
            for (j, iter) in &mut iters.iter_mut().enumerate() {
                let v = match iter.next().expect("should have as many iterations as rows") {
                    Float64(f) => f,
                    _ => panic!("Expected Float64, got {:?}", iter.next().unwrap()),
                };
                if j < n {
                    vals.push(v);
                } else if j == n {
                    err = Some(v)
                } else {
                    grads.push(v);
                }
            }
            expecteds.push(ExpectedStep { vals, err: err.unwrap(), grads });
        }
        (df, expecteds)
    }

    fn write_expecteds(path: &str, expecteds: Vec<ExpectedStep>) -> Result<DataFrame, PolarsError> {
        let mut cols: Vec<Vec<f64>> = vec![];
        let n = expecteds[0].vals.len();
        let num_columns = 1 + n + n;
        for _ in 0..num_columns {
            cols.push(vec![]);
        }
        let path = Path::new(&path);
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        for ExpectedStep { vals, err, grads } in expecteds {
            for (j, val) in vals.into_iter().enumerate() {
                cols[j].push(val);
            }
            cols[n].push(err);
            for (j, grad) in grads.into_iter().enumerate() {
                cols[n+1+j].push(grad);
            }
        }

        let series = cols.into_iter().enumerate().map(|(j, col)| {
            let name = format!("{}", j);
            Series::new(&name, col)
        }).collect();
        let mut df = DataFrame::new(series)?;
        let mut file = std::fs::File::create(path)?;
        CsvWriter::new(&mut file).has_header(false).finish(&mut df)?;
        Ok(df)
    }

    pub struct CoordGetter(pub Box<dyn Fn(Step) -> f64>);

    fn check(
        inputs: Vec<Input>,
        targets: Vec<(&str, f64)>,
        name: &str,
        max_step_error_ratio: f64,
        max_steps: usize
    ) {
        let targets: BTreeMap<_, _> = targets.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut model = Model::new(inputs.clone(), targets);
        model.train(max_step_error_ratio, max_steps);

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
                                Box::new(move |step: Step| match step.shapes()[shape_idx] {
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
                                Box::new(move |step: Step| match step.shapes()[shape_idx].clone() {
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
        let generate_vals = env::var("GEN_VALS").map(|s| s.parse::<usize>().unwrap()).ok();
        let os = env::consts::OS;
        let os = if os == "macos" { "macos" } else { "linux" };
        let expected_path = format!("testdata/{}/{}.csv", name, os);
        match generate_vals {
            Some(_) => {
                let expecteds: Vec<ExpectedStep> = steps.iter().map(|step| get_actual(step, &coord_getters)).collect();
                let df = write_expecteds(&expected_path, expecteds).unwrap();
                info!("Wrote expecteds to {}", expected_path);
                info!("{}", df);
            }
            None => {
                let (df, expecteds) = load_expecteds(&expected_path);
                info!("Read expecteds from {}", expected_path);
                info!("{}", df);
                assert_eq!(steps.len(), expecteds.len());
                for (idx, (step, expected)) in steps.iter().zip(expecteds.into_iter()).enumerate() {
                    let actual = get_actual(step, &coord_getters);
                    assert_eq!(actual, expected, "Step {}", idx);
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
        let ( z, d ) = d_fns(2);
        let inputs: Vec<Input> = vec![
            (Shape::Circle(Circle { c: R2 { x: 0., y: 0. }, r: 1. }), vec![ z( ), z( ), z( ), ]),
            (Shape::Circle(Circle { c: R2 { x: 1., y: 0. }, r: 1. }), vec![ d(0), z( ), d(1), ]),
        ];
        check(inputs, FIZZ_BUZZ.into(), "fizz_buzz_circles", 0.8, 100);
    }

    #[test]
    fn two_circles_disjoint() {
        // 2 Circles, initially disjoint, each already ideally sized, only 2nd circle's x can move, needs to "find" the 1st circle to get the intersection area right.
        let ( z, d ) = d_fns(1);
        let inputs: Vec<Input> = vec![
            (Shape::Circle(Circle { c: R2 { x: 0., y: 0. }, r: 2. }), vec![ z( ), z( ), z( ), ]),
            (Shape::Circle(Circle { c: R2 { x: 4., y: 0. }, r: 1. }), vec![ d(0), z( ), z( ), ]),
        ];
        let targets = [
            ("0*", 4.),
            ("*1", 1.),
            ("01", 0.5),
        ];
        check(inputs, targets.into(), "two_circles_disjoint", 0.5, 100);
    }

    #[test]
    fn two_circle_containment() {
        // 2 Circles, initially disjoint, each already ideally sized, only 2nd circle's x can move, needs to "find" the 1st circle to get the intersection area right.
        let ( z, d ) = d_fns(1);
        let inputs: Vec<Input> = vec![
            (Shape::Circle(Circle { c: R2 { x: 0. , y: 0. }, r: 2. }), vec![ z( ), z( ), z( ), ]),
            (Shape::Circle(Circle { c: R2 { x: 0.5, y: 0. }, r: 1. }), vec![ d(0), z( ), z( ), ]),
        ];
        let targets = [
            ("0*", 4.),
            ("*1", 1.),
            ("01", 0.5),
        ];
        check(inputs, targets.into(), "two_circle_containment", 0.5, 0);
    }

    #[test]
    fn fizz_buzz_circle_ellipse() {
        let ( z, d ) = d_fns(3);
        let inputs: Vec<Input> = vec![
            ( Shape::Circle(Circle { c: R2 { x: 0., y: 0. }, r: 1.                  }), vec![ z( ), z( ), z( ),       ]),
            ( Shape::  XYRR(  XYRR { c: R2 { x: 1., y: 0. }, r: R2 { x: 1., y: 1. } }), vec![ d(0), z( ), d(1), d(2), ]),
        ];
        check(inputs, FIZZ_BUZZ.into(), "fizz_buzz_circle_ellipse", 0.8, 100)
    }

    #[test]
    fn fizz_buzz_ellipses_diag() {
        let ( z, d ) = d_fns(7);
        let inputs: Vec<Input> = vec![
            ( Shape::XYRR(XYRR { c: R2 { x: 1., y: 0. }, r: R2 { x: 1., y: 1. } }), vec![ d(0), z( ), d(1), d(2), ] ),
            ( Shape::XYRR(XYRR { c: R2 { x: 0., y: 1. }, r: R2 { x: 1., y: 1. } }), vec![ d(3), d(4), d(5), d(6), ] ),
        ];
        // TODO: some nondeterminism sets in on the "error" field, from step 15! Debug.
        check(inputs, FIZZ_BUZZ.into(), "fizz_buzz_ellipses_diag", 0.7, 100)
    }

    #[test]
    fn fizz_buzz_bazz_circle_ellipses() {
        let ( z, d ) = d_fns(7);
        let inputs: Vec<Input> = vec![
            ( Shape::Circle(Circle { c: R2 { x: 0., y: 0. }, r: 1. }     ), vec![ z( ), z( ), z( ),       ]),
            ( Shape::  XYRR(Circle { c: R2 { x: 1., y: 0. }, r: 1. }.to()), vec![ d(0), z( ), d(1), d(2), ]),
            ( Shape::  XYRR(Circle { c: R2 { x: 0., y: 1. }, r: 1. }.to()), vec![ d(3), d(4), d(5), d(6), ]),
        ];
        check(inputs, FIZZ_BUZZ_BAZZ.into(), "fizz_buzz_bazz_circle_ellipses", 0.7, 100)
    }

    #[test]
    fn fizz_buzz_bazz_circles() {
        let ( z, d ) = d_fns(5);
        let inputs: Vec<Input> = vec![
            ( Shape::Circle(Circle { c: R2 { x: 0., y: 0. }, r: 1. }), vec![ z( ), z( ), z( ), ] ),
            ( Shape::Circle(Circle { c: R2 { x: 1., y: 0. }, r: 1. }), vec![ d(0), z( ), d(1), ] ),
            ( Shape::Circle(Circle { c: R2 { x: 0., y: 1. }, r: 1. }), vec![ d(2), d(3), d(4), ] ),
        ];
        check(inputs, FIZZ_BUZZ_BAZZ.into(), "fizz_buzz_bazz_circles", 0.7, 100)
    }

    use crate::scene::tests::ellipses4;

    #[test]
    fn variant_callers() {
        let ellipses = ellipses4(2.);
        let [ e0, e1, e2, e3 ] = ellipses;
        let ( z, d ) = d_fns(12);
        let inputs: Vec<Input> = vec![
            ( e0, vec![ z( ), z( ), z(  ), z(  ), ] ),
            ( e1, vec![ d(0), d(1), d( 2), d( 3), ] ),
            ( e2, vec![ d(4), d(5), d( 6), d( 7), ] ),
            ( e3, vec![ d(8), d(9), d(10), d(11), ] ),
        ];
        // Step size 0.2, #6000, 3.54% error (72.0):
        // XYRR { c: { x: 0.057842232929273305, y: 1.3421141998408261 }, r: { x: 0.9042484099819306, y: 1.7746711918630136 } },
        // XYRR { c: { x: 1.1180140103075666, y: 2.702741124677027 }, r: { x: 0.7738576366499212, y: 2.189919683308931 } },
        // XYRR { c: { x: 1.6046271650155772, y: 0.405655309751768 }, r: { x: 0.9752840313567439, y: 0.5126125569023957 } },
        // XYRR { c: { x: 2.65823706629625, y: 1.062726304716347 }, r: { x: 2.36947609319204, y: 0.37496988567008666 } }
        check(inputs, VARIANT_CALLERS.into(), "variant_callers", 0.7, 100)
    }
}
