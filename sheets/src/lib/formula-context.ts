/**
 * Bundle-aware FormulaContext builder.
 *
 * App.tsx used to inline this; Phase 3 lifts it out so the embedder
 * wiring (`SAME`/`DIST` via real Prism embeddings, `COHORT` via the
 * cover-field value, `K` via kappaMap) is testable without React.
 *
 * Cell refs use Excel-style A1 indexing over the *currently-visible*
 * row list and column order. Sort/filter/hide-fields all change which
 * cell is A1 — that's the same UX Sheets has had since 1.0 and is the
 * intended behavior per FORMULAS_SPEC §"No semantic drift under UI
 * changes" (the formula's *meaning* — read the cell at A1 — stays the
 * same; what's *displayed* at A1 changes).
 */

import { sameness as davisSameness } from "./davis";
import type { FormulaContext } from "./formula";
import type { BundleSchema, RowMap } from "./gigi-client";
import { embedBundleRow } from "./prism-workflows";

/**
 * Build a (rowKeyA, rowKeyB) → Davis-sameness function for the current
 * bundle. Caches embeddings per key so repeated queries against the
 * same pivot are O(1). Identical keys short-circuit to S=1 exactly so
 * the Davis identity holds at the degenerate case without float drift.
 *
 * Used by Gallery's find-similar mode — same engine that powers the
 * `=SAME(A1, A2)` formula, but addressed by primary key rather than
 * A1 cell ref (the row-list shape Gallery already has).
 */
export function buildBundleSameness({
  schema,
  rows,
  keyField,
}: {
  schema: BundleSchema | null;
  rows: RowMap[];
  keyField: string | undefined;
}): (keyA: string, keyB: string) => number {
  const cache = new Map<string, Float32Array>();
  function embedFor(key: string): Float32Array | null {
    const hit = cache.get(key);
    if (hit) return hit;
    if (!schema || !keyField) return null;
    const row = rows.find((r) => String(r[keyField] ?? "") === key);
    if (!row) return null;
    const v = embedBundleRow(row, schema);
    cache.set(key, v);
    return v;
  }
  return (a, b) => {
    if (a === b) return 1;
    const va = embedFor(a);
    const vb = embedFor(b);
    if (!va || !vb) return 0;
    return davisSameness(va, vb);
  };
}

export interface BundleFormulaContextInput {
  schema: BundleSchema | null;
  rows: RowMap[];
  kappaMap: Map<string, number>;
  keyField: string | undefined;
  /** Cover field — drives `=COHORT(ref)`. Without it COHORT returns "". */
  coverField: string | undefined;
  /** Optional deterministic `TODAY()` source (Phase 1.5.D). */
  today?: () => number;
}

/**
 * Parse an A1 ref into 0-based (col, row). Returns null on malformed
 * input. `AA1` → col 26; `A12` → col 0, row 11.
 */
function parseRef(ref: string): { col: number; row: number } | null {
  const m = ref.match(/^([A-Z]+)([0-9]+)$/i);
  if (!m) return null;
  let col = 0;
  for (const ch of m[1].toUpperCase()) col = col * 26 + (ch.charCodeAt(0) - 64);
  return { col: col - 1, row: parseInt(m[2], 10) - 1 };
}

/** Inverse of parseRef: (col, 1-based row) → A1. */
function makeRef(col: number, row1: number): string {
  let letters = "";
  let n = col;
  while (true) {
    letters = String.fromCharCode(65 + (n % 26)) + letters;
    if (n < 26) break;
    n = Math.floor(n / 26) - 1;
  }
  return `${letters}${row1}`;
}

