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
// Client Interface
// ============================================================================

export type Unsubscribe = () => void;

export interface TrainingClient {
  /**
   * Create initial model without training (server branch needs this).
   * Returns step 0 with full geometry for initial display.
   */
  createModel(inputs: InputSpec[], targets: TargetsMap): Promise<StepStateWithGeometry>;

  /** Start training with given inputs and targets */
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
  type: "createModel" | "train" | "stop" | "getStep" | "getStepWithGeometry" | "getTraceInfo";
  payload: unknown;
}

export interface WorkerResponse {
  id: string;
  type: "result" | "error" | "progress";
  payload: unknown;
}
