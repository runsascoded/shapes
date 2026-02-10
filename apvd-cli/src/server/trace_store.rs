//! Persistent trace storage for saved traces.

use serde::{Deserialize, Serialize};

use apvd_core::Shape;

use crate::trace::{TraceData, TraceFileV2};
use super::protocol::SavedTraceMeta;

/// A saved trace with full data
pub(super) struct SavedTrace {
    pub meta: SavedTraceMeta,
    pub data: TraceData,
}

/// Storage for saved traces (per-connection, optionally persistent)
pub(super) struct TraceStore {
    traces: std::collections::HashMap<String, SavedTrace>,
    counter: usize,
    /// Optional directory for persistent storage
    storage_dir: Option<std::path::PathBuf>,
}

impl TraceStore {
    pub fn new() -> Self {
        Self {
            traces: std::collections::HashMap::new(),
            counter: 0,
            storage_dir: None,
        }
    }

    /// Create a persistent store that saves to the given directory.
    pub fn with_storage_dir(dir: std::path::PathBuf) -> Self {
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

    pub fn save(&mut self, data: TraceData, name: String) -> SavedTraceMeta {
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

    pub fn list(&self) -> Vec<SavedTraceMeta> {
        self.traces.values().map(|t| t.meta.clone()).collect()
    }

    pub fn get(&self, trace_id: &str) -> Option<&SavedTrace> {
        self.traces.get(trace_id)
    }

    pub fn rename(&mut self, trace_id: &str, name: String) -> Option<SavedTraceMeta> {
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

    pub fn delete(&mut self, trace_id: &str) -> bool {
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
    trace: TraceFileV2,
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
