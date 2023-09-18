use std::collections::BTreeMap;
use std::fmt::Display;

use log::{info, debug};
use serde::{Deserialize, Serialize};
use tsify::{declare, Tsify};

use crate::ellipses::xyrr::XYRR;
use crate::math::recip::Recip;
use crate::shape::{Input, Shape, Duals};
use crate::{circle::Circle, distance::Distance, scene::Scene, math::is_zero::IsZero, r2::R2, areas::Areas, regions};
use crate::dual::{Dual, D};

#[declare]
pub type Targets = BTreeMap<String, f64>;
#[declare]
pub type Errors = BTreeMap<String, Error>;

#[derive(Clone, Debug, Tsify, Serialize, Deserialize)]
pub struct Diagram {
    pub inputs: Vec<Input>,
    pub components: Vec<regions::Component>,
    pub targets: Targets,
    pub total_target_area: f64,
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
    pub total_target_area: f64,
    pub target_frac: f64,
    pub error: Dual,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{}: err {:.3}, {:.3} / {:.3} = {:.3}, {} → {:.3}",
            self.key, self.error.v(),
            self.target_area, self.total_target_area, self.target_frac,
            self.actual_area.clone().map(|a| format!("{:.3}", a.v())).unwrap_or_else(|| "-".to_string()),
            self.actual_frac.v(),
        )
    }
}

impl Diagram {
    pub fn new(inputs: Vec<Input>, targets: Targets, total_target_area: Option<f64>) -> Diagram {
        let shapes: Vec<Shape<D>> = inputs.iter().map(|(c, duals)| c.dual(duals)).collect();
        let scene = Scene::new(shapes);
        let shapes = &scene.shapes;
        // let duals = intersections.duals;
        let all_key = String::from_utf8(vec![b'*'; scene.len()]).unwrap();
        let mut expanded_targets = targets.clone();
        Areas::expand(&mut expanded_targets);
        let total_target_area = total_target_area.unwrap_or_else(|| {
            expanded_targets
            .get(&all_key)
            .expect(&format!("{} not found among {} keys", all_key, expanded_targets.len()))
            .clone()
        });
        let total_area = scene.area(&all_key).unwrap_or_else(|| scene.zero());
        let errors = Self::compute_errors(&scene, &targets, &total_target_area, &total_area);
        let mut error: D = errors.values().into_iter().map(|e| { e.error.abs() }).sum();
        debug!("diagram, error {:?}", error);
        // Optional/Alternate loss function based on per-region squared errors, weights errors by region size:
        // let error = errors.values().into_iter().map(|e| e.error.clone() * &e.error).sum::<D>().sqrt();
        let components: Vec<regions::Component> = scene.components.iter().map(|c| regions::Component::new(&c)).collect();

        let missing_regions: BTreeMap<Vec<usize>, f64> = errors.iter().filter_map(|(key, error)| {
            if error.actual_area.clone().filter(|a| !a.is_zero()).is_none() && error.target_area > 0. {
                let shape_idxs: Vec<usize> = key.chars().enumerate().filter(|(_, c)| *c != '*' && *c != '-').map(|(idx, _)| idx).collect();
                Some((shape_idxs, error.target_area))
            } else {
                None
            }
        }).collect();

        let total_missing = missing_regions.values().sum::<f64>();
        // Include penalties for erroneously-disjoint shapes
        // let mut disjoint_penalties = Vec::<DisjointPenalty>::new();
        let mut total_disjoint_penalty = Dual::zero(error.d().len());

        for (shape_idxs, target_area) in missing_regions.iter() {
            let n = shape_idxs.len();
            let nf = n as f64;
            let centroid: R2<Dual> = shape_idxs.iter().map(|idx| shapes[*idx].center()).sum::<R2<Dual>>();
            let centroid = R2 { x: centroid.x / nf, y: centroid.y / nf };
            shape_idxs.iter().for_each(|idx| {
                let shape = &shapes[*idx];
                let distance = shape.center().distance(&centroid);
                // debug!("  missing region penalty! {}: {} * {}", shape_idxs.iter().map(|idx| idx.to_string()).collect::<Vec<_>>().join(""), &gap, target_area);
                total_disjoint_penalty += distance * target_area / nf;
            });
        }
        if !missing_regions.is_empty() {
            info!("missing_regions: {:?}", missing_regions);
        }
        total_disjoint_penalty = total_disjoint_penalty.recip() * total_missing;

        if total_disjoint_penalty.v() > 0. {
            info!("  total_disjoint_penalty: {}", total_disjoint_penalty);
            error += Dual::new(total_disjoint_penalty.v(), total_disjoint_penalty.d().iter().map(|d| -d).collect());
            // error += total_disjoint_penalty;
        }

        Diagram { inputs, components, targets, total_target_area, total_area, errors, error }
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

    pub fn compute_errors(scene: &Scene<D>, targets: &Targets, total_target_area: &f64, total_area: &Dual) -> Errors {
        let n = scene.len();
        // let all_key = String::from_utf8(vec![b'*'; n]).unwrap();
        let none_key = String::from_utf8(vec![b'-'; n]).unwrap();
        targets.iter().filter_map(|(key, target_area)| {
            if key == &none_key {
                None
            } else {
                let actual_area = scene.area(key);
                let target_frac = target_area / total_target_area;
                let actual_frac = actual_area.clone().unwrap_or_else(|| scene.zero()).clone() / total_area;
                let error = actual_frac.clone() - target_frac;
                Some((
                    key.clone(),
                    Error {
                        key: key.clone(),
                        actual_area,
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
            (
                match s {
                    Shape::Circle(s) => {
                        if duals.len() != 3 {
                            panic!("expected 3 duals for Circle, got {}: {:?}", duals.len(), duals);
                        }
                        let duals = [ duals[0].clone(), duals[1].clone(), duals[2].clone() ];
                        let [ dx, dy, dr ]: [f64; 3] = duals.map(|d| d.iter().zip(&step_vec).map(|(mask, step)| mask * step).sum());
                        let c = R2 {
                            x: s.c.x + dx,
                            y: s.c.y + dy,
                        };
                        Shape::Circle(Circle { idx: s.idx, c, r: s.r + dr })
                    },
                    Shape::XYRR(e) => {
                        if duals.len() != 4 {
                            panic!("expected 4 duals for XYRR ellipse, got {}: {:?}", duals.len(), duals);
                        }
                        let duals = [
                            duals[0].clone(),
                            duals[1].clone(),
                            duals[2].clone(),
                            duals[3].clone(),
                        ];
                        let [ dcx, dcy, drx, dry ]: [f64; 4] = duals.map(|d| d.iter().zip(&step_vec).map(|(mask, step)| mask * step).sum());
                        let c = e.c + R2 { x: dcx, y: dcy, };
                        let r = e.r + R2 { x: drx, y: dry };
                        Shape::XYRR(XYRR { idx: e.idx, c, r })
                    },
                },
                duals ,
            )
        }).collect::<Vec<Input>>();
        for (cur, (nxt, _)) in shapes.iter().zip(new_inputs.iter()) {
            debug!("  {} -> {:?}", cur, nxt);
        }
        Diagram::new(new_inputs, self.targets.clone(), Some(self.total_target_area))
    }
}
