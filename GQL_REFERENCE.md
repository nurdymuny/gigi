# GQL — Geometric Query Language
## Version 2.1 — Complete Reference

**Authors:** Bee Rosa Davis · Davis Geometric  
**The query language for fiber bundle databases.**

SQL thinks in tables and rows. GQL thinks in bundles, sections, and fibers. Every keyword maps to a geometric operation. The language IS the math.

---

## Design Principles

1. **Geometric primitives, not relational.** SECTION, BUNDLE, COVER, PULLBACK, INTEGRATE — not SELECT, FROM, JOIN, GROUP BY.
2. **Confidence is not optional.** Every query returns confidence. Always.
3. **Analytics are verbs, not plugins.** CURVATURE, SPECTRAL, HOLONOMY, CONSISTENCY are first-class.
4. **The wire format is part of the language.** EMIT controls DHOOM serialization.
5. **Silence means agreement.** Omitted clauses inherit bundle defaults.
6. **Geometry determines complexity.** SECTION AT = O(1). COVER ON = O(|r|). PULLBACK = O(|left|). INTEGRATE = O(|group|).
7. **SQL compatibility via TRANSLATE.** Any SQL → GQL. Not all GQL → SQL.

---

## Implementation Status

| Status | Meaning |
|---|---|
| ✅ Implemented | Working in current server (`gigi-stream`) |
| ⚠️ Parsed | Accepted by parser, silently does nothing |
| ❌ 501 | Returns HTTP 501 Not Implemented |

| Feature | Status | Notes |
|---|---|---|
| BUNDLE / GAUGE / COLLAPSE / LENS | ✅ | Full schema operations |
| SECTION / SECTIONS / UPSERT | ✅ | Insert and batch insert |
| REDEFINE / BULK REDEFINE | ✅ | Update operations |
| RETRACT / BULK RETRACT | ✅ | Delete operations |
| SECTION AT / EXISTS SECTION | ✅ | O(1) point queries |
| COVER (all variants) | ✅ | Range, filtered, ranked, paginated |
| INTEGRATE | ✅ | Aggregation with FILTER, HAVING |
| FIBER / TRANSPORT (window) | ✅ | Window functions |
| PULLBACK / PRODUCT | ✅ | Joins |
| ATLAS (transactions) | ✅ | BEGIN / COMMIT / ROLLBACK |
| SHOW BUNDLES / DESCRIBE | ✅ | |
| CURVATURE / RICCI / SPECTRAL | ✅ | |
| CONSISTENCY (+ REPAIR) | ✅ | |
| BETTI / ENTROPY / FREEENERGY | ✅ | |
| GEODESIC / METRIC TENSOR | ✅ | |
| HEALTH | ✅ | |
| CREATE BUNDLE with ENCRYPTED | ✅ | Geometric encryption |
| SUBSCRIBE / UNSUBSCRIBE | ✅ | WebSocket subscriptions |
| EXPLAIN / TRANSLATE SQL | ✅ | |
| COMPLETE / PROPAGATE / SUGGEST_ADJACENCY | ⚠️ | Parsed; sheaf module built but not wired |
| CREATE/DROP TRIGGER | ⚠️ | Parsed; TriggerManager built but not wired |
| INVALIDATE CACHE / COMPACT / ANALYZE / VACUUM / REBUILD INDEX | ⚠️ | Parsed; no-op |
| SHOW INDEXES / SHOW TRIGGERS / etc. | ⚠️ | Parsed; no-op |
| WEAVE / UNWEAVE ROLE | ❌ | 501 |
| GRANT / REVOKE / POLICY | ❌ | 501 |
| PREPARE / EXECUTE / DEALLOCATE | ❌ | 501 |
| BACKUP / RESTORE / VERIFY BACKUP | ❌ | 501 |
| COMMENT ON | ❌ | 501 |
| SET (session variables) | ❌ | 501 |
| SELECT (SQL compat) | ⚠️ | Falls through; use COVER/PULLBACK instead |

---

## Complete SQL → GQL Mapping

| SQL | GQL | Geometric Meaning |
|---|---|---|
| **Schema** | | |
| CREATE TABLE t (...) | BUNDLE t BASE (...) FIBER (...) | Define fiber bundle (E, B, F, π) |
| ALTER TABLE t ADD/DROP/RENAME | GAUGE t TRANSFORM (...) | Gauge transformation on fiber |
| DROP TABLE t | COLLAPSE t | Collapse the bundle |
| CREATE INDEX ON t(f) | (automatic — INDEX modifier) | Topology on base space |
| CREATE VIEW v AS ... | LENS v AS (...) | Virtual bundle via morphism |
| CREATE MATERIALIZED VIEW | LENS v AS (...) MATERIALIZE | Precomputed bundle snapshot |
| **Write** | | |
| INSERT INTO t VALUES (...) | SECTION t (...) | Define section σ at base point |
| batch INSERT | SECTIONS t (...) | Batch section definition |
| UPDATE t SET f=v WHERE k=v | REDEFINE t AT k=v SET (f=v) | Redefine section at a point |
| DELETE FROM t WHERE k=v | RETRACT t AT k=v | Retract section from base point |
| UPSERT / ON CONFLICT | SECTION t (...) UPSERT | Define-or-redefine |
| **Read (point)** | | |
| SELECT * FROM t WHERE pk=v | SECTION t AT pk=v | Section evaluation σ(p) — O(1) |
| SELECT f1,f2 FROM t WHERE pk=v | SECTION t AT pk=v PROJECT (f1,f2) | Projected section |
| SELECT EXISTS(...) | EXISTS SECTION t AT pk=v | Section existence check |
| **Read (range)** | | |
| SELECT * FROM t WHERE f=v | COVER t ON f=v | Sheaf evaluation F(U) — O(|r|) |
| SELECT * FROM t WHERE f IN (...) | COVER t ON f IN (...) | Union of open sets |
| SELECT * FROM t WHERE f > v | COVER t WHERE f > v | Fiber predicate scan |
| SELECT * FROM t WHERE a AND b | COVER t ON a WHERE b | Topology + fiber filter |
| SELECT DISTINCT f FROM t | COVER t DISTINCT f | Field index key enumeration |
| SELECT * ... ORDER BY f | COVER t ... RANK BY f | Ordered cover |
| SELECT * ... LIMIT n | COVER t ... FIRST n | Truncated cover |
| SELECT * ... OFFSET m LIMIT n | COVER t ... SKIP m FIRST n | Paginated cover |
| SELECT * ... WHERE f LIKE '%x' | COVER t WHERE f MATCHES '...' | Pattern on fiber values |
| SELECT * ... WHERE f IS NULL | COVER t WHERE f VOID | Undefined fiber component |
| SELECT * ... WHERE f IS NOT NULL | COVER t WHERE f DEFINED | Defined fiber component |
| **Aggregation** | | |
| SELECT f, AGG(g) GROUP BY f | INTEGRATE t OVER f MEASURE agg(g) | Fiber integral ∫ h dσ |
| SELECT AGG(f) FROM t | INTEGRATE t MEASURE agg(f) | Global fiber integral |
| ... GROUP BY f HAVING agg(g)>v | ... HAVING agg(g) > v | Post-integration filter |
| **Window functions** | | |
| ROW_NUMBER() OVER (PART BY f ORDER BY g) | FIBER RANK t OVER f RANK BY g | Fiber-wise ranking |
| SUM(f) OVER (PART BY g ROWS BETWEEN m AND n) | FIBER SUM t OF f OVER g WINDOW h ROWS (m,n) | Sliding fiber integral |
| LAG(f,n) OVER (ORDER BY g) | TRANSPORT t OF f ALONG g SHIFT -n | Parallel transport backward |
| LEAD(f,n) OVER (ORDER BY g) | TRANSPORT t OF f ALONG g SHIFT +n | Parallel transport forward |
| **Joins** | | |
| t1 JOIN t2 ON fk=pk | PULLBACK t1 ALONG fk ONTO t2 | Pullback bundle f*E₂ — O(|left|) |
| t1 LEFT JOIN t2 ON fk=pk | PULLBACK t1 ALONG fk ONTO t2 PRESERVE LEFT | Left-preserving pullback |
| t1 CROSS JOIN t2 | PRODUCT t1 WITH t2 | Cartesian product bundle |
| t1 UNION t2 | UNION (q1) WITH (q2) | Bundle union |
| t1 INTERSECT t2 | INTERSECT (q1) WITH (q2) | Bundle intersection |
| t1 EXCEPT t2 | SUBTRACT (q1) MINUS (q2) | Bundle difference |
| **Subqueries / CTEs** | | |
| WHERE f IN (SELECT ...) | ON f IN (COVER ... PROJECT f) | Nested sheaf evaluation |
| WITH cte AS (...) SELECT ... | WITH name AS (...) ... | Named intermediate bundle |
| **Transactions** | | |
| BEGIN / COMMIT / ROLLBACK | ATLAS BEGIN / COMMIT / ROLLBACK | Atlas transition (coordinate change) |
| SAVEPOINT name | ATLAS SAVEPOINT name | Atlas checkpoint |
| **Admin** | | |
| EXPLAIN SELECT ... | EXPLAIN (any statement) | Geometric query plan |
| SHOW TABLES | SHOW BUNDLES | Bundle catalog |
| DESCRIBE t | DESCRIBE t | Schema + geometric properties |
| **GQL-Only (impossible in SQL)** | | |
| — | CURVATURE t ON f BY g | Local data variability K |
| — | CONFIDENCE t | Query trust 1/(1+K) |
| — | SPECTRAL t | Index connectivity λ₁ |
| — | HOLONOMY t AROUND (f1,f2) | Loop consistency |
| — | CONSISTENCY t | Čech H¹ diagnostic |
| — | CONSISTENCY t REPAIR | Cocycle resolution |
| — | PARTITION t AT p TOLERANCE τ | Boltzmann approximate query |
| — | CALIBRATE t TOLERANCE τ | Set default temperature |
| — | HEALTH t | Full geometric diagnostic |
| — | PREDICT t ON f BY g | Curvature-ranked forecasting |
| — | TRANSPORT t OF f FROM p TO q | Parallel transport between points |
| — | FLOW t COARSEN BY f LEVELS n | RG flow / C-theorem verification |
| — | GEODESIC t FROM p TO q | Shortest path on bundle |
| — | RESTRICT t TO (cover) AS name | Sheaf restriction to sub-bundle |
| — | GLUE (c1) WITH (c2) CHECK OVERLAP | Explicit sheaf gluing |
| — | SNAPSHOT t AS name | Point-in-time capture |
| — | DIFF t1 AGAINST t2 | Bundle comparison with ΔK |
| — | EMIT DHOOM / JSON / CSV | Wire format control |
| — | TRANSLATE SQL "..." | SQL-to-GQL compiler |
| — | COVER t ... CONFIDENCE >= c | Filter by geometric trust |
| — | HAVING CURVATURE(f) > v | Filter groups by curvature |
| **Spectral Geometry** | | |
| — | SPECTRAL t FULL | Full eigenvalue spectrum of index graph |
| — | BOTTLENECK t | Cheeger inequality — find weakest connectivity |
| — | CLUSTER t INTO n | Spectral clustering via Fiedler vector |
| — | MIXING t | Mixing time from spectral gap: O(1/λ₁) |
| — | CONDUCTANCE t ON f | Graph conductance of field index |
| — | LAPLACIAN t | Hodge Laplacian eigenvalues on data forms |
| **Information Geometry** | | |
| — | ENTROPY t | Shannon entropy of the bundle |
| — | ENTROPY t ON f BY g | Per-group entropy (RG flow integrand) |
| — | DIVERGENCE (c1) FROM (c2) | KL divergence between two covers |
| — | FISHER t ON f1, f2 | Fisher information metric between fields |
| — | MUTUAL t ON f1 WITH f2 | Mutual information between fields |
| — | CAPACITY t | C = τ/K at every base point |
| **Statistical Mechanics** | | |
| — | FREEENERGY t TOLERANCE τ | Helmholtz free energy F = −τ log Z |
| — | PHASE t ON f | Detect phase transitions (discontinuities in K) |
| — | CRITICAL t ON f | Find critical points (∂K/∂f = 0) |
| — | TEMPERATURE t | Effective temperature per neighborhood |
| **Curvature Variants** | | |
| — | RICCI t ON f1, f2 | Ricci curvature between field pairs |
| — | SECTIONAL t ON f1, f2 | Sectional curvature on 2-plane |
| — | SCALAR t | Scalar curvature (trace of Ricci) |
| — | DEVIATION t FROM p ALONG f | Geodesic deviation — how nearby sections diverge |
| — | TREND t ON f BY g | Curvature-based trend detection (dK/dt) |
| **Topology** | | |
| — | BETTI t | Betti numbers β₀, β₁, β₂ of the data bundle |
| — | EULER t | Euler characteristic χ = β₀ − β₁ + β₂ |
| — | COCYCLE t ON (f1, f2) | Extract specific Čech cocycles |
| — | COBOUNDARY t ON cocycle | Check if cocycle is exact (removable) |
| — | TRIVIALIZE t | Check if bundle is globally trivial (E ≅ B × F) |
| **Gauge Theory** | | |
| — | WILSON t AROUND path | Wilson loop — holonomy along explicit path |
| — | GAUGE VERIFY t | Verify K-invariance after migration |
| — | CHARACTERISTIC t | Characteristic classes (Chern/Euler) |
| **Similarity and Discovery** | | |
| — | SIMILAR t TO section_id WITHIN d | Find geometrically similar sections |
| — | CORRELATE t ON f1 WITH f2 | Geometric correlation via connection |
| — | SEGMENT t INTO n BY f | Geometric segmentation (curvature-based clustering) |
| — | OUTLIER t ON f SIGMA n | Flag outliers at nσ from local mean |
| — | PROFILE t | Full geometric data profile (all fields) |
| **Double Cover** | | |
| — | DOUBLECOVER t ON query | Compute S + d² = 1 for any query |
| — | RECALL t ON query | Compute recall S for a query |
| — | COMPLETENESS t ON query | Verify S + d² = 1 holds |
| **Subscriptions** | | |
| — | SUBSCRIBE ANOMALIES t ON f | Real-time curvature spike events |
| — | SUBSCRIBE CURVATURE t DRIFT > ε | Curvature drift events |
| — | SUBSCRIBE CONSISTENCY t | H¹ violation events |
| — | SUBSCRIBE SPECTRAL t | Topology change events |
| — | SUBSCRIBE PHASE t ON f | Phase transition events |
| — | SUBSCRIBE DIVERGENCE t THRESHOLD ε | Distribution drift events |

