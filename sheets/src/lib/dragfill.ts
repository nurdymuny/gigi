/**
 * Drag-fill primitives — Excel-style autofill, GIGI-flavored.
 *
 * The numeric case uses ordinary least-squares fit over the entire seed
 * selection, not the last-pair difference Excel defaults to. So a noisy
 * `[1, 1.9, 3.1, 4]` extrapolates as a true trend (≈ 5.04, 6.08, …)
 * rather than `[3.9, 3.9, …]` from the trivial last-pair delta.
 *
 * Date extrapolation infers the modal step (day / week) from the seed.
 * Categorical fill picks the most-common value — the simplest cohort-aware
 * heuristic; future revisions can swap in sameness-to-cohort selection.
 */

export interface OlsFit {
  slope: number;
  intercept: number;
}

/** Ordinary least-squares fit on (i, values[i]) for i = 0..n-1. */
export function ols(values: number[]): OlsFit {
  const n = values.length;
  if (n < 2) return { slope: 0, intercept: n === 1 ? values[0] : 0 };
  const meanX = (n - 1) / 2;
  let sumY = 0;
  for (let i = 0; i < n; i++) sumY += values[i];
  const meanY = sumY / n;
  let num = 0;
  let den = 0;
  for (let i = 0; i < n; i++) {
    const dx = i - meanX;
    num += dx * (values[i] - meanY);
    den += dx * dx;
  }
  const slope = den > 0 ? num / den : 0;
  const intercept = meanY - slope * meanX;
  return { slope, intercept };
}

/** Extrapolate `count` numeric values past the seed via OLS. */
export function dragFillNumeric(seed: number[], count: number): number[] {
  if (seed.length === 0 || count <= 0) return [];
  const { slope, intercept } = ols(seed);
  const out: number[] = [];
  for (let k = 0; k < count; k++) {
    const idx = seed.length + k;
    out.push(intercept + slope * idx);
  }
  return out;
}

/**
 * Extrapolate `count` dates past the seed using the modal step in days.
 * Inputs / outputs are ISO date strings (YYYY-MM-DD).
 */
export function dragFillDate(seed: string[], count: number): string[] {
  if (seed.length === 0 || count <= 0) return [];
  if (seed.length === 1) {
    // No step inferable; just repeat the date `count` times.
    return new Array(count).fill(seed[0]);
  }
  // Infer the modal step in days between consecutive seeds.
  const deltas: number[] = [];
  for (let i = 1; i < seed.length; i++) {
    const a = Date.parse(seed[i - 1]);
    const b = Date.parse(seed[i]);
    if (Number.isNaN(a) || Number.isNaN(b)) continue;
    const days = Math.round((b - a) / (24 * 60 * 60 * 1000));
    deltas.push(days);
  }
  if (deltas.length === 0) return new Array(count).fill(seed[seed.length - 1]);
  // Pick the modal step. If there's no clear mode, take the median.
  const step = mode(deltas) ?? median(deltas);
  const last = Date.parse(seed[seed.length - 1]);
  const out: string[] = [];
  for (let k = 1; k <= count; k++) {
    const d = new Date(last + k * step * 24 * 60 * 60 * 1000);
    out.push(toIsoDate(d));
  }
  return out;
}

/** Categorical fill — repeat the most-common seed value. */
export function dragFillCategorical(seed: string[], count: number): string[] {
  if (seed.length === 0 || count <= 0) return [];
  const counts = new Map<string, number>();
  for (const v of seed) counts.set(v, (counts.get(v) ?? 0) + 1);
  let best = seed[0];
  let bestN = 0;
  for (const [k, n] of counts) {
    if (n > bestN) {
      best = k;
      bestN = n;
    }
  }
  return new Array(count).fill(best);
}

// ── helpers ────────────────────────────────────────────────────────────

function mode(xs: number[]): number | null {
  if (xs.length === 0) return null;
  const counts = new Map<number, number>();
  for (const x of xs) counts.set(x, (counts.get(x) ?? 0) + 1);
  let best: number | null = null;
  let bestN = 0;
  for (const [k, n] of counts) {
    if (n > bestN) {
      best = k;
      bestN = n;
    }
  }
  return best;
}

function median(xs: number[]): number {
  if (xs.length === 0) return 0;
  const sorted = xs.slice().sort((a, b) => a - b);
  const m = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[m] : (sorted[m - 1] + sorted[m]) / 2;
}

function toIsoDate(d: Date): string {
  const y = d.getUTCFullYear();
  const m = String(d.getUTCMonth() + 1).padStart(2, "0");
  const dd = String(d.getUTCDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}
