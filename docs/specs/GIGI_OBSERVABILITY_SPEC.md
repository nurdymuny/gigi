# GIGI Observability & Logging Specification
## Version 1.1 — April 2026

---

## Executive Summary

GIGI currently logs nothing structured. Every competitor ships with some form of logging but all of them have critical gaps. This spec defines a logging system that covers everything they do and adds a layer they cannot offer: **geometric observability** — curvature, divergence, and topological health embedded directly into every log event.

### Competitor Gap Analysis

| System | Format | Slow Query | Audit | Query-Level Math | Bundle/Segment Health | Stream Lifecycle |
|--------|--------|-----------|-------|------------------|-----------------------|-----------------|
| PostgreSQL | Text/JSON | ✅ (sampled) | Extension | ❌ | ❌ | ❌ |
| ClickHouse | SQL tables | ✅ | `log_queries` | ❌ | ❌ | ❌ |
| MongoDB | JSON | ✅ (profiler) | Enterprise only | ❌ | ❌ | ❌ |
| Elasticsearch | Text/JSON | ✅ (dual) | Commercial | ❌ | Shard health only | ❌ |
| Cassandra | Text only | ❌ none | Enterprise only | ❌ | Thread pool only | ❌ |
| Druid | Text + JMX | ❌ none | Via logs | ❌ | Segment load only | ❌ |
| **GIGI** | **DHOOM** (JSON compat) | **✅** | **✅ native** | **✅ κ, KL, JS per query** | **✅ full** | **✅ full** |

ClickHouse has the best general observability (SQL-queryable system tables, ProfileEvents map with hundreds of counters). PostgreSQL has the best query plan logging. **GIGI's unique advantage: geometric metrics are first-class log fields, not afterthoughts.**

---

## 1. Logging Architecture

### 1.1 Design Principles

1. **DHOOM is the primary log format.** Every log event is stored and transmitted as DHOOM — the same compact, fiber-bundle-schema format GIGI uses for all its data. JSON is a supported compatibility output, not the default. Every competitor chose JSON or text. We chose our own format because it's better: schema declared once per bundle (not repeated on every line), timestamps delta-encoded (microsecond offsets from bundle epoch, not ISO strings), repeated strings interned (bundle names, event types, client IPs), and the whole log file is itself a DHOOM bundle — readable by `gigi-convert`, analysable by GQL, compressible by the same mmap compactor used for all data.
2. **System bundles — logs as data.** Every log event is ingested into an internal GIGI bundle (`_gigi_query_log`, `_gigi_event_log`, etc.) and queryable via GQL. This is more than ClickHouse's system tables: the bundles are WAL-backed, survive restarts, and support the full GQL query language including DIVERGENCE, RICCI, and curvature analysis. You can run `DIVERGENCE FROM _gigi_query_log TO _gigi_query_log_yesterday` to detect query pattern drift using the same geometric engine as everything else.
3. **Zero performance budget.** Logging must never be on the hot path. All log writes are async; the query returns before the log event is flushed.
4. **Severity levels:** `TRACE | DEBUG | INFO | WARN | ERROR | FATAL` — configurable at runtime via `POST /v1/admin/log-level`.
5. **Geometric fields are not optional.** κ (curvature), KL divergence, JS divergence, coherence — these appear on every query log event where they were computed. If a query didn't compute them, the fields are `null`. Not omitted — `null`, so dashboards can count "queries without geometric analysis."

### 1.2 Log Event Base Schema

#### Primary format: DHOOM

Each log category is a DHOOM bundle. The fiber header is written once at bundle creation. Records are rows — compact, delta-encoded, schema-free after the header.

Example: `_gigi_query_log` fiber header:

```
_gigi_query_log [
  ts          ^ @2026-04-18T00:00:00Z       // delta-encoded from bundle epoch (us offsets)
  level       | INFO                         // modal default = INFO; only written when != INFO
  category    | query                        // modal default = query
  event       &[query.complete, query.start, query.error, query.slow]  // interned
  instance    | gigi-stream-fly-01           // modal default = this node
  version     | 0.5.2
  request_id
  duration_us
  statement_type &[SELECT, INSERT, DIVERGENCE, RICCI, CURVATURE, DELETE, GQL]
  bundles_accessed
  records_scanned
  records_returned
  bytes_read
  bytes_returned
  cache_hit
  kl_forward
  kl_reverse
  jensen_shannon
  fields_compared
  ricci
  k_global
  coherence
  slow
  error
]
```

