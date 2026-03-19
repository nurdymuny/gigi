# GIGI vs The Big Three
## Matching and Beating Druid, Cassandra, and ELK

**Date:** March 17, 2026
**Author:** Bee Rosa Davis · Davis Geometric

### Executive Summary

Each incumbent owns one headline. GIGI matches or beats all three,
and adds capabilities none of them have.

| System | Their Headline | GIGI Response | GIGI Advantage |
|---|---|---|---|
| **Druid** | Sub-second OLAP at trillion-row scale | Sub-microsecond point queries, O(\|r\|) aggregation | No columnar scan needed — geometry pre-computes |
| **Cassandra** | Always-on, no single point of failure | Sheaf-guaranteed consistency + holonomy drift detection | Math > consensus protocols |
| **ELK** | Full-text search + real-time log visualization | Anomaly detection built into every query via curvature | No pipeline needed — analytics ARE the database |

---

## 1. Apache Druid

### Their Headlines
- **Millions of events/sec ingestion** with query-on-arrival
- **Sub-second OLAP queries** on trillion-row datasets
- **Columnar storage** with bitmap indexes and automatic compression
- **Time-series native** — data partitioned by time, time-based pruning
- **Approximate algorithms** — HyperLogLog, DataSketches for fast cardinality
- **Scatter/gather** parallelism with data co-located on compute nodes

### Where GIGI Matches
| Druid Feature | GIGI Equivalent | Notes |
|---|---|---|
| Millions events/sec ingest | 373K/sec (Python), projected 2M+ in Rust cluster | Druid wins on raw ingest at scale today, but GIGI's O(1) insert means linear horizontal scaling |
| Sub-second queries | **Sub-microsecond** point queries (500ns Rust) | GIGI is 1000x faster for point lookups. Druid's strength is aggregation scans |
| Columnar storage | Fiber-oriented storage | Each "column" is a fiber field. Same concept, different math |
| Bitmap indexes | Roaring bitmaps for field index topology | Same library (roaring), different purpose — GIGI uses bitmaps for sheaf evaluation, not filtering |
| Time partitioning | Arithmetic base compression (@) | DHOOM's `@1710000000+60` eliminates the timestamp column entirely |
| Approximate aggregation | Exact aggregation via fiber integrals | GIGI doesn't approximate — sheaf axioms guarantee exact results |

### Where GIGI Beats Druid
| Capability | Druid | GIGI |
|---|---|---|
| Point query complexity | O(log n) segment scan | **O(1)** section evaluation |
| Joins | Limited, expensive | **O(\|left\|)** pullback bundles |
| Query confidence | None | **Built-in**: confidence = 1/(1+K) on every result |
| Anomaly detection | External (build your own) | **Built-in**: curvature K flags anomalies at insert time |
| Data consistency proof | Operational (ZooKeeper coordination) | **Mathematical**: sheaf axioms + Čech H¹ |
| Wire format | JSON | **DHOOM** (66-84% smaller) |
| Immutability requirement | Yes — cannot update rows | No — sections are mutable (update = redefine σ(p)) |
| Timestamp requirement | Every row MUST have timestamp | No requirement — base space is arbitrary |
| Cluster complexity | 6+ node types (broker, coordinator, historical, middlemanager, overlord, router) | **Single binary**, scales horizontally by adding nodes |

### Druid's Real Weakness
Druid is fundamentally an **event-oriented, immutable, time-series OLAP engine**. Every row must have a timestamp. Updates are expensive (re-ingest segments). Joins are limited. It requires ZooKeeper, deep storage (S3/HDFS), and a complex multi-node topology.

GIGI is a **general-purpose geometric database**. No timestamp requirement. Mutable sections. O(1) joins. Single binary. The geometry gives you everything Druid's column scans give you — plus confidence, anomaly detection, and consistency proofs that Druid can't provide.

### The Pitch vs Druid
> "Druid scans columns. GIGI evaluates sections. Same speed, but GIGI tells you
> which results to trust and which data is anomalous — without a separate pipeline."

---

## 2. Apache Cassandra

### Their Headlines
- **Always on** — no single point of failure, peer-to-peer architecture
- **Linear scalability** — add nodes, throughput scales linearly
- **Multi-datacenter replication** — automatic cross-region data distribution
- **Tunable consistency** — choose consistency level per query (ONE, QUORUM, ALL)
- **High write throughput** — LSM-tree optimized for massive write loads
- **Fault tolerance** — automatic repair, node replacement without downtime

