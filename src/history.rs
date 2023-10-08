use serde::{Serialize, Deserialize};
use tsify::Tsify;

use crate::{shape::Shape, step, model::Model};

#[derive(Debug, Clone, PartialEq, Tsify, Serialize, Deserialize)]
pub struct Step {
    pub error: f64,
    pub shapes: Vec<Shape<f64>>,
}

impl From<step::Step> for Step {
    fn from(s: step::Step) -> Self {
        Step {
            error: s.error.v(),
            shapes: s.shapes.into_iter().map(|s| s.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, Tsify, Serialize, Deserialize)]
pub struct History {
    pub steps: Vec<Step>,
}

impl From<Model> for History {
    fn from(m: Model) -> Self {
        History {
            steps: m.steps.into_iter().map(|s| s.into()).collect(),
        }
    }
}