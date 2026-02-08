# BTD Trace Format Specification

## Overview

A compact trace storage format optimized for **Best-To-Date (BTD) steps** - the steps where error improved. This format prioritizes the "improvement curve" which is what users actually care about, while supporting configurable tiering and efficient export/import.

## Key Insights

1. **Errors are recomputable** - given shapes + targets, error can be computed at any step. We don't need to store errors.
2. **BTD steps are the interesting ones** - users care about the improvement curve, not intermediate noise.
3. **Tiering is configurable** - export can use different parameters than runtime storage.
4. **P-frames between keyframes** - intermediate steps can be recomputed via gradient descent from nearest keyframe.

## Format Design

### Core Structure

```rust
pub struct BtdTrace {
    /// Format version for compatibility
    pub version: u8,

    /// Creation timestamp (ISO 8601)
    pub created: String,

    /// Initial configuration
    pub config: TraceConfig,

    /// BTD keyframes (evenly-spaced subset of all BTD steps)
    pub btd_keyframes: Vec<BtdKeyframe>,

    /// Regular keyframes (for recomputation, not necessarily BTD)
    pub interval_keyframes: Vec<IntervalKeyframe>,

    /// Tiering configuration used for this export
    pub tiering: TieringConfig,
}

pub struct TraceConfig {
    /// Initial shape definitions (before training)
    pub inputs: Vec<InputSpec>,

    /// Target areas for each region
    pub targets: TargetsMap,

    /// Training parameters
    pub learning_rate: f64,
    pub convergence_threshold: f64,
}

pub struct BtdKeyframe {
    /// Step index where this BTD occurred
    pub step_index: usize,

    /// Shape coordinates at this step
    pub shapes: Vec<Shape>,

    /// Error at this step (redundant but useful for quick display)
    pub error: f64,
}

pub struct IntervalKeyframe {
    /// Step index
    pub step_index: usize,

    /// Shape coordinates
    pub shapes: Vec<Shape>,
}

pub struct TieringConfig {
    /// Maximum number of BTD keyframes to retain
    pub max_btd_keyframes: usize,

    /// Interval keyframe spacing (one keyframe every N steps)
    /// Set to 0 to disable interval keyframes
    pub interval_spacing: usize,

    /// Strategy used: "btd-evenly-spaced", "power-of-2", "tiered-lru"
    pub strategy: String,
}
```

### Default Configuration

```rust
impl Default for TieringConfig {
    fn default() -> Self {
        Self {
            max_btd_keyframes: 1000,
            interval_spacing: 1000,  // One interval keyframe per 1000 steps
            strategy: "btd-evenly-spaced".to_string(),
        }
    }
}
```

## BTD Evenly-Spaced Algorithm

The goal: Keep N BTD keyframes that are as evenly-spaced as possible across the step range.

### Data Structure

