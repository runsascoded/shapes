# Frontend API Spec

The frontend uses a unified API to interact with either:
1. **WebSocket RPC backend** (Rust server via `apvd serve`)
2. **WASM Worker** (shapes-wasm compiled to WebAssembly, running in a Web Worker)

Both backends implement the same interface. The frontend doesn't need to know which transport it's using.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Frontend (React)                         │
├──────────────────────────────────────────────────────────────────┤
│                       TrainingClient API                         │
├─────────────────────────────┬────────────────────────────────────┤
│    WebSocketTransport       │         WorkerTransport            │
│    (RPC to Rust server)     │         (postMessage to Worker)    │
└─────────────────────────────┴────────────────────────────────────┘
             │                              │
             ▼                              ▼
    ┌────────────────┐           ┌─────────────────────┐
    │  apvd serve    │           │  shapes-wasm.wasm   │
    │  (Rust native) │           │  (WASM in Worker)   │
    └────────────────┘           └─────────────────────┘
```

## TrainingClient Interface

```typescript
interface TrainingClient {
  // Start training with given inputs and targets
  startTraining(request: TrainingRequest): Promise<TrainingHandle>;

  // Subscribe to training progress updates
  onProgress(callback: (update: ProgressUpdate) => void): Unsubscribe;

  // Stop training early
  stopTraining(handle: TrainingHandle): Promise<void>;

  // Get a specific step's state (for time-travel scrubbing)
  getStep(handle: TrainingHandle, stepIndex: number): Promise<StepState>;

  // Get trace metadata (BTD indices, total steps, etc.)
  getTraceInfo(handle: TrainingHandle): Promise<TraceInfo>;
}

type Unsubscribe = () => void;
```

## Request/Response Types

### TrainingRequest

```typescript
interface TrainingRequest {
  // Input shapes with trainable coordinate flags
  inputs: InputSpec[];

  // Target area constraints
  targets: TargetsMap;

  // Training parameters
  params?: TrainingParams;
}

interface InputSpec {
  shape: Shape;
  trainable: boolean[];  // Which coords are trainable
}

type Shape =
  | { kind: "Circle"; c: Point; r: number }
  | { kind: "XYRR"; c: Point; rx: number; ry: number }
  | { kind: "XYRRT"; c: Point; rx: number; ry: number; t: number };

type TargetsMap = Record<string, number>;  // e.g., { "0*": 5, "*1": 3, "01": 1 }

interface TrainingParams {
  maxSteps?: number;       // Default: 10000
  learningRate?: number;   // Default: 0.05
  convergenceThreshold?: number;  // Default: 1e-10
  parallel?: number;       // Number of parallel permutations (default: 1)
  robust?: boolean;        // Use Adam optimizer (default: false)
}
```

### TrainingHandle

```typescript
interface TrainingHandle {
  id: string;              // Unique identifier for this training session
  startedAt: number;       // Timestamp
}
```

### ProgressUpdate

Sent periodically during training (typically every N steps or on BTD improvements).

```typescript
interface ProgressUpdate {
  handleId: string;
  type: "progress" | "complete" | "error";

  // Current state
  currentStep: number;
  totalSteps: number;
  error: number;           // Current error value

  // Best so far
  minError: number;
  minStep: number;

  // Current shapes (for live preview)
  shapes: Shape[];

  // Training metadata
  elapsedMs: number;

  // Final result (only when type === "complete")
  finalResult?: TrainingResult;
}

interface TrainingResult {
  success: boolean;
  finalError: number;
  minError: number;
  minStep: number;
  totalSteps: number;
  trainingTimeMs: number;
  shapes: Shape[];         // Final shapes (at min_step)

  // Trace info for time-travel
  traceInfo: TraceInfo;
}
```

### TraceInfo

Metadata about the stored trace, enabling efficient time-travel UI.

```typescript
interface TraceInfo {
  totalSteps: number;

  // BTD (Best To Date) step indices - for "show only improvements" mode
  btdSteps: number[];

  // Tiered keyframe config (if tiered storage is used)
  tiered?: TieredConfig;
}

interface TieredConfig {
  bucketSize: number;      // B value (default: 1024)
}
```

### StepState

Full state at a specific step (returned by `getStep()`).

```typescript
interface StepState {
  stepIndex: number;
  error: number;
  shapes: Shape[];
  isKeyframe: boolean;     // Whether this was stored or recomputed
  recomputedFrom?: number; // If recomputed, which keyframe it started from
}
```

## Transport Implementations

### WebSocketTransport

Connects to `apvd serve` via WebSocket. Messages are JSON-encoded.

```typescript
// Request message format
interface WSRequest {
  id: string;              // Request ID for correlation
  method: string;          // "train", "stop", "getStep", "getTraceInfo"
  params: unknown;
}

