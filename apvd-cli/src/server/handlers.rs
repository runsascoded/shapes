//! RPC handler functions for all JSON-RPC methods.

use std::sync::Arc;

use serde_json::Value;
use tokio::sync::mpsc;

use apvd_core::{
    InputSpec, Step, TargetsMap, Shape,
    StorageStrategy,
};

use crate::trace::{TraceData, TraceFileV2, TrainResult};

use super::protocol::{
    ServerConfig, JsonRpcResponse, JsonRpcError,
    StepStateWithGeometry, StepGeometry, RegionInfo, TrainingHandle,
    shape_has_nan, region_key_to_exclusive,
};
use super::session::TrainingSession;
use super::trace_store::TraceStore;
use super::training::{run_single_training, run_parallel_training};

/// Build geometry response (region infos) from a step.
fn build_region_infos(step: &Step) -> Vec<RegionInfo> {
    let num_shapes = step.shapes.len();
    let mut region_infos = Vec::new();
    for component in &step.components {
        for region in &component.regions {
            let exclusive_key = region_key_to_exclusive(&region.key, num_shapes);
            let error_info = step.errors.get(&exclusive_key);
            region_infos.push(RegionInfo {
                key: region.key.clone(),
                area: region.area,
                target_area: error_info.map(|e| e.target_area),
                error: error_info.map(|e| e.error.v()).unwrap_or(0.0),
            });
        }
    }
    region_infos
}

/// Build a StepStateWithGeometry response from a step.
fn build_step_response(step: &Step, step_index: usize) -> StepStateWithGeometry {
    let shapes: Vec<Value> = step.shapes.iter()
        .map(|s| serde_json::to_value(s.v()).unwrap())
        .collect();
    let region_infos = build_region_infos(step);
    StepStateWithGeometry {
        step_index,
        error: step.error.v(),
        shapes,
        is_keyframe: true,
        geometry: StepGeometry {
            components: step.components.clone(),
            regions: region_infos,
        },
    }
}

/// Shared logic for installing a trace as the current training session.
///
/// Given trace data, this extracts what it needs and delegates to
/// `install_trace_session_from_parts`.
fn install_trace_session(
    id: &str,
    trace_data: &TraceData,
    load_step: &str,
    current_session: &Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>>,
) -> Result<(Step, usize, usize), JsonRpcResponse> {
    install_trace_session_from_parts(
        id,
        load_step,
        &trace_data.inputs(),
        &trace_data.targets(),
        trace_data.learning_rate(),
        trace_data.total_steps(),
        trace_data.min_step(),
        &trace_data.keyframes(),
        current_session,
    )
}

