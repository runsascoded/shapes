#[macro_use]
extern crate approx;
extern crate console_error_panic_hook;

mod areas;
mod circle;
mod diagram;
mod deg;
mod dual;
mod edge;
mod intersection;
mod math;
mod model;
mod r2;
mod region;
mod intersections;
mod zero;
mod js_dual;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use circle::Input;
use diagram::Diagram;
use dual::Dual;
use log::info;
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::console;
use crate::circle::Circle;
use crate::diagram::{Error, Targets};
use crate::r2::R2;
use tsify::{Tsify};

#[derive(Tsify, Serialize, Deserialize)]
pub struct Model {
    pub steps: Vec<Diagram>,
    pub repeat_idx: Option<usize>,
    pub min_idx: usize,
    pub min_step: Diagram,
    pub error: Dual,
}

impl From<model::Model> for Model {
    fn from(m: model::Model) -> Self {
        let min_step = m.min_step.borrow().clone();
        let error = min_step.error.clone();
        Model {
            steps: m.steps.iter().map(|d| From::from(d.borrow().clone())).collect(),
            repeat_idx: m.repeat_idx,
            min_idx: m.min_idx,
            min_step,
            error,
        }
    }
}

#[wasm_bindgen]
pub fn make_diagram(circles: JsValue, targets: JsValue) -> JsValue {
    log::set_logger(&DEFAULT_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);
    console_error_panic_hook::set_once();
    let inputs: Vec<Input> = serde_wasm_bindgen::from_value(circles).unwrap();
    // console::log_2(&"web-sys fit".into(), circles);
    // let circles: Vec<Circle<JsDual>> = serde_wasm_bindgen::from_value(circles.clone()).unwrap();
    // let circles: Vec<Split> = circles.iter().map(|c| {
    //     (
    //         Circle {
    //             idx: c.idx,
    //             c: R2 { x: c.c.x.v, y: c.c.y.v },
    //             r: c.r.v,
    //         },
    //         [c.c.x.d.clone(), c.c.y.d.clone(), c.r.d.clone(), ],
    //     )
    // }).collect();
    // console::log_1(&"made splits".into());
    //
    let targets: Targets = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    // console::log_1(&"target tuples".into());
    // let targets: Targets = targets.into_iter().collect();
    // console::log_1(&"targets".into());
    let diagram = Diagram::new(inputs, targets, None);
    // let js_diagram: JsDiagram = From::from(Rc::new(RefCell::new(diagram)));
    serde_wasm_bindgen::to_value(&diagram).unwrap()
}

// #[wasm_bindgen]
// pub fn step(circles: &JsValue, targets: &JsValue, step_size: f64) -> JsValue {
//     log::set_logger(&DEFAULT_LOGGER).unwrap();
//     log::set_max_level(log::LevelFilter::Info);
//     console_error_panic_hook::set_once();
//     console::log_2(&"web-sys fit".into(), circles);
//     let circles: Vec<Circle<JsDual>> = serde_wasm_bindgen::from_value(circles.clone()).unwrap();
//     let circles: Vec<Split> = circles.iter().map(|c| {
//         (
//             Circle {
//                 idx: c.idx,
//                 c: R2 { x: c.c.x.v, y: c.c.y.v },
//                 r: c.r.v,
//             },
//             [ c.c.x.d.clone(), c.c.y.d.clone(), c.r.d.clone(), ],
//         )
//     }).collect();
//     console::log_1(&"made splits".into());
//
//     let targets: Vec<(String, f64)> = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
//     console::log_1(&"target tuples".into());
//     let targets: Targets = targets.into_iter().collect();
//     console::log_1(&"targets".into());
//     let mut diagram = Diagram::new(circles, targets, None);
//     diagram.step(step_size);
//     let js_diagram: JsDiagram = From::from(Rc::new(RefCell::new(diagram)));
//     // let shapes = diagram.shapes.shapes;
//     // let errors = diagram.errors;
//     // let error = diagram.error;
//     // let model = Model::new(circles, targets, step_size, max_steps);
//     // console::log_1(&"model".into());
//     // let js_model: JsModel = model.into();
//     // console::log_1(&"js_model".into());
//     serde_wasm_bindgen::to_value(&js_diagram).unwrap()
// }

// #[wasm_bindgen]
// pub fn fit(circles: &JsValue, targets: &JsValue, step_size: f64, max_steps: usize) -> JsValue {
//     log::set_logger(&DEFAULT_LOGGER).unwrap();
//     log::set_max_level(log::LevelFilter::Info);
//     console_error_panic_hook::set_once();
//     console::log_2(&"web-sys fit".into(), circles);
//     let circles: Vec<Circle<JsDual>> = serde_wasm_bindgen::from_value(circles.clone()).unwrap();
//     let circles: Vec<Split> = circles.iter().map(|c| {
//         (
//             Circle {
//                 idx: c.idx,
//                 c: R2 { x: c.c.x.v, y: c.c.y.v },
//                 r: c.r.v,
//             },
//             [ c.c.x.d.clone(), c.c.y.d.clone(), c.r.d.clone(), ],
//         )
//     }).collect();
//     console::log_1(&"made splits".into());
//
//     let targets: Vec<(String, f64)> = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
//     console::log_1(&"target tuples".into());
//     let targets: Targets = targets.into_iter().collect();
//     console::log_1(&"targets".into());
//     let model = model::Model::new(circles, targets, step_size, max_steps);
//     console::log_1(&"model".into());
//     let js_model: Model = model.into();
//     console::log_1(&"js_model".into());
//     serde_wasm_bindgen::to_value(&js_model).unwrap()
// }
