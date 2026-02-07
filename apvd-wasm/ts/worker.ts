/**
 * Web Worker that wraps WASM training.
 *
 * This worker handles training in a background thread, streaming progress
 * updates back to the main thread via postMessage.
 *
 * Usage:
 *   const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
 *   worker.postMessage({ id: '1', type: 'train', payload: { inputs, targets, params } });
 */

import type {
  WorkerRequest,
  WorkerResponse,
  TrainingRequest,
  ProgressUpdate,
  TrainingResult,
  TraceInfo,
  StepState,
  StepStateWithGeometry,
  StepGeometry,
  Shape,
  InputSpec,
  TargetsMap,
  TieredConfig,
  BatchTrainingRequest,
  BatchTrainingResult,
  BatchStep,
  ContinueTrainingResult,
  SparklineData,
} from "@apvd/client";

// WASM module will be imported dynamically
let wasm: typeof import("@apvd/wasm") | null = null;

// Active training sessions
interface TrainingSession {
  id: string;
  inputs: InputSpec[];
  targets: TargetsMap;
  params: TrainingRequest["params"];
  currentStep: number;
  totalSteps: number;
  minError: number;
  minStep: number;
  startTime: number;
  stopped: boolean;

  // Step history for time-travel (keyframes only if tiered)
  history: Array<{ stepIndex: number; error: number; shapes: Shape[] }>;
  btdSteps: number[];
  tieredConfig?: TieredConfig;

  // Current WASM step object
  currentWasmStep: unknown;
}

const sessions = new Map<string, TrainingSession>();

// ============================================================================
// Dual number extraction (WASM returns Shape<Dual> where coordinates are {v, d})
// ============================================================================

/**
 * Extracts plain JS number from a WASM value that may be wrapped in Dual { v: number }.
 */
export function extractNumber(val: unknown): number {
  if (typeof val === "number") return val;
  if (val && typeof val === "object" && "v" in val) return (val as { v: number }).v;
  throw new Error(`Cannot extract number from ${JSON.stringify(val)}`);
}

export function extractPoint(pt: unknown): { x: number; y: number } {
  const p = pt as { x: unknown; y: unknown };
  return { x: extractNumber(p.x), y: extractNumber(p.y) };
}

/**
 * Converts WASM Shape<Dual> to plain Shape<number> by extracting values.
 */
export function extractShape(wasmShape: unknown): Shape {
  const s = wasmShape as { kind: string; c?: unknown; r?: unknown; t?: unknown; vertices?: unknown[] };
  if (s.kind === "Circle") {
    return {
      kind: "Circle",
      c: extractPoint(s.c),
      r: extractNumber(s.r),
    };
  } else if (s.kind === "XYRR") {
    return {
      kind: "XYRR",
      c: extractPoint(s.c),
      r: extractPoint(s.r),
    };
  } else if (s.kind === "XYRRT") {
    return {
      kind: "XYRRT",
      c: extractPoint(s.c),
      r: extractPoint(s.r),
      t: extractNumber(s.t),
    };
  } else {
    // Polygon
    const vertices = (s.vertices ?? []).map(v => extractPoint(v));
    return { kind: "Polygon", vertices };
  }
}

export function extractShapes(wasmShapes: unknown[]): Shape[] {
  return wasmShapes.map(s => extractShape(s));
}

// ============================================================================
// Sparkline data extraction
// ============================================================================

function extractSparklineData(
  modelSteps: unknown[],
  startIndex: number,
): { gradients: number[][]; regionErrors: Record<string, number[]> } {
  const gradients: number[][] = [];
  const regionErrors: Record<string, number[]> = {};

  for (let i = startIndex; i < modelSteps.length; i++) {
    const wasmStep = modelSteps[i] as {
      error: { v: number; d?: number[] };
      errors: Map<string, { error: { v: number } }> | Record<string, { error: { v: number } }>;
    };

    gradients.push(wasmStep.error.d || []);

    const errors = wasmStep.errors;
    if (errors) {
      const errorEntries = errors instanceof Map ? errors.entries() : Object.entries(errors);
      for (const [regionKey, regionErr] of errorEntries) {
        if (!regionErrors[regionKey]) {
          regionErrors[regionKey] = [];
        }
        while (regionErrors[regionKey].length < i - startIndex) {
          regionErrors[regionKey].push(0);
        }
        regionErrors[regionKey].push((regionErr as { error: { v: number } }).error.v);
      }
    }
  }

  return { gradients, regionErrors };
}

