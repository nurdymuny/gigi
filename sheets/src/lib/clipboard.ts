/**
 * Clipboard primitives — TSV in/out + bundle-JSON for richer paste.
 *
 * Two serialization formats:
 *   - TSV: lossless tab-separated, round-trips with Excel / Google Sheets
 *   - Bundle JSON: a richer envelope that preserves field types when
 *     pasting between GIGI bundles.
 *
 * `validatePaste` is the GIGI-specific bit: when text lands in a typed
 * column it pre-flags rows whose values don't match the column's declared
 * type. Excel will let you paste "potato" into a number column without a
 * peep — we surface that as a warning before commit.
 */

export type FieldType = "text" | "numeric" | "boolean" | "categorical" | "timestamp";

/** Serialize a 2-D grid of strings into TSV. Each row ends in a newline. */
export function toTsv(grid: string[][]): string {
  return grid.map((row) => row.join("\t")).join("\n") + (grid.length ? "\n" : "");
}

/** Parse TSV into a 2-D grid of strings. Tolerates CRLF and trailing newline. */
export function fromTsv(s: string): string[][] {
  if (!s) return [];
  const normalized = s.replace(/\r\n/g, "\n");
  const trimmed = normalized.endsWith("\n")
    ? normalized.slice(0, -1)
    : normalized;
  if (!trimmed) return [];
  return trimmed.split("\n").map((row) => row.split("\t"));
}

export interface BundleClipboardPayload {
  bundle: string | null;
  columns: string[];
  rows: Record<string, unknown>[];
}

const BUNDLE_KIND = "gigi.clipboard.v1";

/** Serialize a bundle slice into the GIGI clipboard JSON envelope. */
export function toBundleJson(payload: BundleClipboardPayload): string {
  return JSON.stringify({
    kind: BUNDLE_KIND,
    bundle: payload.bundle,
    columns: payload.columns,
    rows: payload.rows,
  });
}

/** Parse a GIGI clipboard JSON envelope. Returns null on any shape error. */
export function fromBundleJson(s: string): BundleClipboardPayload | null {
  try {
    const parsed = JSON.parse(s);
    if (!parsed || parsed.kind !== BUNDLE_KIND) return null;
    if (!Array.isArray(parsed.columns)) return null;
    if (!Array.isArray(parsed.rows)) return null;
    return {
      bundle: typeof parsed.bundle === "string" ? parsed.bundle : null,
      columns: parsed.columns,
      rows: parsed.rows,
    };
  } catch {
    return null;
  }
}

export interface PasteWarning {
  row: number;
  column: string;
  reason: string;
}

export interface PasteValidation {
  warnings: PasteWarning[];
}

/**
 * Pre-flight a TSV paste against a typed target schema. Returns a list of
 * warnings (row, column, reason). Empty cells are *not* flagged — they
 * round-trip as null. Unknown columns (not in the schema) are skipped so
 * partial-schema overlap is tolerated.
 */
export function validatePaste(
  grid: string[][],
  columns: string[],
  schema: Map<string, FieldType>,
): PasteValidation {
  const warnings: PasteWarning[] = [];
  for (let r = 0; r < grid.length; r++) {
    const row = grid[r];
    for (let c = 0; c < row.length && c < columns.length; c++) {
      const col = columns[c];
      const t = schema.get(col);
      if (!t) continue;
      const val = row[c];
      if (val === "" || val == null) continue;
      if (t === "numeric") {
        if (!Number.isFinite(Number(val))) {
          warnings.push({
            row: r,
            column: col,
            reason: `not a numeric value: "${val}"`,
          });
        }
      } else if (t === "boolean") {
        if (!/^(true|false|0|1|yes|no)$/i.test(val)) {
          warnings.push({
            row: r,
            column: col,
            reason: `not a boolean value: "${val}"`,
          });
        }
      } else if (t === "timestamp") {
        if (Number.isNaN(Date.parse(val))) {
          warnings.push({
            row: r,
            column: col,
            reason: `not parseable as a date: "${val}"`,
          });
        }
      }
      // text and categorical are unrestricted on paste.
    }
  }
  return { warnings };
}
