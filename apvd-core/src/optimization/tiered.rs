//! Tiered keyframe storage for training traces.
//!
//! Stores progressively sparser keyframes as training progresses, achieving
//! O(log N) storage for N steps while maintaining O(resolution) seek time.
//!
//! See SPEC-TRACE-STORAGE.md for the full algorithm.

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use super::step::Step;

/// Tiered keyframe configuration.
///
/// Tier structure with bucket size B:
/// - Tier 0: 2B samples at resolution 1 → covers [0, 2B)
/// - Tier 1: B samples at resolution 2  → covers [2B, 4B)
/// - Tier 2: B samples at resolution 4  → covers [4B, 8B)
/// - Tier n: B samples at resolution 2^n → covers [B·2^n, B·2^(n+1))
///
/// With B=1024, 100k steps → ~7k keyframes (14:1 compression).
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct TieredConfig {
    /// Bucket size (B): Tier 0 has 2B samples, other tiers have B samples
    pub bucket_size: usize,
}

impl Default for TieredConfig {
    fn default() -> Self {
        Self::new(None)
    }
}

impl TieredConfig {
    /// Default bucket size (power of 2, ~14:1 compression at 100k steps)
    pub const DEFAULT_BUCKET_SIZE: usize = 1024;

    /// Create a new tiered config with optional bucket size.
    pub fn new(bucket_size: Option<usize>) -> Self {
        Self {
            bucket_size: bucket_size.unwrap_or(Self::DEFAULT_BUCKET_SIZE),
        }
    }

    /// Which tier contains this step.
    pub fn tier(&self, step: usize) -> usize {
        let b = self.bucket_size;
        if step < 2 * b {
            0
        } else {
            (step / b).ilog2() as usize
        }
    }

    /// Resolution (decimation factor) for this tier.
    /// Tier 0: 1, Tier 1: 2, Tier 2: 4, Tier n: 2^n
    pub fn resolution(&self, tier: usize) -> usize {
        1 << tier
    }

    /// Check if this step should be stored as a keyframe.
    pub fn is_keyframe(&self, step: usize) -> bool {
        let tier = self.tier(step);
        let res = self.resolution(tier);
        step % res == 0
    }

    /// First step index of this tier.
    pub fn tier_start(&self, tier: usize) -> usize {
        if tier == 0 {
            0
        } else {
            self.bucket_size << tier
        }
    }

    /// Find the nearest keyframe at or before this step.
    pub fn nearest_keyframe(&self, step: usize) -> usize {
        let tier = self.tier(step);
        let res = self.resolution(tier);
        (step / res) * res
    }

    /// Maximum recomputation steps needed to reach any step in a tier.
    pub fn max_recompute(&self, tier: usize) -> usize {
        if tier == 0 {
            0
        } else {
            self.resolution(tier) - 1
        }
    }

    /// Calculate total keyframes needed for N steps.
    pub fn keyframe_count(&self, total_steps: usize) -> usize {
        if total_steps == 0 {
            return 0;
        }
        (0..total_steps).filter(|&s| self.is_keyframe(s)).count()
    }
}

