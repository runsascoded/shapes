#![allow(mixed_script_confusables)]

#[cfg_attr(not(test), allow(unused_imports))]
#[macro_use]
extern crate approx;
extern crate console_error_panic_hook;

// Organized modules
pub mod analysis;
pub mod geometry;
pub mod math;
pub mod optimization;

// Re-exports for backwards compatibility
pub use geometry::circle;
pub use geometry::ellipses;
pub use geometry::r2;
pub use geometry::rotate;
pub use geometry::shape;
pub use geometry::transform;

pub use optimization::history;
pub use optimization::model;
pub use optimization::step;
pub use optimization::targets;

pub use analysis::component;
pub use analysis::contains;
pub use analysis::distance;
pub use analysis::edge;
pub use analysis::gap;
pub use analysis::hull;
pub use analysis::intersect;
pub use analysis::intersection;
pub use analysis::node;
pub use analysis::region;
pub use analysis::regions;
pub use analysis::scene;
pub use analysis::segment;
pub use analysis::set;
pub use analysis::theta_points;

// Re-export math utilities for backwards compatibility
pub use math::d5;
pub use math::float_arr;
pub use math::float_wrap;
pub use math::roots;
pub use math::sqrt;
pub use math::trig;
pub use math::zero;

// Utility modules
pub mod coord_getter;
pub mod dual;
pub mod duals;
pub mod error;
pub mod fmt;
pub mod js_dual;
pub mod to;

use targets::Targets;
use shape::InputSpec;
use step::Step;
use dual::D;
use ellipses::xyrr::XYRR;
use log::{LevelFilter, info, error};

use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use crate::targets::TargetsMap;
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

/// Initializes the logging system for WASM.
///
/// Sets up console logging and panic hooks for better error reporting in the browser.
/// Should be called once at application startup.
#[wasm_bindgen]
pub fn init_logs() {
    match log::set_logger(&DEFAULT_LOGGER) {
        Ok(_) => info!("Initialized console.logger"),
        Err(e) => error!("failed to set console.logger: {}", e),
    };
    console_error_panic_hook::set_once();
}

/// Updates the log level filter.
///
/// # Arguments
/// * `level` - Log level string: "error", "warn", "info", "debug", or "trace".
///   Defaults to "info" if empty or null.
#[wasm_bindgen]
pub fn update_log_level(level: JsValue) {
    let level = deser_log_level(level);
    log::set_max_level(level);
}

/// Computes a single optimization step for area-proportional Venn diagrams.
///
/// # Arguments
/// * `inputs` - Array of shape specifications with their trainable parameters.
/// * `targets` - Map of region keys to target area sizes.
///
/// # Returns
/// A [`Step`] containing current shapes, computed areas, and error gradients.
///
/// # Panics
/// If the scene cannot be constructed (e.g., invalid geometry).
#[wasm_bindgen]
pub fn make_step(inputs: JsValue, targets: JsValue) -> JsValue {
    let inputs: Vec<InputSpec> = serde_wasm_bindgen::from_value(inputs).unwrap();
    let targets: TargetsMap<f64> = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    let step = Step::new(inputs, targets.into()).expect("Failed to create step");
    serde_wasm_bindgen::to_value(&step).unwrap()
}

/// Creates an optimization model for area-proportional Venn diagrams.
///
/// # Arguments
/// * `inputs` - Array of shape specifications (Circle, XYRR, or XYRRT) with their
///   trainable parameter flags.
/// * `targets` - Map of region keys to target area sizes. Keys use characters
///   to indicate set membership (e.g., "10" = in set 0 only, "11" = in both sets).
///
/// # Returns
/// A [`Model`] ready for training via [`train`].
///
/// # Panics
/// If the scene cannot be constructed (e.g., invalid geometry).
#[wasm_bindgen]
pub fn make_model(inputs: JsValue, targets: JsValue) -> JsValue {
    let inputs: Vec<InputSpec> = serde_wasm_bindgen::from_value(inputs).unwrap();
    let targets: TargetsMap<f64> = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    let model = Model::new(inputs, targets).expect("Failed to create model");
    serde_wasm_bindgen::to_value(&model).unwrap()
}

/// Runs gradient descent training on a model.
///
/// # Arguments
/// * `model` - Model created by [`make_model`].
/// * `max_step_error_ratio` - Stop if error reduction ratio falls below this threshold.
/// * `max_steps` - Maximum number of optimization steps.
///
/// # Returns
/// Updated model with training history containing all intermediate steps.
///
/// # Panics
/// If a training step fails due to invalid geometry.
#[wasm_bindgen]
pub fn train(model: JsValue, max_step_error_ratio: f64, max_steps: usize) -> JsValue {
    let mut model: Model = serde_wasm_bindgen::from_value(model).unwrap();
    model.train(max_step_error_ratio, max_steps).expect("Training failed");
    serde_wasm_bindgen::to_value(&model).unwrap()
}

/// Performs a single gradient descent step.
///
/// # Arguments
/// * `step` - Current optimization state from [`make_step`] or a previous [`step`] call.
/// * `max_step_error_ratio` - Learning rate scaling factor.
///
/// # Returns
/// New [`Step`] with updated shape positions.
///
/// # Panics
/// If the step fails due to invalid geometry.
#[wasm_bindgen]
pub fn step(step: JsValue, max_step_error_ratio: f64) -> JsValue {
    let step: Step = serde_wasm_bindgen::from_value(step).unwrap();
    let step = step.step(max_step_error_ratio).expect("Step failed");
    serde_wasm_bindgen::to_value(&step).unwrap()
}

/// Expands target specifications into fully-qualified region targets.
///
/// Handles inclusive ("1*") and exclusive ("10") region specifications,
/// expanding wildcards and computing disjoint region targets.
///
/// # Arguments
/// * `targets` - Map of region patterns to target sizes.
///
/// # Returns
/// Expanded [`Targets`] with all region keys fully specified.
#[wasm_bindgen]
pub fn expand_targets(targets: JsValue) -> JsValue {
    let targets: TargetsMap<f64> = serde_wasm_bindgen::from_value(targets.clone()).unwrap();
    let targets = Targets::new(targets);
    serde_wasm_bindgen::to_value(&targets).unwrap()
}

/// Computes intersection points between an axis-aligned ellipse and the unit circle.
///
/// Used internally for ellipse-ellipse intersection calculations.
///
/// # Arguments
/// * `xyrr` - Axis-aligned ellipse specification.
///
/// # Returns
/// Array of intersection points on the unit circle.
#[wasm_bindgen]
pub fn xyrr_unit(xyrr: JsValue) -> JsValue {
    let xyrr: XYRR<D> = serde_wasm_bindgen::from_value(xyrr).unwrap();
    let points = xyrr.unit_intersections();
    serde_wasm_bindgen::to_value(&points).unwrap()
}
