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
// Geometry Types (for rich step data)
// ============================================================================

/** A point where two shape boundaries intersect */
export interface IntersectionPoint {
  /** Coordinates */
  p: Point;
  /** Index of first shape */
  shape0: number;
  /** Index of second shape */
  shape1: number;
  /** Theta on first shape's boundary */
  theta0: number;
  /** Theta on second shape's boundary */
  theta1: number;
}

/** An edge segment along a shape boundary */
export interface Edge {
  /** Shape index this edge belongs to */
  shapeIndex: number;
  /** Start point */
  start: Point;
  /** End point */
  end: Point;
  /** Start theta on shape boundary */
  startTheta: number;
  /** End theta on shape boundary */
  endTheta: number;
  /** Which side of the edge is "inside" the region */
  interiorSide: "left" | "right";
}

/** A computed region (intersection of sets) */
export interface Region {
  /** Region key (e.g., "01*" for in sets 0 and 1, any for set 2) */
  key: string;
  /** Computed area */
  area: number;
  /** Target area (if specified) */
  target?: number;
  /** Boundary edges forming this region */
  edges: Edge[];
  /** Centroid of the region */
  centroid?: Point;
}

/** Error info for a single region */
export interface RegionError {
  /** Computed area */
  actual: number;
  /** Target area */
  target: number;
  /** Difference (actual - target) */
  delta: number;
  /** Contribution to total error */
  errorContribution: number;
}

/** A connected component of the diagram */
export interface Component {
  /** Unique key for this component */
  key: string;
  /** Intersection points in this component */
  points: IntersectionPoint[];
  /** Edges in this component */
  edges: Edge[];
  /** Regions in this component */
  regions: Region[];
}

/** Full geometric data for a step */
export interface StepGeometry {
  /** Connected components of the diagram */
  components: Component[];
  /** Total area of all shapes */
  totalArea: number;
  /** Per-region error breakdown */
  errors: Record<string, RegionError>;
  /** All intersection points */
  points: IntersectionPoint[];
  /** All regions (flattened from components) */
  regions: Region[];
}

/** Step state with full geometric data */
export interface StepStateWithGeometry extends StepState {
  geometry: StepGeometry;
}

// ============================================================================
// Batch Training Types (stateless, on-demand step computation)
// ============================================================================

/** Request for stateless batch training */
export interface BatchTrainingRequest {
  /** Current shapes with trainability flags */
  inputs: InputSpec[];
  /** Target region sizes */
  targets: TargetsMap;
  /** Number of steps to compute */
  numSteps: number;
  /** Learning rate (default: 0.05) */
  learningRate?: number;
}

/** A single step in a batch result */
export interface BatchStep {
  /** Relative index within this batch (0 to numSteps-1) */
  stepIndex: number;
  /** Error at this step */
  error: number;
  /** Shape coordinates at this step */
  shapes: Shape[];
}

/** Sparkline data for visualization */
export interface SparklineData {
  /** Error values for each step in the batch */
  errors: number[];
  /** Gradient vectors for each step (gradients[stepIdx][varIdx]) */
  gradients: number[][];
  /** Per-region error values for each step (regionErrors[regionKey][stepIdx]) */
  regionErrors: Record<string, number[]>;
}

/** Result from batch training */
export interface BatchTrainingResult {
  /** All computed steps */
  steps: BatchStep[];
  /** Minimum error in this batch */
  minError: number;
  /** Index of step with minimum error (within batch) */
  minStepIndex: number;
  /** Final shapes (convenience for next batch input) */
  finalShapes: Shape[];
  /** Sparkline-ready data for visualization */
  sparklineData: SparklineData;
}

// ============================================================================
// Trace Export Types
// ============================================================================

/** V1 trace format (dense keyframes + errors array) */
export interface TraceExportV1 {
  version: 1;
  steps: TraceStepV1[];
  minIdx: number;
  minError: number;
  repeatIdx: number | null;
}

export interface TraceStepV1 {
  shapes: Shape[];
  errors: Record<string, unknown>;
  error: { v: number; d?: number[] };
}

/** V2 trace format (sparse BTD + interval keyframes) */
export interface TraceExportV2 {
  version: 2;
  config: TraceConfigV2;
  btdKeyframes: TraceKeyframeV2[];
  intervalKeyframes: TraceKeyframeV2[];
  totalSteps: number;
  minStep: number;
  minError: number;
}

export interface TraceConfigV2 {
  inputs: InputSpec[];
  targets: TargetsMap;
  learningRate: number;
}

export interface TraceKeyframeV2 {
  step: number;
  shapes: Shape[];
  error: number;
}

export type TraceExport = TraceExportV1 | TraceExportV2;

/** Step selection for loading traces */
export type StepSelector = "best" | "last" | "first" | number;

// ============================================================================
// Trace Management Types
// ============================================================================

