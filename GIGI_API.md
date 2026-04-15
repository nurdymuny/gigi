# GIGI Stream — REST API Reference

**Version:** 0.5.0 (current)  
**Default port:** `3142` (configurable via `PORT` env var)  
**Base URL:** `http://localhost:3142`

---

## Overview

GIGI Stream exposes a REST API and WebSocket interface for a geometry-aware database engine. Every write operation returns curvature metadata — scalar curvature $K$, confidence $\in [0,1]$, and capacity — computed from the data's fiber bundle structure.

**Key concepts:**
- **Bundle** = a table. Has a schema with base fields (keys) and fiber fields (values).
- **Base fields** = the key columns that define a record's identity. Hashed for $O(1)$ point queries.
- **Fiber fields** = the value columns attached to each base point.
- **Curvature** = how much the data varies. $K = 0$ means uniform, $K > 0$ means varied.

---

## Quick Start

```bash
# Create a bundle
curl -X POST http://localhost:3142/v1/bundles \
  -H "Content-Type: application/json" \
  -d '{
    "name": "users",
    "schema": {
      "fields": { "id": "numeric", "name": "text", "email": "text", "age": "numeric" },
      "keys": ["id"],
      "indexed": ["email"]
    }
  }'

# Insert records
curl -X POST http://localhost:3142/v1/bundles/users/insert \
  -H "Content-Type: application/json" \
  -d '{ "records": [{ "id": 1, "name": "Alice", "email": "alice@example.com", "age": 30 }] }'

# Query
curl -X POST http://localhost:3142/v1/bundles/users/query \
  -H "Content-Type: application/json" \
  -d '{ "conditions": [{ "field": "age", "op": "gte", "value": 25 }], "limit": 10 }'

# Point lookup — O(1)
curl "http://localhost:3142/v1/bundles/users/get?id=1"
```

---

## Errors

All endpoints return errors in a consistent format:

```json
{ "error": "Bundle 'foo' not found" }
```

| HTTP Status | Meaning |
|---|---|
| `400` | Bad request (invalid body, missing params) |
| `404` | Bundle or record not found |
| `201` | Created (bundle creation only) |
| `200` | Success (everything else) |

---

## Endpoints

### Health

#### `GET /v1/health`

Server health check.

**Response:**
```json
{
  "status": "ok",
  "engine": "gigi-stream",
  "version": "0.1.0",
  "bundles": 3,
  "total_records": 15000
}
```

---

### Bundle Management

#### `GET /v1/bundles`

List all bundles.

**Response:**
```json
[
  { "name": "users", "records": 500, "fields": 4 },
  { "name": "events", "records": 12000, "fields": 6 }
]
```

---

#### `POST /v1/bundles`

Create a new bundle.

**Body:**
```json
{
  "name": "users",
  "schema": {
    "fields": {
      "id": "numeric",
      "name": "text",
      "email": "text",
      "score": "numeric"
    },
    "keys": ["id"],
    "defaults": { "score": 0 },
    "indexed": ["email"]
  }
}
```

| Schema Field | Type | Default | Description |
|---|---|---|---|
| `fields` | `{name: type}` | *required* | Field definitions. Types: `numeric`, `text`/`categorical`, `timestamp` |
| `keys` | `string[]` | `[]` | Base fields (primary key). Also auto-indexed |
| `defaults` | `{name: value}` | `{}` | Default values for fields |
| `indexed` | `string[]` | `[]` | Fields to index for fast range queries |

**Response:** `201 Created`
```json
{ "status": "created", "bundle": "users" }
```

---

#### `DELETE /v1/bundles/{name}`

Drop a bundle and all its data.

**Response:**
```json
{ "status": "dropped", "bundle": "users" }
```

---

#### `GET /v1/bundles/{name}/schema`

Get bundle schema, field definitions, and storage info.

**Response:**
```json
{
  "name": "users",
  "base_fields": [
    { "name": "id", "type": "Numeric", "weight": 1.0 }
  ],
  "fiber_fields": [
    { "name": "name", "type": "Categorical", "weight": 1.0 },
    { "name": "email", "type": "Categorical", "weight": 1.0 }
  ],
  "indexed_fields": ["email", "id"],
  "records": 500,
  "storage_mode": "Hashed"
}
```

---

### Insert

#### `POST /v1/bundles/{name}/insert`

Insert one or more records. Uses batch insert with $O(1)$ amortized per record.

**Alias:** `POST /v1/bundles/{name}/points` (same handler)

**Body:**
```json
{
  "records": [
    { "id": 1, "name": "Alice", "email": "alice@example.com" },
    { "id": 2, "name": "Bob", "email": "bob@example.com" }
  ]
}
```

**Response:**
```json
{
  "status": "inserted",
  "count": 2,
  "total": 502,
  "curvature": 0.0023,
  "confidence": 0.9977
}
```

---

#### `POST /v1/bundles/{name}/stream`

Streaming NDJSON ingest. Send newline-delimited JSON records via chunked body. Max 256 MB.

```bash
curl -X POST http://localhost:3142/v1/bundles/sensors/stream \
  -H "Content-Type: application/x-ndjson" \
  --data-binary @data.ndjson
```

**Response:**
```json
{
  "status": "streamed",
  "count": 10000,
  "parse_errors": 0,
  "total": 10000,
  "curvature": 0.0015,
  "confidence": 0.9985,
  "storage_mode": "Flat"
}
```

---

#### `POST /v1/bundles/{name}/upsert`

Insert a record if it doesn't exist, or update it if it does. Matching is by base fields (keys).

**Body:**
```json
{
  "record": { "id": 1, "name": "Alice Updated", "email": "alice.new@example.com" }
}
```

**Response:**
```json
{
  "status": "inserted",
  "total": 503,
  "curvature": 0.0023,
  "confidence": 0.9977
}
```

`status` is `"inserted"` for new records, `"updated"` for existing ones.

---

### Read

#### `GET /v1/bundles/{name}/get`

Point query — $O(1)$. Pass base field values as query parameters.

```
GET /v1/bundles/users/get?id=1
```

