/**
 * Sort primitives — three modes, one signature.
 *
 *   column      — classic lexicographic / numeric sort by a chosen column
 *   kappa       — sort rows by curvature κ (anomalies first / typicality first)
 *   sameness    — pick a pivot row and sort by sameness to it
 *
 * The sort is always stable. Null/undefined values sink to the end of the
 * ordering regardless of direction (a reader's expectation; otherwise an
 * empty cell would silently float to the top in asc sorts).
 *
 * The caller provides a `lookups` factory so the sort layer doesn't need
 * to know how κ is stored or how sameness is computed — both come from
 * higher-level state.
 */

export type SortDirection = "asc" | "desc";

export type SortSpec =
  | { mode: "column"; column: string; direction: SortDirection }
  | { mode: "kappa"; direction: SortDirection }
  | { mode: "sameness"; pivot: string; direction: SortDirection };

export interface SortLookups {
  /** Pulls a column value out of an arbitrary row object. */
  row: (key: string, column: string) => unknown;
  /** κ for the row with this primary key. 0 for unknown rows. */
  kappa: (key: string) => number;
  /** Sameness between this row and the pivot. Required when mode === "sameness". */
  samenessTo?: (key: string) => number;
}

export function sortRows<T extends Record<string, unknown>>(
  rows: T[],
  spec: SortSpec | null,
  lookups: (rows: T[]) => SortLookups,
  keyField?: string,
): T[] {
  if (!spec) return rows;
  // We allocate index pairs and sort those so the input array stays
  // untouched and we get stability for free via the tie-breaker.
  const lk = lookups(rows);
  const n = rows.length;
  const idx = new Array<number>(n);
  for (let i = 0; i < n; i++) idx[i] = i;

  const dirMul = spec.direction === "desc" ? -1 : 1;

  let keyFn: (i: number) => number | string | null;
  if (spec.mode === "column") {
    keyFn = (i) => {
      const v = rows[i][spec.column];
      if (v == null) return null;
      if (typeof v === "number") return v;
      return String(v);
    };
  } else if (spec.mode === "kappa") {
    if (!keyField) throw new Error("sortRows: keyField required for κ-rank");
    keyFn = (i) => lk.kappa(String(rows[i][keyField] ?? ""));
  } else {
    // sameness
    if (!keyField) throw new Error("sortRows: keyField required for sameness");
    if (!spec.pivot || !lk.samenessTo) {
      return rows; // no pivot → no-op
    }
    keyFn = (i) => lk.samenessTo!(String(rows[i][keyField] ?? ""));
  }

  idx.sort((a, b) => {
    const va = keyFn(a);
    const vb = keyFn(b);
    const nullA = va == null;
    const nullB = vb == null;
    if (nullA && nullB) return a - b;
    if (nullA) return 1; // nulls always last
    if (nullB) return -1;
    let cmp = 0;
    if (typeof va === "number" && typeof vb === "number") {
      cmp = va - vb;
    } else {
      cmp = String(va).localeCompare(String(vb), undefined, { sensitivity: "base" });
    }
    if (cmp === 0) return a - b; // stable tiebreak
    return cmp * dirMul;
  });

  const out = new Array<T>(n);
  for (let i = 0; i < n; i++) out[i] = rows[idx[i]];
  return out;
}
