/**
 * SheetsClient — thin facade over `@gigi-db/client` shaped for the grid.
 *
 * The SDK already wraps every route on `gigi-stream`; this wrapper exists to
 * (a) inject `fetch` for testability, (b) surface typed errors the UI can
 * branch on, and (c) keep the row/meta shape consistent across views.
 *
 * Endpoint contracts come from the engine audit (2026-05-14):
 *   - GET  /v1/bundles/{name}/schema     → BundleSchemaResponse
 *   - POST /v1/bundles/{name}/query      → SectionResponse
 *   - POST /v1/bundles/{name}/update     → UpdateResponse (bundle-level κ)
 *   - GET  /ws                           → SubscriptionEvent stream
 */

import { applyOverlay } from "./demo-encryption-overlay";

export type Fetcher = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

export type FieldType =
  | "text"
  | "numeric"
  | "categorical"
  | "boolean"
  | "timestamp"
  | "encrypted"
  | string; // tolerate engine-side extensions

export interface FieldDescriptor {
  name: string;
  type: FieldType;
  weight?: number;
  /** Present once E-S8a lands; absent on older bundles. */
  encryption?: "none" | "opaque" | "indexed" | "affine";
}

export interface BundleSchema {
  name: string;
  base_fields: FieldDescriptor[];
  fiber_fields: FieldDescriptor[];
  indexed_fields: string[];
  records: number;
  storage_mode: string;
}

/**
 * The engine returns field types in TitleCase (`"Numeric"`, `"Categorical"`).
 * The whole client + UI assumes lowercase. Normalize at the boundary so
 * every downstream filter (`f.type === "numeric"`) keeps working regardless
 * of which casing the engine version emits.
 */
function normalizeField(f: FieldDescriptor): FieldDescriptor {
  return { ...f, type: String(f.type).toLowerCase() as FieldType };
}

export function normalizeSchema(s: BundleSchema): BundleSchema {
  return {
    ...s,
    base_fields: (s.base_fields ?? []).map(normalizeField),
    fiber_fields: (s.fiber_fields ?? []).map(normalizeField),
  };
}

export type RowMap = Record<string, unknown>;

export interface SectionQuery {
  conditions?: Array<{ field: string; op: string; value: unknown }>;
  sort_by?: string;
  sort_desc?: boolean;
  limit?: number;
  offset?: number;
  search?: string;
  search_fields?: string[];
  fields?: string[];
}

export interface SectionResult {
  rows: RowMap[];
  total: number;
  curvature: number;
  confidence: number;
}

export interface BundleListEntry {
  name: string;
  records: number;
  fields: number;
}

export interface SpectralResult {
  lambda1: number;
  diameter: number;
  spectral_capacity: number;
}

export interface BettiResult {
  beta_0: number;
  beta_1: number;
}

export interface TransportResult {
  /** Fiber dimension N. */
  dim: number;
  /** Signed rotation angle (radians). */
  angle: number;
  /** Row-major N×N rotation matrix, length = dim². */
  matrix: number[];
}

export interface HolonomyCentroid {
  label: string;
  fx: number;
  fy: number;
  transport_angle: number;
}

export interface HolonomyResult {
  /** Signed holonomy deficit (radians). */
  angle: number;
  /** True when |angle| < engine threshold (1e-6). */
  trivial: boolean;
  centroids: HolonomyCentroid[];
}

export interface GqlResult {
  rows: Array<Record<string, unknown>>;
  count: number;
}

/**
 * Permissive GQL response shape — whatever the engine sent back, plus a
 * client-side measurement of round-trip time and the original HTTP
 * status. The editor surfaces this directly; it doesn't try to
 * coerce mutation responses into row shapes.
 */
export interface GqlRawResult {
  status: number;
  /** Body parsed as JSON, or null if non-JSON / empty. */
  body: unknown;
  /** ms from POST send to response parse. */
  elapsedMs: number;
}

export interface UpdateArgs {
  /** Primary-key fields identifying the section. */
  key: Record<string, unknown>;
  /** Fields to write. */
  fields: Record<string, unknown>;
  /** Ask the engine to RETURN the updated row in `data`. Default true. */
  returning?: boolean;
  /** Optimistic-concurrency token from a previous read. */
  expected_version?: number;
}

export interface UpdateResult {
  status: string;
  /** Present when returning=true. */
  data?: RowMap;
  total: number;
  curvature: number;
  confidence: number;
  /** Bumped after a successful write when optimistic concurrency is on. */
  version?: number;
}

