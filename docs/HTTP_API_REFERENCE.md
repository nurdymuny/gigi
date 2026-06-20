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

## `GET /v1/bundles/{name}/horizon` `[read]`

**What it does:** Branch VII horizon report.

## `GET /v1/bundles/{name}/depth` `[read]`

**What it does:** Branch VII encoding-depth report.

## `POST /v1/bundles/{name}/perceive` `[read]`

**What it does:** Branch VII perception primitive.

## `POST /v1/bundles/{name}/local_holonomy` `[read]`

**What it does:** local holonomy at a base point.

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
GAUGE_FIELD, GIBBS_SAMPLE, SNAPSHOT, and every other declarative verb.

**Request body:**

```json
{ "query": "CREATE BUNDLE iris (id INT KEY, species TEXT, sepal_length FLOAT)" }
```

**Response (200):** shape depends on the statement.

```json
{ "status": "ok", "result": { /* statement-specific */ } }
```

**Errors:**
- `400` — parse error (body includes parser message).
- `404` — referenced bundle missing.

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
