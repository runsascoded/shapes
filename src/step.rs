use std::collections::BTreeMap;
use std::fmt::Display;

use log::{info, debug, warn};
use serde::{Deserialize, Serialize};
use tsify::{declare, Tsify};

use crate::distance;
use crate::ellipses::xyrr::XYRR;
use crate::ellipses::xyrrt::XYRRT;
use crate::math::recip::Recip;
use crate::shape::{Input, Shape, Duals, Shapes};
use crate::{circle::Circle, distance::Distance, scene::Scene, math::is_zero::IsZero, r2::R2, targets::{Targets, TargetsMap}, regions};
use crate::dual::{Dual, D};

#[declare]
pub type Errors = BTreeMap<String, Error>;

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Step {
    pub inputs: Vec<Input>,
    pub components: Vec<regions::Component>,
    pub targets: Targets<f64>,
    pub total_area: Dual,
    pub errors: Errors,
    pub error: Dual,
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Error {
    pub key: String,
    pub actual_area: Option<Dual>,
    pub actual_frac: Dual,
    pub target_area: f64,
    pub target_frac: f64,
    pub error: Dual,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{}: err {:.3}, target {:.3} ({:.3}), actual {} → {:.3}",
            self.key, self.error.v(),
            self.target_area, self.target_frac,
            self.actual_area.clone().map(|a| format!("{:.3}", a.v())).unwrap_or_else(|| "-".to_string()),
            self.actual_frac.v(),
        )
    }
}

