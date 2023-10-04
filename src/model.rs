

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
    use polars::prelude::*;

    use crate::{dual::Dual, duals::{is_one_hot, D, Z}, scene::tests::ellipses4, shape::{circle, InputSpec, xyrr, xyrrt}, to::To, transform::{CanTransform, Transform::Rotate}};
    use derive_more::Deref;

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
            let val: f64 = getter(step.clone());
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

    #[derive(Deref)]
    pub struct CoordGetter(pub Box<dyn Fn(Step) -> f64>);

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
        let last_step = model.steps[model.steps.len() - 1].clone();
        let shapes = last_step.shapes;

        let mut coord_getters: Vec<(usize, CoordGetter)> = shapes.iter().enumerate().flat_map(
            |(shape_idx, shape)| {
                let getters = shape.getters(shape_idx);
                getters.into_iter().zip(shape.duals()).filter_map(|(getter, dual_vec)| {
                    is_one_hot(&dual_vec).map(|grad_idx| (
                        grad_idx,
                        CoordGetter(Box::new(move |step: Step| getter(step.shapes[shape_idx].v())))
                    )
                )
                }).collect::<Vec<_>>()
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
    fn fizz_buzz_bazz_ellipses() {
        let inputs = vec![
            ( xyrrt(-0.5, 0.                , 1., 1., 0. ), vec![ D, D, D, D, D, ]),
            ( xyrrt( 0. , f64::sqrt(3.) / 2., 1., 1., 0. ), vec![ D, D, D, D, D, ]),
            ( xyrrt( 0.5, 0.                , 1., 1., 0. ), vec![ D, D, D, D, D, ]),
        ];
        // Trying to repro an error from the webapp, appears on computing step idx 1:
        // Bad unit_intersections: { c: ( 1.021, vec![-0.987, -0.000, -1.008, -0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.987,  0.000, -0.000,  0.000, -0.000], -0.000, vec![ 0.000, -0.978,  0.000, -0.000,  28.644,  0.000,  0.000,  0.000,  0.000,  0.000, -0.000,  0.978, -0.000,  0.000, -29.655]), r: ( 0.965, vec![ 0.000,  0.000, -0.953, -0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.987,  0.000, -0.000],  0.966, vec![ 0.000,  0.000,  0.000, -0.944, -0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000,  0.000, -0.000,  0.978,  0.000]) }
        // point: ( 3016.808, vec![-2915.318,  22942914445312.000, -8794953.670,  8708909.166, -457793291157504.000,  0.000,  0.000,  0.000,  0.000,  0.000,  2915.318, -22942914445312.000,  9107084.543, -9017986.607,  939648104792064.000], -14168953874384.000, vec![ 47568239469216.000, -2345553850330437454114710355968.000,  144108236874186752.000, -114524790333243392.000,  15120624010278103370236034023424.000,  0.000,  0.000,  0.000,  0.000,  0.000, -47568239469216.000,  2345553850330437454114710355968.000, -85471077573066752.000,  114085380570218496.000, -132047178696014002492618823434240.000])
        // unit.r:  14168953874384.000, vec![-47568239469216.008,  2345553850330437735589687066624.000, -144108236874186752.000,  114524790333243392.000, -15120624010278103370236034023424.000,  0.000,  0.000,  0.000,  0.000,  0.000,  47568239469216.008, -2345553850330437735589687066624.000,  85471077573066752.000, -114085380570218496.000,  132047178696014002492618823434240.000]
        // self.r:  14671807256066.193, vec![-49256426916915.180,  2428797094398633007069381263360.000, -149222609811165056.000,  118603594486230784.000, -15657251977601760239788918571008.000,  0.000,  0.000,  0.000,  0.000,  0.000,  49256426916915.180, -2428797094398633007069381263360.000,  88504429278116784.000, -118149099148690080.000,  136733507054307868299026247974912.000]', src/ellipses/xyrr.rs:95:21
        // Doesn't repro here, so far
        check(inputs, FIZZ_BUZZ_BAZZ, "fizz_buzz_bazz_ellipses", 0.1, 2)
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
        check(inputs, VARIANT_CALLERS, "variant_callers_diag", 0.5, 100)
    }
}