export type SheetsErrorCode =
  | "http_error"
  | "timeout"
  | "network_error"
  | "parse_error"
  | "version_conflict"
  | "no_key";

export class SheetsClientError extends Error {
  readonly code: SheetsErrorCode;
  readonly status?: number;
  constructor(message: string, code: SheetsErrorCode, status?: number) {
    super(message);
    this.name = "SheetsClientError";
    this.code = code;
    this.status = status;
  }
}

export interface SheetsClientOptions {
  baseUrl: string;
  /** Inject a fetcher for tests; defaults to global fetch. */
  fetch?: Fetcher;
  /** Per-request timeout; defaults to 30s. */
  timeoutMs?: number;
  /**
   * Inject a WebSocket factory for tests. Default uses globalThis.WebSocket.
   * The factory receives the resolved ws:// URL.
   */
  WebSocket?: WebSocketFactory;
}

export type WebSocketFactory = (url: string) => MinimalWebSocket;

/**
 * Minimal WebSocket surface — everything the client touches is here so
 * tests can supply a stub without pulling in `ws` for jsdom.
 */
export interface MinimalWebSocket {
  readyState: number;
  send(data: string): void;
  close(): void;
  addEventListener(
    type: "open" | "message" | "close" | "error",
    listener: (ev: { data?: string }) => void,
  ): void;
}

export interface Subscription {
  /** Close the socket and stop receiving frames. */
  close(): void;
}

import {
  applyEventToRows as _applyEventToRows,
  parseSubscriptionFrame,
  type SubscriptionFrame,
} from "./subscribe";

export { _applyEventToRows as applyEventToRows };
export type { SubscriptionFrame };

export class SheetsClient {
  private readonly baseUrl: string;
  private readonly fetcher: Fetcher;
  private readonly timeoutMs: number;
  private readonly wsFactory: WebSocketFactory;