/// Core logic for installing a trace session from pre-extracted parts.
///
/// 1. Resolves the target step from a "best"/"last"/"first"/numeric specifier
/// 2. Finds the nearest keyframe and recomputes forward to the target step
/// 3. Creates a new TrainingSession and populates it with keyframes
/// 4. Installs the session as the current one
///
/// Returns `(current_step, target_step, keyframe_count)` on success.
fn install_trace_session_from_parts(
    id: &str,
    load_step: &str,
    inputs: &[InputSpec],
    targets: &TargetsMap<f64>,
    learning_rate: f64,
    total_steps: usize,
    min_step: usize,
    keyframes: &[crate::trace::Keyframe],
    current_session: &Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>>,
) -> Result<(Step, usize, usize), JsonRpcResponse> {
    // Determine which step to load
    let target_step = match load_step {
        "best" => min_step,
        "last" => total_steps.saturating_sub(1),
        "first" | "0" => 0,
        s => match s.parse::<usize>() {
            Ok(n) => n.min(total_steps.saturating_sub(1)),
            Err(_) => min_step,
        },
    };

    // Find keyframe at or before target_step
    let keyframe = keyframes
        .iter()
        .filter(|k| k.step_index <= target_step)
        .max_by_key(|k| k.step_index);

    let (shapes_json, kf_step) = match keyframe {
        Some(kf) => (kf.shapes.clone(), kf.step_index),
        None => {
            if let Some(first) = keyframes.first() {
                (first.shapes.clone(), first.step_index)
            } else {
                return Err(JsonRpcResponse {
                    id: id.to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: "No keyframes found in trace".to_string(),
                    }),
                });
            }
        }
    };

    // Parse shapes from keyframe
    let shapes: Vec<Shape<f64>> = match shapes_json
        .iter()
        .map(|v| serde_json::from_value(v.clone()))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(s) => s,
        Err(e) => return Err(JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to parse shapes from keyframe: {}", e),
            }),
        }),
    };

    let inputs_with_loaded_shapes: Vec<InputSpec> = inputs
        .iter()
        .zip(shapes.iter())
        .map(|((_, trainable), shape)| (shape.clone(), trainable.clone()))
        .collect();

    // Create initial step from loaded shapes
    let step = match Step::new(inputs_with_loaded_shapes, targets.clone().into()) {
        Ok(s) => s,
        Err(e) => return Err(JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to create step from loaded shapes: {}", e),
            }),
        }),
    };

    // Recompute forward from keyframe to target step if needed
    let mut current_step = step;
    for _ in kf_step..target_step {
        match current_step.step(learning_rate) {
            Ok(next) => current_step = next,
            Err(e) => return Err(JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: format!("Failed to recompute to step {}: {}", target_step, e),
                }),
            }),
        }
    }

    // Create new training session
    let session = Arc::new(TrainingSession::new(inputs.to_vec(), targets.clone(), learning_rate, StorageStrategy::Tiered));

    // Record the loaded step
    session.record_step(target_step, current_step.clone(), current_step.error.v());

    // Also populate storage with keyframes from the trace for time-travel
    for kf in keyframes {
        if kf.step_index != target_step {
            if let Ok(kf_shapes) = kf.shapes
                .iter()
                .map(|v| serde_json::from_value::<Shape<f64>>(v.clone()))
                .collect::<Result<Vec<_>, _>>()
            {
                let kf_inputs: Vec<InputSpec> = inputs
                    .iter()
                    .zip(kf_shapes.iter())
                    .map(|((_, trainable), shape)| (shape.clone(), trainable.clone()))
                    .collect();

                if let Ok(kf_step_obj) = Step::new(kf_inputs, targets.clone().into()) {
                    session.record_step(kf.step_index, kf_step_obj.clone(), kf.error.unwrap_or(kf_step_obj.error.v()));
                }
            }
        }
    }

    // Install the session
    {
        let mut current = current_session.lock().unwrap();
        if let Some(old_session) = current.take() {
            old_session.request_stop();
        }
        *current = Some(session);
    }

    let keyframe_count = keyframes.len();
    Ok((current_step, target_step, keyframe_count))
}

/// Handle createModel RPC - creates initial step with full geometry
pub(super) fn handle_create_model(id: &str, params: &Value) -> JsonRpcResponse {
    // Parse inputs and targets from params
    let inputs: Vec<InputSpec> = match params.get("inputs") {
        Some(v) => match serde_json::from_value(v.clone()) {
            Ok(i) => i,
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid inputs: {}", e),
                }),
            },
        },
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'inputs' parameter".to_string(),
            }),
        },
    };

    let targets: TargetsMap<f64> = match params.get("targets") {
        Some(v) => match serde_json::from_value(v.clone()) {
            Ok(t) => t,
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid targets: {}", e),
                }),
            },
        },
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'targets' parameter".to_string(),
            }),
        },
    };

    // Debug: log received inputs and targets
    eprintln!("createModel: received {} inputs, targets: {:?}", inputs.len(), targets);

    // Create the step
    let step = match Step::new(inputs, targets.clone().into()) {
        Ok(s) => s,
        Err(e) => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to create model: {}", e),
            }),
        },
    };

    let error_val = step.error.v();
    eprintln!("createModel: computed error = {}, is_nan = {}", error_val, error_val.is_nan());

    let result = build_step_response(&step, 0);

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(serde_json::to_value(result).unwrap()),
        error: None,
    }
}

