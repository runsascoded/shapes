//! JSON-RPC protocol types and utility functions.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use apvd_core::{Shape, regions};

/// Server configuration
#[derive(Clone)]
pub struct ServerConfig {
    pub parallel: usize,
    /// Directory for persistent trace storage (None = in-memory only)
    pub storage_dir: Option<std::path::PathBuf>,
    /// Directory containing sample traces (None = no samples)
    pub samples_dir: Option<std::path::PathBuf>,
}

/// JSON-RPC request message
#[derive(Debug, Deserialize)]
pub(super) struct JsonRpcRequest {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC response message
#[derive(Debug, Serialize)]
pub(super) struct JsonRpcResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub(super) struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// JSON-RPC notification (no id, server-initiated)
#[derive(Debug, Serialize)]
pub(super) struct JsonRpcNotification {
    pub method: String,
    pub params: Value,
}

/// Step state with geometry for frontend display
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StepStateWithGeometry {
    pub step_index: usize,
    pub error: f64,
    pub shapes: Vec<Value>,
    pub is_keyframe: bool,
    pub geometry: StepGeometry,
}

#[derive(Debug, Serialize)]
pub(super) struct StepGeometry {
    pub components: Vec<regions::Component>,
    pub regions: Vec<RegionInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RegionInfo {
    pub key: String,
    pub area: f64,
    pub target_area: Option<f64>,
    pub error: f64,
}

/// Progress update for streaming training progress
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProgressUpdate {
    pub handle_id: String,
    #[serde(rename = "type")]
    pub update_type: String,
    pub current_step: usize,
    pub total_steps: usize,
    pub error: f64,
    pub min_error: f64,
    pub min_step: usize,
    pub shapes: Vec<Value>,
    pub elapsed_ms: u64,
    pub converged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Training handle returned when training starts
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrainingHandle {
    pub id: String,
    pub started_at: u64,
}

/// Metadata for a saved trace
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SavedTraceMeta {
    pub trace_id: String,
    pub name: String,
    pub saved_at: String,
    pub total_steps: usize,
    pub min_error: f64,
    pub min_step: usize,
    pub num_shapes: usize,
    pub shape_types: Vec<String>,
}

/// Check if a shape has any NaN coordinates
pub(super) fn shape_has_nan(shape: &Shape<f64>) -> bool {
    match shape {
        Shape::Circle(c) => c.c.x.is_nan() || c.c.y.is_nan() || c.r.is_nan(),
        Shape::XYRR(e) => e.c.x.is_nan() || e.c.y.is_nan() || e.r.x.is_nan() || e.r.y.is_nan(),
        Shape::XYRRT(e) => e.c.x.is_nan() || e.c.y.is_nan() || e.r.x.is_nan() || e.r.y.is_nan() || e.t.is_nan(),
        Shape::Polygon(p) => p.vertices.iter().any(|v| v.x.is_nan() || v.y.is_nan()),
    }
}

/// Convert a region key (e.g. "01") to exclusive key format (e.g. "01-")
/// by filling in '-' for shapes not included in the key.
pub(super) fn region_key_to_exclusive(key: &str, num_shapes: usize) -> String {
    let included: HashSet<char> = key.chars().collect();
    (0..num_shapes)
        .map(|i| {
            let ch = char::from_digit(i as u32, 10).unwrap();
            if included.contains(&ch) { ch } else { '-' }
        })
        .collect()
}
