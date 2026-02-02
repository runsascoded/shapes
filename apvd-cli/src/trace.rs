//! Trace file operations: info, verify, benchmark, reconstruct, convert, diff.
//!
//! Supports two trace formats:
//! - Current format: Output from `apvd train` with inputs, targets, best, traces
//! - V2 format: BTD evenly-spaced keyframes for efficient random access

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use apvd_core::{InputSpec, Step, Targets, TargetsMap};

// ============================================================================
// Current train output format
// ============================================================================

/// Training result from `apvd train`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainResult {
    pub inputs: Vec<InputSpec>,
    pub targets: TargetsMap<f64>,
    pub best: VariantResult,
    pub traces: Vec<VariantResult>,
    pub total_time_ms: u64,
}

/// Result for a single training variant (permutation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantResult {
    pub variant_id: usize,
    pub permutation: Vec<usize>,
    pub final_error: f64,
    pub min_error: f64,
    pub min_step: usize,
    pub total_steps: usize,
    pub training_time_ms: u64,
    pub final_shapes: Vec<Value>,
    /// Step history (if -H flag used)
    #[serde(default)]
    pub history: Option<Vec<HistoryStep>>,
    /// Sparse checkpoints (if -C flag used)
    #[serde(default)]
    pub checkpoints: Option<Vec<Checkpoint>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStep {
    pub step: usize,
    pub error: f64,
    pub shapes: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub step: usize,
    pub error: f64,
    pub shapes: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

// ============================================================================
// V2 trace format (BTD evenly-spaced)
// ============================================================================

/// V2 trace file format with BTD evenly-spaced keyframes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceFileV2 {
    pub version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    pub config: TraceConfig,
    #[serde(default)]
    pub btd_keyframes: Vec<Keyframe>,
    #[serde(default)]
    pub interval_keyframes: Vec<Keyframe>,
    pub total_steps: usize,
    pub min_error: f64,
    pub min_step: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiering: Option<TieringConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceConfig {
    pub inputs: Vec<InputSpec>,
    pub targets: TargetsMap<f64>,
    #[serde(default = "default_learning_rate")]
    pub learning_rate: f64,
    #[serde(default = "default_threshold")]
    pub convergence_threshold: f64,
}

fn default_learning_rate() -> f64 {
    0.1
}

fn default_threshold() -> f64 {
    1e-10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Keyframe {
    pub step_index: usize,
    pub shapes: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TieringConfig {
    #[serde(default = "default_max_btd")]
    pub max_btd_keyframes: usize,
    #[serde(default = "default_interval")]
    pub interval_spacing: usize,
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

fn default_max_btd() -> usize {
    1000
}

fn default_interval() -> usize {
    1000
}

fn default_strategy() -> String {
    "btd-evenly-spaced".to_string()
}

impl Default for TieringConfig {
    fn default() -> Self {
        Self {
            max_btd_keyframes: default_max_btd(),
            interval_spacing: default_interval(),
            strategy: default_strategy(),
        }
    }
}

// ============================================================================
// Unified trace representation
// ============================================================================

/// Unified trace representation that can be loaded from either format.
pub enum TraceData {
    /// Current train output format
    Train(TrainResult),
    /// V2 format with BTD keyframes
    V2(TraceFileV2),
}

impl TraceData {
    pub fn inputs(&self) -> &Vec<InputSpec> {
        match self {
            TraceData::Train(t) => &t.inputs,
            TraceData::V2(t) => &t.config.inputs,
        }
    }

    pub fn targets(&self) -> &TargetsMap<f64> {
        match self {
            TraceData::Train(t) => &t.targets,
            TraceData::V2(t) => &t.config.targets,
        }
    }

    pub fn total_steps(&self) -> usize {
        match self {
            TraceData::Train(t) => t.best.total_steps,
            TraceData::V2(t) => t.total_steps,
        }
    }

    pub fn min_error(&self) -> f64 {
        match self {
            TraceData::Train(t) => t.best.min_error,
            TraceData::V2(t) => t.min_error,
        }
    }

    pub fn min_step(&self) -> usize {
        match self {
            TraceData::Train(t) => t.best.min_step,
            TraceData::V2(t) => t.min_step,
        }
    }

    pub fn learning_rate(&self) -> f64 {
        match self {
            TraceData::Train(_) => 0.05, // Default for train command
            TraceData::V2(t) => t.config.learning_rate,
        }
    }

    pub fn format_name(&self) -> &'static str {
        match self {
            TraceData::Train(_) => "train-output",
            TraceData::V2(_) => "v2-btd",
        }
    }

    /// Get all keyframes from the trace.
    pub fn keyframes(&self) -> Vec<Keyframe> {
        match self {
            TraceData::Train(t) => {
                let mut kfs = Vec::new();

                // Use checkpoints if available
                if let Some(ref checkpoints) = t.best.checkpoints {
                    for cp in checkpoints {
                        kfs.push(Keyframe {
                            step_index: cp.step,
                            shapes: cp.shapes.clone(),
                            error: Some(cp.error),
                        });
                    }
                }

                // Use history if available
                if let Some(ref history) = t.best.history {
                    for hs in history {
                        kfs.push(Keyframe {
                            step_index: hs.step,
                            shapes: hs.shapes.clone(),
                            error: Some(hs.error),
                        });
                    }
                }

                // Always include final shapes as last keyframe
                if kfs.is_empty() || kfs.last().map(|k| k.step_index) != Some(t.best.total_steps.saturating_sub(1)) {
                    kfs.push(Keyframe {
                        step_index: t.best.total_steps.saturating_sub(1),
                        shapes: t.best.final_shapes.clone(),
                        error: Some(t.best.final_error),
                    });
                }

                kfs.sort_by_key(|k| k.step_index);
                kfs.dedup_by_key(|k| k.step_index);
                kfs
            }
            TraceData::V2(t) => {
                let mut kfs: Vec<Keyframe> = Vec::new();
                kfs.extend(t.btd_keyframes.iter().cloned());
                kfs.extend(t.interval_keyframes.iter().cloned());
                kfs.sort_by_key(|k| k.step_index);
                kfs.dedup_by_key(|k| k.step_index);
                kfs
            }
        }
    }

    /// Number of BTD keyframes (0 for train format without history).
    pub fn btd_keyframe_count(&self) -> usize {
        match self {
            TraceData::Train(_) => 0,
            TraceData::V2(t) => t.btd_keyframes.len(),
        }
    }

    /// Number of interval keyframes.
    pub fn interval_keyframe_count(&self) -> usize {
        match self {
            TraceData::Train(t) => t.best.checkpoints.as_ref().map_or(0, |c| c.len()),
            TraceData::V2(t) => t.interval_keyframes.len(),
        }
    }

    pub fn tiering(&self) -> Option<TieringConfig> {
        match self {
            TraceData::Train(_) => None,
            TraceData::V2(t) => t.tiering.clone(),
        }
    }
}

/// Load a trace file (supports .json and .json.gz, auto-detects format).
pub fn load_trace(path: &str) -> Result<TraceData, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read as generic JSON first to detect format
    let json: Value = if path.extension().map_or(false, |e| e == "gz") {
        let decoder = GzDecoder::new(reader);
        serde_json::from_reader(decoder)?
    } else {
        serde_json::from_reader(reader)?
    };

    // Detect format by checking for characteristic fields
    if json.get("version").is_some() && json.get("config").is_some() {
        // V2 format
        let trace: TraceFileV2 = serde_json::from_value(json)?;
        Ok(TraceData::V2(trace))
    } else if json.get("inputs").is_some() && json.get("traces").is_some() {
        // Train output format
        let train: TrainResult = serde_json::from_value(json)?;
        Ok(TraceData::Train(train))
    } else {
        Err("Unknown trace file format".into())
    }
}

/// Find the nearest keyframe at or before the given step.
pub fn nearest_keyframe_before(trace: &TraceData, step: usize) -> Option<Keyframe> {
    trace
        .keyframes()
        .into_iter()
        .filter(|k| k.step_index <= step)
        .max_by_key(|k| k.step_index)
}

/// Reconstruct shapes at a specific step by recomputing from nearest keyframe.
pub fn reconstruct_step(
    trace: &TraceData,
    target_step: usize,
) -> Result<(Step, usize), Box<dyn std::error::Error>> {
    let keyframe = nearest_keyframe_before(trace, target_step)
        .ok_or_else(|| format!("No keyframe found at or before step {}", target_step))?;

    let kf_step = keyframe.step_index;

    // Parse shapes from keyframe
    let shapes: Vec<apvd_core::Shape<f64>> = keyframe
        .shapes
        .iter()
        .map(|v| serde_json::from_value(v.clone()))
        .collect::<Result<Vec<_>, _>>()?;

    // Convert to InputSpec (assume all coordinates trainable)
    let inputs: Vec<InputSpec> = shapes
        .iter()
        .map(|s| {
            let n = match s {
                apvd_core::Shape::Circle(_) => 3,
                apvd_core::Shape::XYRR(_) => 4,
                apvd_core::Shape::XYRRT(_) => 5,
                apvd_core::Shape::Polygon(p) => p.vertices.len() * 2,
            };
            (s.clone(), vec![true; n])
        })
        .collect();

    // Create initial step from keyframe
    let targets: Targets<f64> = trace.targets().clone().into();
    let mut current = Step::new(inputs, targets)?;

    // Recompute forward to target step
    let lr = trace.learning_rate();
    for _ in kf_step..target_step {
        current = current.step(lr)?;
    }

    Ok((current, kf_step))
}

// ============================================================================
// Statistics and verification
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceStats {
    pub format: String,
    pub total_steps: usize,
    pub min_error: f64,
    pub min_step: usize,

    pub num_shapes: usize,
    pub shape_types: Vec<String>,
    pub total_variables: usize,

    pub btd_keyframe_count: usize,
    pub interval_keyframe_count: usize,
    pub total_keyframes: usize,

    pub tiering: Option<TieringConfig>,

    pub max_recompute_distance: usize,
    pub avg_recompute_distance: f64,
}

/// Compute statistics for a trace.
pub fn compute_stats(trace: &TraceData) -> TraceStats {
    let keyframes = trace.keyframes();
    let keyframe_indices: Vec<usize> = keyframes.iter().map(|k| k.step_index).collect();

    // Compute recomputation distances
    let total_steps = trace.total_steps();
    let (max_dist, avg_dist) = if keyframe_indices.len() > 1 {
        let mut gaps: Vec<usize> = keyframe_indices
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect();

        // Also consider gap from last keyframe to total_steps
        if let Some(&last) = keyframe_indices.last() {
            if last < total_steps {
                gaps.push(total_steps - last);
            }
        }

        let max = gaps.iter().copied().max().unwrap_or(0);
        let avg = if gaps.is_empty() {
            0.0
        } else {
            gaps.iter().sum::<usize>() as f64 / gaps.len() as f64
        };
        (max, avg)
    } else if keyframes.is_empty() {
        (total_steps, total_steps as f64)
    } else {
        // Single keyframe
        let gap = total_steps.saturating_sub(keyframe_indices[0]);
        (gap, gap as f64)
    };

    // Shape info
    let inputs = trace.inputs();
    let num_shapes = inputs.len();
    let shape_types: Vec<String> = inputs
        .iter()
        .map(|(s, _)| match s {
            apvd_core::Shape::Circle(_) => "Circle".to_string(),
            apvd_core::Shape::XYRR(_) => "XYRR".to_string(),
            apvd_core::Shape::XYRRT(_) => "XYRRT".to_string(),
            apvd_core::Shape::Polygon(p) => format!("Polygon({})", p.vertices.len()),
        })
        .collect();

    let total_variables: usize = inputs.iter().map(|(_, t)| t.len()).sum();

    TraceStats {
        format: trace.format_name().to_string(),
        total_steps,
        min_error: trace.min_error(),
        min_step: trace.min_step(),

        num_shapes,
        shape_types,
        total_variables,

        btd_keyframe_count: trace.btd_keyframe_count(),
        interval_keyframe_count: trace.interval_keyframe_count(),
        total_keyframes: keyframes.len(),

        tiering: trace.tiering(),

        max_recompute_distance: max_dist,
        avg_recompute_distance: avg_dist,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub samples_verified: usize,
    pub max_reconstruction_error: f64,
}

/// Verify trace integrity.
pub fn verify_trace(
    trace: &TraceData,
    tolerance: f64,
    samples: usize,
    exhaustive: bool,
    quick: bool,
) -> VerifyResult {
    let mut result = VerifyResult {
        valid: true,
        errors: Vec::new(),
        warnings: Vec::new(),
        samples_verified: 0,
        max_reconstruction_error: 0.0,
    };

    let total_steps = trace.total_steps();

    // Schema checks
    if total_steps == 0 {
        result.errors.push("total_steps is 0".to_string());
        result.valid = false;
    }

    let keyframes = trace.keyframes();
    if keyframes.is_empty() {
        result.warnings.push("No keyframes found (only final shapes available)".to_string());
    }

    // Check keyframes are sorted
    let mut sorted = true;
    for w in keyframes.windows(2) {
        if w[0].step_index >= w[1].step_index {
            sorted = false;
            break;
        }
    }
    if !sorted && keyframes.len() > 1 {
        result.errors.push("Keyframes not sorted by step_index".to_string());
        result.valid = false;
    }

    // Check step 0 present for V2 format
    if let TraceData::V2(_) = trace {
        if !keyframes.iter().any(|k| k.step_index == 0) {
            result.warnings.push("Step 0 not present in keyframes".to_string());
        }
    }

    // BTD should be monotonically decreasing in error (V2 only)
    if let TraceData::V2(t) = trace {
        let btd_errors: Vec<f64> = t.btd_keyframes.iter().filter_map(|k| k.error).collect();
        for w in btd_errors.windows(2) {
            if w[1] > w[0] {
                result.warnings.push("BTD errors not monotonically decreasing".to_string());
                break;
            }
        }
    }

    if quick || !result.valid || total_steps == 0 {
        return result;
    }

    // Reconstruction verification (only if we have keyframes to reconstruct from)
    if keyframes.is_empty() {
        result.warnings.push("Cannot verify reconstruction: no keyframes available".to_string());
        return result;
    }

    // For train format, shapes are stored in Dual format which we can't easily reconstruct from
    if let TraceData::Train(_) = trace {
        result.warnings.push("Reconstruction verification skipped: train format stores shapes in Dual format".to_string());
        return result;
    }

    let steps_to_verify: Vec<usize> = if exhaustive {
        (0..total_steps).collect()
    } else {
        use std::collections::HashSet;
        let mut steps: HashSet<usize> = HashSet::new();

        // Always verify first keyframe and min step
        if let Some(first) = keyframes.first() {
            steps.insert(first.step_index);
        }
        steps.insert(trace.min_step());
        if total_steps > 0 {
            steps.insert(total_steps - 1);
        }

        // Random samples
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let max_samples = samples.min(total_steps);
        while steps.len() < max_samples {
            steps.insert(rng.gen_range(0..total_steps));
        }

        let mut v: Vec<usize> = steps.into_iter().collect();
        v.sort();
        v
    };

    for step in &steps_to_verify {
        match reconstruct_step(trace, *step) {
            Ok((reconstructed, _kf_step)) => {
                let error = reconstructed.error.v();

                // Compare against keyframe error if available
                if let Some(kf) = keyframes.iter().find(|k| k.step_index == *step) {
                    if let Some(expected) = kf.error {
                        let diff = (error - expected).abs();
                        if diff > result.max_reconstruction_error {
                            result.max_reconstruction_error = diff;
                        }
                        if diff > tolerance {
                            result.errors.push(format!(
                                "Step {}: reconstruction error {} differs from stored {} by {}",
                                step, error, expected, diff
                            ));
                            result.valid = false;
                        }
                    }
                }

                result.samples_verified += 1;
            }
            Err(e) => {
                result.errors.push(format!("Step {}: reconstruction failed: {}", step, e));
                result.valid = false;
            }
        }
    }

    result
}

// ============================================================================
// Benchmarking
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkResult {
    pub random_access: AccessStats,
    pub sequential: Option<SequentialStats>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessStats {
    pub samples: usize,
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub keyframe_hits: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequentialStats {
    pub total_steps: usize,
    pub total_ms: f64,
    pub per_step_ms: f64,
}

/// Benchmark recomputation performance.
pub fn benchmark_trace(
    trace: &TraceData,
    samples: usize,
    include_sequential: bool,
) -> BenchmarkResult {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let keyframe_indices: std::collections::HashSet<usize> =
        trace.keyframes().iter().map(|k| k.step_index).collect();

    let total_steps = trace.total_steps();

    // Random access benchmark
    let actual_samples = samples.min(total_steps.max(1));
    let mut times: Vec<f64> = Vec::with_capacity(actual_samples);
    let mut keyframe_hits = 0;

    for _ in 0..actual_samples {
        let step = if total_steps > 0 {
            rng.gen_range(0..total_steps)
        } else {
            0
        };

        let start = Instant::now();
        let _ = reconstruct_step(trace, step);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        times.push(elapsed);
        if keyframe_indices.contains(&step) {
            keyframe_hits += 1;
        }
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let random_access = AccessStats {
        samples: actual_samples,
        min_ms: times.first().copied().unwrap_or(0.0),
        max_ms: times.last().copied().unwrap_or(0.0),
        avg_ms: if times.is_empty() {
            0.0
        } else {
            times.iter().sum::<f64>() / times.len() as f64
        },
        p50_ms: times.get(times.len() / 2).copied().unwrap_or(0.0),
        p95_ms: times.get(times.len() * 95 / 100).copied().unwrap_or(0.0),
        keyframe_hits,
    };

    // Sequential scan benchmark
    let sequential = if include_sequential && total_steps > 0 {
        let start = Instant::now();
        for step in 0..total_steps {
            let _ = reconstruct_step(trace, step);
        }
        let total_ms = start.elapsed().as_secs_f64() * 1000.0;

        Some(SequentialStats {
            total_steps,
            total_ms,
            per_step_ms: total_ms / total_steps as f64,
        })
    } else {
        None
    };

    BenchmarkResult {
        random_access,
        sequential,
    }
}