  constructor(opts: SheetsClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/+$/, "");
    this.fetcher = opts.fetch ?? globalThis.fetch.bind(globalThis);
    this.timeoutMs = opts.timeoutMs ?? 30_000;
    this.wsFactory = opts.WebSocket ?? defaultWebSocketFactory;
  }

  /** Derive ws:// URL from the configured base URL. */
  wsUrl(path = "/ws"): string {
    const m = this.baseUrl.match(/^https?:\/\/(.+)$/);
    const proto = this.baseUrl.startsWith("https") ? "wss" : "ws";
    const hostPath = m ? m[1] : this.baseUrl;
    return `${proto}://${hostPath}${path}`;
  }

  /**
   * Open a WebSocket subscription to a bundle.
   * Calls `onFrame` for each `EVENT` / `NOTICE` line that parses; calls
   * `onStatus` on open/close/error transitions. Returns a `Subscription`
   * the caller closes when done.
   */
  subscribe(
    bundle: string,
    onFrame: (frame: SubscriptionFrame) => void,
    onStatus?: (status: "open" | "close" | "error") => void,
  ): Subscription {
    const ws = this.wsFactory(this.wsUrl("/ws"));
    ws.addEventListener("open", () => {
      onStatus?.("open");
      // The engine accepts the SUBSCRIBE command as a single line.
      ws.send(`SUBSCRIBE ${bundle}`);
    });
    ws.addEventListener("message", (ev) => {
      const data = typeof ev.data === "string" ? ev.data : "";
      for (const line of data.split("\n")) {
        const frame = parseSubscriptionFrame(line);
        if (frame) onFrame(frame);
      }
    });
    ws.addEventListener("close", () => onStatus?.("close"));
    ws.addEventListener("error", () => onStatus?.("error"));
    return {
      close: () => ws.close(),
    };
  }

  /**
   * Fetch sections (rows) from a bundle. Maps to POST /v1/bundles/{name}/query.
   * The response shape is normalized: rows come from `data`/`results`,
   * with κ and confidence promoted to top-level fields.
   */
  async section(bundle: string, query: SectionQuery = {}): Promise<SectionResult> {
    const url = `${this.baseUrl}/v1/bundles/${encodeURIComponent(bundle)}/query`;
    const body = await this.request(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(query),
    });
    return normalizeSection(body);
  }

  /**
   * Update a single section by key. Maps to POST /v1/bundles/{name}/update.
   *
   * Response carries the new bundle-level κ and confidence. Cohort κ deltas
   * are not yet returned by the engine (see E-S1a in the addendum) —
   * recompute is currently client-side in S2.
   */
  async update(bundle: string, args: UpdateArgs): Promise<UpdateResult> {
    if (!args.key || Object.keys(args.key).length === 0) {
      throw new SheetsClientError(
        "update() requires a non-empty key",
        "no_key",
      );
    }
    const url = `${this.baseUrl}/v1/bundles/${encodeURIComponent(bundle)}/update`;
    const body = await this.request(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        key: args.key,
        fields: args.fields,
        returning: args.returning ?? true,
        ...(args.expected_version !== undefined
          ? { expected_version: args.expected_version }
          : {}),
      }),
    });
    return normalizeUpdate(body);
  }

  /**
   * Spectral report — top eigenvalue, diameter, capacity.
   * Maps to GET /v1/bundles/{name}/spectral.
   */
  async spectral(bundle: string): Promise<SpectralResult> {
    const url = `${this.baseUrl}/v1/bundles/${encodeURIComponent(bundle)}/spectral`;
    const body = await this.request(url, { method: "GET" });
    const obj = body as Record<string, unknown>;
    return {
      lambda1: numOr(obj.lambda1, 0),
      diameter: numOr(obj.diameter, 0),
      spectral_capacity: numOr(obj.spectral_capacity, 0),
    };
  }

  /**
   * Betti numbers (sheaf cohomology). Engine returns b₀ and b₁ only —
   * b₂ is not currently computed.
   * Maps to GET /v1/bundles/{name}/betti.
   */
  async betti(bundle: string): Promise<BettiResult> {
    const url = `${this.baseUrl}/v1/bundles/${encodeURIComponent(bundle)}/betti`;
    const body = await this.request(url, { method: "GET" });
    const obj = body as Record<string, unknown>;
    return {
      beta_0: numOr(obj.beta_0, 0),
      beta_1: numOr(obj.beta_1, 0),
    };
  }

  /**
   * Execute a GQL query and return the raw engine response. Unlike
   * `gql()`, this does NOT coerce the body into a row shape — useful
   * for the GQL editor where the user might issue mutations, scalar
   * queries, or anything else the engine accepts.
   *
   * On HTTP errors, returns the response with `status` set rather than
   * throwing. JSON parse failures leave `body: null`.
   */
  async gqlRaw(query: string): Promise<GqlRawResult> {
    const url = `${this.baseUrl}/v1/gql`;
    const t0 = performance.now();
    const ac = new AbortController();
    const timer = setTimeout(() => ac.abort(), this.timeoutMs);
    let res: Response;
    try {
      res = await this.fetcher(url, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ query }),
        signal: ac.signal,
      });
    } catch (err) {
      const name = (err as { name?: string })?.name;
      if (name === "AbortError") {
        throw new SheetsClientError(
          `Request timed out after ${this.timeoutMs}ms`,
          "timeout",
        );
      }
      throw new SheetsClientError(
        (err as Error).message ?? "Network error",
        "network_error",
      );
    } finally {
      clearTimeout(timer);
    }
    let body: unknown = null;
    try {
      body = await res.json();
    } catch {
      body = null;
    }
    return { status: res.status, body, elapsedMs: performance.now() - t0 };
  }

  /** Execute a raw GQL query. Returns `{ rows, count }`. */
  async gql(query: string): Promise<GqlResult> {
    const url = `${this.baseUrl}/v1/gql`;
    const body = await this.request(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ query }),
    });
    const obj = body as Record<string, unknown>;
    const rows = Array.isArray(obj.rows) ? (obj.rows as Array<Record<string, unknown>>) : [];
    return { rows, count: numOr(obj.count, rows.length) };
  }

  /**
   * Parallel-transport an N-dim fiber between two records.
   * Issues:
   *   TRANSPORT <bundle> FROM (k1=v1) TO (k1=v1) ON FIBER (f1, f2[, …]);
   */
  async transport(
    bundle: string,
    from: Record<string, unknown>,
    to: Record<string, unknown>,
    fiberFields: string[],
  ): Promise<TransportResult> {
    const fromExpr = formatKeyExpr(from);
    const toExpr = formatKeyExpr(to);
    const fiberExpr = fiberFields.map(quoteIdent).join(", ");
    const q = `TRANSPORT ${quoteIdent(bundle)} FROM (${fromExpr}) TO (${toExpr}) ON FIBER (${fiberExpr});`;
    const { rows } = await this.gql(q);
    if (rows.length === 0) {
      throw new SheetsClientError("TRANSPORT returned no rows", "parse_error");
    }
    const row = rows[0];
    const matrix = Array.isArray(row.matrix) ? (row.matrix as number[]) : [];
    return {
      dim: numOr(row.dim, Math.round(Math.sqrt(matrix.length))),
      angle: numOr(row.angle, 0),
      matrix,
    };
  }

  /**
   * Holonomy around a categorical loop.
   * Issues:
   *   HOLONOMY <bundle> ON FIBER (f1, f2) AROUND <field>;
   */
  async holonomy(
    bundle: string,
    fiberFields: string[],
    aroundField: string,
  ): Promise<HolonomyResult> {
    if (fiberFields.length < 2) {
      throw new SheetsClientError(
        "HOLONOMY requires at least 2 fiber fields",
        "parse_error",
      );
    }
    const fiberExpr = fiberFields.slice(0, 2).map(quoteIdent).join(", ");
    const q = `HOLONOMY ${quoteIdent(bundle)} ON FIBER (${fiberExpr}) AROUND ${quoteIdent(aroundField)};`;
    const { rows } = await this.gql(q);
    // The engine returns one row per centroid + one trailing "summary" row.
    let angle = 0;
    let trivial = true;
    const centroids: HolonomyCentroid[] = [];
    for (const row of rows) {
      if (row._type === "summary") {
        angle = numOr(row.holonomy_angle, 0);
        trivial = Boolean(row.holonomy_trivial ?? true);
        continue;
      }
      centroids.push({
        label: String(row[aroundField] ?? ""),
        fx: numOr(row[fiberFields[0]], 0),
        fy: numOr(row[fiberFields[1]], 0),
        transport_angle: numOr(row.transport_angle, 0),
      });
    }
    return { angle, trivial, centroids };
  }

  /**
   * List all bundles available on the server.
   * Maps to GET /v1/bundles.
   */
  async listBundles(): Promise<BundleListEntry[]> {
    const url = `${this.baseUrl}/v1/bundles`;
    const body = await this.request(url, { method: "GET" });
    if (!Array.isArray(body)) {
      throw new SheetsClientError(
        "/v1/bundles did not return an array",
        "parse_error",
      );
    }
    return body
      .map((entry): BundleListEntry => {
        const obj = entry as Record<string, unknown>;
        return {
          name: String(obj.name ?? ""),
          records: numOr(obj.records, 0),
          fields: numOr(obj.fields, 0),
        };
      })
      .filter((b) => b.name.length > 0);
  }

  /**
   * Create a new bundle.
   * Maps to POST /v1/bundles.
   *
   * The engine's expected shape (verified against gigi-stream):
   *   { name, schema: { fields: { col: type, ... }, keys: ["col"], indexed: [] } }
   */
  async createBundle(args: {
    name: string;
    fields: Record<string, string>;
    keys: string[];
    indexed?: string[];
  }): Promise<void> {
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(args.name)) {
      throw new SheetsClientError(
        `Bundle name '${args.name}' must match [A-Za-z_][A-Za-z0-9_]*`,
        "parse_error",
      );
    }
    const url = `${this.baseUrl}/v1/bundles`;
    await this.request(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        name: args.name,
        schema: {
          fields: args.fields,
          keys: args.keys,
          indexed: args.indexed ?? [],
        },
      }),
    });
  }

  /**
   * Insert one or many sections.
   * Maps to POST /v1/bundles/{name}/insert with { records: [...] }.
   */
  async insert(bundle: string, records: RowMap | RowMap[]): Promise<void> {
    const list = Array.isArray(records) ? records : [records];
    const url = `${this.baseUrl}/v1/bundles/${encodeURIComponent(bundle)}/insert`;
    await this.request(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ records: list }),
    });
  }

  /**
   * Delete a section by key.
   * Maps to POST /v1/bundles/{name}/delete with { key: { … } }.
   */
  async deleteRow(bundle: string, key: Record<string, unknown>): Promise<void> {
    const url = `${this.baseUrl}/v1/bundles/${encodeURIComponent(bundle)}/delete`;
    await this.request(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ key }),
    });
  }

  /**
   * Add a field to an existing bundle.
   * Maps to POST /v1/bundles/{name}/add-field.
   */
  async addField(
    bundle: string,
    args: { name: string; type: string; default?: unknown },
  ): Promise<void> {
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(args.name)) {
      throw new SheetsClientError(
        `Field name '${args.name}' must match [A-Za-z_][A-Za-z0-9_]*`,
        "parse_error",
      );
    }
    const url = `${this.baseUrl}/v1/bundles/${encodeURIComponent(bundle)}/add-field`;
    await this.request(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(args),
    });
  }

  /**
   * Drop a field from an existing bundle.
   * Maps to POST /v1/bundles/{name}/drop-field.
   */
  async dropField(bundle: string, field: string): Promise<void> {
    const url = `${this.baseUrl}/v1/bundles/${encodeURIComponent(bundle)}/drop-field`;
    await this.request(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ field }),
    });
  }

  /** Fetch a bundle's schema. Maps to GET /v1/bundles/{name}/schema. */
  async schema(bundle: string): Promise<BundleSchema> {
    const url = `${this.baseUrl}/v1/bundles/${encodeURIComponent(bundle)}/schema`;
    const body = await this.request(url, { method: "GET" });
    return applyOverlay(normalizeSchema(body as BundleSchema));
  }

  private async request(url: string, init: RequestInit): Promise<unknown> {
    const ac = new AbortController();
    const timer = setTimeout(() => ac.abort(), this.timeoutMs);
    let res: Response;
    try {
      res = await this.fetcher(url, { ...init, signal: ac.signal });
    } catch (err) {
      const name = (err as { name?: string })?.name;
      if (name === "AbortError") {
        throw new SheetsClientError(
          `Request timed out after ${this.timeoutMs}ms`,
          "timeout",
        );
      }
      throw new SheetsClientError(
        (err as Error).message ?? "Network error",
        "network_error",
      );
    } finally {
      clearTimeout(timer);
    }
    if (!res.ok) {
      throw new SheetsClientError(`HTTP ${res.status}`, "http_error", res.status);
    }
    try {
      return await res.json();
    } catch {
      throw new SheetsClientError("Invalid JSON response", "parse_error");
    }
  }
}

