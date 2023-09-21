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
    static VALS: [ f64; 3 ] = [ -1e2, -1., -1e-2, ];
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
        // actual.asinh() must be within ε of expected.asinh() to be included in "alignment"
        // Approximates absolute difference near 0, log difference for larger (positive and negative) values
        let ε = 1e-2;
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
        )
    }

    #[derive(Clone, Debug)]
    pub struct Alignment {
        pub log_err: f64,
        pub actual: Vec<f64>,
        pub expected: Vec<f64>,
        pub vals: Vec<(f64, f64, f64)>,
        pub missing: Vec<f64>,
        pub extra: Vec<f64>,
    }

    impl Display for Alignment {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut vals = self.vals.clone();
            vals.sort_by_cached_key(|(_, _, err)| OrderedFloat(*err));
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
        pub fn is_exact(&self) -> bool {
            self.log_err == 0. && self.missing.is_empty() && self.extra.is_empty()
        }
        pub fn lines(&self) -> Vec<String> {
            let mut lines = vec![];
            let mut vals = self.vals.clone();
            vals.sort_by_cached_key(|(_, _, err)| OrderedFloat(*err));
            lines.push(format!("Expected: {:?}", self.expected));
            lines.push(format!("Actual: {:?}", self.actual));
            lines.push(format!("Total error frac: {}", self.err()));
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

    fn align(expecteds: &Vec<f64>, alignment: Alignment, ε: f64) -> Alignment {
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
            actuals.iter().filter_map(|actual| {
                let mut alignment = alignment.clone();
                if expected == 0. {
                    Some(
                        if *actual == 0. {
                            alignment.vals.push((*actual, expected, 0.));
                            alignment.extra = alignment.extra.iter().filter(|v| v != &actual).cloned().collect();
                            align(&rest, alignment, ε)
                        } else {
                            alignment.missing.push(expected);
                            align(&rest, alignment, ε)
                        }
                    )
                } else if *actual == 0. {
                    None
                } else {
                    Some({
                        let a_abs = actual.abs();
                        let e_abs = expected.abs();
                        let log_err = f64::max(a_abs.ln(), e_abs.ln());
                        // let err = log_err.exp();
                        // let err = max(a_abs, expected) / (actual.asinh() - expected.asinh()).abs();
                        if log_err <= ε {
                            alignment.vals.push((*actual, expected, log_err));
                            alignment.log_err += log_err;
                            alignment.extra = alignment.extra.iter().filter(|v| v != &actual).cloned().collect();
                            align(&rest, alignment, ε)
                        } else {
                            alignment.missing.push(expected);
                            align(&rest, alignment, ε)
                        }
                    })
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