use std::{collections::HashMap, fmt::Display};

use log::{info, debug};

use crate::{circle::{Circle, Split, Duals}, shapes::Shapes, dual::D, r2::R2, areas::Areas};


pub type Targets = HashMap<String, f64>;
pub type Errors = HashMap<String, Error>;

#[derive(Clone, Debug)]
pub struct Diagram {
    pub inputs: Vec<Split>,
    pub shapes: Shapes,
    pub targets: Targets,
    pub total_target_area: f64,
    pub errors: Errors,
    pub error: D,
}

#[derive(Clone, Debug)]
pub struct Error {
    pub key: String,
    pub actual_area: Option<D>,
    pub total_area: D,
    pub actual_frac: D,
    pub target_area: f64,
    pub total_target_area: f64,
    pub target_frac: f64,
    pub error: D,
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
    pub fn new(inputs: Vec<Split>, targets: HashMap<String, f64>, total_target_area: Option<f64>) -> Diagram {
        let shapes = Shapes::new(&inputs);
        let all_key = String::from_utf8(vec![b'*'; shapes.len()]).unwrap();
        let total_target_area = total_target_area.unwrap_or_else(|| {
            let mut expanded_target = targets.clone();
            Areas::expand(&mut expanded_target);
            expanded_target.get(&all_key).unwrap().clone()
        });
        let errors = Self::compute_errors(&shapes, &targets, total_target_area);
        let error = errors.values().into_iter().map(|e| &e.error).cloned().collect::<Vec<D>>().into_iter().sum();
        Diagram { inputs, shapes, targets, total_target_area: total_target_area.clone(), errors, error }
    }

    pub fn n(&self) -> usize {
        self.shapes.len()
    }

    pub fn compute_errors(shapes: &Shapes, targets: &Targets, total_target_area: f64) -> Errors {
        let n = shapes.len();
        let all_key = String::from_utf8(vec![b'*'; n]).unwrap();
        let none_key = String::from_utf8(vec![b'-'; n]).unwrap();
        let total_area = shapes.area(&all_key).unwrap_or_else(|| shapes.zero());
        targets.iter().filter_map(|(key, target_area)| {
            if key == &none_key {
                None
            } else {
                let actual_area = shapes.area(key);
                let target_frac = target_area / total_target_area;
                let actual_frac = actual_area.clone().unwrap_or_else(|| shapes.zero()).clone() / &total_area;
                let error = (actual_frac.clone() - target_frac).abs();
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

    pub fn step(&mut self, step_size: f64) -> Diagram {
        let error = self.error.clone();
        let error_size = self.error.v();
        let grad_vec = (-error.clone()).d();
        // let max_error = grad_vec.iter().map(|(_, e)| e.error.v()).unwrap().1.error.v();
        let clamped_step_size = f64::min(error_size, step_size);
        let magnitude = grad_vec.iter().map(|d| d * d).sum::<f64>().sqrt();
        let step_vec = grad_vec.iter().map(|d| d / magnitude * clamped_step_size).collect::<Vec<f64>>();
        debug!("  step_vec {:?}", step_vec);
        let shapes = &self.shapes.shapes;
        let new_inputs = shapes.iter().zip(self.duals()).map(|(s, duals)| {
            let updates: [f64; 3] = duals.clone().map(|d| d.iter().zip(&step_vec).map(|(mask, step)| mask * step).sum());
            let c = R2 {
                x: s.c.x + updates[0],
                y: s.c.y + updates[1],
            };
            let r = s.r + updates[2];
            ( Circle { idx: s.idx, c, r }, duals )
        }).collect::<Vec<Split>>();
        debug!("  step_size {}, updates [{}]:", clamped_step_size, step_vec.iter().map(|x| format!("{}", x)).collect::<Vec<String>>().join(", "));
        for (cur, (nxt, _)) in shapes.iter().zip(new_inputs.iter()) {
            debug!("  {} -> {}", cur, nxt);
        }
        let errors = &self.errors;
        for (target, _) in &self.targets {
            let err = errors.get(&target.to_string()).unwrap();
            debug!("  {}", err);
        }
        debug!("  err {:?}", error);
        Diagram::new(new_inputs, self.targets.clone(), Some(self.total_target_area))
    }
}
