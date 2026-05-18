/**
 * Row filter predicates.
 *
 * Filters stack with AND semantics — each one trims the result set. Order
 * doesn't matter for correctness (it can matter for performance, but the
 * row counts we're dealing with don't push that yet).
 *
 *   text       — contains / equals / startsWith / endsWith on a column
 *   range      — numeric inclusive range with optional min/max
 *   sameness   — keep rows with S(row, pivot) ≥ τ
 *   kappa      — keep rows whose κ falls in one of the chosen buckets
 *
 * GIGI-specific behavior: when `kind: "sameness"` or `kind: "kappa"` is
 * used, the caller must supply the relevant lookup on `FilterContext`. If
 * the lookup is missing, the filter becomes a no-op (better than throwing
 * at view-render time).
 */

export type KappaClass = "healthy" | "drift" | "anomaly";

export type Filter =
  | {
      kind: "text";
      column: string;
      op: "contains" | "equals" | "startsWith" | "endsWith";
      value: string;
    }
  | {
      kind: "range";
      column: string;
      min?: number;
      max?: number;
    }
  | {
      kind: "sameness";
      pivot: string;
      threshold: number;
    }
  | {
      kind: "kappa";
      classes: KappaClass[];
    };

export interface FilterContext {
  /** Primary-key field name on the row objects. Required for κ / sameness. */
  keyField?: string;
  /** Sameness lookup for "sameness" filter. */
  samenessTo?: (rowKey: string) => number;
  /** κ lookup for "kappa" filter. */
  kappa?: (rowKey: string) => number;
}

/** Classify a curvature value into healthy / drift / anomaly buckets.
 *  Boundaries match the production Prism Monitor thresholds. */
export function kappaClass(k: number): KappaClass {
  if (k >= 0.3) return "anomaly";
  if (k >= 0.1) return "drift";
  return "healthy";
}

export function applyFilters<T extends Record<string, unknown>>(
  rows: T[],
  filters: Filter[],
  ctx: FilterContext = {},
): T[] {
  if (filters.length === 0) return rows;
  return rows.filter((row) => filters.every((f) => match(row, f, ctx)));
}

function match<T extends Record<string, unknown>>(
  row: T,
  f: Filter,
  ctx: FilterContext,
): boolean {
  if (f.kind === "text") {
    const raw = row[f.column];
    if (raw == null) return false;
    const v = String(raw).toLowerCase();
    const needle = f.value.toLowerCase();
    switch (f.op) {
      case "contains":   return v.includes(needle);
      case "equals":     return v === needle;
      case "startsWith": return v.startsWith(needle);
      case "endsWith":   return v.endsWith(needle);
    }
  }
  if (f.kind === "range") {
    const raw = row[f.column];
    if (typeof raw !== "number" || !Number.isFinite(raw)) return false;
    if (f.min != null && raw < f.min) return false;
    if (f.max != null && raw > f.max) return false;
    return true;
  }
  if (f.kind === "sameness") {
    if (!ctx.samenessTo || !ctx.keyField) return true; // no-op without lookup
    const key = String(row[ctx.keyField] ?? "");
    return ctx.samenessTo(key) >= f.threshold;
  }
  if (f.kind === "kappa") {
    if (!ctx.kappa || !ctx.keyField) return true;
    const key = String(row[ctx.keyField] ?? "");
    return f.classes.includes(kappaClass(ctx.kappa(key)));
  }
  return true;
}