**Response:**
```json
{
  "data": { "id": 1, "name": "Alice", "email": "alice@example.com" },
  "meta": {
    "confidence": 0.9977,
    "curvature": 0.0023,
    "capacity": 434.78,
    "count": 1
  }
}
```

Returns `404` if not found.

---

#### `GET /v1/bundles/{name}/points/{field}/{value}`

Get a record by field/value in the URL path. Tries $O(1)$ point query first, falls back to range query for fiber fields.

```
GET /v1/bundles/users/points/email/alice@example.com
```

**Response:** Same shape as point query.

---

#### `GET /v1/bundles/{name}/range`

Range query — returns all records matching a field value. Pass as query parameter.

```
GET /v1/bundles/users/range?email=alice@example.com
```

**Response:**
```json
{
  "data": [{ "id": 1, "name": "Alice", "email": "alice@example.com" }],
  "meta": {
    "confidence": 0.9977,
    "curvature": 0.0023,
    "count": 1
  }
}
```

---

#### `GET /v1/bundles/{name}/points`

List all records with optional pagination.

```
GET /v1/bundles/users/points?limit=50&offset=0
```

| Param | Type | Default | Description |
|---|---|---|---|
| `limit` | `int` | none | Max records to return |
| `offset` | `int` | `0` | Skip N records |

**Response:**
```json
{
  "data": [{ "id": 1, "name": "Alice" }, ...],
  "meta": { "count": 50 }
}
```

---

#### `POST /v1/bundles/{name}/query`

Filtered query with conditions, sorting, pagination, text search, and field projection.

**Body:**
```json
{
  "conditions": [
    { "field": "age", "op": "gte", "value": 25 },
    { "field": "status", "op": "in", "value": ["active", "pending"] }
  ],
  "sort_by": "age",
  "sort_desc": true,
  "limit": 20,
  "offset": 0,
  "search": "alice",
  "search_fields": ["name", "email"],
  "fields": ["id", "name", "age"]
}
```

| Field | Type | Default | Aliases | Description |
|---|---|---|---|---|
| `conditions` | `ConditionSpec[]` | `[]` | `filters` | Filter conditions (AND logic) |
| `or_conditions` | `ConditionSpec[][]?` | `null` | — | OR condition groups (see below) |
| `sort_by` | `string?` | `null` | `order_by` | Field to sort by (single field) |
| `sort_desc` | `bool?` | `false` | — | Sort descending |
| `sort` | `SortSpec[]?` | `null` | — | Multi-field sort (overrides `sort_by`) |
| `order` | `string?` | `null` | — | `"desc"` or `"asc"` (overrides `sort_desc`) |
| `limit` | `int?` | `null` | — | Max results |
| `offset` | `int?` | `null` | — | Skip N results |
| `search` | `string?` | `null` | — | Text search term (case-insensitive, OR across fields) |
| `search_fields` | `string[]?` | `null` | — | Fields to search (default: all text fields) |
| `fields` | `string[]?` | `null` | — | Field projection — only return these fields |

**OR Conditions:**

Each inner array is ANDed within its group, and the groups are ORed together. The overall filter is:
`(conditions AND) AND (group[0] OR group[1] OR ...)`

```json
{
  "conditions": [{ "field": "department", "op": "eq", "value": "engineering" }],
  "or_conditions": [
    [{ "field": "status", "op": "eq", "value": "active" }],
    [{ "field": "status", "op": "eq", "value": "pending" }]
  ]
}
```

This returns engineering employees who are either active OR pending.

**Multi-field Sort:**

```json
{
  "sort": [
    { "field": "department", "desc": false },
    { "field": "salary", "desc": true }
  ]
}
```

Sorts by department ascending, then by salary descending within each department.

**Response:**
```json
{
  "data": [
    { "id": 1, "name": "Alice", "age": 30 }
  ],
  "meta": {
    "confidence": 0.9977,
    "curvature": 0.0023,
    "count": 1,
    "total": 150
  }
}
```

`count` = number of records returned (after limit/offset).  
`total` = total matching records (before limit/offset) — useful for pagination.

---

### Condition Operators

All operators for the `conditions` array:

| Operator | Aliases | Value Type | Description |
|---|---|---|---|
| `eq` | `=`, `==` | any | Equals |
| `neq` | `!=`, `<>` | any | Not equals |
| `gt` | `>` | numeric | Greater than |
| `gte` | `>=` | numeric | Greater than or equal |
| `lt` | `<` | numeric | Less than |
| `lte` | `<=` | numeric | Less than or equal |
| `contains` | `like` | string | Substring match (case-insensitive) |
| `starts_with` | `startswith` | string | Prefix match (case-insensitive) |
| `ends_with` | `endswith` | string | Suffix match (case-insensitive) |
| `regex` | `matches` | string | Regex pattern match |
| `in` | — | array | Value is one of the given values |
| `not_in` | `notin`, `nin` | array | Value is NOT one of the given values |
| `is_null` | `isnull` | — | Field is null or missing |
| `is_not_null` | `isnotnull`, `not_null` | — | Field exists and is not null |

**Examples:**
```json
{ "field": "status", "op": "eq", "value": "active" }
{ "field": "age", "op": "gte", "value": 18 }
{ "field": "name", "op": "contains", "value": "ali" }
{ "field": "name", "op": "regex", "value": "^[AB].*" }
{ "field": "role", "op": "in", "value": ["admin", "moderator"] }
{ "field": "deleted_at", "op": "is_null", "value": null }
```

For `is_null` / `is_not_null`, the `value` field is ignored but must be present (use `null`).

---

### Count & Exists

#### `POST /v1/bundles/{name}/count`

Count records matching conditions without returning them. Faster than a full query when you only need the count.

**Body:**
```json
{
  "conditions": [
    { "field": "status", "op": "eq", "value": "active" }
  ]
}
```

Pass `"conditions": []` to count all records.

**Response:**
```json
{ "count": 42, "total": 500 }
```

`count` = matching. `total` = bundle size.

---

#### `POST /v1/bundles/{name}/exists`

Check whether any record matches the conditions. Short-circuits — stops at the first match.

