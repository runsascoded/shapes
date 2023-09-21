/// Characterize accuracy/precision of `roots::find_quartic_roots`.
#[cfg(test)]
mod tests {
    use std::fmt::Display;

    use crate::math::{quartic::{NormalizedRoots, tests::coeffs_from_roots, self}, complex::Complex};
    use itertools::Itertools;
    // pub type Roots = NormalizedRoots<f64>;
    use ordered_float::OrderedFloat;
    use roots::find_roots_quartic;

    // static VALS: [ f64; 7 ]  = [ -1e2, -1., -1e-2, 0., 1e-2, 1., 1e2 ];
    static VALS: [ f64; 4 ] = [ -1e2, -1., -1e-2, 0., ];
    static N: usize = VALS.len();

    #[test]
    fn sweep_reals() {
        let mut alignments: Vec<Alignment> =
            (0..N).flat_map(move |i0| {
                let r0 = Complex::re(VALS[i0]);
                (i0..N).flat_map(move |i1| {
                    let r1 = Complex::re(VALS[i1]);
                    (i1..N).flat_map(move |i2| {
                        let r2 = Complex::re(VALS[i2]);
                        (i2..N).map(move |i3|  {
                            let r3 = Complex::re(VALS[i3]);
                            let scale = 1.;
                            check(r0, r1, r2, r3, scale)
                        })
                    })
                })
            }).collect();
        alignments.sort_by_cached_key(|a| (a.extra.len(), a.missing.len(), OrderedFloat(a.log_err)));
        alignments.iter().group_by(|a| (a.extra.len(), a.missing.len())).into_iter().for_each(|((num_extra, num_missing), group)| {
            let mut group = group.collect::<Vec<&Alignment>>();
            group.sort_by_cached_key(|a| OrderedFloat(-a.log_err));
            let num_exact = group.iter().filter(|a| a.is_exact()).count();
            println!(
                "{} extra, {} missing: {} cases{}",
                num_extra,
                num_missing,
                group.len(),
                if num_exact > 0 { format!(" ({} exactly correct omitted)", num_exact) } else { "".to_string() },
            );
            for alignment in group {
                if alignment.is_exact() {
                    continue;
                }
                let lines = alignment.lines();
                println!("\t{}\n", lines.join("\n\t"));
            }
        });
    }

    #[test]
    fn sweep_mixed() {
        // This depressed quartic ends up with c≈0.01, and "d" and "e" below 2e-17; fuzzy zero-cmp on "d" treats it as biquadratic
        // check(Complex::re(-0.1), Complex::re(-0.1), Complex { re: -0.1, im: -0.1, }, Complex { re: -0.1, im:  0.1, }, 1.,);
        // This one is similar, but motivates comparing "d" (≈1e-15) with "c" (≈100). Increasing the imaginary component of the latter pair below can make `d`'s value be larger in absolute terms, but the math relative to "c" is more important.
        // check(Complex::re(-0.1), Complex::re(-0.1), Complex { re: -0.1, im: -10., }, Complex { re: -0.1, im: 10., }, 1.,);
        let vals = [ -10., -1., -0.1, 0., 0.1, 1., 10., ];
        let n = vals.len();
        for i0 in 0..n {
            let r0 = Complex::re(vals[i0]);
            for i1 in i0..n {
                let r1 = Complex::re(vals[i1]);
                for i2 in i1..n {
                    let re = vals[i2];
                    for i3 in i2..n {
                        let im = vals[i3];
                        let im0 = Complex { re, im };
                        let im1 = im0.conj();
                        let scale = 1.;
                        check(r0, r1, im0, im1, scale);
                    }
                }
            }
        }
    }

    #[test]
    fn sweep_imags() {
        // check(
        //     Complex { re: -10.0, im: -10.0 },
        //     Complex { re: -10.0, im:  10.0 },
        //     Complex { re: -10.0, im:  -0.1 },
        //     Complex { re: -10.0, im:   0.1 },
        //     1.
        // );
        let vals = [ -10., -1., -0.1, 0., 0.1, 1., 10., ];
        let n = vals.len();
        for i0 in 0..n {
            let re0 = vals[i0];
            for i1 in i0..n {
                let im0 = vals[i1];
                let c00 = Complex { re: re0, im: im0 };
                let c01 = c00.conj();
                for i2 in i1..n {
                    let re1 = vals[i2];
                    for i3 in i2..n {
                        let im1 = vals[i3];
                        let c10 = Complex { re: re1, im: im1 };
                        let c11 = c10.conj();
                        let scale = 1.;
                        check(c00, c01, c10, c11, scale);
                    }
                }
            }
        }
    }

