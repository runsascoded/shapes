//! WebSocket server for real-time training updates.
//!
//! The server accepts WebSocket connections and handles training requests,
//! streaming step updates back to the client.
//!
//! Uses JSON-RPC protocol: `{"id": "req_1", "method": "createModel", "params": {...}}`

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use apvd_core::{
    InputSpec, Model, Step, TargetsMap, regions,
    TraceStorage, StorageStrategy, create_storage,
};

/// Server configuration
#[derive(Clone)]
pub struct ServerConfig {
    pub parallel: usize,
}

// ============================================================================
// JSON-RPC Protocol Types
// ============================================================================

/// JSON-RPC request message
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: String,
    method: String,
    #[serde(default)]
    params: Value,
}

/// JSON-RPC response message
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

/// JSON-RPC notification (no id, server-initiated)
#[derive(Debug, Serialize)]
struct JsonRpcNotification {
    method: String,
    params: Value,
}

/// Step state with geometry for frontend display
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StepStateWithGeometry {
    step_index: usize,
    error: f64,
    shapes: Vec<Value>,
    is_keyframe: bool,
    geometry: StepGeometry,
}

#[derive(Debug, Serialize)]
struct StepGeometry {
    components: Vec<regions::Component>,
    regions: Vec<RegionInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegionInfo {
    key: String,
    area: f64,
    target_area: Option<f64>,
    error: f64,
}

/// Progress update for streaming training progress
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressUpdate {
    handle_id: String,
    #[serde(rename = "type")]
    update_type: String, // "progress", "complete", or "error"
    current_step: usize,
    total_steps: usize,
    error: f64,
    min_error: f64,
    min_step: usize,
    shapes: Vec<serde_json::Value>,
    elapsed_ms: u64,
    converged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
}

/// Training handle returned when training starts
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrainingHandle {
    id: String,
    started_at: u64,
}

/// Shared state for a training session
struct TrainingSession {
    /// Flag to signal training should stop
    stop_requested: AtomicBool,
    /// Current best error across all variants
    best_error: std::sync::Mutex<f64>,
    /// Which variant currently has best error
    best_variant: AtomicUsize,
    /// Trace storage for steps (tiered by default)
    storage: std::sync::Mutex<Box<dyn TraceStorage>>,
    /// Learning rate used for training (needed for recomputation)
    learning_rate: f64,
}

impl TrainingSession {
    fn new(learning_rate: f64, storage_strategy: StorageStrategy) -> Self {
        Self {
            stop_requested: AtomicBool::new(false),
            best_error: std::sync::Mutex::new(f64::INFINITY),
            best_variant: AtomicUsize::new(0),
            storage: std::sync::Mutex::new(create_storage(storage_strategy, None)),
            learning_rate,
        }
    }

    fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::SeqCst);
    }

    fn should_stop(&self) -> bool {
        self.stop_requested.load(Ordering::SeqCst)
    }

    fn update_best(&self, variant_id: usize, error: f64) -> bool {
        let mut best = self.best_error.lock().unwrap();
        if error < *best {
            *best = error;
            self.best_variant.store(variant_id, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Record a step in the trace storage.
    fn record_step(&self, index: usize, step: Step, error: f64) {
        let mut storage = self.storage.lock().unwrap();
        storage.record(index, step, error);
    }

    /// Get a step from storage (may recompute from keyframe).
    fn get_step(&self, index: usize) -> Result<Step, String> {
        let storage = self.storage.lock().unwrap();
        storage.get(index, self.learning_rate)
    }

    /// Get storage metadata.
    fn get_metadata(&self) -> apvd_core::trace::TraceMetadata {
        let storage = self.storage.lock().unwrap();
        storage.metadata()
    }
}

/// Run the WebSocket server
pub async fn run_server(port: u16, config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(Arc::new(config));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    eprintln!("WebSocket server listening on ws://{}/ws", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(config): State<Arc<ServerConfig>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, config))
}

async fn handle_socket(socket: WebSocket, config: Arc<ServerConfig>) {
    let (mut sender, mut receiver) = socket.split();

    // Channel for sending messages back to the client
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // Current training session (if any)
    let current_session: Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>> =
        Arc::new(std::sync::Mutex::new(None));

    // Task to forward messages to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(json) = rx.recv().await {
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = receiver.next().await {
        let msg = match result {
            Ok(m) => m,
            Err(_) => break,
        };
        match msg {
            Message::Text(text) => {
                // Parse as JSON-RPC request
                match serde_json::from_str::<JsonRpcRequest>(&text) {
                    Ok(rpc_req) => {
                        let response = handle_json_rpc(&rpc_req, &config, &tx, &current_session).await;
                        let json = serde_json::to_string(&response).unwrap();
                        let _ = tx.send(json).await;
                    }
                    Err(e) => {
                        // Send JSON-RPC parse error
                        let error_response = JsonRpcResponse {
                            id: "".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32700,
                                message: format!("Parse error: {}", e),
                            }),
                        };
                        let json = serde_json::to_string(&error_response).unwrap();
                        let _ = tx.send(json).await;
                    }
                }
            }
            Message::Close(_) => {
                // Stop any running training when client disconnects
                let mut current = current_session.lock().unwrap();
                if let Some(session) = current.take() {
                    session.request_stop();
                }
                break;
            }
            _ => {}
        }
    }

    // Clean up
    drop(tx);
    let _ = send_task.await;
}