**Score: 45 SQL operations matched. 50+ GQL-only operations. 0 SQL operations missing.**

---

## I. Schema Operations

### BUNDLE — Define a fiber bundle ✅

```sql
BUNDLE sensors
  BASE (id NUMERIC)
  FIBER (
    city        CATEGORICAL INDEX,
    region      CATEGORICAL INDEX,
    date        NUMERIC,
    temp        NUMERIC RANGE 80,
    temp_max    NUMERIC RANGE 80,
    temp_min    NUMERIC RANGE 80,
    humidity    NUMERIC RANGE 100,
    pressure    NUMERIC RANGE 20,
    wind        NUMERIC RANGE 30,
    solar       NUMERIC RANGE 12,
    status      CATEGORICAL DEFAULT 'normal'
  );
```

**Field Types:** NUMERIC, CATEGORICAL, TEXT, BOOLEAN, TIMESTAMP

**Field Modifiers:**

| Modifier | Effect | DHOOM Wire |
|---|---|---|
| INDEX | Field index topology (enables COVER ON) | — |
| RANGE n | Domain width for curvature: K = variance/range² | — |
| DEFAULT v | Zero section σ₀ | `\|v` (elided when matching) |
| AUTO | Sequential base, array storage | `@start+1` |
| ARITHMETIC | Arithmetic sequence | `@start+step` |
| UNIQUE | Enforce injectivity across sections | — |
| REQUIRED | Must be defined in every section | — |
| NULLABLE | May be VOID (default) | — |
| ENCRYPTED | Gauge-invariant encryption (GaugeKey) | — |

**Storage auto-detection:** Single arithmetic key → SEQUENTIAL (array, K=0). Composite → HASHED. Mostly arithmetic → HYBRID. The curvature of the base space determines the storage engine.

**Bundle Options:**

```sql
BUNDLE events
  BASE (id NUMERIC AUTO)
  FIBER (...)
  OPTIONS (
    STORAGE AUTO,                -- AUTO | SEQUENTIAL | HASHED
    TOLERANCE 0.1,               -- default τ for PARTITION queries
    ANOMALY_THRESHOLD AUTO,      -- AUTO (adaptive) | FIXED n
    WAL ENABLED,                 -- write-ahead log
    COMPACTION INTERVAL 3600     -- seconds between compactions
  );
```

**Bundle with encryption:**

```sql
CREATE BUNDLE secrets
  BASE (id NUMERIC AUTO)
  FIBER (
    label CATEGORICAL,
    payload TEXT ENCRYPTED
  );
-- payload is stored and queried via gauge-invariant GaugeKey encryption
-- K is preserved under encryption (topology of the data is maintained)
```

### GAUGE — Schema migration (gauge transformation) ✅

```sql
GAUGE sensors TRANSFORM (
  ADD altitude NUMERIC RANGE 5000 DEFAULT 0,
  RENAME temp TO temperature,
  DROP humidity
);
-- curvature_before: 0.0346
-- curvature_after:  0.0346  ✓ gauge invariant
```

Curvature is INVARIANT under gauge transforms. The fiber reshapes; the connection adjusts; K is preserved. Mathematically guaranteed.

### COLLAPSE — Drop a bundle ✅

```sql
COLLAPSE sensors;              -- requires CONFIRM in interactive mode
```

### LENS — Views (virtual bundles) ✅

```sql
LENS moscow_temps AS
  COVER sensors ON city = 'Moscow' PROJECT (date, temp, wind);

-- Query the lens like any bundle
SECTION moscow_temps AT date=20240104;
CURVATURE moscow_temps ON temp;

-- Materialized lens with auto-refresh
LENS daily_stats AS
  INTEGRATE sensors OVER city MEASURE avg(temp), max(wind), count(*)
  MATERIALIZE REFRESH 60;
```

### DESCRIBE / SHOW ✅

```sql
SHOW BUNDLES;
-- name | records | base_geometry | K | confidence | storage

DESCRIBE sensors;
-- Full schema + per-field curvature + storage mode

DESCRIBE sensors VERBOSE;
-- Adds value distributions, anomaly thresholds, bitmap sizes
```

---

## II. Write Operations

### SECTION — Insert a single record ✅

```sql
SECTION sensors (
  id: 42, city: 'Moscow', region: 'EU',
  date: 20240104, temp: -31.9, humidity: 97.4, wind: 2.2
);
-- status omitted → defaults to 'normal' (silence = agreement)
-- Response:
-- SECTION STORED AT base=42
-- confidence: 0.9772  curvature: 0.0233  anomaly: TRUE (z=5.30)
```

### SECTIONS — Batch insert ✅

```sql
SECTIONS sensors (
  42, 'Moscow', 'EU', 20240104, -31.9, 97.4, 2.2,
  43, 'Moscow', 'EU', 20240105, -30.3, 97.8, 1.8,
  44, 'Moscow', 'EU', 20240106, -22.1, 94.2, 3.1
);
-- Single WAL flush. Deferred field index. Batch curvature update.
```