**Body:**
```json
{
  "conditions": [
    { "field": "email", "op": "eq", "value": "admin@example.com" }
  ]
}
```

**Response:**
```json
{ "exists": true }
```

---

#### `GET /v1/bundles/{name}/distinct/{field}`

Get all distinct (unique, non-null) values for a field.

```
GET /v1/bundles/users/distinct/status
```

**Response:**
```json
{
  "field": "status",
  "values": ["active", "inactive", "banned"],
  "count": 3
}
```

---

### Update

#### `POST /v1/bundles/{name}/update`

Update a single record by key. $O(1)$.

**Body:**
```json
{
  "key": { "id": 1 },
  "fields": { "name": "Alice Updated", "age": 31 }
}
```

**Response:**
```json
{
  "status": "updated",
  "total": 500,
  "curvature": 0.0023,
  "confidence": 0.9977
}
```

Returns `404` if the record doesn't exist.

---

#### `PATCH /v1/bundles/{name}/points/{field}/{value}`

Update a record by field/value in the URL path.

```bash
curl -X PATCH http://localhost:3142/v1/bundles/users/points/id/1 \
  -H "Content-Type: application/json" \
  -d '{ "fields": { "name": "Alice Updated" } }'
```

**Response:** Same as `POST .../update`.

---

#### `PATCH /v1/bundles/{name}/points`

Bulk update — update all records matching filter conditions.

**Body:**
```json
{
  "filter": [
    { "field": "status", "op": "eq", "value": "pending" }
  ],
  "fields": { "status": "approved" }
}
```

`filter` also accepts alias `filters`.

**Response:**
```json
{
  "status": "updated",
  "matched": 12,
  "total": 500,
  "curvature": 0.0023,
  "confidence": 0.9977
}
```

---

### Delete

#### `POST /v1/bundles/{name}/delete`

Delete a single record by key. $O(1)$.

**Body:**
```json
{ "key": { "id": 1 } }
```

**Response:**
```json
{
  "status": "deleted",
  "total": 499,
  "curvature": 0.0022,
  "confidence": 0.9978
}
```

---

#### `DELETE /v1/bundles/{name}/points/{field}/{value}`

Delete a record by field/value in the URL path.

```
DELETE /v1/bundles/users/points/id/1
```

**Response:** Same as `POST .../delete`.

---

#### `POST /v1/bundles/{name}/bulk-delete`

Delete all records matching conditions.

**Body:**
```json
{
  "conditions": [
    { "field": "status", "op": "eq", "value": "banned" }
  ]
}
```

`conditions` also accepts alias `filters`.

**Response:**
```json
{
  "status": "deleted",
  "deleted": 5,
  "total": 495,
  "curvature": 0.0021,
  "confidence": 0.9979
}
```

---

#### `POST /v1/bundles/{name}/truncate`

Delete all records in a bundle. Schema is preserved.

**Body:** `{}` (empty object)

**Response:**
```json
{ "status": "truncated", "removed": 500, "total": 0 }
```

---

### Analytics

#### `GET /v1/bundles/{name}/curvature`

Scalar curvature report. Computes Riemannian curvature $K$ across the bundle, confidence, capacity, and per-field curvature.

**Response:**
```json
{
  "K": 0.0023,
  "confidence": 0.9977,
  "capacity": 434.78,
  "per_field": [
    { "field": "age", "variance": 120.5, "range": 60.0, "k": 0.033 },
    { "field": "score", "variance": 45.2, "range": 100.0, "k": 0.005 }
  ]
}
```

**Interpretation:**
- $K \approx 0$ → flat data (uniform distribution)
- $K > 0$ → positive curvature (clustered/concentrated)
- Confidence = $e^{-K}$ — higher is more uniform
- Capacity = $\frac{1}{K}$ — information capacity of the bundle

---

#### `GET /v1/bundles/{name}/spectral`

Spectral analysis — eigenvalue gap, graph diameter, spectral capacity.

**Response:**
```json
{
  "lambda1": 0.15,
  "diameter": 4,
  "spectral_capacity": 26.67
}
```

- `lambda1` = first non-zero eigenvalue of the Laplacian (spectral gap)
- `diameter` = longest shortest path in the data graph
- `spectral_capacity` = $\frac{1}{\lambda_1}$

---

#### `GET /v1/bundles/{name}/consistency`

Čech cohomology $H^1$ consistency check. Verifies data integrity across fiber sections.

**Response:**
```json
{
  "h1": 0,
  "cocycles": [],
  "status": "consistent",
  "curvature": 0.0023
}
```

- `h1 = 0` → fully consistent
- `h1 > 0` → conflicts detected (overlapping sections disagree)

---

### Joins & Aggregation

#### `POST /v1/bundles/{name}/join`

Pullback join between two bundles. Left outer join semantics.

**Body:**
```json
{
  "right_bundle": "orders",
  "left_field": "user_id",
  "right_field": "customer_id"
}
```

**Response:**
```json
{
  "data": [
    {
      "left": { "user_id": 1, "name": "Alice" },
      "right": { "customer_id": 1, "amount": 99.99 }
    },
    {
      "left": { "user_id": 2, "name": "Bob" },
      "right": null
    }
  ],
  "meta": { "count": 2 }
}
```

---

#### `POST /v1/bundles/{name}/aggregate`

Fiber integral — GROUP BY aggregation with automatic `count`, `sum`, `avg`, `min`, `max`.
Optionally pre-filter records before aggregation.

**Body:**
```json
{
  "group_by": "department",
  "field": "salary",
  "conditions": [
    { "field": "status", "op": "eq", "value": "active" }
  ]
}
```

| Field | Type | Default | Description |
|---|---|---|---|
| `group_by` | `string` | required | Field to group by |
| `field` | `string` | required | Numeric field to aggregate |
| `conditions` | `ConditionSpec[]` | `[]` | Optional pre-filter before aggregation |

**Response:**
```json
{
  "groups": {
    "engineering": { "count": 50, "sum": 5000000, "avg": 100000, "min": 70000, "max": 150000 },
    "sales": { "count": 30, "sum": 1800000, "avg": 60000, "min": 40000, "max": 90000 }
  }
}
```

