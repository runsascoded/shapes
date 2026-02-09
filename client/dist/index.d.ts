/**
 * @apvd/client - Training client types and WebSocket transport
 *
 * This package provides:
 * - Core types for the TrainingClient interface
 * - WebSocketTrainingClient for connecting to native Rust server (apvd serve)
 *
 * For WASM/Worker-based training in the browser, use @apvd/worker
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
export type { Point, Circle, XYRR, XYRRT, Polygon, Shape, InputSpec, TargetsMap, TrainingParams, TrainingRequest, TrainingHandle, BatchTrainingRequest, BatchStep, BatchTrainingResult, SparklineData, ContinueTrainingResult, ProgressUpdate, TrainingResult, TraceInfo, TieredConfig, StepState, IntersectionPoint, Edge, Region, RegionError, Component, StepGeometry, StepStateWithGeometry, TraceExportV1, TraceStepV1, TraceExportV2, TraceConfigV2, TraceKeyframeV2, TraceExport, StepSelector, LoadTraceResult, SaveTraceResult, TraceSummary, TraceListResult, RenameTraceResult, DeleteTraceResult, SampleTraceSummary, SampleTraceListResult, TrainingClient, Unsubscribe, TransportConfig, WorkerTransportConfig, WebSocketTransportConfig, VersionInfo, WorkerRequest, WorkerResponse, } from "./types";
export { createTrainingClient, WebSocketTrainingClient, } from "./client";
//# sourceMappingURL=index.d.ts.map