### SECTION ... UPSERT — Insert or update ✅

```sql
SECTION sensors (id: 42, city: 'Moscow', temp: -28.5) UPSERT;
-- If exists: update. If not: insert. Response shows which.
```

### REDEFINE — Update ✅

```sql
-- Point update
REDEFINE sensors AT id=42 SET (temp: -28.5);

-- Bulk update via cover
REDEFINE sensors ON city = 'Moscow' SET (region: 'RU');
```

### RETRACT — Delete ✅

```sql
RETRACT sensors AT id=42;

RETRACT sensors ON city = 'TestCity';
-- Response includes curvature_delta
```

### RETURNING — Get back what you wrote ✅

```sql
-- Insert with return
SECTION sensors (id: 99, city: 'Test', region: 'XX', date: 20260317,
  temp: 22.0, humidity: 50.0, wind: 3.0)
RETURNING *;
-- Returns: the stored section + confidence + curvature + anomaly flag

-- Return specific fields
SECTION sensors (...) RETURNING id, confidence, curvature;

-- Update with return (old and new)
REDEFINE sensors AT id=42 SET (temp: -28.5)
RETURNING temp AS new_temp, OLD.temp AS old_temp, curvature;

-- Delete with return
RETRACT sensors AT id=42 RETURNING *;

-- Batch with return
SECTIONS sensors (..., ..., ...)
RETURNING id, anomaly;
-- Returns per-record anomaly flags for the batch
```

---

## III. Point Queries — SECTION AT (O(1)) ✅

```sql
SECTION sensors AT id=42;                               -- full section
SECTION sensors AT id=42 PROJECT (city, temp, wind);    -- projected
SECTION events AT user_id=1001, timestamp=1710000000;   -- composite key
EXISTS SECTION sensors AT id=42;                        -- boolean check
```

Every response includes confidence and curvature. Always.

---

## IV. Range Queries — COVER (O(|r|)) ✅

### The ON / WHERE Distinction

The most important design decision in GQL:

- **ON** uses the **field index** (bitmap lookup). O(|bucket|). For indexed categorical fields.
- **WHERE** applies a **fiber predicate** (value scan). O(|scope|). For comparisons on values.

```sql
COVER sensors ON city = 'Moscow';                        -- bitmap: O(366)
COVER sensors WHERE temp < -25;                          -- full scan: O(7320)
COVER sensors ON city = 'Moscow' WHERE temp < -25;       -- bitmap then filter: O(366)
COVER sensors ON region IN ('EU', 'NA') WHERE wind > 8;  -- union then filter
```

### Full Range Query Capabilities

```sql
-- All sections
COVER sensors ALL;

-- Distinct values
COVER sensors DISTINCT city;                    -- field index keys: O(1)

-- Ordering and pagination
COVER sensors ON city = 'Moscow' RANK BY temp ASC;
COVER sensors RANK BY date DESC FIRST 10;
COVER sensors RANK BY date DESC SKIP 10 FIRST 10;

-- Pattern matching
COVER sensors WHERE city MATCHES 'Mos*';        -- wildcard
COVER sensors WHERE city MATCHES '/^[A-M]/';    -- regex

-- Void/defined handling
COVER sensors WHERE pressure VOID;              -- IS NULL
COVER sensors WHERE pressure DEFINED;           -- IS NOT NULL

-- Confidence filter (GQL-only)
COVER sensors ON city = 'Moscow' CONFIDENCE >= 0.95;
-- Only results from neighborhoods with K < 0.0526

-- Multiple index fields
COVER sensors ON region IN ('EU', 'NA') WHERE wind > 8;

-- Computed projection
COVER sensors PROJECT (
  city, temp,
  temp_max - temp_min AS daily_range,
  CLASSIFY temp
    WHEN temp < 0  THEN 'freezing'
    WHEN temp < 15 THEN 'cold'
    WHEN temp < 25 THEN 'mild'
    ELSE 'hot'
  AS category,
  RESOLVE(pressure, 101.3) AS pressure,
  CONFIDENCE() AS conf,
  CURVATURE() AS K
);

-- Set operations
UNION (COVER sensors ON region = 'EU') WITH (COVER sensors ON region = 'NA');
INTERSECT (COVER sensors WHERE temp > 30) WITH (COVER sensors WHERE humidity > 80);
SUBTRACT (COVER sensors ON city = 'Moscow') MINUS (COVER sensors WHERE temp > 0);

-- Subqueries
COVER sensors ON city IN (
  INTEGRATE sensors OVER city
    MEASURE avg(temp) AS avg_t
    HAVING avg_t < 0
    PROJECT city
);

-- CTEs
WITH cold AS (
  INTEGRATE sensors OVER city MEASURE avg(temp) AS avg_t HAVING avg_t < 5
)
COVER sensors ON city IN (cold PROJECT city) WHERE wind > 8;

-- Filter by geometric functions
COVER sensors WHERE ANOMALY() = TRUE;
COVER sensors WHERE Z_SCORE() > 3.0;
COVER sensors WHERE CONFIDENCE() < 0.90;
```

---

## V. Aggregation — INTEGRATE ✅

INTEGRATE = fiber integral ∫_U h dσ. OVER = base partition. MEASURE = integrand.

```sql
-- Per-group (response auto-includes per-group K and confidence)
INTEGRATE sensors OVER city MEASURE avg(temp), count(*);

-- Global (no OVER = integrate over all of B)
INTEGRATE sensors MEASURE avg(temp), stddev(temp), min(temp), max(temp);

-- Multi-level grouping
INTEGRATE sensors OVER region, city MEASURE avg(temp);

-- HAVING (post-aggregation filter)
INTEGRATE sensors OVER city
  MEASURE avg(temp) AS avg_t, count(*)
  HAVING avg_t < 0;

-- HAVING with curvature (GQL-only)
INTEGRATE sensors OVER city
  MEASURE avg(temp)
  HAVING CURVATURE(temp) > 0.02;

-- Restrict scope before aggregation
INTEGRATE sensors OVER city
  MEASURE avg(temp)
  RESTRICT TO (COVER sensors ON region = 'EU');

-- Ordering + pagination on results
INTEGRATE sensors OVER city
  MEASURE avg(temp) AS avg_t
  RANK BY avg_t ASC FIRST 5;
```

**Aggregate Functions:** avg, sum, count, min, max, stddev, variance, median, percentile(f, p), mode

### Conditional Aggregation — FILTER clause ✅

```sql
INTEGRATE sensors OVER city MEASURE
  count(*) AS total,
  count(*) FILTER (WHERE temp < 0) AS freezing_days,
  count(*) FILTER (WHERE temp > 30) AS hot_days,
  avg(temp) FILTER (WHERE status = 'alert') AS alert_avg_temp,
  max(wind) FILTER (WHERE status = 'alert') AS alert_max_wind,
  stddev(temp) FILTER (WHERE DATEPART(date, 'month') IN (12, 1, 2)) AS winter_stddev;

-- FILTER with geometric functions (GQL-only)
INTEGRATE sensors OVER city MEASURE
  avg(temp) FILTER (WHERE CURVATURE(temp) > 0.01) AS volatile_avg,
  count(*) FILTER (WHERE CONFIDENCE() < 0.95) AS low_confidence_count;
```

---

## VI. Window Functions — FIBER Operations ✅

SQL window functions = GQL fiber operations. Named for what they geometrically ARE.

```sql
FIBER RANK sensors OVER city RANK BY date;                              -- ROW_NUMBER
FIBER SUM sensors OF temp OVER city WINDOW date ROWS (UNBOUNDED, 0);   -- cumulative sum
FIBER AVG sensors OF temp OVER city WINDOW date ROWS (-7, 7);          -- 15-day moving avg
FIBER PERCENTRANK sensors OF temp OVER city RANK BY temp;              -- percent rank

-- LAG = parallel transport backward
TRANSPORT sensors OF temp ALONG date SHIFT -1;

-- LEAD = parallel transport forward
TRANSPORT sensors OF temp ALONG date SHIFT +1;

-- Day-over-day delta (inline transport in PROJECT)
COVER sensors PROJECT (
  city, date, temp,
  temp - (TRANSPORT sensors OF temp ALONG date SHIFT -1) AS daily_delta
);
```

**Why TRANSPORT, not LAG:** SQL's LAG is parallel transport along the base space. On a flat connection, transport = array shift. On a curved connection, transport adjusts for Christoffel symbols. GQL names it what it IS.

---

## VII. Joins — PULLBACK (O(|left|)) ✅

```sql
PULLBACK readings ALONG sensor_id ONTO sensors;                              -- inner join
PULLBACK readings ALONG sensor_id ONTO sensors PRESERVE LEFT;                -- left join
PULLBACK orders ALONG customer_id ONTO customers ALONG region ONTO regions;  -- chain
PULLBACK sensors AS s1 ALONG region ONTO sensors AS s2;                      -- self-join
PRODUCT sensors WITH regions;                                                -- cross product
```

### Recursive Joins — ITERATE ✅

```sql
-- WITH RECURSIVE equivalent
WITH RECURSIVE chain AS (
  SECTION employees AT id=1
  UNION
  PULLBACK chain ALONG manager_id ONTO employees
)
COVER chain;

-- Clean syntax: walk a hierarchy
ITERATE employees
  START AT id=1
  STEP ALONG manager_id
  UNTIL VOID;
-- Returns: all reachable sections via iterated pullback along manager_id

-- With depth limit
ITERATE employees
  START AT id=1
  STEP ALONG manager_id
  UNTIL VOID
  MAX DEPTH 10;

-- With accumulation
ITERATE employees
  START AT id=1
  STEP ALONG manager_id
  UNTIL VOID
  ACCUMULATE count(*) AS team_size, sum(salary) AS total_salary;

-- Graph traversal
ITERATE friends
  START AT user_id=42
  STEP ALONG friend_id
  UNTIL DEPTH 3;
-- 3-hop friend-of-friend network
```

