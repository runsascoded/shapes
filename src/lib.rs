#[macro_use]
extern crate approx;
extern crate console_error_panic_hook;

mod areas;
mod circle;
mod diagram;
mod deg;
mod dual;
mod edge;
mod node;
mod math;
mod model;
mod r2;
mod region;
mod regions;
mod intersections;
mod zero;
mod js_dual;

use circle::Input;
use diagram::Diagram;
use log::{LevelFilter, debug, info};
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use crate::diagram::{Targets};
use crate::model::Model;

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
pub fn make_diagram(inputs: JsValue, targets: JsValue) -> JsValue {
    let inputs: Vec<Input> = serde_wasm_bindgen::from_value(inputs).unwrap();
    let targets: Targets = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    let diagram = Diagram::new(inputs, targets, None);
    serde_wasm_bindgen::to_value(&diagram).unwrap()
}

#[wasm_bindgen]
pub fn make_model(inputs: JsValue, targets: JsValue) -> JsValue {
    let inputs: Vec<Input> = serde_wasm_bindgen::from_value(inputs).unwrap();
    let targets: Targets = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    let model = Model::new(inputs, targets);
    serde_wasm_bindgen::to_value(&model).unwrap()
}

#[wasm_bindgen]
pub fn train(model: JsValue, max_step_error_ratio: f64, max_steps: usize) -> JsValue {
    let mut model: Model = serde_wasm_bindgen::from_value(model).unwrap();
    model.train(max_step_error_ratio, max_steps);
    serde_wasm_bindgen::to_value(&model).unwrap()
}

#[wasm_bindgen]
pub fn step(diagram: JsValue, max_step_error_ratio: f64) -> JsValue {
    let diagram: Diagram = serde_wasm_bindgen::from_value(diagram).unwrap();
    let diagram = diagram.step(max_step_error_ratio);
    let diagram = serde_wasm_bindgen::to_value(&diagram.clone()).unwrap();
    diagram
}
