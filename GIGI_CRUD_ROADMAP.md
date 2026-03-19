# GIGI CRUD & Database Feature Roadmap

**Version:** 0.1  
**Date:** 2026-03-17  
**Purpose:** Comprehensive gap analysis — what every mature database has that GIGI doesn't (yet)

---

## How to Read This

Each feature is tagged:

| Tag | Meaning |
|---|---|
| **T1** | Tier 1 — Table stakes. Every database has this. Users expect it. |
| **T2** | Tier 2 — Common. Most production databases have this. |
| **T3** | Tier 3 — Advanced. Enterprise/specialized databases have this. |
| **GIGI+** | Uniquely GIGI — leverage our geometry in ways others can't. |

Difficulty estimates:

| Tag | Meaning |
|---|---|
| `[S]` | Small — < 50 lines, hours |
| `[M]` | Medium — 50–300 lines, a day |
| `[L]` | Large — 300+ lines, days |
| `[XL]` | Extra large — architecture change, week+ |

---

## ✅ What We Already Have

For reference — what's done and working today:

| Category | Feature | Status |
|---|---|---|
| **Create** | INSERT single/batch, NDJSON stream | ✅ |
| **Read** | Point query (O(1)), range query, filtered query, list all | ✅ |
| **Update** | Partial update by key, by field/value, bulk update with filter | ✅ |
| **Delete** | Delete by key, by field/value | ✅ |
| **Schema** | Create bundle, drop bundle, base/fiber fields, indexes | ✅ |
| **Persistence** | WAL, checkpoint, compaction, replay | ✅ |
| **Analytics** | Curvature, spectral gap, consistency check | ✅ |
| **Query ops** | eq, neq, gt, gte, lt, lte, contains, starts_with | ✅ |
| **Query features** | Sort, limit, offset, multi-field text search | ✅ |
| **Joins** | Pullback join (2-bundle) | ✅ |
| **Aggregation** | GROUP BY with count/sum/avg/min/max | ✅ |
| **TTL** | Expiry via `_ttl` field | ✅ |
| **Wire** | REST, WebSocket, DHOOM | ✅ |

---

## 1. Query Operations

### 1.1 Missing Operators

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 1.1.1 | **IN operator** | T1 | `[S]` | `field IN [val1, val2, val3]` — match any value in a set. Every SQL DB, Mongo `$in`, Redis. Currently requires N separate `eq` conditions OR'd manually. |
| 1.1.2 | **NOT IN operator** | T1 | `[S]` | `field NOT IN [val1, val2]` — exclude a set of values. |
| 1.1.3 | **BETWEEN operator** | T1 | `[S]` | `field BETWEEN min AND max` — inclusive range. Currently requires `gte` + `lte` as two conditions. |
| 1.1.4 | **IS NULL / IS NOT NULL** | T1 | `[S]` | Check for missing/present fields. Currently no way to query "all records where X is null." |
| 1.1.5 | **Regex match** | T2 | `[S]` | `field MATCHES "^CAS-\d+"` — full regex. Mongo `$regex`, Postgres `~`. We only have `contains` and `starts_with`. |
| 1.1.6 | **Case-sensitive match** | T2 | `[S]` | Option for exact-case matching. Our `contains`/`starts_with` are always case-insensitive. |
| 1.1.7 | **Ends with** | T2 | `[S]` | `field ENDS_WITH ".gov"` — suffix match. |
| 1.1.8 | **Numeric range scan** | T2 | `[M]` | Efficient `field > 100 AND field < 200` using sorted index instead of full scan. |

### 1.2 Logical Operators

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 1.2.1 | **OR conditions** | T1 | `[M]` | `(status = "open") OR (status = "escalated")`. Currently all conditions are AND'd. Every SQL DB, Mongo `$or`. |
| 1.2.2 | **NOT / negate** | T1 | `[S]` | `NOT (status = "closed")`. We have `neq` but no general negation of compound conditions. |
| 1.2.3 | **Nested AND/OR** | T2 | `[M]` | `(A AND B) OR (C AND D)` — arbitrary nesting. Mongo, SQL, Elasticsearch. |