A record in this bundle looks like:

```
14:22:01.493291  query.complete  7f3a2c1d-a4b2-4e1f-9c3d-0e2f5a6b7c8d  221043
  DIVERGENCE  [sensor_das,sensor_sonar]  0  1  0  412  true  0.3944  0.3762  0.0647  1  -  -  -  false  -
```

Compare to the equivalent JSON — schema repeated on every line, timestamps as 24-char strings, event type as a full string every row. DHOOM encodes the same event in roughly **40% of the bytes**, with the schema declared once and timestamps as microsecond deltas.

#### Compatibility output: JSON

For operators, log shippers, and external tools, every log event can also be emitted as newline-delimited JSON (configured via `output.format: "json"` — see §6). The JSON schema mirrors the DHOOM fiber fields exactly:

```json
{
  "ts":             "2026-04-18T14:22:01.493291Z",
  "level":          "INFO",
  "category":       "query",
  "event":          "query.complete",
  "instance":       "gigi-stream-fly-01",
  "version":        "0.5.2",
  "request_id":     "7f3a2c1d-...",
  "duration_us":    221043,
  "payload":        { ... }
}
```

`duration_us` is always wall time from first byte received to last byte sent. Never omit it.

**Default output:** DHOOM to internal bundles + JSON to stdout (stdout JSON for log shippers; DHOOM for everything stored and queried).

---

## 2. Log Categories

| Category | Internal Bundle | Description |
|----------|----------------|-------------|
| `query` | `_gigi_query_log` | All GQL and REST query executions |
| `ingest` | `_gigi_ingest_log` | All writes: insert, bulk, CSV import |
| `wal` | `_gigi_wal_log` | WAL append, replay, checkpoint, compaction |
| `connection` | `_gigi_conn_log` | HTTP and WebSocket connection lifecycle |
| `stream` | `_gigi_stream_log` | WebSocket push events, subscriber lifecycle |
| `bundle` | `_gigi_bundle_log` | Bundle create, drop, schema change, stats cache warm |
| `anomaly` | `_gigi_anomaly_log` | All anomaly detection events (is_anomaly=true) |
| `audit` | `_gigi_audit_log` | Auth, admin actions, config changes |
| `system` | `_gigi_system_log` | Startup, shutdown, GC/memory pressure, errors |
| `slow` | `_gigi_slow_log` | Queries exceeding the slow query threshold (mirror of query_log) |

The internal bundles are regular GIGI bundles — WAL-backed, queryable via GQL, subject to configured retention TTL. No special code paths. `SELECT COUNT(*) FROM _gigi_query_log WHERE duration_us > 500000` just works.

---

## 3. Event Catalog

### 3.1 Query Events (`category: "query"`)

#### `query.start`
Emitted when a query is accepted and begins parsing. Useful for detecting queries that started but never completed (timeout, OOM, crash).

```json
{
  "event": "query.start",
  "request_id": "7f3a2c1d-a4b2-4e1f-9c3d-0e2f5a6b7c8d",
  "source": "gql",
  "statement_type": "DIVERGENCE",
  "raw_gql": "DIVERGENCE FROM sensor_das TO sensor_sonar",
  "client_ip": "10.0.1.42",
  "user_agent": "gigi-py/1.0.2"
}
```

#### `query.complete` ⭐ Primary query log event

```json
{
  "event": "query.complete",
  "request_id": "7f3a2c1d-...",
  "source": "gql",
  "statement_type": "DIVERGENCE",
  "raw_gql": "DIVERGENCE FROM sensor_das TO sensor_sonar",
  "duration_us": 221043,
  "parse_us": 89,
  "exec_us": 220954,
  "bundles_accessed": ["sensor_das", "sensor_sonar"],
  "records_scanned": 0,
  "records_returned": 1,
  "bytes_read": 0,
  "bytes_returned": 412,
  "cache_hit": true,
  "stats_cache_warm": true,

  "geometric": {
    "kl_forward": 0.3944,
    "kl_reverse": 0.3762,
    "jensen_shannon": 0.0647,
    "fields_compared": 1,
    "ricci": null,
    "k_global": null,
    "coherence": null
  },

  "slow": false,
  "error": null
}
```