/// Handle trainBatch RPC - stateless batch training for on-demand step computation
pub(super) fn handle_train_batch(id: &str, params: &Value) -> JsonRpcResponse {
    // Parse inputs
    let inputs: Vec<InputSpec> = match params.get("inputs") {
        Some(v) => match serde_json::from_value(v.clone()) {
            Ok(i) => i,
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid inputs: {}", e),
                }),
            },
        },
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'inputs' parameter".to_string(),
            }),
        },
    };

    // Parse targets
    let targets: TargetsMap<f64> = match params.get("targets") {
        Some(v) => match serde_json::from_value(v.clone()) {
            Ok(t) => t,
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid targets: {}", e),
                }),
            },
        },
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'targets' parameter".to_string(),
            }),
        },
    };

    // Parse numSteps (required)
    let num_steps = match params.get("numSteps").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'numSteps' parameter".to_string(),
            }),
        },
    };

    // Parse optional learningRate
    let learning_rate = params.get("learningRate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.05);

    eprintln!("[trainBatch] Starting with {} steps, lr={}", num_steps, learning_rate);

    // Create initial step
    let mut current_step = match Step::new(inputs, targets.into()) {
        Ok(s) => s,
        Err(e) => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to create initial step: {}", e),
            }),
        },
    };

    eprintln!("[trainBatch] Initial step created, error={}", current_step.error.v());

    // Collect all steps
    let mut steps: Vec<Value> = Vec::with_capacity(num_steps);
    let mut min_error = current_step.error.v();
    let mut min_step_index = 0usize;

    // Collect sparkline data: errors and gradients for each step
    let mut sparkline_errors: Vec<f64> = Vec::with_capacity(num_steps);
    let mut sparkline_gradients: Vec<Vec<f64>> = Vec::with_capacity(num_steps);
    // Region errors: key -> [error at step 0, error at step 1, ...]
    let mut sparkline_region_errors: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();

    // Initialize region error vectors from initial step
    for (key, _err) in &current_step.errors {
        sparkline_region_errors.insert(key.clone(), Vec::with_capacity(num_steps));
    }

    // Record initial step (step 0)
    let initial_shapes: Vec<Value> = current_step.shapes.iter()
        .map(|s| serde_json::to_value(s.v()).unwrap())
        .collect();
    steps.push(serde_json::json!({
        "stepIndex": 0,
        "error": current_step.error.v(),
        "shapes": initial_shapes,
    }));
    sparkline_errors.push(current_step.error.v());
    sparkline_gradients.push(current_step.error.d().to_vec());
    for (key, err) in &current_step.errors {
        if let Some(vec) = sparkline_region_errors.get_mut(key) {
            vec.push(err.error.v());
        }
    }

    // Compute remaining steps
    for i in 1..num_steps {
        match current_step.step(learning_rate) {
            Ok(next_step) => {
                let error = next_step.error.v();

                // Check for NaN in error or shape coordinates
                if error.is_nan() {
                    eprintln!("[trainBatch] NaN error detected at step {}", i);
                    return JsonRpcResponse {
                        id: id.to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: format!("NaN error at step {}", i),
                        }),
                    };
                }
                if next_step.shapes.iter().any(|s| shape_has_nan(&s.v())) {
                    eprintln!("[trainBatch] NaN in shapes detected at step {}", i);
                    return JsonRpcResponse {
                        id: id.to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: format!("NaN in shape coordinates at step {}", i),
                        }),
                    };
                }

                // Track minimum
                if error < min_error {
                    min_error = error;
                    min_step_index = i;
                }

                // Record step
                let shapes: Vec<Value> = next_step.shapes.iter()
                    .map(|s| serde_json::to_value(s.v()).unwrap())
                    .collect();
                steps.push(serde_json::json!({
                    "stepIndex": i,
                    "error": error,
                    "shapes": shapes,
                }));

                // Collect sparkline data
                sparkline_errors.push(error);
                sparkline_gradients.push(next_step.error.d().to_vec());
                for (key, err) in &next_step.errors {
                    if let Some(vec) = sparkline_region_errors.get_mut(key) {
                        vec.push(err.error.v());
                    }
                }

                current_step = next_step;
            }
            Err(e) => {
                return JsonRpcResponse {
                    id: id.to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: format!("Step {} failed: {}", i, e),
                    }),
                };
            }
        }
    }

    // Build result
    let final_shapes: Vec<Value> = current_step.shapes.iter()
        .map(|s| serde_json::to_value(s.v()).unwrap())
        .collect();

    eprintln!("[trainBatch] Completed {} steps, minError={} at step {}", steps.len(), min_error, min_step_index);

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(serde_json::json!({
            "steps": steps,
            "minError": min_error,
            "minStepIndex": min_step_index,
            "finalShapes": final_shapes,
            "sparklineData": {
                "errors": sparkline_errors,
                "gradients": sparkline_gradients,
                "regionErrors": sparkline_region_errors,
            },
        })),
        error: None,
    }
}