function normalizeUpdate(body: unknown): UpdateResult {
  if (typeof body !== "object" || body === null) {
    throw new SheetsClientError("Update response is not an object", "parse_error");
  }
  const obj = body as Record<string, unknown>;
  const status = typeof obj.status === "string" ? obj.status : "updated";
  if (status === "version_conflict" || status === "conflict") {
    throw new SheetsClientError(
      "Concurrent write detected (version mismatch)",
      "version_conflict",
    );
  }
  return {
    status,
    data: (obj.data as RowMap | undefined) ?? undefined,
    total: numOr(obj.total, 0),
    curvature: numOr(obj.curvature, 0),
    confidence: numOr(obj.confidence, 0),
    version: typeof obj.version === "number" ? obj.version : undefined,
  };
}

function normalizeSection(body: unknown): SectionResult {
  if (typeof body !== "object" || body === null) {
    throw new SheetsClientError("Section response is not an object", "parse_error");
  }
  const obj = body as Record<string, unknown>;
  const rowsRaw = obj.data ?? obj.results ?? obj.records ?? [];
  if (!Array.isArray(rowsRaw)) {
    throw new SheetsClientError("Section rows are not an array", "parse_error");
  }
  return {
    rows: rowsRaw as RowMap[],
    total: numOr(obj.total, rowsRaw.length),
    curvature: numOr(obj.curvature, 0),
    confidence: numOr(obj.confidence, 0),
  };
}

