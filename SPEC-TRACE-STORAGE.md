# Trace Storage Spec

**Status**: Partially implemented. Tiered keyframes implemented in `@apvd/worker`. BTD bitmask not yet implemented. See also `specs/btd-trace-format.md` for the BTD-specific format spec.

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

## Design: Tiered Keyframes + BTD Bitmask

Two orthogonal concerns:
1. **Keyframes**: Tiered I-frames for efficient random seek
2. **BTD (Best To Date)**: Bitmask marking which steps achieved new minimum error

BTD steps aren't stored specially - if we need one that's not a keyframe, we recompute from the nearest keyframe like any other step.

## Tiered Keyframe Scheme

```
Tier 0: 2B samples at resolution 1  → covers [0, 2B)
Tier 1: B samples at resolution 2   → covers [2B, 4B)
Tier 2: B samples at resolution 4   → covers [4B, 8B)
Tier 3: B samples at resolution 8   → covers [8B, 16B)
...
Tier n: B samples at resolution 2^n → covers [B·2^n, B·2^(n+1))
```

**Key properties**:
- All tier boundaries at B·2^k (powers of 2)
- Coverage after tier n: B·2^(n+1) steps
- Storage: 2B + n·B = B·(n+2) samples

### Example: 100k steps → 10k storage

With B=1563, n=5 tiers:
```
Tier 0: 3126 samples, resolution 1  → [0, 3126)
Tier 1: 1563 samples, resolution 2  → [3126, 6252)
Tier 2: 1563 samples, resolution 4  → [6252, 12504)
Tier 3: 1563 samples, resolution 8  → [12504, 25008)
Tier 4: 1563 samples, resolution 16 → [25008, 50016)
Tier 5: 1563 samples, resolution 32 → [50016, 100032)
```

Total: 10,941 samples for 100k steps (10.9:1 compression)

### Tier Lookup

```rust
fn tier(step: usize, b: usize) -> usize {
    if step < 2 * b { 0 }
    else { (step / b).ilog2() as usize }
}

fn tier_start(tier: usize, b: usize) -> usize {
    if tier == 0 { 0 } else { b << tier }
}

fn resolution(tier: usize) -> usize {
    if tier == 0 { 1 } else { 1 << (tier - 1) }
}
```

### Seek Algorithm

To seek to step N:
1. Find tier: `t = tier(N, B)`
2. Find resolution: `r = resolution(t)`
3. Find nearest keyframe: `kf = (N / r) * r`
4. Load keyframe state at `kf`
5. Recompute `N - kf` steps forward

**Worst case**: resolution 2^n → up to 2^n steps recomputation
**Typical case**: ~B/2 steps for uniformly random seek

## BTD Bitmask

Stored separately from keyframes:
```rust
struct TraceMetadata {
    total_steps: usize,
    btd_steps: Vec<usize>,  // sorted list of BTD step indices
    // or: btd_mask: BitVec,  // if dense enough
}
```

To find BTD steps in a range, binary search the sorted list.
To render a BTD step, seek to it like any other step.

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

### I-frames + P-frames (more compact, future optimization)
```json
{
  "format": "tiered-delta",
  "params": { "bucket_size": 1563 },
  "total_steps": 100000,
  "learning_rate": 0.05,
  "keyframes": [
    { "step_idx": 0, "type": "I", "error": 0.5, "shapes": [...] },
    { "step_idx": 1, "type": "P", "error": 0.48, "gradient": [...] },
    ...
  ]
}
```

P-frames store only gradient vectors (~48 bytes/shape vs ~200+ for full Dual shapes).
4-5x smaller but requires sequential decode from nearest I-frame.

## File Organization

```
testcases/
  fizz-buzz.json       # inputs + targets + expected (git)

traces/
  fizz-buzz.json.dvx   # pointer to S3 (git)
  fizz-buzz.json       # actual trace (S3 via dvx)
```

## Implementation Plan

### Phase 1: Tiered I-frames with BTD bitmask
- `--tiered` flag with B parameter
- Store keyframes at tier boundaries
- Store BTD indices as separate array
- Implement seek algorithm

### Phase 2: P-frames (optional optimization)
- Store gradients instead of full shapes between keyframes
- Reduces storage 4-5x
- Requires sequential decode

### Phase 3: dvx integration
- Move large traces to S3
- Keep small test fixtures in git

## Configuration

For 100k steps → 10k storage: `B = 1563`

```rust
const DEFAULT_BUCKET_SIZE: usize = 1563;  // ~10:1 compression at 100k steps
```

Bucket size can be tuned:
- Smaller B → more compression, longer recomputation
- Larger B → less compression, faster seek

## References

- [RRDtool](https://oss.oetiker.ch/rrdtool/) - canonical tiered time-series DB
- [Exponential Histograms](https://en.wikipedia.org/wiki/Exponential_histogram) - streaming algorithms
- [Video I-frames/P-frames](https://en.wikipedia.org/wiki/Video_compression_picture_types)