```rust
use std::collections::BinaryHeap;

pub struct BtdKeyframeSet {
    /// All BTD steps, ordered by step_index
    btd_steps: BTreeMap<usize, BtdKeyframe>,

    /// Min-heap keyed by gap size (smallest gap = next to drop)
    /// Entry: (gap_to_next, step_index)
    gap_heap: BinaryHeap<Reverse<(usize, usize)>>,

    /// Maximum keyframes to retain
    max_keyframes: usize,
}

impl BtdKeyframeSet {
    pub fn new(max_keyframes: usize) -> Self { ... }

    /// Add a new BTD step. If over capacity, drop the step with smallest gap.
    pub fn record(&mut self, step_index: usize, keyframe: BtdKeyframe) {
        self.btd_steps.insert(step_index, keyframe);

        // Update gaps for this step and its neighbors
        self.update_gaps(step_index);

        // If over capacity, drop step with smallest gap (excluding first/last)
        while self.btd_steps.len() > self.max_keyframes {
            self.drop_smallest_gap();
        }
    }

    /// Find step with smallest gap to neighbors and remove it
    fn drop_smallest_gap(&mut self) {
        while let Some(Reverse((gap, step_idx))) = self.gap_heap.pop() {
            // Skip if step no longer exists or gap is stale
            if !self.btd_steps.contains_key(&step_idx) {
                continue;
            }

            // Never drop first or last step
            if self.is_first_or_last(step_idx) {
                continue;
            }

            // Verify gap is current (not stale)
            if self.current_gap(step_idx) != Some(gap) {
                continue;
            }

            // Drop this step
            self.btd_steps.remove(&step_idx);

            // Update neighbor gaps
            if let Some(prev) = self.prev_step(step_idx) {
                self.update_gaps(prev);
            }
            break;
        }
    }

    fn current_gap(&self, step_idx: usize) -> Option<usize> {
        let prev = self.prev_step(step_idx)?;
        let next = self.next_step(step_idx)?;
        Some(next - prev)  // Gap if this step were removed
    }

    fn update_gaps(&mut self, step_idx: usize) {
        if let Some(gap) = self.current_gap(step_idx) {
            self.gap_heap.push(Reverse((gap, step_idx)));
        }
    }
}
```

### Complexity

- **Insert**: O(log N) for heap operations
- **Drop**: O(log N) amortized (stale entries cleaned lazily)
- **Space**: O(N) for N keyframes + O(N) for heap (with stale entries)

## Recomputation (P-Frames)

To reconstruct a non-keyframe step:

```rust
impl BtdTrace {
    /// Reconstruct shapes at any step by running gradient descent from nearest keyframe
    pub fn reconstruct_step(&self, target_step: usize) -> Option<Vec<Shape>> {
        // Find nearest keyframe at or before target_step
        let keyframe = self.nearest_keyframe_before(target_step)?;

        if keyframe.step_index == target_step {
            return Some(keyframe.shapes.clone());
        }

        // Run gradient descent from keyframe to target
        let mut shapes = keyframe.shapes.clone();
        for _ in keyframe.step_index..target_step {
            shapes = gradient_step(&shapes, &self.config.targets, self.config.learning_rate);
        }

        Some(shapes)
    }

    fn nearest_keyframe_before(&self, step: usize) -> Option<&dyn Keyframe> {
        // Check BTD keyframes first (likely to be closer to interesting steps)
        let btd = self.btd_keyframes.iter()
            .filter(|k| k.step_index <= step)
            .max_by_key(|k| k.step_index);

        // Check interval keyframes
        let interval = self.interval_keyframes.iter()
            .filter(|k| k.step_index <= step)
            .max_by_key(|k| k.step_index);

        // Return whichever is closest
        match (btd, interval) {
            (Some(b), Some(i)) => {
                if b.step_index >= i.step_index { Some(b) } else { Some(i) }
            }
            (Some(b), None) => Some(b),
            (None, Some(i)) => Some(i),
            (None, None) => None,
        }
    }
}
```

### Worst-Case Recomputation

With `interval_spacing = 1000`, worst case is 999 gradient steps to reconstruct any frame.

With BTD keyframes providing additional coverage (especially in early training where BTDs are dense), typical recomputation is much lower.

## Tiering Conversion

### Downsampling

Convert a trace to fewer keyframes:

```rust
impl BtdTrace {
    /// Reduce to target number of BTD keyframes while maintaining even spacing
    pub fn downsample_btd(&mut self, target_count: usize) {
        if self.btd_keyframes.len() <= target_count {
            return;
        }

        let mut set = BtdKeyframeSet::new(target_count);
        for kf in self.btd_keyframes.drain(..) {
            set.record(kf.step_index, kf);
        }
        self.btd_keyframes = set.into_vec();
    }

    /// Reduce interval keyframe frequency
    pub fn downsample_intervals(&mut self, new_spacing: usize) {
        self.interval_keyframes.retain(|kf| kf.step_index % new_spacing == 0);
        self.tiering.interval_spacing = new_spacing;
    }
}
```