// ============================================================================
// JSON-RPC Handler
// ============================================================================

async fn handle_json_rpc(
    req: &JsonRpcRequest,
    config: &ServerConfig,
    tx: &mpsc::Sender<String>,
    current_session: &Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>>,
) -> JsonRpcResponse {
    match req.method.as_str() {
        "getVersion" => JsonRpcResponse {
            id: req.id.clone(),
            result: Some(serde_json::json!({
                "sha": option_env!("APVD_BUILD_SHA").unwrap_or("dev"),
                "version": env!("CARGO_PKG_VERSION"),
            })),
            error: None,
        },
        "createModel" => {
            // Catch panics in createModel to return proper error responses
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handle_create_model(&req.id, &req.params)
            })) {
                Ok(response) => response,
                Err(panic_info) => {
                    let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic".to_string()
                    };
                    eprintln!("createModel panic: {}", msg);
                    JsonRpcResponse {
                        id: req.id.clone(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: format!("Internal error: {}", msg),
                        }),
                    }
                }
            }
        },
        "train" => {
            handle_train(&req.id, &req.params, config, tx.clone(), current_session.clone()).await
        },
        "stop" => {
            let mut current = current_session.lock().unwrap();
            if let Some(session) = current.take() {
                session.request_stop();
                JsonRpcResponse {
                    id: req.id.clone(),
                    result: Some(serde_json::json!({"stopped": true})),
                    error: None,
                }
            } else {
                JsonRpcResponse {
                    id: req.id.clone(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: "No training in progress".to_string(),
                    }),
                }
            }
        },
        "getStep" => {
            let current = current_session.lock().unwrap();
            if let Some(session) = current.as_ref() {
                let step_idx = req.params.get("stepIndex")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);

                match step_idx {
                    Some(idx) => {
                        match session.get_step(idx) {
                            Ok(step) => {
                                let shapes: Vec<serde_json::Value> = step.shapes.iter()
                                    .map(|s| serde_json::to_value(s).unwrap())
                                    .collect();
                                JsonRpcResponse {
                                    id: req.id.clone(),
                                    result: Some(serde_json::json!({
                                        "stepIndex": idx,
                                        "error": step.error.v(),
                                        "shapes": shapes,
                                        "converged": step.converged,
                                    })),
                                    error: None,
                                }
                            }
                            Err(e) => JsonRpcResponse {
                                id: req.id.clone(),
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32000,
                                    message: e,
                                }),
                            },
                        }
                    }
                    None => JsonRpcResponse {
                        id: req.id.clone(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Missing 'stepIndex' parameter".to_string(),
                        }),
                    },
                }
            } else {
                JsonRpcResponse {
                    id: req.id.clone(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: "No training session".to_string(),
                    }),
                }
            }
        },
        "getMetadata" => {
            let current = current_session.lock().unwrap();
            if let Some(session) = current.as_ref() {
                let metadata = session.get_metadata();
                JsonRpcResponse {
                    id: req.id.clone(),
                    result: Some(serde_json::to_value(metadata).unwrap()),
                    error: None,
                }
            } else {
                JsonRpcResponse {
                    id: req.id.clone(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: "No training session".to_string(),
                    }),
                }
            }
        },
        "ping" => JsonRpcResponse {
            id: req.id.clone(),
            result: Some(serde_json::json!({"pong": true})),
            error: None,
        },
        _ => JsonRpcResponse {
            id: req.id.clone(),
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
            }),
        },
    }
}

/// Handle createModel RPC - creates initial step with full geometry
fn handle_create_model(id: &str, params: &Value) -> JsonRpcResponse {
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

    // Extract geometry from step
    let shapes: Vec<Value> = step.shapes.iter()
        .map(|s| serde_json::to_value(s).unwrap())
        .collect();

    // Build region info with errors
    let mut region_infos: Vec<RegionInfo> = Vec::new();
    for component in &step.components {
        for region in &component.regions {
            let error_info = step.errors.get(&region.key);
            region_infos.push(RegionInfo {
                key: region.key.clone(),
                area: region.area,
                target_area: error_info.map(|e| e.target_area),
                error: error_info.map(|e| e.error.v()).unwrap_or(0.0),
            });
        }
    }

    let error_val = step.error.v();
    eprintln!("createModel: computed error = {}, is_nan = {}", error_val, error_val.is_nan());

    let result = StepStateWithGeometry {
        step_index: 0,
        error: error_val,
        shapes,
        is_keyframe: true,
        geometry: StepGeometry {
            components: step.components.clone(),
            regions: region_infos,
        },
    };

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(serde_json::to_value(result).unwrap()),
        error: None,
    }
}