---

### Schema Evolution

#### `POST /v1/bundles/{name}/add-field`

Add a new fiber field to an existing bundle. All existing records get the field's default value.

**Body:**
```json
{
  "name": "status",
  "type": "categorical",
  "default": "active"
}
```

| Field | Type | Default | Description |
|---|---|---|---|
| `name` | `string` | required | New field name |
| `type` | `string` | `"categorical"` | Field type: `numeric`, `categorical`, `text`, `timestamp` |
| `default` | `any?` | type default | Default value for existing records |

**Response:**
```json
{
  "status": "ok",
  "field": "status",
  "type": "Categorical",
  "records_updated": 500
}
```

---

#### `POST /v1/bundles/{name}/add-index`

Create an index on an existing field. Builds the bitmap index from all current records. Idempotent — calling again on an already-indexed field is a no-op.

**Body:**
```json
{
  "field": "email"
}
```

**Response:**
```json
{
  "status": "ok",
  "field": "email",
  "indexed": true
}
```

---

#### `POST /v1/bundles/{name}/drop-field`

Remove a fiber field from an existing bundle. All existing records have that field deleted. Cannot drop base fields (key fields).

**Body:**
```json
{
  "field": "legacy_status"
}
```

**Response:**
```json
{
  "status": "field_dropped",
  "field": "legacy_status",
  "records": 500
}
```

Returns `404` if the field doesn't exist.

---

### Atomic Operations

#### `POST /v1/bundles/{name}/increment`

Atomically increment or decrement a numeric field. Preserves integer type when amount is a whole number.

**Body:**
```json
{
  "key": { "id": 1 },
  "field": "view_count",
  "amount": 1
}
```

| Field | Type | Default | Description |
|---|---|---|---|
| `key` | `Record` | required | Base field key identifying the record |
| `field` | `string` | required | Numeric field to increment |
| `amount` | `number` | `1` | Increment amount (negative to decrement) |

**Response:**
```json
{ "updated": true }
```

Returns `{ "updated": false }` if the record doesn't exist.

---

### Export & Import

#### `GET /v1/bundles/{name}/export`

Export all records as a JSON array.

**Response:**
```json
{
  "bundle": "users",
  "count": 500,
  "records": [
    { "id": 1, "name": "Alice", "email": "alice@example.com" },
    { "id": 2, "name": "Bob", "email": "bob@example.com" }
  ]
}
```

---

#### `GET /v1/bundles/{name}/dhoom`

Export the bundle as DHOOM wire format with compression statistics.

**Response:**
```json
{
  "bundle": "users",
  "count": 500,
  "dhoom": "users{id@1+1, name|Alice, email}:\n1, Alice, alice@example.com\n...",
  "json_bytes": 42000,
  "dhoom_bytes": 17500,
  "compression_pct": 58.3
}
```

- `dhoom` = the full DHOOM-encoded string (header + rows)
- `json_bytes` / `dhoom_bytes` = size comparison
- `compression_pct` = bytes saved relative to JSON

---

#### `POST /v1/bundles/{name}/import`

Import records from a JSON array. Each record is inserted (duplicates overwrite).

**Body:**
```json
{
  "records": [
    { "id": 1, "name": "Alice" },
    { "id": 2, "name": "Bob" }
  ]
}
```

**Response:**
```json
{
  "inserted": 2,
  "meta": {
    "confidence": 0.9977,
    "curvature": 0.0023,
    "capacity": 434.78
  }
}
```

---

### Auto-ID & Timestamps

**Auto-generated IDs:** When inserting records without providing the base field key, the engine auto-generates an incrementing integer ID (starting from 1). Only works when the schema has a single base field.

```json
// Insert without specifying the key
{ "records": [{ "name": "Alice" }] }

// Response includes the auto-generated ID
{ "inserted": 1, "meta": { ... } }
```

**Default timestamps:** If the schema includes `created_at` and/or `updated_at` fields, they are auto-populated:
- On insert: `created_at` and `updated_at` are set to the current UTC epoch milliseconds.
- On update: `updated_at` is refreshed automatically.

---

### TTL / Auto-Expiry

Records with a `_ttl` field are eligible for automatic expiry. The `_ttl` value is an epoch timestamp (milliseconds). Call `/expire_ttl` programmatically or manage expiry externally.

```json
{ "id": "session-001", "user": "alice", "_ttl": 1710700000000 }
```

---

## Advanced Analytics

These endpoints expose deeper geometric properties of the bundle's data manifold.

---

### `GET /v1/bundles/{name}/betti`

Betti numbers — topological invariants $\beta_0$ and $\beta_1$ of the data graph.

**Response:**
```json
{
  "beta_0": 1,
  "beta_1": 3
}
```

- `beta_0` = number of connected components ($\beta_0 = 1$ means fully connected)
- `beta_1` = number of independent cycles (loops) in the record graph

---

### `GET /v1/bundles/{name}/entropy`

Shannon entropy of the bundle's fiber distribution (in nats).

**Response:**
```json
{
  "entropy": 4.302,
  "unit": "nats"
}
```

Higher entropy = more diversity in the data. $H = 0$ means all records are identical.

---

### `GET /v1/bundles/{name}/free-energy`

Helmholtz free energy $F = -\tau \ln Z$ of the bundle's partition function.

**Query parameters:**

| Param | Type | Default | Description |
|---|---|---|---|
| `tau` | `float` | `1.0` | Temperature parameter $\tau$ |

```
GET /v1/bundles/sensors/free-energy?tau=0.5
```

**Response:**
```json
{
  "tau": 0.5,
  "free_energy": -12.45
}
```

---

### `POST /v1/bundles/{name}/geodesic`

Geodesic distance between two records through the data graph (shortest path of hops).

**Body:**
```json
{
  "from": { "id": 1 },
  "to": { "id": 42 },
  "max_hops": 50
}
```

| Field | Type | Default | Description |
|---|---|---|---|
| `from` | `Record` | required | Source record key |
| `to` | `Record` | required | Target record key |
| `max_hops` | `int` | `50` | Maximum path length to search |

**Response:**
```json
{
  "distance": 3.0,
  "path_found": true
}
```

