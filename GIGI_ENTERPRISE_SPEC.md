# GIGI Enterprise Capability Specification
## Version 1.0 — April 2026

---

## Purpose

This document captures the known enterprise capability gaps between GIGI and production-grade
incumbent databases. Each gap is a roadmap item. Priority is ordered by deal-block risk:
gaps marked **CRITICAL** will kill enterprise evaluations before they begin; gaps marked
**HIGH** will stall pilots; **MEDIUM** will cause friction post-adoption.

This is a living document. Each section includes a design sketch sufficient to build a
TDD spec from.

---

## Gap Summary

| Gap | Priority | Incumbent Best-in-Class | Blocks |
|-----|----------|------------------------|--------|
| Replication / HA | **CRITICAL** | Cassandra (leaderless), CockroachDB (Raft) | Any production deployment |
| SQL / JDBC compatibility | **CRITICAL** | PostgreSQL (wire protocol) | BI tools, dbt, Grafana, enterprise tooling |
| Auth / RBAC | **CRITICAL** | PostgreSQL (row-level security, roles) | Multi-tenant, regulated industries |
| Full-text search | HIGH | Elasticsearch (inverted index, BM25) | Entire log/search use-case excluded |
| ANN vector index | HIGH | pgvector (HNSW), Pinecone | RAG pipelines, embedding stores |
| GQL-queryable logs (`_gigi_*`) | HIGH | ClickHouse (system.query_log) | Observability Phase 2 — spec written |
| Runtime log/config API | MEDIUM | PostgreSQL (`ALTER SYSTEM`) | Ops teams expect zero-restart config |
| Export formats | MEDIUM | Parquet (DuckDB, Spark compat) | Data lake integration |
| Multi-tenancy / namespaces | MEDIUM | MongoDB (databases), PostgreSQL (schemas) | SaaS customers sharing one cluster |
| Query plan / EXPLAIN | MEDIUM | PostgreSQL (EXPLAIN ANALYZE) | DBA debugging, optimizer trust |
| Time-series partitioning | LOW | TimescaleDB (hypertable), Druid (segments) | High-cardinality time series at scale |
| ACID transactions (multi-bundle) | LOW | PostgreSQL, CockroachDB | Cross-bundle atomic writes |

---

## 1. Replication / High Availability

**Priority: CRITICAL**

GIGI is currently single-node with no failover. Any ops team evaluating it for production
will reject it at the architecture review — not because of correctness concerns but because
of the operational risk of a single point of failure.

### What the best do

- **Cassandra** — leaderless eventual consistency; every node is equal; replication factor N
  means N-1 nodes can fail with zero downtime. Tunable consistency (ONE / QUORUM / ALL).
- **CockroachDB** — Raft consensus groups (ranges) replicated across nodes. Linearizable
  reads. Automatic rebalancing.
- **Elasticsearch** — primary/replica shards. Automatic shard rebalancing across nodes.
  Index-level replication factor.

### GIGI design sketch

GIGI's fiber bundle architecture has a natural replication primitive: **sheaf gluing**.
Two nodes holding overlapping sections of the same bundle must agree on overlapping
open sets — this is already how GIGI Edge is designed to sync. The same axiom applied
server-side gives replication with geometric correctness guarantees that Cassandra's
"eventual" model cannot offer.

**Proposed architecture:**
1. **Leader/follower WAL streaming.** Leader node streams WAL entries to follower(s).
   Follower replays in order. Reads from follower are geometrically stale-aware — the
   follower can report its replication lag as a curvature coherence delta between its
   bundle state and the acknowledged leader state.
2. **Sheaf-consistent failover.** On leader failure, a follower promotes only after
   confirming its bundle state is sheaf-consistent (holonomy = 0, coherence = 1.0).
   This gives a stronger promotion guarantee than timestamp-based quorum.
3. **Read replicas first.** Simplest viable increment: WAL streaming to read-only
   followers. Write goes to leader. Read can go to any node that acknowledges its lag.