// ============================================================================
// Tiered keyframe helpers
// ============================================================================

export function tier(step: number, b: number): number {
  if (step < 2 * b) return 0;
  return Math.floor(Math.log2(step / b));
}

export function resolution(t: number): number {
  return 1 << t;
}

export function isKeyframe(step: number, b: number): boolean {
  const t = tier(step, b);
  const res = resolution(t);
  return step % res === 0;
}

export function nearestKeyframe(step: number, b: number): number {
  const t = tier(step, b);
  const res = resolution(t);
  return Math.floor(step / res) * res;
}

// ============================================================================
// WASM initialization
// ============================================================================

async function initWasm(): Promise<void> {
  if (wasm) return;

  try {
    // Dynamic import - the actual path will be resolved by the bundler
    wasm = await import("@apvd/wasm");
    // Call the WASM init function (default export) before using any exports
    await (wasm as unknown as { default?: () => Promise<unknown> }).default?.();
    wasm.init_logs();
  } catch (e) {
    throw new Error(`Failed to load WASM: ${e}`);
  }
}

function getWasm(): any {
  if (!wasm) throw new Error("WASM not initialized");
  return wasm;
}

// ============================================================================
// Worker message helpers
// ============================================================================

function respond(response: WorkerResponse): void {
  self.postMessage(response);
}

function sendProgress(
  session: TrainingSession,
  type: "progress" | "complete" | "error",
  errorMessage?: string,
  finalResult?: TrainingResult,
): void {
  const shapes = session.currentWasmStep
    ? extractShapes((session.currentWasmStep as { shapes: unknown[] }).shapes)
    : [];

  const update: ProgressUpdate = {
    handleId: session.id,
    type,
    currentStep: session.currentStep,
    totalSteps: session.totalSteps,
    error: session.currentWasmStep
      ? (session.currentWasmStep as { error: { v: number } }).error.v
      : Infinity,
    minError: session.minError,
    minStep: session.minStep,
    shapes,
    elapsedMs: Date.now() - session.startTime,
    ...(finalResult && { finalResult }),
    ...(errorMessage && { errorMessage }),
  };

  respond({ id: session.id, type: "progress", payload: update });
}

// ============================================================================
// Request handlers
// ============================================================================

