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
    Shape,
};

use crate::trace::{TraceData, TraceFileV2, TrainResult};

/// Check if a shape has any NaN coordinates
fn shape_has_nan(shape: &Shape<f64>) -> bool {
    match shape {
        Shape::Circle(c) => c.c.x.is_nan() || c.c.y.is_nan() || c.r.is_nan(),
        Shape::XYRR(e) => e.c.x.is_nan() || e.c.y.is_nan() || e.r.x.is_nan() || e.r.y.is_nan(),
        Shape::XYRRT(e) => e.c.x.is_nan() || e.c.y.is_nan() || e.r.x.is_nan() || e.r.y.is_nan() || e.t.is_nan(),
        Shape::Polygon(p) => p.vertices.iter().any(|v| v.x.is_nan() || v.y.is_nan()),
    }
}

/// Server configuration
#[derive(Clone)]
pub struct ServerConfig {
    pub parallel: usize,
    /// Directory for persistent trace storage (None = in-memory only)
    pub storage_dir: Option<std::path::PathBuf>,
    /// Directory containing sample traces (None = no samples)
    pub samples_dir: Option<std::path::PathBuf>,
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

/// Metadata for a saved trace
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedTraceMeta {
    trace_id: String,
    name: String,
    saved_at: String,
    total_steps: usize,
    min_error: f64,
    min_step: usize,
    num_shapes: usize,
    shape_types: Vec<String>,
}

/// A saved trace with full data
struct SavedTrace {
    meta: SavedTraceMeta,
    data: TraceData,
}

/// Storage for saved traces (per-connection, optionally persistent)
struct TraceStore {
    traces: std::collections::HashMap<String, SavedTrace>,
    counter: usize,
    /// Optional directory for persistent storage
    storage_dir: Option<std::path::PathBuf>,
}

impl TraceStore {
    fn new() -> Self {
        Self {
            traces: std::collections::HashMap::new(),
            counter: 0,
            storage_dir: None,
        }
    }

    /// Create a persistent store that saves to the given directory.
    fn with_storage_dir(dir: std::path::PathBuf) -> Self {
        // Ensure directory exists
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("Warning: Failed to create trace storage directory {:?}: {}", dir, e);
        }

        let mut store = Self {
            traces: std::collections::HashMap::new(),
            counter: 0,
            storage_dir: Some(dir.clone()),
        };

