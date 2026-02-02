//! Trace storage abstraction for training histories.
//!
//! Provides a trait for storing and retrieving training steps, with multiple
//! implementations for different storage strategies.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
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

// ============================================================================
// BTD Evenly-Spaced Storage - Keep N BTD keyframes evenly distributed
// ============================================================================

/// BTD storage with evenly-spaced retention.
///
/// Keeps a maximum of N BTD keyframes, dropping the one with the smallest gap
/// to neighbors when over capacity. This maintains an even distribution of
/// keyframes across the step range.
///
/// Additionally, maintains interval keyframes at fixed spacing for bounded
/// recomputation (e.g., one keyframe every 1000 steps).
#[derive(Debug, Clone)]
pub struct BtdEvenlySpacedStorage {
    /// Maximum number of BTD keyframes to retain.
    max_btd_keyframes: usize,
    /// Interval keyframe spacing (0 to disable).
    interval_spacing: usize,

    /// BTD keyframes (step_index -> Step).
    btd_steps: BTreeMap<usize, Step>,
    /// Interval keyframes (step_index -> Step).
    interval_steps: BTreeMap<usize, Step>,

    /// Min-heap for gap-based eviction: (gap_if_removed, step_index).
    /// Gap is the distance between prev and next neighbors if this step were removed.
    gap_heap: BinaryHeap<Reverse<(usize, usize)>>,

    /// Total steps recorded.
    total_steps: usize,
    min_error: f64,
    min_index: usize,
}

impl BtdEvenlySpacedStorage {
    pub const DEFAULT_MAX_BTD_KEYFRAMES: usize = 1000;
    pub const DEFAULT_INTERVAL_SPACING: usize = 1000;

    pub fn new(max_btd_keyframes: Option<usize>, interval_spacing: Option<usize>) -> Self {
        Self {
            max_btd_keyframes: max_btd_keyframes.unwrap_or(Self::DEFAULT_MAX_BTD_KEYFRAMES),
            interval_spacing: interval_spacing.unwrap_or(Self::DEFAULT_INTERVAL_SPACING),
            btd_steps: BTreeMap::new(),
            interval_steps: BTreeMap::new(),
            gap_heap: BinaryHeap::new(),
            total_steps: 0,
            min_error: f64::INFINITY,
            min_index: 0,
        }
    }

    /// Get the step index before this one in the BTD set.
    fn prev_btd(&self, step_idx: usize) -> Option<usize> {
        self.btd_steps.range(..step_idx).next_back().map(|(k, _)| *k)
    }

    /// Get the step index after this one in the BTD set.
    fn next_btd(&self, step_idx: usize) -> Option<usize> {
        self.btd_steps.range((step_idx + 1)..).next().map(|(k, _)| *k)
    }

    /// Calculate the gap that would result if this step were removed.
    /// Gap = next - prev (the distance between neighbors).
    fn gap_if_removed(&self, step_idx: usize) -> Option<usize> {
        let prev = self.prev_btd(step_idx)?;
        let next = self.next_btd(step_idx)?;
        Some(next - prev)
    }

    /// Check if this is the first or last BTD step.
    fn is_first_or_last(&self, step_idx: usize) -> bool {
        if self.btd_steps.is_empty() {
            return false;
        }
        let first = *self.btd_steps.keys().next().unwrap();
        let last = *self.btd_steps.keys().next_back().unwrap();
        step_idx == first || step_idx == last
    }

    /// Add gap entry for a step to the heap.
    fn update_gap(&mut self, step_idx: usize) {
        if let Some(gap) = self.gap_if_removed(step_idx) {
            self.gap_heap.push(Reverse((gap, step_idx)));
        }
    }

    /// Drop the step with the smallest gap (excluding first/last).
    fn drop_smallest_gap(&mut self) {
        while let Some(Reverse((gap, step_idx))) = self.gap_heap.pop() {
            // Skip if step no longer exists
            if !self.btd_steps.contains_key(&step_idx) {
                continue;
            }

            // Never drop first or last step
            if self.is_first_or_last(step_idx) {
                continue;
            }

            // Verify gap is current (not stale)
            if self.gap_if_removed(step_idx) != Some(gap) {
                continue;
            }

            // Get prev neighbor before removing
            let prev = self.prev_btd(step_idx);

            // Drop this step
            self.btd_steps.remove(&step_idx);

            // Update gap for prev neighbor (its gap changed)
            if let Some(prev_idx) = prev {
                self.update_gap(prev_idx);
            }

            break;
        }
    }