async function handleTrain(id: string, request: TrainingRequest): Promise<void> {
  await initWasm();
  const apvd = getWasm();

  const params = request.params ?? {};
  const maxSteps = params.maxSteps ?? 10000;
  const learningRate = params.learningRate ?? 0.5;
  const convergenceThreshold = params.convergenceThreshold ?? 1e-10;
  const progressInterval = params.progressInterval ?? 100;
  const bucketSize = 1024;

  const session: TrainingSession = {
    id,
    inputs: request.inputs,
    targets: request.targets,
    params,
    currentStep: 0,
    totalSteps: maxSteps,
    minError: Infinity,
    minStep: 0,
    startTime: Date.now(),
    stopped: false,
    history: [],
    btdSteps: [],
    tieredConfig: { bucketSize },
    currentWasmStep: null,
  };
  sessions.set(id, session);

  try {
    session.currentWasmStep = apvd.make_step(request.inputs, request.targets);
    let currentError = (session.currentWasmStep as { error: { v: number } }).error.v;
    session.minError = currentError;

    // Store initial step (always a keyframe)
    session.history.push({
      stepIndex: 0,
      error: currentError,
      shapes: extractShapes((session.currentWasmStep as { shapes: unknown[] }).shapes),
    });
    session.btdSteps.push(0);

    sendProgress(session, "progress");

    // Return handle immediately so client can start querying steps
    respond({ id, type: "result", payload: { handle: { id, startedAt: session.startTime } } });

    // Training loop (runs asynchronously after returning handle)
    for (let step = 1; step <= maxSteps && !session.stopped; step++) {
      // Error-scaled stepping: step_size = current_error * learningRate
      const prevError = (session.currentWasmStep as { error: { v: number } }).error.v;
      const scaledStepSize = prevError * learningRate;
      session.currentWasmStep = apvd.step(session.currentWasmStep, scaledStepSize);
      session.currentStep = step;
      currentError = (session.currentWasmStep as { error: { v: number } }).error.v;

      if (currentError < session.minError) {
        session.minError = currentError;
        session.minStep = step;
        session.btdSteps.push(step);
      }

      if (isKeyframe(step, bucketSize) || step === maxSteps) {
        session.history.push({
          stepIndex: step,
          error: currentError,
          shapes: extractShapes((session.currentWasmStep as { shapes: unknown[] }).shapes),
        });
      }

      if (step % progressInterval === 0 || currentError < session.minError || step === maxSteps) {
        sendProgress(session, "progress");
      }

      if (currentError < convergenceThreshold) {
        break;
      }

      if (step % 100 === 0) {
        await new Promise(resolve => setTimeout(resolve, 0));
      }
    }

    // Training complete
    const traceInfo: TraceInfo = {
      totalSteps: session.currentStep,
      btdSteps: session.btdSteps,
      tiered: session.tieredConfig,
    };

    const minStepEntry = session.history.find(h => h.stepIndex === session.minStep)
      || session.history.find(h => h.stepIndex === nearestKeyframe(session.minStep, bucketSize));
    const bestShapes = minStepEntry
      ? minStepEntry.shapes as Shape[]
      : extractShapes((session.currentWasmStep as { shapes: unknown[] }).shapes);

    const finalResult: TrainingResult = {
      success: true,
      finalError: (session.currentWasmStep as { error: { v: number } }).error.v,
      minError: session.minError,
      minStep: session.minStep,
      totalSteps: session.currentStep,
      trainingTimeMs: Date.now() - session.startTime,
      shapes: bestShapes,
      traceInfo,
    };

    sendProgress(session, "complete", undefined, finalResult);
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : String(e);
    sendProgress(session, "error", errorMessage);
    // Only respond with error if we haven't already responded with handle
    if (!session.currentWasmStep) {
      respond({ id, type: "error", payload: { message: errorMessage } });
    }
  }
}

function handleStop(id: string, handleId: string): void {
  const session = sessions.get(handleId);
  if (session) {
    session.stopped = true;
  }
  respond({ id, type: "result", payload: { stopped: true } });
}

async function handleGetStep(id: string, handleId: string, stepIndex: number): Promise<void> {
  await initWasm();
  const apvd = getWasm();

  const session = sessions.get(handleId);
  if (!session) {
    respond({ id, type: "error", payload: { message: `Session ${handleId} not found` } });
    return;
  }

  const bucketSize = session.tieredConfig?.bucketSize ?? 1024;

  const exactEntry = session.history.find(h => h.stepIndex === stepIndex);
  if (exactEntry) {
    const state: StepState = {
      stepIndex: exactEntry.stepIndex,
      error: exactEntry.error,
      shapes: exactEntry.shapes as Shape[],
      isKeyframe: true,
    };
    respond({ id, type: "result", payload: state });
    return;
  }

  const kf = nearestKeyframe(stepIndex, bucketSize);
  const keyframeEntry = session.history.find(h => h.stepIndex === kf);

  if (!keyframeEntry) {
    respond({ id, type: "error", payload: { message: `No keyframe found for step ${stepIndex}` } });
    return;
  }

  try {
    let wasmStep = apvd.make_step(
      keyframeEntry.shapes.map((s: unknown) => [s, Array((s as Shape).kind === "Circle" ? 3 : (s as Shape).kind === "XYRR" ? 4 : 5).fill(true)]),
      session.targets
    );

    const learningRate = session.params?.learningRate ?? 0.5;
    for (let i = keyframeEntry.stepIndex; i < stepIndex; i++) {
      const prevError = (wasmStep as { error: { v: number } }).error.v;
      const scaledStepSize = prevError * learningRate;
      wasmStep = apvd.step(wasmStep, scaledStepSize);
    }

    const state: StepState = {
      stepIndex,
      error: (wasmStep as { error: { v: number } }).error.v,
      shapes: extractShapes((wasmStep as { shapes: unknown[] }).shapes),
      isKeyframe: false,
      recomputedFrom: kf,
    };
    respond({ id, type: "result", payload: state });
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : String(e);
    respond({ id, type: "error", payload: { message: errorMessage } });
  }
}

