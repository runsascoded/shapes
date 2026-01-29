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
  Shape,
  TieredConfig,
} from "./types";

// WASM module will be imported dynamically
let wasm: typeof import("apvd-wasm") | null = null;

// Active training sessions
interface TrainingSession {
  id: string;
  inputs: unknown;
  targets: unknown;
  params: TrainingRequest["params"];
  currentStep: number;
  totalSteps: number;
  minError: number;
  minStep: number;
  startTime: number;
  stopped: boolean;

  // Step history for time-travel (keyframes only if tiered)
  history: Array<{ stepIndex: number; error: number; shapes: unknown }>;
  btdSteps: number[];
  tieredConfig?: TieredConfig;

  // Current WASM step object
  currentWasmStep: unknown;
}

const sessions = new Map<string, TrainingSession>();

// Tiered keyframe helpers
function tier(step: number, b: number): number {
  if (step < 2 * b) return 0;
  return Math.floor(Math.log2(step / b));
}

function resolution(t: number): number {
  return 1 << t;
}

function isKeyframe(step: number, b: number): boolean {
  const t = tier(step, b);
  const res = resolution(t);
  return step % res === 0;
}

function nearestKeyframe(step: number, b: number): number {
  const t = tier(step, b);
  const res = resolution(t);
  return Math.floor(step / res) * res;
}

// Initialize WASM
async function initWasm(): Promise<void> {
  if (wasm) return;

  try {
    // Dynamic import - the actual path will be resolved by the bundler
    wasm = await import("apvd-wasm");
    wasm.init_logs();
  } catch (e) {
    throw new Error(`Failed to load WASM: ${e}`);
  }
}

// Send response to main thread
function respond(response: WorkerResponse): void {
  self.postMessage(response);
}

// Send progress update
function sendProgress(session: TrainingSession, type: "progress" | "complete" | "error", errorMessage?: string, finalResult?: TrainingResult): void {
  const shapes = session.currentWasmStep
    ? (session.currentWasmStep as { shapes: Shape[] }).shapes
    : [];

  const update: ProgressUpdate = {
    handleId: session.id,
    type,
    currentStep: session.currentStep,
    totalSteps: session.totalSteps,
    error: session.currentWasmStep ? (session.currentWasmStep as { error: { v: number } }).error.v : Infinity,
    minError: session.minError,
    minStep: session.minStep,
    shapes,
    elapsedMs: Date.now() - session.startTime,
    ...(finalResult && { finalResult }),
    ...(errorMessage && { errorMessage }),
  };

  respond({ id: session.id, type: "progress", payload: update });
}

// Handle training request
async function handleTrain(id: string, request: TrainingRequest): Promise<void> {
  await initWasm();

  const params = request.params ?? {};
  const maxSteps = params.maxSteps ?? 10000;
  const learningRate = params.learningRate ?? 0.05;
  const convergenceThreshold = params.convergenceThreshold ?? 1e-10;
  const progressInterval = params.progressInterval ?? 100;
  const bucketSize = 1024; // Default tiered bucket size

  // Create session
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
    // Create initial step
    session.currentWasmStep = wasm!.make_step(request.inputs, request.targets);
    let currentError = (session.currentWasmStep as { error: { v: number } }).error.v;
    session.minError = currentError;

    // Store initial step (always a keyframe)
    session.history.push({
      stepIndex: 0,
      error: currentError,
      shapes: JSON.parse(JSON.stringify((session.currentWasmStep as { shapes: Shape[] }).shapes)),
    });
    session.btdSteps.push(0);

    // Send initial progress
    sendProgress(session, "progress");

    // Training loop
    for (let step = 1; step <= maxSteps && !session.stopped; step++) {
      // Take a step
      session.currentWasmStep = wasm!.step(session.currentWasmStep, learningRate);
      session.currentStep = step;
      currentError = (session.currentWasmStep as { error: { v: number } }).error.v;

      // Track BTD (best to date)
      if (currentError < session.minError) {
        session.minError = currentError;
        session.minStep = step;
        session.btdSteps.push(step);
      }

      // Store keyframe if tiered storage says so
      if (isKeyframe(step, bucketSize) || step === maxSteps) {
        session.history.push({
          stepIndex: step,
          error: currentError,
          shapes: JSON.parse(JSON.stringify((session.currentWasmStep as { shapes: Shape[] }).shapes)),
        });
      }

      // Send progress update at intervals or on BTD improvement
      if (step % progressInterval === 0 || currentError < session.minError || step === maxSteps) {
        sendProgress(session, "progress");
      }

      // Check convergence
      if (currentError < convergenceThreshold) {
        break;
      }

      // Yield to event loop periodically to allow stop messages
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

    // Get shapes at min step (may need to retrieve from history)
    const minStepEntry = session.history.find(h => h.stepIndex === session.minStep)
      || session.history.find(h => h.stepIndex === nearestKeyframe(session.minStep, bucketSize));
    const bestShapes = minStepEntry
      ? minStepEntry.shapes as Shape[]
      : (session.currentWasmStep as { shapes: Shape[] }).shapes;

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
    respond({ id, type: "result", payload: { handle: { id, startedAt: session.startTime } } });

  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : String(e);
    sendProgress(session, "error", errorMessage);
    respond({ id, type: "error", payload: { message: errorMessage } });
  }
}