---

## VIII. Transactions — ATLAS ✅

An atlas is a collection of coordinate charts. A transaction is an atomic chart transition.

```sql
ATLAS BEGIN;
  SECTION sensors (id: 9001, city: 'Test', ...);
  REDEFINE sensors AT id=42 SET (temp: -28.0);
  RETRACT sensors AT id=43;
ATLAS COMMIT;

ATLAS ROLLBACK;                          -- undo all
ATLAS SAVEPOINT checkpoint1;             -- partial rollback target
ATLAS ROLLBACK TO checkpoint1;

-- Isolation levels
ATLAS BEGIN ISOLATION FLAT;              -- serializable (K=0 in transaction space)
ATLAS BEGIN ISOLATION CURVED;            -- read committed (some K allowed)
```

### Error handling in transactions

```sql
ATLAS BEGIN;
  SECTION orders (id: 1, customer_id: 999, total: 50.0);
  ON ERROR (
    CASE
      WHEN CONSTRAINT_VIOLATION THEN
        NOTIFY 'constraint_error'
      WHEN MORPHISM_VIOLATION THEN
        NOTIFY 'missing_customer'
      ELSE
        ATLAS ROLLBACK
    END
  );
ATLAS COMMIT;

-- Error info
SHOW LAST ERROR;
-- Returns: error_type, message, constraint_name, field, value, suggestion
```

---

## IX. Subscriptions ✅

```sql
SUBSCRIBE sensors ON status = 'alert';                     -- new matching sections
SUBSCRIBE ANOMALIES sensors ON temp;                       -- curvature spikes
SUBSCRIBE CURVATURE sensors ON temp DRIFT > 0.01;         -- K drift events
SUBSCRIBE CONSISTENCY sensors;                             -- H¹ violations
SUBSCRIBE SPECTRAL sensors;                                -- topology changes
SUBSCRIBE PHASE sensors ON temp;                           -- phase transition events
SUBSCRIBE DIVERGENCE sensors THRESHOLD 0.1;                -- distribution drift
UNSUBSCRIBE sub_id;
```

---

## X. Output Control ✅

### EMIT — Wire format

```sql
COVER sensors ON city = 'Moscow' EMIT DHOOM;                              -- default
COVER sensors ON city = 'Moscow' EMIT DHOOM WITH CURVATURE, CONFIDENCE;   -- metadata
COVER sensors ON city = 'Moscow' EMIT DHOOM BARE;                         -- data only
COVER sensors ON city = 'Moscow' EMIT JSON;
COVER sensors ALL EMIT CSV;
```

### EXPLAIN — Geometric query plan

```sql
EXPLAIN COVER sensors ON city = 'Moscow' WHERE temp < -25;
-- PLAN:
--   Step 1: COVER ON city='Moscow' → bitmap lookup → est. 366 sections [O(366)]
--   Step 2: WHERE temp < -25 → fiber scan [O(366)]
--   Total: O(366)
--   Confidence: 0.9772
--   Wire: DHOOM ~54% smaller than JSON
```

### TRANSLATE — SQL compatibility bridge ✅

```sql
TRANSLATE SQL "SELECT city, AVG(temp), COUNT(*)
               FROM sensors WHERE region='EU'
               GROUP BY city HAVING AVG(temp)<10
               ORDER BY AVG(temp) LIMIT 5";
-- Output:
-- INTEGRATE sensors OVER city
--   MEASURE avg(temp) AS avg_t, count(*)
--   RESTRICT TO (COVER sensors ON region = 'EU')
--   HAVING avg_t < 10
--   RANK BY avg_t ASC FIRST 5;
```

---

## XI. Geometric Operations (GQL-Only)

These are impossible in SQL. They exist because the data lives on a fiber bundle.

### CURVATURE — Local data variability ✅

```sql
CURVATURE sensors ON temp;                                    -- global K for temp
CURVATURE sensors ON temp BY city;                            -- per-city K
CURVATURE sensors ON temp, wind, humidity;                    -- multi-field + scalar
CURVATURE sensors ON temp WITHIN (COVER sensors ON region = 'EU');  -- scoped
```

### RICCI — Curvature between field pairs ✅

```sql
RICCI sensors ON temp, humidity;
-- Ricci curvature between temperature and humidity fibers
-- Measures how correlated their variability is
-- High Ricci = when temp is volatile, humidity is too
-- Low Ricci = independent variability

RICCI sensors ON temp, wind BY city;
-- Per-city Ricci curvature between temp and wind
```

### SECTIONAL — Curvature on 2-planes

```sql
SECTIONAL sensors ON temp, humidity;
-- Sectional curvature on the (temp, humidity) 2-plane
-- Positive = data clusters in ellipses. Negative = data spreads in saddles.

SECTIONAL sensors ON temp, humidity BY region;
```

### SCALAR — Total curvature (trace of Ricci)

```sql
SCALAR sensors;
-- Scalar curvature = sum of all sectional curvatures. Single number.

SCALAR sensors BY city;
```

### DEVIATION — Geodesic deviation

```sql
DEVIATION sensors FROM id=42 ALONG date;
-- How does section 42 diverge from neighbors over time?
-- Large deviation = this record is becoming more anomalous

DEVIATION sensors ON temp BY city;
-- Per-city: how are temperature profiles diverging?
```

### TREND — Curvature-based trend detection

```sql
TREND sensors ON temp BY city;
-- dK/dt — is curvature increasing or decreasing over time?

TREND sensors ON temp BY city WINDOW date ROWS (-30, 0);
-- 30-day rolling curvature trend
```

### CONFIDENCE — Query trust level ✅

```sql
CONFIDENCE sensors;                       -- global: 0.9665
CONFIDENCE sensors ON city = 'Moscow';    -- cover-specific: 0.9772
```

### CAPACITY — Query capacity (C = τ/K)

```sql
CAPACITY sensors;
-- C = τ/K. ~how many independent queries can be answered unambiguously per unit τ.

CAPACITY sensors BY city;
CAPACITY sensors TOLERANCE 1.0;
```

### SPECTRAL — Index connectivity ✅

```sql
SPECTRAL sensors;                -- λ₁, components, diameter, mixing_time
SPECTRAL sensors ON region;      -- per-index spectral analysis
SPECTRAL sensors FULL;           -- full eigenvalue spectrum
```

### BOTTLENECK — Find weakest connectivity (Cheeger inequality)

```sql
BOTTLENECK sensors;
-- Finds the minimum cut in the field index graph
-- Returns: bottleneck_set, conductance h, Cheeger bound: λ₁/2 ≤ h ≤ √(2λ₁)

BOTTLENECK sensors ON city;
```

### CLUSTER — Spectral clustering via Fiedler vector

```sql
CLUSTER sensors INTO 4;
-- Pure spectral clustering. No k-means. No embeddings. No Euclidean distance.

CLUSTER sensors INTO 4 BY temp, humidity, wind;

CLUSTER sensors INTO AUTO;
-- Auto-detect cluster count from spectral gap (near-zero eigenvalues)
```

### MIXING — Mixing time estimation

```sql
MIXING sensors;
-- mixing_time = O(1/λ₁)
-- How many hops to reach equilibrium on the field index graph?

MIXING sensors ON region;
```

### CONDUCTANCE — Graph conductance

```sql
CONDUCTANCE sensors;
-- Low conductance = strong internal clusters with weak bridges
-- High conductance = well-mixed data

CONDUCTANCE sensors ON city;
```

### LAPLACIAN — Hodge Laplacian on data forms

```sql
LAPLACIAN sensors;
-- The spectrum IS the topology: zero eigenvalues = components,
-- gap to first nonzero = connectivity strength

LAPLACIAN sensors ON city TOP 5;
```

### HOLONOMY — Loop consistency

```sql
HOLONOMY sensors AROUND (city, region);
-- holonomy=0 → consistent. holonomy>0 → drift with locations.
```

### WILSON — Wilson loop (holonomy along explicit path)

```sql
WILSON sensors AROUND (city='Moscow', region='EU', city='London', region='EU');
-- Parallel transport around a specific path in the index graph
-- Wilson loop ≠ 0 means the data is not self-consistent along this path

WILSON sensors AROUND (date=20240101, date=20240201, date=20240301, date=20240101);
-- Temporal loop: is the data self-consistent across time?
```

### CONSISTENCY — Čech cohomology ✅

```sql
CONSISTENCY sensors;              -- h1: 0 (consistent)
CONSISTENCY sensors REPAIR;       -- attempt cocycle resolution
```

### BETTI — Topological invariants ✅

```sql
BETTI sensors;
-- β₀ = connected components
-- β₁ = independent loops (1-cycles)
-- β₂ = enclosed voids (2-cycles)
-- β₁ > 0 means there are non-contractible loops in your data topology

BETTI sensors ON city, region;
```

### EULER — Euler characteristic

```sql
EULER sensors;
-- χ = β₀ - β₁ + β₂
-- If χ changes after a batch insert, the TOPOLOGY of your data changed
```

### COCYCLE — Extract specific Čech cocycles

```sql
COCYCLE sensors ON (city, region);
-- Returns all nonzero cocycles → each identifies a specific inconsistency

COBOUNDARY sensors ON cocycle_id;
-- Is this cocycle exact (removable by local adjustment)?
-- If no: topological inconsistency — structural, not fixable locally.
```

### TRIVIALIZE — Check bundle triviality

```sql
TRIVIALIZE sensors;
-- TRIVIAL → E ≅ B × F everywhere → can flatten to CSV without loss
-- NON-TRIVIAL → bundle has intrinsic structure that NO flat schema can capture
```

### CHARACTERISTIC — Characteristic classes