/// Handle train RPC - starts training and streams progress updates
pub(super) async fn handle_train(
    id: &str,
    params: &Value,
    config: &ServerConfig,
    tx: mpsc::Sender<String>,
    current_session: Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>>,
) -> JsonRpcResponse {
    // Parse parameters
    let inputs: Vec<InputSpec> = match params.get("inputs") {
        Some(v) => match serde_json::from_value(v.clone()) {
            Ok(i) => i,
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid inputs: {}", e),
                }),
            },
        },
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'inputs' parameter".to_string(),
            }),
        },
    };

    let targets: TargetsMap<f64> = match params.get("targets") {
        Some(v) => match serde_json::from_value(v.clone()) {
            Ok(t) => t,
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid targets: {}", e),
                }),
            },
        },
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'targets' parameter".to_string(),
            }),
        },
    };

    let max_steps = params.get("maxSteps")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000) as usize;
    let learning_rate = params.get("learningRate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.05);
    let robust = params.get("robust")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let num_parallel = params.get("parallel")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(config.parallel);
    let storage_strategy = params.get("storageStrategy")
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "dense" => StorageStrategy::Dense,
            "btd" => StorageStrategy::Btd,
            _ => StorageStrategy::Tiered,
        })
        .unwrap_or(StorageStrategy::Tiered);

    // Generate unique handle ID
    let handle_id = format!("train_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());
    let started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Create new session
    let session = Arc::new(TrainingSession::new(inputs.clone(), targets.clone(), learning_rate, storage_strategy));
    {
        let mut current = current_session.lock().unwrap();
        // Stop any existing training
        if let Some(old_session) = current.take() {
            old_session.request_stop();
        }
        *current = Some(session.clone());
    }

    let handle_id_clone = handle_id.clone();

    // Run training in background
    tokio::task::spawn_blocking(move || {
        if num_parallel > 1 {
            run_parallel_training(
                inputs, targets, max_steps, learning_rate, robust,
                num_parallel, session, tx, handle_id_clone,
            );
        } else {
            run_single_training(
                inputs, targets, max_steps, learning_rate, robust,
                session, tx, handle_id_clone,
            );
        }
    });

    // Return handle immediately
    JsonRpcResponse {
        id: id.to_string(),
        result: Some(serde_json::to_value(TrainingHandle {
            id: handle_id,
            started_at,
        }).unwrap()),
        error: None,
    }
}

