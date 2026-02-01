//! Trace storage abstraction for training histories.
//!
//! Provides a trait for storing and retrieving training steps, with multiple
//! implementations for different storage strategies.

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use super::step::Step;

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
// Tiered LRU Storage - Recent steps at full resolution, older steps sparse
// ============================================================================

/// Tiered LRU storage: recent steps at full resolution, older steps progressively sparser.
///
/// Unlike `TieredStorage` which uses absolute step indices, this uses "age"
/// (distance from current step) to determine tier. As training progresses,
/// older steps are compacted.
///
/// Tier structure (based on age = current_step - step_index):
/// - Tier 0: age 0 to 2B-1 (most recent 2B steps, full resolution)
/// - Tier 1: age 2B to 4B-1 (next 2B steps, keep every 2nd)
/// - Tier 2: age 4B to 8B-1 (next 4B steps, keep every 4th)
/// - Tier n: age 2^n*B to 2^(n+1)*B-1 (keep every 2^n-th)
#[derive(Debug, Clone)]
pub struct TieredLruStorage {
    bucket_size: usize,
    /// All stored steps (keyed by step index)
    steps: BTreeMap<usize, Step>,
    /// Current step count (determines age of all steps)
    total_steps: usize,
    min_error: f64,
    min_index: usize,
    btd_indices: BTreeSet<usize>,
}

impl TieredLruStorage {
    pub const DEFAULT_BUCKET_SIZE: usize = 1024;

    pub fn new(bucket_size: Option<usize>) -> Self {
        Self {
            bucket_size: bucket_size.unwrap_or(Self::DEFAULT_BUCKET_SIZE),
            steps: BTreeMap::new(),
            total_steps: 0,
            min_error: f64::INFINITY,
            min_index: 0,
            btd_indices: BTreeSet::new(),
        }
    }

    /// Calculate tier based on age (distance from current step).
    fn tier_for_age(&self, age: usize) -> usize {
        let b = self.bucket_size;
        if age < 2 * b {
            0
        } else {
            (age / b).ilog2() as usize
        }
    }

    /// Resolution (keep every Nth step) for a tier.
    fn resolution(&self, tier: usize) -> usize {
        1 << tier
    }

    /// Check if a step should be kept given its current age.
    fn should_keep(&self, step_idx: usize, age: usize) -> bool {
        let tier = self.tier_for_age(age);
        let res = self.resolution(tier);
        // Keep if step_idx is divisible by resolution
        step_idx % res == 0
    }

    /// Find the nearest kept step at or before the given index.
    fn nearest_kept(&self, index: usize) -> Option<usize> {
        // Find the largest stored index <= target
        self.steps.range(..=index).next_back().map(|(k, _)| *k)
    }

    /// Compact storage by dropping steps that no longer qualify as keyframes.
    fn compact(&mut self) {
        if self.total_steps == 0 {
            return;
        }

        let current = self.total_steps - 1;
        let mut to_remove = Vec::new();

        for &step_idx in self.steps.keys() {
            let age = current - step_idx;
            if !self.should_keep(step_idx, age) {
                // Don't remove BTD steps
                if !self.btd_indices.contains(&step_idx) {
                    to_remove.push(step_idx);
                }
            }
        }

        for idx in to_remove {
            self.steps.remove(&idx);
        }
    }
}

impl TraceStorage for TieredLruStorage {
    fn record(&mut self, index: usize, step: Step, error: f64) {
        self.total_steps = self.total_steps.max(index + 1);

        // Track best-to-date
        if error < self.min_error {
            self.min_error = error;
            self.min_index = index;
            self.btd_indices.insert(index);
        }

        // Always store the new step (it's in tier 0 with age 0)
        self.steps.insert(index, step);

        // Compact old steps that have aged into sparser tiers
        self.compact();
    }

    fn get(&self, index: usize, learning_rate: f64) -> Result<Step, String> {
        if index >= self.total_steps {
            return Err(format!("Step {} not yet recorded (total: {})", index, self.total_steps));
        }

        // Check if directly stored
        if let Some(step) = self.steps.get(&index) {
            return Ok(step.clone());
        }

        // Find nearest stored step and recompute forward
        let nearest_idx = self.nearest_kept(index)
            .ok_or_else(|| format!("No keyframe found for step {}", index))?;

        let keyframe = self.steps.get(&nearest_idx)
            .ok_or_else(|| format!("Keyframe {} not in storage", nearest_idx))?;

        super::tiered::seek_from_keyframe(keyframe, nearest_idx, index, learning_rate)
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
            strategy: "tiered-lru".to_string(),
            min_index: self.min_index,
            min_error: self.min_error,
            btd_indices: Some(self.btd_indices.iter().cloned().collect()),
        }
    }
}

/// Storage strategy enum for easy configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum StorageStrategy {
    /// Store every step (no compression).
    Dense,
    /// Store only best-to-date steps (new minimum errors).
    Btd,
    /// Tiered: recent at full resolution, older progressively sparser (default).
    Tiered,
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
        StorageStrategy::Btd => Box::new(BtdStorage::new()),
        StorageStrategy::Tiered => Box::new(TieredLruStorage::new(bucket_size)),
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
    fn test_tiered_lru_tier_for_age() {
        let storage = TieredLruStorage::new(Some(100)); // B=100

        // Tier 0: age 0 to 199 (most recent 2B steps)
        assert_eq!(storage.tier_for_age(0), 0);
        assert_eq!(storage.tier_for_age(50), 0);
        assert_eq!(storage.tier_for_age(199), 0);

        // Tier 1: age 200 to 399
        assert_eq!(storage.tier_for_age(200), 1);
        assert_eq!(storage.tier_for_age(399), 1);

        // Tier 2: age 400 to 799
        assert_eq!(storage.tier_for_age(400), 2);
        assert_eq!(storage.tier_for_age(799), 2);
    }

    #[test]
    fn test_tiered_lru_should_keep() {
        let storage = TieredLruStorage::new(Some(100));

        // In tier 0 (age < 200): keep all
        assert!(storage.should_keep(0, 0));
        assert!(storage.should_keep(1, 1));
        assert!(storage.should_keep(99, 99));

        // In tier 1 (age 200-399): keep every 2nd (even step indices)
        assert!(storage.should_keep(0, 200));   // 0 % 2 == 0
        assert!(!storage.should_keep(1, 201));  // 1 % 2 == 1
        assert!(storage.should_keep(2, 202));   // 2 % 2 == 0
        assert!(!storage.should_keep(3, 203));  // 3 % 2 == 1

        // In tier 2 (age 400-799): keep every 4th
        assert!(storage.should_keep(0, 400));   // 0 % 4 == 0
        assert!(!storage.should_keep(1, 401));  // 1 % 4 == 1
        assert!(!storage.should_keep(2, 402));  // 2 % 4 == 2
        assert!(!storage.should_keep(3, 403));  // 3 % 4 == 3
        assert!(storage.should_keep(4, 404));   // 4 % 4 == 0
    }

    #[test]
    fn test_tiered_lru_resolution() {
        let storage = TieredLruStorage::new(Some(100));

        assert_eq!(storage.resolution(0), 1);
        assert_eq!(storage.resolution(1), 2);
        assert_eq!(storage.resolution(2), 4);
        assert_eq!(storage.resolution(3), 8);
    }
}