// Response message format
interface WSResponse {
  id: string;              // Correlates to request
  result?: unknown;
  error?: { code: number; message: string };
}

// Server-initiated message (progress updates)
interface WSNotification {
  method: "progress";
  params: ProgressUpdate;
}
```

### WorkerTransport

Communicates with a Web Worker running shapes-wasm via `postMessage`.

```typescript
// Message to worker
interface WorkerRequest {
  id: string;
  type: "train" | "stop" | "getStep" | "getTraceInfo";
  payload: unknown;
}

// Message from worker
interface WorkerResponse {
  id: string;
  type: "result" | "error" | "progress";
  payload: unknown;
}
```

## Time-Travel / Scrubbing UX

The frontend implements a scrubber UI allowing users to:

1. **Drag to any step**: Slider from 0 to `totalSteps`
2. **Jump to BTD steps**: Show only improvement points using `btdSteps` array
3. **Live preview**: Request step state via `getStep()`, display shapes

### Efficient Seeking with Tiered Keyframes

When tiered storage is enabled:
- Keyframes are stored at exponentially decreasing density
- Non-keyframe steps are recomputed from nearest keyframe
- `getStep()` returns the state, hiding recomputation from the caller

```
Tier 0: steps 0-2047     → every step is a keyframe
Tier 1: steps 2048-4095  → every 2nd step is a keyframe
Tier 2: steps 4096-8191  → every 4th step is a keyframe
Tier 3: steps 8192-16383 → every 8th step is a keyframe
...
```

Worst-case recomputation for step N in tier T: 2^T steps forward from nearest keyframe.

### Suggested UI Behavior

```typescript
// When user scrubs to a step
async function onScrub(stepIndex: number) {
  // Show loading indicator for non-keyframe steps
  const state = await client.getStep(handle, stepIndex);
  renderShapes(state.shapes);
  updateErrorDisplay(state.error);
}

// For "show improvements only" mode
function showBTDOnly(traceInfo: TraceInfo) {
  const btdSteps = traceInfo.btdSteps;
  // Use these as discrete scrubber positions
}
```

## Progress Update Frequency

The backend sends progress updates:
- Every 100 steps (configurable)
- On every BTD improvement (new minimum error)
- On completion or error

This keeps the UI responsive without overwhelming the transport.

## Error Handling

```typescript
interface TrainingError {
  code: "INVALID_INPUT" | "TRAINING_DIVERGED" | "CANCELLED" | "INTERNAL";
  message: string;
  details?: unknown;
}
```

## Example Usage

```typescript
import { createTrainingClient } from "@apvd/client";

// Create client with WebSocket transport
const client = createTrainingClient({
  transport: "websocket",
  url: "ws://localhost:8080",
});

// Or with Worker transport
const client = createTrainingClient({
  transport: "worker",
  wasmUrl: "/shapes.wasm",
});

// Start training
const handle = await client.startTraining({
  inputs: [
    { shape: { kind: "Circle", c: { x: 0, y: 0 }, r: 1 }, trainable: [true, true, true] },
    { shape: { kind: "Circle", c: { x: 1, y: 0 }, r: 1 }, trainable: [true, true, true] },
  ],
  targets: { "0*": 3, "*1": 5, "01": 1 },
  params: { maxSteps: 10000 },
});

// Subscribe to progress
const unsubscribe = client.onProgress((update) => {
  console.log(`Step ${update.currentStep}/${update.totalSteps}, error: ${update.error}`);
  renderShapes(update.shapes);

  if (update.type === "complete") {
    console.log("Training complete!");
    enableTimeTravel(update.finalResult!.traceInfo);
  }
});

// Time-travel to a specific step
const state = await client.getStep(handle, 500);
renderShapes(state.shapes);

// Cleanup
unsubscribe();
```

## Implementation Notes

1. **Same API, different transports**: The frontend code doesn't change between WebSocket and Worker modes. Only the transport configuration differs.

2. **Regeneration, not replay**: The backend always recomputes requested steps from keyframes. There's no "replay" mode that reads from stored history. This keeps the API simple and deterministic.

3. **Compression**: Trace files on disk use gzip compression (18-25x reduction). The API returns uncompressed JSON; compression is a storage concern.

4. **Parallel training**: When `parallel > 1`, the backend trains multiple permutations. Progress updates show the best-so-far across all variants. Final result is the best variant.