/// Handle loadTrace RPC - loads a trace file and reconstructs model state.
pub(super) fn handle_load_trace(
    id: &str,
    params: &Value,
    current_session: &Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>>,
    trace_store: &Arc<std::sync::Mutex<TraceStore>>,
) -> JsonRpcResponse {
    // Get optional name for the trace (from uploaded filename)
    let trace_name = params.get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("trace-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()));

    // Parse trace JSON from params
    let trace_json = match params.get("trace") {
        Some(v) => v.clone(),
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'trace' parameter".to_string(),
            }),
        },
    };

    let load_step = params.get("step")
        .and_then(|v| v.as_str())
        .unwrap_or("best");

    // Detect and parse trace format
    let trace_data = if trace_json.get("version").is_some() && trace_json.get("config").is_some() {
        match serde_json::from_value::<TraceFileV2>(trace_json) {
            Ok(t) => TraceData::V2(t),
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid V2 trace format: {}", e),
                }),
            },
        }
    } else if trace_json.get("inputs").is_some() && trace_json.get("traces").is_some() {
        match serde_json::from_value::<TrainResult>(trace_json) {
            Ok(t) => TraceData::Train(t),
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid train output format: {}", e),
                }),
            },
        }
    } else {
        return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Unknown trace format: expected V2 (version+config) or train output (inputs+traces)".to_string(),
            }),
        };
    };

    let format_name = trace_data.format_name().to_string();
    let keyframe_count = trace_data.keyframes().len();

    // Use shared helper to install session
    let (current_step, target_step, _) = match install_trace_session(id, &trace_data, load_step, current_session) {
        Ok(result) => result,
        Err(response) => return response,
    };

    // Auto-save the loaded trace
    let saved_meta = {
        let mut store = trace_store.lock().unwrap();
        store.save(trace_data, trace_name)
    };

    let result = serde_json::json!({
        "loaded": true,
        "traceId": saved_meta.trace_id,
        "format": format_name,
        "loadedStep": target_step,
        "totalSteps": saved_meta.total_steps,
        "minStep": saved_meta.min_step,
        "minError": saved_meta.min_error,
        "keyframeCount": keyframe_count,
        "step": build_step_response(&current_step, target_step),
    });

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(result),
        error: None,
    }
}

/// Handle saveTrace RPC - save current session's trace.
pub(super) fn handle_save_trace(
    id: &str,
    params: &Value,
    current_session: &Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>>,
    trace_store: &Arc<std::sync::Mutex<TraceStore>>,
) -> JsonRpcResponse {
    let name = params.get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("trace-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()));

    // Get current session
    let current = current_session.lock().unwrap();
    let session = match current.as_ref() {
        Some(s) => s,
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: "No active training session to save".to_string(),
            }),
        },
    };

    // Get session metadata
    let metadata = session.get_metadata();
    let min_step = metadata.min_index;
    let min_error = metadata.min_error;
    let total_steps = metadata.total_steps;

    // Get the best step (min error) from storage
    let best_step = match session.get_step(min_step) {
        Ok(s) => s,
        Err(e) => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to get best step: {}", e),
            }),
        },
    };

    // Build keyframe from best step
    let shapes_json: Vec<Value> = best_step.shapes.iter()
        .map(|s| serde_json::to_value(s.v()).unwrap())
        .collect();

    let keyframe = crate::trace::Keyframe {
        step_index: min_step,
        shapes: shapes_json,
        error: Some(min_error),
    };

    // Build V2 trace
    let trace_v2 = crate::trace::TraceFileV2 {
        version: 2,
        created: Some(format!("{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis())),
        config: crate::trace::TraceConfig {
            inputs: session.inputs.clone(),
            targets: session.targets.clone(),
            learning_rate: session.learning_rate,
            convergence_threshold: 1e-10,
        },
        btd_keyframes: vec![keyframe.clone()],
        interval_keyframes: vec![],
        total_steps,
        min_error,
        min_step,
        tiering: None,
        errors: None,
    };

    // Convert to TraceData and save
    let trace_data = TraceData::V2(trace_v2);
    let saved_meta = {
        let mut store = trace_store.lock().unwrap();
        store.save(trace_data, name)
    };

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(serde_json::json!({
            "traceId": saved_meta.trace_id,
            "name": saved_meta.name,
            "savedAt": saved_meta.saved_at,
        })),
        error: None,
    }
}

