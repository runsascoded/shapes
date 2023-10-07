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
export function make_step(inputs: any, targets: any): any;
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
* @param {any} step
* @param {number} max_step_error_ratio
* @returns {any}
*/
export function step(step: any, max_step_error_ratio: number): any;
/**
* @param {any} targets
* @returns {any}
*/
export function expand_targets(targets: any): any;
/**
* @param {any} xyrr
* @returns {any}
*/
export function xyrr_unit(xyrr: any): any;
export interface Error {
    key: string;
    actual_area: Dual | null;
    actual_frac: Dual;
    target_area: number;
    target_frac: number;
    error: Dual;
}

export interface Step {
    shapes: Shape<D>[];
    components: Component[];
    targets: Targets<number>;
    total_area: Dual;
    errors: Errors;
    error: Dual;
}

export type Errors = Record<string, Error>;

export type Shape<D> = { Circle: Circle<D> } | { XYRR: XYRR<D> } | { XYRRT: XYRRT<D> };

export type Input = [Shape<number>, Duals];

export type Duals = number[][];

export interface XYRR<D> {
    c: R2<D>;
    r: R2<D>;
}

export interface Circle<D> {
    c: R2<D>;
    r: D;
}

export interface Component {
    key: string;
    sets: Set<number>[];
    points: Point[];
    edges: Edge[];
    regions: Region[];
    container_idxs: number[];
    hull: Region;
}

export interface Region {
    key: string;
    segments: Segment[];
    area: Dual;
    container_set_idxs: number[];
    child_component_keys: string[];
}

export interface Segment {
    edge_idx: number;
    fwd: boolean;
}

export interface Edge {
    set_idx: number;
    node0_idx: number;
    node1_idx: number;
    theta0: number;
    theta1: number;
    container_idxs: number[];
    is_component_boundary: boolean;
}

export interface Point {
    p: R2<D>;
    edge_idxs: number[];
}

export interface Model {
    steps: Step[];
    repeat_idx: number | null;
    min_idx: number;
    min_error: number;
}

export type D = Dual;

export interface R2<D> {
    x: D;
    y: D;
}

export type Key = string;

export interface Dual {
    v: number;
    d: number[];
}

export interface Targets<D> {
    all: TargetsMap<D>;
    given: string[];
    n: number;
    total_area: D;
}

export type TargetsMap<D> = Record<string, D>;

export interface Set<D> {
    idx: number;
    child_component_keys: Key[];
    shape: Shape<D>;
}

export interface Intersection<D> {
    x: D;
    y: D;
    c0idx: number;
    c1idx: number;
    t0: D;
    t1: D;
}

export interface XYRRT<D> {
    c: R2<D>;
    r: R2<D>;
    t: D;
}


export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly init_logs: () => void;
  readonly update_log_level: (a: number) => void;
  readonly make_step: (a: number, b: number) => number;
  readonly make_model: (a: number, b: number) => number;
  readonly train: (a: number, b: number, c: number) => number;
  readonly step: (a: number, b: number) => number;
  readonly expand_targets: (a: number) => number;
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
