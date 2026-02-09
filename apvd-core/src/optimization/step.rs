use std::collections::BTreeMap;
use std::fmt::Display;

use log::{debug, warn};
use serde::{Deserialize, Serialize};
use tsify::{declare, Tsify};

use crate::math::recip::Recip;
use crate::shape::{Shape, Shapes, InputSpec};
use crate::{distance::Distance, error::SceneError, scene::Scene, math::is_zero::IsZero, r2::R2, targets::Targets, regions};
use crate::dual::{Dual, D};
use super::adam::AdamState;

#[declare]
pub type Errors = BTreeMap<String, Error>;

/// Convergence threshold - below this error, consider the optimization converged.
pub const CONVERGENCE_THRESHOLD: f64 = 1e-10;

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Step {
    pub shapes: Vec<Shape<D>>,
    pub components: Vec<regions::Component>,
    pub targets: Targets<f64>,
    pub total_area: Dual,
    pub errors: Errors,
    pub error: Dual,
    /// True if error is below convergence threshold (1e-10).
    /// Frontend should stop iterating when this is true.
    pub converged: bool,
    /// Breakdown of penalty terms (values only, gradients are folded into `error`)
    pub penalties: Penalties,
}

/// Classification of a per-region error.
#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ErrorKind {
    /// Region exists and has target > 0, but actual != target
    AreaMismatch {
        /// Positive = too large, negative = too small
        signed_error: f64,
    },
    /// Region should exist (target > 0) but doesn't (actual = 0 or None)
    MissingRegion {
        target_frac: f64,
    },
    /// Region exists (actual > 0) but shouldn't (target = 0)
    ExtraRegion {
        actual_frac: f64,
    },
}

/// Breakdown of penalty terms applied during optimization.
///
/// These values represent the scalar magnitudes of each penalty.
/// Gradients are folded into the `Step::error` dual for optimization,
/// but the values here are the raw (pre-zeroing) penalty magnitudes.
#[derive(Clone, Debug, Default, Tsify, Serialize, Deserialize)]
pub struct Penalties {
    /// Penalty for shapes that should overlap but are completely disjoint
    pub disjoint: f64,
    /// Penalty for shapes where parent regions exist but target intersection doesn't
    pub contained: f64,
    /// Polygon self-intersection penalty
    pub self_intersection: f64,
    /// Polygon regularity penalty (edge variance + convexity)
    pub regularity: f64,
}

impl Penalties {
    pub fn total(&self) -> f64 {
        self.disjoint + self.contained + self.self_intersection + self.regularity
    }
}

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Error {
    pub key: String,
    pub actual_area: Option<f64>,
    pub actual_frac: f64,
    pub target_area: f64,
    pub target_frac: f64,
    pub error: Dual,
    /// Classification of this error
    pub kind: ErrorKind,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{}: err {:.3}, target {:.3} ({:.3}), actual {} → {:.3}",
            self.key, self.error.v(),
            self.target_area, self.target_frac,
            self.actual_area.map(|a| format!("{:.3}", a)).unwrap_or_else(|| "-".to_string()),
            self.actual_frac,
        )
    }
}