/// Handle listTraces RPC - list all saved traces.
pub(super) fn handle_list_traces(
    id: &str,
    trace_store: &Arc<std::sync::Mutex<TraceStore>>,
) -> JsonRpcResponse {
    let store = trace_store.lock().unwrap();
    let traces = store.list();

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(serde_json::json!({
            "traces": traces
        })),
        error: None,
    }
}

/// Handle renameTrace RPC - rename a saved trace.
pub(super) fn handle_rename_trace(
    id: &str,
    params: &Value,
    trace_store: &Arc<std::sync::Mutex<TraceStore>>,
) -> JsonRpcResponse {
    let trace_id = match params.get("traceId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'traceId' parameter".to_string(),
            }),
        },
    };

    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'name' parameter".to_string(),
            }),
        },
    };

    let mut store = trace_store.lock().unwrap();
    match store.rename(trace_id, name) {
        Some(meta) => JsonRpcResponse {
            id: id.to_string(),
            result: Some(serde_json::to_value(meta).unwrap()),
            error: None,
        },
        None => JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Trace not found: {}", trace_id),
            }),
        },
    }
}

/// Handle deleteTrace RPC - delete a saved trace.
pub(super) fn handle_delete_trace(
    id: &str,
    params: &Value,
    trace_store: &Arc<std::sync::Mutex<TraceStore>>,
) -> JsonRpcResponse {
    let trace_id = match params.get("traceId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'traceId' parameter".to_string(),
            }),
        },
    };

    let mut store = trace_store.lock().unwrap();
    if store.delete(trace_id) {
        JsonRpcResponse {
            id: id.to_string(),
            result: Some(serde_json::json!({ "deleted": true })),
            error: None,
        }
    } else {
        JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Trace not found: {}", trace_id),
            }),
        }
    }
}

/// Handle loadSavedTrace RPC - load a previously saved trace by ID.
pub(super) fn handle_load_saved_trace(
    id: &str,
    params: &Value,
    current_session: &Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>>,
    trace_store: &Arc<std::sync::Mutex<TraceStore>>,
) -> JsonRpcResponse {
    let trace_id = match params.get("traceId").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'traceId' parameter".to_string(),
            }),
        },
    };

    let load_step = params.get("step")
        .and_then(|v| v.as_str())
        .unwrap_or("best");

    // Get trace from store — extract all needed data while lock is held
    let (trace_meta, keyframe_count, inputs, targets, learning_rate, total_steps, min_step, keyframes) = {
        let store = trace_store.lock().unwrap();
        let saved_trace = match store.get(trace_id) {
            Some(t) => t,
            None => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: format!("Trace not found: {}", trace_id),
                }),
            },
        };
        let trace_data = &saved_trace.data;
        (
            saved_trace.meta.clone(),
            trace_data.keyframes().len(),
            trace_data.inputs().clone(),
            trace_data.targets().clone(),
            trace_data.learning_rate(),
            trace_data.total_steps(),
            trace_data.min_step(),
            trace_data.keyframes(),
        )
    };

    // Use shared helper to install session
    let (current_step, target_step, _) = match install_trace_session_from_parts(
        id, load_step, &inputs, &targets, learning_rate, total_steps, min_step, &keyframes, current_session,
    ) {
        Ok(result) => result,
        Err(response) => return response,
    };

    let result = serde_json::json!({
        "loaded": true,
        "traceId": trace_meta.trace_id,
        "name": trace_meta.name,
        "loadedStep": target_step,
        "totalSteps": trace_meta.total_steps,
        "minStep": trace_meta.min_step,
        "minError": trace_meta.min_error,
        "keyframeCount": keyframe_count,
        "step": build_step_response(&current_step, target_step),
    });

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(result),
        error: None,
    }
}

