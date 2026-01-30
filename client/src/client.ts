/**
 * TrainingClient implementations for Worker and WebSocket transports.
 *
 * Both transports implement the same TrainingClient interface, allowing
 * the frontend to work identically regardless of the backend.
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
  TransportConfig,
  WorkerRequest,
  WorkerResponse,
  InputSpec,
  TargetsMap,
} from "./types";

// ============================================================================
// Worker Transport
// ============================================================================

export class WorkerTrainingClient implements TrainingClient {
  private worker: Worker;
  private pendingRequests = new Map<string, {
    resolve: (value: unknown) => void;
    reject: (error: Error) => void;
  }>();
  private progressCallbacks = new Set<(update: ProgressUpdate) => void>();
  private requestIdCounter = 0;

  constructor(workerUrl?: string | URL) {
    // Create worker - URL should point to the compiled worker.js
    const url = workerUrl ?? new URL("./worker.js", import.meta.url);
    this.worker = new Worker(url, { type: "module" });

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
}

// ============================================================================
// WebSocket Transport
// ============================================================================

export class WebSocketTrainingClient implements TrainingClient {
  private ws: WebSocket | null = null;
  private url: string;
  private pendingRequests = new Map<string, {
    resolve: (value: unknown) => void;
    reject: (error: Error) => void;
  }>();
  private progressCallbacks = new Set<(update: ProgressUpdate) => void>();
  private requestIdCounter = 0;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private connected = false;
  private connectionPromise: Promise<void> | null = null;

  constructor(url: string) {
    this.url = url;
  }

  private async ensureConnected(): Promise<void> {
    if (this.connected && this.ws?.readyState === WebSocket.OPEN) {
      return;
    }

    if (this.connectionPromise) {
      return this.connectionPromise;
    }

    this.connectionPromise = new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);

      this.ws.onopen = () => {
        this.connected = true;
        this.reconnectAttempts = 0;
        this.connectionPromise = null;
        resolve();
      };

      this.ws.onclose = () => {
        this.connected = false;
        this.connectionPromise = null;
      };

      this.ws.onerror = (event) => {
        console.error("WebSocket error:", event);
        this.connectionPromise = null;
        reject(new Error("WebSocket connection failed"));
      };

      this.ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);

          // Server-initiated progress notification
          if (message.method === "progress") {
            const update = message.params as ProgressUpdate;
            this.progressCallbacks.forEach(cb => cb(update));
            return;
          }

          // Response to a request
          if (message.id) {
            const pending = this.pendingRequests.get(message.id);
            if (pending) {
              this.pendingRequests.delete(message.id);
              if (message.error) {
                pending.reject(new Error(message.error.message));
              } else {
                pending.resolve(message.result);
              }
            }
          }
        } catch (e) {
          console.error("Failed to parse WebSocket message:", e);
        }
      };
    });

    return this.connectionPromise;
  }

  private nextRequestId(): string {
    return `ws_${++this.requestIdCounter}`;
  }

  private async sendRequest<T>(method: string, params: unknown): Promise<T> {
    await this.ensureConnected();

    return new Promise((resolve, reject) => {
      const id = this.nextRequestId();
      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });

      this.ws!.send(JSON.stringify({ id, method, params }));
    });
  }

  async createModel(inputs: InputSpec[], targets: TargetsMap): Promise<StepStateWithGeometry> {
    return this.sendRequest<StepStateWithGeometry>("createModel", { inputs, targets });
  }

  async startTraining(request: TrainingRequest): Promise<TrainingHandle> {
    return this.sendRequest<TrainingHandle>("train", request);
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
    this.ws?.close();
    this.ws = null;
    this.connected = false;
    this.pendingRequests.clear();
    this.progressCallbacks.clear();
  }
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create a TrainingClient with the specified transport.
 *
 * @example
 * // Worker transport (WASM in browser)
 * const client = createTrainingClient({ transport: "worker" });
 *
 * @example
 * // WebSocket transport (native Rust server)
 * const client = createTrainingClient({
 *   transport: "websocket",
 *   url: "ws://localhost:8080"
 * });
 */
export function createTrainingClient(config: TransportConfig): TrainingClient {
  switch (config.transport) {
    case "worker":
      return new WorkerTrainingClient(config.wasmUrl);

    case "websocket":
      return new WebSocketTrainingClient(config.url);

    default:
      throw new Error(`Unknown transport: ${(config as { transport: string }).transport}`);
  }
}
