//! Progress notification helpers for streaming training updates.

use apvd_core::Model;
use tokio::sync::mpsc;

use super::protocol::{JsonRpcNotification, ProgressUpdate};

/// Send a progress notification as JSON-RPC
pub(super) fn send_progress_update(
    tx: &mpsc::Sender<String>,
    handle_id: &str,
    model: &Model,
    step_idx: usize,
    total_steps: usize,
    start_time: std::time::Instant,
) {
    send_progress_update_with_type(tx, handle_id, model, step_idx, total_steps, start_time, "progress");
}

pub(super) fn send_progress_update_with_type(
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

pub(super) fn send_progress_notification(
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