### 1.3 Result Shaping

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 1.3.1 | **Field projection** | T1 | `[S]` | Return only specific fields: `SELECT name, email FROM ...` instead of `SELECT *`. Reduces payload, improves privacy. Every DB has this. |
| 1.3.2 | **Field aliasing** | T2 | `[S]` | `SELECT name AS displayName` — rename fields in response. SQL standard. |
| 1.3.3 | **Multi-field sort** | T1 | `[S]` | `ORDER BY status ASC, created DESC` — sort by multiple fields. We only sort by one field. |
| 1.3.4 | **DISTINCT / unique values** | T1 | `[S]` | Return unique values of a field. `SELECT DISTINCT status FROM cases`. We have `indexed_values()` internally but no REST endpoint. |
| 1.3.5 | **COUNT without data** | T1 | `[S]` | `SELECT COUNT(*) WHERE status = 'open'` — return just the count, not all records. Massive perf win for large results. |
| 1.3.6 | **EXISTS check** | T1 | `[S]` | `EXISTS(id = 'CAS-042')` — boolean check without fetching the record. Lighter than point_query. |
| 1.3.7 | **Cursor-based pagination** | T2 | `[M]` | Stateful cursor for iterating large result sets. Better than offset/limit for large datasets. Mongo cursor, SQL FETCH NEXT. |
| 1.3.8 | **Total count in paginated responses** | T1 | `[S]` | Return `{ data: [...], total: 1542, page: 3, pages: 31 }`. Currently we return count of the page but not total matching. |

### 1.4 Aggregations

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 1.4.1 | **COUNT DISTINCT** | T1 | `[S]` | Count of unique values in a field. |
| 1.4.2 | **Filtered aggregation** | T1 | `[M]` | `GROUP BY status WHERE module = 'sanctions'` — aggregate only matching records. Currently our aggregate has no filter. |
| 1.4.3 | **Multiple aggregation fields** | T2 | `[M]` | Aggregate over multiple fields in one call. Currently one field at a time. |
| 1.4.4 | **HAVING clause** | T2 | `[M]` | Filter groups after aggregation: `GROUP BY dept HAVING COUNT > 5`. |
| 1.4.5 | **Top-N per group** | T2 | `[M]` | Return top N records per group, not just aggregates. |
| 1.4.6 | **Percentile / median** | T2 | `[M]` | P50, P95, P99, median. Common in analytics DBs. |
| 1.4.7 | **Standard deviation** | T2 | `[S]` | We have variance internally — just expose `stddev`. |
| 1.4.8 | **Histogram / bucketing** | T3 | `[M]` | Auto-bucket numeric values into ranges. |

---

## 2. Write Operations

### 2.1 Insert Variants

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 2.1.1 | **UPSERT / INSERT OR UPDATE** | T1 | `[M]` | Insert if not exists, update if exists. Postgres `ON CONFLICT`, Mongo `upsert: true`, Redis `SET`. Extremely common pattern. |
| 2.1.2 | **INSERT IF NOT EXISTS** | T1 | `[S]` | Insert only if the key doesn't exist. Return error/skip if it does. Prevents accidental overwrites. |
| 2.1.3 | **INSERT OR IGNORE** | T2 | `[S]` | Silently skip duplicates in a batch. SQLite `INSERT OR IGNORE`. |
| 2.1.4 | **REPLACE / INSERT OR REPLACE** | T2 | `[S]` | Delete existing + insert new. MySQL `REPLACE INTO`. |
| 2.1.5 | **RETURNING clause** | T2 | `[S]` | Return the inserted/updated record after the write. Postgres `INSERT ... RETURNING *`, Mongo `findOneAndUpdate`. Currently we return status + curvature but not the record itself. |
| 2.1.6 | **Auto-generated IDs** | T1 | `[M]` | Auto-increment integers or UUID generation. Every SQL DB `SERIAL/AUTO_INCREMENT`, Mongo `_id`. |
| 2.1.7 | **Default timestamps** | T1 | `[S]` | Auto-set `created_at` / `updated_at` fields on insert/update. |