    fn check(r0: Complex<f64>, r1: Complex<f64>, r2: Complex<f64>, r3: Complex<f64>, scale: f64) -> Alignment {
        let roots = NormalizedRoots::new([ r0, r1, r2, r3 ]);
        let a4 = scale;
        let [ a3, a2, a1, a0 ] = coeffs_from_roots(roots.clone()).map(|c| c * scale);
        let [ e3, e2, e1, e0 ] = [
            -(r0 + r1 + r2 + r3),
            r0 * r1 + r0 * r2 + r0 * r3 + r1 * r2 + r1 * r3 + r2 * r3,
            -(r0 * r1 * r2 + r0 * r1 * r3 + r0 * r2 * r3 + r1 * r2 * r3),
            r0 * r1 * r2 * r3,
        ].map(|c| c.re * scale);
        assert_relative_eq!(a3, e3, max_relative = 1e-5, epsilon = 1e-14);
        assert_relative_eq!(a2, e2, max_relative = 1e-5, epsilon = 1e-14);
        assert_relative_eq!(a1, e1, max_relative = 1e-5, epsilon = 1e-14);
        assert_relative_eq!(a0, e0, max_relative = 1e-5, epsilon = 1e-14);
        let expected = Into::<quartic::Roots<f64>>::into(roots).reals();
        let roots = find_roots_quartic(a4, a3, a2, a1, a0);
        let actual = roots.as_ref().to_vec();
        // Require |ln(actual) - ln(expected)| ≤ ε for inclusion in an Alignment
        let ε = 5e-2;
        // Allow values less than this to match zero (relative/log comparison above doesn't work when either side is 0); absolute error is recorded and reported for such cases.
        let fuzzy_zero = 1e-10;
        align(
            &expected,
            Alignment {
                log_err: 0.,
                actual: actual.clone(),
                expected: expected.clone(),
                vals: vec![],
                missing: vec![],
                extra: actual.clone(),
            },
            ε,
            fuzzy_zero,
        )
    }

    #[derive(Clone, Debug, PartialEq, PartialOrd)]
    pub enum Err {
        Zero(f64),
        Log(f64),
    }

    impl Eq for Err {}
    impl Ord for Err {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            match (self, other) {
                (Err::Zero(a), Err::Zero(b)) => a.partial_cmp(b).unwrap(),
                (Err::Log(a), Err::Log(b)) => a.partial_cmp(b).unwrap(),
                (Err::Zero(_), Err::Log(_)) => std::cmp::Ordering::Less,
                (Err::Log(_), Err::Zero(_)) => std::cmp::Ordering::Greater,
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct Alignment {
        pub log_err: f64,
        pub actual: Vec<f64>,
        pub expected: Vec<f64>,
        pub vals: Vec<(f64, f64, Err)>,
        pub missing: Vec<f64>,
        pub extra: Vec<f64>,
    }

    impl Display for Alignment {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut vals = self.vals.clone();
            vals.sort_by_cached_key(|(_, _, err)| err.clone());
            write!(
                f,
                "Alignment {{ err: {}, vals: {}{}{}",
                self.err(),
                vals.iter().map(|(actual, expected, _err)| if actual == expected {
                    format!("{}", actual)
                } else {
                    format!("({}, {})", actual, expected)
                }).collect::<Vec<String>>().join(", "),
                if self.missing.is_empty() {
                    "".to_string()
                } else {
                    format!(", missing {:?}", self.missing)
                },
                if self.extra.is_empty() {
                    "".to_string()
                } else {
                    format!(", extra {:?}", self.extra)
                },
            )
        }
    }