```sql
CHARACTERISTIC sensors;
-- c₁ = 0 → bundle is topologically trivializable
-- c₁ ≠ 0 → bundle has intrinsic twist that cannot be removed
```

### ENTROPY — Shannon entropy ✅

```sql
ENTROPY sensors;
-- H = -Σ p(x) log p(x)

ENTROPY sensors ON status;
ENTROPY sensors ON temp BY city;
```

### DIVERGENCE — KL divergence between covers

```sql
DIVERGENCE (COVER sensors ON region = 'EU')
  FROM (COVER sensors ON region = 'AS')
  ON temp;
-- D_KL = 0 → identical distributions. D_KL >> 0 → very different.

DIVERGENCE (COVER sensors ON city = 'Moscow')
  FROM (COVER sensors ON city = 'Singapore')
  ON temp, humidity, wind;
```

### FISHER — Fisher information metric

```sql
FISHER sensors ON temp, humidity;
-- Natural metric on the statistical manifold of the data
-- High Fisher = small changes in temp strongly predict humidity changes

FISHER sensors ON temp, humidity BY city;
```

### MUTUAL — Mutual information between fields

```sql
MUTUAL sensors ON temp WITH humidity;
-- I(temp; humidity) = H(temp) + H(humidity) - H(temp, humidity)
-- I = 0 → independent. I >> 0 → strongly coupled.

MUTUAL sensors ON temp WITH wind BY city;
```

### FREEENERGY — Helmholtz free energy ✅

```sql
FREEENERGY sensors TOLERANCE 0.1;
-- F = -τ log Z(β, p)
-- Low F = data is in a "low energy" state (well-organized)
-- ΔF between snapshots measures how much the data's organization changed

FREEENERGY sensors BY city TOLERANCE 1.0;
```

### TEMPERATURE — Effective temperature per neighborhood

```sql
TEMPERATURE sensors;
-- Effective τ at each neighborhood, computed from local fluctuations
-- High temperature = high local variance = approximate queries are safe

TEMPERATURE sensors BY city;
```

### PHASE — Phase transition detection

```sql
PHASE sensors ON temp;
-- Detect discontinuities in curvature K as function of base parameters

PHASE sensors ON temp ALONG date;
-- "Phase transition detected at date=20240315: K jumped from 0.01 to 0.04"

PHASE sensors ON temp ALONG date BY city;
```

### CRITICAL — Find critical points

```sql
CRITICAL sensors ON temp;
-- Points where ∂K/∂(base) = 0
-- Returns: base_points, K_value, type (minimum / maximum / saddle)

CRITICAL sensors ON temp BY city;
```

### PARTITION — Approximate queries (statistical mechanics)

```sql
PARTITION sensors AT city = 'Moscow' TOLERANCE 0.5;    -- fuzzy match
PARTITION sensors WHERE temp NEAR 22.0 TOLERANCE 2.0;  -- neighborhood

CALIBRATE sensors TOLERANCE 0.01;                       -- set default τ
```

### TRANSPORT — Parallel transport ✅

```sql
TRANSPORT sensors OF temp FROM id=42 TO id=100;          -- point-to-point
TRANSPORT sensors OF temp ALONG date SHIFT -1;           -- LAG (flat)
TRANSPORT sensors OF temp ALONG date SHIFT +1;           -- LEAD (flat)
```

### FLOW — Renormalization group (C-theorem)

```sql
FLOW sensors COARSEN BY region LEVELS 3;
-- Returns entropy per level. C-theorem: non-increasing.
-- Violation = structural anomaly at that scale.

FLOW sensors COARSEN BY city LEVELS 5 SHOW ENTROPY;
```

### GEODESIC — Shortest path ✅

```sql
GEODESIC sensors FROM id=42 TO id=100;
GEODESIC sensors FROM city='Moscow' TO city='Singapore';
```

### SIMILAR — Find geometrically similar sections

```sql
SIMILAR sensors TO id=42 WITHIN 0.1;
-- Uses fiber distance (not Euclidean!) — normalized by field ranges

SIMILAR sensors TO id=42 WITHIN 0.1 ON temp, wind;

SIMILAR sensors TO id=42 TOP 10;
-- 10 nearest geodesic neighbors
```

### CORRELATE — Geometric correlation via connection

```sql
CORRELATE sensors ON temp WITH wind;
-- Geometric correlation on the MANIFOLD, not in flat Euclidean space

CORRELATE sensors ON temp WITH wind BY city;
```

### SEGMENT — Geometric segmentation

```sql
SEGMENT sensors INTO 4 BY temp, humidity, wind;
-- Curvature-based clustering: natural segments where K is locally minimal

SEGMENT sensors INTO AUTO;
-- Number of local K minima = number of natural segments
```

### OUTLIER — Flag outliers by curvature

```sql
OUTLIER sensors ON temp SIGMA 3;
-- Curvature-normalized thresholds (adaptive, not global)
-- High-K neighborhoods → wider thresholds. Low-K → tighter thresholds.

OUTLIER sensors ON temp, wind SIGMA 2.5 BY city;
```

### PROFILE — Full geometric data profile ✅

```sql
PROFILE sensors;
-- Per-field: K, confidence, entropy, range, distinct_count, void_rate
-- Per-pair: Ricci curvature, Fisher information, mutual information
-- Global: scalar curvature, spectral gap, Betti numbers, Euler characteristic
-- Storage: base geometry, estimated compression via DHOOM
-- Anomalies: top outliers with Z-scores
-- One call. Everything you need to understand your data geometrically.
```

### DOUBLECOVER — S + d² = 1 verification

```sql
DOUBLECOVER sensors ON (COVER sensors ON city = 'Moscow');
-- Returns: S=0.998, d=0.045, S+d²=1.000 ✓

RECALL sensors ON (COVER sensors WHERE temp < -25);
-- What fraction of relevant records were returned?

COMPLETENESS sensors ON (COVER sensors WHERE temp < -25);
-- Full Double Cover check: S + d² = 1?
```

### RESTRICT — Sheaf restriction ⚠️

```sql
RESTRICT sensors TO (COVER sensors ON region = 'EU') AS eu_sensors;
```

### GLUE — Explicit sheaf gluing ⚠️

```sql
GLUE (COVER sensors ON region = 'EU') WITH (COVER sensors ON region = 'NA') CHECK OVERLAP;
```

### PREDICT — Curvature forecasting ✅

```sql
PREDICT sensors ON temp BY city;
PREDICT sensors ON temp BY city TRAIN BEFORE date=20240901 TEST AFTER date=20240901;
```

### DIFF — Bundle comparison

```sql
DIFF sensors_v1 AGAINST sensors_v2;
DIFF sensors AT SNAPSHOT 'yesterday' AGAINST sensors;
```

### SNAPSHOT — Point-in-time capture ✅

```sql
SNAPSHOT sensors AS 'pre_migration';
```

### HEALTH — Full diagnostic ✅

```sql
HEALTH sensors;
```

### GAUGE VERIFY — Post-migration invariance check

```sql
GAUGE VERIFY sensors;
-- Returns: K_before, K_after, delta, status (INVARIANT / VIOLATED)
-- If VIOLATED: identifies which transformation broke invariance
```

---

## XII. Bulk Import/Export ✅

### INGEST — Bulk import from files

```sql
-- CSV import
INGEST sensors FROM 'data.csv' FORMAT CSV;
INGEST sensors FROM 'data.csv' FORMAT CSV
  HEADER TRUE
  DELIMITER ','
  NULL_VALUE 'NA'
  SKIP 1;

-- JSON import
INGEST sensors FROM 'data.json' FORMAT JSON;
INGEST sensors FROM 'data.jsonl' FORMAT JSONL;

-- DHOOM (native)
INGEST sensors FROM 'data.dhoom' FORMAT DHOOM;

-- SQL dump
INGEST sensors FROM 'dump.sql' FORMAT SQL;

-- From stdin
INGEST sensors FROM STDIN FORMAT CSV;
INGEST sensors FROM STDIN FORMAT JSONL;

-- From URL
INGEST sensors FROM 'https://api.nasa.gov/power/...' FORMAT JSON
  PATH 'properties.parameter';

-- With transformation on ingest
INGEST sensors FROM 'raw.csv' FORMAT CSV
  MAP (
    temperature -> temp,
    relative_humidity -> humidity,
    wind_speed -> wind
  )
  FILTER (WHERE temp DEFINED AND humidity DEFINED);

-- Response:
-- INGEST COMPLETE: 7320 sections stored
-- rejected: 12 (constraint violations)
-- anomalies_detected: 47
-- curvature_after: 0.0346
-- elapsed: 28ms (256K sections/sec)
```

### EMIT TO — Bulk export to files ✅

```sql
-- Full export
COVER sensors ALL EMIT CSV TO 'export.csv';
COVER sensors ALL EMIT JSON TO 'export.json';
COVER sensors ALL EMIT DHOOM TO 'export.dhoom';

-- Filtered export
COVER sensors ON city = 'Moscow' WHERE temp < -25
  EMIT CSV TO 'moscow_cold.csv';

-- Export with geometric metadata
COVER sensors ALL
  EMIT CSV TO 'export_with_meta.csv'
  WITH CURVATURE, CONFIDENCE;

-- Export to stdout
COVER sensors ON region = 'EU' EMIT CSV TO STDOUT;
COVER sensors ON region = 'EU' EMIT JSONL TO STDOUT;

-- Export aggregation results
INTEGRATE sensors OVER city MEASURE avg(temp), count(*)
  EMIT CSV TO 'city_stats.csv';

-- DHOOM export = complete geometric backup (schema + data + metadata)
COVER sensors ALL EMIT DHOOM TO 'full_backup.dhoom';
```

---

## XIII. Generate Series & Fill

### GENERATE BASE ✅