### 2.2 Update Variants

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 2.2.1 | **Atomic increment / decrement** | T1 | `[S]` | `UPDATE counters SET hits = hits + 1` without read-modify-write race. Mongo `$inc`, Redis `INCR`. |
| 2.2.2 | **Atomic multiply** | T3 | `[S]` | `SET price = price * 1.1` — multiply in place. Mongo `$mul`. |
| 2.2.3 | **Min/Max update** | T2 | `[S]` | `SET high_score = MAX(high_score, new_score)` — only update if new value is greater/less. Mongo `$min/$max`. |
| 2.2.4 | **Conditional update** | T2 | `[M]` | `UPDATE ... WHERE version = 5` — optimistic concurrency control. Only update if current value matches. CAS (compare-and-swap). |
| 2.2.5 | **Array push / append** | T2 | `[M]` | Append to an array field without read-modify-write. Mongo `$push`. PRISM asked for this. |
| 2.2.6 | **Array pull / remove** | T2 | `[M]` | Remove element from array field. Mongo `$pull`. |
| 2.2.7 | **Set add / remove** | T3 | `[M]` | Add to set (deduplicated). Mongo `$addToSet`. |
| 2.2.8 | **Rename field on record** | T3 | `[S]` | Rename a field in a specific record. Mongo `$rename`. |
| 2.2.9 | **Unset / remove field** | T2 | `[S]` | Remove a field from a record entirely. Mongo `$unset`. Set to Null vs truly absent. |

### 2.3 Delete Variants

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 2.3.1 | **Bulk delete by filter** | T1 | `[M]` | `DELETE WHERE status = 'archived'` — delete all matching records. We have bulk_update but no bulk_delete. |
| 2.3.2 | **Delete all / truncate** | T1 | `[S]` | Clear all records from a bundle without dropping it. SQL `TRUNCATE TABLE`, Mongo `deleteMany({})`. |
| 2.3.3 | **Soft delete** | T2 | `[M]` | Mark as deleted (set `_deleted = true`) instead of actual removal. Recoverable. Common in enterprise apps. |
| 2.3.4 | **Delete with RETURNING** | T2 | `[S]` | Return the deleted record. Postgres `DELETE ... RETURNING *`, Mongo `findOneAndDelete`. |

---

## 3. Schema & Structure

### 3.1 Schema Evolution

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 3.1.1 | **Add field to existing bundle** | T1 | `[M]` | `ALTER TABLE ADD COLUMN` — add a new fiber field with a default value. Currently must drop and recreate. |
| 3.1.2 | **Remove field from bundle** | T2 | `[M]` | `ALTER TABLE DROP COLUMN`. |
| 3.1.3 | **Rename field** | T2 | `[M]` | `ALTER TABLE RENAME COLUMN`. |
| 3.1.4 | **Change field type** | T3 | `[L]` | `ALTER TABLE ALTER COLUMN TYPE` — migrate Integer → Float etc. |
| 3.1.5 | **Get bundle schema** | T1 | `[S]` | `GET /v1/bundles/{name}/schema` — return field names, types, indexes, constraints. Currently only name + count returned. |
| 3.1.6 | **Rename bundle** | T2 | `[S]` | `ALTER TABLE RENAME TO`. |

### 3.2 Constraints

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 3.2.1 | **Unique constraint** | T1 | `[M]` | Enforce uniqueness on non-key fields. E.g., `email UNIQUE`. |
| 3.2.2 | **NOT NULL constraint** | T1 | `[S]` | Require a field to be present. Currently all fields are nullable. |
| 3.2.3 | **Check constraint** | T2 | `[M]` | `CHECK (age >= 0 AND age <= 150)` — value range validation. |
| 3.2.4 | **Foreign key constraint** | T2 | `[L]` | `REFERENCES other_bundle(id)` — referential integrity. |
| 3.2.5 | **Default value on insert** | T1 | `[S]` | Schema-level defaults. We have `FieldDef.default` but it's not enforced in the REST API / used consistently. |

### 3.3 Indexes

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 3.3.1 | **Create index on existing bundle** | T1 | `[M]` | `CREATE INDEX ON bundle(field)` — add index after creation. Currently only via schema at create time. |
| 3.3.2 | **Drop index** | T1 | `[S]` | `DROP INDEX`. |
| 3.3.3 | **List indexes** | T1 | `[S]` | Show which fields are indexed on a bundle. |
| 3.3.4 | **Compound index** | T2 | `[L]` | Index on multiple fields together. `CREATE INDEX ON (status, module)`. |
| 3.3.5 | **Unique index** | T2 | `[M]` | Index that also enforces uniqueness. |
| 3.3.6 | **Partial / filtered index** | T3 | `[L]` | Index only records matching a condition. Postgres `WHERE` clause on indexes. |
| 3.3.7 | **Full-text index** | T3 | `[XL]` | Inverted index for text search. Elasticsearch, Postgres `tsvector`. Our `contains` does a scan. |