function handleGetTraceInfo(id: string, handleId: string): void {
  const session = sessions.get(handleId);
  if (!session) {
    respond({ id, type: "error", payload: { message: `Session ${handleId} not found` } });
    return;
  }

  const traceInfo: TraceInfo = {
    totalSteps: session.currentStep,
    btdSteps: session.btdSteps,
    tiered: session.tieredConfig,
  };
  respond({ id, type: "result", payload: traceInfo });
}

/**
 * Extract geometry from a WASM step object.
 */
function extractGeometry(wasmStep: unknown, targets: unknown): StepGeometry {
  const step = wasmStep as {
    shapes: Shape[];
    regions?: Array<{ key: string; area: { v: number }; edges?: unknown[] }>;
    points?: Array<{ p: { x: number; y: number }; shape0: number; shape1: number; theta0: number; theta1: number }>;
    components?: Array<{ key: string; points: unknown[]; edges: unknown[]; regions: unknown[] }>;
    total_area?: { v: number };
    error: { v: number };
  };

  const regions = (step.regions ?? []).map(r => ({
    key: r.key,
    area: r.area?.v ?? 0,
    edges: [],
  }));

  const points = (step.points ?? []).map(p => ({
    p: p.p,
    shape0: p.shape0,
    shape1: p.shape1,
    theta0: p.theta0,
    theta1: p.theta1,
  }));

  const components = (step.components ?? []).map(c => ({
    key: c.key,
    points: c.points as never[],
    edges: c.edges as never[],
    regions: c.regions as never[],
  }));

  const targetMap = targets as TargetsMap;
  const errors: Record<string, { actual: number; target: number; delta: number; errorContribution: number }> = {};

  for (const region of regions) {
    const target = targetMap[region.key] ?? 0;
    const actual = region.area;
    const delta = actual - target;
    errors[region.key] = {
      actual,
      target,
      delta,
      errorContribution: delta * delta,
    };
  }

  return {
    components,
    totalArea: step.total_area?.v ?? 0,
    errors,
    points,
    regions,
  };
}

async function handleCreateModel(id: string, inputs: InputSpec[], targets: TargetsMap): Promise<void> {
  await initWasm();
  const apvd = getWasm();

  try {
    const wasmStep = apvd.make_step(inputs, targets);

    const result = {
      stepIndex: 0,
      isKeyframe: true,
      raw: wasmStep,
    };

    respond({ id, type: "result", payload: result });
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : String(e);
    respond({ id, type: "error", payload: { message: errorMessage } });
  }
}

// Handle trainBatch request - stateless batch computation using train()
// Uses error-scaled stepping which matches main branch behavior
async function handleTrainBatch(id: string, request: BatchTrainingRequest): Promise<void> {
  await initWasm();
  const apvd = getWasm();

  const { inputs, targets, numSteps, learningRate = 0.5 } = request;

  try {
    // Create model and train with error-scaled stepping
    const wasmModel = apvd.make_model(inputs, targets);
    const trainedModel = apvd.train(wasmModel, learningRate, numSteps);

    const modelSteps = (trainedModel as { steps: unknown[] }).steps;
    const minIdx = (trainedModel as { min_idx: number }).min_idx;
    const minError = (trainedModel as { min_error: number }).min_error;

    const steps: BatchStep[] = modelSteps.map((wasmStep: unknown, i: number) => ({
      stepIndex: i,
      error: (wasmStep as { error: { v: number } }).error.v,
      shapes: extractShapes((wasmStep as { shapes: unknown[] }).shapes),
    }));

    // Extract sparkline data from all steps
    const { gradients, regionErrors } = extractSparklineData(modelSteps, 0);

    const result: BatchTrainingResult = {
      steps,
      minError,
      minStepIndex: minIdx,
      finalShapes: steps[steps.length - 1].shapes,
      sparklineData: {
        errors: steps.map(s => s.error),
        gradients,
        regionErrors,
      },
    };

    respond({ id, type: "result", payload: result });
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : String(e);
    respond({ id, type: "error", payload: { message: errorMessage } });
  }
}