impl Step {
    pub fn new(input_specs: Vec<InputSpec>, targets: Targets<f64>) -> Result<Step, SceneError> {
        let shapes = Shapes::from_vec(&input_specs);
        Step::nxt(shapes, targets)
    }
    pub fn nxt(shapes: Vec<Shape<D>>, targets: Targets<f64>) -> Result<Step, SceneError> {
        let scene = Scene::new(shapes)?;
        let sets = &scene.sets;
        let all_key = "*".repeat(scene.len());
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
        let components: Vec<regions::Component> = scene.components.iter().map(regions::Component::new).collect();
        debug!("{} components, num sets {}", components.len(), components.iter().map(|c| c.sets.len().to_string()).collect::<Vec<_>>().join(", "));

        // Include penalties for erroneously-disjoint shapes
        // let mut disjoint_penalties = Vec::<DisjointPenalty>::new();
        let mut total_disjoint_penalty = scene.zero();
        let mut total_contained_penalty = scene.zero();

        debug!("all targets: {}", targets.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<String>>().join(", "));
        let missing_regions: BTreeMap<String, f64> = disjoint_targets.into_iter().filter(|(key, target)| {
            let err = errors.get(key).unwrap_or_else(|| panic!("No key {} among error keys {}", key, errors.keys().cloned().collect::<Vec<String>>().join(", ")));
            let region_should_exist = target > &0.;
            let region_exists = err.actual_area.filter(|a| !a.is_zero()).is_some();
            region_should_exist && !region_exists
        }).collect();

        let mut total_missing_disjoint = 0.;
        let mut total_missing_contained = 0.;
        for (key, target) in missing_regions.iter() {
            let set_idxs: Vec<usize> = key.chars().enumerate().filter(|(_, c)| *c != '*' && *c != '-').map(|(idx, _)| idx).collect();
            let n = set_idxs.len();
            let nf = n as f64;
            let centroid: R2<Dual> = set_idxs.iter().map(|idx| sets[*idx].borrow().shape.center()).sum::<R2<Dual>>();
            let centroid = R2 { x: centroid.x / nf, y: centroid.y / nf };
            let parents_key = key.replace('-', "*");
            let parent_regions_exist = errors.get(&parents_key).unwrap().actual_area.filter(|a| !a.is_zero()).is_some();
            debug!("missing region {:?}, centroid {:?}, parents {} ({})", set_idxs, centroid, parents_key, parent_regions_exist);
            if parent_regions_exist {
                let mut parents = Vec::<usize>::new();
                for (idx, ch) in parents_key.char_indices() {
                    if ch == '*' {
                        let parent_key = format!("{}{}{}", &key[..idx], Targets::<f64>::idx(idx), &key[idx+1..]);
                        let parent_region_exists = errors.get(&parent_key).unwrap().actual_area.filter(|a| !a.is_zero()).is_some();
                        if parent_region_exists {
                            parents.push(idx);
                        }
                    }
                }
                let np = parents.len() as f64;
                debug!("  {} parents: {}", np, parents.iter().map(|idx| format!("{}", idx)).collect::<Vec<String>>().join(", "));
                for parent_idx in &parents {
                    let center = sets[*parent_idx].borrow().shape.center();
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
                    let distance = set.borrow().shape.center().distance(&centroid);
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

        // Add polygon regularization penalties (self-intersection, convexity, edge regularity)
        let mut total_self_intersection_penalty = scene.zero();
        let mut total_regularity_penalty = scene.zero();
        for set in sets.iter() {
            if let crate::shape::Shape::Polygon(poly) = &set.borrow().shape {
                // Self-intersection penalty (high weight - this is a serious problem)
                let self_int_penalty = poly.self_intersection_penalty_dual();
                if self_int_penalty.v() > 0.0 {
                    debug!("  self-intersection penalty: {}", self_int_penalty.v());
                    total_self_intersection_penalty = total_self_intersection_penalty + self_int_penalty;
                }

                // Regularity penalty (edge variance + convexity, lower weight)
                let reg_penalty = poly.regularity_penalty();
                if reg_penalty.v() > 0.0 {
                    debug!("  regularity penalty: {}", reg_penalty.v());
                    // Scale down regularity penalty relative to area error
                    total_regularity_penalty = total_regularity_penalty + reg_penalty * 0.01;
                }
            }
        }

        let self_int_v = total_self_intersection_penalty.v();
        let regularity_v = total_regularity_penalty.v();
        if self_int_v > 0.0 {
            debug!("  total_self_intersection_penalty: {}", self_int_v);
            error += Dual::new(0., total_self_intersection_penalty.d());
        }
        if regularity_v > 0.0 {
            debug!("  total_regularity_penalty: {}", regularity_v);
            error += Dual::new(0., total_regularity_penalty.d());
        }

        let penalties = Penalties {
            disjoint: total_disjoint_penalty.v(),
            contained: total_contained_penalty.v(),
            self_intersection: self_int_v,
            regularity: regularity_v,
        };

        // Take shapes back from `scene`
        let shapes = sets.iter().map(|s| s.borrow().to_owned().shape).collect::<Vec<Shape<D>>>();

        let converged = error.v() < CONVERGENCE_THRESHOLD;
        debug!("all-in error: {:?}, converged: {}", error, converged);
        Ok(Step { shapes, components, targets, total_area, errors, error, converged, penalties })
    }

    pub fn n(&self) -> usize {
        self.shapes.len()
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
                let actual_frac_v = actual_frac.v();
                let kind = if target_frac > 0.0 && (actual_area.is_none() || actual_frac_v < 1e-12) {
                    ErrorKind::MissingRegion { target_frac }
                } else if target_frac < 1e-12 && actual_frac_v > 1e-12 {
                    ErrorKind::ExtraRegion { actual_frac: actual_frac_v }
                } else {
                    ErrorKind::AreaMismatch { signed_error: error.v() }
                };
                Some((
                    key.clone(),
                    Error {
                        key: key.clone(),
                        actual_area: actual_area.map(|a| a.v()),
                        target_area: *target_area,
                        actual_frac: actual_frac_v,
                        target_frac,
                        error,
                        kind,
                    }
                ))
            }
        }).collect()
    }

    // pub fn duals(&self) -> Vec<Vec<InitDual>> {
        // self.shapes.iter().map(|(_, duals)| duals.clone()).collect()
    // }

