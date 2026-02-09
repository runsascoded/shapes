/**
 * WebSocketTrainingClient - Training client using WebSocket transport.
 *
 * This client connects to a native Rust server via WebSocket (apvd serve).
 * For WASM/Worker-based training, use the WorkerTrainingClient from apvd-wasm.
 */
import type { TrainingClient, TrainingRequest, TrainingHandle, ProgressUpdate, StepState, StepStateWithGeometry, TraceInfo, Unsubscribe, TransportConfig, InputSpec, TargetsMap, BatchTrainingRequest, BatchTrainingResult, ContinueTrainingResult, TraceExport, StepSelector, LoadTraceResult, SaveTraceResult, TraceListResult, RenameTraceResult, DeleteTraceResult, SampleTraceListResult } from "./types";
export declare class WebSocketTrainingClient implements TrainingClient {
    private ws;
    private url;
    private expectedSha?;
    private versionMismatch;
    private pendingRequests;
    private progressCallbacks;
    private requestIdCounter;
    private reconnectAttempts;
    private maxReconnectAttempts;
    private connected;
    private connectionPromise;
    private versionChecked;
    constructor(url: string, expectedSha?: string, versionMismatch?: "warn" | "error" | "ignore");
    private ensureConnected;
    private checkVersion;
    private nextRequestId;
    private sendRequestRaw;
    private sendRequest;
    createModel(inputs: InputSpec[], targets: TargetsMap): Promise<StepStateWithGeometry>;
    trainBatch(request: BatchTrainingRequest): Promise<BatchTrainingResult>;
    continueTraining(handle: TrainingHandle, numSteps: number): Promise<ContinueTrainingResult>;
    startTraining(request: TrainingRequest): Promise<TrainingHandle>;
    onProgress(callback: (update: ProgressUpdate) => void): Unsubscribe;
    stopTraining(handle: TrainingHandle): Promise<void>;
    getStep(handle: TrainingHandle, stepIndex: number): Promise<StepState>;
    getStepWithGeometry(handle: TrainingHandle, stepIndex: number): Promise<StepStateWithGeometry>;
    getTraceInfo(handle: TrainingHandle): Promise<TraceInfo>;
    loadTrace(trace: TraceExport, step?: StepSelector, name?: string): Promise<LoadTraceResult>;
    loadSavedTrace(traceId: string, step?: StepSelector): Promise<LoadTraceResult>;
    saveTrace(name?: string): Promise<SaveTraceResult>;
    listTraces(): Promise<TraceListResult>;
    renameTrace(traceId: string, name: string): Promise<RenameTraceResult>;
    deleteTrace(traceId: string): Promise<DeleteTraceResult>;
    listSampleTraces(): Promise<SampleTraceListResult>;
    loadSampleTrace(filename: string, step?: StepSelector): Promise<LoadTraceResult>;
    disconnect(): void;
}
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
export declare function createTrainingClient(config: TransportConfig): TrainingClient;
//# sourceMappingURL=client.d.ts.map