**Minimum viable spec:**
- `fly.toml` `[processes]` — leader vs follower process tags
- `POST /v1/admin/replicate` — enroll a follower with leader URL
- `GET /v1/replication/status` — lag_records, lag_bytes, coherence_delta, is_promoted
- WAL entries already include enough info for replay — no schema change needed

**Competitive narrative:** "GIGI replication is sheaf-guaranteed consistent. A follower
that promotes knows mathematically that its data is complete — not just that it caught
up on timestamps."

---

## 2. SQL / JDBC Compatibility

**Priority: CRITICAL**

Grafana, dbt, Metabase, Tableau, Apache Superset, and every enterprise BI tool speaks
SQL over JDBC or PostgreSQL wire protocol. Without this, GIGI cannot be plugged into
any existing data stack. This is a deal-blocker for every enterprise that already has a
BI layer — which is all of them.

### What the best do

- **PostgreSQL wire protocol** is the de facto standard. DuckDB, CockroachDB, YugabyteDB,
  and Redshift all speak it. Any tool that works with Postgres works with them.
- **DuckDB** added a PostgreSQL-compatible server mode, turning it instantly compatible
  with the entire ecosystem.

### GIGI design sketch

Full PostgreSQL wire protocol implementation is months of work. The right first move is
a **translation layer** — a lightweight server that accepts PostgreSQL simple query
protocol and translates to GIGI GQL.

**Proposed architecture:**
1. **`gigi-sql` binary** (separate from `gigi-stream`). Listens on port 5432.
   Speaks PostgreSQL simple query protocol (text mode only — no prepared statements yet).
2. **SQL → GQL transpiler.** For the subset of SQL that maps cleanly:
   - `SELECT col FROM bundle WHERE cond` → `COVER bundle ON cond` + field projection
   - `SELECT COUNT(*) FROM bundle` → `SELECT COUNT(*) FROM bundle`
   - `SELECT * FROM bundle LIMIT n` → `SELECT * FROM bundle LIMIT n`
   - `SHOW TABLES` → `SHOW BUNDLES`
   - `DESCRIBE table` → bundle schema lookup
3. **Information schema.** Grafana requires `information_schema.tables` and
   `information_schema.columns`. These become GQL queries against the bundle registry.
4. **Read-only first.** No INSERT/UPDATE/DELETE via SQL — use the REST/GQL API for writes.
   SQL compat is for BI reads only.

**Minimum viable:** Grafana can connect, run a dashboard, and display bundle data.
That demo alone changes the enterprise conversation.

**Competitive narrative:** "Connect Grafana to GIGI in 30 seconds. Existing dashboards,
no migration."

---

## 3. Auth / RBAC

**Priority: CRITICAL**

GIGI currently has a single `GIGI_API_KEY` environment variable. There is no concept of
users, roles, row-level security, or tenant isolation. This excludes GIGI from:
- Any multi-tenant SaaS deployment
- Regulated industries (HIPAA, SOC 2, PCI DSS)
- Enterprise environments with IAM policies

### What the best do

- **PostgreSQL** — full role hierarchy, `GRANT`/`REVOKE`, row-level security policies,
  `pg_hba.conf` host-based auth, TLS cert auth.
- **MongoDB** — database-level users, collection-level roles, LDAP integration
  (Enterprise), field-level encryption.
- **Elasticsearch** — API keys with index-level privilege sets, RBAC, document-level
  security (commercial).

### GIGI design sketch

**Layer 1 — API keys with scopes (minimum viable):**
```json
{
  "key_id": "prod-key-7f3a",
  "scopes": ["read:*", "write:sensor_*", "admin:false"],
  "rate_limit_rps": 1000,
  "expires_at": null
}
```

Key management via `POST /v1/admin/keys` (requires master key).
Every request validated against key scopes before any bundle access.

**Layer 2 — Bundle-level permissions:**
```json
{
  "key_id": "tenant-a-key",
  "allow_bundles": ["tenant_a_*"],
  "deny_bundles": ["_gigi_audit_log", "_gigi_system_log"]
}
```