**The `geometric` block is GIGI's exclusive differentiator.** No other database logs the mathematical output of its query as structured fields. This enables dashboards like "average KL divergence of all DIVERGENCE queries this week" or alerting on "any query where js > 0.5 against the model bundle."

#### `query.slow`
Emitted in addition to `query.complete` when `duration_us > slow_query_threshold_us` (default: 1,000,000μs = 1s). Mirrors the full `query.complete` payload. Goes to both `_gigi_query_log` and `_gigi_slow_log`.

```json
{
  "event": "query.slow",
  "slow_threshold_us": 1000000,
  "duration_us": 10243011,
  "statement_type": "DIVERGENCE",
  "raw_gql": "DIVERGENCE FROM chembl_activities TO chembl_assays",
  "cache_warm_before": false,
  "cache_warm_after": true,
  "note": "First access — stats cache cold. Subsequent queries will be O(1)."
}
```

Note: GIGI's slow log is self-annotating. It explains *why* the query was slow (cold cache, large scan, parse failure, etc.) rather than just reporting the duration.

#### `query.error`

```json
{
  "event": "query.error",
  "request_id": "...",
  "raw_gql": "DIVERGENCE FROM nonexistent TO sensor_das",
  "duration_us": 312,
  "error_class": "BundleNotFound",
  "error_msg": "Bundle 'nonexistent' does not exist",
  "http_status": 404
}
```

---

### 3.2 Ingest Events (`category: "ingest"`)

#### `ingest.complete`

```json
{
  "event": "ingest.complete",
  "bundle": "sensor_das",
  "records_written": 500,
  "bytes_written": 24680,
  "duration_us": 3841,
  "wal_synced": true,
  "schema_changed": false,
  "fields_added": [],

  "geometric": {
    "k_before": 0.018,
    "k_after": 0.021,
    "k_delta": 0.003,
    "anomaly_triggered": false
  }
}
```

The `k_delta` field shows how much the ingest shifted the geometric structure of the bundle. A sudden large `k_delta` on ingest (before any query) is an early warning signal — the data distribution just changed.

#### `ingest.bulk`

```json
{
  "event": "ingest.bulk",
  "bundle": "sensor_das",
  "records_written": 50000,
  "bytes_written": 2460800,
  "duration_us": 284932,
  "throughput_rps": 175504,
  "wal_synced": true,
  "batches": 100
}
```

---

### 3.3 WAL Events (`category: "wal"`)

#### `wal.append`

```json
{
  "event": "wal.append",
  "bundle": "sensor_das",
  "record_id": "rec_a3f2c1",
  "wal_offset": 204831,
  "duration_us": 42
}
```

#### `wal.checkpoint`

```json
{
  "event": "wal.checkpoint",
  "bundle": "sensor_das",
  "records_flushed": 50000,
  "bytes_flushed": 2460800,
  "wal_size_before": 14400000,
  "wal_size_after": 0,
  "duration_us": 184022
}
```

#### `wal.replay`

```json
{
  "event": "wal.replay",
  "bundle": "sensor_das",
  "records_recovered": 1247,
  "duration_us": 12843,
  "triggered_by": "startup"
}
```

#### `wal.compaction`

```json
{
  "event": "wal.compaction",
  "bundle": "sensor_das",
  "segments_merged": 12,
  "size_before": 48600000,
  "size_after": 14200000,
  "compression_ratio": 3.42,
  "duration_us": 2841033
}
```

---

### 3.4 Connection Events (`category: "connection"`)

#### `connection.open` / `connection.close`

```json
{
  "event": "connection.open",
  "protocol": "http",
  "client_ip": "10.0.1.42",
  "user_agent": "curl/8.1.2",
  "tls": true
}
```

```json
{
  "event": "connection.close",
  "protocol": "http",
  "client_ip": "10.0.1.42",
  "session_duration_us": 441209,
  "requests_served": 3,
  "bytes_sent": 8841,
  "bytes_received": 312
}
```

---

### 3.5 WebSocket Stream Events (`category: "stream"`)

No competitor logs WebSocket streams at the semantic level. They log TCP connections. GIGI logs *what the stream is doing*.