```sql
GENERATE BASE sensors
  FROM date=20240101 TO date=20241231 STEP 1;
-- Creates section stubs at every base point. Sections with no data have VOID fibers.

-- Pre-populate specific fields
GENERATE BASE sensors
  FROM date=20240101 TO date=20241231 STEP 1
  WITH (city: 'Moscow', region: 'EU');

-- Detect gaps (dates with no data)
COVER sensors ON city = 'Moscow' WHERE temp VOID;

-- Integer sequence (for testing / synthetic data)
GENERATE BASE test_bundle
  FROM id=1 TO id=1000000 STEP 1;
```

### FILL — Gap filling

```sql
-- Fill via parallel transport (curvature-aware)
FILL sensors ON date USING TRANSPORT;

-- Linear interpolation
FILL sensors ON date USING INTERPOLATE LINEAR;

-- Constant fill
FILL sensors ON date
  USING CONSTANT (temp: 0.0, humidity: 50.0);
```

---

## XIV. Copy Between Bundles — TRANSPLANT

```sql
-- Copy all sections
TRANSPLANT sensors INTO sensors_archive;

-- Copy with filter
TRANSPLANT sensors INTO sensors_archive
  WHERE date < 20240601;

-- Copy with field rename
TRANSPLANT sensors INTO sensors_v2
  MAP (
    temp -> temperature,
    humidity -> relative_humidity
  );

-- Move (copy + retract from source)
TRANSPLANT sensors INTO sensors_archive
  WHERE date < 20240101
  RETRACT SOURCE;
```

---

## XV. Built-in Functions

### Math Functions

```sql
COVER sensors PROJECT (
  ABS(temp) AS abs_temp,
  ROUND(temp, 1) AS rounded,
  CEIL(wind) AS wind_ceil,
  FLOOR(humidity) AS humid_floor,
  TRUNC(temp, 0) AS truncated,
  POWER(temp, 2) AS temp_squared,
  SQRT(ABS(temp)) AS temp_root,
  CBRT(ABS(temp)) AS cube_root,
  LOG(pressure) AS log_p,
  LOG10(pressure) AS log10_p,
  LOG2(pressure) AS log2_p,
  EXP(temp / 100) AS exp_scaled,
  MOD(id, 3) AS bucket,
  SIGN(temp) AS direction,
  GREATEST(temp, 0) AS temp_floor_zero,
  LEAST(temp, 40) AS temp_cap,
  PI() AS pi_val,
  RANDOM() AS rand
);
```

### String Functions

```sql
COVER sensors PROJECT (
  UPPER(city) AS city_upper,
  LOWER(city) AS city_lower,
  INITCAP(city) AS city_title,
  LENGTH(city) AS name_len,
  SUBSTR(city, 1, 3) AS prefix,
  LEFT(city, 3) AS left3,
  RIGHT(city, 3) AS right3,
  CONCAT(city, '-', region) AS label,
  CONCAT_WS(', ', city, region) AS full,
  REPLACE(status, 'normal', 'ok') AS stat,
  TRIM(city) AS trimmed,
  LTRIM(city) AS left_trimmed,
  RTRIM(city) AS right_trimmed,
  LPAD(id, 5, '0') AS padded_id,
  RPAD(city, 15, '.') AS dotted,
  REVERSE(city) AS reversed,
  REPEAT('*', 3) AS stars,
  POSITION('ow' IN city) AS pos,
  SPLIT(city, '_') AS parts,
  MD5(city) AS checksum
);
```

### Date/Time Functions

```sql
COVER events PROJECT (
  NOW() AS current_time,
  TODAY() AS current_date,
  EPOCH(timestamp) AS unix_ts,
  FROM_EPOCH(1710000000) AS ts,
  DATEPART(timestamp, 'year') AS yr,
  DATEPART(timestamp, 'month') AS mo,
  DATEPART(timestamp, 'day') AS dy,
  DATEPART(timestamp, 'hour') AS hr,
  DATEPART(timestamp, 'dow') AS day_of_week,  -- 0=Sun, 6=Sat
  DATEPART(timestamp, 'doy') AS day_of_year,
  DATETRUNC(timestamp, 'month') AS month_start,
  DATETRUNC(timestamp, 'day') AS day_start,
  DATEADD(timestamp, 7, 'day') AS next_week,
  DATEADD(timestamp, -1, 'month') AS last_month,
  DATEDIFF(end_ts, start_ts, 'hours') AS duration_hrs,
  DATEDIFF(end_ts, start_ts, 'days') AS duration_days,
  FORMAT_DATE(timestamp, 'YYYY-MM-DD') AS formatted,
  PARSE_DATE('2024-01-04', 'YYYY-MM-DD') AS parsed
);
```

### Type Casting

```sql
COVER sensors PROJECT (
  CAST(temp AS TEXT) AS temp_str,
  CAST('42' AS NUMERIC) AS num,
  CAST(temp AS INTEGER) AS temp_int,
  CAST(1 AS BOOLEAN) AS flag
);

-- Shorthand
COVER sensors PROJECT (
  temp::TEXT AS temp_str,
  '42'::NUMERIC AS num
);
```

### Conditional Expressions

```sql
COVER sensors PROJECT (
  -- CLASSIFY (extended CASE/WHEN)
  CLASSIFY temp
    WHEN temp < -20 THEN 'extreme_cold'
    WHEN temp < 0   THEN 'freezing'
    WHEN temp < 15  THEN 'cold'
    WHEN temp < 25  THEN 'mild'
    WHEN temp < 35  THEN 'warm'
    ELSE 'extreme_heat'
  AS category,

  -- IF (two-branch)
  IF(temp < 0, 'below_zero', 'above_zero') AS freezing,

  -- RESOLVE (COALESCE)
  RESOLVE(pressure, 101.3) AS pressure_safe,

  -- VOIDIF (NULLIF equivalent)
  VOIDIF(wind, 0) AS wind_nonzero,

  -- GREATEST / LEAST
  GREATEST(temp, temp_min, 0) AS warmest,
  LEAST(temp, temp_max, 40) AS coolest
);
```

### Array/Collection Functions

```sql
-- Array construction
COVER sensors PROJECT (
  ARRAY(temp, humidity, wind) AS measurements,
  ARRAY_LENGTH(ARRAY(temp, humidity, wind)) AS n_measures
);

-- Array aggregation
INTEGRATE sensors OVER city MEASURE
  ARRAY_AGG(temp) AS all_temps,
  ARRAY_AGG(temp RANK BY date ASC) AS ordered_temps;

-- JSON construction
COVER sensors PROJECT (
  JSON_BUILD(
    'city', city,
    'temp', temp,
    'confidence', CONFIDENCE()
  ) AS record_json
);
```

### System Functions (usable in PROJECT or WHERE)

```sql
NOW()                -- current timestamp
TODAY()              -- current date
CURRENT_ROLE()       -- current role name
CURRENT_BUNDLE()     -- last accessed bundle
VERSION()            -- GIGI version string

-- Geometric state functions
CONFIDENCE()         -- confidence at current base point
CURVATURE()          -- K at current base point
ANOMALY()            -- boolean anomaly flag
Z_SCORE()            -- z-score at current base point
CAPACITY()           -- C = τ/K at current base point
DRIFT(field)         -- recent curvature drift (used in triggers)
DIFF(OLD, NEW)       -- field diff (used in triggers)
```

---

## XVI. Access Control ❌ (501 — Not Yet Implemented)

The access control syntax is fully specified and parsed. All operations return HTTP 501.

### WEAVE — Create roles

```sql
WEAVE ROLE analyst;
WEAVE ROLE admin;
WEAVE ROLE api_user PASSWORD 'secure_hash_here';
WEAVE ROLE admin INHERITS analyst;
WEAVE ROLE superadmin SUPERWEAVE;

UNWEAVE ROLE analyst;
SHOW ROLES;
```

### GRANT / REVOKE — Permissions

```sql
GRANT SECTION ON sensors TO admin;
GRANT COVER ON sensors TO analyst;
GRANT REDEFINE ON sensors TO admin;
GRANT RETRACT ON sensors TO admin;
GRANT CURVATURE ON sensors TO analyst;
GRANT SPECTRAL ON sensors TO analyst;
GRANT HEALTH ON sensors TO analyst;
GRANT ALL ON sensors TO admin;
GRANT COVER ON ALL BUNDLES TO readonly;

REVOKE RETRACT ON sensors FROM analyst;
REVOKE ALL ON sensors FROM readonly;

GRANT BUNDLE TO admin;
GRANT GAUGE ON sensors TO admin;
GRANT COLLAPSE ON sensors TO admin;
```

### POLICY — Row-level security via geometric restriction

```sql
-- Analyst can only see EU data
POLICY analyst_eu ON sensors
  FOR COVER
  RESTRICT TO (COVER sensors ON region = 'EU')
  TO analyst;

-- Write restriction
POLICY admin_eu ON sensors
  FOR SECTION, REDEFINE, RETRACT
  RESTRICT TO (COVER sensors ON region = 'EU')
  TO admin;

-- Curvature-based access (GQL-only — no Postgres equivalent)
POLICY high_confidence ON sensors
  FOR COVER
  RESTRICT TO (COVER sensors CONFIDENCE >= 0.95)
  TO analyst;
-- Analyst can only see records from low-curvature regions. The geometry determines access.

DROP POLICY analyst_eu ON sensors;
SHOW POLICIES ON sensors;
```

### AUDIT — Audit trail

```sql
AUDIT sensors ON;
AUDIT sensors ON SECTION, REDEFINE, RETRACT, GAUGE;
AUDIT SHOW sensors;
AUDIT SHOW sensors SINCE '2024-01-01';
AUDIT SHOW sensors ROLE admin;
AUDIT sensors OFF;
```

---

## XVII. Constraints ✅

### Fiber Value Constraints (inline)

