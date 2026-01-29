/**
 * @apvd/client - Training client for area-proportional Venn diagrams
 *
 * This package provides a unified TrainingClient interface that works with
 * either a Worker transport (WASM in browser) or WebSocket transport
 * (native Rust server via `apvd serve`).
 *
 * @example
 * import { createTrainingClient } from "@apvd/client";
 *
 * // Create client with Worker transport (default)
 * const client = createTrainingClient({ transport: "worker" });
 *
 * // Or with WebSocket transport
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

  // Client interface
  TrainingClient,
  Unsubscribe,

  // Transport config
  TransportConfig,
  WorkerTransportConfig,
  WebSocketTransportConfig,
} from "./types";

// Re-export client implementations
export {
  createTrainingClient,
  WorkerTrainingClient,
  WebSocketTrainingClient,
} from "./client";
