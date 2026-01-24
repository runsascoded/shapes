//! WASM bindings for area-proportional Venn diagrams.
//!
//! This crate provides JavaScript/WASM bindings for the apvd-core library,
//! enabling browser-based Venn diagram optimization.

use apvd_core::{
    Model, Step, Targets, TargetsMap, InputSpec, XYRR, D,
    shape::Shape,
};
use log::{info, error};
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;

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
    let level: Option<String> = serde_wasm_bindgen::from_value(level).unwrap();
    let level = apvd_core::parse_log_level(level.as_deref());
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

/// Runs Adam optimizer training on a model.
///
/// Adam (Adaptive Moment Estimation) maintains per-parameter momentum and variance
/// estimates, enabling better convergence for complex optimization landscapes.
/// Particularly useful for mixed shape scenes (e.g., polygon + circle).
///
/// # Arguments
/// * `model` - Model created by [`make_model`].
/// * `learning_rate` - Adam learning rate (typical: 0.001 to 0.1).
/// * `max_steps` - Maximum number of optimization steps.
///
/// # Returns
/// Updated model with training history containing all intermediate steps.
///
/// # Panics
/// If a training step fails due to invalid geometry.
#[wasm_bindgen]
pub fn train_adam(model: JsValue, learning_rate: f64, max_steps: usize) -> JsValue {
    let mut model: Model = serde_wasm_bindgen::from_value(model).unwrap();
    model.train_adam(learning_rate, max_steps).expect("Adam training failed");
    serde_wasm_bindgen::to_value(&model).unwrap()
}

/// Runs robust optimization with Adam, gradient clipping, and backtracking.
///
/// This is the recommended training method. It combines:
/// - Adam optimizer for per-parameter adaptive learning rates
/// - Gradient clipping to prevent catastrophically large steps
/// - Learning rate warmup for stability
/// - Step rejection when error increases significantly
///
/// # Arguments
/// * `model` - Model created by [`make_model`].
/// * `max_steps` - Maximum number of optimization steps.
///
/// # Returns
/// Updated model with training history containing all intermediate steps.
///
/// # Panics
/// If a training step fails due to invalid geometry.
#[wasm_bindgen]
pub fn train_robust(model: JsValue, max_steps: usize) -> JsValue {
    let mut model: Model = serde_wasm_bindgen::from_value(model).unwrap();
    model.train_robust(max_steps).expect("Robust training failed");
    serde_wasm_bindgen::to_value(&model).unwrap()
}

/// Performs a single gradient descent step with gradient clipping (recommended).
///
/// Uses fixed learning rate with gradient clipping for stable updates.
/// This is the recommended method - it prevents the oscillation that occurs
/// with error-scaled step sizes.
///
/// # Arguments
/// * `step` - Current optimization state from [`make_step`] or a previous [`step`] call.
/// * `learning_rate` - Fixed learning rate (typical: 0.01 to 0.1, default 0.05).
///
/// # Returns
/// New [`Step`] with updated shape positions.
#[wasm_bindgen]
pub fn step(step: JsValue, learning_rate: f64) -> JsValue {
    let step: Step = serde_wasm_bindgen::from_value(step).unwrap();
    // Use sensible defaults for clipping
    let step = step.step_clipped(learning_rate, 0.5, 1.0).expect("Step failed");
    serde_wasm_bindgen::to_value(&step).unwrap()
}

/// Legacy step function that scales step size by error.
///
/// **Deprecated**: Use [`step`] instead. This function can cause oscillation
/// when error is high because step_size = error * max_step_error_ratio.
///
/// # Arguments
/// * `step` - Current optimization state.
/// * `max_step_error_ratio` - Learning rate scaling factor.
#[wasm_bindgen]
pub fn step_legacy(step: JsValue, max_step_error_ratio: f64) -> JsValue {
    let step: Step = serde_wasm_bindgen::from_value(step).unwrap();
    let step = step.step(max_step_error_ratio).expect("Step failed");
    serde_wasm_bindgen::to_value(&step).unwrap()
}

/// Check if a step has converged below a custom threshold.
///
/// Use this to implement user-configurable convergence thresholds.
/// The step.converged field uses the default threshold (1e-10), but
/// this function lets you check against any threshold.
///
/// # Arguments
/// * `step` - Current optimization state.
/// * `threshold` - Custom convergence threshold (e.g., 1e-6 for fast, 1e-14 for precise).
///
/// # Returns
/// True if step.error < threshold.
#[wasm_bindgen]
pub fn is_converged(step: JsValue, threshold: f64) -> bool {
    let step: Step = serde_wasm_bindgen::from_value(step).unwrap();
    step.error.v() < threshold
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

/// Checks if any polygon shapes in the given step are self-intersecting.
///
/// Self-intersecting polygons have edges that cross each other, which
/// invalidates area calculations and causes visual artifacts.
///
/// # Arguments
/// * `step` - Current optimization state.
///
/// # Returns
/// Array of strings describing any validity issues (empty if valid).
#[wasm_bindgen]
pub fn check_polygon_validity(step: JsValue) -> JsValue {
    let step: Step = serde_wasm_bindgen::from_value(step).unwrap();
    let mut issues = Vec::<String>::new();

    for (idx, shape) in step.shapes.iter().enumerate() {
        if let Shape::Polygon(poly) = shape {
            let poly_f64 = poly.v();
            if poly_f64.is_self_intersecting() {
                issues.push(format!("Shape {} (polygon) is self-intersecting", idx));
            }
        }
    }

    serde_wasm_bindgen::to_value(&issues).unwrap()
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