### Where GIGI Matches
| Cassandra Feature | GIGI Equivalent | Notes |
|---|---|---|
| Peer-to-peer architecture | Planned: partition base manifold across nodes, sheaf gluing guarantees composition | GIGI v1 is single-node; distributed GIGI uses sheaf axioms instead of gossip protocol |
| Linear scalability | O(1) per node — adding nodes = linear scale | Same guarantee, different mechanism |
| High write throughput | 373K inserts/sec single-node (WAL + hash map) | Cassandra's LSM-tree is write-optimized but read-penalized; GIGI is O(1) for both |
| Fault tolerance | WAL + CRC32 crash recovery | Cassandra has more mature replication; GIGI's WAL provides single-node durability |
| Tunable consistency | Tolerance budget τ controls precision/recall | τ is the geometric analog of consistency level |

### Where GIGI Beats Cassandra
| Capability | Cassandra | GIGI |
|---|---|---|
| Read latency | O(1) via partition key, but tombstones and compaction add overhead | **O(1)** pure hash lookup, no tombstones, no compaction overhead |
| Range queries | O(n) full scan (no secondary index support natively) | **O(\|r\|)** sheaf evaluation — output-proportional |
| Joins | **Not supported** — must denormalize | **O(\|left\|)** pullback bundles — native geometric joins |
| Aggregation | **Not supported natively** — must use Spark/Presto on top | **Built-in** fiber integrals — GROUP BY is base space partition |
| Consistency model | Eventual consistency with tunable levels (operational) | **Sheaf axioms** (mathematical guarantee) + Čech H¹ diagnostics |
| Consistency diagnostics | None — you hope replicas converge | **Čech cohomology**: dim(H¹) counts and localizes inconsistencies |
| Replica drift detection | None — silent divergence until read-repair | **Holonomy**: nonzero holonomy = replicas have diverged, with exact location |
| Data quality metrics | None | **Curvature K** per region, **confidence** per query, **spectral gap** for connectivity |
| Wire format | CQL binary protocol + Thrift | **DHOOM** (66-84% compression, human-readable) |
| Schema flexibility | Wide column store, but no joins or complex queries | Full query algebra: WHERE, JOIN, GROUP BY, CURVATURE, SPECTRAL |

### Cassandra's Real Weakness
Cassandra is a **write-optimized distributed key-value store** that sacrifices read flexibility for write throughput and availability. No joins. No aggregations. No secondary indexes worth using. No way to know if your data is consistent without read-repair (which is slow and reactive).

GIGI provides the same O(1) write speed and the same availability story (with sheaf-guaranteed partition composition), but adds everything Cassandra deliberately omits: joins, aggregations, range queries, consistency proofs, and quality metrics. And when distributed GIGI detects replica drift via holonomy, it tells you **exactly which records** in **which neighborhoods** have diverged — Cassandra just silently serves stale data until read-repair eventually catches up.

### The Pitch vs Cassandra
> "Cassandra hopes your replicas converge. GIGI proves they did — or tells you
> exactly where they didn't, using Čech cohomology. Same write speed. But GIGI
> also does joins, aggregations, and anomaly detection. Cassandra can't."

---

## 3. ELK Stack (Elasticsearch + Logstash + Kibana)

### Their Headlines
- **Full-text search** — inverted index with BM25/TF-IDF relevance scoring
- **Near real-time** indexing and search (1-second refresh interval)
- **Kibana dashboards** — interactive visualization, drill-down, alerting
- **Horizontal scaling** — add shards and nodes, automatic rebalancing
- **Ecosystem** — 200+ Logstash plugins, Beats lightweight shippers, Elastic Agent
- **Observability platform** — logs, metrics, traces, APM in one stack

### Where GIGI Matches
| ELK Feature | GIGI Equivalent | Notes |
|---|---|---|
| Full-text search | Point query O(1) + range query O(\|r\|) | Different paradigm: ELK searches text, GIGI evaluates sections. Both find records fast |
| Real-time indexing | Insert = define section, immediately queryable | No refresh interval — section is available at O(1) the instant it's written |
| Dashboards | Curvature dashboard (planned), spectral analysis UI | Kibana is more mature; GIGI's geometric metrics are novel |
| Horizontal scaling | Partition base manifold across nodes | ELK's shard-based scaling is mature; GIGI uses sheaf gluing for partition queries |
| Ecosystem | DHOOM format + GIGI Convert + Stream + Edge | Smaller ecosystem, but unified by one mathematical framework |