    /// Find the nearest stored step (BTD or interval) at or before the given index.
    fn nearest_keyframe(&self, index: usize) -> Option<(usize, &Step)> {
        let btd = self.btd_steps.range(..=index).next_back();
        let interval = self.interval_steps.range(..=index).next_back();

        match (btd, interval) {
            (Some((bi, bs)), Some((ii, is))) => {
                if bi >= ii { Some((*bi, bs)) } else { Some((*ii, is)) }
            }
            (Some((bi, bs)), None) => Some((*bi, bs)),
            (None, Some((ii, is))) => Some((*ii, is)),
            (None, None) => None,
        }
    }

    /// Get all BTD step indices.
    pub fn btd_indices(&self) -> Vec<usize> {
        self.btd_steps.keys().cloned().collect()
    }
}

impl TraceStorage for BtdEvenlySpacedStorage {
    fn record(&mut self, index: usize, step: Step, error: f64) {
        self.total_steps = self.total_steps.max(index + 1);

        // Record interval keyframe if spacing is enabled
        if self.interval_spacing > 0 && index % self.interval_spacing == 0 {
            self.interval_steps.insert(index, step.clone());
        }

        // Check if this is a new minimum (BTD)
        if error < self.min_error {
            self.min_error = error;
            self.min_index = index;

            // Add to BTD steps
            self.btd_steps.insert(index, step);

            // Update gaps for this step and its neighbors
            self.update_gap(index);
            if let Some(prev) = self.prev_btd(index) {
                self.update_gap(prev);
            }

            // Evict if over capacity (excluding first/last)
            while self.btd_steps.len() > self.max_btd_keyframes {
                self.drop_smallest_gap();
            }
        }
    }

    fn get(&self, index: usize, learning_rate: f64) -> Result<Step, String> {
        if index >= self.total_steps {
            return Err(format!("Step {} not yet recorded (total: {})", index, self.total_steps));
        }

        // Check BTD steps first
        if let Some(step) = self.btd_steps.get(&index) {
            return Ok(step.clone());
        }

        // Check interval steps
        if let Some(step) = self.interval_steps.get(&index) {
            return Ok(step.clone());
        }

        // Find nearest keyframe and recompute
        let (kf_idx, kf_step) = self.nearest_keyframe(index)
            .ok_or_else(|| format!("No keyframe found for step {}", index))?;

        super::tiered::seek_from_keyframe(kf_step, kf_idx, index, learning_rate)
    }

    fn is_stored(&self, index: usize) -> bool {
        self.btd_steps.contains_key(&index) || self.interval_steps.contains_key(&index)
    }

    fn len(&self) -> usize {
        self.total_steps
    }

    fn stored_count(&self) -> usize {
        // Count unique indices (some might be in both)
        let mut all: BTreeSet<usize> = self.btd_steps.keys().cloned().collect();
        all.extend(self.interval_steps.keys().cloned());
        all.len()
    }

    fn stored_indices(&self) -> Vec<usize> {
        let mut all: BTreeSet<usize> = self.btd_steps.keys().cloned().collect();
        all.extend(self.interval_steps.keys().cloned());
        all.into_iter().collect()
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
            strategy: "btd-evenly-spaced".to_string(),
            min_index: self.min_index,
            min_error: self.min_error,
            btd_indices: Some(self.btd_indices()),
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
    /// BTD evenly-spaced: keep N BTD keyframes evenly distributed + interval keyframes.
    BtdEvenlySpaced,
}

impl Default for StorageStrategy {
    fn default() -> Self {
        Self::Tiered
    }
}

/// Configuration for BTD evenly-spaced storage.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct BtdEvenlySpacedConfig {
    /// Maximum number of BTD keyframes to retain.
    pub max_btd_keyframes: Option<usize>,
    /// Interval keyframe spacing (0 to disable).
    pub interval_spacing: Option<usize>,
}

