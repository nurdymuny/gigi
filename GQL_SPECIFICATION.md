# GQL — Geometric Query Language
## Version 2.0 — Complete Specification

**The query language for fiber bundle databases.**

SQL thinks in tables and rows. GQL thinks in bundles, sections, and fibers. Every keyword maps to a geometric operation. The language IS the math.

## Design Principles

1. **Geometric primitives, not relational.** SECTION, BUNDLE, COVER, PULLBACK, INTEGRATE — not SELECT, FROM, JOIN, GROUP BY.
2. **Confidence is not optional.** Every query returns confidence. Always.
3. **Analytics are verbs, not plugins.** CURVATURE, SPECTRAL, HOLONOMY, CONSISTENCY are first-class.
4. **The wire format is part of the language.** EMIT controls DHOOM serialization.
5. **Silence means agreement.** Omitted clauses inherit bundle defaults.
6. **Geometry determines complexity.** SECTION AT = O(1). COVER ON = O(|r|). PULLBACK = O(|left|). INTEGRATE = O(|group|).
7. **SQL compatibility via TRANSLATE.** Any SQL → GQL. Not all GQL → SQL.

## Complete Mapping: SQL to GQL

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
| **Subscriptions (expanded)** | | |
| — | SUBSCRIBE ANOMALIES t ON f | Real-time curvature spike events |
| — | SUBSCRIBE CURVATURE t DRIFT > ε | Curvature drift events |
| — | SUBSCRIBE CONSISTENCY t | H¹ violation events |
| — | SUBSCRIBE SPECTRAL t | Topology change events |
| — | SUBSCRIBE PHASE t ON f | Phase transition events |
| — | SUBSCRIBE DIVERGENCE t THRESHOLD ε | Distribution drift events |

**Score: 45 SQL operations matched. 50+ GQL-only operations. 0 SQL operations missing.**

---

## I. Schema Operations

### BUNDLE — Define a fiber bundle

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

### GAUGE — Schema migration (gauge transformation)

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

### COLLAPSE — Drop a bundle

```sql
COLLAPSE sensors;              -- requires CONFIRM in interactive mode
```

### LENS — Views (virtual bundles)

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

### DESCRIBE / SHOW

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

### SECTION — Insert a single record

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

### SECTIONS — Batch insert

```sql
SECTIONS sensors (
  42, 'Moscow', 'EU', 20240104, -31.9, 97.4, 2.2,
  43, 'Moscow', 'EU', 20240105, -30.3, 97.8, 1.8,
  44, 'Moscow', 'EU', 20240106, -22.1, 94.2, 3.1
);
-- Single WAL flush. Deferred field index. Batch curvature update.
```

### SECTION ... UPSERT — Insert or update

```sql
SECTION sensors (id: 42, city: 'Moscow', temp: -28.5) UPSERT;
-- If exists: update. If not: insert. Response shows which.
```

### REDEFINE — Update (redefine σ at a point)

```sql
-- Point update
REDEFINE sensors AT id=42 SET (temp: -28.5);

-- Bulk update via cover
REDEFINE sensors ON city = 'Moscow' SET (region: 'RU');
```

### RETRACT — Delete (retract σ from a point)

```sql
RETRACT sensors AT id=42;

RETRACT sensors ON city = 'TestCity';
-- Response includes curvature_delta
```

---

## III. Point Queries — SECTION AT (O(1))

```sql
SECTION sensors AT id=42;                               -- full section
SECTION sensors AT id=42 PROJECT (city, temp, wind);    -- projected
SECTION events AT user_id=1001, timestamp=1710000000;   -- composite key
EXISTS SECTION sensors AT id=42;                        -- boolean check
```

Every response includes confidence and curvature. Always.

---

## IV. Range Queries — COVER (O(|r|))

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
-- Distinct
COVER sensors DISTINCT city;                    -- field index keys: O(1)

-- Ordering
COVER sensors ON city = 'Moscow' RANK BY temp ASC;

