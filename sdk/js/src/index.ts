/**
 * @gigi-db/client — JavaScript SDK for GIGI Stream & GIGI Edge
 *
 * Speaks DHOOM wire protocol over WebSocket, with REST fallback.
 * Every query returns curvature metadata (confidence, K, capacity).
 *
 * Usage:
 *   const db = new GIGIClient('http://localhost:3142');
 *   await db.bundle('sensors').create({ ... });
 *   await db.bundle('sensors').insert({ sensor_id: 'T-001', ... });
 *   const result = await db.bundle('sensors').get({ sensor_id: 'T-001' });
 *   console.log(result.data, result.confidence, result.curvature);
 */

// ── Types ──

export interface BundleSchema {
  fields: Record<string, string>;
  keys?: string[];
  defaults?: Record<string, unknown>;
  indexed?: string[];
}

export interface InsertResult {
  status: string;
  count: number;
  total: number;
  curvature: number;
  confidence: number;
}

export interface UpdateResult {
  status: string;
  total: number;
  curvature: number;
  confidence: number;
  /** Returned when using RETURNING clause */
  data?: Record<string, unknown>;
  /** Returned when using optimistic concurrency */
  version?: number;
}

export interface DeleteResult {
  status: string;
  total: number;
  curvature: number;
  confidence: number;
  /** Returned when using RETURNING clause */
  data?: Record<string, unknown>;
}

export interface QueryCondition {
  field: string;
  op: 'eq' | 'neq' | 'gt' | 'gte' | 'lt' | 'lte' | 'contains' | 'starts_with'
    | 'ends_with' | 'regex' | 'in' | 'not_in' | 'is_null' | 'is_not_null';
  value: unknown;
}

export interface SortSpec {
  field: string;
  desc?: boolean;
}

export interface FilteredQueryOptions {
  conditions?: QueryCondition[];
  /** Alias for conditions (PRISM compat) */
  filters?: QueryCondition[];
  sort_by?: string;
  /** Alias for sort_by (PRISM compat) */
  order_by?: string;
  sort_desc?: boolean;
  /** "desc" or "asc" (PRISM compat) */
  order?: 'desc' | 'asc';
  limit?: number;
  offset?: number;
  /** Multi-field text search (OR across search_fields) */
  search?: string;
  /** Which fields to search across (default: all text fields) */
  search_fields?: string[];
  /** Field projection — only return these fields */
  fields?: string[];
  /** OR condition groups — each inner array is ANDed, outer array is ORed */
  or_conditions?: QueryCondition[][];
  /** Multi-field sort */
  sort?: SortSpec[];
}

export interface BulkUpdateOptions {
  filter: QueryCondition[];
  fields: Record<string, unknown>;
}

export interface BulkUpdateResult {
  status: string;
  matched: number;
  total: number;
  curvature: number;
  confidence: number;
}

export interface ListAllOptions {
  limit?: number;
  offset?: number;
}

export interface QueryResult<T = Record<string, unknown>> {
  data: T;
  meta: {
    confidence: number;
    curvature: number;
    capacity: number;
  };
}

export interface RangeResult<T = Record<string, unknown>> {
  data: T[];
  count: number;
  meta: {
    confidence: number;
    curvature: number;
  };
}

export interface CurvatureReport {
  K: number;
  confidence: number;
  capacity: number;
  per_field: Array<{
    field: string;
    variance: number;
    range: number;
    k: number;
  }>;
}

export interface SpectralReport {
  lambda1: number;
  diameter: number;
  spectral_capacity: number;
}

export interface ConsistencyReport {
  h1: number;
  cocycles: unknown[];
}

export interface AggregateResult {
  groups: Record<
    string,
    {
      count: number;
      sum: number;
      avg: number;
      min: number;
      max: number;
    }
  >;
}

export interface JoinResult {
  data: Array<{ left: Record<string, unknown>; right: Record<string, unknown> }>;
  count: number;
}

export interface SyncReport {
  status: string;
  pushed: number;
  pulled: number;
  h1: number;
  conflicts: number;
  timestamp: number;
}

