# @apvd/client: Batch Training API

## Context

The current `TrainingClient` API uses session-based long-running training, which doesn't fit well with the frontend's interactive control flow. The frontend wants to compute steps on-demand when the user clicks "advance" or "play".

## Required Changes

### 1. Add Types to `src/types.ts`

```typescript
export interface BatchTrainingRequest {
  /** Current shapes with trainability flags */
  inputs: InputSpec[]
  /** Target region sizes */
  targets: TargetsMap
  /** Number of steps to compute */
  numSteps: number
  /** Learning rate (default: 0.05) */
  learningRate?: number
}

export interface BatchStep {
  /** Relative index within this batch (0 to numSteps-1) */
  stepIndex: number
  /** Error at this step */
  error: number
  /** Shape coordinates at this step */
  shapes: Shape[]
}

export interface BatchTrainingResult {
  /** All computed steps */
  steps: BatchStep[]
  /** Minimum error in this batch */
  minError: number
  /** Index of step with minimum error (within batch) */
  minStepIndex: number
  /** Final shapes (convenience for next batch input) */
  finalShapes: Shape[]
}
```

### 2. Extend TrainingClient Interface

```typescript
export interface TrainingClient {
  // ... existing methods ...

  /**
   * Compute a batch of training steps synchronously.
   *
   * Stateless request - takes current shapes and targets,
   * returns shapes after numSteps gradient descent iterations.
   *
   * @param request - Batch training parameters
   * @returns Promise resolving to batch results
   */
  trainBatch(request: BatchTrainingRequest): Promise<BatchTrainingResult>
}
```

### 3. Worker Transport Implementation

In `src/worker/handler.ts` (or equivalent):

```typescript
case "trainBatch": {
  const { inputs, targets, numSteps, learningRate = 0.05 } = payload as BatchTrainingRequest

  // Create initial step from inputs
  let wasmStep = apvd.make_step(inputs, targets)

  const steps: BatchStep[] = [{
    stepIndex: 0,
    error: extractError(wasmStep),
    shapes: extractShapes(wasmStep.shapes),
  }]

  let minError = steps[0].error
  let minStepIndex = 0

  // Compute remaining steps
  for (let i = 1; i < numSteps; i++) {
    wasmStep = apvd.step(wasmStep, learningRate)
    const error = extractError(wasmStep)

    steps.push({
      stepIndex: i,
      error,
      shapes: extractShapes(wasmStep.shapes),
    })

    if (error < minError) {
      minError = error
      minStepIndex = i
    }

    // Yield periodically for responsiveness
    if (i % 100 === 0) {
      await yieldToEventLoop()
    }
  }

  return {
    steps,
    minError,
    minStepIndex,
    finalShapes: steps[steps.length - 1].shapes,
  }
}
```

### 4. WebSocket Transport Implementation

Same interface, sends request to Rust server:

```typescript
async trainBatch(request: BatchTrainingRequest): Promise<BatchTrainingResult> {
  return this.sendRequest("trainBatch", request)
}
```

Server-side Rust implementation computes batch and returns result.

## Migration Notes

- Session-based methods (`startTraining`, `stopTraining`, `getStep`, `onProgress`) can remain for now
- Frontend will migrate to use `trainBatch` for the primary workflow
- Session-based training may be useful for server-side heavy workloads later

## Testing

Add tests for:
- `trainBatch` with various step counts (1, 10, 100, 1000)
- Error decreases over batch (sanity check)
- `minError` and `minStepIndex` are correct
- `finalShapes` matches last step's shapes
- Handles edge cases (numSteps=0, numSteps=1)
