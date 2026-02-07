/**
 * WebSocketTrainingClient - Training client using WebSocket transport.
 *
 * This client connects to a native Rust server via WebSocket (apvd serve).
 * For WASM/Worker-based training, use the WorkerTrainingClient from apvd-wasm.
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
  InputSpec,
  TargetsMap,
  BatchTrainingRequest,
  BatchTrainingResult,
  TraceExport,
  StepSelector,
  LoadTraceResult,
  SaveTraceResult,
  TraceListResult,
  RenameTraceResult,
  DeleteTraceResult,
  SampleTraceListResult,
} from "./types";

// ============================================================================
// WebSocket Transport
// ============================================================================

export class WebSocketTrainingClient implements TrainingClient {
  private ws: WebSocket | null = null;
  private url: string;
  private expectedSha?: string;
  private versionMismatch: "warn" | "error" | "ignore";
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
  private versionChecked = false;

  constructor(url: string, expectedSha?: string, versionMismatch?: "warn" | "error" | "ignore") {
    this.url = url;
    this.expectedSha = expectedSha;
    this.versionMismatch = versionMismatch ?? "warn";
  }

  private async ensureConnected(): Promise<void> {
    if (this.connected && this.ws?.readyState === WebSocket.OPEN) {
      // Check version on first use after connect
      if (!this.versionChecked && this.expectedSha) {
        await this.checkVersion();
      }
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

  private async checkVersion(): Promise<void> {
    if (this.versionChecked || !this.expectedSha || this.versionMismatch === "ignore") {
      this.versionChecked = true;
      return;
    }

    try {
      const version = await this.sendRequestRaw<{ sha: string; version: string }>("getVersion", {});
      this.versionChecked = true;

      if (version.sha !== this.expectedSha) {
        const msg = `Version mismatch: client expects ${this.expectedSha}, server has ${version.sha}`;
        if (this.versionMismatch === "error") {
          throw new Error(msg);
        } else {
          console.warn(msg);
        }
      }
    } catch (e) {
      // If getVersion fails (old server), just warn and continue
      console.warn("Could not verify server version:", e);
      this.versionChecked = true;
    }
  }

  private nextRequestId(): string {
    return `ws_${++this.requestIdCounter}`;
  }

  // Raw request without version check (used by checkVersion itself)
  private async sendRequestRaw<T>(method: string, params: unknown): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = this.nextRequestId();
      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      this.ws!.send(JSON.stringify({ id, method, params }));
    });
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

  async trainBatch(request: BatchTrainingRequest): Promise<BatchTrainingResult> {
    return this.sendRequest<BatchTrainingResult>("trainBatch", request);
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

  // ==========================================================================
  // Trace Management
  // ==========================================================================

  async loadTrace(trace: TraceExport, step: StepSelector = "best", name?: string): Promise<LoadTraceResult> {
    return this.sendRequest<LoadTraceResult>("loadTrace", { trace, step, name });
  }

  async loadSavedTrace(traceId: string, step: StepSelector = "best"): Promise<LoadTraceResult> {
    return this.sendRequest<LoadTraceResult>("loadSavedTrace", { traceId, step });
  }

  async saveTrace(name?: string): Promise<SaveTraceResult> {
    return this.sendRequest<SaveTraceResult>("saveTrace", { name });
  }

  async listTraces(): Promise<TraceListResult> {
    return this.sendRequest<TraceListResult>("listTraces", {});
  }

  async renameTrace(traceId: string, name: string): Promise<RenameTraceResult> {
    return this.sendRequest<RenameTraceResult>("renameTrace", { traceId, name });
  }

  async deleteTrace(traceId: string): Promise<DeleteTraceResult> {
    return this.sendRequest<DeleteTraceResult>("deleteTrace", { traceId });
  }

  // ==========================================================================
  // Sample Traces
  // ==========================================================================

  async listSampleTraces(): Promise<SampleTraceListResult> {
    return this.sendRequest<SampleTraceListResult>("listSampleTraces", {});
  }

  async loadSampleTrace(filename: string, step: StepSelector = "best"): Promise<LoadTraceResult> {
    return this.sendRequest<LoadTraceResult>("loadSampleTrace", { filename, step });
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
 * // WebSocket transport (native Rust server)
 * const client = createTrainingClient({
 *   transport: "websocket",
 *   url: "ws://localhost:8080"
 * });
 *
 * Note: For Worker transport (WASM in browser), use createWorkerTrainingClient
 * from the @apvd/worker package instead.
 */
export function createTrainingClient(config: TransportConfig): TrainingClient {
  switch (config.transport) {
    case "worker":
      throw new Error(
        "Worker transport is not available in @apvd/client. " +
        "Use createWorkerTrainingClient from @apvd/worker instead."
      );

    case "websocket":
      return new WebSocketTrainingClient(config.url, config.expectedSha, config.versionMismatch);

    default:
      throw new Error(`Unknown transport: ${(config as { transport: string }).transport}`);
  }
}