    pub fn step(&self, max_step_error_ratio: f64) -> Result<Step, SceneError> {
        let error = self.error.clone();
        // let error = self.errors.values().into_iter().map(|e| e.error.clone() * &e.error).sum::<D>().sqrt();
        let error_size = &error.v();
        let grad_vec = (-error.clone()).d();

        let step_size = error_size * max_step_error_ratio;
        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>().sqrt();

        // If magnitude is zero (no gradient), skip the step and return a clone with same shapes
        if magnitude == 0. || magnitude.is_nan() {
            debug!("  skipping step: magnitude is {} (error_size: {}, grad_vec: {:?})", magnitude, error_size, grad_vec);
            return Step::nxt(self.shapes.clone(), self.targets.clone());
        }

        let grad_scale = step_size / magnitude;
        let step_vec = grad_vec.iter().map(|grad| grad * grad_scale).collect::<Vec<f64>>();

        debug!("  err {:?}", error);
        debug!("  step_size {}, magnitude {}, grad_scale {}", step_size, magnitude, grad_scale);
        debug!("  step_vec {:?}", step_vec);
        let shapes = &self.shapes;
        let new_shapes = shapes.iter().map(|s| s.step(&step_vec)).collect::<Vec<Shape<D>>>();
        for (cur, nxt) in shapes.iter().zip(new_shapes.iter()) {
            debug!("  {} -> {:?}", cur.v(), nxt.v());
        }
        Step::nxt(new_shapes, self.targets.clone())
    }

    /// Take an optimization step using Adam optimizer.
    ///
    /// Unlike vanilla gradient descent which uses `step_size = error * max_step_error_ratio`,
    /// Adam maintains per-parameter momentum and variance estimates, enabling:
    /// - Escape from local minima via momentum
    /// - Adaptive per-parameter learning rates
    /// - Smoother convergence with less oscillation
    pub fn step_with_adam(&self, adam: &mut AdamState, learning_rate: f64) -> Result<Step, SceneError> {
        let error = self.error.clone();
        let grad_vec = (-error.clone()).d();

        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>().sqrt();

        // If magnitude is zero (no gradient), skip the step and return a clone with same shapes
        if magnitude == 0. || magnitude.is_nan() {
            debug!("  skipping step: magnitude is {} (grad_vec: {:?})", magnitude, grad_vec);
            return Step::nxt(self.shapes.clone(), self.targets.clone());
        }

        // Adam computes the step vector using momentum and variance estimates
        let step_vec = adam.step(&grad_vec, learning_rate);

        debug!("  err {:?} (Adam step {})", error, adam.t);
        debug!("  learning_rate {}, magnitude {}", learning_rate, magnitude);
        debug!("  grad_vec {:?}", grad_vec);
        debug!("  step_vec {:?}", step_vec);

        let shapes = &self.shapes;
        let new_shapes = shapes.iter().map(|s| s.step(&step_vec)).collect::<Vec<Shape<D>>>();
        for (cur, nxt) in shapes.iter().zip(new_shapes.iter()) {
            debug!("  {} -> {:?}", cur.v(), nxt.v());
        }
        Step::nxt(new_shapes, self.targets.clone())
    }

    /// Take an optimization step with gradient clipping.
    ///
    /// Unlike vanilla `step()` which can take very large steps when error is high,
    /// this method clips gradients to prevent oscillation:
    /// - Per-component clipping: each gradient is clamped to [-max_grad, max_grad]
    /// - L2 norm clipping: total gradient magnitude is clamped to max_grad_norm
    /// - Fixed learning rate: doesn't scale with error, providing more stable updates
    ///
    /// This is a simpler alternative to Adam that doesn't require persistent state.
    pub fn step_clipped(&self, learning_rate: f64, max_grad_value: f64, max_grad_norm: f64) -> Result<Step, SceneError> {
        let error = self.error.clone();
        let grad_vec = (-error.clone()).d();

        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>().sqrt();

        // If magnitude is zero (no gradient), skip the step
        if magnitude == 0. || magnitude.is_nan() {
            debug!("  skipping step: magnitude is {} (grad_vec: {:?})", magnitude, grad_vec);
            return Step::nxt(self.shapes.clone(), self.targets.clone());
        }

        // Clip by value (per-component)
        let mut clipped: Vec<f64> = grad_vec.iter()
            .map(|&g| g.clamp(-max_grad_value, max_grad_value))
            .collect();

        // Clip by L2 norm
        let clipped_norm: f64 = clipped.iter().map(|g| g * g).sum::<f64>().sqrt();
        if clipped_norm > max_grad_norm {
            let scale = max_grad_norm / clipped_norm;
            for g in &mut clipped {
                *g *= scale;
            }
        }

        // Apply fixed learning rate (not scaled by error)
        let step_vec: Vec<f64> = clipped.iter().map(|&g| g * learning_rate).collect();

        debug!("  err {:?} (clipped step)", error);
        debug!("  learning_rate {}, original magnitude {}, clipped norm {}", learning_rate, magnitude, clipped_norm.min(max_grad_norm));
        debug!("  step_vec {:?}", step_vec);

        let shapes = &self.shapes;
        let new_shapes = shapes.iter().map(|s| s.step(&step_vec)).collect::<Vec<Shape<D>>>();
        Step::nxt(new_shapes, self.targets.clone())
    }
}