export interface BundleStats {
  name: string;
  record_count: number;
  base_fields: number;
  fiber_fields: number;
  indexed_fields: string[];
  storage_mode: string;
  index_sizes: Record<string, number>;
  field_cardinalities: Record<string, number>;
  field_stats: Record<string, unknown>;
  curvature: number;
}

export interface QueryPlan {
  scan_type: string;
  total_records: number;
  index_scans: string[];
  full_scan_conditions: string[];
  or_group_count: number;
  has_sort: boolean;
  has_limit: boolean;
  has_offset: boolean;
  storage_mode: string;
}

export interface TransactionOp {
  op: 'insert' | 'update' | 'delete' | 'increment';
  record?: Record<string, unknown>;
  key?: Record<string, unknown>;
  fields?: Record<string, unknown>;
  field?: string;
  amount?: number;
}

export interface TransactionResult {
  status: string;
  results: string[];
  total: number;
  curvature: number;
}

export interface UpdateOptions {
  /** Return the updated record in the response */
  returning?: boolean;
  /** Optimistic concurrency — only update if _version matches */
  expectedVersion?: number;
}

export interface SubscriptionCallback<T = Record<string, unknown>> {
  (records: Array<QueryResult<T>>): void;
}

// ── Bundle Handle ──

export class BundleHandle {
  private client: GIGIClient;
  private name: string;
  private _whereField?: string;
  private _whereValue?: string;

  constructor(client: GIGIClient, name: string) {
    this.client = client;
    this.name = name;
  }

  /** Create the bundle with a schema. */
  async create(schema: BundleSchema): Promise<{ status: string; bundle: string }> {
    return this.client.request("POST", "/v1/bundles", {
      name: this.name,
      schema,
    });
  }

  /** Drop the bundle. */
  async drop(): Promise<void> {
    return this.client.request("DELETE", `/v1/bundles/${this.name}`);
  }

  /** Insert one or more records. O(1) per record. */
  async insert(
    records: Record<string, unknown> | Record<string, unknown>[]
  ): Promise<InsertResult> {
    const arr = Array.isArray(records) ? records : [records];
    return this.client.request("POST", `/v1/bundles/${this.name}/insert`, {
      records: arr,
    });
  }

  /** Update a record — partial field patches. O(1). Supports RETURNING and optimistic concurrency. */
  async update(
    key: Record<string, unknown>,
    fields: Record<string, unknown>,
    opts?: UpdateOptions
  ): Promise<UpdateResult> {
    return this.client.request("POST", `/v1/bundles/${this.name}/update`, {
      key,
      fields,
      returning: opts?.returning ?? false,
      expected_version: opts?.expectedVersion,
    });
  }

  /** Delete a record by key. O(1). Supports RETURNING. */
  async deleteRecord(
    key: Record<string, unknown>,
    opts?: { returning?: boolean }
  ): Promise<DeleteResult> {
    return this.client.request("POST", `/v1/bundles/${this.name}/delete`, {
      key,
      returning: opts?.returning ?? false,
    });
  }

  /** Filtered query with conditions, sort, limit, offset. */
  async query(opts: FilteredQueryOptions): Promise<RangeResult> {
    return this.client.request("POST", `/v1/bundles/${this.name}/query`, opts);
  }

  /** List all records in the bundle. */
  async listAll(opts?: ListAllOptions): Promise<RangeResult> {
    const params = new URLSearchParams();
    if (opts?.limit !== undefined) params.set('limit', String(opts.limit));
    if (opts?.offset !== undefined) params.set('offset', String(opts.offset));
    const qs = params.toString();
    const path = `/v1/bundles/${this.name}/points${qs ? '?' + qs : ''}`;
    return this.client.request('GET', path);
  }

  /** Get a record by field/value (URL-path style). */
  async getByField(field: string, value: string | number): Promise<QueryResult> {
    return this.client.request(
      'GET',
      `/v1/bundles/${this.name}/points/${encodeURIComponent(field)}/${encodeURIComponent(String(value))}`
    );
  }