Bundle name prefixes as tenant namespaces. `tenant_a_orders`, `tenant_a_users` are
isolated from `tenant_b_*` by key scope. No data mixing possible.

**Layer 3 — Row-level security (future):**
Bundle-level security predicates attached to keys. Every query automatically ANDs in
the predicate. `key.predicate = "tenant_id = 'tenant_a'"` → every GQL query on that
key appends `AND tenant_id = 'tenant_a'` before execution.

**Competitive narrative:** "GIGI's multi-tenancy is enforced at the geometric level —
bundle scopes are declared in the fiber header. A tenant key cannot access another
tenant's bundles; it cannot even see their names."

---

## 4. Full-Text Search

**Priority: HIGH**

GIGI has no inverted index. Text fields are treated as categorical (equality comparison
only). This excludes the entire document search, log search, and NLP workload —
Elasticsearch's entire market.

### What the best do

- **Elasticsearch** — BM25 + TF-IDF, field boosting, fuzzy match, phrase match,
  highlighting, multi-match, nested objects, completion suggester.
- **PostgreSQL** — `tsvector`/`tsquery`, GIN inverted index, `@@` match operator,
  `ts_rank` scoring. Reasonable for moderate volume.
- **Typesense / Meilisearch** — pure FTS databases, typo-tolerance, faceting.

### GIGI design sketch

GIGI should not try to compete with Elasticsearch on full-text. The play is **geometric
full-text**: represent documents as fiber bundles where the fiber is the TF-IDF or
embedding vector, and use GIGI's existing geometric query machinery.

**Short-term (pragmatic):**
- Add `TEXT_SEARCH` field type that builds a trigram index alongside the categorical index
- New GQL clause: `MATCH bundle ON field LIKE "search terms"` → trigram-accelerated scan
- Ranking by curvature proximity to query embedding (requires embedding field)

**Long-term (geometric):**
- `EMBED bundle ON field USING model` — stores sentence embeddings as fiber dimensions
- `SEARCH bundle FOR "natural language query"` → GIGI computes query embedding,
  runs geometric nearest-neighbor in fiber space
- This collapses FTS and vector search into one operation (see Gap 5)

**Competitive narrative:** "GIGI doesn't rank by TF-IDF. It ranks by geometric distance
on the fiber — documents that are structurally similar, not just lexically similar."

---

## 5. ANN Vector Index (HNSW)

**Priority: HIGH**

GIGI's current `VECTOR SEARCH` endpoint appears to be exact (O(n) scan). For embedding
stores with millions of vectors this is unusable. Every RAG pipeline needs sub-10ms
approximate nearest-neighbor search at scale.

### What the best do

- **pgvector** — HNSW and IVFFlat indexes. HNSW gives O(log n) approximate search,
  tunable recall/speed tradeoff. Now the default for Postgres-based RAG.
- **Pinecone, Weaviate, Qdrant** — purpose-built ANN databases. HNSW or HNSW variants.
  Billion-scale with hardware acceleration.
- **Elasticsearch** — dense vector fields with HNSW index. `knn` query clause.

### GIGI design sketch

GIGI's fiber bundle structure is naturally suited to HNSW because the fiber space IS the
vector space — no separate index structure needed. The fiber dimensions define the metric.

**Proposed implementation:**
- New field type: `vector(n)` — fixed-dimension float array stored as n fiber dimensions
- On `CREATE BUNDLE` with vector field: build HNSW graph over fiber section values
- New GQL: `NEAR bundle ON vector_field TO [0.1, 0.2, ..., 0.9] LIMIT 10`
- HNSW parameters configurable at bundle creation: `M` (connections), `ef_construction`
- Returns: `{ records: [...], distances: [...], confidence: [...] }`

**Geometric bonus:** GIGI can report curvature of the ANN result set — are the returned
neighbors clustered (low K) or scattered (high K)? No other ANN database does this.

**Competitive narrative:** "pgvector gives you HNSW. GIGI gives you HNSW + curvature of
the result set. When K is high, your results are scattered — the query is ambiguous.
When K is low, the results form a tight geometric cluster — high confidence retrieval."