```sql
BUNDLE orders
  BASE (id NUMERIC AUTO)
  FIBER (
    customer_id NUMERIC REQUIRED,
    total NUMERIC REQUIRED CHECK (total > 0),
    qty NUMERIC CHECK (qty >= 1 AND qty <= 10000),
    status CATEGORICAL IN ('pending', 'shipped', 'delivered', 'cancelled'),
    email TEXT CHECK (email MATCHES '/^[^@]+@[^@]+\.[^@]+$/'),
    discount NUMERIC CHECK (discount >= 0 AND discount <= 1),
    customer_id NUMERIC REFERENCES customers(id),
    region_id NUMERIC REFERENCES regions(id)
  );
```

### CONSTRAINT block — Cross-field and composite constraints

```sql
BUNDLE orders
  BASE (id NUMERIC AUTO)
  FIBER (...)
  CONSTRAINT (
    UNIQUE (customer_id, date),
    CHECK (ship_date >= order_date),
    CHECK (discount_total <= total),
    CHECK (qty * unit_price = line_total),
    CHECK (status = 'shipped' IMPLIES ship_date DEFINED),
    CHECK (status = 'cancelled' IMPLIES cancel_reason DEFINED),
    MORPHISM customer_id -> customers(id),
    MORPHISM region_id -> regions(id),
    EXCLUDE (customer_id WITH =, daterange WITH OVERLAPS)
  );
```

### Adding/Dropping constraints after creation

```sql
GAUGE orders CONSTRAIN (
  ADD CHECK (total > 0) AS positive_total,
  ADD UNIQUE (customer_id, date) AS unique_order,
  ADD MORPHISM region_id -> regions(id)
);

GAUGE orders UNCONSTRAIN positive_total;
SHOW CONSTRAINTS ON orders;
```

---

## XVIII. Triggers ⚠️ (Parsed — Not Yet Implemented)

The trigger syntax is fully specified. The `TriggerManager` is built in the engine. Nothing is wired to HTTP yet.

```sql
-- Trigger on insert
ON SECTION sensors
  EXECUTE NOTIFY 'new_reading';

-- Conditional trigger
ON SECTION sensors
  WHERE temp < -30
  EXECUTE ALERT 'extreme_cold';

-- Trigger with action
ON SECTION sensors
  WHERE status = 'alert'
  EXECUTE (
    SECTION alerts (
      source: 'sensors',
      record_id: NEW.id,
      message: CONCAT('Alert from ', NEW.city, ': temp=', NEW.temp),
      timestamp: NOW()
    )
  );

-- Trigger on curvature drift
ON CURVATURE sensors
  WHERE DRIFT(temp) > 0.01
  EXECUTE NOTIFY 'volatility_shift';

-- Auto-repair on consistency violation
ON CONSISTENCY sensors
  WHERE h1 > 0
  EXECUTE REPAIR;

-- Cascade delete
ON RETRACT customers
  CASCADE RETRACT orders WHERE customer_id = OLD.id;

-- Cascade via morphism
ON RETRACT customers
  CASCADE MORPHISM;

-- Before-trigger (validation)
BEFORE SECTION orders
  CHECK (EXISTS SECTION customers AT id = NEW.customer_id)
  ON FAIL REJECT 'Customer does not exist';

-- After-trigger (logging)
AFTER REDEFINE sensors
  EXECUTE (
    SECTION audit_log (
      bundle: 'sensors',
      action: 'REDEFINE',
      record_id: OLD.id,
      changed_fields: DIFF(OLD, NEW),
      timestamp: NOW(),
      role: CURRENT_ROLE()
    )
  );

DROP TRIGGER extreme_cold ON sensors;
SHOW TRIGGERS ON sensors;
```

---

## XIX. Prepared Statements ❌ (501 — Not Yet Implemented)

```sql
PREPARE point_lookup AS
  SECTION sensors AT id = $1;

EXECUTE point_lookup (42);

PREPARE city_query AS
  COVER sensors ON city = $1 WHERE temp < $2;

EXECUTE city_query ('Moscow', -25);

PREPARE region_stats AS
  INTEGRATE sensors OVER city
    MEASURE avg(temp), count(*)
    RESTRICT TO (COVER sensors ON region = $1);

EXECUTE region_stats ('EU');

DEALLOCATE point_lookup;
DEALLOCATE ALL;
SHOW PREPARED;
```

---

## XX. Backup & Restore ❌ (501 — Not Yet Implemented)

```sql
BACKUP sensors TO 'sensors_2024.gigi';
BACKUP ALL TO 'full_backup_2024.gigi';
BACKUP sensors TO 'sensors.gigi.zst' COMPRESS;
BACKUP sensors TO 'sensors_incr.gigi' INCREMENTAL SINCE '2024-06-01';

RESTORE sensors FROM 'sensors_2024.gigi';
RESTORE sensors FROM 'sensors_2024.gigi' AT SNAPSHOT 'pre_migration';
RESTORE sensors FROM 'sensors_2024.gigi' AS sensors_restored;

VERIFY BACKUP 'sensors_2024.gigi';
-- Returns: sections, schema hash, curvature at backup time, CRC check

SHOW BACKUPS;
```

---

## XXI. Maintenance Operations ⚠️ (Parsed — Not Yet Implemented)

```sql
-- WAL compaction
COMPACT sensors;
COMPACT sensors ANALYZE;

-- Refresh geometric statistics
ANALYZE sensors;
ANALYZE sensors ON temp;
ANALYZE sensors FULL;

-- Storage report ✅ (this one works)
STORAGE sensors;
-- base_geometry: SEQUENTIAL
-- sections: 7320
-- memory_usage: 4.2 MB
-- wal_size: 1.8 MB
-- compression_ratio: 3.2x vs raw JSON

-- Rebuild field index
REBUILD INDEX sensors;
REBUILD INDEX sensors ON city;

-- Vacuum (reclaim space from retracted sections)
VACUUM sensors;
VACUUM sensors FULL;

-- Check integrity
CHECK sensors;
-- Verifies WAL CRC, field index consistency, GIGI hash uniqueness,
-- constraint satisfaction, curvature cache accuracy

-- Repair
REPAIR sensors;
```

---

## XXII. Session & Connection Management ❌ (501 — Not Yet Implemented)

```sql
SET TOLERANCE 0.01;
SET EMIT DHOOM;
SET ANOMALY_THRESHOLD 2.5;
SET MAX_SECTIONS 1000000;

SHOW SETTINGS;
SHOW SESSION;
SHOW CURRENT ROLE;

RESET TOLERANCE;
RESET ALL;
```

---

## XXIII. Information Schema ✅

```sql
SHOW BUNDLES;
SHOW BUNDLES VERBOSE;

DESCRIBE sensors;
DESCRIBE sensors VERBOSE;

SHOW FIELDS ON sensors;
-- name, type, modifiers, K, distinct_count, void_rate, index_size

SHOW INDEXES ON sensors;
-- field, distinct_values, bucket_sizes, bitmap_memory

SHOW CONSTRAINTS ON orders;
SHOW MORPHISMS ON sensors;
-- field, target_bundle, target_field, referential_integrity_status

SHOW TRIGGERS ON sensors;
SHOW POLICIES ON sensors;

SHOW STATISTICS ON sensors;
-- per-field mean, stddev, min, max, K, entropy, distinct, void_rate

SHOW GEOMETRY ON sensors;
-- scalar_K, spectral_gap, betti_numbers, euler_characteristic,
-- storage_mode, base_geometry, double_cover_status
```

---

## XXIV. Comments ❌ (501 — Not Yet Implemented)

```sql
COMMENT ON BUNDLE sensors IS 'NASA POWER atmospheric data, 20 cities, 2024';
COMMENT ON FIELD sensors.temp IS 'Temperature at 2 meters (°C), RANGE 80 for K normalization';
COMMENT ON CONSTRAINT positive_total ON orders IS 'Prevents negative order totals';
SHOW COMMENTS ON sensors;
```

---

## XXV. Formal Grammar (EBNF)

