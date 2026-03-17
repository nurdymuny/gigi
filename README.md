# GIGI — Geometric Intrinsic Global Index

A database engine built on the mathematics of fiber bundles. Every table is a geometric object, every query is a geometric operation, and every result carries curvature and confidence — not just data.

Written in Rust. Created by [Davis Geometric](https://davisgeometric.com).

> U.S. Patent Pending · Application No. 64/008,940 · Filed March 18, 2026

---

## Why GIGI?

Traditional databases store rows. GIGI stores **geometry**. The data structure *is* the math — fiber bundles, gauge fields, curvature tensors — so the answers fall out of the structure itself, with zero distance computations at query time.

**What does that buy you?**

| Operation | PostgreSQL / MySQL | GIGI |
|---|---|---|
| Point query | O(log N) B-tree | **O(1)** — one hash, one lookup |
| Range query | O(log N + \|result\|) | **O(\|result\|)** — independent of N |
| JOIN | O(\|L\| + \|R\|) hash join | **O(\|L\|)** — pullback bundle, no build phase |
| Insert | O(log N) | **O(1)** amortized |
| Anomaly detection | Separate ML pipeline | **Built-in** — curvature identifies outliers natively |
| Confidence score | Not available | **Every query** — 1/(1+K), always in [0, 1] |
| Consistency proof | Manual checks | **Čech cohomology** — H¹ counts and locates conflicts |
| Wire protocol | JSON (verbose) | **DHOOM** — 50–84% smaller, patent pending |

These aren't benchmarks. They're **theorems** — proven complexities, not measured averages.

---

## Quick Start

### Docker

```bash
docker pull beerosadavis/gigi:latest
docker run -p 3142:3142 beerosadavis/gigi:latest
```

### Build from Source

```bash
# Requires Rust 1.92+
cargo build --release
cargo test  # 289 tests
```

---

## Architecture

GIGI replaces the relational model with differential geometry:

| Relational | GIGI |
|---|---|
| Table | **Bundle** — fiber bundle (E, B, F, π, Φ) |
| Row | **Section** — σ: B → E mapping base points to fiber values |
| Primary key | **Base point** — addressed by the GIGI hash (64-bit, full avalanche) |
| Columns | **Fiber** — the value space F₁ × F₂ × … × Fₖ |
| Default values | **Zero section** σ₀ — baseline from which deviations are measured |
| JOIN | **Pullback bundle** f*E₂ |
| GROUP BY | **Base space partition** |
| Index | **Field index** — bitmap-based open set topology |

**18 Rust modules**: types, hash, bundle, metric, query, curvature, join, aggregation, wal, engine, gauge, spectral, parser, concurrent, dhoom, convert, edge, crypto.

---

## GQL — Geometric Query Language

GQL is a strict superset of SQL. Every SQL operation has a geometric equivalent. GQL adds **50+ operations impossible in SQL**.

```sql
-- Familiar SQL works as-is
CREATE BUNDLE sensors (temperature FIBER, humidity FIBER, location SECTION);
INSERT INTO sensors VALUES (72.5, 45.0, "lab-1");
SELECT * FROM sensors WHERE temperature > 70;

-- GQL-only: curvature, anomalies, prediction
SELECT curvature(temperature, humidity) FROM sensors;
SELECT * FROM sensors WHERE curvature > 2.0;  -- anomalies
PREDICT temperature FROM sensors;

-- Spectral geometry
SELECT spectral_gap FROM sensors;
SELECT bottleneck FROM sensors;

-- Consistency proofs
SELECT consistency FROM sensors;  -- H¹ = 0 means fully consistent

-- Geometric encryption — analytics on encrypted data at native speed
ENCRYPT sensors WITH KEY 'my-secret';
SELECT curvature(temperature) FROM sensors;  -- works on encrypted data

-- Real-time subscriptions
SUBSCRIBE ANOMALIES FROM sensors WHERE curvature > 3.0;
SUBSCRIBE CURVATURE DRIFT > 0.5 FROM sensors;
SUBSCRIBE CONSISTENCY FROM sensors;
```

### GQL-Only Categories

| Category | Operations |
|---|---|
| **Curvature** | CURVATURE, RICCI, SECTIONAL, SCALAR, TREND |
| **Spectral** | SPECTRAL, BOTTLENECK, CLUSTER, MIXING, CONDUCTANCE, LAPLACIAN |
| **Information** | ENTROPY, DIVERGENCE, FISHER, MUTUAL, CAPACITY |
| **Stat Mech** | FREEENERGY, PHASE, CRITICAL, TEMPERATURE, PARTITION |
| **Topology** | BETTI, EULER, COCYCLE, COBOUNDARY, TRIVIALIZE |
| **Gauge Theory** | WILSON, GAUGE VERIFY, CHARACTERISTIC |
| **Transport** | TRANSPORT, GEODESIC, FLOW |
| **Similarity** | SIMILAR, CORRELATE, SEGMENT, OUTLIER, PROFILE |
| **Consistency** | CONSISTENCY, CONSISTENCY REPAIR, HOLONOMY |
| **Prediction** | PREDICT (curvature-ranked forecasting) |
| **Subscriptions** | ANOMALIES, CURVATURE DRIFT, PHASE, DIVERGENCE |
| **SQL Compat** | TRANSLATE SQL "..." → auto-converts SQL to GQL |

Full specification: [GQL_SPECIFICATION.md](GQL_SPECIFICATION.md) · [GQL_ADDENDUM_v2.1.md](GQL_ADDENDUM_v2.1.md)

---

## DHOOM Wire Protocol

GIGI's native serialization format exploits the fiber bundle's zero section. Only deviations from the default are transmitted.

- **`@start+step`** — arithmetic sequences are entirely elided (timestamps, IDs)
- **`|value`** — modal defaults — only deviations transmitted
- **Trailing elision** — omit trailing default fields per record

Result: **50–84% smaller than JSON** on real workloads. Curvature can be estimated *during deserialization* at zero additional cost.

---

## Geometric Encryption

Per-bundle gauge transformations on fiber coordinates:

- Numeric fields encrypted via per-field affine transform derived from a 32-byte key
- **Curvature is gauge-invariant** — all geometric operations (curvature, confidence, spectral gap, anomaly detection, prediction) work on encrypted data at native speed
- Only human-readable output (SELECT) requires decryption

---

## Edge Sync

Local-first architecture with sheaf-based conflict resolution:

- All reads are **always local, O(1), offline-capable**
- Writes queue locally, sync when connected
- Sync uses the **sheaf gluing axiom**: H¹ = 0 → clean merge, H¹ > 0 → conflicts detected and located
- Conflicts identified by (bundle, field, key) with local + remote values

---

## Binaries

| Binary | Description |
|---|---|
| `gigi` | CLI — ingest, query, convert |
| `gigi-stream` | HTTP/WebSocket streaming server (port 3142) |
| `gigi-server` | Classic HTTP API server |
| `gigi-edge` | Edge-sync replication node |
| `gigi-convert` | JSON ↔ DHOOM format converter |
| `gigi-stress` | Load testing / stress tool |

---

## API

39 REST endpoints + WebSocket API. Full CRUD, batch operations, streaming ingest, pullback joins, aggregation, curvature, spectral analysis, consistency proofs, real-time subscriptions.

OpenAPI spec: [openapi.json](openapi.json) · API docs: [GIGI_API.md](GIGI_API.md)

---

## Tests

289 tests across 17 modules. 170 API endpoint tests. 86 TDD specifications with 104 assertions.

```bash
cargo test
```

---

## License

Source-available — **free for non-commercial use** (personal projects, academic research, education, evaluation). Commercial use and patent rights require a separate license from [Davis Geometric](https://davisgeometric.com).

For commercial licensing: bee_davis@alumni.brown.edu

See [LICENSE](LICENSE).