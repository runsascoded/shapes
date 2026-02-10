//! WebSocket server for real-time training updates.
//!
//! The server accepts WebSocket connections and handles training requests,
//! streaming step updates back to the client.
//!
//! Uses JSON-RPC protocol: `{"id": "req_1", "method": "createModel", "params": {...}}`

mod handlers;
mod progress;
mod protocol;
mod session;
mod trace_store;
mod training;

use std::net::SocketAddr;
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
use tokio::sync::mpsc;

pub use protocol::ServerConfig;
use protocol::{JsonRpcRequest, JsonRpcResponse, JsonRpcError};
use session::TrainingSession;
use trace_store::TraceStore;

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
                handlers::handle_create_model(&req.id, &req.params)
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
            handlers::handle_train_batch(&req.id, &req.params)
        },
        "train" => {
            handlers::handle_train(&req.id, &req.params, config, tx.clone(), current_session.clone()).await
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
            handlers::handle_load_trace(&req.id, &req.params, current_session, trace_store)
        },
        "saveTrace" => {
            handlers::handle_save_trace(&req.id, &req.params, current_session, trace_store)
        },
        "listTraces" => {
            handlers::handle_list_traces(&req.id, trace_store)
        },
        "renameTrace" => {
            handlers::handle_rename_trace(&req.id, &req.params, trace_store)
        },
        "deleteTrace" => {
            handlers::handle_delete_trace(&req.id, &req.params, trace_store)
        },
        "loadSavedTrace" => {
            handlers::handle_load_saved_trace(&req.id, &req.params, current_session, trace_store)
        },
        "listSampleTraces" => {
            handlers::handle_list_sample_traces(&req.id, config)
        },
        "loadSampleTrace" => {
            handlers::handle_load_sample_trace(&req.id, &req.params, config, current_session, trace_store)
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
