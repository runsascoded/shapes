use std::{collections::HashMap, fmt::Display, cell::Ref};

use log::{info, debug};
use serde::{Deserialize, Serialize};
use tsify::{declare, Tsify};

use crate::{circle::{Circle, Input, Duals}, intersections::Intersections, dual::D, r2::R2, areas::Areas, regions::Regions, zero::Zero};
use crate::dual::Dual;

#[declare]
pub type Targets = HashMap<String, f64>;
#[declare]
pub type Errors = HashMap<String, Error>;

pub struct DisjointPenalty {
    pub i: usize,
    pub j: usize,
    pub gap: Dual,
    pub target: f64,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Diagram {
    pub inputs: Vec<Input>,
    pub regions: Regions,
    //pub shapes: Vec<Circle<f64>>,
    pub targets: Targets,
    pub total_target_area: f64,
    pub errors: Errors,
    pub error: Dual,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Error {
    pub key: String,
    pub actual_area: Option<Dual>,
    pub total_area: Dual,
    pub actual_frac: Dual,
    pub target_area: f64,
    pub total_target_area: f64,
    pub target_frac: f64,
    pub error: Dual,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{}: err {:.3}, {:.3} / {:.3} = {:.3}, {} / {:.3} = {:.3}",
            self.key, self.error.v(),
            self.target_area, self.total_target_area, self.target_frac,
            self.actual_area.clone().map(|a| format!("{:.3}", a.v())).unwrap_or_else(|| "-".to_string()),
            self.total_area.v(), self.actual_frac.v(),
        )
    }
}

impl Diagram {
    pub fn new(inputs: Vec<Input>, targets: Targets, total_target_area: Option<f64>) -> Diagram {
        let intersections = Intersections::new(&inputs);
        // let duals = intersections.duals;
        let all_key = String::from_utf8(vec![b'*'; intersections.len()]).unwrap();
        let mut expanded_targets = targets.clone();
        Areas::expand(&mut expanded_targets);
        let total_target_area = total_target_area.unwrap_or_else(|| {
            expanded_targets
            .get(&all_key)
            .expect(&format!("{} not found among {} keys", all_key, expanded_targets.len()))
            .clone()
        });
        let errors = Self::compute_errors(&intersections, &targets, &total_target_area);
        let mut error = errors.values().into_iter().map(|e| e.error.abs()).sum();
        // Optional/Alternate loss function based on per-region squared errors, weights errors by region size:
        // let error = errors.values().into_iter().map(|e| e.error.clone() * &e.error).sum::<D>().sqrt();
        let regions = Regions::new(&intersections);
        let duals = &intersections.duals;

        // Include penalties for erroneously-disjoint shapes
        // let mut disjoint_penalties = Vec::<DisjointPenalty>::new();
        let mut total_disjoint_penalty = Dual::zero(&error);
        let n = inputs.len();
        for i in 0..(n - 1) {
            let ci = duals[i].clone();
            for j in (i + 1)..n {
                let mut key = String::from_utf8(vec![b'*'; n]).unwrap();
                let chi = char::to_string(&char::from_digit(i as u32, 10).unwrap());
                let chj = char::to_string(&char::from_digit(j as u32, 10).unwrap());
                key.replace_range(i..i+1, &chi);
                key.replace_range(j..j+1, &chj);
                let target = expanded_targets.get(&key);
                match target {
                    Some(target) => {
                        match ci.borrow().distance(&*duals[j].borrow()) {
                            Some(gap) => {
                                // disjoint_penalties.push(
                                //     DisjointPenalty { i, j, gap: gap.clone(), target: target.clone() }
                                // );
                                debug!("  disjoint penalty: {}: {} * {}", key, &gap, target);
                                total_disjoint_penalty += gap * target;
                            },
                            None => (),
                        }
                    },
                    None => ()
                }
            }
        }

        if total_disjoint_penalty.v() > 0. {
            debug!("  total_disjoint_penalty: {}", total_disjoint_penalty);
            error += total_disjoint_penalty;
        }

        Diagram { inputs, regions, targets, total_target_area, errors, error }
    }

    pub fn shapes(&self) -> Vec<Circle<f64>> {
        self.inputs.iter().map(|(s, _)| s.clone()).collect()
    }

    pub fn n(&self) -> usize {
        self.shapes().len()
    }

    pub fn compute_errors(intersections: &Intersections, targets: &Targets, total_target_area: &f64) -> Errors {
        let n = intersections.len();
        let all_key = String::from_utf8(vec![b'*'; n]).unwrap();
        let none_key = String::from_utf8(vec![b'-'; n]).unwrap();
        let total_area = intersections.area(&all_key).unwrap_or_else(|| intersections.zero());
        targets.iter().filter_map(|(key, target_area)| {
            if key == &none_key {
                None
            } else {
                let actual_area = intersections.area(key);
                let target_frac = target_area / total_target_area;
                let actual_frac = actual_area.clone().unwrap_or_else(|| intersections.zero()).clone() / &total_area;
                let error = actual_frac.clone() - target_frac;
                Some((
                    key.clone(),
                    Error {
                        key: key.clone(),
                        actual_area, total_area: total_area.clone(),
                        target_area: target_area.clone(), total_target_area: total_target_area.clone(),
                        actual_frac,
                        target_frac,
                        error,
                    }
                ))
            }
        }).collect()
    }

    pub fn duals(&self) -> Vec<Duals> {
        self.inputs.iter().map(|(_, duals)| duals.clone()).collect()
    }

    pub fn step(&self, max_step_error_ratio: f64) -> Diagram {
        let error = self.error.clone();
        // let error = self.errors.values().into_iter().map(|e| e.error.clone() * &e.error).sum::<D>().sqrt();
        let error_size = &error.v();
        let grad_vec = (-error.clone()).d();

        let step_size = error_size * max_step_error_ratio;
        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>().sqrt();
        let grad_scale = step_size / magnitude;
        let step_vec = grad_vec.iter().map(|grad| grad * grad_scale).collect::<Vec<f64>>();

        debug!("  err {:?}", error);
        debug!("  step_size {}, magnitude {}, grad_scale {}", step_size, magnitude, grad_scale);
        debug!("  step_vec {:?}", step_vec);
        let shapes = &self.shapes();
        let new_inputs = shapes.iter().zip(self.duals()).map(|(s, duals)| {
            let [ dx, dy, dr ]: [f64; 3] = duals.clone().map(|d| d.iter().zip(&step_vec).map(|(mask, step)| mask * step).sum());
            let c = R2 {
                x: s.c.x + dx,
                y: s.c.y + dy,
            };
            let r = s.r + dr;
            ( Circle { idx: s.idx, c, r }, duals )
        }).collect::<Vec<Input>>();
        for (cur, (nxt, _)) in shapes.iter().zip(new_inputs.iter()) {
            debug!("  {} -> {}", cur, nxt);
        }
        Diagram::new(new_inputs, self.targets.clone(), Some(self.total_target_area))
    }
}