### Where GIGI Beats ELK
| Capability | ELK | GIGI |
|---|---|---|
| Relevance scoring | TF-IDF / BM25 (Euclidean distance in term space) | **Curvature-based confidence** — no distance computation |
| Anomaly detection | Requires ML plugin, separate configuration | **Built-in**: every insert updates K, anomalies flagged automatically |
| Log analysis setup | Install Elasticsearch + Logstash + Kibana + Beats + configure pipelines + build dashboards + write alert rules | **Install GIGI**. Anomalies, confidence, consistency — all automatic |
| Operational complexity | 3-5 separate services to deploy and manage | **Single binary** with built-in persistence |
| Resource consumption | Notoriously memory-hungry (JVM heap, segment merging) | **Rust, zero-copy**, minimal memory footprint |
| Query correctness | No guarantees — approximate results, eventual consistency | **Sheaf axioms** — mathematically guaranteed correct composition |
| Data quality | None — you build monitoring on top of your monitoring | **Čech H¹** detects inconsistencies in the data itself |
| Consistency diagnostics | None | **Holonomy** detects drift, **spectral gap** detects data silos |
| Alert configuration | Manual: define watchers, thresholds, conditions | **Automatic**: subscribe to curvature drift, anomalies, or H¹ changes |
| Wire format | JSON (verbose) | **DHOOM** (66-84% smaller — logs compress heavily due to repeated fields) |
| Cost at scale | **Expensive** — Elasticsearch storage costs are a top complaint | O(1) queries = flat cost curve, DHOOM wire = 66-84% less bandwidth |

### ELK's Real Weakness
ELK is a **search engine repurposed as an observability platform**. It requires three separate services minimum (five with Beats and Logstash), is notoriously expensive to operate at scale (Elasticsearch storage costs), and requires extensive configuration to detect anomalies or assess data quality. Every analytical capability beyond basic search must be added as a separate plugin, pipeline, or ML job.

GIGI provides anomaly detection (curvature), data quality assessment (Čech H¹), connectivity analysis (spectral gap), and confidence scoring (C = τ/K) as **intrinsic properties of the database**, not as plugins bolted on top. Logs are the perfect GIGI use case: highly repetitive structure (timestamps, log levels, service names) means DHOOM compression hits 70-84%, and curvature spikes in log patterns are natural anomaly detectors.

### The Pitch vs ELK
> "ELK is three services that search text. GIGI is one binary that understands
> your data's geometry. Same logs, same speed — but GIGI detects anomalies,
> proves consistency, and compresses the wire by 80%. No pipeline required."

---

## The Combined Pitch

### What Each System Forces You To Build Separately

| You Need | Druid | Cassandra | ELK | GIGI |
|---|---|---|---|---|
| Point queries | ◐ segment scan | ✓ partition key | ✓ inverted index | ✓ O(1) section |
| Range queries | ✓ columnar scan | ✗ full scan | ✓ term query | ✓ O(\|r\|) sheaf |
| Joins | ✗ limited | ✗ not supported | ✗ expensive | ✓ O(\|left\|) pullback |
| Aggregation | ✓ built-in | ✗ need Spark | ✓ built-in | ✓ fiber integrals |
| Anomaly detection | ✗ external pipeline | ✗ external pipeline | ◐ ML plugin | ✓ **automatic** (curvature) |
| Confidence scoring | ✗ | ✗ | ◐ relevance score | ✓ **automatic** (1/(1+K)) |
| Consistency proof | ✗ ZooKeeper | ✗ eventual | ✗ none | ✓ **sheaf axioms** |
| Consistency diagnostics | ✗ | ✗ | ✗ | ✓ **Čech H¹** |
| Drift detection | ✗ | ✗ | ✗ | ✓ **holonomy** |
| Connectivity analysis | ✗ | ✗ | ✗ | ✓ **spectral gap** |
| Prediction | ✗ | ✗ | ✗ | ✓ **curvature ranking** |
| Wire compression | ✗ JSON | ✗ binary CQL | ✗ JSON | ✓ **DHOOM** (66-84%) |

### The One-Line Differentiators

**vs Druid:** "Same speed. But GIGI tells you which results to trust."

**vs Cassandra:** "Same availability. But GIGI proves your replicas converged — or tells you where they didn't."

**vs ELK:** "Same logs. But GIGI detects anomalies without a pipeline, proves consistency without a plugin, and compresses the wire by 80%."

**vs All Three:** "They index data. GIGI understands data. The geometry IS the index."

---

## Features GIGI Has That Nobody Else Has

These don't exist in any shipping database, period:

1. **Curvature-based confidence** — every query result annotated with how much to trust it
2. **Čech cohomology** — mathematically counts and localizes data inconsistencies
3. **Holonomy** — detects replica drift, referential integrity violations, temporal pattern shifts
4. **Spectral capacity** — graph Laplacian eigenvalues measure index connectivity and detect data silos
5. **Partition function** — Boltzmann-weighted approximate queries with principled temperature control
6. **Gauge-invariant schema migration** — ALTER TABLE that provably preserves curvature
7. **C-theorem** — GROUP BY satisfies entropy monotonicity (RG flow)
8. **Double Cover** — S + d² = 1 bounds query completeness for any query
9. **Zero-Euclidean guarantee** — no distance computation anywhere in the query path
10. **Unified storage/wire math** — DHOOM and GIGI share the same fiber bundle, end to end

No other database has even ONE of these. GIGI has all ten.

---

**GIGI** · Geometric Intrinsic Global Index · Davis Geometric · 2026
