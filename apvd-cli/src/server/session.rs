//! Shared state for a training session.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use apvd_core::{
    InputSpec, Step, TargetsMap, PhaseConfig,
    TraceStorage, StorageStrategy, create_storage,
};

/// Shared state for a training session
pub(super) struct TrainingSession {
    /// Flag to signal training should stop
    stop_requested: AtomicBool,
    /// Current best error across all variants
    best_error: std::sync::Mutex<f64>,
    /// Which variant currently has best error
    best_variant: AtomicUsize,
    /// Trace storage for steps (tiered by default)
    storage: std::sync::Mutex<Box<dyn TraceStorage>>,
    /// Phase config used for training (needed for recomputation from keyframes)
    pub phase_config: PhaseConfig,
    /// Original inputs (for trace export)
    pub inputs: Vec<InputSpec>,
    /// Original targets (for trace export)
    pub targets: TargetsMap<f64>,
}

impl TrainingSession {
    pub fn new(inputs: Vec<InputSpec>, targets: TargetsMap<f64>, learning_rate: f64, storage_strategy: StorageStrategy) -> Self {
        Self {
            stop_requested: AtomicBool::new(false),
            best_error: std::sync::Mutex::new(f64::INFINITY),
            best_variant: AtomicUsize::new(0),
            storage: std::sync::Mutex::new(create_storage(storage_strategy, None)),
            phase_config: PhaseConfig { learning_rate, ..PhaseConfig::default() },
            inputs,
            targets,
        }
    }

    pub fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::SeqCst);
    }

    pub fn should_stop(&self) -> bool {
        self.stop_requested.load(Ordering::SeqCst)
    }

    pub fn update_best(&self, variant_id: usize, error: f64) -> bool {
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
    pub fn record_step(&self, index: usize, step: Step, error: f64) {
        let mut storage = self.storage.lock().unwrap();
        storage.record(index, step, error);
    }

    /// Get a step from storage (may recompute from keyframe).
    pub fn get_step(&self, index: usize) -> Result<Step, String> {
        let storage = self.storage.lock().unwrap();
        storage.get(index, &self.phase_config)
    }

    /// Get storage metadata.
    pub fn get_metadata(&self) -> apvd_core::trace::TraceMetadata {
        let storage = self.storage.lock().unwrap();
        storage.metadata()
    }
}