```
program       = statement ( ";" statement )* ";"?

statement     = bundle_stmt | gauge_stmt | collapse_stmt | lens_stmt
              | section_stmt | sections_stmt | redefine_stmt | retract_stmt
              | point_query | cover_stmt | integrate_stmt | pullback_stmt
              | fiber_window | curvature_stmt | confidence_stmt | spectral_stmt
              | holonomy_stmt | consistency_stmt | partition_stmt | calibrate_stmt
              | transport_stmt | flow_stmt | geodesic_stmt | restrict_stmt
              | glue_stmt | predict_stmt | diff_stmt | snapshot_stmt
              | health_stmt | atlas_stmt | subscribe_stmt | explain_stmt
              | translate_stmt | show_stmt | describe_stmt
              | set_op | product_stmt | with_stmt

bundle_stmt   = "BUNDLE" name "BASE" "(" field_defs ")" "FIBER" "(" field_defs ")"
                ( "OPTIONS" "(" option_list ")" )?
gauge_stmt    = "GAUGE" name "TRANSFORM" "(" alterations ")"
collapse_stmt = "COLLAPSE" name
lens_stmt     = "LENS" name "AS" query ( "MATERIALIZE" ( "REFRESH" number )? )?
section_stmt  = "SECTION" name "(" kv_pairs ")" ( "UPSERT" )? ( "RETURNING" proj_list )?
sections_stmt = "SECTIONS" name "(" value_rows ")" ( "RETURNING" proj_list )?
redefine_stmt = "REDEFINE" name ( "AT" key_preds | on_clause ) "SET" "(" kv_pairs ")"
                ( "RETURNING" proj_list )?
retract_stmt  = "RETRACT" name ( "AT" key_preds | on_clause ) ( "RETURNING" proj_list )?
point_query   = ( "EXISTS" )? "SECTION" name "AT" key_preds ( project )?
cover_stmt    = "COVER" name ( "ALL" | on_clause? where_clause? )
                distinct? rank? pagination? confidence_filter? emit?
integrate_stmt = "INTEGRATE" name ( "OVER" field_list )? "MEASURE" agg_list
                 restrict? having? rank? pagination?
fiber_window  = "FIBER" agg_name name "OF" field "OVER" field
                ( "WINDOW" field "ROWS" "(" bound "," bound ")" )?
              | "FIBER" ( "RANK" | "PERCENTRANK" ) name "OF"? field? "OVER" field
                "RANK" "BY" field
pullback_stmt = "PULLBACK" name ("AS" alias)? ("ALONG" field "ONTO" name ("AS" alias)?)+
                ("PRESERVE" "LEFT")?
transport_stmt = "TRANSPORT" name "OF" field
                 ("FROM" key_preds "TO" key_preds | "ALONG" field "SHIFT" signed_num)
product_stmt  = "PRODUCT" name "WITH" name
set_op        = ("UNION"|"INTERSECT") "(" query ")" "WITH" "(" query ")"
              | "SUBTRACT" "(" query ")" "MINUS" "(" query ")"
with_stmt     = "WITH" ("RECURSIVE")? name "AS" "(" query ")" statement
iterate_stmt  = "ITERATE" name "START" "AT" key_preds "STEP" "ALONG" field
                "UNTIL" ("VOID" | "DEPTH" number) ("MAX" "DEPTH" number)?
                ("ACCUMULATE" agg_list)?
curvature_stmt  = "CURVATURE" name "ON" field_list ("BY" field)? ("WITHIN" "(" query ")")?
confidence_stmt = "CONFIDENCE" name (on_clause)?
spectral_stmt   = "SPECTRAL" name ("ON" field)? ("FULL")?
holonomy_stmt   = "HOLONOMY" name "AROUND" "(" field_list ")"
consistency_stmt = "CONSISTENCY" name ("REPAIR")?
partition_stmt  = "PARTITION" name ("AT" key_preds | "WHERE" field "NEAR" value) "TOLERANCE" num
calibrate_stmt  = "CALIBRATE" name "TOLERANCE" number
flow_stmt       = "FLOW" name "COARSEN" "BY" field "LEVELS" number
                  ("MEASURE" agg_list)? ("SHOW" "ENTROPY")?
geodesic_stmt   = "GEODESIC" name "FROM" key_preds "TO" key_preds
restrict_stmt   = "RESTRICT" name "TO" "(" query ")" ("AS" name)?
glue_stmt       = "GLUE" "(" query ")" "WITH" "(" query ")" ("CHECK" "OVERLAP")?
predict_stmt    = "PREDICT" name "ON" field "BY" field
                  ("TRAIN" "BEFORE" predicate "TEST" "AFTER" predicate)?
diff_stmt       = "DIFF" name ("AT" "SNAPSHOT" string)? "AGAINST" name
snapshot_stmt   = "SNAPSHOT" name "AS" string
health_stmt     = "HEALTH" name
atlas_stmt      = "ATLAS" ("BEGIN" ("ISOLATION" ("FLAT"|"CURVED"))?
                | "COMMIT" | "ROLLBACK" ("TO" name)? | "SAVEPOINT" name)
subscribe_stmt  = "SUBSCRIBE" subscribe_target | "UNSUBSCRIBE" name
subscribe_target = name on_clause?
                 | "ANOMALIES" name ("ON" field)?
                 | "CURVATURE" name ("ON" field)? "DRIFT" ">" number
                 | "CONSISTENCY" name | "SPECTRAL" name
                 | "PHASE" name "ON" field
                 | "DIVERGENCE" name "THRESHOLD" number
explain_stmt    = "EXPLAIN" statement
translate_stmt  = "TRANSLATE" "SQL" string
show_stmt       = "SHOW" "BUNDLES" ("VERBOSE")?
describe_stmt   = "DESCRIBE" name ("VERBOSE")?
ingest_stmt     = "INGEST" name "FROM" (string | "STDIN") "FORMAT" format_name
                  ("HEADER" bool)? ("DELIMITER" char)? ("NULL_VALUE" string)?
                  ("SKIP" number)? ("MAP" "(" field_map ")")? ("FILTER" "(" where_clause ")")?
emit_file_stmt  = cover_stmt "EMIT" format_name "TO" (string | "STDOUT")
                  ("WITH" meta_list)?
generate_stmt   = "GENERATE" "BASE" name "FROM" key_preds "TO" key_preds "STEP" number
                  ("WITH" "(" kv_pairs ")")?
fill_stmt       = "FILL" name "ON" field "USING" ("TRANSPORT" | "INTERPOLATE" interp_method
                  | "CONSTANT" "(" kv_pairs ")")
transplant_stmt = "TRANSPLANT" name "INTO" name
                  ("WHERE" pred_expr)? ("MAP" "(" field_map ")")? ("RETRACT" "SOURCE")?

on_clause       = "ON" index_pred ("AND" index_pred)*
where_clause    = "WHERE" pred_expr
project         = "PROJECT" "(" proj_list ")"
distinct        = "DISTINCT" field
rank            = "RANK" "BY" field ("ASC"|"DESC")?
pagination      = ("FIRST" number)? ("SKIP" number)?
confidence_filter = "CONFIDENCE" (">="|">") number
restrict        = "RESTRICT" "TO" "(" query ")"
having          = "HAVING" pred_expr
emit            = "EMIT" ("DHOOM"|"JSON"|"CSV") ("WITH" meta_list)? ("BARE")?

index_pred      = field "=" value | field "IN" "(" (values | query) ")"
pred_expr       = pred_term ("OR" pred_term)*
pred_term       = pred_atom ("AND" pred_atom)*
pred_atom       = field comp_op value | field "MATCHES" string
                | field "VOID" | field "DEFINED" | field "NEAR" value
                | "CURVATURE" "(" field ")" comp_op number
                | "CONFIDENCE" "(" ")" comp_op number
                | "ANOMALY" "(" ")" "=" bool
                | "Z_SCORE" "(" ")" comp_op number
```

---

## XXVI. PostgreSQL Parity Checklist

| Category | PostgreSQL | GQL v2.1 | Status |
|---|---|---|---|
| CRUD | ✓ | ✓ | ✅ SECTION, REDEFINE, RETRACT, UPSERT |
| Range queries | ✓ | ✓ | ✅ COVER with ON/WHERE, DISTINCT, RANK, FIRST, SKIP |
| Aggregation | ✓ | ✓ | ✅ INTEGRATE with FILTER clause |
| Window functions | ✓ | ✓ | ✅ FIBER operations + TRANSPORT |
| Joins | ✓ | ✓ | ✅ PULLBACK + PRODUCT + set ops |
| Subqueries / CTEs | ✓ | ✓ | ✅ Nested covers + WITH + WITH RECURSIVE |
| Recursive queries | ✓ | ✓ | ✅ ITERATE |
| Transactions | ✓ | ✓ | ✅ ATLAS with FLAT/CURVED isolation |
| Constraints | ✓ | ✓ | ✅ CHECK, UNIQUE, REFERENCES, MORPHISM, IN, EXCLUDE |
| Access control | ✓ | ✓ | ❌ Specified; returns 501 |
| Row-level security | ✓ | ✓ | ❌ POLICY specified; returns 501 |
| Audit trail | ✓ | ✓ | ❌ Specified; returns 501 |
| Math functions | ✓ | ✓ | ✅ |
| String functions | ✓ | ✓ | ✅ |
| Date functions | ✓ | ✓ | ✅ |
| Type casting | ✓ | ✓ | ✅ CAST, :: shorthand |
| Conditional expressions | ✓ | ✓ | ✅ CLASSIFY, IF, RESOLVE, VOIDIF, GREATEST, LEAST |
| RETURNING | ✓ | ✓ | ✅ On SECTION, REDEFINE, RETRACT |
| Conditional aggregation | ✓ | ✓ | ✅ FILTER (WHERE ...) on any aggregate |
| Generate series | ✓ | ✓ | ✅ GENERATE BASE + FILL |
| Bulk import | ✓ | ✓ | ✅ INGEST from CSV/JSON/JSONL/DHOOM/SQL/URL |
| Bulk export | ✓ | ✓ | ✅ EMIT TO file (CSV/JSON/DHOOM) |
| Prepared statements | ✓ | ✓ | ❌ Specified; returns 501 |
| Triggers | ✓ | ✓ | ⚠️ Parsed; TriggerManager built but not wired |
| Cascade operations | ✓ | ✓ | ⚠️ Parsed; not wired |
| Backup/restore | ✓ | ✓ | ❌ Specified; returns 501 |
| Maintenance | ✓ | ✓ | ⚠️ COMPACT/ANALYZE/VACUUM parsed; no-op |
| Session management | ✓ | ✓ | ❌ SET/RESET return 501 |
| Information schema | ✓ | ✓ | ✅ SHOW BUNDLES/FIELDS/INDEXES/CONSTRAINTS/etc. |
| Comments | ✓ | ✓ | ❌ Returns 501 |
| **GQL-only (no SQL equivalent)** | | | |
| Curvature / Ricci / Spectral | — | ✓ | ✅ |
| Betti / Euler / Topology | — | ✓ | ✅ |
| Entropy / Free Energy | — | ✓ | ✅ |
| Geodesic / Similarity | — | ✓ | ✅ |
| Phase transitions | — | ✓ | ✅ |
| Anomaly detection | — | ✓ | ✅ |
| Double Cover (S + d² = 1) | — | ✓ | ✅ |
| Geometric encryption | — | ✓ | ✅ GaugeKey on ENCRYPTED fields |
| Confidence filtering | — | ✓ | ✅ |
| Sheaf restriction / gluing | — | ✓ | ⚠️ Sheaf module built; not wired |
| Subscriptions (WebSocket) | — | ✓ | ✅ |