    impl Alignment {
        pub fn err(&self) -> f64 {
            self.log_err.exp()
        }
        pub fn max_log_err(&self) -> Option<f64> {
            self.log_errs().into_iter().max_by_key(|err| OrderedFloat(*err))
        }
        pub fn max_zero_err(&self) -> Option<f64> {
            self.zero_errs().into_iter().max_by_key(|err| OrderedFloat(*err))
        }
        pub fn zero_errs(&self) -> Vec<f64> {
            self.vals.iter().filter_map(|(_, _, err)| match err {
                Err::Zero(err) => Some(*err),
                Err::Log(_) => None,
            }).collect()
        }
        pub fn log_errs(&self) -> Vec<f64> {
            self.vals.iter().filter_map(|(_, _, err)| match err {
                Err::Zero(_) => None,
                Err::Log(err) => Some(*err),
            }).collect()
        }
        pub fn is_exact(&self) -> bool {
            self.log_err == 0. && self.missing.is_empty() && self.extra.is_empty()
        }
        pub fn lines(&self) -> Vec<String> {
            let mut lines = vec![];
            let mut vals = self.vals.clone();
            vals.sort_by_cached_key(|(_, _, err)| err.clone());
            lines.push(format!("Expected: {:?}", self.expected));
            lines.push(format!("Actual: {:?}", self.actual));
            lines.push(format!("Total error frac: {}{}", self.err(), self.max_log_err().map(|err| format!(" (max: {})", err)).unwrap_or("".to_string())));
            self.max_zero_err().into_iter().filter(|e| *e > 0.).for_each(|err| {
                lines.push(format!("Max zero error: {}", err));
            });
            // lines.push(
            //     format!(
            //         "Aligned values: {}",
            //         vals.iter().map(|(actual, expected, _err)| if actual == expected {
            //             format!("{}", actual)
            //         } else {
            //             format!("({}, {})", actual, expected)
            //         }).collect::<Vec<String>>().join(", ")
            //     )
            // );
            if !self.missing.is_empty() {
                lines.push(format!("Missing: {:?}", self.missing));
            }
            if !self.extra.is_empty() {
                lines.push(format!("Extra: {:?}", self.extra));
            }
            lines
        }
    }

    fn align(expecteds: &Vec<f64>, alignment: Alignment, ε: f64, fuzzy_zero: f64) -> Alignment {
        let actuals = &alignment.actual;
        if actuals.is_empty() {
            let mut alignment = alignment.clone();
            alignment.missing.extend(expecteds);
            alignment
        } else if expecteds.is_empty() {
            alignment
        } else {
            let expected = expecteds[0];
            let rest = expecteds[1..].to_vec();
            actuals.iter().map(|actual| {
                let mut alignment = alignment.clone();
                if expected == 0. {
                    if actual.abs() <= fuzzy_zero {
                        alignment.vals.push((*actual, expected, Err::Zero(*actual)));
                        alignment.extra = alignment.extra.iter().filter(|v| v != &actual).cloned().collect();
                        align(&rest, alignment, ε, fuzzy_zero)
                    } else {
                        alignment.missing.push(expected);
                        align(&rest, alignment, ε, fuzzy_zero)
                    }
                } else if *actual == 0. {
                    alignment.missing.push(expected);
                    align(&rest, alignment, ε, fuzzy_zero)
                } else if actual.signum() != expected.signum() {
                    alignment.missing.push(expected);
                    align(&rest, alignment, ε, fuzzy_zero)
                } else {
                    let a_abs = actual.abs();
                    let e_abs = expected.abs();
                    let log_err = (a_abs.ln() - e_abs.ln()).abs();
                    if log_err <= ε {
                        alignment.vals.push((*actual, expected, Err::Log(log_err)));
                        alignment.log_err += log_err;
                        alignment.extra = alignment.extra.iter().filter(|v| v != &actual).cloned().collect();
                        align(&rest, alignment, ε, fuzzy_zero)
                    } else {
                        alignment.missing.push(expected);
                        align(&rest, alignment, ε, fuzzy_zero)
                    }
                }
            }).min_by_key(|alignment| (
                // 1st priority: maximize number of "aligned" roots (expected roots that are within ε of an actual root)
                alignment.missing.len() + alignment.extra.len(),
                // 2nd priority: minimize total error among aligned roots
                OrderedFloat(alignment.log_err),
            )).unwrap()
        }
    }
}