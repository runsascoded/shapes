/// Characterize accuracy/precision of `roots::find_quartic_roots`.
#[cfg(test)]
mod tests {
    use crate::math::{quartic::NormalizedRoots, complex::Complex};
    pub type Roots = NormalizedRoots<f64>;
    use NormalizedRoots::*;

    static VALS: [ f64; 7 ]  = [ -1e2, -1., -1e-2, 0., 1e-2, 1., 1e2 ];
    static N: usize = VALS.len();

    #[test]
    fn four_distinct_reals() {
        let cases: Vec<Roots> =
            (0..N-3).flat_map(|i0| {
                let r0 = VALS[i0];
                (i0+1..N-2).flat_map(move |i1| {
                    let r1 = VALS[i1];
                    (i1+1..N-1).flat_map(move |i2| {
                        let r2 = VALS[i2];
                        (i2+1..N).map(move |i3| {
                            let r3 = VALS[i3];
                            Reals([ r0, r1, r2, r3 ])
                        })
                    })
                })
            }).collect();
    }

    #[test]
    fn distinct_real_real_double() {
        let cases: Vec<Roots> =
            (0..N).flat_map(|double_root_idx| {
                let double_root = VALS[double_root_idx];
                (0..N-1).flat_map(move |i0| {
                    if i0 == double_root_idx {
                        vec![]
                    } else {
                        let r0 = VALS[i0];
                        (i0+1..N).filter_map(move |i1| {
                            if i1 == double_root_idx {
                                None
                            } else {
                                let r1 = VALS[i1];
                                Some(Reals([ double_root, double_root, r0, r1 ]))
                            }
                        }).collect()
                    }
                })
            }).collect();
    }

    #[test]
    fn distinct_real_triple() {
        let cases: Vec<Roots> =
            (0..N).flat_map(|i0| {
                let r0 = VALS[i0];
                (0..N-2).flat_map(move |i1| {
                    if i1 == i0 {
                        vec![]
                    } else {
                        let r1 = VALS[i1];
                        (i1+1..N-1).flat_map(move |i2| {
                            if i2 == i0 {
                                vec![]
                            } else {
                                let r2 = VALS[i2];
                                (i2+1..N).filter_map(move |i3| {
                                    if i3 == i0 {
                                        None
                                    } else {
                                        let r3 = VALS[i3];
                                        Some(Reals([ r0, r1, r2, r3 ]))
                                    }
                                }).collect()
                            }
                        }).collect()
                    }
                })
            }).collect();
    }

    #[test]
    fn quadruple_root() {
        let cases: Vec<Roots> =
            (0..N).map(|i| {
                let r = VALS[i];
                Roots::new([ r, r, r, r ].map(Complex::re))
            }).collect();
    }

    #[test]
    fn distinct_real_real_complex_pair() {
        let cases: Vec<Roots> =
            (0..N-1).flat_map(|i0| {
                let r0 = VALS[i0];
                (i0..N).flat_map(move |i1| {
                    let r1 = VALS[i1];
                    VALS.iter().cloned().flat_map(move |re| {
                        VALS.iter().cloned().filter(|v| *v != 0.).map(move |im| {
                            Mixed(r0, r1, Complex { re, im })
                        })
                    })
                })
            }).collect();
    }

    #[test]
    fn double_real_complex_pair() {
        let cases: Vec<Roots> =
            VALS.iter().cloned().flat_map(|double_root| {
                VALS.iter().cloned().flat_map(move |re| {
                    VALS.iter().cloned().filter(|v| *v != 0.).map(move |im| {
                        Mixed(double_root, double_root, Complex { re, im })
                    })
                })
            }).collect();
    }

    #[test]
    fn two_distinct_complex_pairs() {
        let cases: Vec<Roots> =
            VALS.iter().cloned().flat_map(move |re0| {
                VALS.iter().cloned().filter(|v| *v != 0.).flat_map(move |im0| {
                    VALS.iter().cloned().flat_map(move |re1| {
                        VALS.iter().cloned().filter(move |v| *v != 0. && (re0 != re1 || im0 != *v)).map(move |im1| {
                            Imags(Complex { re: re0, im: im0 }, Complex { re: re1, im: im1 })
                        })
                    })
                })
            }).collect();
    }

    #[test]
    fn complex_pair_double_root() {
        let cases: Vec<Roots> =
            VALS.iter().cloned().flat_map(move |re| {
                VALS.iter().cloned().filter(|v| *v != 0.).map(move |im| {
                    Imags(Complex { re, im }, Complex { re, im })
                })
            }).collect();
    }
}