/// Seek to a target step by recomputing from a keyframe.
///
/// Given a keyframe step and target index, runs forward steps to reach target.
/// Returns the step at `target_idx` or an error if recomputation fails.
pub fn seek_from_keyframe(
    keyframe: &Step,
    keyframe_idx: usize,
    target_idx: usize,
    learning_rate: f64,
) -> Result<Step, String> {
    if target_idx < keyframe_idx {
        return Err(format!(
            "Target {} is before keyframe {}",
            target_idx, keyframe_idx
        ));
    }

    let steps_needed = target_idx - keyframe_idx;
    if steps_needed == 0 {
        return Ok(keyframe.clone());
    }

    let mut current = keyframe.clone();
    for i in 0..steps_needed {
        current = current.step(learning_rate).map_err(|e| {
            format!(
                "Recompute failed at step {} (offset {}): {}",
                keyframe_idx + i + 1,
                i + 1,
                e
            )
        })?;
    }

    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_assignment() {
        let config = TieredConfig::new(Some(100)); // B=100

        // Tier 0: [0, 200)
        assert_eq!(config.tier(0), 0);
        assert_eq!(config.tier(50), 0);
        assert_eq!(config.tier(199), 0);

        // Tier 1: [200, 400)
        assert_eq!(config.tier(200), 1);
        assert_eq!(config.tier(399), 1);

        // Tier 2: [400, 800)
        assert_eq!(config.tier(400), 2);
        assert_eq!(config.tier(799), 2);

        // Tier 3: [800, 1600)
        assert_eq!(config.tier(800), 3);
    }

    #[test]
    fn test_resolution() {
        let config = TieredConfig::default();

        assert_eq!(config.resolution(0), 1);
        assert_eq!(config.resolution(1), 2);
        assert_eq!(config.resolution(2), 4);
        assert_eq!(config.resolution(3), 8);
        assert_eq!(config.resolution(4), 16);
    }

    #[test]
    fn test_is_keyframe() {
        let config = TieredConfig::new(Some(100));

        // Tier 0: every step is a keyframe
        assert!(config.is_keyframe(0));
        assert!(config.is_keyframe(1));
        assert!(config.is_keyframe(199));

        // Tier 1 (resolution 2): even steps only
        assert!(config.is_keyframe(200));
        assert!(!config.is_keyframe(201));
        assert!(config.is_keyframe(202));

        // Tier 2 (resolution 4): every 4th step
        assert!(config.is_keyframe(400));
        assert!(!config.is_keyframe(401));
        assert!(!config.is_keyframe(402));
        assert!(!config.is_keyframe(403));
        assert!(config.is_keyframe(404));
    }

    #[test]
    fn test_nearest_keyframe() {
        let config = TieredConfig::new(Some(100));

        // Tier 0: every step is keyframe
        assert_eq!(config.nearest_keyframe(0), 0);
        assert_eq!(config.nearest_keyframe(50), 50);

        // Tier 1 (resolution 2)
        assert_eq!(config.nearest_keyframe(200), 200);
        assert_eq!(config.nearest_keyframe(201), 200);
        assert_eq!(config.nearest_keyframe(202), 202);
        assert_eq!(config.nearest_keyframe(203), 202);

        // Tier 2 (resolution 4)
        assert_eq!(config.nearest_keyframe(400), 400);
        assert_eq!(config.nearest_keyframe(401), 400);
        assert_eq!(config.nearest_keyframe(403), 400);
        assert_eq!(config.nearest_keyframe(404), 404);
    }

    #[test]
    fn test_tier_start() {
        let config = TieredConfig::new(Some(100));

        assert_eq!(config.tier_start(0), 0);
        assert_eq!(config.tier_start(1), 200);  // 100 << 1
        assert_eq!(config.tier_start(2), 400);  // 100 << 2
        assert_eq!(config.tier_start(3), 800);  // 100 << 3
    }

    #[test]
    fn test_keyframe_count() {
        let config = TieredConfig::new(Some(100));

        // Tier 0 only (steps 0-199): 200 keyframes
        assert_eq!(config.keyframe_count(200), 200);

        // Include tier 1 (steps 200-399): 200 + 100 = 300
        assert_eq!(config.keyframe_count(400), 300);

        // Include tier 2 (steps 400-799): 300 + 100 = 400
        assert_eq!(config.keyframe_count(800), 400);
    }

    #[test]
    fn test_compression_ratio() {
        let config = TieredConfig::new(Some(1024));

        // 100k steps should compress to ~7k keyframes
        let count = config.keyframe_count(100_000);
        let ratio = 100_000.0 / count as f64;

        // Should be roughly 14:1 compression
        assert!(ratio > 10.0, "Compression ratio {} too low", ratio);
        assert!(ratio < 20.0, "Compression ratio {} too high", ratio);
    }
}