impl Step {
    pub fn new(inputs: Vec<Input>, targets: Targets<f64>) -> Step {
        let shapes = Shapes::from(&inputs);
        let scene = Scene::new(shapes);
        let sets = &scene.sets;
        let all_key = String::from_utf8(vec![b'*'; scene.len()]).unwrap();
        let total_area = scene.area(&all_key).unwrap_or_else(|| scene.zero());
        debug!("scene: {} components, total_area {}, component sizes {}", scene.components.len(), total_area, scene.components.iter().map(|c| c.sets.len().to_string()).collect::<Vec<_>>().join(", "));
        for component in &scene.components {
            debug!("  {} regions", component.regions.len());
            for region in &component.regions {
                debug!("    {}: {} segments, area {}", region.key, region.segments.len(), region.area());
            }
        }
        let errors = Self::compute_errors(&scene, &targets, &total_area);
        let disjoint_targets = targets.disjoints();
        let mut error = scene.zero();
        for key in disjoint_targets.keys() {
            let e = errors.get(key).unwrap();
            let err = e.error.abs();
            debug!("  {}: error {}, {}", key, e, err);
            error += err;
        }
        // let mut error: D = disjoint_targets.iter().map(|(key, _)| errors.get(key).unwrap().error.abs()).sum();
        debug!("step error {:?}", error);
        // Optional/Alternate loss function based on per-region squared errors, weights errors by region size:
        // let error = errors.values().into_iter().map(|e| e.error.clone() * &e.error).sum::<D>().sqrt();
        let components: Vec<regions::Component> = scene.components.iter().map(|c| regions::Component::new(&c)).collect();
        debug!("{} components, num sets {}", components.len(), components.iter().map(|c| c.sets.len().to_string()).collect::<Vec<_>>().join(", "));

        // Include penalties for erroneously-disjoint shapes
        // let mut disjoint_penalties = Vec::<DisjointPenalty>::new();
        let mut total_disjoint_penalty = scene.zero();
        let mut total_contained_penalty = scene.zero();

        debug!("all targets: {}", targets.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<String>>().join(", "));
        let missing_regions: BTreeMap<String, f64> = disjoint_targets.into_iter().filter(|(key, target)| {
            let err = errors.get(key).expect(&format!("No key {} among error keys {}", key, errors.keys().cloned().collect::<Vec<String>>().join(", ")));
            let region_should_exist = target > &0.;
            let region_exists = err.actual_area.clone().filter(|a| !a.is_zero()).is_some();
            if region_should_exist && !region_exists {
                true
            } else {
                false
            }
        }).collect();

        let mut total_missing_disjoint = 0.;
        let mut total_missing_contained = 0.;
        for (key, target) in missing_regions.iter() {
            let set_idxs: Vec<usize> = key.chars().enumerate().filter(|(_, c)| *c != '*' && *c != '-').map(|(idx, _)| idx).collect();
            let n = set_idxs.len();
            let nf = n as f64;
            let centroid: R2<Dual> = set_idxs.iter().map(|idx| sets[*idx].shape.center()).sum::<R2<Dual>>();
            let centroid = R2 { x: centroid.x / nf, y: centroid.y / nf };
            let parents_key = key.replace('-', "*");
            let parent_regions_exist = errors.get(&parents_key).unwrap().actual_area.clone().filter(|a| !a.is_zero()).is_some();
            debug!("missing region {:?}, centroid {:?}, parents {} ({})", set_idxs, centroid, parents_key, parent_regions_exist);
            if parent_regions_exist {
                let mut parents = Vec::<usize>::new();
                for (idx, ch) in parents_key.char_indices() {
                    if ch == '*' {
                        let parent_key = format!("{}{}{}", &key[..idx], Targets::<f64>::idx(idx), &key[idx+1..]);
                        let parent_region_exists = errors.get(&parent_key).unwrap().actual_area.clone().filter(|a| !a.is_zero()).is_some();
                        if parent_region_exists {
                            parents.push(idx);
                        }
                    }
                }
                let np = parents.len() as f64;
                debug!("  {} parents: {}", np, parents.iter().map(|idx| format!("{}", idx)).collect::<Vec<String>>().join(", "));
                for parent_idx in &parents {
                    let center = scene.sets[*parent_idx].shape.center();
                    let distance = center.distance(&centroid);
                    if distance.is_zero() {
                        warn!("  missing region penalty: {}, parent {}, distance {}, skipping", key, parent_idx, &distance);
                    } else {
                        debug!("  missing region penalty: {}, parent {}, distance {}", key, parent_idx, &distance);
                        total_contained_penalty += distance.recip() * target / np;
                    }
                }
                total_missing_contained += target;
            } else {
                set_idxs.iter().for_each(|idx| {
                    let set = &sets[*idx];
                    let distance = set.shape.center().distance(&centroid);
                    debug!("  missing region penalty: {}, shape {}, distance {}", key, idx, &distance);
                    total_disjoint_penalty += distance * target / nf;
                });
                total_missing_disjoint += target;
            }
        }

        if !missing_regions.is_empty() {
            debug!("missing_regions: {:?}, {:?}", total_missing_disjoint, missing_regions);
            debug!("   disjoint: total {}, unscaled penalty {}", total_missing_disjoint, total_disjoint_penalty);
            debug!("  contained: total {}, unscaled penalty {}", total_missing_contained, total_contained_penalty);
        }
        let total_disjoint_penalty_v = total_disjoint_penalty.v();
        if total_disjoint_penalty_v > 0. {
            total_disjoint_penalty = total_disjoint_penalty * (total_missing_disjoint / total_disjoint_penalty_v / targets.total_area);
            debug!("  total_disjoint_penalty: {}", total_disjoint_penalty);
            error += Dual::new(0., total_disjoint_penalty.d());
        }
        let total_contained_penalty_v = total_contained_penalty.v();
        if total_contained_penalty_v > 0. {
            total_contained_penalty = total_contained_penalty * (total_missing_contained / total_contained_penalty_v / targets.total_area);
            debug!("  total_contained_penalty: {}", total_contained_penalty);
            error += Dual::new(0., total_contained_penalty.d());
        }

        debug!("all-in error: {:?}", error);
        Step { inputs, components, targets, total_area, errors, error }
    }

    pub fn shapes(&self) -> Vec<Shape<f64>> {
        self.inputs.iter().map(|(s, _)| s.clone()).collect()
    }

    pub fn n(&self) -> usize {
        self.shapes().len()
    }

    pub fn grad_size(&self) -> usize {
        self.error.1
    }

    pub fn compute_errors(scene: &Scene<D>, targets: &Targets<f64>, total_area: &Dual) -> Errors {
        let none_key = targets.none_key();
        targets.iter().filter_map(|(key, target_area)| {
            if key == &none_key {
                None
            } else {
                let actual_area = scene.area(key);
                let target_frac = target_area / targets.total_area;
                let actual_frac = actual_area.clone().unwrap_or_else(|| scene.zero()).clone() / total_area;
                let error = actual_frac.clone() - target_frac;
                Some((
                    key.clone(),
                    Error {
                        key: key.clone(),
                        actual_area,
                        target_area: target_area.clone(),
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

    pub fn step(&self, max_step_error_ratio: f64) -> Step {
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
        let new_inputs = shapes.iter().zip(self.duals()).map(|(s, duals)| s.step(&duals, &step_vec)).collect::<Vec<Input>>();
        for (cur, (nxt, _)) in shapes.iter().zip(new_inputs.iter()) {
            debug!("  {} -> {:?}", cur, nxt);
        }
        Step::new(new_inputs, self.targets.clone())
    }
}