// Handle continueTraining - continue a session for more steps
// Uses train() for batch computation with error-scaled stepping
async function handleContinueTraining(id: string, handleId: string, numSteps: number): Promise<void> {
  await initWasm();
  const apvd = getWasm();

  const session = sessions.get(handleId);
  if (!session) {
    respond({ id, type: "error", payload: { message: `Session ${handleId} not found` } });
    return;
  }

  const learningRate = session.params?.learningRate ?? 0.5;
  const bucketSize = session.tieredConfig?.bucketSize ?? 1024;
  const startStep = session.currentStep;

  try {
    // Create a seed model from the last step
    const lastStep = session.currentWasmStep;
    const batchSeed = {
      steps: [lastStep],
      repeat_idx: null,
      min_idx: 0,
      min_error: (lastStep as { error: { v: number } }).error.v,
    };

    // Call train() to compute the batch - handles error-scaled stepping
    const trainedModel = apvd.train(batchSeed, learningRate, numSteps);
    const modelSteps = (trainedModel as { steps: unknown[] }).steps;
    const batchMinIdx = (trainedModel as { min_idx: number }).min_idx;
    const batchMinError = (trainedModel as { min_error: number }).min_error;

    // Collect per-step data (skip first step as it duplicates last)
    const batchSteps: Array<{ stepIndex: number; error: number; shapes: Shape[] }> = [];

    // Extract sparkline data (starting from index 1, skipping duplicate)
    const { gradients: sparklineGradients, regionErrors: sparklineRegionErrors } =
      extractSparklineData(modelSteps, 1);

    for (let i = 1; i < modelSteps.length; i++) {
      const wasmStep = modelSteps[i] as { error: { v: number }; shapes: unknown[] };
      const stepIndex = startStep + i;
      const error = wasmStep.error.v;
      const shapes = extractShapes(wasmStep.shapes);

      batchSteps.push({ stepIndex, error, shapes });

      // Store keyframe in session history if tiered storage says so
      if (isKeyframe(stepIndex, bucketSize)) {
        session.history.push({ stepIndex, error, shapes });
      }
    }

    // Update session state with final step
    const finalStep = modelSteps[modelSteps.length - 1];
    session.currentWasmStep = finalStep;
    session.currentStep = startStep + modelSteps.length - 1;

    // Update best step tracking
    const absoluteBatchMinStep = startStep + batchMinIdx;
    if (batchMinError < session.minError) {
      session.minError = batchMinError;
      session.minStep = absoluteBatchMinStep;
      session.btdSteps.push(absoluteBatchMinStep);
    }

    const currentError = (finalStep as { error: { v: number } }).error.v;
    const currentShapes = extractShapes((finalStep as { shapes: unknown[] }).shapes);

    const result: ContinueTrainingResult = {
      totalSteps: session.currentStep,
      currentStep: session.currentStep,
      minError: session.minError,
      minStep: session.minStep,
      currentShapes,
      currentError,
      steps: batchSteps,
      sparklineData: {
        errors: batchSteps.map(s => s.error),
        gradients: sparklineGradients,
        regionErrors: sparklineRegionErrors,
      },
    };

    respond({ id, type: "result", payload: result });
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : String(e);
    respond({ id, type: "error", payload: { message: errorMessage } });
  }
}