// Handle stop request
function handleStop(id: string, handleId: string): void {
  const session = sessions.get(handleId);
  if (session) {
    session.stopped = true;
  }
  respond({ id, type: "result", payload: { stopped: true } });
}

// Handle getStep request (for time-travel)
async function handleGetStep(id: string, handleId: string, stepIndex: number): Promise<void> {
  await initWasm();

  const session = sessions.get(handleId);
  if (!session) {
    respond({ id, type: "error", payload: { message: `Session ${handleId} not found` } });
    return;
  }

  const bucketSize = session.tieredConfig?.bucketSize ?? 1024;

  // Check if we have this exact step in history
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

  // Find nearest keyframe and recompute
  const kf = nearestKeyframe(stepIndex, bucketSize);
  const keyframeEntry = session.history.find(h => h.stepIndex === kf);

  if (!keyframeEntry) {
    respond({ id, type: "error", payload: { message: `No keyframe found for step ${stepIndex}` } });
    return;
  }

  // Recompute from keyframe
  try {
    let wasmStep = wasm!.make_step(
      keyframeEntry.shapes.map((s: unknown) => [s, Array((s as Shape).kind === "Circle" ? 3 : (s as Shape).kind === "XYRR" ? 4 : 5).fill(true)]),
      session.targets
    );

    const learningRate = session.params?.learningRate ?? 0.05;
    for (let i = keyframeEntry.stepIndex; i < stepIndex; i++) {
      wasmStep = wasm!.step(wasmStep, learningRate);
    }

    const state: StepState = {
      stepIndex,
      error: (wasmStep as { error: { v: number } }).error.v,
      shapes: (wasmStep as { shapes: Shape[] }).shapes,
      isKeyframe: false,
      recomputedFrom: kf,
    };
    respond({ id, type: "result", payload: state });

  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : String(e);
    respond({ id, type: "error", payload: { message: errorMessage } });
  }
}

// Handle getTraceInfo request
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

// Message handler
self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const { id, type, payload } = event.data;

  switch (type) {
    case "train":
      await handleTrain(id, payload as TrainingRequest);
      break;

    case "stop":
      handleStop(id, (payload as { handleId: string }).handleId);
      break;

    case "getStep":
      await handleGetStep(id, (payload as { handleId: string; stepIndex: number }).handleId, (payload as { handleId: string; stepIndex: number }).stepIndex);
      break;

    case "getTraceInfo":
      handleGetTraceInfo(id, (payload as { handleId: string }).handleId);
      break;

    default:
      respond({ id, type: "error", payload: { message: `Unknown request type: ${type}` } });
  }
};

// Export for type checking (not actually used at runtime)
export {};
