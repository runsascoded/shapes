/**
 * WorkerTrainingClient - Training client using Web Worker + WASM.
 *
 * This client runs training in a background worker thread using the
 * @apvd/wasm module for computation.
 */

import type {
  TrainingClient,
  TrainingRequest,
  TrainingHandle,
  ProgressUpdate,
  StepState,
  StepStateWithGeometry,
  TraceInfo,
  Unsubscribe,
  WorkerRequest,
  WorkerResponse,
  InputSpec,
  TargetsMap,
  BatchTrainingRequest,
  BatchTrainingResult,
  ContinueTrainingResult,
} from "@apvd/client";

export class WorkerTrainingClient implements TrainingClient {
  private worker: Worker;
  private pendingRequests = new Map<string, {
    resolve: (value: unknown) => void;
    reject: (error: Error) => void;
  }>();
  private progressCallbacks = new Set<(update: ProgressUpdate) => void>();
  private requestIdCounter = 0;

  constructor(workerOrUrl?: Worker | string | URL) {
    if (workerOrUrl instanceof Worker) {
      this.worker = workerOrUrl;
    } else {
      const url = workerOrUrl ?? new URL("./worker.js", import.meta.url);
      this.worker = new Worker(url, { type: "module" });
    }

    this.worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const { id, type, payload } = event.data;

      if (type === "progress") {
        // Broadcast progress to all listeners
        const update = payload as ProgressUpdate;
        this.progressCallbacks.forEach(cb => cb(update));
        return;
      }

      // Handle request response
      const pending = this.pendingRequests.get(id);
      if (pending) {
        this.pendingRequests.delete(id);
        if (type === "error") {
          pending.reject(new Error((payload as { message: string }).message));
        } else {
          pending.resolve(payload);
        }
      }
    };

    this.worker.onerror = (event) => {
      console.error("Worker error:", event);
    };
  }

  private nextRequestId(): string {
    return `req_${++this.requestIdCounter}`;
  }

  private sendRequest<T>(type: WorkerRequest["type"], payload: unknown): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = this.nextRequestId();
      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      this.worker.postMessage({ id, type, payload } as WorkerRequest);
    });
  }

  async createModel(inputs: InputSpec[], targets: TargetsMap): Promise<StepStateWithGeometry> {
    return this.sendRequest<StepStateWithGeometry>("createModel", { inputs, targets });
  }

  async trainBatch(request: BatchTrainingRequest): Promise<BatchTrainingResult> {
    return this.sendRequest<BatchTrainingResult>("trainBatch", request);
  }

  async continueTraining(handle: TrainingHandle, numSteps: number): Promise<ContinueTrainingResult> {
    return this.sendRequest<ContinueTrainingResult>("continueTraining", { handleId: handle.id, numSteps });
  }

  async startTraining(request: TrainingRequest): Promise<TrainingHandle> {
    const result = await this.sendRequest<{ handle: TrainingHandle }>("train", request);
    return result.handle;
  }

  onProgress(callback: (update: ProgressUpdate) => void): Unsubscribe {
    this.progressCallbacks.add(callback);
    return () => {
      this.progressCallbacks.delete(callback);
    };
  }

  async stopTraining(handle: TrainingHandle): Promise<void> {
    await this.sendRequest("stop", { handleId: handle.id });
  }

  async getStep(handle: TrainingHandle, stepIndex: number): Promise<StepState> {
    return this.sendRequest<StepState>("getStep", { handleId: handle.id, stepIndex });
  }

  async getStepWithGeometry(handle: TrainingHandle, stepIndex: number): Promise<StepStateWithGeometry> {
    return this.sendRequest<StepStateWithGeometry>("getStepWithGeometry", { handleId: handle.id, stepIndex });
  }

  async getTraceInfo(handle: TrainingHandle): Promise<TraceInfo> {
    return this.sendRequest<TraceInfo>("getTraceInfo", { handleId: handle.id });
  }

  disconnect(): void {
    this.worker.terminate();
    this.pendingRequests.clear();
    this.progressCallbacks.clear();
  }

  // Trace persistence stubs (OPFS support planned)
  async loadTrace(): Promise<never> {
    throw new Error("loadTrace not implemented in Worker client - use uploadTrace() for file import");
  }
  async loadSavedTrace(): Promise<never> {
    throw new Error("loadSavedTrace not implemented - OPFS persistence planned");
  }
  async saveTrace(): Promise<never> {
    throw new Error("saveTrace not implemented - OPFS persistence planned");
  }
  async listTraces(): Promise<never> {
    throw new Error("listTraces not implemented - OPFS persistence planned");
  }
  async renameTrace(): Promise<never> {
    throw new Error("renameTrace not implemented - OPFS persistence planned");
  }
  async deleteTrace(): Promise<never> {
    throw new Error("deleteTrace not implemented - OPFS persistence planned");
  }
  async listSampleTraces(): Promise<never> {
    throw new Error("listSampleTraces not implemented in Worker client");
  }
  async loadSampleTrace(): Promise<never> {
    throw new Error("loadSampleTrace not implemented in Worker client");
  }
}

/**
 * Create a WorkerTrainingClient.
 *
 * @param workerUrl - Optional URL to the worker script. Defaults to ./worker.js.
 */
export function createWorkerTrainingClient(workerUrl?: string | URL): WorkerTrainingClient {
  return new WorkerTrainingClient(workerUrl);
}