`distance` is `null` and `path_found` is `false` if no path exists within `max_hops`.

---

### `GET /v1/bundles/{name}/metric`

Metric tensor of the fiber bundle — the matrix $g_{ij}$ encoding how fields relate geometrically.

**Response:**
```json
{
  "matrix": [[1.0, 0.12], [0.12, 0.87]],
  "eigenvalues": [1.09, 0.78],
  "condition_number": 1.4,
  "effective_dimension": 1.85,
  "field_names": ["age", "salary"]
}
```

- `matrix` = $d \times d$ metric tensor (fiber fields only)
- `eigenvalues` = eigenvalues of the metric (sorted descending)
- `condition_number` = $\lambda_{\max} / \lambda_{\min}$ — how isotropic the metric is
- `effective_dimension` = participation ratio $\frac{(\sum \lambda_i)^2}{\sum \lambda_i^2}$ — intrinsic dimensionality

---

## Anomaly Detection

---

### `POST /v1/bundles/{name}/anomalies`

Detect anomalous records using the K-score threshold ($\mu_K + n \cdot \sigma_K$). Returns records whose local curvature deviates more than `threshold_sigma` standard deviations above the bundle mean.

**Body:**
```json
{
  "threshold_sigma": 2.0,
  "filters": [
    { "field": "status", "op": "eq", "value": "active" }
  ],
  "fields": ["id", "name", "score"],
  "limit": 100,
  "include_scores": true
}
```

| Field | Type | Default | Description |
|---|---|---|---|
| `threshold_sigma` | `float` | `2.0` | Sigma multiplier for anomaly threshold |
| `filters` | `ConditionSpec[]` | `[]` | Pre-filter before anomaly scan |
| `fields` | `string[]` | `[]` | Restrict returned record fields (empty = all) |
| `limit` | `int` | `100` | Max anomalies to return |
| `include_scores` | `bool` | `true` | Include curvature/z-score in response |

**Response:**
```json
{
  "bundle": "employees",
  "threshold_sigma": 2.0,
  "k_mean": 0.021,
  "k_std": 0.008,
  "k_threshold": 0.037,
  "total_records": 500,
  "anomaly_count": 7,
  "anomalies": [
    {
      "record": { "id": 42, "name": "Charlie", "salary": 999999 },
      "local_curvature": 0.115,
      "z_score": 11.75,
      "confidence": 0.89,
      "deviation_norm": 3,
      "deviation_distance": 0.82,
      "neighbourhood_size": 4,
      "contributing_fields": ["salary"]
    }
  ]
}
```

---

### `GET /v1/bundles/{name}/health`

Bundle health snapshot: record count, global and per-record curvature statistics, confidence, and anomaly rate.

**Response:**
```json
{
  "bundle": "employees",
  "record_count": 500,
  "k_global": 0.021,
  "k_mean": 0.021,
  "k_std": 0.008,
  "k_threshold_2s": 0.037,
  "k_threshold_3s": 0.045,
  "confidence": 0.979,
  "anomaly_rate_2s": 0.014,
  "per_field": [
    { "field": "salary", "k": 0.033, "variance": 1.2e9, "range": 120000 },
    { "field": "age",    "k": 0.005, "variance": 85.4,  "range": 60 }
  ]
}
```

---

### `POST /v1/bundles/{name}/predict`

Predict field volatility by group. Groups records by `group_by` field, computes mean, standard deviation, and relative volatility index for `field` within each group.

**Body:**
```json
{
  "group_by": "department",
  "field": "salary"
}
```

**Response:**
```json
{
  "bundle": "employees",
  "group_by": "department",
  "field": "salary",
  "predictions": [
    { "group": "engineering", "count": 50, "mean": 105000, "std_dev": 18000, "volatility_index": 0.171 },
    { "group": "sales",       "count": 30, "mean": 62000,  "std_dev": 9500,  "volatility_index": 0.153 }
  ]
}
```

`volatility_index` = $\sigma / \max(|\mu|, 1)$ — relative dispersion within the group.

---

### `POST /v1/bundles/{name}/anomalies/field`

Anomaly detection scoped to a specific field. Returns records ranked by z-score where the named field is a primary contributing factor.

**Body:**
```json
{
  "field": "salary",
  "threshold_sigma": 2.0,
  "limit": 50
}
```

| Field | Type | Default | Description |
|---|---|---|---|
| `field` | `string` | required | Field to focus anomaly detection on |
| `threshold_sigma` | `float` | `2.0` | Sigma threshold |
| `limit` | `int` | `100` | Max results |

**Response:**
```json
{
  "bundle": "employees",
  "field": "salary",
  "threshold_sigma": 2.0,
  "anomaly_count": 3,
  "anomalies": [
    {
      "record": { "id": 42, "salary": 999999 },
      "local_curvature": 0.115,
      "z_score": 11.75,
      "confidence": 0.89,
      "deviation_norm": 3,
      "deviation_distance": 0.82,
      "neighbourhood_size": 4,
      "contributing_fields": ["salary"]
    }
  ]
}
```

---

## Vector Search

---

### `POST /v1/bundles/{name}/vector-search`

K-nearest neighbour search against a vector field. Finds the `top_k` most similar records using the specified distance metric.

**Body:**
```json
{
  "field": "embedding",
  "vector": [0.1, 0.5, -0.3, 0.8],
  "top_k": 10,
  "metric": "cosine",
  "filters": [
    { "field": "status", "op": "eq", "value": "active" }
  ]
}
```

| Field | Type | Default | Description |
|---|---|---|---|
| `field` | `string` | required | Name of the vector field to search |
| `vector` | `float[]` | required | Query vector (must match stored dimensionality) |
| `top_k` | `int` | `10` | Number of nearest neighbors to return |
| `metric` | `string` | `"cosine"` | Distance metric: `"cosine"`, `"euclidean"` / `"l2"`, `"dot"` / `"inner_product"` |
| `filters` | `ConditionSpec[]` | `[]` | Optional pre-filter — only score matching records |

