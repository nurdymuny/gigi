/**
 * Tiny CSV / TSV parser with schema inference.
 *
 * Handles the common cases:
 *   - quoted fields with embedded commas / newlines
 *   - doubled-quote escapes ("")
 *   - CRLF and LF line endings
 *   - auto-detects comma vs tab as delimiter from the header line
 *
 * Schema inference walks the first N data rows and picks the narrowest
 * type that fits every cell in a column:
 *   - all parse as finite numbers → "numeric"
 *   - all parse as booleans (true/false/0/1) → "boolean"
 *   - all parse as ISO timestamps → "timestamp"
 *   - else if cardinality ≤ 20 → "categorical"
 *   - else → "text"
 */

import type { RowMap } from "./gigi-client";

export type InferredType = "text" | "numeric" | "boolean" | "categorical" | "timestamp";

export interface CsvParseResult {
  /** Field names in source order. */
  headers: string[];
  /** Inferred type per header (same order). */
  types: InferredType[];
  /** Parsed rows; each row is keyed by header name. */
  rows: RowMap[];
  /** Detected delimiter (comma or tab). */
  delimiter: "," | "\t";
  /** Number of rows that failed to parse cleanly. */
  skipped: number;
}

const ISO_DATE = /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:?\d{2})?)?$/;

export function parseCsv(text: string): CsvParseResult {
  const trimmed = text.replace(/^﻿/, "").trimEnd();
  if (!trimmed) {
    return { headers: [], types: [], rows: [], delimiter: ",", skipped: 0 };
  }
  // Auto-detect delimiter from the first 1KB (count tabs vs commas
  // outside quoted strings — quick heuristic).
  const sample = trimmed.slice(0, 1024);
  const commas = countOutsideQuotes(sample, ",");
  const tabs = countOutsideQuotes(sample, "\t");
  const delimiter: "," | "\t" = tabs > commas ? "\t" : ",";

  const cells = tokenize(trimmed, delimiter);
  if (cells.length === 0) {
    return { headers: [], types: [], rows: [], delimiter, skipped: 0 };
  }
  const headers = cells[0].map((h, i) => sanitizeHeader(h, i));
  let skipped = 0;
  const rows: RowMap[] = [];
  for (let r = 1; r < cells.length; r++) {
    const row = cells[r];
    if (row.length === 1 && row[0] === "") continue; // blank line
    if (row.length === 0) continue;
    const obj: RowMap = {};
    for (let c = 0; c < headers.length; c++) {
      obj[headers[c]] = row[c] ?? "";
    }
    rows.push(obj);
  }
  const types = headers.map((h) => inferType(h, rows));
  // Coerce values to their inferred type so the engine receives proper
  // numbers / booleans instead of strings.
  for (const row of rows) {
    for (let c = 0; c < headers.length; c++) {
      const h = headers[c];
      const t = types[c];
      const raw = row[h];
      row[h] = coerce(raw, t);
    }
  }
  return { headers, types, rows, delimiter, skipped };
}

function countOutsideQuotes(s: string, ch: string): number {
  let count = 0;
  let inQ = false;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (c === '"') {
      if (inQ && s[i + 1] === '"') {
        i++;
        continue;
      }
      inQ = !inQ;
      continue;
    }
    if (!inQ && c === ch) count++;
  }
  return count;
}

function tokenize(s: string, delim: string): string[][] {
  const out: string[][] = [];
  let row: string[] = [];
  let cell = "";
  let inQ = false;
  let i = 0;
  while (i < s.length) {
    const c = s[i];
    if (inQ) {
      if (c === '"') {
        if (s[i + 1] === '"') {
          cell += '"';
          i += 2;
          continue;
        }
        inQ = false;
        i++;
        continue;
      }
      cell += c;
      i++;
      continue;
    }
    if (c === '"') {
      inQ = true;
      i++;
      continue;
    }
    if (c === delim) {
      row.push(cell);
      cell = "";
      i++;
      continue;
    }
    if (c === "\n" || c === "\r") {
      row.push(cell);
      out.push(row);
      row = [];
      cell = "";
      // skip the \n after \r
      if (c === "\r" && s[i + 1] === "\n") i++;
      i++;
      continue;
    }
    cell += c;
    i++;
  }
  // Trailing cell + row.
  row.push(cell);
  if (row.length > 1 || row[0] !== "") out.push(row);
  return out;
}

/** Replace anything not [A-Za-z0-9_] with _, prefix with _ if it starts with a digit. */
function sanitizeHeader(raw: string, idx: number): string {
  let h = raw.trim().replace(/[^A-Za-z0-9_]/g, "_");
  if (!h) h = `col_${idx}`;
  if (/^\d/.test(h)) h = `_${h}`;
  return h;
}

function inferType(field: string, rows: RowMap[]): InferredType {
  if (rows.length === 0) return "text";
  let allNum = true;
  let allBool = true;
  let allTs = true;
  const seen = new Set<string>();
  for (const r of rows) {
    const v = String(r[field] ?? "").trim();
    if (!v) continue;
    seen.add(v);
    if (allNum && !isFiniteNumberStr(v)) allNum = false;
    if (allBool && !isBoolStr(v)) allBool = false;
    if (allTs && !ISO_DATE.test(v)) allTs = false;
  }
  if (allNum && seen.size > 0) return "numeric";
  if (allBool && seen.size > 0) return "boolean";
  if (allTs && seen.size > 0) return "timestamp";
  if (seen.size > 0 && seen.size <= 20) return "categorical";
  return "text";
}

function isFiniteNumberStr(s: string): boolean {
  if (!s) return false;
  const n = Number(s);
  return Number.isFinite(n);
}

function isBoolStr(s: string): boolean {
  const l = s.toLowerCase();
  return l === "true" || l === "false" || l === "0" || l === "1";
}

function coerce(v: unknown, t: InferredType): unknown {
  if (v == null) return v;
  const s = String(v);
  if (s === "") return null;
  if (t === "numeric") {
    const n = Number(s);
    return Number.isFinite(n) ? n : s;
  }
  if (t === "boolean") {
    const l = s.toLowerCase();
    if (l === "true" || l === "1") return true;
    if (l === "false" || l === "0") return false;
    return s;
  }
  return s;
}

/**
 * Pick a primary-key field. Preference:
 *   1. A header literally named "id", "key", or "_id"
 *   2. The first column if it has unique non-empty values
 *   3. null — caller can prompt
 */
export function pickKeyField(headers: string[], rows: RowMap[]): string | null {
  const preferred = headers.find((h) =>
    ["id", "_id", "key", "uuid"].includes(h.toLowerCase()),
  );
  if (preferred) return preferred;
  if (headers.length === 0) return null;
  const first = headers[0];
  const seen = new Set<string>();
  for (const r of rows) {
    const v = String(r[first] ?? "");
    if (!v || seen.has(v)) return null;
    seen.add(v);
  }
  return first;
}
