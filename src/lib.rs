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

use circle::Input;
use diagram::Diagram;
use dual::Dual;
use log::LevelFilter;
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::console;
use crate::diagram::{Targets};
use tsify::{Tsify};

#[wasm_bindgen]
pub fn init_logs(level: JsValue) {
    let level: Option<String> = serde_wasm_bindgen::from_value(level).unwrap();
    log::set_logger(&DEFAULT_LOGGER).unwrap();
    let level = match level.as_deref() {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") | Some("") | None => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        Some(level) => panic!("invalid log level: {}", level),
    };
    log::set_max_level(level);
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn make_diagram(circles: JsValue, targets: JsValue) -> JsValue {
    let inputs: Vec<Input> = serde_wasm_bindgen::from_value(circles).unwrap();
    let targets: Targets = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    let diagram = Diagram::new(inputs, targets, None);
    serde_wasm_bindgen::to_value(&diagram).unwrap()
}

#[wasm_bindgen]
pub fn step(diagram: JsValue, step_size: f64) -> JsValue {
    let diagram: Diagram = serde_wasm_bindgen::from_value(diagram).unwrap();
    let diagram = diagram.step(step_size);
    let diagram = serde_wasm_bindgen::to_value(&diagram.clone()).unwrap();
    diagram
}

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