**Response:**
```json
{
  "results": [
    { "score": 0.987, "record": { "id": 7, "text": "hello world", "embedding": [0.09, 0.51, -0.28, 0.79] } },
    { "score": 0.941, "record": { "id": 3, "text": "foo bar",     "embedding": [0.11, 0.44, -0.35, 0.82] } }
  ],
  "meta": {
    "count": 2,
    "metric": "cosine",
    "query_dims": 4,
    "top_k": 10
  }
}
```

For `cosine` and `dot`, higher score = more similar. For `euclidean`, lower score = closer.

---

## GQL — Query Language

---

### `POST /v1/gql`

Execute a GIGI Query Language statement (SQL-like syntax). Supports schema creation, data insertion, queries, aggregations, joins, and analytics in a single text protocol.

**Body:**
```json
{
  "query": "SELECT name, age FROM users WHERE age >= 25 ORDER BY age DESC LIMIT 10"
}
```

**Supported statements:**

```sql
-- Schema
CREATE BUNDLE employees (id) FIBER (name, salary, dept) INDEX (dept);

-- Insert
INSERT INTO employees (id, name, salary, dept) VALUES (1, 'Alice', 90000, 'Eng');

-- Select with conditions
SELECT name, salary FROM employees WHERE dept = 'Eng' AND salary >= 80000;

-- Aggregation
SELECT AVG(salary) FROM employees GROUP BY dept;

-- Join
JOIN users ON orders.user_id = users.id;

-- Analytics
CURVATURE employees;
SPECTRAL employees;

-- Fiber Geometric Analysis (NLP / embedding bundles)
-- Fiber-space Laplacian eigenmodes: how many semantic clusters exist?
SPECTRAL corpus ON FIBER (f11, f12) MODES 3;
-- Returns: [{ mode: 1, lambda: 0.012, ipr: 0.83 }, ...]

-- Fiber parallel transport: how much does the embedding rotate between two tokens?
TRANSPORT corpus FROM (token_id=42) TO (token_id=99) ON FIBER (f11, f12);
-- Returns: { transport_angle, t00, t01, t10, t11, displacement_0, displacement_1 }

-- Global fiber holonomy: is the fiber bundle curved globally?
HOLONOMY corpus ON FIBER (f11, f12) AROUND tense_label;
-- Returns centroid rows per group + summary: { holonomy_angle, holonomy_trivial }

-- Local fiber holonomy: fiber curvature near a query point
HOLONOMY corpus NEAR (f11=1.0, f12=0.0) WITHIN 0.3 ON FIBER (f11, f12) AROUND tense_label;
HOLONOMY corpus NEAR (f11=1.0, f12=0.0) WITHIN 0.1 METRIC cosine ON FIBER (f11, f12) AROUND tense_label;
-- Returns: { local_holonomy_angle, neighbourhood_size }

-- Gauge invariance test: do two bundles share the same fiber topology?
GAUGE corpus_en VS corpus_fr ON FIBER (f11, f12) AROUND tense_label;
-- Returns: { bundle1, bundle2, holonomy_1, holonomy_2, gauge_difference, gauge_invariant }

-- List bundles
SHOW BUNDLES;
```

**Response (SELECT):**
```json
{
  "data": [
    { "name": "Alice", "age": 30 },
    { "name": "Bob",   "age": 28 }
  ]
}
```

**Response (CREATE / INSERT):**
```json
{ "status": "ok" }
```

Returns `400` with `{ "error": "..." }` on parse errors.

---

## Admin

---

### `POST /v1/admin/snapshot`

Write DHOOM-encoded snapshots for all bundles to disk and compact the WAL to schema-only entries. Use this to reduce WAL size after large bulk loads.

**No body required.**

**Response:**
```json
{
  "status": "ok",
  "total_records_snapshotted": 15000,
  "message": "DHOOM snapshots written; WAL compacted to schema-only."
}
```

Requires admin access (same `GIGI_API_KEY` as other authenticated endpoints).

---

## WebSocket API

Connect to `ws://localhost:3142/ws` for real-time operations using a text-based command protocol.

### Dashboard WebSockets

Two additional WebSocket endpoints stream live analytics for dashboards:

- **`ws://localhost:3142/v1/ws/dashboard`** — global dashboard feed: broadcasts curvature, record counts, and anomaly alerts for all bundles.
- **`ws://localhost:3142/v1/ws/{bundle}/dashboard`** — per-bundle dashboard feed: streams continuous health metrics for a single bundle.

These push JSON objects automatically as data changes. No command protocol — just connect and receive.

### Commands

#### INSERT
```
INSERT bundle_name
<DHOOM encoded data>
```
**Response:** `OK inserted=N total=N K=0.0023 confidence=0.9977`

#### QUERY (point lookup)
```
QUERY bundle_name WHERE field = "value"
QUERY bundle_name WHERE field1 = "val1" AND field2 = "val2"
```
**Response:**
```
RESULT {"id":1,"name":"Alice"}
META confidence=0.9977 curvature=0.0023
```

#### RANGE
```
RANGE bundle_name WHERE field = "value"
```
**Response:**
```
RESULT [{"id":1},{"id":2}]
META count=2 confidence=0.9977 curvature=0.0023
```

#### SUBSCRIBE
```
SUBSCRIBE bundle_name WHERE field = "value"
```
**Response:** `SUBSCRIBED bundle_name WHERE field = "value"`

Subsequent inserts to matching records are pushed to the client.

#### CURVATURE
```
CURVATURE bundle_name
CURVATURE bundle_name.field_name
```
**Response:** `CURVATURE K=0.0023 confidence=0.9977 capacity=434.78`

#### CONSISTENCY
```
CONSISTENCY bundle_name
```
**Response:** `CONSISTENCY h1=0 cocycles=0`

---

## Value Types

| JSON Type | GIGI Type | Notes |
|---|---|---|
| integer | `Integer` | 64-bit signed |
| float | `Float` | 64-bit IEEE 754 |
| string | `Text` | UTF-8 |
| boolean | `Bool` | `true`/`false` |
| null | `Null` | Explicit null |
| — | `Timestamp` | Epoch ms (stored as i64, returned as number) |

---

## Field Types (Schema)

