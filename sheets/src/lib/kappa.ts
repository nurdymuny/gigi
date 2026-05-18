/**
 * κ (curvature) kernel — client-side cohort computation.
 *
 * Until the engine ships per-cohort κ deltas in the update response
 * (addendum E-S1a), Sheets computes κ locally with the same shape the
 * engine uses internally: leave-one-out Euclidean distance from the
 * fiber-space centroid of a row's cohort.
 *
 * A "cohort" is the set of rows that share a value on the chosen cover
 * field. Default cover field selection: first categorical-ish fiber
 * field. The user can change it from the toolbar.
 *
 * This module is pure — no React, no DOM. Easy to test, easy to swap
 * out when the engine starts returning κ deltas natively.
 */

import type { BundleSchema, FieldDescriptor, RowMap } from "./gigi-client";

export type KappaClass = "ok" | "warn" | "bad";

export interface KappaThresholds {
  warn: number;
  bad: number;
}

export const DEFAULT_THRESHOLDS: KappaThresholds = { warn: 0.8, bad: 2.0 };

export function kappaClass(
  k: number,
  thresholds: KappaThresholds = DEFAULT_THRESHOLDS,
): KappaClass {
  if (!Number.isFinite(k) || k < 0) return "ok";
  if (k >= thresholds.bad) return "bad";
  if (k >= thresholds.warn) return "warn";
  return "ok";
}

/**
 * Pick a sensible default cover field from a schema.
 *
 * Preference order:
 *   1. First fiber field with type "categorical"
 *   2. First fiber field with type "text" (not encrypted)
 *   3. The primary key (degenerate — every row is alone, κ = 0)
 */
export function pickDefaultCoverField(schema: BundleSchema): string {
  for (const f of schema.fiber_fields) {
    if (f.type === "categorical" && !isEncrypted(f)) return f.name;
  }
  for (const f of schema.fiber_fields) {
    if (f.type === "text" && !isEncrypted(f)) return f.name;
  }
  return schema.base_fields[0]?.name ?? "";
}

/** Numeric fiber fields used by the kernel as the "fiber space" axes. */
export function numericFiberFields(schema: BundleSchema): string[] {
  return schema.fiber_fields
    .filter((f) => f.type === "numeric" && !isEncrypted(f))
    .map((f) => f.name);
}

function isEncrypted(f: FieldDescriptor): boolean {
  return Boolean(f.encryption && f.encryption !== "none");
}

export interface CohortKappaInput {
  rows: RowMap[];
  keyField: string;
  coverField: string;
  fiberFields: string[];
  /** Divisor that maps raw fiber-space distance → κ. Match the engine's scale. */
  scale?: number;
}

/**
 * Compute κ for every row.
 *
 * For each row r in cohort C(r) = { row : row[coverField] === r[coverField] }:
 *   centroid = mean of fiber values over C(r) \ {r}     // leave-one-out
 *   κ(r) = ‖fiber(r) − centroid‖₂ / scale
 *
 * Singleton cohorts get κ = 0 (no peers ⇒ no comparison).
 */
export function computeCohortKappa({
  rows,
  keyField,
  coverField,
  fiberFields,
  scale = 10,
}: CohortKappaInput): Map<string, number> {
  const out = new Map<string, number>();
  if (rows.length === 0 || fiberFields.length === 0) return out;

  // Group row indices by cover value.
  const groups = new Map<string, number[]>();
  for (let i = 0; i < rows.length; i++) {
    const cover = String(rows[i][coverField] ?? "");
    const bucket = groups.get(cover);
    if (bucket) bucket.push(i);
    else groups.set(cover, [i]);
  }

  for (const indices of groups.values()) {
    // Precompute per-field cohort sums so each leave-one-out centroid is O(1).
    const sums = new Map<string, number>();
    const counts = new Map<string, number>();
    for (const field of fiberFields) {
      let s = 0;
      let c = 0;
      for (const idx of indices) {
        const v = Number(rows[idx][field]);
        if (Number.isFinite(v)) {
          s += v;
          c += 1;
        }
      }
      sums.set(field, s);
      counts.set(field, c);
    }

    for (const idx of indices) {
      const rowKey = String(rows[idx][keyField] ?? idx);
      if (indices.length < 2) {
        out.set(rowKey, 0);
        continue;
      }
      let dist2 = 0;
      for (const field of fiberFields) {
        const target = Number(rows[idx][field]);
        if (!Number.isFinite(target)) continue;
        const sum = sums.get(field) ?? 0;
        const count = counts.get(field) ?? 0;
        const peerCount = count - 1;
        if (peerCount <= 0) continue;
        const centroid = (sum - target) / peerCount;
        const d = target - centroid;
        dist2 += d * d;
      }
      out.set(rowKey, Math.sqrt(dist2) / scale);
    }
  }

  return out;
}
