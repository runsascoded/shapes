//! WebSocket server for real-time training updates.
//!
//! The server accepts WebSocket connections and handles training requests,
//! streaming step updates back to the client.

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
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use apvd_core::{InputSpec, Model, TargetsMap};

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

/// Messages from server to client
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Training step update
    StepUpdate {
        step_idx: usize,
        error: f64,
        min_error: f64,
        min_step: usize,
        shapes: Vec<serde_json::Value>,
        converged: bool,
    },
    /// Training completed
    TrainingComplete {
        total_steps: usize,
        final_error: f64,
        min_error: f64,
        min_step: usize,
    },
    /// Error occurred
    Error { message: String },
    /// Pong response
    Pong,
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

async fn handle_socket(socket: WebSocket, _config: Arc<ServerConfig>) {
    let (mut sender, mut receiver) = socket.split();

    // Channel for sending messages back to the client
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

    // Task to forward messages to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
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
                let client_msg: Result<ClientMessage, _> = serde_json::from_str(&text);
                match client_msg {
                    Ok(ClientMessage::StartTraining {
                        shapes,
                        targets,
                        max_steps,
                        learning_rate,
                        robust,
                    }) => {
                        let tx = tx.clone();
                        // Run training in a blocking task (uses rayon internally)
                        tokio::task::spawn_blocking(move || {
                            run_training(shapes, targets, max_steps, learning_rate, robust, tx);
                        });
                    }
                    Ok(ClientMessage::StopTraining) => {
                        // TODO: Implement cancellation
                        let _ = tx.send(ServerMessage::Error {
                            message: "Stop not yet implemented".to_string(),
                        }).await;
                    }
                    Ok(ClientMessage::Ping) => {
                        let _ = tx.send(ServerMessage::Pong).await;
                    }
                    Err(e) => {
                        let _ = tx.send(ServerMessage::Error {
                            message: format!("Invalid message: {}", e),
                        }).await;
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Clean up
    drop(tx);
    let _ = send_task.await;
}

fn run_training(
    shapes: Vec<InputSpec>,
    targets: TargetsMap<f64>,
    max_steps: usize,
    learning_rate: f64,
    robust: bool,
    tx: mpsc::Sender<ServerMessage>,
) {
    // Create model
    let model_result = Model::new(shapes, targets.clone());
    let mut model = match model_result {
        Ok(m) => m,
        Err(e) => {
            let _ = tx.blocking_send(ServerMessage::Error {
                message: format!("Failed to create model: {}", e),
            });
            return;
        }
    };

    // Send initial step
    send_step_update(&tx, &model, 0);

    // Training loop - we do it manually to send updates
    for step_idx in 0..max_steps {
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
                    let _ = tx.blocking_send(ServerMessage::Error {
                        message: "NaN error encountered".to_string(),
                    });
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
                    send_step_update(&tx, &model, step_idx + 1);
                }
            }
            Err(e) => {
                let _ = tx.blocking_send(ServerMessage::Error {
                    message: format!("Step failed: {}", e),
                });
                break;
            }
        }
    }

    // Send final update
    let final_step = model.steps.last().unwrap();
    let _ = tx.blocking_send(ServerMessage::TrainingComplete {
        total_steps: model.steps.len(),
        final_error: final_step.error.v(),
        min_error: model.min_error,
        min_step: model.min_idx,
    });
}

fn send_step_update(tx: &mpsc::Sender<ServerMessage>, model: &Model, step_idx: usize) {
    let step = model.steps.last().unwrap();
    let shapes: Vec<serde_json::Value> = step
        .shapes
        .iter()
        .map(|s| serde_json::to_value(s).unwrap())
        .collect();

    let _ = tx.blocking_send(ServerMessage::StepUpdate {
        step_idx,
        error: step.error.v(),
        min_error: model.min_error,
        min_step: model.min_idx,
        shapes,
        converged: step.converged,
    });
}
