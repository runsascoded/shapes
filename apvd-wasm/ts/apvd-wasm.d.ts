/**
 * Type declarations for @apvd/wasm module.
 * The actual module is the WASM build output (../pkg/).
 */
declare module "@apvd/wasm" {
  /** Initialize logging */
  export function init_logs(): void;

  /** Create initial step from inputs and targets */
  export function make_step(inputs: unknown, targets: unknown): unknown;

  /** Take one optimization step */
  export function step(wasmStep: unknown, learningRate: number): unknown;
}
