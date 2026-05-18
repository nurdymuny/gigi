/**
 * Find & replace primitives with three search modes.
 *
 *   exact      — substring match, case-insensitive (the Excel default)
 *   canonical  — strip whitespace/punctuation then exact-match. Catches
 *                reference-drift duplicates the way Prism Dedup does.
 *   sameness   — keep rows whose S(row, pivot) ≥ τ. A geometric query
 *                masquerading as a find.
 *
 * Replace operates only on `exact` and `canonical`. Sameness-find has no
 * natural notion of "replace what?" so it's read-only.
 */

import { canonicalize } from "./canon";

export type FindSpec =
  | { mode: "exact"; query: string }
  | { mode: "canonical"; query: string }
  | { mode: "sameness"; pivot: string; threshold: number };

export interface FindContext {
  keyField?: string;
  samenessTo?: (rowKey: string) => number;
}

/** Return rows where at least one of `columns` matches the spec. */
export function findInRows<T extends Record<string, unknown>>(
  rows: T[],
  spec: FindSpec,
  columns: string[],
  ctx: FindContext = {},
): T[] {
  if (spec.mode === "exact") {
    if (!spec.query) return [];
    const needle = spec.query.toLowerCase();
    return rows.filter((r) =>
      columns.some((c) => {
        const v = r[c];
        if (v == null) return false;
        return String(v).toLowerCase().includes(needle);
      }),
    );
  }
  if (spec.mode === "canonical") {
    if (!spec.query) return [];
    const canon = canonicalize(spec.query);
    if (!canon) return [];
    return rows.filter((r) =>
      columns.some((c) => {
        const v = r[c];
        if (v == null) return false;
        return canonicalize(String(v)) === canon;
      }),
    );
  }
  // sameness
  if (!ctx.samenessTo || !ctx.keyField) return [];
  return rows.filter((r) => {
    const k = String(r[ctx.keyField!] ?? "");
    return ctx.samenessTo!(k) >= spec.threshold;
  });
}

export interface ReplaceResult<T> {
  rows: T[];
  count: number;
}

/**
 * Replace matched values in the given columns. Returns a new array — input
 * is never mutated. The number of cells changed is returned so the UI can
 * surface a "1234 replacements made" toast.
 *
 * - exact: substring replace within the cell value
 * - canonical: when a cell canonicalizes to match the query's canonical
 *   form, the *whole* cell value is replaced (because we can't surgically
 *   replace canonical-equal substrings in arbitrary formats)
 */
export function replaceInRows<T extends Record<string, unknown>>(
  rows: T[],
  spec: FindSpec,
  replacement: string,
  columns: string[],
): ReplaceResult<T> {
  if (spec.mode === "sameness") {
    return { rows, count: 0 }; // sameness has no replace semantics
  }
  let count = 0;
  const out = rows.map((row) => {
    let copy: T | null = null;
    if (spec.mode === "exact") {
      const needle = spec.query.toLowerCase();
      if (!needle) return row;
      for (const c of columns) {
        const v = row[c];
        if (v == null) continue;
        const str = String(v);
        const lower = str.toLowerCase();
        if (lower.includes(needle)) {
          // Case-insensitive substring replace.
          const re = new RegExp(escapeRegExp(needle), "gi");
          const replaced = str.replace(re, replacement);
          if (replaced !== str) {
            copy = copy ?? ({ ...row } as T);
            (copy as Record<string, unknown>)[c] = replaced;
            count++;
          }
        }
      }
    } else {
      // canonical
      const canon = canonicalize(spec.query);
      if (!canon) return row;
      for (const c of columns) {
        const v = row[c];
        if (v == null) continue;
        if (canonicalize(String(v)) === canon) {
          copy = copy ?? ({ ...row } as T);
          (copy as Record<string, unknown>)[c] = replacement;
          count++;
        }
      }
    }
    return copy ?? row;
  });
  return { rows: out, count };
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
