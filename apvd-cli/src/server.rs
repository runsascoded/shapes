//! WebSocket server for real-time training updates.
//!
//! The server accepts WebSocket connections and handles training requests,
//! streaming step updates back to the client.
//!
//! Supports two protocols:
//! 1. Legacy tag-based: `{"type": "StartTraining", ...}`
//! 2. JSON-RPC style: `{"id": "req_1", "method": "createModel", "params": {...}}`

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

use apvd_core::{InputSpec, Model, Step, TargetsMap, regions};

/// Server configuration
#[derive(Clone)]
pub struct ServerConfig {
    pub parallel: usize,
}

/// Messages from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Start training with given shapes and targets
    StartTraining {
        shapes: Vec<InputSpec>,
        targets: TargetsMap<f64>,
        #[serde(default = "default_max_steps")]
        max_steps: usize,
        #[serde(default = "default_learning_rate")]
        learning_rate: f64,
        #[serde(default)]
        robust: bool,
        /// Number of parallel variants to train (overrides server config if provided)
        #[serde(default)]
        parallel: Option<usize>,
    },
    /// Stop current training
    StopTraining,
    /// Ping to keep connection alive
    Ping,
}

fn default_max_steps() -> usize {
    1000
}

fn default_learning_rate() -> f64 {
    0.05
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
#[allow(dead_code)]
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

/// Messages from server to client
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Training step update (from best variant when running parallel)
    StepUpdate {
        step_idx: usize,
        error: f64,
        min_error: f64,
        min_step: usize,
        shapes: Vec<serde_json::Value>,
        converged: bool,
        /// Which variant this update is from (0 if single training)
        #[serde(skip_serializing_if = "Option::is_none")]
        variant_id: Option<usize>,
    },
    /// Training completed
    TrainingComplete {
        total_steps: usize,
        final_error: f64,
        min_error: f64,
        min_step: usize,
        /// Permutation used for best result (if parallel training)
        #[serde(skip_serializing_if = "Option::is_none")]
        best_permutation: Option<Vec<usize>>,
    },
    /// Error occurred
    Error { message: String },
    /// Training was stopped
    Stopped,
    /// Pong response
    Pong,
}

/// Shared state for a training session
struct TrainingSession {
    /// Flag to signal training should stop
    stop_requested: AtomicBool,
    /// Current best error across all variants
    best_error: std::sync::Mutex<f64>,
    /// Which variant currently has best error
    best_variant: AtomicUsize,
}

