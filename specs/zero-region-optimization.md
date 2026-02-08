# Zero-Region Optimization

## Problem Statement

When target areas include regions with `target = 0`, the optimizer struggles to completely eliminate these regions. Gradient descent can shrink regions but has no mechanism to achieve the "topology change" required to make a region truly non-existent.

### Observed Behavior

In variant-callers with hexagons (4 shapes, 15 regions), two regions have `target = 0`:
- `0-2-`: Shapes 0 ∩ 2, excluding shapes 1, 3
- `0-23`: Shapes 0 ∩ 2 ∩ 3, excluding shape 1

Training to 10k-20k steps achieves ~3e-3 error, with roughly half the error coming from a single zero-target region that maintains a small but persistent non-zero area.

### Why It's Hard

1. **Gradient only says "smaller"**: For `(actual - 0)² = actual²`, the gradient `2 * actual` points toward reduction but provides no direction for *how* to eliminate the intersection entirely.

2. **Topology is discrete**: Whether two shapes intersect is a binary property. Continuous optimization can approach but not cross this boundary efficiently.

3. **Local minima**: Shapes may settle into configurations where further reduction requires large coordinated movements that gradient descent won't discover.

## Proposed Solutions

### 1. Loss Function Modifications

#### 1a. Asymmetric penalty for zero targets
```
loss = Σ (actual - target)²           # normal regions
     + Σ w * actual²                  # zero-target regions, w > 1
```

#### 1b. Multiplicative/logarithmic loss
```
loss = Σ (actual - target)² / target  # relative error (undefined for target=0)
     + Σ log(1 + actual/ε)            # zero-target regions
```
The log term has derivative `1/(ε + actual)` which increases as actual → 0.

#### 1c. Soft threshold penalty
```
loss = Σ (actual - target)²
     + Σ max(0, actual - threshold)^p  # penalty kicks in above threshold
```

### 2. Initial Layout Strategies

#### 2a. Topology-aware initialization
Given target regions, start with a layout that already has the correct "existence pattern":
1. Parse targets to identify zero regions
2. Start with fully separated shapes (all disjoint)
3. Incrementally merge shapes that need to intersect
4. Result: zero-target regions start at zero

#### 2b. On-demand layout generation
Rather than precompute all possible topologies, generate layouts dynamically:
```rust
fn generate_layout_for_targets(targets: &Targets) -> Vec<Shape> {
    let zero_regions = targets.iter().filter(|(_, v)| *v == 0).collect();
    // Use constraint solver or heuristics to find valid layout
}
```

### 3. Two-Phase Optimization

**Phase 1: Topology matching**
- Heavily penalize zero-target violations
- Allow larger learning rate / more aggressive moves
- Goal: get existence pattern correct

**Phase 2: Size optimization**
- Normal loss function
- Standard learning rate
- Goal: fine-tune region sizes

### 4. Combinatorial Moves

Add discrete "move types" that can achieve topology changes:
- "Separate": Move shape far from others (makes all its intersections zero)
- "Contain": Shrink shape to fit entirely inside another
- "Exclude": Move shape to not intersect specific other shapes

These could be triggered when gradient descent stalls on zero-target regions.

## Mathematical Background

### Topology Enumeration

For **n** shapes, there are **2ⁿ - 1** exclusive regions (non-empty subsets of the power set).

The number of possible "existence patterns" (which regions are non-empty) is **2^(2ⁿ-1)**:
- n=2: 2³ = 8 patterns
- n=3: 2⁷ = 128 patterns
- n=4: 2¹⁵ = 32,768 patterns

With symmetry under Sₙ (shape permutations), this reduces significantly but is still large.

### Related Mathematical Concepts

- **Boolean lattice antichains**: Related to valid existence patterns
- **Dedekind numbers**: Count monotone Boolean functions
- **Venn diagram realizability**: Not all existence patterns are realizable with convex shapes

### Open Questions

1. Is there an OEIS sequence for "number of realizable Venn topologies with n convex shapes"?
2. What's the minimum number of "template layouts" needed to cover all common use cases?
3. Can we characterize which existence patterns are realizable with circles vs. ellipses vs. polygons?

## Implementation Plan

### Phase 1: Loss function experiments
- [ ] Add `zero_weight` parameter to training
- [ ] Implement logarithmic loss variant
- [ ] Benchmark on variant-callers test case

### Phase 2: Topology-aware initialization
- [ ] Implement "fully separated" initial layout
- [ ] Add simple heuristics for merging shapes based on targets
- [ ] Test on various target sets

### Phase 3: Two-phase optimization
- [ ] Implement phase detection (when to switch)
- [ ] Tune hyperparameters for each phase
- [ ] Evaluate convergence improvements

## Test Cases

Priority test cases for zero-region optimization:
- `variant-callers-hexagons`: Has 2 zero regions (`0-2-`, `0-23`)
- Synthetic: Create targets with varying numbers of zero regions
- Edge cases: All regions zero except one, nested containment, etc.
