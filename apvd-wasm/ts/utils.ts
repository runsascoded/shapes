/**
 * Pure utility functions for Dual number extraction and tiered keyframe logic.
 *
 * These are separated from worker.ts to avoid pulling in the worker's
 * `self.onmessage` side effect when imported on the main thread.
 */

import type { Shape } from "@apvd/client";

// ============================================================================
// Dual number extraction (WASM returns Shape<Dual> where coordinates are {v, d})
// ============================================================================

/**
 * Extracts plain JS number from a WASM value that may be wrapped in Dual { v: number }.
 */
export function extractNumber(val: unknown): number {
  if (typeof val === "number") return val;
  if (val && typeof val === "object" && "v" in val) return (val as { v: number }).v;
  throw new Error(`Cannot extract number from ${JSON.stringify(val)}`);
}

export function extractPoint(pt: unknown): { x: number; y: number } {
  const p = pt as { x: unknown; y: unknown };
  return { x: extractNumber(p.x), y: extractNumber(p.y) };
}

/**
 * Converts WASM Shape<Dual> to plain Shape<number> by extracting values.
 */
export function extractShape(wasmShape: unknown): Shape {
  const s = wasmShape as { kind: string; c?: unknown; r?: unknown; t?: unknown; vertices?: unknown[] };
  if (s.kind === "Circle") {
    return {
      kind: "Circle",
      c: extractPoint(s.c),
      r: extractNumber(s.r),
    };
  } else if (s.kind === "XYRR") {
    return {
      kind: "XYRR",
      c: extractPoint(s.c),
      r: extractPoint(s.r),
    };
  } else if (s.kind === "XYRRT") {
    return {
      kind: "XYRRT",
      c: extractPoint(s.c),
      r: extractPoint(s.r),
      t: extractNumber(s.t),
    };
  } else {
    // Polygon
    const vertices = (s.vertices ?? []).map(v => extractPoint(v));
    return { kind: "Polygon", vertices };
  }
}

export function extractShapes(wasmShapes: unknown[]): Shape[] {
  return wasmShapes.map(s => extractShape(s));
}

// ============================================================================
// Sparkline data extraction
// ============================================================================

export function extractSparklineData(
  modelSteps: unknown[],
  startIndex: number,
): { gradients: number[][]; regionErrors: Record<string, number[]> } {
  const gradients: number[][] = [];
  const regionErrors: Record<string, number[]> = {};

  for (let i = startIndex; i < modelSteps.length; i++) {
    const wasmStep = modelSteps[i] as {
      error: { v: number; d?: number[] };
      errors: Map<string, { error: { v: number } }> | Record<string, { error: { v: number } }>;
    };

    gradients.push(wasmStep.error.d || []);

    const errors = wasmStep.errors;
    if (errors) {
      const errorEntries = errors instanceof Map ? errors.entries() : Object.entries(errors);
      for (const [regionKey, regionErr] of errorEntries) {
        if (!regionErrors[regionKey]) {
          regionErrors[regionKey] = [];
        }
        while (regionErrors[regionKey].length < i - startIndex) {
          regionErrors[regionKey].push(0);
        }
        regionErrors[regionKey].push((regionErr as { error: { v: number } }).error.v);
      }
    }
  }

  return { gradients, regionErrors };
}

// ============================================================================
// Tiered keyframe helpers
// ============================================================================

export function tier(step: number, b: number): number {
  if (step < 2 * b) return 0;
  return Math.floor(Math.log2(step / b));
}

export function resolution(t: number): number {
  return 1 << t;
}

export function isKeyframe(step: number, b: number): boolean {
  const t = tier(step, b);
  const res = resolution(t);
  return step % res === 0;
}

export function nearestKeyframe(step: number, b: number): number {
  const t = tier(step, b);
  const res = resolution(t);
  return Math.floor(step / res) * res;
}