impl Default for BtdEvenlySpacedConfig {
    fn default() -> Self {
        Self {
            max_btd_keyframes: Some(BtdEvenlySpacedStorage::DEFAULT_MAX_BTD_KEYFRAMES),
            interval_spacing: Some(BtdEvenlySpacedStorage::DEFAULT_INTERVAL_SPACING),
        }
    }
}

/// Create a storage instance based on strategy.
pub fn create_storage(strategy: StorageStrategy, bucket_size: Option<usize>) -> Box<dyn TraceStorage> {
    match strategy {
        StorageStrategy::Dense => Box::new(DenseStorage::new()),
        StorageStrategy::Btd => Box::new(BtdStorage::new()),
        StorageStrategy::Tiered => Box::new(TieredLruStorage::new(bucket_size)),
        StorageStrategy::BtdEvenlySpaced => Box::new(BtdEvenlySpacedStorage::new(None, None)),
    }
}

/// Create BTD evenly-spaced storage with custom config.
pub fn create_btd_evenly_spaced_storage(config: BtdEvenlySpacedConfig) -> Box<dyn TraceStorage> {
    Box::new(BtdEvenlySpacedStorage::new(
        config.max_btd_keyframes,
        config.interval_spacing,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_btd_evenly_spaced_defaults() {
        assert_eq!(
            BtdEvenlySpacedStorage::DEFAULT_MAX_BTD_KEYFRAMES,
            1000
        );
        assert_eq!(
            BtdEvenlySpacedStorage::DEFAULT_INTERVAL_SPACING,
            1000
        );
    }

    #[test]
    fn test_btd_evenly_spaced_config_default() {
        let config = BtdEvenlySpacedConfig::default();
        assert_eq!(config.max_btd_keyframes, Some(1000));
        assert_eq!(config.interval_spacing, Some(1000));
    }

    #[test]
    fn test_btd_evenly_spaced_strategy() {
        // Verify the strategy variant exists and serializes correctly
        let strategy = StorageStrategy::BtdEvenlySpaced;
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("btdEvenlySpaced") || json.contains("btd"));
    }

    // Helper to create a simple Step for testing
    fn make_test_step(_error_val: f64) -> Step {
        use crate::circle::Circle;
        use crate::r2::R2;
        use crate::shape::Shape;
        use crate::targets::Targets;

        let inputs: Vec<crate::InputSpec> = vec![
            (Shape::Circle(Circle { c: R2 { x: 0.0, y: 0.0 }, r: 1.0 }), vec![true, true, true]),
            (Shape::Circle(Circle { c: R2 { x: 1.5, y: 0.0 }, r: 1.0 }), vec![true, true, true]),
        ];
        // Targets chosen to produce non-zero error
        let targets: Targets<f64> = [("0-".to_string(), 2.0), ("-1".to_string(), 2.0), ("01".to_string(), 0.5)]
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>()
            .into();

        Step::new(inputs, targets).expect("Failed to create test step")
    }

    #[test]
    fn test_btd_evenly_spaced_records_btd_steps() {
        let mut storage = BtdEvenlySpacedStorage::new(Some(10), Some(0)); // No interval keyframes

        // Record steps with decreasing errors (all are BTD)
        for i in 0..5 {
            let step = make_test_step(10.0 - i as f64);
            storage.record(i, step, 10.0 - i as f64);
        }

        assert_eq!(storage.len(), 5);
        assert_eq!(storage.btd_steps.len(), 5);
        assert_eq!(storage.min_error(), 6.0); // 10.0 - 4
        assert_eq!(storage.min_index(), 4);
    }

    #[test]
    fn test_btd_evenly_spaced_only_records_improvements() {
        let mut storage = BtdEvenlySpacedStorage::new(Some(10), Some(0));

        // Record step 0 with error 10
        storage.record(0, make_test_step(10.0), 10.0);
        assert_eq!(storage.btd_steps.len(), 1);

        // Record step 1 with error 15 (worse) - should NOT be stored as BTD
        storage.record(1, make_test_step(15.0), 15.0);
        assert_eq!(storage.btd_steps.len(), 1); // Still 1

        // Record step 2 with error 5 (better) - should be stored
        storage.record(2, make_test_step(5.0), 5.0);
        assert_eq!(storage.btd_steps.len(), 2);

        assert!(storage.btd_steps.contains_key(&0));
        assert!(!storage.btd_steps.contains_key(&1));
        assert!(storage.btd_steps.contains_key(&2));
    }

    #[test]
    fn test_btd_evenly_spaced_eviction() {
        // Max 5 BTD keyframes
        let mut storage = BtdEvenlySpacedStorage::new(Some(5), Some(0));

        // Record 7 BTD steps at indices 0, 10, 20, 30, 40, 50, 60
        // Each has decreasing error so all are BTD
        for (i, idx) in [0, 10, 20, 30, 40, 50, 60].iter().enumerate() {
            let step = make_test_step(100.0 - i as f64);
            storage.record(*idx, step, 100.0 - i as f64);
        }

        // Should have evicted to 5 keyframes
        assert_eq!(storage.btd_steps.len(), 5);

        // First (0) and last (60) should always be kept
        assert!(storage.btd_steps.contains_key(&0), "First should be kept");
        assert!(storage.btd_steps.contains_key(&60), "Last should be kept");

        // The indices should be roughly evenly spaced
        let indices: Vec<usize> = storage.btd_indices();
        assert_eq!(indices.len(), 5);
    }

    #[test]
    fn test_btd_evenly_spaced_gap_calculation() {
        let mut storage = BtdEvenlySpacedStorage::new(Some(100), Some(0));

        // Manually insert BTD steps at known positions
        storage.btd_steps.insert(0, make_test_step(10.0));
        storage.btd_steps.insert(10, make_test_step(9.0));
        storage.btd_steps.insert(30, make_test_step(8.0));
        storage.btd_steps.insert(100, make_test_step(7.0));

        // Gap if removing step 10: next(30) - prev(0) = 30
        assert_eq!(storage.gap_if_removed(10), Some(30));

        // Gap if removing step 30: next(100) - prev(10) = 90
        assert_eq!(storage.gap_if_removed(30), Some(90));

        // First and last have no gap (missing neighbor)
        assert_eq!(storage.gap_if_removed(0), None);
        assert_eq!(storage.gap_if_removed(100), None);
    }

    #[test]
    fn test_btd_evenly_spaced_interval_keyframes() {
        // Interval spacing of 10
        let mut storage = BtdEvenlySpacedStorage::new(Some(100), Some(10));

        // Record 25 steps (none are BTD except first)
        for i in 0..25 {
            let error = if i == 0 { 10.0 } else { 20.0 }; // Only first is BTD
            storage.record(i, make_test_step(error), error);
        }

        // Should have interval keyframes at 0, 10, 20
        assert!(storage.interval_steps.contains_key(&0));
        assert!(storage.interval_steps.contains_key(&10));
        assert!(storage.interval_steps.contains_key(&20));
        assert!(!storage.interval_steps.contains_key(&5));
        assert!(!storage.interval_steps.contains_key(&15));

        // BTD should only have step 0
        assert_eq!(storage.btd_steps.len(), 1);
        assert!(storage.btd_steps.contains_key(&0));
    }

    #[test]
    fn test_btd_evenly_spaced_is_stored() {
        let mut storage = BtdEvenlySpacedStorage::new(Some(100), Some(10));

        storage.record(0, make_test_step(10.0), 10.0);
        storage.record(5, make_test_step(5.0), 5.0);  // BTD
        storage.record(10, make_test_step(20.0), 20.0); // Interval only

        assert!(storage.is_stored(0));  // BTD + interval
        assert!(storage.is_stored(5));  // BTD only
        assert!(storage.is_stored(10)); // Interval only
        assert!(!storage.is_stored(3)); // Not stored
    }

    #[test]
    fn test_btd_evenly_spaced_metadata() {
        let mut storage = BtdEvenlySpacedStorage::new(Some(100), Some(0));

        storage.record(0, make_test_step(10.0), 10.0);
        storage.record(5, make_test_step(5.0), 5.0);
        storage.record(10, make_test_step(3.0), 3.0);

        let meta = storage.metadata();
        assert_eq!(meta.total_steps, 11);
        assert_eq!(meta.stored_steps, 3);
        assert_eq!(meta.strategy, "btd-evenly-spaced");
        assert_eq!(meta.min_index, 10);
        assert_eq!(meta.min_error, 3.0);
        assert_eq!(meta.btd_indices, Some(vec![0, 5, 10]));
    }
}
