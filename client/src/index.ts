/**
 * @apvd/client - Training client types and WebSocket transport
 *
 * This package provides:
 * - Core types for the TrainingClient interface
 * - WebSocketTrainingClient for connecting to native Rust server (apvd serve)
 *
 * For WASM/Worker-based training in the browser, use the apvd-wasm package
 * which provides WorkerTrainingClient.
 *
 * @example
 * import { createTrainingClient } from "@apvd/client";
 *
 * // Create client with WebSocket transport
 * const client = createTrainingClient({
 *   transport: "websocket",
 *   url: "ws://localhost:8080"
 * });
 *
 * // Start training
 * const handle = await client.startTraining({
 *   inputs: [
 *     [{ kind: "Circle", c: { x: 0, y: 0 }, r: 1 }, [true, true, true]],
 *     [{ kind: "Circle", c: { x: 1, y: 0 }, r: 1 }, [true, true, true]],
 *   ],
 *   targets: { "0*": 3, "*1": 5, "01": 1 },
 * });
 *
 * // Subscribe to progress
 * const unsubscribe = client.onProgress((update) => {
 *   console.log(`Step ${update.currentStep}, error: ${update.error}`);
 *   renderShapes(update.shapes);
 *
 *   if (update.type === "complete") {
 *     console.log("Training complete!");
 *   }
 * });
 *
 * // Time-travel to a specific step
 * const state = await client.getStep(handle, 500);
 * renderShapes(state.shapes);
 *
 * // Cleanup
 * unsubscribe();
 * client.disconnect();
 */

// Re-export types
export type {
  // Shape types
  Point,
  Circle,
  XYRR,
  XYRRT,
  Polygon,
  Shape,

  // Input/Target types
  InputSpec,
  TargetsMap,

  // Training types
  TrainingParams,
  TrainingRequest,
  TrainingHandle,

  // Progress/Result types
  ProgressUpdate,
  TrainingResult,

  // Trace/Time-travel types
  TraceInfo,
  TieredConfig,
  StepState,

  // Geometry types (rich step data)
  IntersectionPoint,
  Edge,
  Region,
  RegionError,
  Component,
  StepGeometry,
  StepStateWithGeometry,

  // Client interface
  TrainingClient,
  Unsubscribe,

  // Transport config
  TransportConfig,
  WorkerTransportConfig,
  WebSocketTransportConfig,

  // Version info
  VersionInfo,

  // Worker message types (for apvd-wasm)
  WorkerRequest,
  WorkerResponse,
} from "./types";

// Re-export client implementations
export {
  createTrainingClient,
  WebSocketTrainingClient,
} from "./client";