#### `stream.subscribe`

```json
{
  "event": "stream.subscribe",
  "connection_id": "ws_f3a2c1d4",
  "bundle": "sensor_das",
  "client_ip": "10.0.1.42",
  "mode": "dashboard"
}
```

#### `stream.push`

```json
{
  "event": "stream.push",
  "connection_id": "ws_f3a2c1d4",
  "bundle": "sensor_das",
  "message_seq": 142,
  "duration_us": 884,
  "bytes_sent": 312,
  "k_global": 0.021,
  "is_anomaly": false,
  "z_score": 0.41
}
```

#### `stream.anomaly_push`

```json
{
  "event": "stream.anomaly_push",
  "connection_id": "ws_f3a2c1d4",
  "bundle": "sensor_das",
  "message_seq": 198,
  "k_global": 0.089,
  "k_threshold_3s": 0.045,
  "z_score": 3.72,
  "is_anomaly": true,
  "contributing_fields": ["pressure", "temp"],
  "duration_us": 1022
}
```

#### `stream.disconnect`

```json
{
  "event": "stream.disconnect",
  "connection_id": "ws_f3a2c1d4",
  "bundle": "sensor_das",
  "session_duration_us": 1802441,
  "messages_sent": 201,
  "anomalies_sent": 3,
  "reason": "client_close"
}
```

---

### 3.6 Bundle Events (`category: "bundle"`)

#### `bundle.create`

```json
{
  "event": "bundle.create",
  "bundle": "sensor_das",
  "schema_fields": ["temp", "pressure", "humidity", "timestamp_us"],
  "storage_type": "mmap",
  "source": "api"
}
```

#### `bundle.stats_cache_warm`

GIGI-exclusive. Logs when the lazy base_stats cache warms for a bundle — explains the one-time slow query.

```json
{
  "event": "bundle.stats_cache_warm",
  "bundle": "chembl_activities",
  "records_scanned": 4900000,
  "fields_cached": 4,
  "duration_us": 9841022,
  "triggered_by": "DIVERGENCE FROM chembl_activities TO chembl_assays"
}
```

#### `bundle.schema_change`

```json
{
  "event": "bundle.schema_change",
  "bundle": "sensor_das",
  "fields_added": ["co2_ppm"],
  "fields_removed": [],
  "triggered_by": "ingest"
}
```

#### `bundle.drop`

```json
{
  "event": "bundle.drop",
  "bundle": "test_bundle",
  "records_deleted": 1247,
  "bytes_freed": 62350,
  "triggered_by": "api",
  "client_ip": "10.0.1.42"
}
```

---

### 3.7 Anomaly Events (`category: "anomaly"`)

Every anomaly that GIGI detects — whether triggered by a query, a stream push, or a background health check — gets its own log entry.

```json
{
  "event": "anomaly.detected",
  "bundle": "sensor_das",
  "record_id": "rec_a3f2c1",
  "k_record": 0.089,
  "k_mean": 0.021,
  "k_std": 0.008,
  "z_score": 8.5,
  "threshold_2s": 0.037,
  "threshold_3s": 0.045,
  "sigma_level": 3,
  "contributing_fields": ["pressure"],
  "detection_source": "stream",
  "duration_us": 441
}
```

This makes `_gigi_anomaly_log` a dedicted anomaly audit trail. You can query:
```sql
SELECT bundle, COUNT(*) as anomalies, AVG(z_score) as avg_z
FROM _gigi_anomaly_log
WHERE ts > NOW() - INTERVAL '1 hour'
GROUP BY bundle
ORDER BY anomalies DESC
```

---

### 3.8 Audit Events (`category: "audit"`)

All administrative actions are audit-logged. Unlike MongoDB (Enterprise-only) or Elasticsearch (Commercial), GIGI audit logging is **always on, always free**.

```json
{
  "event": "audit.bundle_drop",
  "actor": "api_key:prod-key-7f3a",
  "client_ip": "10.0.1.42",
  "bundle": "test_bundle",
  "outcome": "success"
}
```

```json
{
  "event": "audit.log_level_change",
  "actor": "api_key:prod-key-7f3a",
  "old_level": "INFO",
  "new_level": "DEBUG",
  "outcome": "success"
}
```

