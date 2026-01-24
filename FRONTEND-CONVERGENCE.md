# Configurable Convergence Threshold

## Summary

The shapes library now exposes a `converged` flag on each `Step` that indicates when error has dropped below a threshold. Currently this threshold is hardcoded at `1e-10`, but it should be user-configurable.

## Current State (shapes library)

```typescript
interface Step {
  // ... existing fields ...
  converged: boolean;  // true when error < CONVERGENCE_THRESHOLD (currently 1e-10)
}
```

## Proposed Frontend Changes

### 1. Add UI Control

Add a convergence threshold setting, possibly in a "Settings" or "Advanced" panel:

```
Convergence threshold: [1e-10 ▼]  // dropdown or input
Options: 1e-6, 1e-8, 1e-10, 1e-12, 1e-14, Custom...
```

### 2. Use the threshold in iteration loop

```typescript
// Instead of relying on step.converged (which uses hardcoded 1e-10):
const userThreshold = settings.convergenceThreshold ?? 1e-10;

while (stepCount < maxSteps) {
  step = wasm.step(step, learningRate);

  if (step.error < userThreshold) {
    console.log(`Converged at step ${stepCount} with error ${step.error}`);
    break;
  }

  stepCount++;
}
```

### 3. Alternative: Pass threshold to backend

If we want the backend to handle this, we could add a new function:

```rust
// In shapes library
#[wasm_bindgen]
pub fn step_with_threshold(step: JsValue, learning_rate: f64, convergence_threshold: f64) -> JsValue {
    // ... returns step with converged computed using custom threshold
}
```

Let me know which approach you prefer and I can implement the backend changes if needed.

## Recommended Default Values

| Use Case | Threshold | Notes |
|----------|-----------|-------|
| Quick preview | 1e-6 | Fast, visually good enough |
| Standard | 1e-10 | Good balance (current default) |
| High precision | 1e-14 | Near floating-point limits |
| Publication quality | 1e-12 | Very accurate areas |

## Related: Early Stopping

The frontend currently stops when a step repeats a recent state. At very low error levels (< 1e-14), floating-point noise prevents exact repeats. The convergence threshold provides a cleaner stopping criterion.

Consider replacing or supplementing the "repeat detection" with:
```typescript
if (step.error < userThreshold || step.converged) {
  break;
}
```