| Type String | Aliases | Description |
|---|---|---|
| `numeric` | `number`, `float`, `int`, `integer` | Numeric field (fiber metric: absolute difference) |
| `text` | `categorical` | Categorical field (fiber metric: 0 or 1) |
| `timestamp` | `time`, `date` | Timestamp field (treated as numeric) |

---

## JavaScript SDK

```bash
npm install @gigi-db/client
```

```typescript
import { GIGIClient } from '@gigi-db/client';

const db = new GIGIClient('http://localhost:3142');
const users = db.bundle('users');

// Schema
await users.create({
  fields: { id: 'numeric', name: 'text', email: 'text' },
  keys: ['id'],
  indexed: ['email']
});

// CRUD
await users.insert({ id: 1, name: 'Alice', email: 'alice@example.com' });
await users.upsert({ id: 1, name: 'Alice Updated' });
await users.update({ id: 1 }, { name: 'Alice V2' });
await users.deleteRecord({ id: 1 });

// Queries
const result = await users.get({ id: 1 });
const list = await users.listAll({ limit: 50, offset: 0 });
const filtered = await users.query({
  conditions: [{ field: 'age', op: 'gte', value: 25 }],
  sort_by: 'age',
  limit: 10,
  fields: ['id', 'name', 'age']
});

// Count, Exists, Distinct
const { count } = await users.count([{ field: 'status', op: 'eq', value: 'active' }]);
const { exists } = await users.exists([{ field: 'email', op: 'eq', value: 'admin@x.com' }]);
const { values } = await users.distinct('status');

// Bulk operations
await users.bulkUpdate({
  filter: [{ field: 'status', op: 'eq', value: 'pending' }],
  fields: { status: 'approved' }
});
await users.bulkDelete([{ field: 'status', op: 'eq', value: 'banned' }]);
await users.truncate();

// Path-style CRUD
await users.getByField('email', 'alice@example.com');
await users.updateByField('id', 1, { name: 'Alice' });
await users.deleteByField('id', 1);

// Analytics
const curv = await users.curvature();    // { K, confidence, capacity, per_field }
const spec = await users.spectral();     // { lambda1, diameter, spectral_capacity }
const cons = await users.checkConsistency(); // { h1, cocycles }

// Schema info
const schema = await users.schema();

// Joins & Aggregation
const joined = await users.join('orders', 'id', 'customer_id');
const agg = await users.aggregate({ groupBy: 'department', field: 'salary' });
const filteredAgg = await users.aggregate({
  groupBy: 'department',
  field: 'salary',
  conditions: [{ field: 'status', op: 'eq', value: 'active' }]
});

// Sprint 2: OR conditions + multi-field sort
const complex = await users.query({
  conditions: [{ field: 'department', op: 'eq', value: 'engineering' }],
  or_conditions: [
    [{ field: 'status', op: 'eq', value: 'active' }],
    [{ field: 'status', op: 'eq', value: 'pending' }]
  ],
  sort: [{ field: 'department' }, { field: 'salary', desc: true }],
  limit: 50
});

// Atomic operations
await users.increment({ id: 1 }, 'view_count', 1);

// Schema evolution
await users.addField('status', 'categorical', 'active');
await users.addIndex('status');

// Export & Import
const data = await users.export();
await users.importData(data.records);

// Real-time subscriptions (WebSocket)
const unsubscribe = users.where('department', 'engineering').subscribe(records => {
  console.log('New records:', records);
});
```

---

## Endpoint Summary

| Method | Path | Description |
|---|---|---|
| `GET` | `/v1/health` | Health check |
| `GET` | `/v1/bundles` | List bundles |
| `POST` | `/v1/bundles` | Create bundle |
| `DELETE` | `/v1/bundles/{name}` | Drop bundle |
| `GET` | `/v1/bundles/{name}/schema` | Bundle schema |
| `POST` | `/v1/bundles/{name}/insert` | Insert records |
| `POST` | `/v1/bundles/{name}/stream` | NDJSON stream ingest |
| `POST` | `/v1/bundles/{name}/upsert` | Insert or update |
| `GET` | `/v1/bundles/{name}/get` | Point query $O(1)$ |
| `GET` | `/v1/bundles/{name}/range` | Range query |
| `POST` | `/v1/bundles/{name}/query` | Filtered query |
| `POST` | `/v1/bundles/{name}/count` | Count matching |
| `POST` | `/v1/bundles/{name}/exists` | Existence check |
| `GET` | `/v1/bundles/{name}/distinct/{field}` | Distinct values |
| `POST` | `/v1/bundles/{name}/update` | Update by key $O(1)$ |
| `POST` | `/v1/bundles/{name}/delete` | Delete by key $O(1)$ |
| `POST` | `/v1/bundles/{name}/bulk-delete` | Bulk delete by filter |
| `POST` | `/v1/bundles/{name}/truncate` | Truncate (clear all) |
| `GET` | `/v1/bundles/{name}/points` | List all records |
| `POST` | `/v1/bundles/{name}/points` | Insert (alias) |
| `PATCH` | `/v1/bundles/{name}/points` | Bulk update |
| `GET` | `/v1/bundles/{name}/points/{f}/{v}` | Get by field/value |
| `PATCH` | `/v1/bundles/{name}/points/{f}/{v}` | Update by field/value |
| `DELETE` | `/v1/bundles/{name}/points/{f}/{v}` | Delete by field/value |
| `POST` | `/v1/bundles/{name}/join` | Pullback join |
| `POST` | `/v1/bundles/{name}/aggregate` | Group-by aggregation |
| `GET` | `/v1/bundles/{name}/curvature` | Curvature report |
| `GET` | `/v1/bundles/{name}/spectral` | Spectral analysis |
| `GET` | `/v1/bundles/{name}/consistency` | Consistency check |
| `GET` | `/v1/bundles/{name}/betti` | Betti numbers $\beta_0, \beta_1$ |
| `GET` | `/v1/bundles/{name}/entropy` | Shannon entropy |
| `GET` | `/v1/bundles/{name}/free-energy` | Helmholtz free energy |
| `POST` | `/v1/bundles/{name}/geodesic` | Geodesic distance |
| `GET` | `/v1/bundles/{name}/metric` | Metric tensor |
| `POST` | `/v1/bundles/{name}/anomalies` | Anomaly detection |
| `GET` | `/v1/bundles/{name}/health` | Bundle health snapshot |
| `POST` | `/v1/bundles/{name}/predict` | Field volatility prediction |
| `POST` | `/v1/bundles/{name}/anomalies/field` | Per-field anomaly detection |
| `POST` | `/v1/bundles/{name}/vector-search` | K-NN vector similarity search |
| `POST` | `/v1/bundles/{name}/increment` | Atomic increment |
| `POST` | `/v1/bundles/{name}/add-field` | Add schema field |
| `POST` | `/v1/bundles/{name}/drop-field` | Drop schema field |
| `POST` | `/v1/bundles/{name}/add-index` | Add index |
| `GET` | `/v1/bundles/{name}/export` | Export JSON |
| `GET` | `/v1/bundles/{name}/dhoom` | Export DHOOM format |
| `POST` | `/v1/bundles/{name}/import` | Import JSON |
| `GET` | `/v1/bundles/{name}/stats` | Bundle statistics |
| `POST` | `/v1/bundles/{name}/explain` | Query execution plan |
| `POST` | `/v1/bundles/{name}/transaction` | Atomic transaction |
| `POST` | `/v1/gql` | GQL query language |
| `POST` | `/v1/admin/snapshot` | DHOOM snapshot + WAL compaction |
| `GET` | `/v1/openapi.json` | OpenAPI 3.0 spec |
| `GET` | `/ws` | WebSocket (command protocol) |
| `GET` | `/v1/ws/dashboard` | WebSocket global dashboard feed |
| `GET` | `/v1/ws/{bundle}/dashboard` | WebSocket per-bundle dashboard feed |