---

## 4. Transactions & Concurrency

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 4.1 | **Multi-op transaction** | T2 | `[XL]` | `BEGIN` → multiple INSERT/UPDATE/DELETE → `COMMIT` or `ROLLBACK`. All-or-nothing atomicity. |
| 4.2 | **Optimistic concurrency (versioning)** | T2 | `[M]` | Each record has a `_version` field. Updates fail if version doesn't match. Prevents lost updates. CouchDB, DynamoDB. |
| 4.3 | **Read-your-writes consistency** | T2 | `[M]` | After a write, any subsequent read by the same client sees the write. Important for distributed setups. |
| 4.4 | **Batch mixed operations** | T2 | `[L]` | Submit a batch of `[{op: "insert", ...}, {op: "update", ...}, {op: "delete", ...}]` in a single request. DynamoDB `BatchWriteItem`. |

---

## 5. Administration & Operations

### 5.1 Introspection

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 5.1.1 | **Bundle stats endpoint** | T1 | `[S]` | Record count, storage size (bytes), index size, storage mode, create time. Currently `/health` gives totals only. |
| 5.1.2 | **EXPLAIN / query plan** | T2 | `[M]` | Show how a query will be executed — full scan vs index lookup vs point query. Invaluable for debugging perf. |
| 5.1.3 | **Slow query log** | T2 | `[M]` | Log queries that exceed a time threshold. |
| 5.1.4 | **Active connections** | T2 | `[S]` | Show connected WebSocket clients and their subscriptions. |
| 5.1.5 | **Server metrics** | T2 | `[M]` | Request count, latency percentiles, error rates. Prometheus-compatible `/metrics`. |

### 5.2 Backup & Recovery

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 5.2.1 | **Export bundle to JSON** | T1 | `[M]` | `GET /v1/bundles/{name}/export` → download all records as JSON array. Currently must use list-all + pagination. |
| 5.2.2 | **Export bundle to CSV** | T1 | `[M]` | Same but CSV format. |
| 5.2.3 | **Import from JSON** | T1 | `[M]` | `POST /v1/bundles/{name}/import` — bulk load from file upload. |
| 5.2.4 | **Import from CSV** | T1 | `[M]` | Same but CSV. We have `gigi-convert` CLI but no REST endpoint. |
| 5.2.5 | **Database snapshot** | T2 | `[L]` | Atomic snapshot of all bundles. For backup. |
| 5.2.6 | **Point-in-time recovery** | T3 | `[XL]` | Restore to a specific WAL position. We have WAL but no replay-to-timestamp. |

### 5.3 Configuration

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 5.3.1 | **Configurable port** | T1 | `[S]` | `--port 8080` or `GIGI_PORT=8080`. Currently hardcoded to 3142. |
| 5.3.2 | **Configurable data directory** | T1 | `[S]` | `--data-dir /var/gigi` for WAL and snapshots. |
| 5.3.3 | **Max request size** | T1 | `[S]` | Configurable body size limit. Currently hardcoded 256MB for NDJSON. |
| 5.3.4 | **Log level** | T1 | `[S]` | `--log-level debug\|info\|warn\|error`. |
| 5.3.5 | **Config file** | T2 | `[M]` | `gigi.toml` or `gigi.yaml` for all settings. |

---

## 6. Security

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 6.1 | **API key auth** | T1 | `[M]` | `Authorization: Bearer <key>`. Reject requests without valid key. Currently wide open. |
| 6.2 | **Per-bundle permissions** | T2 | `[L]` | Different API keys have different access. Key A → read-only on `prism_users`, Key B → full access. |
| 6.3 | **Rate limiting** | T1 | `[M]` | Limit requests per IP/key. Prevent abuse. |
| 6.4 | **HTTPS / TLS** | T1 | `[M]` | TLS termination. Currently plain HTTP. (Often handled by reverse proxy, but native support is expected.) |
| 6.5 | **Audit log** | T2 | `[M]` | Log who did what — all mutations with timestamp, client IP, API key. |
| 6.6 | **Field-level encryption** | T3 | `[L]` | Encrypt sensitive fields at rest. |
| 6.7 | **IP allowlist** | T2 | `[S]` | Only accept connections from specified IPs. |

---

