# Training Roadmap

## Context

Generate showcase diagrams for reference target sets via CLI batch training. As we hit real-world difficulties, triage improvements.

## Done

### Error-Scaled Stepping (fixed)
FE used `step_size = error * lr`, suppressing penalties as error dropped. Fixed via `step_clipped` (fixed LR + gradient clipping). CLI `--robust` uses Adam + fixed LR 0.05 + clipping + backtracking.

### N-Shape Symmetric Layout (99bee47)
Generalized `5-shape-layout` → `shape-layout` with `-n` flag. Trainable offset + angle_step as Dual parameters. KL divergence loss. Restart-from-best. `--fix-angle` for fixed symmetry.
- N=4 fixed 90°: 15/15 regions, min/total 4.6%, teardrop deformations
- N=5: 31/31 regions as before

### Error Components (76f7a54)
`ErrorKind` enum, `Penalties` struct on `Step`. TypeScript types auto-generated.

### Gradient Anchors (289e4b4)
Per-shape anchor points with position gradients on `Step`. Frontend rendering not yet done.

### Per-Region/Shape Penalty Breakdown (224a508)
`PenaltyConfig` with per-region and per-shape penalty attribution.

### PhaseConfig (d1e4795)
`step_with_config` WASM export for FE live-adjustment of training params.

## Next: Showcase Diagram Generation

Run batch training on real target sets and evaluate results:

**Target sets**:
- `VARIANT_CALLERS` (4-set, 15 regions)
- `MPOWER` (4-set, 15 regions)
- `FIZZ_BUZZ_BAZZ` (3-set, 7 regions)
- 5-set examples (from testcases/)

```bash
# 4-set with robust optimizer, parallel variants
apvd train -s testcases/variant-callers.json -t targets.json --robust -m 10000 -p 12

# 3-set
apvd train -s circles3.json -t fizz-buzz-bazz.json --robust -m 5000
```

Use these runs to identify what actually needs improving.

## Backlog (revisit as needed)

### P:A Crossover Weighting
Auto-scale P:A weight: `pa_weight = (crossover_pct * initial_area_error) / initial_pa_penalty`. Low priority — try default weights first.

### Missing Region Penalty Redesign
Parent/child cascading walk instead of centroid-distance heuristics. See `specs/missing-region-penalties.md`. Implement if showcase runs reveal missing-region problems.

### Zero-Region Optimization
Eliminating regions with `target=0`. See `specs/zero-region-optimization.md`. Implement if showcase runs stall on zero-target regions.

### Timing Metrics
Instrument `step.rs` with per-phase timing. See `specs/timing-metrics.md`. Implement when profiling is needed.

### Infinite Error Bug
`createModel` returns `error: inf` for some 3-shape inputs. See `specs/infinite-error-bug.md`. Fix when encountered in practice.