---

## 6. GQL-Queryable System Logs (`_gigi_*` bundles)

**Priority: HIGH — Phase 2 of GIGI_OBSERVABILITY_SPEC.md**

This is the only gap with a complete spec already written. See `GIGI_OBSERVABILITY_SPEC.md §2–4`.

**Status:** Phase 1 shipped (stdout JSON + `/v1/metrics`). Phase 2 not yet started.

**What ships in Phase 2:**
- `LogIngester` writes every log event into an internal GIGI bundle (`_gigi_query_log`, etc.)
- Bundles are WAL-backed, survive restarts, queryable via GQL
- `SHOW BUNDLES` returns `_gigi_*` bundles with `internal: true` flag
- `SELECT * FROM _gigi_query_log WHERE duration_us > 500000` just works
- `DIVERGENCE FROM _gigi_query_log TO _gigi_query_log_yesterday` works (time windows)

**Competitive narrative:** "ClickHouse has `system.query_log`. GIGI has `_gigi_query_log`
— and you can run `CURVATURE _gigi_query_log` to detect anomalies in your own query
patterns. ClickHouse cannot analyze its own logs geometrically."

**Implementation notes:** Phase 2 spec is in `GIGI_OBSERVABILITY_SPEC.md §8, Phase 2`.

---

## 7. Runtime Log / Config API

**Priority: MEDIUM**

GIGI requires a process restart to change any configuration. PostgreSQL's `ALTER SYSTEM`,
ClickHouse's HTTP config reload, and MongoDB's `setParameter` all allow zero-restart
config changes. Ops teams expect this.

**Minimum viable:**
- `POST /v1/admin/log-level` — change log level without restart (already in observability spec)
- `POST /v1/admin/log-config` — full log config as per `GIGI_OBSERVABILITY_SPEC.md §6`
- `GET /v1/admin/config` — read current runtime config snapshot
- Config stored in a `_gigi_config` system bundle so it survives restart
- Hot-reload pattern: `Arc<RwLock<Config>>` shared between request handlers and admin endpoint

**What this unlocks:** Ops teams can tune `slow_query_threshold_us`, enable debug logging
for a single category, or disable connection logging in production — all without a deploy.

---

## 8. Export Formats — Parquet / Arrow

**Priority: MEDIUM**

GIGI exports DHOOM (proprietary) and JSON. Neither integrates with the data lake
ecosystem. Parquet is the universal interchange format for Spark, Databricks, DuckDB,
BigQuery, Snowflake, and every analytics warehouse.

**Minimum viable:**
- `GET /v1/bundles/{name}/export?format=parquet` — export bundle as Apache Parquet file
- Use the `arrow2` or `parquet` Rust crates (well-maintained, no JVM dependency)
- Schema mapping: GIGI fiber fields → Parquet column types (straightforward for numeric/text/timestamp)
- Streaming: chunked HTTP response, one row group per WAL segment

**What this unlocks:** "GIGI as a hot tier" — ingest and serve real-time queries from
GIGI, export to Parquet nightly for historical analytics in BigQuery/Snowflake. This is
a deployment pattern that enterprise data teams understand immediately.

---

## 9. Multi-Tenancy / Namespace Isolation

**Priority: MEDIUM**

GIGI has a flat bundle namespace. All bundles are in one list. Multi-tenant deployments
must use naming conventions (`tenant_a_orders`) and rely on API key scopes (see Gap 3).
PostgreSQL has databases and schemas. MongoDB has databases and collections.

**Minimum viable:**
- Bundle name prefix as namespace: `{namespace}/{bundle_name}` e.g. `acme/orders`
- `SHOW BUNDLES IN acme` GQL clause
- `CREATE BUNDLE acme/orders (...)` syntax
- API key scoped to `acme/*` cannot see or query `globex/*`
- `GET /v1/namespaces` — list all namespaces the calling key can see