        // Load existing traces from directory
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        if let Ok(saved) = serde_json::from_str::<SavedTraceFile>(&contents) {
                            let trace_id = saved.meta.trace_id.clone();
                            // Update counter to avoid ID collisions
                            if let Some(num) = trace_id.strip_prefix("trace_").and_then(|s| s.parse::<usize>().ok()) {
                                store.counter = store.counter.max(num);
                            }
                            // Convert to TraceData
                            if let Ok(trace_data) = saved.to_trace_data() {
                                store.traces.insert(trace_id, SavedTrace {
                                    meta: saved.meta,
                                    data: trace_data,
                                });
                            }
                        }
                    }
                }
            }
        }

        eprintln!("Loaded {} traces from {:?}", store.traces.len(), dir);
        store
    }

    fn generate_id(&mut self) -> String {
        self.counter += 1;
        format!("trace_{}", self.counter)
    }

    fn save(&mut self, data: TraceData, name: String) -> SavedTraceMeta {
        let trace_id = self.generate_id();
        let inputs = data.inputs();
        let shape_types: Vec<String> = inputs.iter().map(|(s, _)| {
            match s {
                Shape::Circle(_) => "Circle".to_string(),
                Shape::XYRR(_) => "XYRR".to_string(),
                Shape::XYRRT(_) => "XYRRT".to_string(),
                Shape::Polygon(p) => format!("Polygon({})", p.vertices.len()),
            }
        }).collect();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let meta = SavedTraceMeta {
            trace_id: trace_id.clone(),
            name,
            saved_at: format!("{}", now),
            total_steps: data.total_steps(),
            min_error: data.min_error(),
            min_step: data.min_step(),
            num_shapes: inputs.len(),
            shape_types,
        };

        // Persist to disk if storage_dir is set
        if let Some(ref dir) = self.storage_dir {
            let file_path = dir.join(format!("{}.json", trace_id));
            let file_data = SavedTraceFile::from_trace_data(&meta, &data);
            if let Ok(json) = serde_json::to_string_pretty(&file_data) {
                if let Err(e) = std::fs::write(&file_path, json) {
                    eprintln!("Warning: Failed to persist trace to {:?}: {}", file_path, e);
                }
            }
        }

        self.traces.insert(trace_id, SavedTrace {
            meta: meta.clone(),
            data,
        });

        meta
    }

    fn list(&self) -> Vec<SavedTraceMeta> {
        self.traces.values().map(|t| t.meta.clone()).collect()
    }

    fn get(&self, trace_id: &str) -> Option<&SavedTrace> {
        self.traces.get(trace_id)
    }

    fn rename(&mut self, trace_id: &str, name: String) -> Option<SavedTraceMeta> {
        if let Some(trace) = self.traces.get_mut(trace_id) {
            trace.meta.name = name.clone();

            // Update on disk
            if let Some(ref dir) = self.storage_dir {
                let file_path = dir.join(format!("{}.json", trace_id));
                let file_data = SavedTraceFile::from_trace_data(&trace.meta, &trace.data);
                if let Ok(json) = serde_json::to_string_pretty(&file_data) {
                    let _ = std::fs::write(&file_path, json);
                }
            }

            Some(trace.meta.clone())
        } else {
            None
        }
    }

    fn delete(&mut self, trace_id: &str) -> bool {
        let removed = self.traces.remove(trace_id).is_some();

        // Delete from disk
        if removed {
            if let Some(ref dir) = self.storage_dir {
                let file_path = dir.join(format!("{}.json", trace_id));
                let _ = std::fs::remove_file(file_path);
            }
        }

        removed
    }
}

/// File format for persisted traces (serializable version of SavedTrace)
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedTraceFile {
    meta: SavedTraceMeta,
    /// The trace data in V2 format (or converted to V2)
    trace: crate::trace::TraceFileV2,
}

impl SavedTraceFile {
    fn from_trace_data(meta: &SavedTraceMeta, data: &TraceData) -> Self {
        let trace = match data {
            TraceData::V2(v2) => v2.clone(),
            TraceData::Train(train) => {
                // Convert TrainResult to V2 format
                let keyframes = data.keyframes();
                crate::trace::TraceFileV2 {
                    version: 2,
                    created: Some(meta.saved_at.clone()),
                    config: crate::trace::TraceConfig {
                        inputs: train.inputs.clone(),
                        targets: train.targets.clone(),
                        learning_rate: 0.05, // default
                        convergence_threshold: 1e-10,
                    },
                    btd_keyframes: keyframes,
                    interval_keyframes: vec![],
                    total_steps: train.best.total_steps,
                    min_error: train.best.min_error,
                    min_step: train.best.min_step,
                    tiering: None,
                    errors: None,
                }
            }
        };
        Self { meta: meta.clone(), trace }
    }

    fn to_trace_data(&self) -> Result<TraceData, String> {
        Ok(TraceData::V2(self.trace.clone()))
    }
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
    /// Original inputs (for trace export)
    inputs: Vec<InputSpec>,
    /// Original targets (for trace export)
    targets: TargetsMap<f64>,
}

