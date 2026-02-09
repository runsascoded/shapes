/**
 * @apvd/worker - WorkerTrainingClient for running training
 * in a Web Worker using the @apvd/wasm module.
 *
 * @example
 * import { createWorkerTrainingClient } from "@apvd/worker";
 *
 * const client = createWorkerTrainingClient();
 * const model = await client.createModel(inputs, targets);
 */

export { WorkerTrainingClient, createWorkerTrainingClient } from "./client";

// Dual number extraction utilities (for unwrapping WASM Shape<Dual> → Shape<number>)
// Imported from utils.ts (not worker.ts) to avoid pulling in worker's
// `self.onmessage` side effect when used on the main thread.
export {
  extractNumber,
  extractPoint,
  extractShape,
  extractShapes,
  tier,
  resolution,
  isKeyframe,
  nearestKeyframe,
} from "./utils";

// Re-export types from @apvd/client for convenience
export type {
  TrainingClient,
  TrainingRequest,
  TrainingHandle,
  ProgressUpdate,
  StepState,
  StepStateWithGeometry,
  TraceInfo,
  Unsubscribe,
  Shape,
  InputSpec,
  TargetsMap,
  BatchTrainingRequest,
  BatchTrainingResult,
  ContinueTrainingResult,
  SparklineData,
} from "@apvd/client";
