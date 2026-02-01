//! Trace storage abstraction for training histories.
//!
//! Provides a trait for storing and retrieving training steps, with multiple
//! implementations for different storage strategies.

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use super::step::Step;
use super::tiered::TieredConfig;

/// A stored step with its index and whether it's a keyframe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredStep {
    pub index: usize,
    pub step: Step,
    pub is_keyframe: bool,
}

/// Trait for trace storage implementations.
///
/// Different storage strategies can be used:
/// - Dense: Store every step (no compression)
/// - Tiered: Store keyframes at progressively sparser intervals
/// - BTD: Store only "best to date" steps (new minimum errors)
/// - Custom combinations
pub trait TraceStorage: Send + Sync {
    /// Record a training step. The implementation decides whether to store it.
    fn record(&mut self, index: usize, step: Step, error: f64);

    /// Get a step by index. May recompute from a keyframe if not stored directly.
    fn get(&self, index: usize, learning_rate: f64) -> Result<Step, String>;

    /// Check if a step is stored directly (without recomputation).
    fn is_stored(&self, index: usize) -> bool;

    /// Get the total number of steps recorded.
    fn len(&self) -> usize;

    /// Check if storage is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the number of steps actually stored.
    fn stored_count(&self) -> usize;

    /// Get all stored step indices.
    fn stored_indices(&self) -> Vec<usize>;

    /// Get the minimum error seen so far.
    fn min_error(&self) -> f64;

    /// Get the index of the minimum error step.
    fn min_index(&self) -> usize;

    /// Get storage metadata for serialization.
    fn metadata(&self) -> TraceMetadata;
}

/// Metadata about a stored trace.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct TraceMetadata {
    /// Total steps in the trace.
    pub total_steps: usize,
    /// Number of steps actually stored.
    pub stored_steps: usize,
    /// Storage strategy name.
    pub strategy: String,
    /// Step index with minimum error.
    pub min_index: usize,
    /// Minimum error value.
    pub min_error: f64,
    /// Indices of "best to date" steps (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub btd_indices: Option<Vec<usize>>,
}

// ============================================================================
// Dense Storage - Store every step
// ============================================================================

/// Dense storage that keeps every step in memory.
#[derive(Debug, Clone)]
pub struct DenseStorage {
    steps: Vec<Step>,
    min_error: f64,
    min_index: usize,
}

impl DenseStorage {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            min_error: f64::INFINITY,
            min_index: 0,
        }
    }
}

impl Default for DenseStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceStorage for DenseStorage {
    fn record(&mut self, index: usize, step: Step, error: f64) {
        // Ensure we have space
        while self.steps.len() <= index {
            // This shouldn't happen in normal use, but handle gaps gracefully
            if self.steps.is_empty() {
                break;
            }
            self.steps.push(self.steps.last().unwrap().clone());
        }

        if index == self.steps.len() {
            self.steps.push(step);
        } else {
            self.steps[index] = step;
        }

        if error < self.min_error {
            self.min_error = error;
            self.min_index = index;
        }
    }

    fn get(&self, index: usize, _learning_rate: f64) -> Result<Step, String> {
        self.steps
            .get(index)
            .cloned()
            .ok_or_else(|| format!("Step {} not found", index))
    }

    fn is_stored(&self, index: usize) -> bool {
        index < self.steps.len()
    }

    fn len(&self) -> usize {
        self.steps.len()
    }

    fn stored_count(&self) -> usize {
        self.steps.len()
    }

    fn stored_indices(&self) -> Vec<usize> {
        (0..self.steps.len()).collect()
    }

    fn min_error(&self) -> f64 {
        self.min_error
    }

    fn min_index(&self) -> usize {
        self.min_index
    }

    fn metadata(&self) -> TraceMetadata {
        TraceMetadata {
            total_steps: self.steps.len(),
            stored_steps: self.steps.len(),
            strategy: "dense".to_string(),
            min_index: self.min_index,
            min_error: self.min_error,
            btd_indices: None,
        }
    }
}

// ============================================================================
// Tiered Storage - Keyframes at progressively sparser intervals
// ============================================================================

/// Tiered keyframe storage with O(log N) space complexity.
#[derive(Debug, Clone)]
pub struct TieredStorage {
    config: TieredConfig,
    keyframes: BTreeMap<usize, Step>,
    total_steps: usize,
    min_error: f64,
    min_index: usize,
    btd_indices: BTreeSet<usize>,
}

