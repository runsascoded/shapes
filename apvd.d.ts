/* tslint:disable */
/* eslint-disable */
/**
*/
export function init_logs(): void;
/**
* @param {any} level
*/
export function update_log_level(level: any): void;
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
/**
* @param {any} targets
* @returns {any}
*/
export function expand_areas(targets: any): any;
/**
* @param {any} xyrr
* @returns {any}
*/
export function xyrr_unit(xyrr: any): any;
export interface Circle<D> {
    idx: number;
    c: R2<D>;
    r: D;
}

export interface Regions {
    shapes: Shape<number>[];
    points: Point[];
    edges: Edge[];
    regions: Region[];
}

export interface Region {
    key: string;
    segments: Segment[];
    area: Dual;
    container_idxs: number[];
    container_bmp: boolean[];
}

export interface Segment {
    edge_idx: number;
    fwd: boolean;
}

export interface Edge {
    cidx: number;
    i0: number;
    i1: number;
    t0: number;
    t1: number;
    containers: number[];
    containments: boolean[];
}

export interface Point {
    i: Intersection<D>;
    edge_idxs: number[];
}

export interface Error {
    key: string;
    actual_area: Dual | null;
    actual_frac: Dual;
    target_area: number;
    total_target_area: number;
    target_frac: number;
    error: Dual;
}

export interface Diagram {
    inputs: Input[];
    regions: Regions;
    targets: Targets;
    total_target_area: number;
    total_area: Dual;
    errors: Errors;
    error: Dual;
}

export type Errors = Record<string, Error>;

export type Targets = Record<string, number>;

export interface XYRR<D> {
    idx: number;
    c: R2<D>;
    r: R2<D>;
}

export interface Intersection<D> {
    x: D;
    y: D;
    c0idx: number;
    c1idx: number;
    t0: D;
    t1: D;
}

export type D = Dual;

export type Shape<D> = { Circle: Circle<D> } | { XYRR: XYRR<D> };

export type Input = [Shape<number>, Duals];

export type Duals = number[][];

export interface Model {
    steps: Diagram[];
    repeat_idx: number | null;
    min_idx: number;
    min_error: number;
}

export interface Dual {
    v: number;
    d: number[];
}

export interface R2<D> {
    x: D;
    y: D;
}


export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly init_logs: () => void;
  readonly update_log_level: (a: number) => void;
  readonly make_diagram: (a: number, b: number) => number;
  readonly make_model: (a: number, b: number) => number;
  readonly train: (a: number, b: number, c: number) => number;
  readonly step: (a: number, b: number) => number;
  readonly expand_areas: (a: number) => number;
  readonly xyrr_unit: (a: number) => number;
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