impl TrainingSession {
    fn new() -> Self {
        Self {
            stop_requested: AtomicBool::new(false),
            best_error: std::sync::Mutex::new(f64::INFINITY),
            best_variant: AtomicUsize::new(0),
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

/// Outbound message - can be legacy ServerMessage or raw JSON for JSON-RPC
#[derive(Debug)]
enum OutboundMessage {
    Legacy(ServerMessage),
    Json(String),
}

async fn handle_socket(socket: WebSocket, config: Arc<ServerConfig>) {
    let (mut sender, mut receiver) = socket.split();

    // Channel for sending messages back to the client
    let (tx, mut rx) = mpsc::channel::<OutboundMessage>(100);

    // Current training session (if any)
    let current_session: Arc<std::sync::Mutex<Option<Arc<TrainingSession>>>> =
        Arc::new(std::sync::Mutex::new(None));

    // Task to forward messages to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = match msg {
                OutboundMessage::Legacy(m) => serde_json::to_string(&m).unwrap(),
                OutboundMessage::Json(s) => s,
            };
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
                // Try JSON-RPC format first (has "id" and "method" fields)
                if let Ok(rpc_req) = serde_json::from_str::<JsonRpcRequest>(&text) {
                    let response = handle_json_rpc(&rpc_req, &config).await;
                    let json = serde_json::to_string(&response).unwrap();
                    let _ = tx.send(OutboundMessage::Json(json)).await;
                    continue;
                }

                // Fall back to legacy tag-based protocol
                let client_msg: Result<ClientMessage, _> = serde_json::from_str(&text);
                match client_msg {
                    Ok(ClientMessage::StartTraining {
                        shapes,
                        targets,
                        max_steps,
                        learning_rate,
                        robust,
                        parallel,
                    }) => {
                        // Create new session
                        let session = Arc::new(TrainingSession::new());
                        {
                            let mut current = current_session.lock().unwrap();
                            // Stop any existing training
                            if let Some(old_session) = current.take() {
                                old_session.request_stop();
                            }
                            *current = Some(session.clone());
                        }

                        let tx = tx.clone();
                        let num_parallel = parallel.unwrap_or(config.parallel);

                        // Run training in a blocking task
                        tokio::task::spawn_blocking(move || {
                            if num_parallel > 1 {
                                run_parallel_training(
                                    shapes,
                                    targets,
                                    max_steps,
                                    learning_rate,
                                    robust,
                                    num_parallel,
                                    session,
                                    tx,
                                );
                            } else {
                                run_single_training(
                                    shapes,
                                    targets,
                                    max_steps,
                                    learning_rate,
                                    robust,
                                    session,
                                    tx,
                                );
                            }
                        });
                    }
                    Ok(ClientMessage::StopTraining) => {
                        let has_session = {
                            let mut current = current_session.lock().unwrap();
                            if let Some(session) = current.take() {
                                session.request_stop();
                                true
                            } else {
                                false
                            }
                        };
                        if has_session {
                            let _ = tx.send(OutboundMessage::Legacy(ServerMessage::Stopped)).await;
                        } else {
                            let _ = tx.send(OutboundMessage::Legacy(ServerMessage::Error {
                                message: "No training in progress".to_string(),
                            })).await;
                        }
                    }
                    Ok(ClientMessage::Ping) => {
                        let _ = tx.send(OutboundMessage::Legacy(ServerMessage::Pong)).await;
                    }
                    Err(e) => {
                        let _ = tx.send(OutboundMessage::Legacy(ServerMessage::Error {
                            message: format!("Invalid message: {}", e),
                        })).await;
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

async fn handle_json_rpc(req: &JsonRpcRequest, _config: &ServerConfig) -> JsonRpcResponse {
    match req.method.as_str() {
        "createModel" => handle_create_model(&req.id, &req.params),
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

    let result = StepStateWithGeometry {
        step_index: 0,
        error: step.error.v(),
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

fn run_single_training(
    shapes: Vec<InputSpec>,
    targets: TargetsMap<f64>,
    max_steps: usize,
    learning_rate: f64,
    robust: bool,
    session: Arc<TrainingSession>,
    tx: mpsc::Sender<OutboundMessage>,
) {
    // Create model
    let model_result = Model::new(shapes, targets.clone());
    let mut model = match model_result {
        Ok(m) => m,
        Err(e) => {
            let _ = tx.blocking_send(OutboundMessage::Legacy(ServerMessage::Error {
                message: format!("Failed to create model: {}", e),
            }));
            return;
        }
    };

    // Send initial step
    send_step_update(&tx, &model, 0, None);

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
                    let _ = tx.blocking_send(OutboundMessage::Legacy(ServerMessage::Error {
                        message: "NaN error encountered".to_string(),
                    }));
                    break;
                }

                // Update min tracking
                if err < model.min_error {
                    model.min_idx = model.steps.len();
                    model.min_error = err;
                }

                model.steps.push(step);

                // Send update every 10 steps or on significant changes
                if step_idx % 10 == 0 || step_idx < 20 {
                    send_step_update(&tx, &model, step_idx + 1, None);
                }
            }
            Err(e) => {
                let _ = tx.blocking_send(OutboundMessage::Legacy(ServerMessage::Error {
                    message: format!("Step failed: {}", e),
                }));
                break;
            }
        }
    }

    if session.should_stop() {
        return;
    }

    // Send final update
    let final_step = model.steps.last().unwrap();
    let _ = tx.blocking_send(OutboundMessage::Legacy(ServerMessage::TrainingComplete {
        total_steps: model.steps.len(),
        final_error: final_step.error.v(),
        min_error: model.min_error,
        min_step: model.min_idx,
        best_permutation: None,
    }));
}

fn run_parallel_training(
    shapes: Vec<InputSpec>,
    targets: TargetsMap<f64>,
    max_steps: usize,
    learning_rate: f64,
    robust: bool,
    num_parallel: usize,
    session: Arc<TrainingSession>,
    tx: mpsc::Sender<OutboundMessage>,
) {
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

                    if err < model.min_error {
                        model.min_idx = model.steps.len();
                        model.min_error = err;
                    }

                    model.steps.push(step);

                    // Check if this is now the best variant
                    if session.update_best(variant_id, err) {
                        // Send update (only from best variant)
                        if step_idx % 10 == 0 || step_idx < 20 {
                            send_step_update(&tx, &model, step_idx + 1, Some(variant_id));
                        }
                    }
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
    if let Some((permutation, model)) = best.as_ref() {
        let final_step = model.steps.last().unwrap();
        let _ = tx.blocking_send(OutboundMessage::Legacy(ServerMessage::TrainingComplete {
            total_steps: model.steps.len(),
            final_error: final_step.error.v(),
            min_error: model.min_error,
            min_step: model.min_idx,
            best_permutation: Some(permutation.clone()),
        }));
    } else {
        let _ = tx.blocking_send(OutboundMessage::Legacy(ServerMessage::Error {
            message: "All training variants failed".to_string(),
        }));
    }
}

fn send_step_update(
    tx: &mpsc::Sender<OutboundMessage>,
    model: &Model,
    step_idx: usize,
    variant_id: Option<usize>,
) {
    let step = model.steps.last().unwrap();
    let shapes: Vec<serde_json::Value> = step
        .shapes
        .iter()
        .map(|s| serde_json::to_value(s).unwrap())
        .collect();

    let _ = tx.blocking_send(OutboundMessage::Legacy(ServerMessage::StepUpdate {
        step_idx,
        error: step.error.v(),
        min_error: model.min_error,
        min_step: model.min_idx,
        shapes,
        converged: step.converged,
        variant_id,
    }));
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
