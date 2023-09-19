#![allow(mixed_script_confusables)]

#[macro_use]
extern crate approx;
extern crate console_error_panic_hook;

pub mod areas;
pub mod circle;
pub mod component;
pub mod contains;
pub mod d5;
pub mod distance;
pub mod dual;
pub mod edge;
pub mod ellipses;
pub mod float_arr;
pub mod float_vec;
pub mod float_wrap;
pub mod fmt;
pub mod gap;
pub mod intersect;
pub mod intersection;
pub mod node;
pub mod math;
pub mod model;
pub mod r2;
pub mod region;
pub mod regions;
pub mod roots;
pub mod rotate;
pub mod scene;
pub mod segment;
pub mod shape;
pub mod sqrt;
pub mod step;
pub mod theta_points;
pub mod to;
pub mod transform;
pub mod trig;
pub mod zero;
pub mod js_dual;

use areas::Areas;
use shape::Input;
use step::Step;
use dual::D;
use ellipses::xyrr::XYRR;
use log::{LevelFilter, info, error};

use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use crate::step::Targets;
use crate::model::Model;

pub fn deser_log_level(level: JsValue) -> LevelFilter {
    let level: Option<String> = serde_wasm_bindgen::from_value(level).unwrap();
    let level = match level.as_deref() {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") | Some("") | None => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        Some(level) => panic!("invalid log level: {}", level),
    };
    level
}

#[wasm_bindgen]
pub fn init_logs() {
    match log::set_logger(&DEFAULT_LOGGER) {
        Ok(_) => info!("Initialized console.logger"),
        Err(e) => error!("failed to set console.logger: {}", e),
    };
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn update_log_level(level: JsValue) {
    let level = deser_log_level(level);
    log::set_max_level(level);
}

#[wasm_bindgen]
pub fn make_step(inputs: JsValue, targets: JsValue) -> JsValue {
    let inputs: Vec<Input> = serde_wasm_bindgen::from_value(inputs).unwrap();
    let targets: Targets = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    let step = Step::new(inputs, targets, None);
    serde_wasm_bindgen::to_value(&step).unwrap()
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
    println!("log level: {}", log::max_level());
    serde_wasm_bindgen::to_value(&model).unwrap()
}

#[wasm_bindgen]
pub fn step(step: JsValue, max_step_error_ratio: f64) -> JsValue {
    let step: Step = serde_wasm_bindgen::from_value(step).unwrap();
    let step = step.step(max_step_error_ratio);
    let step = serde_wasm_bindgen::to_value(&step.clone()).unwrap();
    step
}

#[wasm_bindgen]
pub fn expand_areas(targets: JsValue) -> JsValue {
    let mut targets: Targets = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    Areas::expand(&mut targets);
    serde_wasm_bindgen::to_value(&targets).unwrap()
}

#[wasm_bindgen]
pub fn xyrr_unit(xyrr: JsValue) -> JsValue {
    let xyrr: XYRR<D> = serde_wasm_bindgen::from_value(xyrr).unwrap();
    let points = xyrr.unit_intersections();
    serde_wasm_bindgen::to_value(&points).unwrap()
}
