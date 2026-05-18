/**
 * Pure projection math for the Geometry tab.
 *
 * Given a set of rows and two fiber field names, computes:
 *   - the data domain (axis ranges with padding)
 *   - per-point pixel coordinates
 *   - the nearest neighbor of a target point (for the transport overlay)
 *
 * No React, no DOM. Pixel math is plain enough that the same module
 * is testable headlessly and reusable from any view.
 */

import type { RowMap } from "./gigi-client";

export interface ProjectionAxis {
  field: string;
  min: number;
  max: number;
}

export interface ProjectionDomain {
  x: ProjectionAxis;
  y: ProjectionAxis;
}

export interface ViewBox {
  width: number;
  height: number;
  margin: { l: number; r: number; t: number; b: number };
}

export interface ProjectedPoint {
  /** Row key (base_fields[0] value). */
  key: string;
  /** Original fiber value. */
  x: number;
  /** Original fiber value. */
  y: number;
  /** Pixel x within the SVG. */
  px: number;
  /** Pixel y within the SVG (top-down). */
  py: number;
}

/**
 * Compute the (x, y) domain of `rows` over `xField` × `yField`.
 *
 * - Non-finite values are skipped.
 * - If every value is identical, the axis gets a unit pad so the domain
 *   doesn't collapse to a single pixel.
 * - Returns null if no row has a finite (x, y) pair.
 */
export function computeProjectionDomain(
  rows: RowMap[],
  xField: string,
  yField: string,
  padding = 0.08,
): ProjectionDomain | null {
  let xMin = Infinity;
  let xMax = -Infinity;
  let yMin = Infinity;
  let yMax = -Infinity;
  let any = false;
  for (const r of rows) {
    const x = Number(r[xField]);
    const y = Number(r[yField]);
    if (!Number.isFinite(x) || !Number.isFinite(y)) continue;
    any = true;
    if (x < xMin) xMin = x;
    if (x > xMax) xMax = x;
    if (y < yMin) yMin = y;
    if (y > yMax) yMax = y;
  }
  if (!any) return null;
  const xPad = xMax > xMin ? (xMax - xMin) * padding : 1;
  const yPad = yMax > yMin ? (yMax - yMin) * padding : 1;
  return {
    x: { field: xField, min: xMin - xPad, max: xMax + xPad },
    y: { field: yField, min: yMin - yPad, max: yMax + yPad },
  };
}

/** Project a single (x, y) value to SVG pixel space, accounting for margins. */
export function projectPoint(
  value: { x: number; y: number },
  domain: ProjectionDomain,
  view: ViewBox,
): { px: number; py: number } {
  const { l, r, t, b } = view.margin;
  const innerW = Math.max(1, view.width - l - r);
  const innerH = Math.max(1, view.height - t - b);
  const xRange = domain.x.max - domain.x.min || 1;
  const yRange = domain.y.max - domain.y.min || 1;
  const px = l + ((value.x - domain.x.min) / xRange) * innerW;
  const py = view.height - b - ((value.y - domain.y.min) / yRange) * innerH;
  return { px, py };
}

/**
 * Project every row that has finite (x, y) values for the given fields.
 * Rows without finite coordinates are silently dropped — same contract as
 * `computeProjectionDomain`.
 */
export function projectRows(
  rows: RowMap[],
  keyField: string,
  xField: string,
  yField: string,
  domain: ProjectionDomain,
  view: ViewBox,
): ProjectedPoint[] {
  const out: ProjectedPoint[] = [];
  for (const r of rows) {
    const x = Number(r[xField]);
    const y = Number(r[yField]);
    if (!Number.isFinite(x) || !Number.isFinite(y)) continue;
    const key = String(r[keyField] ?? "");
    const { px, py } = projectPoint({ x, y }, domain, view);
    out.push({ key, x, y, px, py });
  }
  return out;
}

/**
 * Find the closest point in `candidates` to `target` in fiber-value
 * space (NOT pixel space — units stay meaningful across re-resizes).
 *
 * `excludeKey` skips the target itself, since the transport overlay never
 * draws a self-loop.
 */
export function nearestNeighbor(
  target: ProjectedPoint,
  candidates: ProjectedPoint[],
  excludeKey: string,
): ProjectedPoint | null {
  let best: ProjectedPoint | null = null;
  let bestD = Infinity;
  for (const c of candidates) {
    if (c.key === excludeKey) continue;
    const dx = c.x - target.x;
    const dy = c.y - target.y;
    const d = dx * dx + dy * dy;
    if (d < bestD) {
      bestD = d;
      best = c;
    }
  }
  return best;
}

/**
 * Deterministic per-cover color from a small palette. Same `value` always
 * gets the same color across renders / sessions.
 */
const COVER_PALETTE = [
  "#4f46e5", // indigo
  "#0e7490", // teal
  "#b45309", // amber
  "#be185d", // rose
  "#047857", // emerald
  "#7c3aed", // violet
  "#0891b2", // cyan
  "#c2410c", // orange
];

export function coverColor(value: string): string {
  let h = 2166136261; // FNV-1a offset basis
  for (let i = 0; i < value.length; i++) {
    h ^= value.charCodeAt(i);
    h = Math.imul(h, 16777619) >>> 0;
  }
  return COVER_PALETTE[h % COVER_PALETTE.length] ?? COVER_PALETTE[0];
}

/** Generate N axis tick values evenly spaced across [min, max]. */
export function axisTicks(min: number, max: number, count = 5): number[] {
  const out: number[] = [];
  if (count < 2) return [min];
  for (let i = 0; i < count; i++) {
    out.push(min + ((max - min) * i) / (count - 1));
  }
  return out;
}
