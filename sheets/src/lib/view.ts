/**
 * Saved view — serializable snapshot of the user's working state.
 *
 * A view encodes everything that distinguishes one workspace from
 * another: the cover field, overlay toggle, active tab, the GQL query
 * draft. It's URL-hash-shareable and persistable to localStorage.
 *
 * The bundle name is NOT part of the view — it's in the URL path. A
 * view applies to whichever bundle is currently loaded.
 */

export interface ViewSpec {
  /** Schema version — bump when ViewSpec shape changes. */
  v: 1;
  coverField?: string;
  overlayOn?: boolean;
  activeView?: "grid" | "geometry" | "charts" | "kanban" | "gallery" | "form" | "gql";
  inspectorOpen?: boolean;
  gqlQuery?: string;
  /** Sort state — column name OR the κ pseudo-field "__kappa__". */
  sortField?: string;
  sortDir?: "asc" | "desc";
  /** When true, restrict the grid to rows in the anomaly κ-class. */
  anomaliesOnly?: boolean;
}

const STORAGE_KEY = "gigi.sheets.views";
const URL_PARAM = "view";

export interface NamedView {
  id: string;
  name: string;
  /** Bundle this view was saved against. Used to filter in the drawer. */
  bundle: string;
  spec: ViewSpec;
  createdAt: number;
}

/* ── Serialization ───────────────────────────────────────────────────── */

/** Encode a ViewSpec to a URL-safe string. */
export function encodeView(spec: ViewSpec): string {
  // Strip undefineds so the URL stays compact and round-trips cleanly.
  const compact: Record<string, unknown> = { v: spec.v };
  if (spec.coverField !== undefined) compact.c = spec.coverField;
  if (spec.overlayOn !== undefined) compact.o = spec.overlayOn ? 1 : 0;
  if (spec.activeView !== undefined) compact.t = spec.activeView;
  if (spec.inspectorOpen !== undefined) compact.i = spec.inspectorOpen ? 1 : 0;
  if (spec.gqlQuery) compact.q = spec.gqlQuery;
  if (spec.sortField) compact.sf = spec.sortField;
  if (spec.sortDir) compact.sd = spec.sortDir;
  if (spec.anomaliesOnly !== undefined) compact.a = spec.anomaliesOnly ? 1 : 0;
  return base64UrlEncode(JSON.stringify(compact));
}

/** Decode a URL-safe string back to a ViewSpec, or null on parse failure. */
export function decodeView(encoded: string): ViewSpec | null {
  if (!encoded) return null;
  let raw: string;
  try {
    raw = base64UrlDecode(encoded);
  } catch {
    return null;
  }
  let obj: Record<string, unknown>;
  try {
    obj = JSON.parse(raw);
  } catch {
    return null;
  }
  if (obj.v !== 1) return null;
  const spec: ViewSpec = { v: 1 };
  if (typeof obj.c === "string") spec.coverField = obj.c;
  if (obj.o === 1) spec.overlayOn = true;
  else if (obj.o === 0) spec.overlayOn = false;
  const t = obj.t;
  if (
    t === "grid" ||
    t === "geometry" ||
    t === "charts" ||
    t === "kanban" ||
    t === "gallery" ||
    t === "form" ||
    t === "gql"
  )
    spec.activeView = t;
  if (obj.i === 1) spec.inspectorOpen = true;
  else if (obj.i === 0) spec.inspectorOpen = false;
  if (typeof obj.q === "string") spec.gqlQuery = obj.q;
  if (typeof obj.sf === "string") spec.sortField = obj.sf;
  if (obj.sd === "asc" || obj.sd === "desc") spec.sortDir = obj.sd;
  if (obj.a === 1) spec.anomaliesOnly = true;
  else if (obj.a === 0) spec.anomaliesOnly = false;
  return spec;
}

/** Read the URL fragment, returning a spec if `?view=…` is present. */
export function viewFromUrl(search: string): ViewSpec | null {
  const params = new URLSearchParams(search.startsWith("?") ? search.slice(1) : search);
  const v = params.get(URL_PARAM);
  if (!v) return null;
  return decodeView(v);
}

/** Build a URL search string `?view=…` for sharing. */
export function urlForView(spec: ViewSpec): string {
  const encoded = encodeView(spec);
  return `?${URL_PARAM}=${encoded}`;
}

/* ── Persistence (localStorage) ──────────────────────────────────────── */

interface StorageShape {
  v: 1;
  views: NamedView[];
}

function readStorage(): NamedView[] {
  if (typeof localStorage === "undefined") return [];
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw) as StorageShape;
    if (parsed?.v !== 1 || !Array.isArray(parsed.views)) return [];
    return parsed.views;
  } catch {
    return [];
  }
}

function writeStorage(views: NamedView[]): void {
  if (typeof localStorage === "undefined") return;
  const payload: StorageShape = { v: 1, views };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
}

export function listViews(bundle?: string): NamedView[] {
  const all = readStorage();
  if (!bundle) return all;
  return all.filter((v) => v.bundle === bundle);
}

export function saveView(input: {
  name: string;
  bundle: string;
  spec: ViewSpec;
}): NamedView {
  const all = readStorage();
  const id = `v_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
  const v: NamedView = {
    id,
    name: input.name,
    bundle: input.bundle,
    spec: input.spec,
    createdAt: Date.now(),
  };
  all.unshift(v);
  writeStorage(all);
  return v;
}

export function deleteView(id: string): void {
  const next = readStorage().filter((v) => v.id !== id);
  writeStorage(next);
}

/* ── base64url helpers ───────────────────────────────────────────────── */

function base64UrlEncode(s: string): string {
  const bytes = new TextEncoder().encode(s);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function base64UrlDecode(s: string): string {
  let b64 = s.replace(/-/g, "+").replace(/_/g, "/");
  while (b64.length % 4) b64 += "=";
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return new TextDecoder().decode(bytes);
}
