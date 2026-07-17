# GIGI HTTP API Reference

This is the canonical reference for every `/v1/*` endpoint exposed by
`gigi_stream`, the HTTP front-end of the GIGI engine. It is generated
to mirror the in-tree OpenAPI spec at `openapi.json` (served live at
[`GET /v1/openapi.json`](#get-v1openapijson)) and supplements it with
the feature-gated endpoints that the OpenAPI document does not yet
enumerate (Brain primitives, WISH, Causal States, GQL, Lattice / Gauge
Field).

For a hands-on walkthrough see `docs/GETTING_STARTED.md`; for the
production-stable vs research-stage matrix see
`docs/STABILITY_GUARANTEES.md`.

---

## Conventions

### Base URL

- Local: `http://localhost:3142`
- Public read-only demo (no key): `https://gigi-stream.fly.dev`

The port is controlled by the `PORT` env var.

### Authentication

GIGI uses a single header for API-key auth:

```
X-API-Key: <your-key>
```

- Header name is **`X-API-Key`**, not `Authorization: Bearer …`.
- The key is set per deployment via the `GIGI_API_KEY` env var.
- When `GIGI_API_KEY` is unset, the server runs unauthenticated and
  every endpoint is reachable without a header.
- When set, every endpoint **except `/v1/health` and `/v1/openapi.json`**
  requires the header.
- Multi-tenant deployments may additionally enable JWT verification by
  setting `GIGI_JWT_SECRET`. When enabled, requests carrying a
  `Authorization: Bearer <jwt>` header are gated by the claims
  (namespace prefix enforcement on `/v1/bundles/<name>/*`).

### Error response shape

Every error returns JSON in this shape:

```json
{ "error": "human-readable error string" }
```

`409 Conflict` on the optimistic-concurrency `update` endpoint adds a
`current_version` field; transaction errors may add structured fields
documented per endpoint.

### Status code conventions

| Code | Meaning                                                    |
|------|------------------------------------------------------------|
| 200  | OK — operation succeeded                                   |
| 400  | Bad request — malformed body, bad params, validation fail  |
| 401  | Unauthorized — missing or wrong `X-API-Key`                |
| 403  | Forbidden — JWT namespace claim denies the bundle path     |
| 404  | Not found — bundle, record, or sub-resource missing        |
| 409  | Conflict — bundle already exists / version mismatch        |
| 429  | Rate limited                                               |
| 500  | Internal server error                                      |
| 503  | Not ready — WAL replay in progress                         |

### Endpoint tags

Each endpoint below carries a tag in its heading:

- `[read]` — read-only. Safe to call against the public demo with no
  side effects.
- `[write]` — mutates state. Requires `X-API-Key` on any deployment
  with `GIGI_API_KEY` set; will not work against the public demo
  unless the demo's key is provisioned.
- `[admin]` — restricted to operators (snapshot, log config).

---

## Endpoint families

| Family                  | Path prefix                              | Feature flag       |
|-------------------------|------------------------------------------|--------------------|
| Health                  | `/v1/health`, `/v1/openapi.json`         | always on          |
| Bundles                 | `/v1/bundles`, `/v1/bundles/{name}`      | always on          |
| Data                    | `/v1/bundles/{name}/{insert,update,…}`   | always on          |
| Query                   | `/v1/bundles/{name}/{query,get,range,…}` | always on          |
| Analytics               | `/v1/bundles/{name}/{stats,curvature,…}` | always on          |
| Schema                  | `/v1/bundles/{name}/{schema,add-…}`      | always on          |
| Import / Export         | `/v1/bundles/{name}/{export,import,…}`   | always on          |
| Joins                   | `/v1/bundles/{name}/join`                | always on          |
| Transactions (atomic)   | `/v1/bundles/{name}/transaction`         | always on          |
| Transactions (2-PC)     | `/v1/transactions/*`                     | `transactions`     |
| PRISM compatibility     | `/v1/bundles/{name}/points*`             | always on          |
| GQL                     | `/v1/gql`                                | always on          |
| Brain primitives        | `/v1/bundles/{name}/brain/*`             | `kahler`           |
| WISH                    | `/v1/wish`, `/v1/bundles/{name}/wish`    | `wish`             |
| Patterns                | `/v1/patterns`, `…/hunt`                 | `patterns`         |
| Causal States           | `/v1/causal_states/commutator`           | `causal_states`    |
| Lattice / Gauge field   | `/v1/lattice`, `/v1/gauge_field/*`       | `lattice`, `gauge` |
| Sharded analytics       | `/v1/bundles/{name}/sharded/*`           | `sharded`          |
| Quantum cohomology      | `/v1/quantum_cohomology/*`               | `kahler`           |
| WebSocket               | `/ws`, `/v1/ws/dashboard`                | always on          |
| Admin                   | `/v1/admin/*`                            | always on (gated)  |
| Observability           | `/v1/metrics`                            | always on          |

---

# Health

## `GET /v1/health` `[read]`

**What it does:** liveness check. Returns engine identity, version, and
basic counts. The only endpoint that is always reachable without an
API key.

**Auth:** not required.

**Response (200):**

```json
{
  "status": "ok",
  "engine": "GIGI",
  "version": "0.4.0",
  "bundles": 17,
  "total_records": 4_213_550
}
```

**Curl:**

```bash
curl https://gigi-stream.fly.dev/v1/health
```

## `GET /v1/openapi.json` `[read]`

**What it does:** serves the OpenAPI 3.0.3 spec for the public surface.

**Auth:** not required.

**Curl:**

```bash
curl https://gigi-stream.fly.dev/v1/openapi.json | jq .info
```

---

# Bundles

## `GET /v1/bundles` `[read]`

**What it does:** lists every bundle on the engine with record counts
and field counts.

**Auth:** required if `GIGI_API_KEY` is set.

**Response (200):**

```json
{
  "data": [
    { "name": "iris", "records": 150, "fields": 5 },
    { "name": "halcyon_thermal", "records": 200, "fields": 4 }
  ],
  "meta": { "count": 2 }
}
```

**Curl:**

```bash
curl https://gigi-stream.fly.dev/v1/bundles
```

## `POST /v1/bundles` `[write]`

**What it does:** creates a new bundle with the given schema.

**Auth:** required.

**Request body:**

```json
{
  "name": "demo",
  "schema": {
    "fields": {
      "id":     "integer",
      "name":   "text",
      "score":  "float",
      "active": "boolean",
      "seen_at":"timestamp"
    },
    "keys":    ["id"],
    "indexed": ["name"],
    "defaults": { "active": true }
  }
}
```

Field types: `integer | float | text | boolean | timestamp`.

**Response (200):**

```json
{ "status": "created", "bundle": "demo" }
```

**Errors:**
- `409` — bundle already exists.

**Curl:**

```bash
curl -X POST http://localhost:3142/v1/bundles \
  -H 'Content-Type: application/json' \
  -H 'X-API-Key: $GIGI_API_KEY' \
  -d '{"name":"demo","schema":{"fields":{"id":"integer","name":"text"},"keys":["id"]}}'
```

## `DELETE /v1/bundles/{name}` `[write]`

**What it does:** drops the bundle (records + schema + indexes).

**Auth:** required.

**Response (200):**

```json
{ "status": "dropped", "bundle": "demo" }
```

**Errors:** `404` if the bundle does not exist.

**Curl:**

```bash
curl -X DELETE http://localhost:3142/v1/bundles/demo \
  -H 'X-API-Key: $GIGI_API_KEY'
```

## `GET /v1/bundles/{name}/schema` `[read]`

**What it does:** returns the declared base fields and fiber fields.

**Response (200):**

```json
{
  "name": "demo",
  "base_fields":  [{ "name": "id",   "type": "integer", "indexed": true }],
  "fiber_fields": [{ "name": "name", "type": "text" }, { "name": "score", "type": "float" }]
}
```

---

# Data operations

## `POST /v1/bundles/{name}/insert` `[write]`

**What it does:** inserts one or more records. Returns the post-insert
curvature and confidence so callers can react to manifold drift in the
same response.

**Request body:**

```json
{
  "records": [
    { "id": 1, "name": "alice", "score": 0.9 },
    { "id": 2, "name": "bob",   "score": 0.7 }
  ]
}
```

**Response (200):**

```json
{
  "status": "inserted",
  "inserted": 2,
  "total": 152,
  "curvature": 0.0413,
  "confidence": 0.94
}
```

**Errors:** `404` bundle not found.

**Curl:**

```bash
curl -X POST http://localhost:3142/v1/bundles/demo/insert \
  -H 'Content-Type: application/json' \
  -H 'X-API-Key: $GIGI_API_KEY' \
  -d '{"records":[{"id":1,"name":"alice","score":0.9}]}'
```

## `POST /v1/bundles/{name}/update` `[write]`

**What it does:** updates a single record by key. Supports optimistic
concurrency via `expected_version` and round-trip echo via `returning`.

**Request body:**

```json
{
  "key":    { "id": 1 },
  "fields": { "score": 0.95 },
  "returning": true,
  "expected_version": 3
}
```

**Response (200):**

```json
{
  "status": "updated",
  "total": 152,
  "curvature": 0.0411,
  "confidence": 0.94,
  "version": 4,
  "data":  { "id": 1, "name": "alice", "score": 0.95 }
}
```

**Errors:**
- `404` — record or bundle not found.
- `409` — version conflict; body includes `current_version`.

## `POST /v1/bundles/{name}/delete` `[write]`

**What it does:** deletes one record by key. With `returning: true` the
deleted record is echoed back.

**Request body:**

```json
{ "key": { "id": 1 }, "returning": false }
```

**Response (200):**

```json
{ "status": "deleted", "total": 151, "curvature": 0.0412, "confidence": 0.94 }
```

**Errors:** `404` record or bundle not found.

## `POST /v1/bundles/{name}/upsert` `[write]`

**What it does:** inserts or updates a record (decided by key match).

**Request body:**

```json
{ "record": { "id": 1, "name": "alice", "score": 0.92 } }
```

**Response (200):**

```json
{ "status": "inserted", "total": 152, "curvature": 0.0412 }
```

`status` is `"inserted"` or `"updated"`.

## `POST /v1/bundles/{name}/increment` `[write]`

**What it does:** atomically adds `amount` (default `1`) to a numeric
field of a single record.

**Request body:**

```json
{ "key": { "id": 1 }, "field": "score", "amount": 0.05 }
```

**Response (200):**

```json
{ "status": "incremented", "new_value": 0.97, "total": 152, "curvature": 0.0411 }
```

## `POST /v1/bundles/{name}/bulk-delete` `[write]`

**What it does:** deletes every record matching `conditions`.

**Request body:**

```json
{ "conditions": [ { "field": "active", "op": "eq", "value": false } ] }
```

**Response (200):**

```json
{ "status": "ok", "deleted": 42, "total": 110, "curvature": 0.0420 }
```

## `POST /v1/bundles/{name}/truncate` `[write]`

**What it does:** drops every record but keeps the schema.

**Response (200):**

```json
{ "status": "ok", "removed": 152 }
```

## `POST /v1/bundles/{name}/stream` `[write]`

**What it does:** batch ingest via `text/plain` newline-delimited JSON.
One record per line.

**Request body** (`Content-Type: text/plain`):

```
{"id":1,"name":"alice"}
{"id":2,"name":"bob"}
```

**Curl:**

```bash
curl -X POST http://localhost:3142/v1/bundles/demo/stream \
  -H 'X-API-Key: $GIGI_API_KEY' \
  --data-binary $'{"id":1,"name":"alice"}\n{"id":2,"name":"bob"}\n'
```

---

# Queries

All filtered queries share the same `ConditionSpec`:

```json
{ "field": "score", "op": "gte", "value": 0.5 }
```

Operators: `eq | neq | gt | gte | lt | lte | contains | starts_with |
ends_with | regex | in | not_in | is_null | is_not_null`.

`SortSpec`:

```json
{ "field": "score", "desc": true }
```

## `POST /v1/bundles/{name}/query` `[read]`

**What it does:** filtered query with conjunctive `conditions`,
disjunctive `or_conditions` (array of arrays — outer OR, inner AND),
sort, limit, offset, projection, and free-text `search`.

**Request body:**

```json
{
  "conditions":    [ { "field": "active", "op": "eq", "value": true } ],
  "or_conditions": [ [ { "field": "score", "op": "gte", "value": 0.9 } ] ],
  "sort":          [ { "field": "score", "desc": true } ],
  "limit":  20,
  "offset": 0,
  "fields": ["id", "name", "score"],
  "search": "alice",
  "search_fields": ["name"]
}
```

**Response (200):**

```json
{
  "data": [{ "id": 1, "name": "alice", "score": 0.95 }],
  "meta": { "count": 1, "curvature": 0.041, "confidence": 0.94 }
}
```

`GIGI_QUERY_MAX_ROWS` caps the response.

## `GET /v1/bundles/{name}/get` `[read]`

**What it does:** O(1) point query. Pass every key field as a query
parameter.

**Query params:** one `<key_field>=<value>` pair per key field.

**Response (200):**

```json
{
  "data": { "id": 1, "name": "alice", "score": 0.95 },
  "meta": { "curvature": 0.041, "confidence": 0.94 }
}
```

**Curl:**

```bash
curl 'http://localhost:3142/v1/bundles/demo/get?id=1'
```

## `GET /v1/bundles/{name}/range` `[read]`

**What it does:** records where `field` is in `[min, max]`. Uses an
index when `field` is indexed.

**Query params:** `field=<name>&min=<num>&max=<num>`.

**Curl:**

```bash
curl 'http://localhost:3142/v1/bundles/demo/range?field=score&min=0.5&max=1.0'
```

## `POST /v1/bundles/{name}/count` `[read]`

**What it does:** returns the count of records matching `conditions`
(and optional `or_conditions`). No row materialization.

**Response (200):**

```json
{ "count": 42 }
```

## `POST /v1/bundles/{name}/exists` `[read]`

**What it does:** boolean: does at least one record match?

**Response (200):**

```json
{ "exists": true }
```

## `GET /v1/bundles/{name}/distinct/{field}` `[read]`

**What it does:** returns the set of distinct values for `field`.

**Response (200):**

```json
{ "field": "name", "values": ["alice", "bob"], "count": 2 }
```

## `POST /v1/bundles/{name}/query-stream` `[read]`

**What it does:** like `/query` but streams results as newline-delimited
JSON. One record per line. Use for large result sets that would
otherwise hit `GIGI_QUERY_MAX_ROWS`.

**Response:** `application/x-ndjson`.

## `GET /v1/bundles/{name}/record/{id}/vector` `[read]`

**What it does:** returns the numeric fiber-field vector representation
of a single record, suitable for vector search input.

## `POST /v1/bundles/{name}/vector-search` `[read]`

**What it does:** k-nearest-neighbor search on numeric fiber-field
embeddings.

**Request body:**

```json
{ "query": [0.1, 0.2, 0.3], "fields": ["x","y","z"], "k": 10 }
```

---

# Analytics

## `GET /v1/bundles/{name}/stats` `[read]`

**What it does:** complete bundle statistics — record count, schema
counts, index sizes, per-field cardinalities, Welford field stats, and
the bundle's current curvature scalar.

**Response (200):**

```json
{
  "name": "demo",
  "record_count": 152,
  "base_fields":  1,
  "fiber_fields": 4,
  "indexed_fields": ["name"],
  "storage_mode": "heap",
  "index_sizes":      { "name": 152 },
  "field_cardinalities": { "name": 78 },
  "field_stats": { "score": { "mean": 0.61, "variance": 0.082 } },
  "curvature": 0.0412
}
```

## `GET /v1/bundles/{name}/curvature` `[read]`

**What it does:** curvature analysis report — total `K`, `confidence`,
`capacity`, and a per-field breakdown of variance, range, and curvature
contribution.

**Response (200):**

```json
{
  "K": 0.0412,
  "confidence": 0.94,
  "capacity": 0.083,
  "per_field": [
    { "field": "score", "variance": 0.082, "range": 1.0, "k": 0.041 }
  ]
}
```

## `GET /v1/bundles/{name}/spectral` `[read]`

**What it does:** spectral report — Fiedler value (`lambda1`), graph
diameter, and spectral capacity.

**Response (200):**

```json
{ "lambda1": 0.183, "diameter": 7, "spectral_capacity": 0.146 }
```

## `GET /v1/bundles/{name}/consistency` `[read]`

**What it does:** consistency check — verifies that bundle sections
glue without violation.

**Response (200):**

```json
{ "consistent": true, "total_sections": 8, "violations": 0 }
```

## `GET /v1/bundles/{name}/betti` `[read]`

**What it does:** Betti number report on the bundle's adjacency complex.

## `GET /v1/bundles/{name}/entropy` `[read]`

**What it does:** Shannon entropy report across categorical fields.

## `GET /v1/bundles/{name}/free-energy` `[read]`

**What it does:** Friston-style variational free-energy report. The
substrate primitive that the brain primitives consume.

## `POST /v1/bundles/{name}/geodesic` `[read]`

**What it does:** computes a geodesic between two records or two
points in field space. POST because the path may be long.

## `GET /v1/bundles/{name}/metric` `[read]`

**What it does:** metric tensor sample report at the field-mean.

## `GET /v1/bundles/{name}/capacity` `[read]`

**What it does:** Branch VII cognitive-geometry capacity `C = τ / K`.

**Optional scoping query params** (same on `/horizon`): compute the
statistics over a named vector family and/or a k-NN neighborhood instead
of the whole bundle, through the **same formulas** — this fixes
whole-bundle-Welford pollution (e.g. `l_c` `4.7e6` → `0.03` when scoped).

| Param    | Form                | Effect                                        |
|----------|---------------------|-----------------------------------------------|
| `fields` | `fields=<spec>`     | scope stats to a named vector family (range sugar `v0..v383` or explicit list) |
| `locus`  | `locus=<field>=<v>` | scope to a k-NN neighborhood anchored at the record where `<field>=<v>` |
| `k`      | `k=<n>`             | neighborhood size for `locus`                 |

With no params the response is **byte-identical** to before.
Precedence: `estimator=fixed` > `fields`/`locus` > default.

## `GET /v1/bundles/{name}/horizon` `[read]`

**What it does:** Branch VII horizon report. Accepts the same optional
`fields=` / `locus=` / `k=` scoping params as `/capacity` above, through
the same formulas; the default (no-param) response is byte-identical.

## `GET /v1/bundles/{name}/depth` `[read]`

**What it does:** Branch VII encoding-depth report.

## `POST /v1/bundles/{name}/perceive` `[read]`

**What it does:** Branch VII perception primitive.

## `POST /v1/bundles/{name}/local_holonomy` `[read]`

**What it does:** local holonomy at a base point.

## `POST /v1/bundles/{name}/windowed_coherence` `[read]`

**What it does:** slides a fixed window along an ordered path and emits a
per-window laminar verdict, computed server-side (composes
transport ∘ holonomy). Replaces the client-side transport/holonomy round
trips.

**Request body:**

```json
{
  "path":      ["k0", "k1", "k2", "…"],
  "key_field": "id",
  "window":    8,
  "fiber":     ["v0", "v1", "…"],
  "threshold": 0.91
}
```

`threshold` is optional; it defaults to `0.91`.

**Response (200):**

```json
{
  "windows": [
    {
      "start_index":     0,
      "keys":            ["k0", "k1", "…"],
      "holonomy_defect": 0.03,
      "coherence":       0.97,
      "laminar":         true
    }
  ],
  "n_windows":     41,
  "laminar_all":   true,
  "threshold_used": 0.91,
  "dim":           384,
  "window":        8,
  "bundle":        "marcella_source",
  "lambda_budget": 1.0
}
```

A window is `laminar` when its `coherence` is at or above
`threshold_used`; `laminar_all` is the AND over every window.

## `POST /v1/bundles/{name}/explain` `[read]`

**What it does:** returns the query execution plan **without running
the query**. Same request shape as `/query`.

**Response (200):**

```json
{
  "scan_type": "index",
  "total_records": 152,
  "index_scans": ["name"],
  "full_scan_conditions": [],
  "or_group_count": 0,
  "has_sort":   true,
  "has_limit":  true,
  "has_offset": false,
  "storage_mode": "heap"
}
```

## `POST /v1/bundles/{name}/aggregate` `[read]`

**What it does:** `GROUP BY` over `group_by`, aggregating `field`.
Optional `conditions` pre-filter.

**Request body:**

```json
{
  "group_by":   "name",
  "field":      "score",
  "conditions": [ { "field": "active", "op": "eq", "value": true } ]
}
```

**Response (200):**

```json
{
  "data": {
    "groups": {
      "alice": { "count": 2, "sum": 1.9, "avg": 0.95, "min": 0.9, "max": 1.0 }
    }
  },
  "meta": { "count": 1 }
}
```

## `POST /v1/bundles/{name}/anomalies` `[read]`

**What it does:** anomaly detection using the bundle's curvature field
+ Welford stats.

## `POST /v1/bundles/{name}/anomalies/field` `[read]`

**What it does:** anomaly detection scoped to a single field.

## `GET /v1/bundles/{name}/health` `[read]`

**What it does:** per-bundle health summary (record count, curvature,
recent error rate).

## `POST /v1/bundles/{name}/predict` `[read]`

**What it does:** volatility prediction on a numeric field.

## `POST /v1/divergence` `[read]`

**What it does:** KL divergence between two bundles (cross-bundle
information geometry).

**Request body:**

```json
{ "bundle_a": "demo", "bundle_b": "demo_prev", "field": "score" }
```

## `POST /v1/bundles/{name}/verify_invariant` `[read]`

**What it does:** Sprint N invariant-consistency verification. Auditor
surface; enforces bundle_id binding.

---

# Joins

## `POST /v1/bundles/{name}/join` `[read]`

**What it does:** pullback join between `name` (left) and
`right_bundle`. Pairs records where `left_field` equals `right_field`.

**Request body:**

```json
{
  "right_bundle": "users",
  "left_field":   "user_id",
  "right_field":  "id"
}
```

**Response (200):**

```json
{
  "data": [{ "user_id": 1, "name": "alice", "score": 0.95 }],
  "meta": { "count": 1 }
}
```

---

# Schema evolution

## `POST /v1/bundles/{name}/add-field` `[write]`

**What it does:** adds a new field to the schema. Existing records pick
up the field default (or null) lazily.

**Request body:**

```json
{ "name": "department", "type": "text", "default": "general" }
```

## `POST /v1/bundles/{name}/drop-field` `[write]`

**What it does:** removes a field from the schema (and from all
records).

**Request body:**

```json
{ "name": "department" }
```

## `POST /v1/bundles/{name}/add-index` `[write]`

**What it does:** adds an index on an existing field.

**Request body:**

```json
{ "field": "department" }
```

---

# Import / Export

## `GET /v1/bundles/{name}/export` `[read]`

**What it does:** exports every record as a single JSON array.

**Response (200):**

```json
{ "data": [ { "id": 1, "name": "alice" } ], "meta": { "count": 1 } }
```

## `POST /v1/bundles/{name}/import` `[write]`

**What it does:** bulk import — same wire shape as `/insert`.

## `GET /v1/bundles/{name}/dhoom` `[read]`

**What it does:** DHOOM-format snapshot export (binary, mmap-friendly).
DHOOM is GIGI's native on-disk format.

## `POST /v1/bundles/{name}/ingest` `[write]`

**What it does:** DHOOM-format ingest counterpart of `/dhoom`.

---

# PRISM-compatible REST

A thin layer that maps PRISM (Davis Geometric's reconciliation
platform) URL conventions onto GIGI bundles. Use these when integrating
with PRISM tooling; otherwise prefer the canonical endpoints above.

## `GET /v1/bundles/{name}/points` `[read]`

**What it does:** lists all records, paginated.

**Query params:** `limit`, `offset`, `order_by`, `order=asc|desc`.

## `POST /v1/bundles/{name}/points` `[write]`

**What it does:** insert (alias of `/insert`).

## `PATCH /v1/bundles/{name}/points` `[write]`

**What it does:** bulk update by filter.

**Request body:**

```json
{
  "filter": [ { "field": "active", "op": "eq", "value": true } ],
  "fields": { "score": 0.5 }
}
```

## `GET /v1/bundles/{name}/points/{field}/{value}` `[read]`

**What it does:** fetches a record by `field=value` (single match).

## `PATCH /v1/bundles/{name}/points/{field}/{value}` `[write]`

**What it does:** patches the record matched by `field=value`.

## `DELETE /v1/bundles/{name}/points/{field}/{value}` `[write]`

**What it does:** deletes the record matched by `field=value`.

---

# Atomic transactions (always-on)

## `POST /v1/bundles/{name}/transaction` `[write]`

**What it does:** runs an array of operations on a single bundle as
all-or-nothing. Order is preserved; failures roll back the batch.

**Request body:**

```json
{
  "ops": [
    { "op": "insert",    "record": { "id": 10, "name": "carol", "score": 0.5 } },
    { "op": "update",    "key": { "id": 1 }, "fields": { "score": 0.9 } },
    { "op": "increment", "key": { "id": 1 }, "field":  "score", "amount": 0.05 },
    { "op": "delete",    "key": { "id": 2 } }
  ]
}
```

`op` values: `insert | update | delete | increment`.

**Response (200):**

```json
{
  "status": "ok",
  "results": ["inserted","updated","incremented","deleted"],
  "total": 152,
  "curvature": 0.0413
}
```

**Errors:** `400` rollback (body has `error`), `404` bundle not found.

---

# Two-phase transactions (`transactions` feature)

Atomic Sheaf Commits Phase-A surface. Cross-bundle ACID with the three
GIGI invariants (cocycle bound, K-monotone, connection-coherent).

## `POST /v1/transactions/begin` `[write]`

**What it does:** opens a new transaction. Returns a transaction id.

**Response (200):**

```json
{ "tx_id": "tx_8c14…", "status": "open" }
```

## `GET /v1/transactions/{tx_id}` `[read]`

**What it does:** returns the transaction's current state.

## `POST /v1/transactions/{tx_id}/write` `[write]`

**What it does:** stages a write on the transaction.

**Request body:**

```json
{
  "bundle": "demo",
  "op":     "insert",
  "record": { "id": 11, "name": "dave" }
}
```

## `POST /v1/transactions/{tx_id}/commit` `[write]`

**What it does:** prepares + commits. All staged writes apply atomically
or none do.

## `POST /v1/transactions/{tx_id}/rollback` `[write]`

**What it does:** discards all staged writes.

---

# GQL

## `POST /v1/gql` `[write]`

**What it does:** evaluates a GIGI Query Language statement. GQL is the
high-level surface for CREATE BUNDLE, INSERT, SELECT, LATTICE,
GAUGE_FIELD, GIBBS_SAMPLE, SNAPSHOT, INGEST, EMIT, the topology verbs
(CHERN_CLASS, SPECTRAL_GAUGE, PI_1, OBSTRUCTION), and every other
declarative verb.

**Request body:**

```json
{ "query": "CREATE BUNDLE iris (id INT KEY, species TEXT, sepal_length FLOAT)" }
```

**Response (200):** shape depends on the statement.

```json
{ "status": "ok" }
```

- `{"status": "ok"}` — statement executed, nothing to return.
- `{"status": "ok", "notice": "<message>"}` — the statement parsed and
  validated but either intentionally did nothing or has a receipt to
  report; the notice says which. A bare `"ok"` never implies work
  happened when the engine knows it didn't.
- `{"rows": [...], "count": N}` — rows-producing statements (COVER,
  INTEGRATE, SHOW FIELDS, EXPLAIN SECTION, …).
- `{"value": ...}` — scalar results; `{"affected": N}` — counts.

**Errors:**
- `400` — parse error (body includes parser message).
- `404` — referenced bundle missing.

Parse errors are human-readable: they name the offending token, show
near-context, and suggest the closest verb for a misspelled first word
(`Unknown statement: 'COVERR' — did you mean 'COVER'?`; with no close
match the error points at `GQL_REFERENCE.md`). Trailing input after a
complete statement is refused rather than silently discarded:

```
Statement parsed, but this trailing input is not a supported clause
and was NOT executed: '<tokens>'. Remove it, or check the GQL
reference for the supported form of this statement.
```

**Halcyon canonical chain — Yang-Mills thermalization:**

```bash
curl -X POST http://localhost:3142/v1/gql \
  -H 'Content-Type: application/json' \
  -H 'X-API-Key: $GIGI_API_KEY' \
  -d '{"query":"LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2'"}'

curl -X POST http://localhost:3142/v1/gql \
  -H 'Content-Type: application/json' \
  -H 'X-API-Key: $GIGI_API_KEY' \
  -d '{"query":"GAUGE_FIELD U ON LATTICE bb GROUP SU(2) INIT IDENTITY"}'

curl -X POST http://localhost:3142/v1/gql \
  -H 'Content-Type: application/json' \
  -H 'X-API-Key: $GIGI_API_KEY' \
  -d '{"query":"GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616"}'
```

GQL is the only path that reaches every embedded-only verb (E_FIELD,
SYMPLECTIC_FLOW); the REST routes for the same primitives are
read-only.

## GQL: server-side file ingest — INGEST

```
INGEST <bundle> FROM '<path>' FORMAT CSV|NPZ|JSONL [KEY <name>]
  [AS GAUGE_FIELD GROUP <group> ON LATTICE <lattice>];
```

**What it does:** reads a file on the server's filesystem into a
bundle, inferring or extending the schema from the source.

- `FORMAT NPZ` — NumPy archive. `KEY <name>` selects one array from a
  multi-array archive; omitting KEY on a multi-array archive is an
  error that lists the member names. `f32` data upconverts to `f64`;
  unsupported dtypes error by name.
- `FORMAT CSV` — header row required; column types are inferred.
  Optional `KEY <col>` names the base-key column (default: the first
  column).
- `FORMAT JSONL` — one JSON object per line; array values land as
  first-class vector fibers. `KEY <col>` is **required** — it names
  the base-key column, since JSON objects carry no reliable column
  order.
- `AS GAUGE_FIELD GROUP <g> ON LATTICE <l>` interprets an NPZ array as
  a lattice gauge field: canonical fiber names per group (`q0..q3` for
  SU(2), `re_00..im_22` for SU(3), `theta` for U(1)), plus `vertex_a` /
  `vertex_b` INT base fields derived from the lattice's column-major
  `site_of`. On an OBC lattice, wrap-edge records are omitted — the
  record set equals the lattice edge set.

**Path containment (fail-closed):** every file-reading INGEST format is
gated on the `GIGI_INGEST_DIR` env var. When it is unset, INGEST
refuses:

```
INGEST from a server-side file requires GIGI_INGEST_DIR to be set;
set it to the directory that ingest sources live under
```

When set, the source path must resolve inside that directory
(canonical-to-canonical compare; drive/UNC prefixes, absolute paths,
`..` components, and symlink/junction tunnels out of the root are all
refused). Lexical escape errors carry the shape (prefixed `INGEST: `
on this verb):

```
INGEST: path '<path>' escapes containment root '<root>': <reason>
```

A symlink or junction inside the root that resolves outside it is
refused as `resolved path '<path>' is not under containment root
'<root>'`.

The production deployment (`fly.toml`) sets
`GIGI_INGEST_DIR=/data/ingest`.

## GQL: structured export — EMIT

```
<rows-statement> EMIT CSV TO '<relative-path>';
```

**What it does:** suffix on any rows-producing statement (COVER,
INTEGRATE, SHOW FIELDS, …). Serializes the rows as CSV (columns are
the sorted union of row keys) and writes the file inside
`GIGI_EMIT_DIR`. Success returns a notice receipt:

```json
{ "status": "ok", "notice": "EMIT CSV: wrote 42 rows to /data/emit/out.csv" }
```

**Gate (fail-closed):** `GIGI_EMIT_DIR` unset refuses with:

```
EMIT is disabled on this engine: set GIGI_EMIT_DIR=<directory> to
enable it — exported files are written inside that directory only.
(Over HTTP, prefer requesting the rows and saving client-side.)
```

The path must be relative and contained; escapes refuse with:

```
EMIT path '<path>' must be relative, without '..' — files land
inside GIGI_EMIT_DIR
```

EMIT wrapping a bundle-less statement (e.g.
`SHOW BUNDLES … EMIT CSV TO 'x.csv'`) now dispatches through the parser
executor instead of silently returning ok — with `GIGI_EMIT_DIR` unset
it returns the gate error above rather than a bare `{"status":"ok"}`.

## GQL: cubic / OBC lattices and topology verbs

```
LATTICE l24 FROM CUBIC L=24 DIM=3 OBC AXIS 2;
CHERN_CLASS U ORDER 2 ON LATTICE l24 GROUP SU(3);
SPECTRAL_GAUGE U WHERE sector = 1 ON FIBER (q0, q1, q2, q3) GROUP SU(2);
SPECTRAL_GAUGE b ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL LIMIT 64;
SPECTRAL_GAUGE b ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 50 AROUND 0.0;
ALTER BUNDLE U ADD BASE q_rounded INT;
```

- `LATTICE <name> FROM CUBIC L=<int> DIM=<int> [PERIODIC]
  [OBC AXIS <k>] [TOPOLOGY '<string>'];` — `OBC AXIS <k>` opens the
  boundary along axis `k` (edge/plaquette omission at the wall).
  `PERIODIC` is the default. Fully-open boundaries (`OPEN`) are
  deferred to Phase 2 — the flag parses but fails at execution, and
  cannot be combined with `OBC AXIS`.
- `CHERN_CLASS <bundle> ORDER <n>` takes its clauses in any order
  after ORDER: `ON FIBER (<fields>)` or `ON LATTICE <l>`,
  `GROUP <g>`, `PER <field>`, `INTO_COLUMN <col>`. `INTO_COLUMN`
  writes the per-record result back into a base column (declare it
  first with ALTER BUNDLE).
- `SPECTRAL_GAUGE <bundle> [WHERE <conditions>] ON FIBER (<fields>)
  [GROUP <g>] [MODE MAGNETIC] [ FULL [LIMIT k] | BULK k [AROUND σ |
  IN [a,b]] ];` — the optional WHERE clause stratifies the spectrum to
  a sector; conditions use the same grammar as COVER WHERE. The trailing
  `FULL` / `BULK` selectors are **mutually exclusive** (a second one
  errors). Clause order is strict: bundle → WHERE → `ON FIBER (…)` →
  GROUP → MODE → the FULL/BULK selector.
  - Default (no `MODE`): the real cos-weight Laplacian (SU(2) path,
    unchanged). Its nearest-neighbour spacing-ratio sits in the GOE
    symmetry class (measured `r̃ ≈ 0.527`, GOE anchor 0.5307, Atas
    et al.).
  - `MODE MAGNETIC` (requires `GROUP U(1)`): the complex Hermitian
    magnetic Laplacian — off-diagonal `−e^{iθ}` with conjugate pairs.
    Its spectrum sits in the GUE class (measured `r̃ ≈ 0.605`, GUE
    anchor 0.5996). This is the
    **RIEMANN line**: a gauge-native observable that reads/restages
    Bee's documented spectral signature — it is **evidence in the Davis
    framework, not a proof of the Riemann Hypothesis**.
  - `FULL` populates the full ascending real spectrum (dense solver,
    `V ≤` the dense ceiling); `FULL LIMIT k` keeps the `k` smallest.
  - `BULK k` returns the `k` eigenvalues nearest the spectral **center**
    (interior window, contiguous, ascending) — a re-centering slice on
    the sorted dense spectrum. Plain `BULK` auto-centers on the
    positional median; `AROUND σ` recenters on scalar `σ`; `IN [a,b]`
    centers on the midpoint of the bracketed real interval. `BULK`
    requires `MODE MAGNETIC` this phase.
  - **Dense-solver ceiling.** `FULL`/`BULK` run a dense eigensolver
    capped at `V = 4096` by default. `GIGI_DENSE_CEIL` opts a higher
    ceiling in, clamped to the safe band `[4096, 8192]` (raise-only;
    values below 4096 or above 8192 clamp into the band; missing or
    unparseable → 4096). A `FULL`/`BULK` request on `V` above the
    in-force ceiling returns a typed `SparseUnavailable`:

    ```
    SPECTRAL_GAUGE: FULL/BULK on V = <n> vertices exceeds the dense
    eigensolver ceiling (in force: V = <threshold>, spec §6 boundary
    4096). Opt in to a higher dense ceiling up to 8192 by setting
    GIGI_DENSE_CEIL — but note the memory cost: a V ≈ 8000
    complex-Hermitian Laplacian is ~1 GB for the matrix plus ~1 GB for
    eigenvectors (~2–3 GB peak RSS, O(V³) work) and can OOM a laptop,
    which is why the default stays 4096. For V beyond 8192 the sparse
    interior Lanczos arm ships in Phase 2.1; until then run FULL/BULK on
    a smaller (sectoral / downsampled) subgraph, or drop FULL/BULK for
    the λ₁-only gap
    ```

    The sparse interior arm (Phase 2.1) is **not shipped** — deferred.
  - Mode-guard errors (verbatim):

    ```
    SPECTRAL_GAUGE MODE: only MAGNETIC is a user-facing mode this phase
    (got '<word>') — the dense/sparse solver choice is internal per the
    Phase-2 reconciliation
    ```

    ```
    SPECTRAL_GAUGE: MODE MAGNETIC requires GROUP U(1) in this phase
    (matrix-valued magnetic Laplacians are a later phase); got GROUP <g>
    ```

    ```
    SPECTRAL_GAUGE: BULK requires MODE MAGNETIC in this phase — the
    interior center-window (RH / number-variance) statistics live in the
    magnetic complex-Hermitian spectrum; add MODE MAGNETIC before BULK,
    or use FULL for the cos-weight spectrum
    ```

    ```
    SPECTRAL_GAUGE: FULL and BULK are mutually exclusive — FULL returns
    the k smallest eigenvalues ascending, BULK returns the k centermost
    window; pick one
    ```
  - A `BULK` response adds `bulk: true`, `bulk_center`,
    `bulk_center_index`, `bulk_lo`, `bulk_hi` to the spectrum row.
- `ALTER BUNDLE <bundle> ADD BASE <field> <type>;` — append-only
  schema evolution. Phase 1 is ADD BASE only (no DROP, no ALTER
  FIBER, no rename); heap bundles only. Type names map exactly as in
  CREATE BUNDLE: `INT`/`INTEGER`/`NUMERIC`/`FLOAT`/`REAL`/`DOUBLE` →
  numeric, `TEXT`/`VARCHAR`/`STRING`/`CATEGORICAL`/`BOOL`/`BOOLEAN` →
  categorical, `TIMESTAMP`/`DATETIME`/`DATE` → timestamp; unrecognized
  names fall back to categorical. Existing records carry the new
  field as null.

## GQL: gauge fields — GAUGE_FIELD INIT FLUX

```
GAUGE_FIELD phi GROUP U(1) ON LATTICE l24 INIT FLUX RANDOM SEED 42;
GAUGE_FIELD phi GROUP U(1) ON LATTICE l24 INIT FLUX UNIFORM 0.1;
```

- `GAUGE_FIELD <name> <clause>*;` — the `ON LATTICE <l>`, `GROUP <g>`,
  and `INIT <init>` clauses are each required exactly once and may
  appear in any order (order relaxed 2026-07-16).
- `INIT` is one of `IDENTITY` | `HAAR_RANDOM [SEED <n>]` |
  `FROM <src>` | `FLUX (RANDOM SEED <n> | UNIFORM <φ>)`. `INIT FLUX`
  materializes a `theta` bundle of seeded deterministic per-edge U(1)
  phases (U(1)-only at the executor).
- `FLUX RANDOM` **requires** `SEED` — flux reproducibility is
  contractual. Omitting it errors:

  ```
  GAUGE_FIELD INIT FLUX RANDOM requires SEED <n> — flux
  reproducibility is contractual (declare INIT FLUX RANDOM SEED <u64>)
  ```

  `FLUX UNIFORM <φ>` takes one scalar phase applied to every edge.

## GQL: raw-symmetric spectrum — SPECTRAL MODE MATRIX

```
SPECTRAL b ON FIBER (h) MODE MATRIX;
SPECTRAL b ON FIBER (h) MODE MATRIX DIAGONAL d FULL LIMIT 32;
```

**What it does:** builds the raw signed symmetric matrix
`M[a][b] = M[b][a] = h` directly from the fiber weight (**not** the
Laplacian — negative weights are preserved) and returns its
eigenvalues. This is the **P-vs-NP line**: a gauge-native observable
that reads/restages Bee's documented instability signature — it is
**evidence in the Davis framework, not a proof that P ≠ NP**.

- `SPECTRAL <bundle> ON FIBER (<h>) [GROUP g] MODE MATRIX [GROUP g]
  [DIAGONAL <field>] [FULL [LIMIT k]];` — exactly one fiber field is
  required (the signed weight `h`). Any `GROUP` clause is consumed and
  ignored (raw signed real weights need no group). The diagonal comes
  from self-loop records (`vertex_a == vertex_b`), or from the
  `DIAGONAL <field>` override. `MODE` (singular) `= MATRIX`; note
  `MODES` (plural) `k` is the separate PCA `SpectralFiber` statement.
- Requiring more or fewer than one fiber errors:

  ```
  SPECTRAL ... MODE MATRIX requires exactly one fiber field (the
  signed Hessian weight h); got <n>
  ```

  A non-MATRIX mode word on the plain `SPECTRAL` verb errors:

  ```
  SPECTRAL MODE: only MATRIX is a mode on the plain SPECTRAL verb (got
  '<word>') — MODE MAGNETIC lives on SPECTRAL_GAUGE
  ```
- **Response (200):**

  ```json
  {
    "eigenvalues":          [-2.13, -0.44, 0.91, "…"],
    "n_records_used":       512,
    "mode_used":            "matrix",
    "n_negative":           37,
    "instability_fraction": 0.072
  }
  ```

  `instability_fraction = n_negative / V`.

## GQL: gauge holonomy — HOLONOMY AROUND CYCLE

```
HOLONOMY phi AROUND CYCLE AXIS z AT (0, 12);
HOLONOMY phi AROUND CYCLE EDGES (3, 7, 11, 4);
```

**What it does:** walks a closed loop through a `GAUGE_FIELD` and
returns the ordered product of edge transports. This is the
**Poincaré line**: `order_estimate` reads the lens-space
`π₁ = ℤ/p` class — a gauge-native observable that reads/restages Bee's
documented holonomy signature, **evidence in the Davis framework, not a
proof of the Poincaré conjecture**.

- `HOLONOMY <gauge_field> AROUND CYCLE ( AXIS <ax> AT (<c0>,<c1>) |
  EDGES (<e0>,<e1>,…) );` — the operand is a `GAUGE_FIELD` name (from
  the gauge registry), not a bundle. The `AROUND` keyword disambiguates
  this from `HOLONOMY ON FIBER` / `HOLONOMY NEAR`.
  - `AXIS <ax>` takes a letter (`x`→0, `y`→1, `z`→2, `w`/`t`→3) or a
    0-based numeric index, then `AT (<c0>,<c1>)` (two `usize`
    coordinates).
  - `EDGES (…)` takes a comma-separated list of `usize` edge ids.
- **Direction convention:** a `+axis` walk (or an edge matching its
  stored direction) applies `U`; the reverse applies `U†`.
- SU(2)-only this phase (quaternion readout). A non-SU(2) gauge field
  errors:

  ```
  HOLONOMY AROUND CYCLE requires GROUP SU(2) in this phase (quaternion
  readout); got <group>
  ```
- `order_estimate` is meaningful only on clean lens-space wraps.
- **Response (200):**

  ```json
  {
    "q0":             0.5,
    "q1":             0.0,
    "q2":             0.0,
    "q3":             0.866,
    "re_trace":       0.5,
    "order_estimate": 3,
    "group_used":     "SU(2)"
  }
  ```

  `re_trace = ½·Tr = q0`; `order_estimate` reads the `π₁ = ℤ/p` order.

## GQL: evidence-grade error bars — WITH JACKKNIFE

```
INTEGRATE runs MEASURE avg(x) WITH JACKKNIFE ALONG sweep SKIP FIRST 500;
```

**What it does:** attaches autocorrelation-honest jackknife error bars
to each `avg()` measure. `ALONG <field>` is mandatory — it orders the
samples; omitting it errors with:

```
WITH JACKKNIFE requires ALONG <order_field> — the field that defines
sample order (e.g. sweep number or timestamp); autocorrelation is
undefined without an ordering
```

`SKIP FIRST n` is the optional thermalization cut (drop the first `n`
ordered samples before analysis; default 0).

## GQL: introspection — SHOW FIELDS / EXPLAIN SECTION

```
SHOW FIELDS ON demo;
EXPLAIN SECTION demo AT id=1;
EXPLAIN SECTION demo AT id=1 VECTOR (v0..v383);
EXPLAIN SECTION demo AT id IN (1, 2, 3);
```

- `SHOW FIELDS ON <bundle>;` returns one real row per field (`field`,
  `kind`, `type`, `indexed`, plus `range` when the field has one); a
  missing bundle errors with `No bundle: <name>`.
- `EXPLAIN SECTION <bundle> AT <key>=<value> [VECTOR (…)] [PROJECT
  (<fields>)];` returns the per-field curvature (kappa) decomposition of
  one record, loudest-first. Each row also carries `record_kappa` —
  constant per record, equal to the record's total κ; the mean of the
  per-field **scalar** kappas equals `record_kappa` (vector rows are
  excluded from that invariant).
- A missing key is a typed miss: **404**
  `{"error":"EXPLAIN: no section at <key>='<value>' in bundle '<b>'"}`.
  (Plain `SECTION AT` keeps its silent `200 {"rows":[],"count":0}` miss
  shape; only EXPLAIN's miss is loud.)
- `VECTOR (v0..v383)` (range sugar) / `VECTOR (f1, f2, …)` (explicit
  list; mixable) assembles the named scalar fibers into one virtual
  vector and appends ONE additive row tagged `kind:"vector"`:
  `kappa_v = |1 − cos(v, mu_v)| / R_cos`, where `mu_v` is the
  per-component bundle mean and `R_cos` the observed max − min of
  (1 − cos), EPSILON-floored — both computed on demand in the same call.
  The row carries `cos`, `one_minus_cos`, `r_cos`, `dim`, `n`. kappa_v
  does **not** participate in `record_kappa` and is not bounded by 1.
  True `Value::Vector` fiber fields get the row automatically (no
  clause needed).
- `AT <field> IN (v1, …, vn)` — batch form: grouped rows in the caller's
  input order, each group one record's full EXPLAIN output with the key
  value stamped as a discriminator column on every row. A missing key
  emits one `kind:"miss"` row naming the key and bundle instead of
  failing the batch; found-but-undecomposable records emit one
  `kind:"empty"` note row. One engine read-lock spans the batch.
- mmap-backed bundles no longer decline: per-field stats are computed on
  demand (one O(N) scan on first access, cached in memory until restart,
  nothing persisted). VECTOR contexts cost O(N) per vector target per
  statement. EXPLAIN is a diagnostic verb; that price is accepted.

## GQL: TIMESTAMP and NOW

`TIMESTAMP` is a first-class field type in CREATE BUNDLE and ALTER
BUNDLE. Value positions accept ISO-8601 literals (`'2026-07-02'`,
`'2026-07-02T14:30:05Z'`) or epoch-ms integers; `NOW` evaluates to the
current epoch-ms integer. Coercion to epoch-ms applies at the
write/compare boundary on both the insert and update paths, so
comparisons against date strings work in WHERE clauses.

---

# Brain primitives (`kahler` feature)

The twelve brain primitives sit on the Kähler bundle and consume the
free-energy substrate. Each takes a list of numeric fiber fields and
returns geometry-aware structure on top of those fields. SAMPLE is the
canonical entry point; the rest compose on the same machinery.

All Brain endpoints share these request fields:

| Field                 | Type           | Notes                                       |
|-----------------------|----------------|---------------------------------------------|
| `fields`              | `[string]`     | numeric fiber fields used as manifold dims  |
| `fit_mode`            | `string`       | `"isotropic"` (default) or `"diagonal"`     |
| `sigma_floor_epsilon` | `number`       | per-axis floor for diagonal fit (default `1e-3`) |
| `seed`                | `integer\|null` | PRNG seed; omit for entropy                |

## `POST /v1/bundles/{name}/brain/sample` `[read]`

**What it does:** SAMPLE — Langevin draws from the bundle's fitted
Gaussian.

**Additional fields:** `n_samples` (default 100), `temperature`
(default 1.0), `burn_in` (default 2000).

**Response (200):**

```json
{
  "samples":      [[0.31, 0.12], [0.28, 0.09]],
  "fit_mean":     [0.30, 0.10],
  "fit_sigma_sq": 0.0042
}
```

## `POST /v1/bundles/{name}/brain/dream` `[read]`

**What it does:** DREAM — anisotropic Langevin trajectory. Returns the
full walk so consumers can inspect narrative structure.

**Additional fields:** `initial` (defaults to fit mean), `n_steps`
(default 1000), `temperature` (default 4.0), `dt` (default 0.01).

**Response (200):**

```json
{
  "trajectory":          [[0.30, 0.10], [0.33, 0.12], "…"],
  "fit_mean":            [0.30, 0.10],
  "fit_sigma_sq":        0.0042,
  "temperature_used":    4.0,
  "mean_dist_from_mean": 0.18,
  "max_dist_from_mean":  0.42
}
```

## `POST /v1/bundles/{name}/brain/attend` `[read]`

**What it does:** ATTEND — softmax-weighted retrieval. With `top_k`
returns only the strongest `k` matches (FOCUS).

**Additional fields:** `query` (length must equal `fields`),
`bandwidth` (defaults to fit σ), `top_k` (optional).

**Response (200):**

```json
{
  "weights":   [0.41, 0.22, 0.19, "…"],
  "indices":   [37, 14, 5, "…"],
  "bandwidth": 0.064,
  "n_samples": 150
}
```

## `POST /v1/bundles/{name}/brain/confidence` `[read]`

**What it does:** CONFIDENCE — Fisher-precision gate at a candidate
point. The refuse-gate primitive Marcella uses to decide whether to
answer.

## `POST /v1/bundles/{name}/brain/confidence_with_explain` `[read]`

**What it does:** CONFIDENCE + EXPLAIN in one network call. Composite
refuse-gate.

## `POST /v1/bundles/{name}/brain/episodic` `[read]`

**What it does:** EPISODIC — change-point detection on a single field
via persistent H₀ on the sorted-values MST.

**Additional fields:** `field` (single field name), `min_persistence_ratio`
(default 50).

## `POST /v1/bundles/{name}/brain/explain` `[read]`

**What it does:** EXPLAIN — projection that reports which fields drive
a query result.

## `GET /v1/bundles/{name}/brain/semantic` `[read]`

**What it does:** SEMANTIC — Morse-compressed gist of the bundle.
Returned as a GET because there is no input beyond the bundle id.

## `POST /v1/bundles/{name}/brain/forecast` `[read]`

**What it does:** FORECAST — Hamilton-flow extension of a state.

## `POST /v1/bundles/{name}/brain/reconstruct` `[read]`

**What it does:** RECONSTRUCT — MAP estimate of a corrupted state.

## `POST /v1/bundles/{name}/brain/inpaint` `[read]`

**What it does:** INPAINT — conditional reconstruction; fills missing
field values given partial observation.

## `POST /v1/bundles/{name}/brain/predict` `[read]`

**What it does:** PREDICT — one-step prediction at a state.

## `POST /v1/bundles/{name}/brain/fit_diagnostics` `[read]`

**What it does:** the H2-vs-H1 verdict endpoint. Returns the full
eigenvalue spectrum, per-axis variance, and fit_mean for any
`(bundle, fit_mode, fields, sigma_floor_epsilon)` configuration. Warm
calls are sub-microsecond via the `BundleFlowCache`.

## `POST /v1/bundles/{name}/brain/distance_to_fit_mean` `[read]`

**What it does:** reports a target vector's percentile within the
bundle's distance distribution. Target at `p < 0.01` indicates an H2
attractor source.

## `POST /v1/bundles/{name}/brain/sudoku` `[read]`

**What it does:** SUDOKU — constraint-inference meta-primitive.
Tristate verdict (`granted | unreachable | indeterminate`) on field
predicates.

## `POST /v1/bundles/{name}/brain/sample_transport` `[read]`

**What it does:** curvature-bounded neighborhood sampling on the fiber.
Returns `k` candidates from `N(p_src, τ)` weighted by `exp(−β · d²)`.

## `POST /v1/bundles/{name}/brain/intent_gate` `[read]`

**What it does:** composite refuse-gate (SUDOKU + Čech pre-flight +
kernel-density confidence) in one atomic call.

---

# WISH (`wish` feature)

WISH is the boundary-value-problem geodesic verb — given seed and
target points, return the geodesic or a verdict of why none exists.

## `POST /v1/wish` `[read]`

**What it does:** global WISH against a built-in metric (Flat / S² /
CP¹ / Pinch). Verifies seed and target are finite; dispatches to the
solver.

**Request body:**

```json
{
  "seed":   [0.1, 0.1],
  "target": [0.9, 0.9],
  "metric": "s2",
  "max_imagined_curvature":  4.0,
  "max_accumulated_holonomy": 3.14,
  "max_arc_length":           10.0,
  "max_solve_ms":   2000,
  "max_iterations": 200,
  "n_nodes":  64,
  "grad_tol": 1e-6
}
```

`metric` values: `flat | s2 | cp1 | pinch`.

**Response (200):** shape depends on `verdict`.

```json
{
  "verdict": "granted",
  "unsat":   false,
  "capacity":             0.91,
  "arc_length":           1.24,
  "integrated_curvature": 0.18,
  "accumulated_holonomy": 0.02,
  "solver_iterations":    37,
  "path": [[0.10, 0.10], [0.18, 0.16], "…", [0.90, 0.90]]
}
```

Unreachable response carries `frontier_waypoint`, `waypoint_kind`,
`reached_fraction`, `blocked_by`, `capacity_to_waypoint`. Indeterminate
carries `reason` and `final_residual`.

**Errors:** `400` if `seed` or `target` contains a non-finite value.

## `POST /v1/bundles/{name}/wish` `[read]`

**What it does:** bundle-scoped WISH. Verifies the bundle exists, then
runs the same solver as `/v1/wish`. The substrate dim-lift will swap in
the bundle's Kähler metric without changing the request shape.

## `POST /v1/bundles/{name}/imagine_coherence` `[read]` (`imagine` feature)

**What it does:** IMAGINE_COHERENCE — predictive coherence trajectory
along an imagined geodesic. Marcella's predictive-gain gate.

---

# Patterns (`patterns` feature)

Phase 1 ships the parser-only surface; the in-memory registry executes
DEFINE / DROP / SHOW. Executor phases (HUNT, EXCLUDING IN) follow.

`DEFINE [OR REPLACE] PATTERN <name> AS <pred> [OR <pred>]* [WEIGHT (…)]
[USING (…)];` — the pattern body now consumes an `AND`/`OR` combinator
chain (COVER-parity; the pre-existing `OR` gap is closed). The base
predicate is `AND`-chained; each trailing `OR` opens a new
`AND`-chained alternative (an *or-group*). A row matches when the base
predicate matches **and** at least one or-group matches —
base-AND-(groups-ORed), not a flat boolean OR. `HUNT` desugars to
`COVER` and passes the or-groups through untouched.

## `GET /v1/patterns` `[read]`

**What it does:** lists every defined pattern.

## `POST /v1/patterns` `[write]`

**What it does:** defines a new pattern.

**Request body:**

```json
{ "name": "anomaly_high_score", "spec": "score > 0.95 AND active = true" }
```

## `DELETE /v1/patterns/{name}` `[write]`

**What it does:** drops the named pattern.

## `POST /v1/bundles/{name}/hunt` `[read]`

**What it does:** evaluates a pattern against the bundle. Returns
matching records with WEIGHT scores when the executor is enabled.

---

# Causal States (`causal_states` feature)

The Davis (2026) update-commutator wire — companion HTTP surface to the
paper's substrate.

## `POST /v1/causal_states/commutator` `[read]`

**What it does:** computes `Ω = U_a ∘ U_b − U_b ∘ U_a` on a base
belief. Returns forward / backward beliefs, the three scalar
diagnostics (TV / Hellinger / KL), and a Sofic / Smooth / Borderline
regime label.

**Request body:**

```json
{
  "a":    { "kind": "hmm", "alpha": 0.2, "beta": 0.3, "symbol": 0 },
  "b":    { "kind": "hmm", "alpha": 0.2, "beta": 0.3, "symbol": 1 },
  "base_belief": [0.5, 0.5],
  "bands": { "tv_low": 0.30, "tv_high": 0.95 }
}
```

Operator kinds: `even_u0 | even_u1 | hmm`. HMM requires `alpha`,
`beta`, `symbol ∈ {0, 1}`.

**Response (200):**

```json
{
  "forward":   [0.4469, 0.5531],
  "backward":  [0.5531, 0.4469],
  "tv":        0.1062,
  "hellinger": 0.0752,
  "kl":        { "kind": "finite", "value": 0.0327 },
  "regime":    "smooth"
}
```

**Errors:** `400` for bad input; KL responses set `"kind": "infinite"`
when one composition path is inadmissible.

---

# Lattice / Gauge field (`lattice`, `gauge` features)

Halcyon's HTTP substrate — the same routes back the in-process test
harness and the production binary.

## `POST /v1/lattice` `[write]`

**What it does:** declares a canonical lattice in the process-wide
registry. Currently ships `TRUNCATED_ICOSAHEDRON` (a.k.a. `BUCKYBALL`).

**Request body:**

```json
{
  "name":          "bb",
  "topology":      "TRUNCATED_ICOSAHEDRON",
  "topology_hint": "S2",
  "persist":       false
}
```

`persist: true` WAL-logs the declaration so it survives restarts.

**Response (200):**

```json
{
  "name":       "bb",
  "n_vertices": 60,
  "n_edges":    90,
  "n_faces":    32,
  "topology":   "S2",
  "gql":        "LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2'"
}
```

## `GET /v1/lattice/{name}` `[read]`

**What it does:** returns the lattice view (same shape as the POST
response).

## `POST /v1/gauge_field` `[write]`

**What it does:** declares a gauge field on a previously declared
lattice. SU(2) is fully implemented; U(1) / SU(3) / Z(N) parse but
fail at use.

**Request body:**

```json
{
  "name":    "U",
  "lattice": "bb",
  "group":   "SU(2)",
  "init":    { "kind": "identity" },
  "persist": false
}
```

`init.kind`: `identity | haar_random | from_field`. `haar_random`
requires `seed` (locked decision — bit-identity contract).

**Response (200):**

```json
{
  "name":      "U",
  "lattice":   "bb",
  "group":     "SU(2)",
  "repr_dim":  4,
  "n_edges":   90,
  "init_kind": "IDENTITY"
}
```

**Errors:** `400` for unknown group, undeclared lattice, missing seed
on `haar_random`.

## `GET /v1/gauge_field/{name}` `[read]`

**What it does:** returns the gauge-field envelope including the
row-major `(n_edges, repr_dim)` buffer. For SU(2) each row is the
scalar-first quaternion `(q₀, q₁, q₂, q₃)`.

## `GET /v1/gauge_field/{name}/plaquette` `[read]`

**What it does:** plaquette observable.

**Query params:** `reduction=per_face | mean | sum`.

**Response (200) — mean:**

```json
{ "reduction": "mean", "value": 0.812 }
```

**Response (200) — per_face:**

```json
{ "reduction": "per_face", "values": [0.81, 0.79, 0.82, "…"] }
```

## `GET /v1/gauge_field/{name}/observables/q_surrogate` `[read]`

**What it does:** scalar Q_SURROGATE.

```json
{ "value": 0.0042 }
```

## `POST /v1/gauge_field/{name}/observables` `[read]`

**What it does:** batched read of multiple observables. POST is the
consumer-safe batched-read pattern; no side effects.

**Request body:**

```json
{ "observables": ["mean_plaquette", "sum_plaquette", "q_surrogate"] }
```

**Response (200):**

```json
{
  "mean_plaquette": 0.812,
  "sum_plaquette":  73.08,
  "q_surrogate":    0.0042
}
```

## `GET /v1/e_field/{name}` `[read]`

**What it does:** electric-field metadata. Set `with_buffer=true` to
materialize the `(n_edges, 4)` Lie buffer (`q₀ = 0` invariant per row).

**Query params:** `with_buffer=true|false`.

## `GET /v1/gauge_field/{name}/h_total` `[read]`

**What it does:** total Hamiltonian — kinetic + potential.

**Query params:** `e_field=<name>` (required).

**Response (200):**

```json
{ "h_total": 12.41, "kinetic": 7.20, "potential": 5.21 }
```

## `GET /v1/gauge_field/{name}/gauss_residual_max` `[read]`

**What it does:** maximum Gauss residual.

**Query params:** `e_field=<name>` (required), `reduction=covariant |
flat` (default `covariant`).

## `GET /v1/symplectic_flow/diagnostics/{run_id}` `[read]`

**What it does:** reads a symplectic-flow run's diagnostics from the
process-local LRU cache. SYMPLECTIC_FLOW itself is only reachable via
`POST /v1/gql` (locked decision — no dedicated POST route).

---

# Sharded analytics (`sharded` feature)

API stable; execution bodies of some paths are still skeleton. See
`docs/STABILITY_GUARANTEES.md`.

## `POST /v1/bundles/{name}/sharded/spectral_gap` `[read]`

**What it does:** distributed Lanczos spectral-gap estimate over the
atlas-cover of the bundle.

## `POST /v1/bundles/{name}/sharded/curvature` `[read]`

**What it does:** sharded curvature report.

## `POST /v1/bundles/{name}/sharded/holonomy_loop` `[read]`

**What it does:** holonomy walk across charts of the atlas cover.

---

# Quantum cohomology (`kahler` feature)

## `POST /v1/quantum_cohomology/compose` `[read]`

**What it does:** Frobenius composition on the quantum cohomology ring.

## `POST /v1/quantum_cohomology/capacity` `[read]`

**What it does:** capacity report on the same ring.

## `POST /v1/bundles/{name}/holonomy_debt` `[read]`

**What it does:** holonomy-debt accounting along a transport path.

## `POST /v1/bundles/{name}/flat_transport` `[read]`

**What it does:** flat parallel transport between two field-space
points.

## `GET /v1/bundles/{name}/spectral_gap` `[read]`

**What it does:** Kähler spectral gap (Marcella contract surface).

---

# WebSockets

## `GET /ws` `[read]`

**What it does:** per-bundle subscription socket. Client sends a
JSON subscribe frame; server pushes updates as JSON frames.

## `GET /v1/ws/dashboard` `[read]`

**What it does:** global dashboard socket — engine-wide stats stream.

## `GET /v1/ws/{bundle}/dashboard` `[read]`

**What it does:** per-bundle dashboard socket.

## `GET /dashboard` `[read]`

**What it does:** serves the dashboard UI (HTML, not JSON).

---

# Admin

## `POST /v1/admin/snapshot` `[admin]`

**What it does:** triggers a DHOOM snapshot + WAL compaction.

**Response (200):**

```json
{ "status": "ok", "snapshot_path": "./gigi_data/snapshots/2026-06-20T10:00:00Z" }
```

## `GET /v1/admin/log-config` `[admin]`

**What it does:** returns the current log configuration.

## `POST /v1/admin/log-config` `[admin]`

**What it does:** updates the log configuration.

**Request body:**

```json
{ "level": "info", "json": true }
```

## `POST /v1/admin/log-level` `[admin]`

**What it does:** changes the runtime log level.

**Request body:**

```json
{ "level": "debug" }
```

---

# Observability

## `GET /v1/metrics` `[read]`

**What it does:** Prometheus-compatible metrics endpoint.

**Response:** `text/plain; version=0.0.4`.

**Curl:**

```bash
curl https://gigi-stream.fly.dev/v1/metrics | head
```

---

# Compatibility notes

- The public OpenAPI document at `/v1/openapi.json` enumerates the
  always-on surface plus a few stable extensions. Feature-gated
  endpoints (Brain, WISH, Causal States, Lattice / Gauge, Patterns,
  Sharded) are documented here but may not be reflected in the spec
  until they leave research stage.
- Request and response shapes for stable endpoints follow semver: a
  field is never removed without a major bump.
- Research-stage endpoints may change their request or response shapes
  between minor releases. See `docs/STABILITY_GUARANTEES.md` for the
  per-feature contract.
- For very large result sets prefer `/query-stream` over `/query` to
  bypass `GIGI_QUERY_MAX_ROWS`.
- Set `GIGI_CORS_ORIGIN=*` for local development; pin to your
  application origin in production.
- Engine locks are poison-proof: a request handler that panics fails
  that one request; it no longer poisons the shared lock and takes the
  rest of the server down with it.
- Server-side file I/O over GQL is fail-closed: `INGEST FROM '<file>'`
  requires `GIGI_INGEST_DIR` and `EMIT CSV TO` requires
  `GIGI_EMIT_DIR`; both refuse with an explicit error when unset and
  contain all paths inside the configured directory when set. The
  production deployment uses `/data/ingest`.