## 7. Data Types & Values

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 7.1 | **Array / List type** | T2 | `[L]` | `Value::Array(Vec<Value>)` — first-class arrays. Currently arrays must be JSON-serialized into Text. Mongo, Postgres have native arrays. |
| 7.2 | **Object / Map type** | T2 | `[L]` | `Value::Object(HashMap<String, Value>)` — nested documents. MongoDB's core strength. |
| 7.3 | **Binary / Blob type** | T2 | `[M]` | `Value::Binary(Vec<u8>)` — raw bytes. For files, images, hashes. |
| 7.4 | **UUID type** | T2 | `[S]` | `Value::Uuid` — native UUID generation and comparison. |
| 7.5 | **Date type** | T2 | `[M]` | `Value::Date` — date without time. ISO date parsing and comparison. |
| 7.6 | **Decimal / BigDecimal** | T3 | `[M]` | Exact decimal arithmetic for financial data. Avoid floating-point errors. |
| 7.7 | **Enum type** | T3 | `[M]` | Schema-declared enum with allowed values. Postgres `CREATE TYPE`. |

---

## 8. Replication & Distribution

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 8.1 | **Read replicas** | T3 | `[XL]` | Secondary servers that replicate from primary. Read scaling. |
| 8.2 | **Leader election** | T3 | `[XL]` | Automatic failover when primary goes down. Raft/Paxos. |
| 8.3 | **Multi-region sync** | T3 | `[XL]` | GIGI Edge does some of this — but full CRDTs / conflict-free sync is T3. |
| 8.4 | **Sharding** | T3 | `[XL]` | Distribute bundles across multiple servers. Mongo sharding, Cassandra partitions. |
| 8.5 | **Change data capture (CDC)** | T2 | `[L]` | Stream of all mutations for external consumers. Debezium, Postgres logical replication. We have WebSocket SUBSCRIBE but it's not persistent or replayable. |

---

## 9. Developer Experience

| # | Feature | Tier | Diff | Description |
|---|---|---|---|---|
| 9.1 | **OpenAPI / Swagger spec** | T1 | `[M]` | Auto-generated API documentation. Try-it-out in browser. |
| 9.2 | **Python SDK** | T1 | `[M]` | `pip install gigi-client`. PRISM is Python. We have JS SDK but not Python. |
| 9.3 | **CLI client** | T2 | `[M]` | `gigi query prism_cases --where "status=open"` — interactive CLI. We have `gigi-convert` but no general-purpose CLI. |
| 9.4 | **Web UI / dashboard** | T2 | `[L]` | Browser-based bundle explorer. Browse records, run queries, see curvature. Like Mongo Compass, pgAdmin. |
| 9.5 | **Error codes** | T1 | `[S]` | Structured error codes (`BUNDLE_NOT_FOUND`, `RECORD_EXISTS`, `INVALID_QUERY`) not just string messages. |
| 9.6 | **Request ID / tracing** | T2 | `[S]` | Return `X-Request-Id` header for debugging. Correlate logs. |
| 9.7 | **Pagination links** | T2 | `[S]` | Return `next`/`prev` URLs in paginated responses. HATEOAS. |
| 9.8 | **Webhook notifications** | T2 | `[L]` | HTTP POST to a URL when records change. Alternative to WebSocket. |

---

## 10. GIGI+ — Geometry-Native Features No One Else Has

These leverage our fiber bundle architecture. Nobody else can do these.

| # | Feature | Diff | Description |
|---|---|---|---|
| 10.1 | **Anomaly detection via curvature** | `[M]` | Auto-flag records where local curvature exceeds threshold. `GET /v1/bundles/{name}/anomalies?k_threshold=0.5`. |
| 10.2 | **Similarity search** | `[L]` | "Find records similar to X" using metric space distance. Not just field matching — geometric neighborhood. |
| 10.3 | **Drift detection** | `[M]` | Track curvature over time. Alert when data distribution shifts. `GET /v1/bundles/{name}/drift`. |
| 10.4 | **Automatic schema suggestion** | `[L]` | Given raw JSON data, suggest optimal base vs fiber field split for minimal curvature. |
| 10.5 | **Bundle health score** | `[S]` | Single 0–100 score combining curvature, spectral gap, and consistency. `GET /v1/bundles/{name}/health`. |
| 10.6 | **Cross-bundle correlation** | `[L]` | Detect geometric relationships between bundles. "Cases and audit logs have correlated curvature spikes." |
| 10.7 | **Curvature-aware query optimizer** | `[XL]` | Use curvature to choose query strategy. Low K → sequential scan is fine. High K → use hash index. |
| 10.8 | **Geometric data compression** | `[L]` | Compress fiber values based on curvature — low-variance fibers store deltas from zero section. |
| 10.9 | **Predictive capacity** | `[M]` | "At current insertion rate and curvature trajectory, this bundle will need rebalancing in ~2 days." |
| 10.10 | **Fiber diffusion** | `[L]` | Propagate values across geometric neighbors. Fill missing values based on nearby records. Like a neural network but via Riemannian geometry. |