function defaultWebSocketFactory(url: string): MinimalWebSocket {
  const Ctor = (globalThis as { WebSocket?: typeof WebSocket }).WebSocket;
  if (!Ctor) {
    throw new SheetsClientError(
      "WebSocket is not available in this environment",
      "network_error",
    );
  }
  return new Ctor(url) as unknown as MinimalWebSocket;
}

function numOr(v: unknown, fallback: number): number {
  return typeof v === "number" && Number.isFinite(v) ? v : fallback;
}

/** Render a GQL identifier (column / bundle name). Strict: must be [A-Za-z_][A-Za-z0-9_]*. */
function quoteIdent(ident: string): string {
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(ident)) {
    throw new SheetsClientError(
      `Unsafe identifier: ${JSON.stringify(ident)}`,
      "parse_error",
    );
  }
  return ident;
}

/** Render a `(field=value, ...)` key expression for GQL, escaping strings. */
function formatKeyExpr(keys: Record<string, unknown>): string {
  return Object.entries(keys)
    .map(([k, v]) => `${quoteIdent(k)}=${gqlLiteral(v)}`)
    .join(", ");
}

function gqlLiteral(v: unknown): string {
  if (v === null || v === undefined) return "NULL";
  if (typeof v === "number") return String(v);
  if (typeof v === "boolean") return v ? "TRUE" : "FALSE";
  // Strings — escape single quotes.
  const s = String(v).replace(/'/g, "''");
  return `'${s}'`;
}