impl TieredStorage {
    pub fn new(config: TieredConfig) -> Self {
        Self {
            config,
            keyframes: BTreeMap::new(),
            total_steps: 0,
            min_error: f64::INFINITY,
            min_index: 0,
            btd_indices: BTreeSet::new(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(TieredConfig::default())
    }

    /// Get the tiered config.
    pub fn config(&self) -> &TieredConfig {
        &self.config
    }
}

impl TraceStorage for TieredStorage {
    fn record(&mut self, index: usize, step: Step, error: f64) {
        self.total_steps = self.total_steps.max(index + 1);

        // Track best-to-date
        if error < self.min_error {
            self.min_error = error;
            self.min_index = index;
            self.btd_indices.insert(index);
        }

        // Only store keyframes
        if self.config.is_keyframe(index) {
            self.keyframes.insert(index, step);
        }
    }

    fn get(&self, index: usize, learning_rate: f64) -> Result<Step, String> {
        // Check if directly stored
        if let Some(step) = self.keyframes.get(&index) {
            return Ok(step.clone());
        }

        // Find nearest keyframe and recompute
        let keyframe_idx = self.config.nearest_keyframe(index);
        let keyframe = self.keyframes.get(&keyframe_idx).ok_or_else(|| {
            format!(
                "Keyframe {} not found for step {}",
                keyframe_idx, index
            )
        })?;

        super::tiered::seek_from_keyframe(keyframe, keyframe_idx, index, learning_rate)
    }

    fn is_stored(&self, index: usize) -> bool {
        self.keyframes.contains_key(&index)
    }

    fn len(&self) -> usize {
        self.total_steps
    }

    fn stored_count(&self) -> usize {
        self.keyframes.len()
    }

    fn stored_indices(&self) -> Vec<usize> {
        self.keyframes.keys().cloned().collect()
    }

    fn min_error(&self) -> f64 {
        self.min_error
    }

    fn min_index(&self) -> usize {
        self.min_index
    }

    fn metadata(&self) -> TraceMetadata {
        TraceMetadata {
            total_steps: self.total_steps,
            stored_steps: self.keyframes.len(),
            strategy: "tiered".to_string(),
            min_index: self.min_index,
            min_error: self.min_error,
            btd_indices: Some(self.btd_indices.iter().cloned().collect()),
        }
    }
}

// ============================================================================
// BTD Storage - Only store "best to date" steps
// ============================================================================

/// Storage that only keeps steps achieving new minimum error.
#[derive(Debug, Clone)]
pub struct BtdStorage {
    steps: BTreeMap<usize, Step>,
    total_steps: usize,
    min_error: f64,
    min_index: usize,
}

impl BtdStorage {
    pub fn new() -> Self {
        Self {
            steps: BTreeMap::new(),
            total_steps: 0,
            min_error: f64::INFINITY,
            min_index: 0,
        }
    }
}

impl Default for BtdStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceStorage for BtdStorage {
    fn record(&mut self, index: usize, step: Step, error: f64) {
        self.total_steps = self.total_steps.max(index + 1);

        // Only store if this is a new minimum
        if error < self.min_error {
            self.min_error = error;
            self.min_index = index;
            self.steps.insert(index, step);
        }
    }

    fn get(&self, index: usize, _learning_rate: f64) -> Result<Step, String> {
        // BTD storage can only return stored steps
        self.steps
            .get(&index)
            .cloned()
            .ok_or_else(|| format!("Step {} is not a BTD step", index))
    }

    fn is_stored(&self, index: usize) -> bool {
        self.steps.contains_key(&index)
    }

    fn len(&self) -> usize {
        self.total_steps
    }

    fn stored_count(&self) -> usize {
        self.steps.len()
    }

    fn stored_indices(&self) -> Vec<usize> {
        self.steps.keys().cloned().collect()
    }

    fn min_error(&self) -> f64 {
        self.min_error
    }

    fn min_index(&self) -> usize {
        self.min_index
    }

    fn metadata(&self) -> TraceMetadata {
        TraceMetadata {
            total_steps: self.total_steps,
            stored_steps: self.steps.len(),
            strategy: "btd".to_string(),
            min_index: self.min_index,
            min_error: self.min_error,
            btd_indices: Some(self.steps.keys().cloned().collect()),
        }
    }
}

// ============================================================================
// Hybrid Storage - Tiered keyframes + BTD steps
// ============================================================================

/// Hybrid storage combining tiered keyframes with BTD step tracking.
///
/// Stores:
/// - All tiered keyframes (for efficient random seek)
/// - All BTD steps (even if not keyframes, for quick access to best steps)
#[derive(Debug, Clone)]
pub struct HybridStorage {
    config: TieredConfig,
    keyframes: BTreeMap<usize, Step>,
    btd_steps: BTreeMap<usize, Step>,
    total_steps: usize,
    min_error: f64,
    min_index: usize,
}

impl HybridStorage {
    pub fn new(config: TieredConfig) -> Self {
        Self {
            config,
            keyframes: BTreeMap::new(),
            btd_steps: BTreeMap::new(),
            total_steps: 0,
            min_error: f64::INFINITY,
            min_index: 0,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(TieredConfig::default())
    }
}

impl TraceStorage for HybridStorage {
    fn record(&mut self, index: usize, step: Step, error: f64) {
        self.total_steps = self.total_steps.max(index + 1);

        // Store keyframes
        if self.config.is_keyframe(index) {
            self.keyframes.insert(index, step.clone());
        }

        // Store BTD steps
        if error < self.min_error {
            self.min_error = error;
            self.min_index = index;
            self.btd_steps.insert(index, step);
        }
    }

    fn get(&self, index: usize, learning_rate: f64) -> Result<Step, String> {
        // Check BTD steps first (they're "important" steps)
        if let Some(step) = self.btd_steps.get(&index) {
            return Ok(step.clone());
        }

        // Check keyframes
        if let Some(step) = self.keyframes.get(&index) {
            return Ok(step.clone());
        }

        // Recompute from nearest keyframe
        let keyframe_idx = self.config.nearest_keyframe(index);
        let keyframe = self.keyframes.get(&keyframe_idx).ok_or_else(|| {
            format!(
                "Keyframe {} not found for step {}",
                keyframe_idx, index
            )
        })?;

        super::tiered::seek_from_keyframe(keyframe, keyframe_idx, index, learning_rate)
    }

    fn is_stored(&self, index: usize) -> bool {
        self.keyframes.contains_key(&index) || self.btd_steps.contains_key(&index)
    }

    fn len(&self) -> usize {
        self.total_steps
    }

    fn stored_count(&self) -> usize {
        // Count unique indices (some may be in both)
        let mut indices: BTreeSet<usize> = self.keyframes.keys().cloned().collect();
        indices.extend(self.btd_steps.keys().cloned());
        indices.len()
    }

    fn stored_indices(&self) -> Vec<usize> {
        let mut indices: BTreeSet<usize> = self.keyframes.keys().cloned().collect();
        indices.extend(self.btd_steps.keys().cloned());
        indices.into_iter().collect()
    }

    fn min_error(&self) -> f64 {
        self.min_error
    }

    fn min_index(&self) -> usize {
        self.min_index
    }

    fn metadata(&self) -> TraceMetadata {
        TraceMetadata {
            total_steps: self.total_steps,
            stored_steps: self.stored_count(),
            strategy: "hybrid".to_string(),
            min_index: self.min_index,
            min_error: self.min_error,
            btd_indices: Some(self.btd_steps.keys().cloned().collect()),
        }
    }
}

/// Storage strategy enum for easy configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum StorageStrategy {
    /// Store every step.
    Dense,
    /// Store tiered keyframes only.
    Tiered,
    /// Store only best-to-date steps.
    Btd,
    /// Store tiered keyframes + BTD steps.
    Hybrid,
}

impl Default for StorageStrategy {
    fn default() -> Self {
        Self::Tiered
    }
}

/// Create a storage instance based on strategy.
pub fn create_storage(strategy: StorageStrategy, bucket_size: Option<usize>) -> Box<dyn TraceStorage> {
    match strategy {
        StorageStrategy::Dense => Box::new(DenseStorage::new()),
        StorageStrategy::Tiered => Box::new(TieredStorage::new(TieredConfig::new(bucket_size))),
        StorageStrategy::Btd => Box::new(BtdStorage::new()),
        StorageStrategy::Hybrid => Box::new(HybridStorage::new(TieredConfig::new(bucket_size))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock step for testing (we can't easily create real Steps without full setup)
    fn mock_step() -> Step {
        // This would need real inputs/targets - for now we'll skip step-dependent tests
        unimplemented!("Need mock Step for tests")
    }

    #[test]
    fn test_storage_strategy_default() {
        assert_eq!(StorageStrategy::default(), StorageStrategy::Tiered);
    }

    #[test]
    fn test_tiered_storage_keyframe_selection() {
        let config = TieredConfig::new(Some(100));
        let storage = TieredStorage::new(config);

        // Tier 0: all steps are keyframes
        assert!(storage.config.is_keyframe(0));
        assert!(storage.config.is_keyframe(50));
        assert!(storage.config.is_keyframe(199));

        // Tier 1: every 2nd step
        assert!(storage.config.is_keyframe(200));
        assert!(!storage.config.is_keyframe(201));
        assert!(storage.config.is_keyframe(202));
    }
}