---

## Summary Count

| Category | T1 (table stakes) | T2 (common) | T3 (advanced) | GIGI+ |
|---|---|---|---|---|
| Query Operations | 14 | 12 | 1 | — |
| Write Operations | 8 | 9 | 3 | — |
| Schema & Structure | 7 | 7 | 4 | — |
| Transactions | — | 3 | — | — |
| Administration | 8 | 6 | 2 | — |
| Security | 3 | 3 | 1 | — |
| Data Types | — | 4 | 3 | — |
| Replication | — | 1 | 4 | — |
| Developer Experience | 3 | 5 | — | — |
| Geometry-Native | — | — | — | 10 |
| **Totals** | **43** | **50** | **18** | **10** |

**Grand total: 121 features we don't have yet.**

---

## Suggested Implementation Order

If we were building toward production-readiness, here's the order that maximizes value:

### Sprint 1 — "Stop Embarrassing Ourselves" (T1 essentials)
1. `[S]` UPSERT (2.1.1)
2. `[S]` COUNT without data (1.3.5)
3. `[S]` EXISTS check (1.3.6)
4. `[S]` Field projection (1.3.1)
5. `[S]` DISTINCT values (1.3.4)
6. `[S]` Total count in pagination (1.3.8)
7. `[S]` IN operator (1.1.1)
8. `[S]` IS NULL / IS NOT NULL (1.1.4)
9. `[S]` Bulk delete by filter (2.3.1)
10. `[S]` Truncate / delete all (2.3.2)
11. `[S]` Get bundle schema endpoint (3.1.5)
12. `[S]` Configurable port (5.3.1)
13. `[S]` Error codes (9.5)

### Sprint 2 — "Feels Like a Real Database" ✅
14. ~~`[M]` OR conditions (1.2.1)~~ ✅
15. ~~`[M]` Multi-field sort (1.3.3)~~ ✅
16. ~~`[M]` Auto-generated IDs (2.1.6)~~ ✅
17. ~~`[S]` Default timestamps (2.1.7)~~ ✅
18. ~~`[S]` Atomic increment/decrement (2.2.1)~~ ✅
19. ~~`[M]` Add field to existing bundle (3.1.1)~~ ✅
20. ~~`[M]` Create index on existing bundle (3.3.1)~~ ✅
21. ~~`[M]` Filtered aggregation (1.4.2)~~ ✅
22. ~~`[M]` Export to JSON (5.2.1)~~ ✅
23. ~~`[M]` Import from JSON (5.2.3)~~ ✅
24. `[M]` Python SDK (9.2) — deferred

### Sprint 3 — "Enterprise-Ready" ✅
25. ~~`[M]` API key auth (6.1)~~ ✅
26. ~~`[M]` Rate limiting (6.3)~~ ✅
27. ~~`[M]` RETURNING clause (2.1.5)~~ ✅
28. ~~`[M]` Optimistic concurrency / versioning (4.2)~~ ✅
29. ~~`[M]` EXPLAIN / query plan (5.1.2)~~ ✅
30. ~~`[M]` OpenAPI spec (9.1)~~ ✅
31. ~~`[M]` Bundle stats endpoint (5.1.1)~~ ✅
32. ~~`[L]` Multi-op transactions (4.1)~~ ✅

### Sprint 4 — "GIGI Is Different"
33. `[S]` Bundle health score (10.5)
34. `[M]` Anomaly detection (10.1)
35. `[M]` Drift detection (10.3)
36. `[M]` Predictive capacity (10.9)
37. `[L]` Similarity search (10.2)

---

*We have 170 tests passing today. Every sprint should end with zero regressions.*
