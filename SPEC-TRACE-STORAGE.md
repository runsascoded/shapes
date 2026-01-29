# Trace Storage Spec

## Problem

Training histories can grow arbitrarily long (10k+ steps). Storing every step:
- Wastes disk/memory for long runs
- Makes git unwieldy
- But we need random seek for time-travel UI

## Requirements

1. **Random seek**: Jump to any step with bounded recomputation
2. **Deterministic**: Recomputation produces identical results
3. **Bounded storage**: O(log N) or O(√N) for N steps
4. **Git-friendly**: Small files for test fixtures
5. **Large data on S3**: Full histories via dvx, not git

## Proposed: Tiered/Tower Sampling (RRD-style)

```
Tier 0: Last B steps at full resolution
Tier 1: B steps at 1/k decimation
Tier 2: B steps at 1/k² decimation
...

Parameters:
- B = bucket size (e.g., 100)
- k = decimation factor (e.g., 2)
```

### Example (B=100, k=2)

For 10,000 steps:
```
Tier 0: steps 9900-9999 (100 steps, full)
Tier 1: steps 9700-9899 at 1/2 → 100 stored
Tier 2: steps 9300-9699 at 1/4 → 100 stored
Tier 3: steps 8500-9299 at 1/8 → 100 stored
Tier 4: steps 6900-8499 at 1/16 → 100 stored
Tier 5: steps 3700-6899 at 1/32 → 100 stored
Tier 6: steps 0-3699 at 1/64 → ~58 stored
```

**Total: ~658 steps stored for 10k history** (vs 10k for full)

### Seek Algorithm

To seek to step N:
1. Find tier containing N
2. Find nearest stored keyframe before N
3. Recompute from keyframe to N

**Worst case recomputation**: k^tier ≈ O(N) for very old steps
**Typical case**: O(B) for recent steps

## Alternative: BTD (Best-To-Date) Only

Store only steps where error improved:
- Natural for optimization traces
- Much sparser (maybe 50-200 steps for 10k run)
- Lose intermediate states, but those are less interesting

**Hybrid**: BTD + tiered sampling for non-BTD steps?

## Alternative: Phi-based (Golden Ratio) Spacing

Store steps at: 0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, ...

- O(log₁.618(N)) ≈ O(1.44 * log₂(N)) storage
- Even spacing on log scale
- Simple to implement

## I-frames vs P-frames (Video Codec Analogy)

Like video compression:
- **I-frames** (keyframes): Full shape state, independently renderable
- **P-frames** (predicted): Delta from previous frame, requires decoding chain

### Option A: I-frames only (current)
Store full shapes at each keyframe. Simple, but larger.

### Option B: I-frames + P-frames
- Keyframes: Full shapes (every B steps)
- P-frames: Just the gradient vector and learning rate (tiny)
- To render step N: load nearest keyframe, replay P-frames

**P-frame size**: ~48 bytes per shape (6 f64 coords × 8 bytes) vs ~200+ bytes for full shape with Dual
**Compression**: 4-5x smaller for P-frames

### Seek Complexity
- **I-frame only**: O(1) seek to any keyframe
- **I + P**: O(distance to nearest keyframe) to decode

This is exactly why video seeking is "slow" - must decode from nearest I-frame.

## Storage Format

### I-frames only (simpler)
```json
{
  "format": "tiered",
  "params": { "bucket_size": 100, "decimation": 2 },
  "total_steps": 10000,
  "keyframes": [
    { "step_idx": 0, "tier": 6, "error": 0.5, "shapes": [...] },
    { "step_idx": 64, "tier": 6, "error": 0.3, "shapes": [...] },
    ...
  ]
}
```

### I-frames + P-frames (more compact)
```json
{
  "format": "tiered-delta",
  "params": { "bucket_size": 100, "decimation": 2 },
  "total_steps": 10000,
  "learning_rate": 0.05,
  "keyframes": [
    { "step_idx": 0, "type": "I", "error": 0.5, "shapes": [...] },
    { "step_idx": 1, "type": "P", "error": 0.48, "gradient": [...] },
    { "step_idx": 2, "type": "P", "error": 0.45, "gradient": [...] },
    ...
    { "step_idx": 100, "type": "I", "error": 0.1, "shapes": [...] },
    ...
  ]
}
```

## File Organization

```
testcases/
  fizz-buzz.json       # inputs + targets + expected (git)

traces/
  fizz-buzz.json.dvx   # pointer to S3 (git)
  fizz-buzz.json       # actual trace (S3 via dvx)
```

## Implementation Plan

### Phase 1: BTD-only mode
- `--btd` flag: only save steps where error improved
- Simplest, often sufficient

### Phase 2: Tiered sampling
- `--tiered B=100,k=2` flag
- Implement tier assignment and seek

### Phase 3: dvx integration
- Move large traces to S3
- Keep small test fixtures in git

## Open Questions

1. What bucket size B? (100 seems reasonable)
2. What decimation factor k? (2 is standard, 4 might be enough)
3. BTD-only vs hybrid?
4. LRU cache for recomputed steps in frontend?

## References

- [RRDtool](https://oss.oetiker.ch/rrdtool/) - canonical tiered time-series DB
- [Exponential Histograms](https://en.wikipedia.org/wiki/Exponential_histogram) - streaming algorithms
- [Prometheus downsampling](https://prometheus.io/docs/prometheus/latest/querying/basics/)
