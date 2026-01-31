# Rich Step Data Spec

## Problem

The current `@apvd/client` API returns simplified `StepState`:

```typescript
interface StepState {
  stepIndex: number;
  error: number;
  shapes: Shape[];
  isKeyframe: boolean;
}
```

But the apvd frontend requires full geometric data for visualization:

```typescript
// Frontend's Step type (from lib/regions.ts)
interface Step {
  sets: S[];
  points: Point[];        // Intersection points
  edges: Edge[];          // Shape boundary segments
  regions: Region[];      // Computed regions with areas
  components: Component[]; // Connected components
  targets: Targets<number>;
  total_area: Dual;
  errors: Errors;         // Per-region error values
  error: Dual;
}
```

This data is computed by `makeStep()` which processes raw WASM `apvd.Step` output. The current client API cannot support the full visualization because it doesn't include regions, edges, intersection points, etc.

## Current Architecture

```
Frontend                    Worker/Server
   │                            │
   │  startTraining(inputs)     │
   ├───────────────────────────►│
   │                            │ runs gradient descent
   │  onProgress(shapes, error) │ stores keyframes
   │◄───────────────────────────┤
   │                            │
   │  getStep(idx)              │
   ├───────────────────────────►│
   │  StepState(shapes, error)  │
   │◄───────────────────────────┤
   │                            │
   │  ??? geometry ???          │
   │                            │
```

The frontend currently calls WASM directly to compute geometry:
1. Gets shapes from training
2. Calls `apvd.make_step()` to compute regions/edges/points
3. Processes via `makeStep()` to build frontend `Step` type

## Options

### Option A: Extend StepState with Geometry

Add full geometric data to `StepState` and `ProgressUpdate`:

```typescript
interface StepState {
  stepIndex: number;
  error: number;
  shapes: Shape[];
  isKeyframe: boolean;
  // NEW: Full geometric data
  geometry?: StepGeometry;
}

interface StepGeometry {
  components: Component[];
  regions: Region[];
  total_area: number;
  errors: Record<string, { actual: number; target: number; delta: number }>;
}
```

**Pros:**
- Frontend becomes a pure display layer
- No WASM dependency in frontend for `static` branch
- Unified data flow through client API

**Cons:**
- More data per step (~10-100x larger payloads)
- Worker/server must serialize complex nested structures
- May impact scrubbing performance

### Option B: Hybrid - Lightweight Progress, On-Demand Geometry

Keep `ProgressUpdate` lightweight (shapes/error only), add separate `getStepGeometry()`:

```typescript
interface TrainingClient {
  // ... existing methods ...

  // NEW: Get full geometry for a step (expensive)
  getStepGeometry(handle: TrainingHandle, stepIndex: number): Promise<StepGeometry>;
}
```

**Pros:**
- Progress updates stay fast
- Geometry only fetched when needed (displaying a step)
- Backward compatible with existing API

**Cons:**
- Two round-trips for full step display
- Complexity in frontend to manage partial data

### Option C: Keep Direct WASM for Geometry (Current)

Frontend continues to call WASM directly for geometry computation:

```typescript
// Frontend pattern (current)
const shapes = progressUpdate.shapes; // from client
const rawStep = apvd.make_step(shapes, targets); // direct WASM call
const step = makeStep(rawStep, initialSets); // frontend processing
```

**Pros:**
- No changes to client/protocol
- Works today
- Geometry computed on-demand

**Cons:**
- Frontend needs WASM (fine for `static` branch, odd for `server` branch)
- Duplicates computation (Worker already computed during training)
- Can't fully decouple frontend from WASM

## Recommendation

**Option B (Hybrid)** provides the best balance:

1. **Fast progress updates** - Keep `ProgressUpdate` lightweight with just shapes/error for live preview
2. **Rich step data on demand** - Add `getStepWithGeometry()` that returns full geometric data
3. **Caching** - Worker/server can cache recent geometries to avoid recomputation

### Proposed API Changes

```typescript
interface TrainingClient {
  // Existing
  startTraining(request: TrainingRequest): Promise<TrainingHandle>;
  onProgress(callback: (update: ProgressUpdate) => void): Unsubscribe;
  stopTraining(handle: TrainingHandle): Promise<void>;
  getStep(handle: TrainingHandle, stepIndex: number): Promise<StepState>;
  getTraceInfo(handle: TrainingHandle): Promise<TraceInfo>;

  // NEW: Get step with full geometric data
  getStepWithGeometry(
    handle: TrainingHandle,
    stepIndex: number
  ): Promise<StepStateWithGeometry>;
}

interface StepStateWithGeometry extends StepState {
  geometry: StepGeometry;
}

interface StepGeometry {
  components: Component[];
  // Component includes: key, points, edges, regions
  total_area: number;
  errors: Record<string, RegionError>;
}

interface RegionError {
  actual: number;
  target: number;
  delta: number;
  // gradient info if available
}
```

### Implementation Notes

1. **Worker transport**: Worker calls `make_step()` internally, serializes full result
2. **WebSocket transport**: Server computes geometry and returns JSON
3. **Frontend**: Uses `getStep()` for quick preview, `getStepWithGeometry()` for full display
4. **Caching**: Last N step geometries cached to avoid recomputation during scrubbing