```json
{
  "event": "audit.config_change",
  "actor": "api_key:prod-key-7f3a",
  "field": "slow_query_threshold_us",
  "old_value": 1000000,
  "new_value": 500000
}
```

---

### 3.9 System Events (`category: "system"`)

#### `system.startup`

```json
{
  "event": "system.startup",
  "version": "0.5.2",
  "data_path": "/data/gigi",
  "bundles_loaded": 14,
  "wal_replayed": 3,
  "records_recovered": 1247,
  "duration_us": 2841022,
  "listen_addr": "0.0.0.0:3142"
}
```

#### `system.shutdown`

```json
{
  "event": "system.shutdown",
  "uptime_us": 864000000000,
  "queries_served": 14822,
  "records_ingested": 2841022,
  "anomalies_detected": 47,
  "wal_checkpoints": 12,
  "reason": "SIGTERM"
}
```

#### `system.memory_pressure`

```json
{
  "event": "system.memory_pressure",
  "heap_used_mb": 3841,
  "heap_limit_mb": 4096,
  "pressure_pct": 93.8,
  "action": "evict_stats_cache",
  "bundles_evicted": ["chembl_activities"]
}
```

---

## 4. System Query Bundles (GQL-Queryable Logs)

The best idea ClickHouse ever had: make logs queryable as tables. GIGI does this natively because every log category is a regular bundle.

All `_gigi_*` bundles are:
- **Read-only via GQL** (no INSERT allowed by users)
- **Configurable TTL** — default 30 days, configurable per bundle
- **WAL-backed** — survive restarts
- **Queryable with all GQL features** including curvature, divergence, GROUP BY, FILTER

### Example Queries Against Log Bundles

**Top 10 slowest queries in the last hour:**
```sql
SELECT statement_type, raw_gql, duration_us
FROM _gigi_query_log
WHERE ts > NOW() - INTERVAL '1 hour'
ORDER BY duration_us DESC
LIMIT 10
```

**Query pattern drift — are this week's queries geometrically different from last week's?**
```sql
DIVERGENCE FROM _gigi_query_log_this_week TO _gigi_query_log_last_week
```
*(This uses time-windowed bundle views — see §5.)*

**Anomaly rate per bundle over 24h:**
```sql
SELECT bundle, COUNT(*) as count, AVG(z_score) as avg_z, MAX(z_score) as max_z
FROM _gigi_anomaly_log
WHERE ts > NOW() - INTERVAL '24 hours'
GROUP BY bundle
ORDER BY count DESC
```

**WAL health — checkpoint frequency and size reduction:**
```sql
SELECT ts, bundle, records_flushed, compression_ratio
FROM _gigi_wal_log
WHERE event = 'wal.checkpoint'
ORDER BY ts DESC
LIMIT 20
```

**WebSocket stream uptime — sessions longer than 10 minutes:**
```sql
SELECT connection_id, bundle, session_duration_us / 1e6 as duration_sec, messages_sent, anomalies_sent
FROM _gigi_stream_log
WHERE event = 'stream.disconnect'
AND session_duration_us > 600000000
ORDER BY duration_sec DESC
```

---

## 5. Metrics Endpoint

All metrics exposed at `GET /v1/metrics` in both JSON (default) and Prometheus text format (`Accept: text/plain`).

### 5.1 System Metrics

```json
{
  "ts": "2026-04-18T14:22:01.493Z",
  "uptime_sec": 86412,
  "version": "0.5.2",

  "queries": {
    "total": 14822,
    "per_sec_1m": 12.4,
    "per_sec_5m": 9.8,
    "per_sec_15m": 8.2,
    "p50_us": 198000,
    "p95_us": 441000,
    "p99_us": 1240000,
    "slow_total": 14,
    "error_total": 3,
    "by_type": {
      "DIVERGENCE": 441,
      "RICCI": 88,
      "SELECT": 12841,
      "INSERT": 1248,
      "CURVATURE": 204
    }
  },

  "ingest": {
    "records_total": 2841022,
    "bytes_total": 148000000,
    "per_sec_1m": 84.2
  },

  "bundles": {
    "total": 14,
    "total_records": 7841022,
    "total_bytes_mmap": 392000000,
    "total_bytes_wal": 14800000,
    "stats_cache_warm": 11,
    "stats_cache_cold": 3
  },

  "connections": {
    "http_active": 4,
    "ws_active": 12,
    "http_total": 14822,
    "ws_total": 48
  },

  "anomalies": {
    "detected_1h": 3,
    "detected_24h": 47,
    "by_bundle": {
      "sensor_das": 31,
      "sensor_sonar": 16
    }
  },

  "wal": {
    "total_checkpoints": 12,
    "pending_bytes": 284100,
    "last_checkpoint_us": 184022
  },

  "geometric": {
    "avg_kl_forward_1h": 0.312,
    "avg_jensen_shannon_1h": 0.051,
    "avg_k_global": { "sensor_das": 0.021, "sensor_sonar": 0.019 }
  }
}
```

