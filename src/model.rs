use log::{info, debug, warn};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::{step::Step, targets::TargetsMap, shape::InputSpec};

#[derive(Debug, Clone, Tsify, Serialize, Deserialize)]
pub struct Model {
    pub steps: Vec<Step>,
    pub repeat_idx: Option<usize>,
    pub min_idx: usize,
    pub min_error: f64,
}

impl Model {
    pub fn new(input_specs: Vec<InputSpec>, targets: TargetsMap<f64>) -> Model {
        let step = Step::new(input_specs, targets.into());
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
                    .shapes
                    .iter()
                    .zip(nxt.shapes.iter())
                    .all(|(a, b)| {
                        //println!("Checking {} vs {}", a, b);
                        a.v() == b.v()
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
    use std::{env, path::Path, f64::consts::PI};
    use polars::{prelude::*, series::SeriesIter};

    use crate::{duals::{D, Z}, scene::tests::ellipses4, shape::{circle, InputSpec, xyrr, xyrrt}, to::To, transform::{CanTransform, Transform::Rotate}, coord_getter::{CoordGetter, CoordGetters}};

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
        err: f64,
        vals: Vec<f64>,
    }
    impl<const N: usize> From<([f64; N], f64)> for ExpectedStep {
        fn from((vals, err): ([f64; N], f64)) -> Self {
            ExpectedStep { err, vals: vals.to_vec() }
        }
    }

    fn get_actual(step: &Step, getters: &Vec<CoordGetter<Step>>) -> ExpectedStep {
        let error = step.error.clone();
        let err = error.v();
        let mut vals: Vec<f64> = Vec::new();
        getters.iter().for_each(|getter| {
            let val: f64 = getter(step.clone());
            vals.push(val);
        });
        ExpectedStep { err, vals }
    }

    #[derive(Clone, Debug, derive_more::Deref, PartialEq)]
    pub struct ExpectedSteps(pub Vec<ExpectedStep>);

    use AnyValue::Float64;

    impl ExpectedSteps {
            pub fn load(path: &str) -> (DataFrame, ExpectedSteps) {
            let mut df = CsvReader::from_path(path).unwrap().has_header(true).finish().unwrap();
            df.as_single_chunk_par();
            let mut iters = df.iter().map(|s| s.iter());
            let mut err_iter = iters.next().unwrap();
            let mut val_iters = iters.collect::<Vec<_>>();
            let mut expecteds: Vec<ExpectedStep> = Vec::new();

            let next = |j: usize, iter: &mut SeriesIter| -> f64 {
                match iter.next().expect("should have as many iterations as rows") {
                    Float64(f) => f,
                    v => panic!("Expected Float64 in col {}, got {:?}", j, v),
                }
            };

            for _ in 0..df.height() {
                let err = next(0, &mut err_iter);
                let mut vals: Vec<f64> = Vec::new();
                for (j, mut iter) in val_iters.iter_mut().enumerate() {
                    let val = next(j + 1, &mut iter);
                    vals.push(val);
                }
                expecteds.push(ExpectedStep { err, vals });
            }
            (df, Self(expecteds))
        }


        pub fn write(self, path: &str, col_names: Vec<String>) -> Result<DataFrame, PolarsError> {
            let mut cols: Vec<Vec<f64>> = vec![];
            let n = self[0].vals.len();
            let num_columns = 1 + n;
            for _ in 0..num_columns {
                cols.push(vec![]);
            }
            let path = Path::new(&path);
            let dir = path.parent().unwrap();
            std::fs::create_dir_all(dir)?;
            for ExpectedStep { err, vals } in self.0 {
                cols[0].push(err);
                for (j, val) in vals.into_iter().enumerate() {
                    cols[j + 1].push(val);
                }
            }

            let series = cols.into_iter().enumerate().map(|(j, col)| {
                Series::new(&col_names[j], col)
            }).collect();
            let mut df = DataFrame::new(series)?;
            let mut file = std::fs::File::create(path)?;
            CsvWriter::new(&mut file).has_header(true).finish(&mut df)?;
            Ok(df)
        }
    }

    // Values from https://jitc.bmj.com/content/jitc/10/2/e003027.full.pdf?with-ds=yes (pg. 13)
    static MPOWER: [ (&str, f64); 15 ] = [
        ( "0---",  42. ),
        ( "-1--",  15. ),
        ( "--2-",  10. ),
        ( "---3", 182. ),
        ( "01--",  16. ),
        ( "0-2-",  10. ),
        ( "0--3",  60. ),
        ( "-12-",  12. ),
        ( "-1-3",  23. ),
        ( "--23",  44. ),
        ( "012-",  25. ),
        ( "01-3",  13. ),
        ( "0-23",  13. ),
        ( "-123",  18. ),
        ( "0123",  11. ),
    ];

    fn check<const N: usize>(
        inputs: Vec<InputSpec>,
        targets: [(&str, f64); N],
        name: &str,
        max_step_error_ratio: f64,
        max_steps: usize
    ) {
        let targets: TargetsMap<_> = targets.to();
        let mut model = Model::new(inputs.clone(), targets);
        let max_steps = env::var("STEPS").map(|s| s.parse::<usize>().unwrap()).unwrap_or(max_steps);
        model.train(max_step_error_ratio, max_steps);

        let coord_getters: CoordGetters<Step> = inputs.into();
        assert_eq!(model.grad_size(), coord_getters.len());
        // debug!("coord_getters: {:?}", coord_getters.iter().map(|(idx, _)| idx).collect::<Vec<_>>());

        let steps = model.steps;
        let generate_vals = env::var("GEN_VALS").map(|s| s.parse::<usize>().unwrap()).ok();
        let os = env::consts::OS;
        let os = if os == "macos" { "macos" } else { "linux" };
        let expected_path = format!("testdata/{}/{}.csv", name, os);
        match generate_vals {
            Some(_) => {
                let expecteds = ExpectedSteps(steps.iter().map(|step| get_actual(step, &coord_getters)).collect());
                let mut col_names: Vec<_> = coord_getters.iter().map(|getter| getter.name.clone()).collect();
                col_names.insert(0, "error".to_string());
                let df = expecteds.write(&expected_path, col_names).unwrap();
                info!("Wrote expecteds to {}", expected_path);
                info!("{}", df);
            }
            None => {
                let (df, expecteds) = ExpectedSteps::load(&expected_path);
                info!("Read expecteds from {}", expected_path);
                info!("{}", df);
                assert_eq!(steps.len(), expecteds.len());
                for (idx, (step, expected)) in steps.iter().zip(expecteds.0.into_iter()).enumerate() {
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
        let inputs = vec![
            (circle(0., 0., 1.), vec![ Z, Z, Z, ]),
            (circle(1., 0., 1.), vec![ D, Z, D, ]),
        ];
        check(inputs, FIZZ_BUZZ, "fizz_buzz_circles", 0.8, 100);
    }

    #[test]
    fn two_circles_disjoint() {
        // 2 Circles, initially disjoint, each already ideally sized, only 2nd circle's x can move, needs to "find" the 1st circle to get the intersection area right.
        let inputs = vec![
            (circle(0., 0., 2.), vec![ Z, Z, Z, ]),
            (circle(4., 0., 1.), vec![ D, Z, Z, ]),
        ];
        let targets = [
            ("0*", 4.),
            ("*1", 1.),
            ("01", 0.5),
        ];
        check(inputs, targets, "two_circles_disjoint", 0.5, 100);
    }

    #[test]
    fn two_circles_tangent() {
        let inputs = vec![
            (circle(0., 0., 2.), vec![ Z, Z, Z, ]),
            (circle(3., 0., 1.), vec![ D, Z, Z, ]),
        ];
        let targets = [
            ("0*", 4.),
            ("*1", 1.),
            ("01", 0.5),
        ];
        check(inputs, targets, "two_circles_tangent", 0.5, 100);
    }

    #[test]
    fn two_circle_containment() {
        // 2 Circles, initially disjoint, each already ideally sized, only 2nd circle's x can move, needs to "find" the 1st circle to get the intersection area right.
        let inputs = vec![
            (circle(0. , 0., 2.), vec![ Z, Z, Z, ]),
            (circle(0.5, 0., 1.), vec![ D, Z, Z, ]),
        ];
        let targets = [
            ("0*", 4.),
            ("*1", 1.),
            ("01", 0.5),
        ];
        check(inputs, targets, "two_circle_containment", 0.5, 100);
    }

    #[test]
    fn centroid_repel() {
        let inputs = vec![
            ( xyrr( 0. , 0., 1., 3.), vec![ D; 4 ] ),
            ( xyrr( 0.5, 1., 1., 1.), vec![ D; 4 ] ),
            ( xyrr(-0.5, 1., 1., 1.), vec![ D; 4 ] ),
        ];
        let targets = [
            ("0**", 3. ),
            ("*1*", 1. ),
            ("**2", 1. ),
            ("01*", 0.3),
            ("0*2", 0.3),
            ("*12", 0.3),
            ("012", 0.1),
        ];
        check(inputs, targets, "centroid_repel", 0.8, 100);
    }

    #[test]
    fn fizz_buzz_circle_ellipse() {
        let inputs = vec![
            ( circle(0., 0., 1.,    ), vec![ Z, Z, Z,    ]),
            (   xyrr(1., 0., 1., 1.,), vec![ D, Z, D, D, ]),
        ];
        check(inputs, FIZZ_BUZZ, "fizz_buzz_circle_ellipse", 0.8, 100)
    }

    #[test]
    fn fizz_buzz_ellipses_diag() {
        let inputs = vec![
            ( xyrr(1., 0., 1., 1.), vec![ D, Z, D, D, ] ),
            ( xyrr(0., 1., 1., 1.), vec![ D, D, D, D, ] ),
        ];
        // TODO: some nondeterminism sets in on the "error" field, from step 15! Debug.
        check(inputs, FIZZ_BUZZ, "fizz_buzz_ellipses_diag", 0.7, 100)
    }

    #[test]
    fn fizz_buzz_bazz_circle_ellipses() {
        let inputs = vec![
            ( circle(0., 0., 1.,    ), vec![ Z, Z, Z,    ]),
            (   xyrr(1., 0., 1., 1. ), vec![ D, Z, D, D, ]),
            (   xyrr(0., 1., 1., 1. ), vec![ D, D, D, D, ]),
        ];
        check(inputs, FIZZ_BUZZ_BAZZ, "fizz_buzz_bazz_circle_ellipses", 0.7, 100)
    }

    #[test]
    fn fizz_buzz_bazz_circles() {
        let inputs = vec![
            ( circle(0., 0., 1.), vec![ Z, Z, Z, ] ),
            ( circle(1., 0., 1.), vec![ D, Z, D, ] ),
            ( circle(0., 1., 1.), vec![ D, D, D, ] ),
        ];
        check(inputs, FIZZ_BUZZ_BAZZ, "fizz_buzz_bazz_circles", 0.7, 100)
    }

    #[test]
    fn variant_callers() {
        let ellipses = ellipses4(2.);
        let [ e0, e1, e2, e3 ] = ellipses;
        let inputs = vec![
            ( e0, vec![ Z, Z, Z, Z, ] ),
            ( e1, vec![ D, D, D, D, ] ),
            ( e2, vec![ D, D, D, D, ] ),
            ( e3, vec![ D, D, D, D, ] ),
        ];
        // Step size 0.2, #6000, 3.54% error (72.0):
        // XYRR { c: { x: 0.057842232929273305, y: 1.3421141998408261 }, r: { x: 0.9042484099819306, y: 1.7746711918630136 } },
        // XYRR { c: { x: 1.1180140103075666, y: 2.702741124677027 }, r: { x: 0.7738576366499212, y: 2.189919683308931 } },
        // XYRR { c: { x: 1.6046271650155772, y: 0.405655309751768 }, r: { x: 0.9752840313567439, y: 0.5126125569023957 } },
        // XYRR { c: { x: 2.65823706629625, y: 1.062726304716347 }, r: { x: 2.36947609319204, y: 0.37496988567008666 } }
        check(inputs, VARIANT_CALLERS, "variant_callers", 0.7, 100)
    }

    #[test]
    fn disjoint_variant_callers_bug() {
        let inputs = vec![
            ( xyrr(0., 0., 1., 1.), vec![ D; 4 ] ),
            ( xyrr(3., 0., 1., 1.), vec![ D; 4 ] ),
            ( xyrr(0., 3., 1., 1.), vec![ D; 4 ] ),
            ( xyrr(3., 3., 1., 1.), vec![ D; 4 ] ),
        ];
        check(inputs, VARIANT_CALLERS, "disjoint_variant_callers_bug", 0.5, 100);
    }

    #[test]
    fn variant_callers_diag() {
        let ellipses = ellipses4(2.).map(|e| e.transform(&Rotate(PI / 4.)));
        let [ e0, e1, e2, e3 ] = ellipses;
        let inputs = vec![
            ( e0, vec![ D; 5 ] ),
            ( e1, vec![ D; 5 ] ),
            ( e2, vec![ D; 5 ] ),
            ( e3, vec![ D; 5 ] ),
        ];
        check(inputs, VARIANT_CALLERS, "variant_callers_diag", 0.5, 100);
    }

    #[test]
    fn webapp_bug1() {
        let inputs = vec![
            ( xyrrt( 0.7319754427924579, -2.1575408875986393e-16, 1.2448120381919545, 0.9798569195408114,  4.8268551130929626e-17), vec![ D; 5 ] ),
            ( xyrrt(-1.5088966066610663,  1.0407479831736694e-16, 1.97886101672388  , 2.178313681735663 , -3.664600361442153e-17 ), vec![ D; 5 ] ),
            ( xyrrt( 2.2769211638686104,  1.2002706758532478e-16, 2.8997542067333413, 2.8817259204197674,  2.976941513813048e-17 ), vec![ D; 5 ] ),
        ];
        check(inputs, FIZZ_BUZZ_BAZZ, "webapp_bug1", 0.1, 100);
    }

    #[test]
    fn mpower_spike() {
        let inputs = vec![
            ( xyrrt(-0.7795647537412774 , 1.3864596428989213, 0.9779421231703596, 2.116221516077534 , 0.5929345063728056 ), vec![ D; 5 ] ),
            ( xyrrt(-0.334020975375785  , 2.2012585482178664, 0.8217004750509326, 1.8949774045049235, 0.8930950419653292 ), vec![ D; 5 ] ),
            ( xyrrt( 0.15416800838315917, 2.5522066048894576, 2.052620045044561 , 0.7844775004499663, 0.47084646751887366), vec![ D; 5 ] ),
            ( xyrrt( 0.9594177207338993 , 1.5988440867036033, 2.417618150694609 , 1.4130685330891937, 0.8644165147959761 ), vec![ D; 5 ] ),
        ];
        check(inputs, MPOWER, "mpower_spike", 0.1, 2);
    }
}
