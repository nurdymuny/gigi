/**
 * Selection model for Sheets.
 *
 * Two orthogonal selection modes coexist:
 *   - rect: a rectangular cell range (Excel-style)
 *   - rowKeys: a set of row primary-key strings (Airtable-style)
 *
 * Plus the GIGI extension: `extendByKappaNeighborhood` grows a row set by
 * pulling in any row whose embedding sits within ε of the selection's
 * centroid. The "shift+G to grow by similarity" affordance from §3 of
 * FEATURE_PARITY.md.
 *
 * All operations are immutable — every mutator returns a fresh object so
 * the React side can rely on referential change to trigger re-renders.
 */

import { cohortCentroid, sameness } from "./davis";

/** A rectangular cell range. Inclusive on all sides; (r1,c1) is one
 *  corner, (r2,c2) is the opposite. Orientation isn't constrained — use
 *  `normalizeRect` to get top-left → bottom-right. */
export interface Rect {
  r1: number;
  c1: number;
  r2: number;
  c2: number;
}

export interface Selection {
  rect: Rect | null;
  rowKeys: Set<string>;
}

export function emptySelection(): Selection {
  return { rect: null, rowKeys: new Set() };
}

export function normalizeRect(r: Rect): Rect {
  return {
    r1: Math.min(r.r1, r.r2),
    c1: Math.min(r.c1, r.c2),
    r2: Math.max(r.r1, r.r2),
    c2: Math.max(r.c1, r.c2),
  };
}

/** Expand a rect into an ordered list of {row, col} cells. */
export function cellRange(r: Rect): Array<{ row: number; col: number }> {
  const n = normalizeRect(r);
  const out: Array<{ row: number; col: number }> = [];
  for (let row = n.r1; row <= n.r2; row++) {
    for (let col = n.c1; col <= n.c2; col++) {
      out.push({ row, col });
    }
  }
  return out;
}

export function isCellSelected(
  sel: Selection,
  row: number,
  col: number,
): boolean {
  if (!sel.rect) return false;
  const n = normalizeRect(sel.rect);
  return row >= n.r1 && row <= n.r2 && col >= n.c1 && col <= n.c2;
}

export function isRowSelected(sel: Selection, rowKey: string): boolean {
  return sel.rowKeys.has(rowKey);
}

/** Toggle a row's membership. Returns a fresh Selection. */
export function toggleRow(sel: Selection, rowKey: string): Selection {
  const next = new Set(sel.rowKeys);
  if (next.has(rowKey)) {
    next.delete(rowKey);
  } else {
    next.add(rowKey);
  }
  return { rect: sel.rect, rowKeys: next };
}

/**
 * κ-neighborhood extension. Compute the centroid of the seed rows'
 * embeddings, then add every row whose sameness to that centroid is ≥ τ.
 *
 * Seed rows are always retained — even if their own sameness dips below
 * τ (which can happen for outliers within a small seed).
 *
 * Missing embeddings are silently skipped so a stale rowKey set doesn't
 * blow up the grid. The caller is responsible for keeping the embedding
 * map in sync with the row list.
 */
export function extendByKappaNeighborhood(
  seed: Set<string>,
  embeddings: Map<string, Float32Array>,
  threshold: number,
): Set<string> {
  const seedVecs: Float32Array[] = [];
  for (const k of seed) {
    const v = embeddings.get(k);
    if (v) seedVecs.push(v);
  }
  if (seedVecs.length === 0) return new Set(seed);
  const centroid = cohortCentroid(seedVecs);
  // Zero centroid → no preferred direction; can't extend meaningfully.
  let centroidNorm = 0;
  for (let i = 0; i < centroid.length; i++) centroidNorm += centroid[i] * centroid[i];
  if (centroidNorm === 0) return new Set(seed);

  const out = new Set(seed);
  for (const [key, vec] of embeddings) {
    if (out.has(key)) continue;
    if (sameness(vec, centroid) >= threshold) {
      out.add(key);
    }
  }
  return out;
}

export interface SelectionStats {
  count: number;
  sum: number;
  /** Mean of the finite values; null if count is 0. */
  mean: number | null;
}

export function selectionStats(values: Array<number | null | undefined>): SelectionStats {
  let sum = 0;
  let count = 0;
  for (const v of values) {
    if (typeof v === "number" && Number.isFinite(v)) {
      sum += v;
      count += 1;
    }
  }
  return {
    count,
    sum,
    mean: count > 0 ? sum / count : null,
  };
}