The `geometric` block on the metrics endpoint is unique. It tells operators the **average mathematical health of the system's data** — not just throughput numbers.

### 5.2 Prometheus Format

`GET /v1/metrics` with `Accept: text/plain`:

```
# HELP gigi_queries_total Total queries executed
# TYPE gigi_queries_total counter
gigi_queries_total 14822

# HELP gigi_query_duration_microseconds Query latency percentiles
# TYPE gigi_query_duration_microseconds summary
gigi_query_duration_microseconds{quantile="0.5"} 198000
gigi_query_duration_microseconds{quantile="0.95"} 441000
gigi_query_duration_microseconds{quantile="0.99"} 1240000

# HELP gigi_anomalies_detected_total Total anomalies detected
# TYPE gigi_anomalies_detected_total counter
gigi_anomalies_detected_total{bundle="sensor_das"} 31
gigi_anomalies_detected_total{bundle="sensor_sonar"} 16

# HELP gigi_bundle_k_global Current Ricci curvature of bundle
# TYPE gigi_bundle_k_global gauge
gigi_bundle_k_global{bundle="sensor_das"} 0.021
gigi_bundle_k_global{bundle="sensor_sonar"} 0.019

# HELP gigi_kl_forward_avg_1h Average KL divergence for DIVERGENCE queries (1h)
# TYPE gigi_kl_forward_avg_1h gauge
gigi_kl_forward_avg_1h 0.312
```

---

## 6. Log Configuration

Runtime-configurable via `POST /v1/admin/log-config` (requires admin key):

```json
{
  "level": "INFO",
  "slow_query_threshold_us": 1000000,
  "categories": {
    "query": true,
    "ingest": true,
    "wal": true,
    "connection": false,
    "stream": true,
    "bundle": true,
    "anomaly": true,
    "audit": true,
    "system": true
  },
  "retention": {
    "query_log_days": 30,
    "slow_log_days": 90,
    "anomaly_log_days": 90,
    "audit_log_days": 365,
    "stream_log_days": 7,
    "wal_log_days": 14,
    "ingest_log_days": 14,
    "connection_log_days": 7,
    "system_log_days": 30
  },
  "output": {
    "stdout": true,
    "stdout_format": "json",
    "internal_bundles": true,
    "internal_format": "dhoom",
    "file": null,
    "file_format": "dhoom"
  }
}
```

**Defaults:**
- `connection` logging is **off by default** (high volume, low signal — turn on for debugging)
- `audit` logging is **always on** and cannot be disabled
- `slow_query_threshold_us`: 1,000,000 (1 second)
- Retention for audit/anomaly logs defaults to 365/90 days (compliance-friendly)

---

## 7. Log Output Destinations

GIGI writes logs to three destinations simultaneously (each configurable):

1. **Internal `_gigi_*` bundles (DHOOM, primary)** — async write to WAL-backed DHOOM bundles. Queryable via GQL with the full GIGI query engine. This is the killer feature: **GIGI queries its own operation logs using the same geometric engine it uses for everything else.** Every `_gigi_*` bundle is a first-class DHOOM bundle: mmap-backed after checkpoint, delta-encoded timestamps, interned strings for event types and bundle names, compacted on the same schedule as user data.

2. **stdout (JSON, compatibility)** — newline-delimited JSON for log shippers. Ship to Loki, Splunk, Datadog, CloudWatch with a sidecar or log driver. Zero dependency. This is the compatibility bridge to the ecosystem, not the primary format.

