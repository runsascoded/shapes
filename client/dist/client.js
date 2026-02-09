/**
 * WebSocketTrainingClient - Training client using WebSocket transport.
 *
 * This client connects to a native Rust server via WebSocket (apvd serve).
 * For WASM/Worker-based training, use the WorkerTrainingClient from apvd-wasm.
 */
// ============================================================================
// WebSocket Transport
// ============================================================================
export class WebSocketTrainingClient {
    constructor(url, expectedSha, versionMismatch) {
        this.ws = null;
        this.pendingRequests = new Map();
        this.progressCallbacks = new Set();
        this.requestIdCounter = 0;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.connected = false;
        this.connectionPromise = null;
        this.versionChecked = false;
        this.url = url;
        this.expectedSha = expectedSha;
        this.versionMismatch = versionMismatch ?? "warn";
    }
    async ensureConnected() {
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
                        const update = message.params;
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
                            }
                            else {
                                pending.resolve(message.result);
                            }
                        }
                    }
                }
                catch (e) {
                    console.error("Failed to parse WebSocket message:", e);
                }
            };
        });
        return this.connectionPromise;
    }
    async checkVersion() {
        if (this.versionChecked || !this.expectedSha || this.versionMismatch === "ignore") {
            this.versionChecked = true;
            return;
        }
        try {
            const version = await this.sendRequestRaw("getVersion", {});
            this.versionChecked = true;
            if (version.sha !== this.expectedSha) {
                const msg = `Version mismatch: client expects ${this.expectedSha}, server has ${version.sha}`;
                if (this.versionMismatch === "error") {
                    throw new Error(msg);
                }
                else {
                    console.warn(msg);
                }
            }
        }
        catch (e) {
            // If getVersion fails (old server), just warn and continue
            console.warn("Could not verify server version:", e);
            this.versionChecked = true;
        }
    }
    nextRequestId() {
        return `ws_${++this.requestIdCounter}`;
    }
    // Raw request without version check (used by checkVersion itself)
    async sendRequestRaw(method, params) {
        return new Promise((resolve, reject) => {
            const id = this.nextRequestId();
            this.pendingRequests.set(id, {
                resolve: resolve,
                reject,
            });
            this.ws.send(JSON.stringify({ id, method, params }));
        });
    }
    async sendRequest(method, params) {
        await this.ensureConnected();
        return new Promise((resolve, reject) => {
            const id = this.nextRequestId();
            this.pendingRequests.set(id, {
                resolve: resolve,
                reject,
            });
            this.ws.send(JSON.stringify({ id, method, params }));
        });
    }
    async createModel(inputs, targets) {
        return this.sendRequest("createModel", { inputs, targets });
    }
    async trainBatch(request) {
        return this.sendRequest("trainBatch", request);
    }
    async continueTraining(handle, numSteps) {
        return this.sendRequest("continueTraining", { handleId: handle.id, numSteps });
    }
    async startTraining(request) {
        return this.sendRequest("train", request);
    }
    onProgress(callback) {
        this.progressCallbacks.add(callback);
        return () => {
            this.progressCallbacks.delete(callback);
        };
    }
    async stopTraining(handle) {
        await this.sendRequest("stop", { handleId: handle.id });
    }
    async getStep(handle, stepIndex) {
        return this.sendRequest("getStep", { handleId: handle.id, stepIndex });
    }
    async getStepWithGeometry(handle, stepIndex) {
        return this.sendRequest("getStepWithGeometry", { handleId: handle.id, stepIndex });
    }
    async getTraceInfo(handle) {
        return this.sendRequest("getTraceInfo", { handleId: handle.id });
    }
    // ==========================================================================
    // Trace Management
    // ==========================================================================
    async loadTrace(trace, step = "best", name) {
        return this.sendRequest("loadTrace", { trace, step, name });
    }
    async loadSavedTrace(traceId, step = "best") {
        return this.sendRequest("loadSavedTrace", { traceId, step });
    }
    async saveTrace(name) {
        return this.sendRequest("saveTrace", { name });
    }
    async listTraces() {
        return this.sendRequest("listTraces", {});
    }
    async renameTrace(traceId, name) {
        return this.sendRequest("renameTrace", { traceId, name });
    }
    async deleteTrace(traceId) {
        return this.sendRequest("deleteTrace", { traceId });
    }
    // ==========================================================================
    // Sample Traces
    // ==========================================================================
    async listSampleTraces() {
        return this.sendRequest("listSampleTraces", {});
    }
    async loadSampleTrace(filename, step = "best") {
        return this.sendRequest("loadSampleTrace", { filename, step });
    }
    disconnect() {
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
export function createTrainingClient(config) {
    switch (config.transport) {
        case "worker":
            throw new Error("Worker transport is not available in @apvd/client. " +
                "Use createWorkerTrainingClient from @apvd/worker instead.");
        case "websocket":
            return new WebSocketTrainingClient(config.url, config.expectedSha, config.versionMismatch);
        default:
            throw new Error(`Unknown transport: ${config.transport}`);
    }
}
//# sourceMappingURL=client.js.map