  /** Update a record by field/value (PATCH style). */
  async updateByField(
    field: string,
    value: string | number,
    fields: Record<string, unknown>
  ): Promise<UpdateResult> {
    return this.client.request(
      'PATCH',
      `/v1/bundles/${this.name}/points/${encodeURIComponent(field)}/${encodeURIComponent(String(value))}`,
      { fields }
    );
  }

  /** Delete a record by field/value (DELETE style). */
  async deleteByField(field: string, value: string | number): Promise<DeleteResult> {
    return this.client.request(
      'DELETE',
      `/v1/bundles/${this.name}/points/${encodeURIComponent(field)}/${encodeURIComponent(String(value))}`
    );
  }

  /** Bulk update: update all records matching filter conditions. */
  async bulkUpdate(opts: BulkUpdateOptions): Promise<BulkUpdateResult> {
    return this.client.request('PATCH', `/v1/bundles/${this.name}/points`, opts);
  }

  /** Upsert — insert if not exists, update if exists. */
  async upsert(record: Record<string, unknown>): Promise<UpdateResult> {
    return this.client.request('POST', `/v1/bundles/${this.name}/upsert`, { record });
  }

  /** Count records matching filter conditions. */
  async count(conditions?: QueryCondition[]): Promise<{ count: number; total: number }> {
    return this.client.request('POST', `/v1/bundles/${this.name}/count`, {
      conditions: conditions || [],
    });
  }

  /** Check if any record matches the conditions. */
  async exists(conditions: QueryCondition[]): Promise<{ exists: boolean }> {
    return this.client.request('POST', `/v1/bundles/${this.name}/exists`, {
      conditions,
    });
  }

  /** Get distinct values for a field. */
  async distinct(field: string): Promise<{ field: string; values: unknown[]; count: number }> {
    return this.client.request('GET', `/v1/bundles/${this.name}/distinct/${encodeURIComponent(field)}`);
  }

  /** Bulk delete: remove all records matching conditions. */
  async bulkDelete(conditions: QueryCondition[]): Promise<DeleteResult & { deleted: number }> {
    return this.client.request('POST', `/v1/bundles/${this.name}/bulk-delete`, {
      conditions,
    });
  }

  /** Truncate — delete all records in the bundle. */
  async truncate(): Promise<{ status: string; removed: number; total: number }> {
    return this.client.request('POST', `/v1/bundles/${this.name}/truncate`, {});
  }

  /** Get bundle schema info. */
  async schema(): Promise<Record<string, unknown>> {
    return this.client.request('GET', `/v1/bundles/${this.name}/schema`);
  }

  /** Atomic increment/decrement a numeric field. */
  async increment(
    key: Record<string, unknown>,
    field: string,
    amount: number = 1
  ): Promise<UpdateResult> {
    return this.client.request('POST', `/v1/bundles/${this.name}/increment`, {
      key,
      field,
      amount,
    });
  }

  /** Add a fiber field to the bundle schema. */
  async addField(
    name: string,
    type: string = 'categorical',
    defaultValue?: unknown
  ): Promise<Record<string, unknown>> {
    return this.client.request('POST', `/v1/bundles/${this.name}/add-field`, {
      name,
      type,
      default: defaultValue,
    });
  }

  /** Add an index on a field. */
  async addIndex(field: string): Promise<Record<string, unknown>> {
    return this.client.request('POST', `/v1/bundles/${this.name}/add-index`, {
      field,
    });
  }

  /** Export all records as JSON. */
  async export(): Promise<{ bundle: string; count: number; records: Record<string, unknown>[] }> {
    return this.client.request('GET', `/v1/bundles/${this.name}/export`);
  }

  /** Import records from JSON array. */
  async importData(records: Record<string, unknown>[]): Promise<InsertResult> {
    return this.client.request('POST', `/v1/bundles/${this.name}/import`, {
      records,
    });
  }

  /** Get bundle statistics (record count, fields, indexes, cardinalities, curvature). */
  async stats(): Promise<BundleStats> {
    return this.client.request('GET', `/v1/bundles/${this.name}/stats`);
  }

