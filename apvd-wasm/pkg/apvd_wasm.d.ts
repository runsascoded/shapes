/* tslint:disable */
/* eslint-disable */
export interface AdamConfig {
    beta1: number;
    beta2: number;
    epsilon: number;
}

export interface AdamState {
    m: number[];
    v: number[];
    t: number;
    beta1: number;
    beta2: number;
    epsilon: number;
}

export interface BtdEvenlySpacedConfig {
    maxBtdKeyframes: number | null;
    intervalSpacing: number | null;
}

export interface Circle<D> {
    c: R2<D>;
    r: D;
}

export interface Component {
    key: string;
    points: Point[];
    edges: Edge[];
    regions: Region[];
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
    kind: ErrorKind;
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

export interface OptimConfig {
    learning_rate: number;
    max_grad_norm: number;
    max_grad_value: number;
    beta1: number;
    beta2: number;
    epsilon: number;
    warmup_steps: number;
    max_error_increase: number;
    max_rejections: number;
}

export interface Penalties {
    disjoint: number;
    contained: number;
    self_intersection: number;
    regularity: number;
}

export interface Point {
    p: R2<number>;
    edge_idxs: number[];
}

export interface Polygon<D> {
    vertices: R2<D>[];
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
    converged: boolean;
    penalties: Penalties;
}

export interface Targets<D> {
    all: TargetsMap<D>;
    given: string[];
    n: number;
    total_area: D;
}

export interface TieredConfig {
    bucketSize: number;
}

export interface TraceMetadata {
    totalSteps: number;
    storedSteps: number;
    strategy: string;
    minIndex: number;
    minError: number;
    btdIndices?: number[];
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

export type ErrorKind = { type: "AreaMismatch"; signed_error: number } | { type: "MissingRegion"; target_frac: number } | { type: "ExtraRegion"; actual_frac: number };

export type Errors = Record<string, Error>;

export type History = HistoryStep[];

export type Input = [Shape<number>, Duals];

export type Key = string;

export type Shape<D> = ({ kind: "Circle" } & Circle<D>) | ({ kind: "XYRR" } & XYRR<D>) | ({ kind: "XYRRT" } & XYRRT<D>) | ({ kind: "Polygon" } & Polygon<D>);

export type StorageStrategy = "dense" | "btd" | "tiered" | "btdevenlyspaced";

export type TargetsMap<D> = Record<string, D>;


/**
 * Checks if any polygon shapes in the given step are self-intersecting.
 *
 * Self-intersecting polygons have edges that cross each other, which
 * invalidates area calculations and causes visual artifacts.
 *
 * # Arguments
 * * `step` - Current optimization state.
 *
 * # Returns
 * Array of strings describing any validity issues (empty if valid).
 */
export function check_polygon_validity(step: any): any;

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
 * Check if a step has converged below a custom threshold.
 *
 * Use this to implement user-configurable convergence thresholds.
 * The step.converged field uses the default threshold (1e-10), but
 * this function lets you check against any threshold.
 *
 * # Arguments
 * * `step` - Current optimization state.
 * * `threshold` - Custom convergence threshold (e.g., 1e-6 for fast, 1e-14 for precise).
 *
 * # Returns
 * True if step.error < threshold.
 */
export function is_converged(step: any, threshold: number): boolean;

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
 * Creates a tiered keyframe configuration.
 *
 * Tiered storage achieves O(log N) storage for N steps while maintaining
 * bounded seek time via recomputation from keyframes.
 *
 * # Arguments
 * * `bucket_size` - Optional bucket size B (default: 1024). Tier 0 has 2B
 *   samples, other tiers have B samples.
 *
 * # Returns
 * A [`TieredConfig`] for determining which steps are keyframes.
 */
export function make_tiered_config(bucket_size?: number | null): any;

/**
 * Performs a single gradient descent step with gradient clipping (recommended).
 *
 * Uses fixed learning rate with gradient clipping for stable updates.
 * This is the recommended method - it prevents the oscillation that occurs
 * with error-scaled step sizes.
 *
 * # Arguments
 * * `step` - Current optimization state from [`make_step`] or a previous [`step`] call.
 * * `learning_rate` - Fixed learning rate (typical: 0.01 to 0.1, default 0.05).
 *
 * # Returns
 * New [`Step`] with updated shape positions.
 */
export function step(step: any, learning_rate: number): any;

/**
 * Legacy step function that scales step size by error.
 *
 * **Deprecated**: Use [`step`] instead. This function can cause oscillation
 * when error is high because step_size = error * max_step_error_ratio.
 *
 * # Arguments
 * * `step` - Current optimization state.
 * * `max_step_error_ratio` - Learning rate scaling factor.
 */
export function step_legacy(step: any, max_step_error_ratio: number): any;

/**
 * Check if a step should be stored as a keyframe.
 *
 * # Arguments
 * * `config` - Tiered configuration from [`make_tiered_config`].
 * * `step_idx` - Step index to check.
 *
 * # Returns
 * True if this step should be stored as a keyframe.
 */
export function tiered_is_keyframe(config: any, step_idx: number): boolean;

/**
 * Calculate keyframe count for N steps.
 *
 * # Arguments
 * * `config` - Tiered configuration.
 * * `total_steps` - Total number of steps.
 *
 * # Returns
 * Number of keyframes needed to store total_steps.
 */
export function tiered_keyframe_count(config: any, total_steps: number): number;

/**
 * Find the nearest keyframe at or before a step.
 *
 * # Arguments
 * * `config` - Tiered configuration from [`make_tiered_config`].
 * * `step_idx` - Target step index.
 *
 * # Returns
 * Index of the nearest keyframe ≤ step_idx.
 */
export function tiered_nearest_keyframe(config: any, step_idx: number): number;

/**
 * Seek to a target step by recomputing from a keyframe.
 *
 * Given a keyframe step, recomputes forward to reach the target step.
 * This enables random access to any step with bounded recomputation.
 *
 * # Arguments
 * * `keyframe` - The stored keyframe step.
 * * `keyframe_idx` - Index of the keyframe.
 * * `target_idx` - Target step index to seek to.
 * * `learning_rate` - Learning rate for recomputation steps.
 *
 * # Returns
 * The step at target_idx, or throws if recomputation fails.
 */
export function tiered_seek(keyframe: any, keyframe_idx: number, target_idx: number, learning_rate: number): any;

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
 * Runs Adam optimizer training on a model.
 *
 * Adam (Adaptive Moment Estimation) maintains per-parameter momentum and variance
 * estimates, enabling better convergence for complex optimization landscapes.
 * Particularly useful for mixed shape scenes (e.g., polygon + circle).
 *
 * # Arguments
 * * `model` - Model created by [`make_model`].
 * * `learning_rate` - Adam learning rate (typical: 0.001 to 0.1).
 * * `max_steps` - Maximum number of optimization steps.
 *
 * # Returns
 * Updated model with training history containing all intermediate steps.
 *
 * # Panics
 * If a training step fails due to invalid geometry.
 */
export function train_adam(model: any, learning_rate: number, max_steps: number): any;

/**
 * Runs robust optimization with Adam, gradient clipping, and backtracking.
 *
 * This is the recommended training method. It combines:
 * - Adam optimizer for per-parameter adaptive learning rates
 * - Gradient clipping to prevent catastrophically large steps
 * - Learning rate warmup for stability
 * - Step rejection when error increases significantly
 *
 * # Arguments
 * * `model` - Model created by [`make_model`].
 * * `max_steps` - Maximum number of optimization steps.
 *
 * # Returns
 * Updated model with training history containing all intermediate steps.
 *
 * # Panics
 * If a training step fails due to invalid geometry.
 */
export function train_robust(model: any, max_steps: number): any;

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
    readonly check_polygon_validity: (a: any) => any;
    readonly expand_targets: (a: any) => any;
    readonly init_logs: () => void;
    readonly is_converged: (a: any, b: number) => number;
    readonly make_model: (a: any, b: any) => any;
    readonly make_step: (a: any, b: any) => any;
    readonly make_tiered_config: (a: number) => any;
    readonly step: (a: any, b: number) => any;
    readonly step_legacy: (a: any, b: number) => any;
    readonly tiered_is_keyframe: (a: any, b: number) => number;
    readonly tiered_keyframe_count: (a: any, b: number) => number;
    readonly tiered_nearest_keyframe: (a: any, b: number) => number;
    readonly tiered_seek: (a: any, b: number, c: number, d: number) => [number, number, number];
    readonly train: (a: any, b: number, c: number) => any;
    readonly train_adam: (a: any, b: number, c: number) => any;
    readonly train_robust: (a: any, b: number) => any;
    readonly update_log_level: (a: any) => void;
    readonly xyrr_unit: (a: any) => any;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
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
