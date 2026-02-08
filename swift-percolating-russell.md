# Consolidate @apvd/worker and Static Frontend

**Status**: Partially done. `@apvd/worker` now has `continueTraining`, Dual extraction, and WASM init fix (commit `2b73c4b`). Static frontend still has duplicated worker/client logic that should be replaced with `@apvd/worker` imports.

## Context

Static's training pipeline has ~1500 lines of duplicated worker/client logic across 3 files, reimplementing what `@apvd/worker` should provide. The duplication includes:
- `extractNumber/Point/Shape` Dual unwrapping (3 copies)
- `tier/resolution/isKeyframe/nearestKeyframe` tiered storage (3 copies)
- Session management, training loops, progress reporting (2 copies)
- `WorkerRequest/WorkerResponse` types (2 copies)

Additionally, `@apvd/worker` has bugs (missing WASM init, no Dual unwrapping) and missing features (`continueTraining`, sparkline data extraction).

**Goal**: Make `@apvd/worker` the single source of truth, then have static use it.

## Current Architecture

```
Static FE active code path:
  App.tsx
  → TrainingClientProvider (contexts/TrainingClientContext.tsx, 789 lines)
    → createWorkerClient(worker) [prod] / createMainThreadClient() [dev]
  → useTrainingClientHook (hooks/useTrainingClient.ts, 1059 lines)
    → client.continueTraining() for batch advances
    → apvd.make_step() on main thread for step display

Dead code:
  hooks/useTraining.ts (303 lines) - no longer imported
  hooks/useCompactedModel.ts - only imported by useTraining.ts
```

## Phase 1: Enhance @apvd/worker (shapes repo)

### 1a. Add shared types to `client/src/types.ts`