  /** Get query execution plan without running the query. */
  async explain(opts: {
    conditions?: QueryCondition[];
    or_conditions?: QueryCondition[][];
    sort?: SortSpec[];
    limit?: number;
    offset?: number;
  }): Promise<QueryPlan> {
    return this.client.request('POST', `/v1/bundles/${this.name}/explain`, opts);
  }

  /** Execute multiple operations atomically (all-or-nothing). */
  async transaction(ops: TransactionOp[]): Promise<TransactionResult> {
    return this.client.request('POST', `/v1/bundles/${this.name}/transaction`, {
      ops,
    });
  }

  /** Point query — O(1). Returns record with curvature metadata. */
  async get(key: Record<string, unknown>): Promise<QueryResult> {
    const params = new URLSearchParams();
    for (const [k, v] of Object.entries(key)) {
      params.set(k, String(v));
    }
    return this.client.request(
      "GET",
      `/v1/bundles/${this.name}/get?${params.toString()}`
    );
  }

  /** Range query — returns a chainable handle for .subscribe(). */
  where(field: string, value: unknown): BundleHandle {
    const handle = new BundleHandle(this.client, this.name);
    handle._whereField = field;
    handle._whereValue = String(value);
    return handle;
  }

  /** Execute a range query. */
  async range(
    field?: string,
    value?: unknown
  ): Promise<RangeResult> {
    const f = field || this._whereField;
    const v = value !== undefined ? value : this._whereValue;
    if (!f || v === undefined) throw new Error("No query parameters");
    return this.client.request(
      "GET",
      `/v1/bundles/${this.name}/range?${f}=${encodeURIComponent(String(v))}`
    );
  }

  /** Pullback join — O(|left|). */
  async join(
    rightBundle: BundleHandle | string,
    leftField: string,
    rightField: string
  ): Promise<JoinResult> {
    const rightName =
      typeof rightBundle === "string" ? rightBundle : rightBundle.name;
    return this.client.request("POST", `/v1/bundles/${this.name}/join`, {
      right_bundle: rightName,
      left_field: leftField,
      right_field: rightField,
    });
  }

  /** Fiber integral — GROUP BY aggregation with optional pre-filter. */
  async aggregate(opts: {
    groupBy: string;
    field: string;
    conditions?: QueryCondition[];
  }): Promise<AggregateResult> {
    return this.client.request("POST", `/v1/bundles/${this.name}/aggregate`, {
      group_by: opts.groupBy,
      field: opts.field,
      conditions: opts.conditions
        ? opts.conditions.map((c) => ({
            field: c.field,
            op: c.op,
            value: c.value,
          }))
        : [],
    });
  }

  /** Curvature report. */
  async curvature(): Promise<CurvatureReport> {
    return this.client.request(
      "GET",
      `/v1/bundles/${this.name}/curvature`
    );
  }

  /** Spectral gap analysis. */
  async spectral(): Promise<SpectralReport> {
    return this.client.request(
      "GET",
      `/v1/bundles/${this.name}/spectral`
    );
  }

  /** Čech cohomology consistency check. */
  async checkConsistency(): Promise<ConsistencyReport> {
    return this.client.request(
      "GET",
      `/v1/bundles/${this.name}/consistency`
    );
  }

  /** Subscribe to real-time updates via WebSocket. */
  subscribe(callback: SubscriptionCallback): () => void {
    const field = this._whereField;
    const value = this._whereValue;

    const ws = this.client.getWebSocket();
    if (!ws) {
      console.warn("WebSocket not available, subscription not started");
      return () => {};
    }

    const command = field
      ? `SUBSCRIBE ${this.name} WHERE ${field} = "${value}"`
      : `SUBSCRIBE ${this.name}`;

    ws.send(command);

    const handler = (event: MessageEvent) => {
      const data = String(event.data);
      if (data.startsWith("SUBSCRIPTION") && data.includes(this.name)) {
        try {
          // Parse DHOOM subscription response
          const records = this.parseDhoomResponse(data);
          callback(records);
        } catch {
          // Non-DHOOM response, try JSON
          try {
            const json = JSON.parse(data);
            callback([json]);
          } catch {
            // ignore unparseable messages
          }
        }
      }
    };

    ws.addEventListener("message", handler);

    return () => {
      ws.removeEventListener("message", handler);
    };
  }