/// Handle listSampleTraces RPC - list available sample traces.
pub(super) fn handle_list_sample_traces(
    id: &str,
    config: &ServerConfig,
) -> JsonRpcResponse {
    let samples_dir = match &config.samples_dir {
        Some(dir) => dir,
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: Some(serde_json::json!({ "samples": [] })),
            error: None,
        },
    };

    let mut samples: Vec<serde_json::Value> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(samples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if filename.ends_with(".trace.json") {
                let name = filename.strip_suffix(".trace.json")
                    .unwrap_or(filename)
                    .to_string();

                let size_bytes = entry.metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);

                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(trace) = serde_json::from_str::<serde_json::Value>(&contents) {
                        let total_steps = trace.get("totalSteps")
                            .and_then(|v| v.as_u64())
                            .or_else(|| trace.get("best").and_then(|b| b.get("total_steps")).and_then(|v| v.as_u64()))
                            .unwrap_or(0) as usize;
                        let min_error = trace.get("minError")
                            .and_then(|v| v.as_f64())
                            .or_else(|| trace.get("best").and_then(|b| b.get("min_error")).and_then(|v| v.as_f64()))
                            .unwrap_or(0.0);
                        let min_step = trace.get("minStep")
                            .and_then(|v| v.as_u64())
                            .or_else(|| trace.get("best").and_then(|b| b.get("min_step")).and_then(|v| v.as_u64()))
                            .unwrap_or(0) as usize;

                        let num_shapes = trace.get("config")
                            .and_then(|c| c.get("inputs"))
                            .and_then(|i| i.as_array())
                            .map(|a| a.len())
                            .or_else(|| trace.get("inputs").and_then(|i| i.as_array()).map(|a| a.len()))
                            .unwrap_or(0);

                        samples.push(serde_json::json!({
                            "filename": filename,
                            "name": name,
                            "totalSteps": total_steps,
                            "minError": min_error,
                            "minStep": min_step,
                            "numShapes": num_shapes,
                            "sizeBytes": size_bytes,
                        }));
                    }
                }
            }
        }
    }

    // Sort by name
    samples.sort_by(|a, b| {
        let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        name_a.cmp(name_b)
    });

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(serde_json::json!({ "samples": samples })),
        error: None,
    }
}

/// Handle loadSampleTrace RPC - load a sample trace by filename.
pub(super) fn handle_load_sample_trace(
    id: &str,
    params: &Value,
    config: &ServerConfig,
    current_session: &Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>>,
    trace_store: &Arc<std::sync::Mutex<TraceStore>>,
) -> JsonRpcResponse {
    let samples_dir = match &config.samples_dir {
        Some(dir) => dir,
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: "No samples directory configured".to_string(),
            }),
        },
    };

    let filename = match params.get("filename").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Missing 'filename' parameter".to_string(),
            }),
        },
    };

    // Sanitize filename to prevent directory traversal
    let safe_filename = std::path::Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if safe_filename.is_empty() || !safe_filename.ends_with(".trace.json") {
        return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid filename".to_string(),
            }),
        };
    }

    let file_path = samples_dir.join(safe_filename);
    let contents = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to read sample trace: {}", e),
            }),
        },
    };

    let trace_json: Value = match serde_json::from_str(&contents) {
        Ok(v) => v,
        Err(e) => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to parse sample trace: {}", e),
            }),
        },
    };

    let trace_name = safe_filename
        .strip_suffix(".trace.json")
        .unwrap_or(safe_filename)
        .to_string();

    let load_step = params.get("step")
        .and_then(|v| v.as_str())
        .unwrap_or("best");

    // Build params for loadTrace
    let load_params = serde_json::json!({
        "trace": trace_json,
        "name": trace_name,
        "step": load_step,
    });

    // Delegate to loadTrace handler
    handle_load_trace(id, &load_params, current_session, trace_store)
}
