/**
 * Core types for the APVD training client API.
 *
 * These types are used by both the Worker and WebSocket transports,
 * providing a unified interface for the frontend.
 */

// ============================================================================
// Shape Types
// ============================================================================

export interface Point {
  x: number;
  y: number;
}

export interface Circle {
  kind: "Circle";
  c: Point;
  r: number;
}

export interface XYRR {
  kind: "XYRR";
  c: Point;
  r: Point;  // { x: rx, y: ry }
}

export interface XYRRT {
  kind: "XYRRT";
  c: Point;
  r: Point;  // { x: rx, y: ry }
  t: number; // rotation angle in radians
}

export interface Polygon {
  kind: "Polygon";
  vertices: Point[];
}

export type Shape = Circle | XYRR | XYRRT | Polygon;

// ============================================================================
// Input/Target Types
// ============================================================================

/** Shape with trainable coordinate flags */
export type InputSpec = [Shape, boolean[]];

/** Map of region keys to target area sizes */
export type TargetsMap = Record<string, number>;

// ============================================================================
// Training Request/Response Types
// ============================================================================

export interface TrainingParams {
  /** Maximum training steps (default: 10000) */
  maxSteps?: number;
  /** Learning rate (default: 0.05) */
  learningRate?: number;
  /** Stop if error falls below this (default: 1e-10) */
  convergenceThreshold?: number;
  /** Use robust optimizer with Adam + clipping (default: false) */
  robust?: boolean;
  /** Progress update interval in steps (default: 100) */
  progressInterval?: number;
}

export interface TrainingRequest {
  /** Input shapes with trainable coordinate flags */
  inputs: InputSpec[];
  /** Target area constraints */
  targets: TargetsMap;
  /** Training parameters */
  params?: TrainingParams;
}

export interface TrainingHandle {
  /** Unique identifier for this training session */
  id: string;
  /** Timestamp when training started */
  startedAt: number;
}

// ============================================================================
// Progress/Result Types
// ============================================================================

export interface ProgressUpdate {
  handleId: string;
  type: "progress" | "complete" | "error";

  /** Current step index */
  currentStep: number;
  /** Total steps (max_steps) */
  totalSteps: number;
  /** Current error value */
  error: number;

  /** Best error seen so far */
  minError: number;
  /** Step where best error was achieved */
  minStep: number;

  /** Current shapes (for live preview) */
  shapes: Shape[];

  /** Elapsed training time in ms */
  elapsedMs: number;

  /** Final result (only when type === "complete") */
  finalResult?: TrainingResult;

  /** Error message (only when type === "error") */
  errorMessage?: string;
}

export interface TrainingResult {
  success: boolean;
  finalError: number;
  minError: number;
  minStep: number;
  totalSteps: number;
  trainingTimeMs: number;
  /** Final shapes at min_step (best result) */
  shapes: Shape[];
  /** Trace info for time-travel */
  traceInfo: TraceInfo;
}

// ============================================================================
// Trace/Time-Travel Types
// ============================================================================

export interface TraceInfo {
  totalSteps: number;
  /** BTD (Best To Date) step indices */
  btdSteps: number[];
  /** Tiered keyframe config (if tiered storage is used) */
  tiered?: TieredConfig;
}

export interface TieredConfig {
  /** Bucket size B (default: 1024) */
  bucketSize: number;
}

export interface StepState {
  stepIndex: number;
  error: number;
  shapes: Shape[];
  /** Whether this was stored as a keyframe */
  isKeyframe: boolean;
  /** If recomputed, which keyframe it started from */
  recomputedFrom?: number;
}

// ============================================================================
// Client Interface
// ============================================================================

export type Unsubscribe = () => void;

export interface TrainingClient {
  /** Start training with given inputs and targets */
  startTraining(request: TrainingRequest): Promise<TrainingHandle>;

  /** Subscribe to training progress updates */
  onProgress(callback: (update: ProgressUpdate) => void): Unsubscribe;

  /** Stop training early */
  stopTraining(handle: TrainingHandle): Promise<void>;

  /** Get a specific step's state (for time-travel scrubbing) */
  getStep(handle: TrainingHandle, stepIndex: number): Promise<StepState>;

  /** Get trace metadata (BTD indices, total steps, etc.) */
  getTraceInfo(handle: TrainingHandle): Promise<TraceInfo>;

  /** Disconnect and clean up resources */
  disconnect(): void;
}

// ============================================================================
// Transport Configuration
// ============================================================================

export interface WorkerTransportConfig {
  transport: "worker";
  /** URL to the WASM file (default: auto-detect from apvd-wasm) */
  wasmUrl?: string;
}

export interface WebSocketTransportConfig {
  transport: "websocket";
  /** WebSocket URL (e.g., "ws://localhost:8080") */
  url: string;
}

export type TransportConfig = WorkerTransportConfig | WebSocketTransportConfig;

// ============================================================================
// Worker Message Types (internal)
// ============================================================================

export interface WorkerRequest {
  id: string;
  type: "train" | "stop" | "getStep" | "getTraceInfo";
  payload: unknown;
}

export interface WorkerResponse {
  id: string;
  type: "result" | "error" | "progress";
  payload: unknown;
}
