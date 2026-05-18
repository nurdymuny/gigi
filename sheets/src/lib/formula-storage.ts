/**
 * Formula sidecar storage.
 *
 * The bundle row holds only the *evaluated value* of a formula — the
 * displayed number/string the engine, GQL console, and SDK all see.
 * The raw formula text (`=A1+B1`) lives here, keyed by
 * `(bundleName, rowKey, fieldName)`.
 *
 * See FORMULAS_SPEC §"Formula cell semantics" for the rationale: this
 * keeps the engine pure — pure-data clients see plain values, and a
 * round-trip through the GQL console doesn't lose formulas because
 * re-importing into a fresh Sheets session reattaches them via this
 * sidecar.
 *
 * Persistence target: `localStorage` for v1. The view-storage migration
 * (engine-side, sync across devices) is a single helper-swap away.
 */

const STORAGE_KEY = "gigi.sheets.formulas";

/**
 * One sidecar entry. `text` is always a formula (starts with `=`).
 * Anything that isn't a formula goes in the bundle row, not here.
 */
export interface FormulaEntry {
  bundle: string;
  rowKey: string;
  field: string;
  text: string;
}

/**
 * Flat-map storage shape. Keyed by an escaped composite of
 * `(bundle, rowKey, field)` — see `composeKey` for the escape rule.
 * Versioned so the migration to engine-side storage is a one-liner.
 */
interface StorageShape {
  v: 1;
  map: Record<string, string>;
}

function read(): StorageShape {
  if (typeof localStorage === "undefined") return { v: 1, map: {} };
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return { v: 1, map: {} };
  try {
    const parsed = JSON.parse(raw) as StorageShape;
    if (parsed?.v !== 1 || typeof parsed.map !== "object" || parsed.map === null) {
      return { v: 1, map: {} };
    }
    return parsed;
  } catch {
    return { v: 1, map: {} };
  }
}

function write(state: StorageShape): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

/**
 * Escape a single key segment so the field separator (`:`) inside a
 * bundle name, row key, or field name can't be mistaken for a delimiter.
 * Backslash is the escape char: `\` → `\\`, `:` → `\:`.
 *
 * Without this, a rowKey of `"r:amount"` and field of `"x"` would hash
 * to the same composite as rowKey `"r"`, field `"amount:x"` — corrupting
 * adjacent cells silently.
 */
function escapeSegment(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/:/g, "\\:");
}

function composeKey(bundle: string, rowKey: string, field: string): string {
  return `${escapeSegment(bundle)}:${escapeSegment(rowKey)}:${escapeSegment(field)}`;
}

/** Split a composite key back into its three segments. */
function decomposeKey(key: string): { bundle: string; rowKey: string; field: string } | null {
  // Walk char-by-char, splitting on un-escaped `:`. We expect exactly 3
  // segments. Anything else is a corrupt key — return null so the caller
  // can skip it.
  const segments: string[] = [];
  let buf = "";
  for (let i = 0; i < key.length; i++) {
    const c = key[i];
    if (c === "\\" && i + 1 < key.length) {
      buf += key[i + 1];
      i++;
      continue;
    }
    if (c === ":") {
      segments.push(buf);
      buf = "";
      continue;
    }
    buf += c;
  }
  segments.push(buf);
  if (segments.length !== 3) return null;
  return { bundle: segments[0], rowKey: segments[1], field: segments[2] };
}

/** Read the raw formula text for a cell, or null if none is stored. */
export function getFormula(bundle: string, rowKey: string, field: string): string | null {
  const state = read();
  return state.map[composeKey(bundle, rowKey, field)] ?? null;
}

/**
 * Store a formula. If `text` doesn't start with `=` (or is empty), the
 * slot is cleared — only formulas live in the sidecar; plain values
 * belong in the bundle row.
 */
export function setFormula(
  bundle: string,
  rowKey: string,
  field: string,
  text: string,
): void {
  const state = read();
  const key = composeKey(bundle, rowKey, field);
  if (!text || !text.startsWith("=")) {
    delete state.map[key];
  } else {
    state.map[key] = text;
  }
  write(state);
}

/** Remove the formula for a specific cell. No-op if none stored. */
export function clearFormula(bundle: string, rowKey: string, field: string): void {
  const state = read();
  const key = composeKey(bundle, rowKey, field);
  if (key in state.map) {
    delete state.map[key];
    write(state);
  }
}

/**
 * Enumerate stored formulas. If `bundle` is provided, only entries
 * matching that bundle name are returned. Useful for hydrating the
 * sidecar on bundle load, or for a debug dump.
 */
export function listFormulas(bundle?: string): FormulaEntry[] {
  const state = read();
  const out: FormulaEntry[] = [];
  for (const [key, text] of Object.entries(state.map)) {
    const parts = decomposeKey(key);
    if (!parts) continue;
    if (bundle !== undefined && parts.bundle !== bundle) continue;
    out.push({ ...parts, text });
  }
  return out;
}

/**
 * Drop every formula for a bundle. Called when a bundle is deleted so
 * stale sidecar entries don't accumulate.
 */
export function clearBundleFormulas(bundle: string): void {
  const state = read();
  let changed = false;
  for (const key of Object.keys(state.map)) {
    const parts = decomposeKey(key);
    if (parts && parts.bundle === bundle) {
      delete state.map[key];
      changed = true;
    }
  }
  if (changed) write(state);
}