-- Pagination
COVER sensors RANK BY date DESC FIRST 10;
COVER sensors RANK BY date DESC SKIP 10 FIRST 10;

-- All sections
COVER sensors ALL;

-- Pattern matching
COVER sensors WHERE city MATCHES 'Mos*';        -- wildcard
COVER sensors WHERE city MATCHES '/^[A-M]/';    -- regex

-- Null handling
COVER sensors WHERE pressure VOID;              -- IS NULL
COVER sensors WHERE pressure DEFINED;           -- IS NOT NULL

-- Confidence filter (GQL-only)
COVER sensors ON city = 'Moscow' CONFIDENCE >= 0.95;
-- Only results from neighborhoods with K < 0.0526

-- Computed fields
COVER sensors PROJECT (
  city, temp,
  temp_max - temp_min AS daily_range,
  CLASSIFY temp
    WHEN temp < 0  THEN 'freezing'
    WHEN temp < 15 THEN 'cold'
    WHEN temp < 25 THEN 'mild'
    ELSE 'hot'
  AS category,
  RESOLVE(pressure, 101.3) AS pressure
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
```

---

## V. Aggregation — INTEGRATE

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

---

## VI. Window Functions — FIBER Operations

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

## VII. Joins — PULLBACK (O(|left|))

```sql
PULLBACK readings ALONG sensor_id ONTO sensors;                         -- inner join
PULLBACK readings ALONG sensor_id ONTO sensors PRESERVE LEFT;           -- left join
PULLBACK orders ALONG customer_id ONTO customers ALONG region ONTO regions;  -- chain
PULLBACK sensors AS s1 ALONG region ONTO sensors AS s2;                 -- self-join
PRODUCT sensors WITH regions;                                           -- cross product
```

---

## VIII. Geometric Operations (GQL-Only)

These are impossible in SQL. They exist because the data lives on a fiber bundle.

### CURVATURE — Local data variability

```sql
CURVATURE sensors ON temp;                                    -- global K for temp
CURVATURE sensors ON temp BY city;                            -- per-city K
CURVATURE sensors ON temp, wind, humidity;                    -- multi-field + scalar
CURVATURE sensors ON temp WITHIN (COVER sensors ON region = 'EU');  -- scoped
```

### RICCI — Curvature between field pairs

```sql
RICCI sensors ON temp, humidity;
-- Ricci curvature between temperature and humidity fibers
-- Measures how correlated their variability is
-- High Ricci = when temp is volatile, humidity is too
-- Low Ricci = independent variability

RICCI sensors ON temp, wind BY city;
-- Per-city Ricci curvature between temp and wind
-- Moscow: Ricci(temp,wind)=0.012 (cold snaps bring calm winds)
-- Cape Town: Ricci(temp,wind)=0.089 (storms = wind + temp together)
```

### SECTIONAL — Curvature on 2-planes

```sql
SECTIONAL sensors ON temp, humidity;
-- Sectional curvature on the (temp, humidity) 2-plane
-- Measures Gaussian curvature of the 2D slice of fiber space
-- Positive = data clusters in ellipses. Negative = data spreads in saddles.
-- Zero = flat (independent variation)

SECTIONAL sensors ON temp, humidity BY region;
-- Per-region sectional curvature
```

### SCALAR — Total curvature (trace of Ricci)

```sql
SCALAR sensors;
-- Scalar curvature = sum of all sectional curvatures
-- Single number summarizing total data variability
-- Returns: R = 0.0346

SCALAR sensors BY city;
-- Per-city scalar curvature
```

### DEVIATION — Geodesic deviation (how sections diverge)

```sql
DEVIATION sensors FROM id=42 ALONG date;
-- How does section σ(42) diverge from its neighbors over time?
-- Returns: deviation vector, Jacobi field magnitude
-- Large deviation = this record is becoming more anomalous over time

DEVIATION sensors ON temp BY city;
-- Per-city: how are temperature profiles diverging from each other?
-- Identifies cities whose climate is becoming more dissimilar
```

### TREND — Curvature-based trend detection

```sql
TREND sensors ON temp BY city;
-- dK/dt — is curvature increasing or decreasing over time?
-- Moscow: dK/dt = +0.003 (becoming MORE volatile)
-- Singapore: dK/dt = 0.000 (stable as always)

TREND sensors ON temp BY city WINDOW date ROWS (-30, 0);
-- 30-day rolling curvature trend
```

### CONFIDENCE — Query trust level

```sql
CONFIDENCE sensors;                       -- global: 0.9665
CONFIDENCE sensors ON city = 'Moscow';    -- cover-specific: 0.9772
```

### CAPACITY — Query capacity (C = τ/K)

```sql
CAPACITY sensors;
-- Global capacity: C = 0.1 / 0.0346 = 2.89
-- Interpretation: ~2.89 independent queries can be answered unambiguously
-- per unit of tolerance budget

CAPACITY sensors BY city;
-- Per-city capacity
-- Singapore: C = 1000 (ultra-high — data is so flat you can ask anything)
-- Moscow: C = 4.29 (moderate — high variability limits precision)

CAPACITY sensors TOLERANCE 1.0;
-- Recompute at different tolerance: C = 1.0 / 0.0346 = 28.9
```

### SPECTRAL — Index connectivity

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
-- "The weakest link between your data clusters is the region boundary"

BOTTLENECK sensors ON city;
-- Which city-to-city connection is weakest?
-- Returns: Moscow↔Singapore, conductance=0.001 (barely connected)
```

### CLUSTER — Spectral clustering via Fiedler vector

```sql
CLUSTER sensors INTO 4;
-- Spectral clustering: compute Fiedler vector (eigenvector of λ₁)
-- Partition into 4 clusters by sign/magnitude of Fiedler components
-- Returns: cluster assignments, within-cluster K, between-cluster distance
-- No k-means. No embeddings. No Euclidean distance. Pure spectral geometry.

CLUSTER sensors INTO 4 BY temp, humidity, wind;
-- Cluster on specific fiber fields

CLUSTER sensors INTO AUTO;
-- Auto-detect number of clusters from spectral gap structure
-- Number of near-zero eigenvalues = number of natural clusters
```

### MIXING — Mixing time estimation

```sql
MIXING sensors;
-- How many hops to reach equilibrium on the field index graph?
-- mixing_time = O(1/λ₁)
-- If λ₁=0: infinite (disconnected — never mixes)
-- If λ₁=0.75: ~1.33 steps (well-connected)
-- Practical meaning: how many random walks before you've "seen" the whole dataset

MIXING sensors ON region;
-- Mixing time restricted to region index
```

### CONDUCTANCE — Graph conductance

```sql
CONDUCTANCE sensors;
-- Minimum ratio of edges leaving a subset to edges within it
-- Low conductance = data has strong internal clusters with weak bridges
-- High conductance = data is well-mixed, no isolated pockets

CONDUCTANCE sensors ON city;
-- Conductance of the city index graph
```

### LAPLACIAN — Hodge Laplacian on data forms

```sql
LAPLACIAN sensors;
-- Eigenvalues of the graph Laplacian on the full field index
-- Returns: eigenvalue spectrum, multiplicity structure
-- The spectrum IS the topology: number of zero eigenvalues = components,
-- gap to first nonzero = connectivity strength, distribution = geometry

LAPLACIAN sensors ON city TOP 5;
-- Top 5 eigenvalues (smallest, including zero)
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
-- Returns: holonomy value, accumulated curvature, path length
-- Wilson loop ≠ 0 means the data is not self-consistent along this path

WILSON sensors AROUND (date=20240101, date=20240201, date=20240301, date=20240101);
-- Temporal loop: is the data self-consistent across time?
```

### CONSISTENCY — Čech cohomology

```sql
CONSISTENCY sensors;              -- h1: 0 (consistent)
CONSISTENCY sensors REPAIR;       -- attempt cocycle resolution
```

### BETTI — Topological invariants

```sql
BETTI sensors;
-- β₀ = number of connected components
-- β₁ = number of independent loops (1-cycles)
-- β₂ = number of enclosed voids (2-cycles)
--
-- β₀=20, β₁=0, β₂=0 → 20 isolated clusters, no loops, no voids
-- β₀=1, β₁=3, β₂=0 → connected, 3 independent cycles
-- β₁ > 0 means there are non-contractible loops in your data topology

BETTI sensors ON city, region;
-- Betti numbers of the (city, region) index subgraph
```

### EULER — Euler characteristic

```sql
EULER sensors;
-- χ = β₀ - β₁ + β₂
-- Topological invariant: doesn't change under continuous deformation
-- χ = 20 (20 components, 0 loops, 0 voids)
-- If χ changes after a batch insert, the TOPOLOGY of your data changed
```

### COCYCLE — Extract specific Čech cocycles

```sql
COCYCLE sensors ON (city, region);
-- Returns all nonzero cocycles on the (city, region) cover
-- Each cocycle identifies a specific inconsistency:
--   "city=Moscow ∩ region=EU: temp disagrees by 0.3°C"

COBOUNDARY sensors ON cocycle_id;
-- Is this cocycle exact (a coboundary)?
-- If yes: the inconsistency is removable by local adjustment
-- If no: the inconsistency is topological (structural, not fixable locally)
```

### TRIVIALIZE — Check bundle triviality

```sql
TRIVIALIZE sensors;
-- Is the bundle globally trivial? E ≅ B × F everywhere?
-- TRIVIAL → all transition functions are identity → all charts agree
-- NON-TRIVIAL → there exist transition functions ≠ identity
-- Practical meaning: can you flatten this data into a single CSV
-- without losing structure? TRIVIAL = yes. NON-TRIVIAL = no.
```

### CHARACTERISTIC — Characteristic classes

```sql
CHARACTERISTIC sensors;
-- Chern class: obstruction to trivializing the bundle
-- Returns: c₁ (first Chern number)
-- c₁ = 0 → bundle is topologically trivializable
-- c₁ ≠ 0 → bundle has intrinsic twist that cannot be removed
-- Practical meaning: your data has structure that NO flat schema can capture
```

### ENTROPY — Shannon entropy

```sql
ENTROPY sensors;
-- Shannon entropy of the value distribution across all sections
-- H = -Σ p(x) log p(x)
-- High entropy = data is diverse/unpredictable
-- Low entropy = data is concentrated/predictable

ENTROPY sensors ON status;
-- Entropy of just the status field
-- If 95% are 'normal': H ≈ 0.29 (low — very predictable)

ENTROPY sensors ON temp BY city;
-- Per-city temperature entropy
-- Moscow: H = 3.2 (high — temperature varies widely)
-- Singapore: H = 1.1 (low — temperature barely moves)
```

### DIVERGENCE — KL divergence between covers

```sql
DIVERGENCE (COVER sensors ON region = 'EU')
  FROM (COVER sensors ON region = 'AS')
  ON temp;
-- KL divergence D_KL(EU || AS) on temperature distribution
-- Measures how "different" EU temperatures are from AS temperatures
-- D_KL = 0 → identical distributions
-- D_KL >> 0 → very different distributions

DIVERGENCE (COVER sensors ON city = 'Moscow')
  FROM (COVER sensors ON city = 'Singapore')
  ON temp, humidity, wind;
-- Multi-field divergence
```

### FISHER — Fisher information metric

```sql
FISHER sensors ON temp, humidity;
-- Fisher information matrix between temp and humidity
-- Measures how much information each field carries about the other
-- The natural metric on the statistical manifold of the data
-- High Fisher = small changes in temp strongly predict humidity changes
-- Low Fisher = fields are informationally independent

FISHER sensors ON temp, humidity BY city;
-- Per-city Fisher information
```

### MUTUAL — Mutual information between fields

```sql
MUTUAL sensors ON temp WITH humidity;
-- I(temp; humidity) = H(temp) + H(humidity) - H(temp, humidity)
-- How much knowing temperature tells you about humidity
-- I = 0 → independent. I >> 0 → strongly coupled.

MUTUAL sensors ON temp WITH wind BY city;
-- Per-city mutual information between temp and wind
```

### FREEENERGY — Helmholtz free energy

```sql
FREEENERGY sensors TOLERANCE 0.1;
-- F = -τ log Z(β, p)
-- The thermodynamic free energy of the data at temperature τ
-- Low F = data is in a "low energy" state (well-organized)
-- High F = data is in a "high energy" state (disordered)
-- ΔF between snapshots measures how much the data's organization changed

FREEENERGY sensors BY city TOLERANCE 1.0;
-- Per-city free energy at τ=1.0
```

### TEMPERATURE — Effective temperature per neighborhood

```sql
TEMPERATURE sensors;
-- Effective τ at each neighborhood, computed from local fluctuations
-- High temperature = high local variance = approximate queries are safe
-- Low temperature = low variance = exact queries are needed

TEMPERATURE sensors BY city;
-- Moscow: τ_eff = 12.3 (hot — highly variable, approximate is fine)
-- Singapore: τ_eff = 0.01 (cold — barely varies, demand precision)
```

### PHASE — Phase transition detection

```sql
PHASE sensors ON temp;
-- Detect discontinuities in curvature K as a function of base parameters
-- Phase transition = sudden change in data regime
-- Returns: transition_points, K_before, K_after, order of transition

PHASE sensors ON temp ALONG date;
-- Where in time did the temperature distribution change regime?
-- "Phase transition detected at date=20240315: K jumped from 0.01 to 0.04"
-- Spring arrived. The geometry saw it.

PHASE sensors ON temp ALONG date BY city;
-- Per-city phase transitions over time
```

### CRITICAL — Find critical points

```sql
CRITICAL sensors ON temp;
-- Points where ∂K/∂(base) = 0
-- These are extrema or saddle points of the curvature landscape
-- Returns: base_points, K_value, type (minimum / maximum / saddle)
-- Practical: where does data variability peak or trough?

CRITICAL sensors ON temp BY city;
-- Per-city: when is temperature variability at its max/min?
```

### PARTITION — Approximate queries (statistical mechanics)

```sql
PARTITION sensors AT city = 'Moskow' TOLERANCE 0.5;              -- fuzzy match
PARTITION sensors WHERE temp NEAR 22.0 TOLERANCE 2.0;            -- neighborhood
CALIBRATE sensors TOLERANCE 0.01;                                 -- set default τ
```

### TRANSPORT — Parallel transport

```sql
TRANSPORT sensors OF temp FROM id=42 TO id=100;                 -- point-to-point
TRANSPORT sensors OF temp ALONG date SHIFT -1;                   -- LAG (flat)
```

### FLOW — Renormalization group (C-theorem)

```sql
FLOW sensors COARSEN BY region LEVELS 3;
-- Returns entropy per level. C-theorem: non-increasing.
-- Violation = structural anomaly at that scale.

FLOW sensors COARSEN BY city LEVELS 5 SHOW ENTROPY;
-- 5-level coarsening with explicit entropy trace
```

### GEODESIC — Shortest path

```sql
GEODESIC sensors FROM id=42 TO id=100;
GEODESIC sensors FROM city='Moscow' TO city='Singapore';
```

### SIMILAR — Find geometrically similar sections

```sql
SIMILAR sensors TO id=42 WITHIN 0.1;
-- Find all sections within geodesic distance 0.1 of section 42
-- Uses fiber distance (not Euclidean!) — normalized by field ranges
-- Returns: similar sections ranked by geodesic distance

SIMILAR sensors TO id=42 WITHIN 0.1 ON temp, wind;
-- Similarity restricted to specific fiber fields

SIMILAR sensors TO id=42 TOP 10;
-- 10 nearest geodesic neighbors
```

### CORRELATE — Geometric correlation via connection

```sql
CORRELATE sensors ON temp WITH wind;
-- Geometric correlation: how does the connection couple temp and wind?
-- Unlike Pearson r, this measures correlation on the MANIFOLD,
-- not in flat Euclidean space. Respects field ranges and curvature.

CORRELATE sensors ON temp WITH wind BY city;
-- Per-city geometric correlation
-- Moscow: corr(temp,wind) = -0.23 (cold = calm)
-- Cape Town: corr(temp,wind) = 0.67 (warm = windy)
```

### SEGMENT — Geometric segmentation

```sql
SEGMENT sensors INTO 4 BY temp, humidity, wind;
-- Curvature-based clustering: find natural segments where K is
-- locally minimal (flat regions) separated by high-K boundaries
-- Unlike k-means: no distance computation. Segments are defined
-- by the curvature landscape. Boundaries are curvature ridges.

SEGMENT sensors INTO AUTO;
-- Auto-detect segment count from curvature landscape topology
-- Number of local K minima = number of natural segments
```

### OUTLIER — Flag outliers by curvature

```sql
OUTLIER sensors ON temp SIGMA 3;
-- All sections where temp is > 3σ from local neighborhood mean
-- Uses curvature-normalized thresholds (adaptive, not global)
-- High-K neighborhoods get wider thresholds (variability expected)
-- Low-K neighborhoods get tighter thresholds (deviation is meaningful)

OUTLIER sensors ON temp, wind SIGMA 2.5 BY city;
-- Per-city, multi-field outlier detection
```

### PROFILE — Full geometric data profile

```sql
PROFILE sensors;
-- Complete geometric analysis of every field:
--   Per-field: K, confidence, entropy, range, distinct_count, void_rate
--   Per-pair: Ricci curvature, Fisher information, mutual information
--   Global: scalar curvature, spectral gap, Betti numbers, Euler characteristic
--   Storage: base geometry, estimated compression via DHOOM
--   Anomalies: top outliers with Z-scores
-- One call. Everything you need to understand your data geometrically.
```

### DOUBLECOVER — S + d² = 1 verification

```sql
DOUBLECOVER sensors ON (COVER sensors ON city = 'Moscow');
-- Computes recall S and deviation d for the Moscow cover
-- Verifies S + d² = 1 (must hold for all valid queries)
-- Returns: S=0.998, d=0.045, S+d²=1.000 ✓

RECALL sensors ON (COVER sensors WHERE temp < -25);
-- Just the recall: what fraction of relevant records were returned?

COMPLETENESS sensors ON (COVER sensors WHERE temp < -25);
-- Full Double Cover check: S + d² = 1?
```

### RESTRICT — Sheaf restriction

```sql
RESTRICT sensors TO (COVER sensors ON region = 'EU') AS eu_sensors;
```

### GLUE — Explicit sheaf gluing

```sql
GLUE (COVER sensors ON region = 'EU') WITH (COVER sensors ON region = 'NA') CHECK OVERLAP;
```

### PREDICT — Curvature forecasting

```sql
PREDICT sensors ON temp BY city;
PREDICT sensors ON temp BY city TRAIN BEFORE date=20240901 TEST AFTER date=20240901;
```

### DIFF — Bundle comparison

```sql
DIFF sensors_v1 AGAINST sensors_v2;
DIFF sensors AT SNAPSHOT 'yesterday' AGAINST sensors;
```

### SNAPSHOT — Point-in-time capture

```sql
SNAPSHOT sensors AS 'pre_migration';
```

### HEALTH — Full diagnostic

```sql
HEALTH sensors;
```

### GAUGE VERIFY — Post-migration invariance check

```sql
GAUGE VERIFY sensors;
-- After a GAUGE TRANSFORM, verify that K is actually invariant
-- Returns: K_before, K_after, delta, status (INVARIANT / VIOLATED)
-- If VIOLATED: identifies which transformation broke invariance
```

---

## IX. Transactions — ATLAS

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

---

## X. Subscriptions

```sql
SUBSCRIBE sensors ON status = 'alert';                     -- new matching sections
SUBSCRIBE ANOMALIES sensors ON temp;                       -- curvature spikes
SUBSCRIBE CURVATURE sensors ON temp DRIFT > 0.01;         -- K drift events
SUBSCRIBE CONSISTENCY sensors;                             -- H¹ violations
SUBSCRIBE SPECTRAL sensors;                                -- topology changes
UNSUBSCRIBE sub_id;
```

---

## XI. Output Control

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

### TRANSLATE — SQL compatibility bridge

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

## XII. Formal Grammar (EBNF)

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
section_stmt  = "SECTION" name "(" kv_pairs ")" ( "UPSERT" )?
sections_stmt = "SECTIONS" name "(" value_rows ")"
redefine_stmt = "REDEFINE" name ( "AT" key_preds | on_clause ) "SET" "(" kv_pairs ")"
retract_stmt  = "RETRACT" name ( "AT" key_preds | on_clause )
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
with_stmt     = "WITH" name "AS" "(" query ")" statement
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
explain_stmt    = "EXPLAIN" statement
translate_stmt  = "TRANSLATE" "SQL" string
show_stmt       = "SHOW" "BUNDLES"
describe_stmt   = "DESCRIBE" name ("VERBOSE")?

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
                | "NOT" pred_atom | "(" pred_expr ")"
comp_op         = "<" | ">" | "=" | "!=" | "<=" | ">="

field_defs      = field_def ("," field_def)*
field_def       = name type modifier*
type            = "NUMERIC" | "CATEGORICAL" | "TEXT" | "BOOLEAN" | "TIMESTAMP"
modifier        = "INDEX" | "RANGE" num | "DEFAULT" value | "AUTO"
                | "ARITHMETIC" | "UNIQUE" | "REQUIRED" | "NULLABLE"
alterations     = alteration ("," alteration)*
alteration      = "ADD" field_def | "DROP" name | "RENAME" name "TO" name
agg_list        = agg_func ("," agg_func)*
agg_func        = agg_name "(" (field|"*") ")" ("AS" name)?
agg_name        = "avg"|"sum"|"count"|"min"|"max"|"stddev"|"variance"|"median"|"mode"|"percentile"
proj_list       = proj_item ("," proj_item)*
proj_item       = field | expr "AS" name | classify | resolve
classify        = "CLASSIFY" field ("WHEN" pred_expr "THEN" value)+ "ELSE" value "AS" name
resolve         = "RESOLVE" "(" field "," value ")" ("AS" name)?
meta_list       = meta ("," meta)*
meta            = "CURVATURE" | "CONFIDENCE" | "ANOMALIES" | "SPECTRAL"
kv_pairs        = (name ":" value) ("," name ":" value)*
key_preds       = (name "=" value) ("," name "=" value)*
field_list      = field ("," field)*
option_list     = (name value) ("," name value)*
values          = value ("," value)*
value_rows      = values ("," values)*
expr            = term (( "+"|"-") term)*
term            = factor (("*"|"/") factor)*
factor          = field | number | "(" expr ")" | agg_func
                | "TRANSPORT" name "OF" field "ALONG" field "SHIFT" signed_num
name            = [A-Za-z_][A-Za-z0-9_]*
alias           = name
field           = name ("." name)?
value           = number | string | "TRUE" | "FALSE" | "VOID"
number          = [0-9]+ ("." [0-9]+)?
signed_num      = ("+"|"-") number
string          = "'" [^']* "'"
bound           = number | "UNBOUNDED"
query           = cover_stmt | integrate_stmt | point_query
```

---

## XIII. Reserved Words (128)

```
ADD AGAINST ALL ALONG ANOMALIES ARITHMETIC AS ASC AT ATLAS AUTO
BARE BASE BEGIN BETTI BOOLEAN BOTTLENECK BUNDLE BY
CALIBRATE CAPACITY CATEGORICAL CHARACTERISTIC CHECK CLASSIFY CLUSTER
  COBOUNDARY COCYCLE COARSEN COLLAPSE COMMIT COMPLETENESS CONDUCTANCE
  CONFIDENCE CONSISTENCY CORRELATE COVER CRITICAL CSV CURVATURE CURVED
DEFAULT DEFINED DESC DESCRIBE DEVIATION DIFF DHOOM DISTINCT DIVERGENCE
  DRIFT DROP DOUBLECOVER
ELSE EMIT ENTROPY EULER EXISTS EXPLAIN
FIBER FIRST FISHER FLAT FLOW FREEENERGY FROM FULL
GAUGE GEODESIC GLUE
HAVING HEALTH HOLONOMY
IN INDEX INTEGRATE INTERSECT INTO ISOLATION
JSON
LAPLACIAN LENS LEVELS
MATCHES MATERIALIZE MEASURE MINUS MIXING MODE MUTUAL
NEAR NULLABLE NUMERIC
ON ONTO OPTIONS OR OUTLIER OVER OVERLAP
PARTITION PERCENTILE PERCENTRANK PHASE PREDICT PRESERVE
  PRODUCT PROFILE PROJECT
RANGE RANK RECALL REDEFINE REFRESH REPAIR REQUIRED RESOLVE
  RESTRICT RETRACT RICCI ROLLBACK ROWS
SAVEPOINT SCALAR SECTION SECTIONS SECTIONAL SEGMENT SET SHIFT
  SHOW SIGMA SIMILAR SKIP SNAPSHOT SPECTRAL SQL SUBSCRIBE SUBTRACT
TEMPERATURE TEST TEXT THEN THRESHOLD TIMESTAMP TO TOLERANCE TOP
  TRAIN TRANSLATE TRANSPORT TREND TRIVIALIZE
UNBOUNDED UNION UNIQUE UNSUBSCRIBE UPSERT
VERBOSE VERIFY VOID
WHEN WHERE WILSON WINDOW WITH WITHIN
```

SQL-92 has 222 reserved words. GQL has 128. GQL provides 50+ operations SQL cannot express. 87 of GQL's keywords have no SQL equivalent. The geometry is more expressive than set theory.

---

## XIV. Why GQL Exists

SQL was designed in 1974 for relational algebra on flat tables. It has no concept of curvature, connection, sheaves, parallel transport, cohomology, spectral gaps, partition functions, phase transitions, Betti numbers, Fisher information, or geodesics.

GQL makes all of this first-class. 50+ operations that SQL cannot express:

**Curvature family (7):** CURVATURE, RICCI, SECTIONAL, SCALAR, DEVIATION, TREND, CAPACITY
**Spectral family (6):** SPECTRAL, BOTTLENECK, CLUSTER, MIXING, CONDUCTANCE, LAPLACIAN
**Topology family (6):** CONSISTENCY, BETTI, EULER, COCYCLE, COBOUNDARY, TRIVIALIZE
**Information family (5):** ENTROPY, DIVERGENCE, FISHER, MUTUAL, PROFILE
**Statistical mechanics (5):** PARTITION, FREEENERGY, PHASE, CRITICAL, TEMPERATURE
**Gauge theory (3):** WILSON, GAUGE VERIFY, CHARACTERISTIC
**Transport and paths (3):** TRANSPORT, GEODESIC, FLOW
**Similarity and discovery (4):** SIMILAR, CORRELATE, SEGMENT, OUTLIER
**Double Cover (3):** DOUBLECOVER, RECALL, COMPLETENESS
**Data management (4):** PREDICT, DIFF, SNAPSHOT, HEALTH
**Sheaf operations (3):** RESTRICT, GLUE, CONFIDENCE
**Real-time (6):** SUBSCRIBE to ANOMALIES, CURVATURE, CONSISTENCY, SPECTRAL, PHASE, DIVERGENCE

Every keyword is simultaneously a mathematical operation and a complexity guarantee. SECTION AT = O(1). COVER ON = O(|r|). PULLBACK ALONG = O(|left|). CURVATURE = O(1) incremental. SPECTRAL = O(|buckets| × α(n)). The syntax IS the cost model.

SQL describes WHAT you want.
GQL describes WHAT you want, HOW the geometry answers it, and WHY you should trust the result.

**GQL v2.0** · Geometric Query Language · Davis Geometric · 2026