## Impact on Branches

### `static` branch (WASM Worker)
- Worker already has WASM, just needs to serialize geometry in `getStepWithGeometry()`
- Frontend removes direct WASM calls, uses client API exclusively
- Uses `WorkerTrainingClient` from `apvd-wasm` package

### `server` branch (WebSocket)
- Server computes geometry using native Rust
- Frontend has no WASM dependency at all - uses `@apvd/client` only
- Uses `WebSocketTrainingClient` from `@apvd/client` package
- **Current state**: Needs:
  1. `createModel(inputs, targets)` RPC → returns initial Step with geometry
  2. `train(model, params)` or streaming progress → returns Steps with geometry
  3. Server implementation in `shapes` crate

### Shared Code
- Types from `@apvd/client` are shared by both transports
- `apvd-wasm` uses `import type` from `@apvd/client` for type compatibility
- Frontend visualization code unchanged - works with either client

## Migration Path

1. **Phase 1**: Add `getStepWithGeometry()` to client API (both transports)
2. **Phase 1b** (server only): Add `createModel()` RPC - server branch has no WASM, needs server to create initial model
3. **Phase 2**: Update frontend to use new method for step display
4. **Phase 3**: Remove direct WASM imports from frontend (static branch)
5. **Phase 4**: Optional - deprecate `getStep()` if never needed

**Note**: Server branch currently has stub functions `make_model()` and `train()` that throw errors. These must be replaced with RPC calls before the server branch is functional.

## Questions

1. Should `ProgressUpdate` include geometry for the "current shapes" preview? (Probably no - too expensive at 100Hz)
2. What geometry fields are actually needed? (Regions yes, edges maybe, raw points unclear)
3. Cache size for geometry? (Probably just current step + 2-3 nearby for scrubbing)

## Server-Specific Considerations

The `server` branch has additional requirements beyond the Worker transport:

### Session/Concurrency Management

The server handles multiple clients simultaneously. Each training session should:
- Have a unique `TrainingHandle.id` (UUID or similar)
- Be isolated from other sessions
- Support cleanup when client disconnects or times out
- Allow reconnection to existing sessions (optional but valuable)

### Initial Model Creation

The `server` branch removed `apvd-wasm`, so it can't call `make_model()` locally. We need an additional RPC:

```typescript
interface TrainingClient {
  // NEW: Create initial model (needed for server branch)
  createModel(
    inputs: InputSpec[],
    targets: TargetsMap
  ): Promise<StepStateWithGeometry>;
  // ... existing methods ...
}
```

This returns the initial step (step 0) with full geometry, before any training.

### Batch Step Fetching

For efficient scrubbing, consider batched requests:

```typescript
// Fetch multiple steps in one round-trip
getStepsWithGeometry(
  handle: TrainingHandle,
  stepIndices: number[]
): Promise<StepStateWithGeometry[]>;

// Or fetch a range
getStepRange(
  handle: TrainingHandle,
  startIndex: number,
  endIndex: number,
  includeGeometry: boolean
): Promise<StepState[] | StepStateWithGeometry[]>;
```

### Streaming for Large Traces

For very long traces (100k+ steps), the server might want to:
- Stream progress via Server-Sent Events (SSE) as alternative to WebSocket
- Paginate `getTraceInfo()` for BTD step lists
- Support partial trace export

### Parallel Training

Server can leverage multi-core:
- Run N training sessions with different initial layouts
- Fan out, collect best result
- Consider API for this:

```typescript
interface ParallelTrainingRequest {
  variants: TrainingRequest[];  // Different initial conditions
  selectionCriteria: "min_error" | "first_convergence";
}

interface ParallelTrainingResult {
  bestVariantIndex: number;
  results: TrainingResult[];
}
```

### Connection Recovery

WebSocket connections can drop. Options:
- Client auto-reconnects with same `handle.id`
- Server preserves session for N minutes after disconnect
- Or: training completes server-side, client fetches results later

### RPC Protocol

Recommend JSON-RPC 2.0 format for consistency:

```json
// Request
{"jsonrpc": "2.0", "id": "1", "method": "getStepWithGeometry", "params": {"handleId": "abc", "stepIndex": 42}}

// Response
{"jsonrpc": "2.0", "id": "1", "result": {"stepIndex": 42, "shapes": [...], "geometry": {...}}}

// Error
{"jsonrpc": "2.0", "id": "1", "error": {"code": -32000, "message": "Session not found"}}

// Server-initiated notification (progress)
{"jsonrpc": "2.0", "method": "progress", "params": {"handleId": "abc", "currentStep": 100, ...}}
```

### Caching Strategy

Server-side caching differs from Worker:
- LRU cache keyed by `(session_id, step_index)`
- Consider Redis/memcached for multi-instance deployments
- Cache geometry separately from shapes (shapes are smaller, useful for quick preview)

### Authentication (Future)

For production deployment:
- Token-based auth (JWT or API key)
- Rate limiting per client
- Resource quotas (max concurrent sessions, max steps)

## Related

- `SPEC-FRONTEND-API.md` - Current client API definition
- `SPEC-TRACE-STORAGE.md` - Keyframe storage for time-travel
- `apvd/static/src/lib/regions.ts` - Frontend Step type definition