**55 endpoints.**

---

## Sprint 3: Enterprise Features

### Authentication

Set the `GIGI_API_KEY` environment variable to enable API key authentication. When set, all requests (except `GET /v1/health`) must include the `X-API-Key` header.

```
X-API-Key: your-secret-key
```

**Response (401):**
```json
{ "error": "missing or invalid API key" }
```

### Rate Limiting

Configure via environment variables:
- `GIGI_RATE_LIMIT` — max requests per window per IP (default: `0` = unlimited)
- `GIGI_RATE_WINDOW` — window duration in seconds (default: `60`)

When exceeded, returns `429 Too Many Requests`.

---

### Update with RETURNING & Optimistic Concurrency

#### `POST /v1/bundles/{name}/update`

Enhanced update with optional `returning` (get patched record back) and `expected_version` (optimistic concurrency control).

**Body:**
```json
{
  "key": { "id": 1 },
  "fields": { "salary": 90000 },
  "returning": true,
  "expected_version": 2
}
```

**Response (200) — success with RETURNING:**
```json
{
  "status": "updated",
  "version": 3,
  "total": 100,
  "curvature": 0.12,
  "confidence": 0.85,
  "data": { "id": 1, "name": "Alice", "salary": 90000, "_version": 3 }
}
```

**Response (409) — version conflict:**
```json
{
  "error": "version_conflict",
  "current_version": 5
}
```

---

### Delete with RETURNING

#### `POST /v1/bundles/{name}/delete`

Enhanced delete with optional `returning` (get deleted record back).

**Body:**
```json
{
  "key": { "id": 1 },
  "returning": true
}
```

**Response (200) — with RETURNING:**
```json
{
  "status": "deleted",
  "total": 99,
  "curvature": 0.11,
  "confidence": 0.84,
  "data": { "id": 1, "name": "Alice", "salary": 75000 }
}
```

---

### Bundle Statistics

#### `GET /v1/bundles/{name}/stats`

Returns comprehensive bundle statistics including field cardinalities, index sizes, and curvature.

**Response:**
```json
{
  "name": "employees",
  "record_count": 500,
  "base_fields": 1,
  "fiber_fields": 3,
  "indexed_fields": ["dept"],
  "storage_mode": "flat",
  "index_sizes": { "dept": 5 },
  "field_cardinalities": { "dept": 5, "name": 500 },
  "field_stats": {
    "salary": { "min": 50000, "max": 150000, "avg": 85000, "count": 500 }
  },
  "curvature": 0.12
}
```

---

### EXPLAIN Query Plan

#### `POST /v1/bundles/{name}/explain`

Returns the query execution plan without running the query.

**Body:**
```json
{
  "conditions": [
    { "field": "dept", "op": "eq", "value": "Eng" }
  ],
  "sort": [{ "field": "salary", "desc": true }],
  "limit": 10
}
```

**Response:**
```json
{
  "scan_type": "index_scan",
  "total_records": 500,
  "index_scans": ["dept"],
  "full_scan_conditions": [],
  "or_group_count": 0,
  "has_sort": true,
  "has_limit": true,
  "has_offset": false,
  "storage_mode": "flat"
}
```

---

### Atomic Transactions

#### `POST /v1/bundles/{name}/transaction`

Execute multiple operations atomically. If any operation fails, all changes are rolled back.

**Supported operations:** `insert`, `update`, `delete`, `increment`

**Body:**
```json
{
  "ops": [
    { "op": "insert", "record": { "id": 10, "name": "Charlie", "salary": 70000, "dept": "Eng" } },
    { "op": "update", "key": { "id": 1 }, "fields": { "salary": 95000 } },
    { "op": "delete", "key": { "id": 5 } },
    { "op": "increment", "key": { "id": 2 }, "field": "salary", "amount": 5000 }
  ]
}
```

**Response (200) — success:**
```json
{
  "status": "committed",
  "results": ["Ok", "Ok", "Ok", "Ok"],
  "total": 501,
  "curvature": 0.13,
  "confidence": 0.86
}
```

**Response (400) — rolled back:**
```json
{
  "error": "Transaction failed at op 2: Record not found — rolled back"
}
```

---

### OpenAPI Specification

#### `GET /v1/openapi.json`

Returns the full OpenAPI 3.0 specification for the GIGI Stream API. No authentication required.