### Upsampling

Add more keyframes by materializing from existing ones:

```rust
impl BtdTrace {
    /// Add interval keyframes at finer spacing
    pub fn upsample_intervals(&mut self, new_spacing: usize) -> Result<(), Error> {
        if new_spacing >= self.tiering.interval_spacing {
            return Err(Error::InvalidSpacing);
        }

        let total_steps = self.total_steps();
        let mut new_keyframes = Vec::new();

        for step in (0..total_steps).step_by(new_spacing) {
            if !self.has_keyframe_at(step) {
                let shapes = self.reconstruct_step(step)
                    .ok_or(Error::CannotReconstruct(step))?;
                new_keyframes.push(IntervalKeyframe { step_index: step, shapes });
            }
        }

        self.interval_keyframes.extend(new_keyframes);
        self.interval_keyframes.sort_by_key(|k| k.step_index);
        self.tiering.interval_spacing = new_spacing;
        Ok(())
    }
}
```

### Round-Trip Considerations

Due to floating-point precision:
- Downsampling then upsampling may not produce identical results
- Recomputed steps accumulate small errors from gradient descent
- For verification, compare within tolerance (e.g., 1e-10)

## Serialization

### JSON Format

```json
{
  "version": 1,
  "created": "2026-02-01T14:30:52Z",
  "config": {
    "inputs": [...],
    "targets": {...},
    "learning_rate": 0.1,
    "convergence_threshold": 1e-10
  },
  "btd_keyframes": [
    { "step_index": 0, "shapes": [...], "error": 0.5 },
    { "step_index": 15, "shapes": [...], "error": 0.3 },
    ...
  ],
  "interval_keyframes": [
    { "step_index": 1000, "shapes": [...] },
    { "step_index": 2000, "shapes": [...] },
    ...
  ],
  "tiering": {
    "max_btd_keyframes": 1000,
    "interval_spacing": 1000,
    "strategy": "btd-evenly-spaced"
  }
}
```

### Binary Format (Future)

For even smaller exports, consider:
- MessagePack or CBOR for structure
- Float16 for shape coordinates (sufficient precision for display)
- Delta encoding for sequential keyframes
- Gzip compression on top

## Offline WASM Testing

### Approach

Use `wasm-pack test --node` to run the exact same WASM code outside the browser:

```rust
// tests/trace_roundtrip.rs

#[wasm_bindgen_test]
fn test_btd_trace_roundtrip() {
    // Create deterministic training scenario
    let config = TraceConfig {
        inputs: vec![/* fixed circles */],
        targets: /* fixed targets */,
        learning_rate: 0.1,
        convergence_threshold: 1e-10,
    };

    // Run training
    let trace = train_to_trace(&config, 10000);

    // Export at different tiering levels
    let compact = trace.clone().with_tiering(TieringConfig {
        max_btd_keyframes: 100,
        interval_spacing: 500,
        ..Default::default()
    });

    // Verify reconstruction accuracy
    for step in [0, 500, 1234, 5000, 9999] {
        let original = trace.reconstruct_step(step).unwrap();
        let reconstructed = compact.reconstruct_step(step).unwrap();

        for (orig, recon) in original.iter().zip(reconstructed.iter()) {
            assert!((orig.x() - recon.x()).abs() < 1e-10);
            assert!((orig.y() - recon.y()).abs() < 1e-10);
        }
    }
}

#[wasm_bindgen_test]
fn test_btd_evenly_spaced() {
    let mut set = BtdKeyframeSet::new(10);

    // Add 100 BTD steps
    for i in [0, 5, 12, 25, 30, 45, 60, 70, 80, 85, 90, 95, 100] {
        set.record(i, make_keyframe(i));
    }

    // Should have at most 10, evenly distributed
    let remaining: Vec<_> = set.step_indices().collect();
    assert!(remaining.len() <= 10);
    assert!(remaining.contains(&0));   // First always kept
    assert!(remaining.contains(&100)); // Last always kept

    // Verify roughly even spacing
    let gaps: Vec<_> = remaining.windows(2).map(|w| w[1] - w[0]).collect();
    let mean_gap = gaps.iter().sum::<usize>() / gaps.len();
    for gap in gaps {
        assert!(gap >= mean_gap / 2, "Gap too small: {}", gap);
    }
}
```

