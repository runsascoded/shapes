/**
 * apvd-wasm TypeScript exports
 *
 * This module provides the WorkerTrainingClient for running training
 * in a Web Worker using the apvd-wasm module.
 *
 * @example
 * import { createWorkerTrainingClient } from "apvd-wasm";
 *
 * const client = createWorkerTrainingClient();
 * const model = await client.createModel(inputs, targets);
 */

export { WorkerTrainingClient, createWorkerTrainingClient } from "./client";

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
} from "@apvd/client";