- Add `"continueTraining"` to `WorkerRequest.type` union
- Add `ContinueTrainingResult` type (move from static's TrainingClientContext.tsx):
  ```ts
  export interface ContinueTrainingResult {
    totalSteps: number
    currentStep: number
    minError: number
    minStep: number
    currentShapes: Shape[]
    currentError: number
    steps: Array<{ stepIndex: number; error: number; shapes: Shape[] }>
    sparklineData?: SparklineData
  }
  ```
- Add `continueTraining(handle, numSteps)` to `TrainingClient` interface

### 1b. Fix and enhance `apvd-wasm/ts/worker.ts`

1. **Fix WASM init bug** (line 86): Add `await wasm.default?.()` before `wasm.init_logs()`
2. **Add Dual unwrapping** functions (`extractNumber`, `extractPoint`, `extractShape`, `extractShapes`) — move from static's worker.ts
3. **Use `extractShapes`** everywhere instead of raw `as Shape[]` casts (current code returns Dual-wrapped shapes to client)
4. **Add `handleContinueTraining`** handler — port from static's worker.ts (uses `apvd.train()` for batch computation, extracts sparkline data)
5. **Add `"continueTraining"` case** to `onmessage` switch
6. **Add sparkline data extraction** to `handleTrainBatch` (currently missing — static's version extracts gradients and per-region errors)

### 1c. Enhance `apvd-wasm/ts/client.ts`

- Add `continueTraining(handle: TrainingHandle, numSteps: number): Promise<ContinueTrainingResult>` method to `WorkerTrainingClient`

### 1d. Export extraction utils from `apvd-wasm/ts/index.ts`

- Export `extractNumber`, `extractPoint`, `extractShape`, `extractShapes` (static's main-thread client and useTrainingClient hook need these)

## Phase 2: Switch static to use @apvd/worker (apvd/static repo)

### 2a. Replace `src/workers/training.worker.ts` (~750 → ~5 lines)

```ts
// Re-export @apvd/worker's worker implementation.
// This file exists so Vite can bundle it with proper dependency resolution.
import "@apvd/worker/worker"
```

### 2b. Simplify `src/contexts/TrainingClientContext.tsx` (~789 → ~150 lines)

**Remove:**
- `createWorkerClient()` function (~70 lines) — use `WorkerTrainingClient` from `@apvd/worker`
- All `extractNumber/Point/Shape/Shapes` functions (~50 lines) — import from `@apvd/worker`
- `WorkerRequest/WorkerResponse` type definitions
- `BatchTrainingRequest/BatchStep/BatchTrainingResult` type definitions — import from `@apvd/client`
- `ContinueTrainingResult/SparklineData` type definitions — import from `@apvd/client`

**Keep:**
- `TrainingClient` extended interface (adds `continueTraining` to base)
- `createMainThreadClient()` for dev mode (but import `extractShapes` from `@apvd/worker`)
- `TrainingClientProvider`, `useTrainingClient` context/hook
- `isDev` check for prod/dev switching

**Update production path:**
```ts
import { WorkerTrainingClient } from "@apvd/worker"
// ...
const workerClient = new WorkerTrainingClient(
  new URL("../workers/training.worker?worker", import.meta.url)
)
```

### 2c. Clean up `src/hooks/useTrainingClient.ts`

- Remove `extractNumber/Point/Shape/Shapes` (3rd copy, ~50 lines) — import from `@apvd/worker`
- Remove `tier/resolution/isKeyframe` helpers (~20 lines) — import from `@apvd/worker` or `@apvd/wasm`
- Import `ContinueTrainingResult`, `SparklineData`, `TraceExport`, `TraceExportV2` from `@apvd/client` instead of redefining
- Remove local `TraceExport`/`TraceExportV2` type definitions (~50 lines) — already in `@apvd/client/types.ts`

### 2d. Delete dead code

- Delete `src/hooks/useTraining.ts` (303 lines, no imports)
- Delete `src/hooks/useCompactedModel.ts` (only imported by dead useTraining.ts)

## Key Technical Details

### Stepping Algorithm
- @apvd/worker currently uses fixed LR: `step(wasmStep, learningRate)` (default 0.05)
- Static uses error-scaled: `step(wasmStep, prevError * learningRate)` (default 0.5)
- `continueTraining` uses Rust's `train()` which does error-scaled internally
- **Resolution**: Keep fixed LR for `handleTrain` (recommended), add `continueTraining` which uses `train()` for batch computation (matching static's behavior)

### WASM Init Bug
- @apvd/worker's `initWasm()` only calls `wasm.init_logs()`, missing `await wasm.default()`
- Static correctly calls `await wasmModule.default()` first
- Fix: add `await (wasm as any).default?.()` before `init_logs()`

### Dual Number Unwrapping
- WASM `make_step` returns `Step<Dual>` where coordinates are `{ v: number, d: number[] }`
- @apvd/worker casts directly to `Shape[]` — BUG: returns Dual-wrapped shapes
- Static correctly uses `extractShape()` to unwrap Dual → plain number
- Fix: add extraction functions to @apvd/worker, use everywhere

### Dev Mode (Vite)
- Static needs a main-thread fallback for Vite dev server (Workers don't work well in dev)
- Keep `createMainThreadClient()` in static's TrainingClientContext
- Import shared utils (`extractShapes`) from `@apvd/worker` to reduce duplication

## Files Modified

### shapes repo
| File | Change |
|------|--------|
| `client/src/types.ts` | Add `ContinueTrainingResult`, update `WorkerRequest.type`, add `continueTraining` to `TrainingClient` |
| `apvd-wasm/ts/worker.ts` | Fix WASM init, add Dual extraction, add `continueTraining`, add sparklines |
| `apvd-wasm/ts/client.ts` | Add `continueTraining()` method |
| `apvd-wasm/ts/index.ts` | Export extraction utils |

### apvd/static repo
| File | Change |
|------|--------|
| `src/workers/training.worker.ts` | Replace 750 lines with 1-line re-export |
| `src/contexts/TrainingClientContext.tsx` | Remove ~600 lines of duplication |
| `src/hooks/useTrainingClient.ts` | Remove ~120 lines of duplication, import from shared |
| `src/hooks/useTraining.ts` | DELETE (dead code) |
| `src/hooks/useCompactedModel.ts` | DELETE (dead code) |

**Net reduction**: ~1400 lines removed from static

## Verification

1. **Build @apvd/worker**: `cd apvd-wasm/ts && pnpm build`
2. **Build @apvd/client**: `cd client && pnpm build`
3. **Build WASM**: `cd apvd-wasm && wasm-pack build --target web`
4. **Test static dev mode**: `cd ~/c/rac/apvd/static && pnpm dev` → verify training works
5. **Test static prod build**: `cd ~/c/rac/apvd/static && pnpm build` → verify no build errors
6. **Manual test**: Load a layout, click play, verify training advances, verify step navigation works