// Handle getStepWithGeometry request
// Returns shapes and targets so the main thread can call make_step
// (WASM Step objects don't serialize properly through postMessage)
async function handleGetStepWithGeometry(id: string, handleId: string, stepIndex: number): Promise<void> {
  await initWasm();
  const apvd = getWasm();

  const session = sessions.get(handleId);
  if (!session) {
    respond({ id, type: "error", payload: { message: `Session ${handleId} not found` } });
    return;
  }

  const bucketSize = session.tieredConfig?.bucketSize ?? 1024;

  try {
    let shapes: Shape[];
    let error: number;
    let isKf = false;
    let recomputedFrom: number | undefined;

    const exactEntry = session.history.find(h => h.stepIndex === stepIndex);
    if (exactEntry) {
      shapes = exactEntry.shapes as Shape[];
      error = exactEntry.error;
      isKf = true;
    } else {
      const kf = nearestKeyframe(stepIndex, bucketSize);
      const keyframeEntry = session.history.find(h => h.stepIndex === kf);

      if (!keyframeEntry) {
        respond({ id, type: "error", payload: { message: `No keyframe found for step ${stepIndex}` } });
        return;
      }

      let wasmStep = apvd.make_step(
        (keyframeEntry.shapes as Shape[]).map((s: Shape) => [s, Array(s.kind === "Circle" ? 3 : s.kind === "XYRR" ? 4 : 5).fill(true)]),
        session.targets
      );

      const lr = session.params?.learningRate ?? 0.5;
      for (let i = keyframeEntry.stepIndex; i < stepIndex; i++) {
        const prevError = (wasmStep as { error: { v: number } }).error.v;
        const scaledStepSize = prevError * lr;
        wasmStep = apvd.step(wasmStep, scaledStepSize);
      }

      shapes = extractShapes((wasmStep as { shapes: unknown[] }).shapes);
      error = (wasmStep as { error: { v: number } }).error.v;
      recomputedFrom = kf;
    }

    // Return shapes and targets so main thread can call make_step
    const result = {
      stepIndex,
      isKeyframe: isKf,
      ...(recomputedFrom !== undefined && { recomputedFrom }),
      shapes,
      error,
      targets: session.targets,
      inputs: session.inputs,
    };

    respond({ id, type: "result", payload: result });
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : String(e);
    respond({ id, type: "error", payload: { message: errorMessage } });
  }
}

// ============================================================================
// Message handler
// ============================================================================

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const { id, type, payload } = event.data;

  try {
    switch (type) {
      case "createModel": {
        const { inputs, targets } = payload as { inputs: InputSpec[]; targets: TargetsMap };
        await handleCreateModel(id, inputs, targets);
        break;
      }
      case "train":
        await handleTrain(id, payload as TrainingRequest);
        break;
      case "trainBatch":
        await handleTrainBatch(id, payload as BatchTrainingRequest);
        break;
      case "continueTraining": {
        const { handleId, numSteps } = payload as { handleId: string; numSteps: number };
        await handleContinueTraining(id, handleId, numSteps);
        break;
      }
      case "stop": {
        const { handleId } = payload as { handleId: string };
        handleStop(id, handleId);
        break;
      }
      case "getStep": {
        const { handleId, stepIndex } = payload as { handleId: string; stepIndex: number };
        await handleGetStep(id, handleId, stepIndex);
        break;
      }
      case "getStepWithGeometry": {
        const { handleId, stepIndex } = payload as { handleId: string; stepIndex: number };
        await handleGetStepWithGeometry(id, handleId, stepIndex);
        break;
      }
      case "getTraceInfo": {
        const { handleId } = payload as { handleId: string };
        handleGetTraceInfo(id, handleId);
        break;
      }
      default:
        respond({ id, type: "error", payload: { message: `Unknown request type: ${type}` } });
    }
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : String(e);
    respond({ id, type: "error", payload: { message: errorMessage } });
  }
};

// Exported functions (extractNumber, extractPoint, extractShape, extractShapes,
// tier, resolution, isKeyframe, nearestKeyframe) are used by @apvd/worker/index.ts
// and downstream consumers. The `self.onmessage` handler above runs in worker context.