export function buildBundleFormulaContext(
  input: BundleFormulaContextInput,
): FormulaContext {
  const { schema, rows, kappaMap, keyField, coverField, today } = input;
  const columns = schema
    ? [...schema.base_fields, ...schema.fiber_fields].map((f) => f.name)
    : [];

  // Embedding cache. Hot path — sameness over a range of refs touches
  // the same rows many times. Cache keyed by row index.
  const embedCache = new Map<number, Float32Array>();
  function embedAt(rowIdx: number): Float32Array | null {
    if (!schema) return null;
    if (rowIdx < 0 || rowIdx >= rows.length) return null;
    let v = embedCache.get(rowIdx);
    if (!v) {
      v = embedBundleRow(rows[rowIdx], schema);
      embedCache.set(rowIdx, v);
    }
    return v;
  }

  function rowAtRef(ref: string): RowMap | null {
    const p = parseRef(ref);
    if (!p || p.row < 0 || p.row >= rows.length) return null;
    return rows[p.row];
  }

  return {
    cell: (ref) => {
      const p = parseRef(ref);
      if (!p || p.col < 0 || p.col >= columns.length) return null;
      const row = rowAtRef(ref);
      if (!row) return null;
      const v = row[columns[p.col]];
      if (typeof v === "number") return v;
      if (v == null) return null;
      return String(v);
    },
    sameness: (refA, refB) => {
      // Self-comparison short-circuits to 1 so float noise doesn't drift
      // the identity at the degenerate case. The Davis identity at θ=0
      // is `cos²(0) + sin²(0) = 1 + 0`, and we want that exactly.
      if (refA === refB) return 1;
      const pa = parseRef(refA);
      const pb = parseRef(refB);
      if (!pa || !pb) return 0;
      const ea = embedAt(pa.row);
      const eb = embedAt(pb.row);
      if (!ea || !eb) return 0;
      return davisSameness(ea, eb);
    },
    kappa: (ref) => {
      const row = rowAtRef(ref);
      if (!row || !keyField) return 0;
      return kappaMap.get(String(row[keyField] ?? "")) ?? 0;
    },
    cohort: (ref) => {
      // The Phase 1 stub took a column name and echoed it back. The
      // intent (per FORMULAS_SPEC's `COHORT` row) is to return the
      // cohort label *of the row containing `ref`* — i.e. its value at
      // the cover field. That's what kappa.ts groups on.
      if (!coverField) return "";
      const row = rowAtRef(ref);
      if (!row) return "";
      const v = row[coverField];
      return v == null ? "" : String(v);
    },
    resolveField: (name) => {
      const colIdx = columns.indexOf(name);
      if (colIdx < 0) return null;
      const refs: string[] = [];
      for (let r = 0; r < rows.length; r++) refs.push(makeRef(colIdx, r + 1));
      return refs;
    },
    fieldRowRef: (name, row) => {
      const colIdx = columns.indexOf(name);
      if (colIdx < 0) return null;
      if (row < 1 || row > rows.length) return null;
      return makeRef(colIdx, row);
    },
    kappaRank: (ref) => {
      // Dense rank by κ descending. Rank 1 = highest κ; ties share a
      // rank, the next distinct value gets +1 (NOT +tied-count). Pure
      // function of (kappaMap, ref's row), no other state.
      const p = parseRef(ref);
      if (!p || !keyField || p.row < 0 || p.row >= rows.length) return null;
      const targetK = kappaMap.get(String(rows[p.row][keyField] ?? "")) ?? 0;
      const distinct = new Set<number>();
      for (const r of rows) distinct.add(kappaMap.get(String(r[keyField] ?? "")) ?? 0);
      const sorted = [...distinct].sort((a, b) => b - a);
      const rank = sorted.indexOf(targetK);
      return rank < 0 ? null : rank + 1;
    },
    samenessRank: (pivotRef, ref) => {
      const pp = parseRef(pivotRef);
      const pr = parseRef(ref);
      if (!pp || !pr) return null;
      if (pp.row < 0 || pp.row >= rows.length) return null;
      if (pr.row < 0 || pr.row >= rows.length) return null;
      const pivotEmb = embedAt(pp.row);
      if (!pivotEmb) return null;
      const targetEmb = embedAt(pr.row);
      if (!targetEmb) return null;
      const targetS =
        pp.row === pr.row ? 1 : davisSameness(pivotEmb, targetEmb);
      const distinct = new Set<number>();
      for (let i = 0; i < rows.length; i++) {
        const e = embedAt(i);
        if (!e) continue;
        distinct.add(i === pp.row ? 1 : davisSameness(pivotEmb, e));
      }
      const sorted = [...distinct].sort((a, b) => b - a);
      // Float-noise tolerance: find the closest sorted entry rather
      // than relying on exact-equality. Ranks are 1-based.
      let best = -1;
      let bestDelta = Infinity;
      for (let i = 0; i < sorted.length; i++) {
        const d = Math.abs(sorted[i] - targetS);
        if (d < bestDelta) { bestDelta = d; best = i; }
      }
      return best < 0 ? null : best + 1;
    },
    today,
  };
}