/// Handle train RPC - starts training and streams progress updates
async fn handle_train(
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
    let session = Arc::new(TrainingSession::new(learning_rate, storage_strategy));
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

fn run_single_training(
    shapes: Vec<InputSpec>,
    targets: TargetsMap<f64>,
    max_steps: usize,
    learning_rate: f64,
    robust: bool,
    session: Arc<TrainingSession>,
    tx: mpsc::Sender<String>,
    handle_id: String,
) {
    let start_time = std::time::Instant::now();

    // Create model
    let model_result = Model::new(shapes, targets.clone());
    let mut model = match model_result {
        Ok(m) => m,
        Err(e) => {
            send_progress_notification(&tx, &handle_id, "error", 0, max_steps, 0.0, 0.0, 0, vec![], start_time.elapsed().as_millis() as u64, false, Some(e.to_string()));
            return;
        }
    };

    // Record and send initial step
    let initial_step = model.steps.last().unwrap();
    session.record_step(0, initial_step.clone(), initial_step.error.v());
    send_progress_update(&tx, &handle_id, &model, 0, max_steps, start_time);

    // Training loop
    for step_idx in 0..max_steps {
        // Check for stop request
        if session.should_stop() {
            return;
        }

        let current_step = model.steps.last().unwrap();

        // Check for convergence
        if current_step.converged {
            break;
        }

        // Take a step
        let next_step = if robust {
            current_step.step_clipped(learning_rate, 0.5, 1.0)
        } else {
            current_step.step(learning_rate)
        };

        match next_step {
            Ok(step) => {
                let err = step.error.v();
                if err.is_nan() {
                    send_progress_notification(&tx, &handle_id, "error", step_idx, max_steps, err, model.min_error, model.min_idx, vec![], start_time.elapsed().as_millis() as u64, false, Some("NaN error encountered".to_string()));
                    break;
                }

                // Record step in trace storage
                let new_step_idx = model.steps.len();
                session.record_step(new_step_idx, step.clone(), err);

                // Update min tracking
                if err < model.min_error {
                    model.min_idx = new_step_idx;
                    model.min_error = err;
                }

                model.steps.push(step);

                // Send update every 10 steps or on significant changes
                if step_idx % 10 == 0 || step_idx < 20 {
                    send_progress_update(&tx, &handle_id, &model, step_idx + 1, max_steps, start_time);
                }
            }
            Err(e) => {
                send_progress_notification(&tx, &handle_id, "error", step_idx, max_steps, 0.0, model.min_error, model.min_idx, vec![], start_time.elapsed().as_millis() as u64, false, Some(format!("Step failed: {}", e)));
                break;
            }
        }
    }

    if session.should_stop() {
        return;
    }

    // Send completion
    send_progress_update_with_type(&tx, &handle_id, &model, model.steps.len() - 1, max_steps, start_time, "complete");
}

fn run_parallel_training(
    shapes: Vec<InputSpec>,
    targets: TargetsMap<f64>,
    max_steps: usize,
    learning_rate: f64,
    robust: bool,
    num_parallel: usize,
    session: Arc<TrainingSession>,
    tx: mpsc::Sender<String>,
    handle_id: String,
) {
    let start_time = std::time::Instant::now();
    let num_shapes = shapes.len();
    let permutations = generate_permutations(num_shapes, num_parallel);

    // Shared state for tracking best result
    let best_result: Arc<std::sync::Mutex<Option<(Vec<usize>, Model)>>> =
        Arc::new(std::sync::Mutex::new(None));

    // Train all variants in parallel
    permutations.into_par_iter().enumerate().for_each(|(variant_id, permutation)| {
        if session.should_stop() {
            return;
        }

        // Reorder inputs according to permutation
        let reordered_inputs: Vec<InputSpec> = permutation
            .iter()
            .map(|&idx| shapes[idx].clone())
            .collect();

        // Create model
        let model_result = Model::new(reordered_inputs, targets.clone());
        let mut model = match model_result {
            Ok(m) => m,
            Err(_) => return,
        };

        // Training loop
        for step_idx in 0..max_steps {
            if session.should_stop() {
                return;
            }

            let current_step = model.steps.last().unwrap();

            if current_step.converged {
                break;
            }

            let next_step = if robust {
                current_step.step_clipped(learning_rate, 0.5, 1.0)
            } else {
                current_step.step(learning_rate)
            };

            match next_step {
                Ok(step) => {
                    let err = step.error.v();
                    if err.is_nan() {
                        break;
                    }

                    let new_step_idx = model.steps.len();

                    if err < model.min_error {
                        model.min_idx = new_step_idx;
                        model.min_error = err;
                    }

                    // Check if this is now the best variant
                    if session.update_best(variant_id, err) {
                        // Record step from best variant
                        session.record_step(new_step_idx, step.clone(), err);
                        // Send update (only from best variant)
                        if step_idx % 10 == 0 || step_idx < 20 {
                            send_progress_update(&tx, &handle_id, &model, step_idx + 1, max_steps, start_time);
                        }
                    }

                    model.steps.push(step);
                }
                Err(_) => break,
            }
        }

        // Update best result if this variant is best
        let final_error = model.steps.last().map(|s| s.error.v()).unwrap_or(f64::INFINITY);
        let mut best = best_result.lock().unwrap();
        if best.is_none() || final_error < best.as_ref().unwrap().1.min_error {
            *best = Some((permutation, model));
        }
    });

    if session.should_stop() {
        return;
    }

    // Send final result
    let best = best_result.lock().unwrap();
    if let Some((_permutation, model)) = best.as_ref() {
        send_progress_update_with_type(&tx, &handle_id, model, model.steps.len() - 1, max_steps, start_time, "complete");
    } else {
        send_progress_notification(&tx, &handle_id, "error", 0, max_steps, 0.0, 0.0, 0, vec![], start_time.elapsed().as_millis() as u64, false, Some("All training variants failed".to_string()));
    }
}

/// Send a progress notification as JSON-RPC
fn send_progress_update(
    tx: &mpsc::Sender<String>,
    handle_id: &str,
    model: &Model,
    step_idx: usize,
    total_steps: usize,
    start_time: std::time::Instant,
) {
    send_progress_update_with_type(tx, handle_id, model, step_idx, total_steps, start_time, "progress");
}

fn send_progress_update_with_type(
    tx: &mpsc::Sender<String>,
    handle_id: &str,
    model: &Model,
    step_idx: usize,
    total_steps: usize,
    start_time: std::time::Instant,
    update_type: &str,
) {
    let step = model.steps.last().unwrap();
    let shapes: Vec<serde_json::Value> = step
        .shapes
        .iter()
        .map(|s| serde_json::to_value(s).unwrap())
        .collect();

    send_progress_notification(
        tx,
        handle_id,
        update_type,
        step_idx,
        total_steps,
        step.error.v(),
        model.min_error,
        model.min_idx,
        shapes,
        start_time.elapsed().as_millis() as u64,
        step.converged,
        None,
    );
}

fn send_progress_notification(
    tx: &mpsc::Sender<String>,
    handle_id: &str,
    update_type: &str,
    current_step: usize,
    total_steps: usize,
    error: f64,
    min_error: f64,
    min_step: usize,
    shapes: Vec<serde_json::Value>,
    elapsed_ms: u64,
    converged: bool,
    error_message: Option<String>,
) {
    let update = ProgressUpdate {
        handle_id: handle_id.to_string(),
        update_type: update_type.to_string(),
        current_step,
        total_steps,
        error,
        min_error,
        min_step,
        shapes,
        elapsed_ms,
        converged,
        error_message,
    };
    let notification = JsonRpcNotification {
        method: "progress".to_string(),
        params: serde_json::to_value(update).unwrap(),
    };
    let _ = tx.blocking_send(serde_json::to_string(&notification).unwrap());
}

/// Generate shape permutations for parallel training.
fn generate_permutations(n: usize, max_count: usize) -> Vec<Vec<usize>> {
    if max_count == 1 {
        return vec![(0..n).collect()];
    }

    let mut permutations = Vec::new();
    let mut arr: Vec<usize> = (0..n).collect();

    fn heap_permute(k: usize, arr: &mut Vec<usize>, result: &mut Vec<Vec<usize>>) {
        if k == 1 {
            result.push(arr.clone());
            return;
        }
        heap_permute(k - 1, arr, result);
        for i in 0..k - 1 {
            if k % 2 == 0 {
                arr.swap(i, k - 1);
            } else {
                arr.swap(0, k - 1);
            }
            heap_permute(k - 1, arr, result);
        }
    }

    heap_permute(n, &mut arr, &mut permutations);

    if permutations.len() > max_count {
        let step = permutations.len() / max_count;
        permutations = permutations
            .into_iter()
            .step_by(step)
            .take(max_count)
            .collect();
    }

    permutations
}