### CLI Tool

Add a CLI for offline trace manipulation:

```bash
# Convert tiering
apvd-trace convert input.json -o output.json \
  --max-btd 500 --interval 2000

# Benchmark recomputation
apvd-trace benchmark input.json --samples 1000

# Verify round-trip
apvd-trace verify input.json --tolerance 1e-10

# Show stats
apvd-trace info input.json
# Output:
#   Total steps: 10000
#   BTD keyframes: 156
#   Interval keyframes: 10
#   Estimated size: 45KB (gzipped)
#   Max recomputation distance: 423 steps
```

## Frontend Integration

### Export Options UI

```typescript
interface TraceExportOptions {
  maxBtdKeyframes: number;      // Default: 1000
  intervalSpacing: number;       // Default: 1000, 0 to disable
  includeErrors: boolean;        // Default: false (recomputable)
  compress: boolean;             // Default: true (.gz)
}
```

### Settings Panel Addition

Add to trace filename template area:
- Max BTD keyframes slider (10-10000)
- Interval spacing dropdown (100, 500, 1000, 2000, 5000, disabled)
- Checkbox: "Include error array" (for legacy compatibility)

### Import Handling

```typescript
async function importBtdTrace(trace: BtdTrace): Promise<void> {
  // Reconstruct full error curve for display
  const errors: number[] = [];
  for (let i = 0; i <= trace.totalSteps; i++) {
    const shapes = trace.reconstructStep(i);
    const error = computeError(shapes, trace.config.targets);
    errors.push(error);
  }

  // Or: lazy reconstruction, only compute visible range
}
```

## Migration Path

### Version 1 (Current)
- Power-of-2 keyframes
- Dense error array
- No BTD awareness

### Version 2 (This Spec)
- BTD-aware keyframes with evenly-spaced retention
- Interval keyframes for bounded recomputation
- Optional error array (for legacy compatibility)
- Tiering configuration metadata

### Compatibility

Version 2 readers should handle version 1:
- Treat all keyframes as interval keyframes
- Use error array directly (no recomputation needed)
- No BTD information available

## Size Estimates

For a 10,000 step trace with 3 ellipses (5 vars each = 15 vars total):

| Component | v1 (current) | v2 (BTD) |
|-----------|--------------|----------|
| Errors array | 80KB (10k × 8 bytes) | 0 (recomputable) |
| Keyframes | 2KB (14 × 150 bytes) | 15KB (100 BTD × 150) |
| Interval KF | 0 | 1.5KB (10 × 150) |
| Config/meta | 1KB | 1KB |
| **Total** | ~83KB | ~17KB |
| **Gzipped** | ~35KB | ~8KB |

With `max_btd_keyframes: 1000` and `interval_spacing: 1000`:
- ~150KB uncompressed, ~40KB gzipped
- Still 2x smaller than v1 due to no error array

## Simplifications

**Learning rate and shape changes reset the trace.** A trace assumes constant:
- Learning rate
- Shape types and count
- Polygon vertex counts

Changes to any of these start a new trace. Compound/segmented traces (multiple segments with different parameters) are a future consideration.

## Open Questions

1. **Should interval keyframes also track BTD status?**
   - Pro: Avoids duplicate storage if interval keyframe is also BTD
   - Con: More complex merging logic

2. **Error precision in BTD keyframes?**
   - Full f64 precision or reduced (6 sig figs)?
   - Error is recomputable, so this is purely for display convenience