impl TrainingSession {
    fn new(inputs: Vec<InputSpec>, targets: TargetsMap<f64>, learning_rate: f64, storage_strategy: StorageStrategy) -> Self {
        Self {
            stop_requested: AtomicBool::new(false),
            best_error: std::sync::Mutex::new(f64::INFINITY),
            best_variant: AtomicUsize::new(0),
            storage: std::sync::Mutex::new(create_storage(storage_strategy, None)),
            learning_rate,
            inputs,
            targets,
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

    // Saved traces for this connection (persistent if storage_dir configured)
    let trace_store: Arc<std::sync::Mutex<TraceStore>> = Arc::new(std::sync::Mutex::new(
        match &config.storage_dir {
            Some(dir) => TraceStore::with_storage_dir(dir.clone()),
            None => TraceStore::new(),
        }
    ));

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
                        let response = handle_json_rpc(&rpc_req, &config, &tx, &current_session, &trace_store).await;
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
    trace_store: &Arc<std::sync::Mutex<TraceStore>>,
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
        "trainBatch" => {
            handle_train_batch(&req.id, &req.params)
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
        "loadTrace" => {
            handle_load_trace(&req.id, &req.params, current_session, trace_store)
        },
        "saveTrace" => {
            handle_save_trace(&req.id, &req.params, current_session, trace_store)
        },
        "listTraces" => {
            handle_list_traces(&req.id, trace_store)
        },
        "renameTrace" => {
            handle_rename_trace(&req.id, &req.params, trace_store)
        },
        "deleteTrace" => {
            handle_delete_trace(&req.id, &req.params, trace_store)
        },
        "loadSavedTrace" => {
            handle_load_saved_trace(&req.id, &req.params, current_session, trace_store)
        },
        "listSampleTraces" => {
            handle_list_sample_traces(&req.id, config)
        },
        "loadSampleTrace" => {
            handle_load_sample_trace(&req.id, &req.params, config, current_session, trace_store)
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

    // Extract geometry from step - convert Shape<Dual> to Shape<f64> for plain numbers
    let shapes: Vec<Value> = step.shapes.iter()
        .map(|s| serde_json::to_value(s.v()).unwrap())
        .collect();

    // Build region info with errors
    // Convert region key (e.g. "01") to exclusive key format (e.g. "01-") to look up in step.errors
    let num_shapes = step.shapes.len();
    let region_key_to_exclusive = |key: &str| -> String {
        let included: std::collections::HashSet<char> = key.chars().collect();
        (0..num_shapes)
            .map(|i| {
                let ch = char::from_digit(i as u32, 10).unwrap();
                if included.contains(&ch) { ch } else { '-' }
            })
            .collect()
    };

    let mut region_infos: Vec<RegionInfo> = Vec::new();
    for component in &step.components {
        for region in &component.regions {
            let exclusive_key = region_key_to_exclusive(&region.key);
            let error_info = step.errors.get(&exclusive_key);
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

/// Handle trainBatch RPC - stateless batch training for on-demand step computation
fn handle_train_batch(id: &str, params: &Value) -> JsonRpcResponse {
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
    // Convert Shape<Dual> to Shape<f64> before serializing to get plain numbers
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
    // Collect initial region errors
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

                // Record step - convert Shape<Dual> to Shape<f64> for plain numbers
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
                // Collect region errors
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

    // Build result - convert Shape<Dual> to Shape<f64> for plain numbers
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
///
/// This allows the server to load a previously saved trace and continue training
/// from where it left off (though optimizer state like Adam momentum is not preserved).
fn handle_load_trace(
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

    // Optional: which step to load (defaults to min_step for best result)
    let load_step = params.get("step")
        .and_then(|v| v.as_str())
        .unwrap_or("best");

    // Detect and parse trace format
    let trace_data = if trace_json.get("version").is_some() && trace_json.get("config").is_some() {
        // V2 format
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
        // Train output format
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

    // Capture format name before potential move
    let format_name = trace_data.format_name().to_string();

    // Determine which step to load
    let target_step = match load_step {
        "best" => trace_data.min_step(),
        "last" => trace_data.total_steps().saturating_sub(1),
        "first" | "0" => 0,
        s => match s.parse::<usize>() {
            Ok(n) => n.min(trace_data.total_steps().saturating_sub(1)),
            Err(_) => trace_data.min_step(),
        },
    };

    // Get keyframes and find the one at or before target_step
    let keyframes = trace_data.keyframes();
    let keyframe = keyframes
        .iter()
        .filter(|k| k.step_index <= target_step)
        .max_by_key(|k| k.step_index);

    let (shapes_json, kf_step) = match keyframe {
        Some(kf) => (kf.shapes.clone(), kf.step_index),
        None => {
            // No keyframe found, use first available or return error
            if let Some(first) = keyframes.first() {
                (first.shapes.clone(), first.step_index)
            } else {
                return JsonRpcResponse {
                    id: id.to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: "No keyframes found in trace".to_string(),
                    }),
                };
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
        Err(e) => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to parse shapes from keyframe: {}", e),
            }),
        },
    };

    // Convert to InputSpec (assume all coordinates trainable, matching original)
    let inputs: Vec<InputSpec> = trace_data.inputs().clone();

    // Update shapes in inputs to match loaded keyframe
    let inputs_with_loaded_shapes: Vec<InputSpec> = inputs
        .iter()
        .zip(shapes.iter())
        .map(|((_, trainable), shape)| (shape.clone(), trainable.clone()))
        .collect();

    let targets = trace_data.targets().clone();
    let learning_rate = trace_data.learning_rate();

    // Create initial step from loaded shapes
    let step = match Step::new(inputs_with_loaded_shapes.clone(), targets.clone().into()) {
        Ok(s) => s,
        Err(e) => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to create step from loaded shapes: {}", e),
            }),
        },
    };

    // Recompute forward from keyframe to target step if needed
    let mut current_step = step;
    for _ in kf_step..target_step {
        match current_step.step(learning_rate) {
            Ok(next) => current_step = next,
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: format!("Failed to recompute to step {}: {}", target_step, e),
                }),
            },
        }
    }