  private parseDhoomResponse(data: string): QueryResult[] {
    // Simple DHOOM response parser
    const lines = data.split("\n");
    const results: QueryResult[] = [];

    for (const line of lines) {
      if (line.startsWith("META ")) {
        // Parse META confidence=X curvature=Y capacity=Z
        const meta = { confidence: 0, curvature: 0, capacity: 0 };
        const parts = line.substring(5).split(" ");
        for (const part of parts) {
          const [key, val] = part.split("=");
          if (key === "confidence") meta.confidence = parseFloat(val);
          if (key === "curvature") meta.curvature = parseFloat(val);
          if (key === "capacity") meta.capacity = parseFloat(val);
        }
        if (results.length > 0) {
          results[results.length - 1].meta = meta;
        }
      }
    }

    return results;
  }
}

// ── Main Client ──

export class GIGIClient {
  private baseUrl: string;
  private apiKey?: string;
  private ws?: WebSocket;

  constructor(url: string, opts?: { apiKey?: string }) {
    this.baseUrl = url.replace(/\/$/, "");
    this.apiKey = opts?.apiKey;
  }

  /** Get a bundle handle for operations. */
  bundle(name: string): BundleHandle {
    return new BundleHandle(this, name);
  }

  /** Health check. */
  async health(): Promise<Record<string, unknown>> {
    return this.request("GET", "/v1/health");
  }

  /** List all bundles. */
  async listBundles(): Promise<{ data: Array<{ name: string; records: number; fields: number }>; meta: unknown }> {
    return this.request("GET", "/v1/bundles");
  }

  /** Get the OpenAPI specification. */
  async openapi(): Promise<Record<string, unknown>> {
    return this.request("GET", "/v1/openapi.json");
  }

  /** Execute a GQL (Geometric Query Language) query. */
  async gql(query: string): Promise<Record<string, unknown>> {
    return this.request("POST", "/v1/gql", { query });
  }

  /** Sync (for GIGI Edge instances). */
  async sync(): Promise<SyncReport> {
    return this.request("POST", "/v1/sync");
  }

  /** Status (for GIGI Edge instances). */
  async status(): Promise<Record<string, unknown>> {
    return this.request("GET", "/v1/status");
  }

  /** Open a WebSocket connection for subscriptions. */
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      const wsUrl = this.baseUrl.replace(/^http/, "ws") + "/ws";
      this.ws = new WebSocket(wsUrl);
      this.ws.onopen = () => resolve();
      this.ws.onerror = (e) => reject(e);
    });
  }

  /** Close the connection. */
  async close(): Promise<void> {
    if (this.ws) {
      this.ws.close();
      this.ws = undefined;
    }
  }

  /** @internal */
  getWebSocket(): WebSocket | undefined {
    return this.ws;
  }

  /** @internal */
  async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.apiKey) {
      headers["X-API-Key"] = this.apiKey;
    }

    const opts: RequestInit = { method, headers };
    if (body && method !== "GET") {
      opts.body = JSON.stringify(body);
    }

    const response = await fetch(url, opts);
    if (!response.ok) {
      const text = await response.text();
      throw new Error(`GIGI API error ${response.status}: ${text}`);
    }
    return response.json() as Promise<T>;
  }
}

// ── GIGI Edge Client ──

export class GIGIEdge extends GIGIClient {
  constructor(opts: {
    url: string;
    apiKey?: string;
  }) {
    super(opts.url, { apiKey: opts.apiKey });
  }

  /** Trigger sync with remote GIGI Stream. */
  async sync(): Promise<SyncReport> {
    return this.request("POST", "/v1/sync");
  }
}

export default GIGIClient;
