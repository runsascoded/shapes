/* tslint:disable */
/* eslint-disable */
/**
* @param {any} level
*/
export function init_logs(level: any): void;
/**
* @param {any} inputs
* @param {any} targets
* @returns {any}
*/
export function make_diagram(inputs: any, targets: any): any;
/**
* @param {any} inputs
* @param {any} targets
* @returns {any}
*/
export function make_model(inputs: any, targets: any): any;
/**
* @param {any} model
* @param {number} max_step_error_ratio
* @param {number} max_steps
* @returns {any}
*/
export function train(model: any, max_step_error_ratio: number, max_steps: number): any;
/**
* @param {any} diagram
* @param {number} max_step_error_ratio
* @returns {any}
*/
export function step(diagram: any, max_step_error_ratio: number): any;
export interface Dual {
    v: number;
    d: number[];
}

export type D = Dual;

export interface R2<D> {
    x: D;
    y: D;
}

export type Input = [Circle<number>, Duals];

export type Duals = [number[], number[], number[]];

export interface Circle<D> {
    idx: number;
    c: R2<D>;
    r: D;
}

export interface Error {
    key: string;
    actual_area: Dual | null;
    total_area: Dual;
    actual_frac: Dual;
    target_area: number;
    total_target_area: number;
    target_frac: number;
    error: Dual;
}

export interface Diagram {
    inputs: Input[];
    shapes: Circle<number>[];
    targets: Targets;
    total_target_area: number;
    errors: Errors;
    error: Dual;
}

export type Errors = Record<string, Error>;

export type Targets = Record<string, number>;

export interface Model {
    steps: Diagram[];
    repeat_idx: number | null;
    min_idx: number;
    min_error: number;
}


export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly init_logs: (a: number) => void;
  readonly make_diagram: (a: number, b: number) => number;
  readonly make_model: (a: number, b: number) => number;
  readonly train: (a: number, b: number, c: number) => number;
  readonly step: (a: number, b: number) => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