3. **File (DHOOM or JSON, optional)** — rolling file at a configurable path. Format configurable: `dhoom` produces a compact binary-friendly file readable by `gigi-convert`; `json` produces standard NDJSON. For environments without log shipping infrastructure.

### Why DHOOM beats JSON for logs

| Property | JSON (every competitor) | DHOOM (GIGI) |
|----------|------------------------|--------------|
| Schema | Repeated on every line | Declared once in fiber header |
| Timestamps | 24-char ISO string per event | μs delta from bundle epoch |
| Repeated strings | Full string every row | Interned pool (`&[...]`) |
| Bundle names | `"sensor_das"` × N rows | Interned index × N rows |
| Event types | `"query.complete"` × N rows | 1-byte index × N rows |
| Storage overhead | ~400 bytes/event | ~120 bytes/event (est. 70% reduction) |
| Queryable | Only with external parser | Native GQL — SELECT, GROUP BY, DIVERGENCE |
| Compaction | External rotation/gzip | Native GIGI compactor |
| Roundtrip | jq / grep | `gigi-convert` + GQL |

---

## 8. Implementation Plan

### Phase 1 — Foundation (Week 1)
- Add `tracing` + `tracing-subscriber` crates (JSON formatter)
- Replace all `eprintln!` with structured `tracing::info!` / `tracing::error!` calls
- Implement base log schema with `request_id`, `duration_us`, `ts`, `level`
- Wire `query.complete` and `query.error` events in `gigi_stream.rs`

### Phase 2 — System Bundles (Week 2)
- Implement `LogIngester` — async channel receiver that writes log events to internal bundles
- Create `_gigi_query_log`, `_gigi_slow_log`, `_gigi_anomaly_log`, `_gigi_system_log`
- Wire `bundle.stats_cache_warm` event in `mmap_bundle.rs`
- Wire WAL events in `wal.rs`

### Phase 3 — Metrics + Stream (Week 3)
- Implement `GET /v1/metrics` (JSON + Prometheus)
- Wire stream lifecycle events (`stream.subscribe`, `stream.push`, `stream.anomaly_push`, `stream.disconnect`)
- Wire `ingest.complete` + `ingest.bulk` with `k_delta`
- Wire audit events

### Phase 4 — Config + Retention (Week 4)
- `POST /v1/admin/log-config` runtime configuration
- TTL-based retention on `_gigi_*` bundles
- `GET /v1/admin/log-config` to read current config
- Documentation + demo update

---

## 9. What This Beats and Why

| Feature | ClickHouse (best competitor) | GIGI |
|---------|------------------------------|------|
| Log format | SQL-queryable system tables (CSV rows) | GQL-queryable DHOOM bundles |
| Query log | ✅ ProfileEvents (100s of counters) | ✅ + geometric fields (κ, KL, JS) |
| Slow query | ✅ | ✅ + self-annotating (explains why) |
| Audit log | `log_queries` only | ✅ Full audit trail, always free |
| Stream lifecycle | ❌ | ✅ Full WebSocket semantic logging |
| Stats cache | No equivalent | ✅ `bundle.stats_cache_warm` explains cold query |
| Ingest geometry | ❌ | ✅ `k_delta` on every ingest |
| Anomaly log | ❌ | ✅ Dedicated `_gigi_anomaly_log` bundle |
| Log drift detection | ❌ | ✅ `DIVERGENCE FROM _gigi_query_log_week1 TO _gigi_query_log_week2` |
| Prometheus export | Via external plugin | ✅ Native |
| Audit: always free | ❌ Enterprise/Commercial | ✅ Always |
| Log storage format | CSV rows (ClickHouse) / text files (rest) | **DHOOM bundles** — delta-encoded, interned, compacted |
| Log file portability | Proprietary / syslog | `gigi-convert` reads any `_gigi_*` bundle |
| Log compression | External gzip | Native GIGI mmap compactor (same pipeline as user data) |

The core advantage is twofold: **GIGI's logs are queryable with the same geometric engine as its data** (run curvature on your query log; detect anomalies in your anomaly log), and **GIGI's logs are stored in DHOOM** — the same compact, fiber-bundle-schema format as all GIGI data. Log files are not second-class citizens. They are first-class GIGI bundles.

---

*Spec status: DRAFT v1.1 — ready for implementation review*
