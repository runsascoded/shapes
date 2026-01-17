/* tslint:disable */
/* eslint-disable */
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
    hull: Segment[];
}

export interface Dual {
    v: number;
    d: number[];
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

export interface Error {
    key: string;
    actual_area: number | null;
    actual_frac: number;
    target_area: number;
    target_frac: number;
    error: Dual;
}

export interface HistoryStep {
    error: number;
    shapes: Shape<number>[];
}

export interface Intersection<D> {
    x: D;
    y: D;
    c0idx: number;
    c1idx: number;
    t0: D;
    t1: D;
}

export interface Model {
    steps: Step[];
    repeat_idx: number | null;
    min_idx: number;
    min_error: number;
}

export interface Point {
    p: R2<number>;
    edge_idxs: number[];
}

export interface R2<D> {
    x: D;
    y: D;
}

export interface Region {
    key: string;
    segments: Segment[];
    area: number;
    container_set_idxs: number[];
    child_component_keys: string[];
}

export interface Segment {
    edge_idx: number;
    fwd: boolean;
}

export interface Set<D> {
    idx: number;
    child_component_keys: Key[];
    shape: Shape<D>;
}

export interface Step {
    shapes: Shape<D>[];
    components: Component[];
    targets: Targets<number>;
    total_area: Dual;
    errors: Errors;
    error: Dual;
}

export interface Targets<D> {
    all: TargetsMap<D>;
    given: string[];
    n: number;
    total_area: D;
}

export interface XYRR<D> {
    c: R2<D>;
    r: R2<D>;
}

export interface XYRRT<D> {
    c: R2<D>;
    r: R2<D>;
    t: D;
}

export type D = Dual;

export type Duals = number[][];

export type Errors = Record<string, Error>;

export type History = HistoryStep[];

export type Input = [Shape<number>, Duals];

export type Key = string;

export type Shape<D> = { Circle: Circle<D> } | { XYRR: XYRR<D> } | { XYRRT: XYRRT<D> };

export type TargetsMap<D> = Record<string, D>;


/**
 * Expands target specifications into fully-qualified region targets.
 *
 * Handles inclusive ("1*") and exclusive ("10") region specifications,
 * expanding wildcards and computing disjoint region targets.
 *
 * # Arguments
 * * `targets` - Map of region patterns to target sizes.
 *
 * # Returns
 * Expanded [`Targets`] with all region keys fully specified.
 */
export function expand_targets(targets: any): any;

/**
 * Initializes the logging system for WASM.
 *
 * Sets up console logging and panic hooks for better error reporting in the browser.
 * Should be called once at application startup.
 */
export function init_logs(): void;

/**
 * Creates an optimization model for area-proportional Venn diagrams.
 *
 * # Arguments
 * * `inputs` - Array of shape specifications (Circle, XYRR, or XYRRT) with their
 *   trainable parameter flags.
 * * `targets` - Map of region keys to target area sizes. Keys use characters
 *   to indicate set membership (e.g., "10" = in set 0 only, "11" = in both sets).
 *
 * # Returns
 * A [`Model`] ready for training via [`train`].
 *
 * # Panics
 * If the scene cannot be constructed (e.g., invalid geometry).
 */
export function make_model(inputs: any, targets: any): any;

/**
 * Computes a single optimization step for area-proportional Venn diagrams.
 *
 * # Arguments
 * * `inputs` - Array of shape specifications with their trainable parameters.
 * * `targets` - Map of region keys to target area sizes.
 *
 * # Returns
 * A [`Step`] containing current shapes, computed areas, and error gradients.
 *
 * # Panics
 * If the scene cannot be constructed (e.g., invalid geometry).
 */
export function make_step(inputs: any, targets: any): any;

/**
 * Performs a single gradient descent step.
 *
 * # Arguments
 * * `step` - Current optimization state from [`make_step`] or a previous [`step`] call.
 * * `max_step_error_ratio` - Learning rate scaling factor.
 *
 * # Returns
 * New [`Step`] with updated shape positions.
 *
 * # Panics
 * If the step fails due to invalid geometry.
 */
export function step(step: any, max_step_error_ratio: number): any;

/**
 * Runs gradient descent training on a model.
 *
 * # Arguments
 * * `model` - Model created by [`make_model`].
 * * `max_step_error_ratio` - Stop if error reduction ratio falls below this threshold.
 * * `max_steps` - Maximum number of optimization steps.
 *
 * # Returns
 * Updated model with training history containing all intermediate steps.
 *
 * # Panics
 * If a training step fails due to invalid geometry.
 */
export function train(model: any, max_step_error_ratio: number, max_steps: number): any;

/**
 * Updates the log level filter.
 *
 * # Arguments
 * * `level` - Log level string: "error", "warn", "info", "debug", or "trace".
 *   Defaults to "info" if empty or null.
 */
export function update_log_level(level: any): void;

/**
 * Computes intersection points between an axis-aligned ellipse and the unit circle.
 *
 * Used internally for ellipse-ellipse intersection calculations.
 *
 * # Arguments
 * * `xyrr` - Axis-aligned ellipse specification.
 *
 * # Returns
 * Array of intersection points on the unit circle.
 */
export function xyrr_unit(xyrr: any): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly expand_targets: (a: any) => any;
    readonly init_logs: () => void;
    readonly make_model: (a: any, b: any) => any;
    readonly make_step: (a: any, b: any) => any;
    readonly step: (a: any, b: number) => any;
    readonly train: (a: any, b: number, c: number) => any;
    readonly update_log_level: (a: any) => void;
    readonly xyrr_unit: (a: any) => any;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