**What this unlocks:** Selling GIGI Stream as a hosted service. Each customer gets a
namespace; isolation is enforced by the engine, not the application layer.

---

## 10. EXPLAIN / Query Plan

**Priority: MEDIUM**

Every DBA who evaluates GIGI will ask "how do I see what the query is doing?" There is
no `EXPLAIN` equivalent. This makes it impossible to debug slow queries without reading
source code.

### GIGI design sketch

GQL `EXPLAIN` prefix:
```
EXPLAIN DIVERGENCE FROM sensor_das TO sensor_sonar
```

Response:
```json
{
  "plan": [
    { "step": "stats_cache_check", "bundle": "sensor_das", "hit": true, "us": 1 },
    { "step": "stats_cache_check", "bundle": "sensor_sonar", "hit": true, "us": 1 },
    { "step": "kl_compute", "fields": ["temp", "pressure"], "records": 50000, "us": 188 },
    { "step": "js_compute", "us": 31 }
  ],
  "total_us": 221,
  "geometric": { "kl_forward": 0.39, "jensen_shannon": 0.06 }
}
```

This is minimal — each query handler returns a `Vec<PlanStep>` in debug mode.
Not as rich as PostgreSQL's EXPLAIN ANALYZE but enough for DBA trust.

---

## 11. Time-Series Partitioning

**Priority: LOW**

High-cardinality time series (IoT, telemetry, financial tick data) at billions of rows
requires automatic time-based partitioning. TimescaleDB's hypertables, Druid's segment
granularity, and InfluxDB's shards all address this. GIGI stores everything in one
mmap bundle with no partitioning.

**Design sketch:** Bundle partitioning by timestamp field:
```
CREATE BUNDLE sensor_data (
  ts timestamp, temp numeric, pressure numeric
)
PARTITION BY ts INTERVAL '1 day'
```

Behind the scenes: `sensor_data_20260418`, `sensor_data_20260419`, etc. Query planner
prunes partitions by timestamp predicate. Old partitions get compacted and archived.
`SHOW BUNDLES LIKE 'sensor_data_%'` returns all partitions.

**When to tackle:** After replication and SQL compat — those unlock the deployment
patterns that make time-series volume a real problem.

---

## 12. ACID Transactions (Multi-Bundle)

**Priority: LOW**

GIGI has `ATLAS BEGIN/COMMIT/ROLLBACK` for single-bundle atomic operations (already
implemented, v2.1 unimplemented per the parser). True cross-bundle ACID transactions
— insert into `orders` and `inventory` atomically — require distributed coordination.

**Current state:** Single-bundle atomicity is guaranteed by the WAL. Cross-bundle is not.

**Design sketch:** Two-phase commit protocol over the WAL:
- `ATLAS BEGIN MULTI (orders, inventory)` — locks both bundles
- INSERT INTO orders / INSERT INTO inventory
- `ATLAS COMMIT MULTI` — WAL fence across both bundles, then unlock
- `ATLAS ROLLBACK MULTI` — WAL revert on both

This is significant work and a low-priority enterprise gap — most workloads can be
designed to avoid cross-bundle atomicity. Tackle after replication.

---

## Priority Order for Implementation

1. **Phase 2 Observability** — already specced, already started, closes the ClickHouse gap
2. **Auth / API key scopes** — minimum viable RBAC, unlocks multi-tenant demos
3. **Runtime log/config API** — small lift, high ops credibility
4. **EXPLAIN** — small lift, huge DBA trust signal
5. **SQL / JDBC (read-only)** — Grafana demo, changes the enterprise conversation
6. **Parquet export** — data lake integration story
7. **Multi-tenancy namespaces** — SaaS deployment pattern
8. **HNSW vector index** — RAG pipeline story
9. **Full-text / MATCH** — trigram first, geometric later
10. **Replication / HA** — fundamental but architectural; scope carefully
11. **Time-series partitioning** — after scale is a real problem
12. **Multi-bundle ACID** — after replication

---

*Spec status: LIVE — update as items ship*
*Last updated: April 2026*