    // Create new training session
    let session = Arc::new(TrainingSession::new(inputs.clone(), targets.clone(), learning_rate, StorageStrategy::Tiered));

    // Record the loaded step
    session.record_step(target_step, current_step.clone(), current_step.error.v());

    // Also populate storage with keyframes from the trace for time-travel
    for kf in &keyframes {
        if kf.step_index != target_step {
            // Parse shapes and create step for storage
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

    // Build response with geometry
    let num_shapes = current_step.shapes.len();
    let region_key_to_exclusive = |key: &str| -> String {
        let included: std::collections::HashSet<char> = key.chars().collect();
        (0..num_shapes)
            .map(|i| {
                let ch = char::from_digit(i as u32, 10).unwrap();
                if included.contains(&ch) { ch } else { '-' }
            })
            .collect()
    };

    let mut region_infos: Vec<RegionInfo> = Vec::new();
    for component in &current_step.components {
        for region in &component.regions {
            let exclusive_key = region_key_to_exclusive(&region.key);
            let error_info = current_step.errors.get(&exclusive_key);
            region_infos.push(RegionInfo {
                key: region.key.clone(),
                area: region.area,
                target_area: error_info.map(|e| e.target_area),
                error: error_info.map(|e| e.error.v()).unwrap_or(0.0),
            });
        }
    }

    let shapes_json: Vec<Value> = current_step.shapes.iter()
        .map(|s| serde_json::to_value(s.v()).unwrap())
        .collect();

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
        "keyframeCount": keyframes.len(),
        "step": StepStateWithGeometry {
            step_index: target_step,
            error: current_step.error.v(),
            shapes: shapes_json,
            is_keyframe: true,
            geometry: StepGeometry {
                components: current_step.components.clone(),
                regions: region_infos,
            },
        },
    });

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(result),
        error: None,
    }
}