/** Result from loading a trace */
export interface LoadTraceResult {
  loaded: true;
  traceId: string;
  format: "v1" | "v2-btd";
  loadedStep: number;
  totalSteps: number;
  minStep: number;
  minError: number;
  keyframeCount: number;
  step: StepStateWithGeometry;
}

/** Result from saving a trace */
export interface SaveTraceResult {
  traceId: string;
  name: string;
  savedAt: string;
}

/** Summary of a saved trace */
export interface TraceSummary {
  traceId: string;
  name: string;
  savedAt: string;
  totalSteps: number;
  minError: number;
  numShapes: number;
  shapeTypes: string[];
}

/** Result from listing traces */
export interface TraceListResult {
  traces: TraceSummary[];
}

/** Result from renaming a trace */
export interface RenameTraceResult {
  traceId: string;
  name: string;
}

/** Result from deleting a trace */
export interface DeleteTraceResult {
  deleted: true;
}

/** Summary of a sample trace */
export interface SampleTraceSummary {
  filename: string;
  name: string;
  totalSteps: number;
  minError: number;
  minStep: number;
  numShapes: number;
  sizeBytes: number;
}

/** Result from listing sample traces */
export interface SampleTraceListResult {
  samples: SampleTraceSummary[];
}

// ============================================================================
// Client Interface
// ============================================================================

export type Unsubscribe = () => void;

export interface TrainingClient {
  /**
   * Create initial model without training (server branch needs this).
   * Returns step 0 with full geometry for initial display.
   */
  createModel(inputs: InputSpec[], targets: TargetsMap): Promise<StepStateWithGeometry>;

  /**
   * Compute a batch of training steps synchronously.
   *
   * Stateless request - takes current shapes and targets,
   * returns shapes after numSteps gradient descent iterations.
   * Use this for on-demand step computation when user clicks "advance" or "play".
   *
   * @param request - Batch training parameters
   * @returns Promise resolving to batch results
   */
  trainBatch(request: BatchTrainingRequest): Promise<BatchTrainingResult>;

  /** Start training with given inputs and targets (session-based, streaming) */
  startTraining(request: TrainingRequest): Promise<TrainingHandle>;

  /** Subscribe to training progress updates */
  onProgress(callback: (update: ProgressUpdate) => void): Unsubscribe;

  /** Stop training early */
  stopTraining(handle: TrainingHandle): Promise<void>;

  /** Get a specific step's state (for time-travel scrubbing) - lightweight, no geometry */
  getStep(handle: TrainingHandle, stepIndex: number): Promise<StepState>;

  /**
   * Get step with full geometric data (regions, edges, intersection points).
   * Use this for displaying a step; more expensive than getStep().
   */
  getStepWithGeometry(handle: TrainingHandle, stepIndex: number): Promise<StepStateWithGeometry>;

  /** Get trace metadata (BTD indices, total steps, etc.) */
  getTraceInfo(handle: TrainingHandle): Promise<TraceInfo>;

  // ==========================================================================
  // Trace Management (server mode only)
  // ==========================================================================

  /**
   * Load a trace and reconstruct model state for continued training.
   * Server auto-saves the trace on load.
   */
  loadTrace(trace: TraceExport, step?: StepSelector, name?: string): Promise<LoadTraceResult>;

  /**
   * Load a previously saved trace by ID.
   */
  loadSavedTrace(traceId: string, step?: StepSelector): Promise<LoadTraceResult>;

  /**
   * Save the current trace (after training) with an optional name.
   */
  saveTrace(name?: string): Promise<SaveTraceResult>;

  /**
   * List all saved traces for the current session.
   */
  listTraces(): Promise<TraceListResult>;

  /**
   * Rename a saved trace.
   */
  renameTrace(traceId: string, name: string): Promise<RenameTraceResult>;

  /**
   * Delete a saved trace.
   */
  deleteTrace(traceId: string): Promise<DeleteTraceResult>;

  // ==========================================================================
  // Sample Traces (server mode only, requires --samples-dir)
  // ==========================================================================

  /**
   * List available sample traces (from samples directory).
   */
  listSampleTraces(): Promise<SampleTraceListResult>;

  /**
   * Load a sample trace by filename.
   */
  loadSampleTrace(filename: string, step?: StepSelector): Promise<LoadTraceResult>;

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
  /** Expected server SHA for version validation */
  expectedSha?: string;
  /** Behavior on version mismatch: "warn" (default), "error", or "ignore" */
  versionMismatch?: "warn" | "error" | "ignore";
}

/** Server version info returned by getVersion RPC */
export interface VersionInfo {
  sha: string;
  version: string;
}

export type TransportConfig = WorkerTransportConfig | WebSocketTransportConfig;

// ============================================================================
// Worker Message Types (internal)
// ============================================================================

export interface WorkerRequest {
  id: string;
  type: "createModel" | "trainBatch" | "train" | "stop" | "getStep" | "getStepWithGeometry" | "getTraceInfo";
  payload: unknown;
}

export interface WorkerResponse {
  id: string;
  type: "result" | "error" | "progress";
  payload: unknown;
}