/// Handle saveTrace RPC - save current session's trace.
fn handle_save_trace(
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
fn handle_list_traces(
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
fn handle_rename_trace(
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
fn handle_delete_trace(
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
fn handle_load_saved_trace(
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

    // Get trace from store
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
    let trace_meta = saved_trace.meta.clone();

    // Determine which step to load
    let target_step = match load_step {
        "best" => trace_data.min_step(),
        "last" => trace_data.total_steps().saturating_sub(1),
        "first" | "0" => 0,
        s => match s.parse::<usize>() {
            Ok(n) => n.min(trace_data.total_steps().saturating_sub(1)),
            Err(_) => trace_data.min_step(),
        },
    };

    // Get keyframes and find the one at or before target_step
    let keyframes = trace_data.keyframes();
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
                return JsonRpcResponse {
                    id: id.to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: "No keyframes found in trace".to_string(),
                    }),
                };
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
        Err(e) => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to parse shapes from keyframe: {}", e),
            }),
        },
    };

    let inputs = trace_data.inputs().clone();
    let inputs_with_loaded_shapes: Vec<InputSpec> = inputs
        .iter()
        .zip(shapes.iter())
        .map(|((_, trainable), shape)| (shape.clone(), trainable.clone()))
        .collect();

    let targets = trace_data.targets().clone();
    let learning_rate = trace_data.learning_rate();

    // Drop the store lock before creating session
    drop(store);

    // Create initial step from loaded shapes
    let step = match Step::new(inputs_with_loaded_shapes.clone(), targets.clone().into()) {
        Ok(s) => s,
        Err(e) => return JsonRpcResponse {
            id: id.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: format!("Failed to create step from loaded shapes: {}", e),
            }),
        },
    };

    // Recompute forward from keyframe to target step if needed
    let mut current_step = step;
    for _ in kf_step..target_step {
        match current_step.step(learning_rate) {
            Ok(next) => current_step = next,
            Err(e) => return JsonRpcResponse {
                id: id.to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: format!("Failed to recompute to step {}: {}", target_step, e),
                }),
            },
        }
    }

    // Create new training session
    let session = Arc::new(TrainingSession::new(inputs.clone(), targets.clone(), learning_rate, StorageStrategy::Tiered));
    session.record_step(target_step, current_step.clone(), current_step.error.v());

    // Install the session
    {
        let mut current = current_session.lock().unwrap();
        if let Some(old_session) = current.take() {
            old_session.request_stop();
        }
        *current = Some(session);
    }

    // Build response with geometry
    let num_shapes = current_step.shapes.len();
    let region_key_to_exclusive = |key: &str| -> String {
        let included: std::collections::HashSet<char> = key.chars().collect();
        (0..num_shapes)
            .map(|i| {
                let ch = char::from_digit(i as u32, 10).unwrap();
                if included.contains(&ch) { ch } else { '-' }
            })
            .collect()
    };

    let mut region_infos: Vec<RegionInfo> = Vec::new();
    for component in &current_step.components {
        for region in &component.regions {
            let exclusive_key = region_key_to_exclusive(&region.key);
            let error_info = current_step.errors.get(&exclusive_key);
            region_infos.push(RegionInfo {
                key: region.key.clone(),
                area: region.area,
                target_area: error_info.map(|e| e.target_area),
                error: error_info.map(|e| e.error.v()).unwrap_or(0.0),
            });
        }
    }

    let result_shapes: Vec<Value> = current_step.shapes.iter()
        .map(|s| serde_json::to_value(s.v()).unwrap())
        .collect();

    let result = serde_json::json!({
        "loaded": true,
        "traceId": trace_meta.trace_id,
        "name": trace_meta.name,
        "loadedStep": target_step,
        "totalSteps": trace_meta.total_steps,
        "minStep": trace_meta.min_step,
        "minError": trace_meta.min_error,
        "keyframeCount": keyframes.len(),
        "step": StepStateWithGeometry {
            step_index: target_step,
            error: current_step.error.v(),
            shapes: result_shapes,
            is_keyframe: true,
            geometry: StepGeometry {
                components: current_step.components.clone(),
                regions: region_infos,
            },
        },
    });

    JsonRpcResponse {
        id: id.to_string(),
        result: Some(result),
        error: None,
    }
}

/// Handle listSampleTraces RPC - list available sample traces.
fn handle_list_sample_traces(
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
            // Only look at .trace.json files
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if filename.ends_with(".trace.json") {
                // Extract name without .trace.json suffix
                let name = filename.strip_suffix(".trace.json")
                    .unwrap_or(filename)
                    .to_string();

                // Get file size
                let size_bytes = entry.metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);

                // Try to read basic metadata from file
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(trace) = serde_json::from_str::<serde_json::Value>(&contents) {
                        // Try V2 format first, then fall back to V1 format
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

                        // Count shapes from config (V2) or inputs (V1)
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
fn handle_load_sample_trace(
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

    // Extract name from filename
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
