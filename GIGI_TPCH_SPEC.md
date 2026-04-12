# GIGI TPC-H Benchmark Specification

**Version**: 0.1  
**Date**: April 2026  
**Status**: TDD — spec first, bench second

---

## 1. Purpose

TPC-H is the Transaction Processing Performance Council's Decision Support
benchmark. It defines 8 tables, a data generator (`dbgen`), and 22 SQL
queries at standardised scale factors (SF=0.1 → 10 TB). Unlike TPC-C
(OLTP, concurrent writers), TPC-H targets read-heavy analytics workloads —
the domain GIGI is designed for.

This spec defines:

1. How each TPC-H table maps to a GIGI bundle schema  
2. Which mathematical properties each bundle should exhibit (testable)  
3. Which of the 22 queries GIGI can execute natively, which need a thin
   aggregation wrapper, and which require future work  
4. The TDD test plan (Python math validation → Rust unit → Rust bench)  
5. Benchmark design and expected results

---

## 2. Scale Factors

| SF   | LINEITEM rows | ORDERS rows | Total size | Use           |
|------|--------------|-------------|-----------|---------------|
| 0.001 | 6 000        | 1 500       | ~1 MB     | Unit tests    |
| 0.01 | 60 000       | 15 000      | ~10 MB    | Integration   |
| 0.1  | 600 000      | 150 000     | ~100 MB   | Bench (quick) |
| 1.0  | 6 000 000    | 1 500 000   | ~1 GB     | Bench (full)  |

All deterministic data generated in-process (no dbgen dependency for tests).
For serious comparison benchmarks, use official `dbgen` at SF=1.

---

## 3. Bundle Schema Mapping

### Modelling principle

GIGI's base field is the **unique key** that enables O(1) point lookup.
For 1:N relationships (ORDERS→LINEITEM), the child table must be queryable
by the parent key. The strategy: use an **auto-increment base key** and
**index the foreign key** as a fiber field — this gives O(result) bitmap
lookup via `range_query`.

### 3.1 REGION

```
base:  r_regionkey  Numeric         -- 0..4, arithmetic → K=0 → Sequential
fiber: r_name       Categorical
       r_comment    Categorical
```

**Predicted K**: 0.0 (5 rows, keys 0-4, step 1)  
**Storage mode**: Sequential  
**Size at any SF**: 5 rows (fixed)

### 3.2 NATION

```
base:  n_nationkey  Numeric         -- 0..24, arithmetic → K=0 → Sequential
fiber: n_name       Categorical
       n_regionkey  Numeric         -- FK → REGION, index this
       n_comment    Categorical
index: [n_regionkey, n_name]
```

**Predicted K**: 0.0 (25 rows, keys 0-24, step 1)  
**Storage mode**: Sequential  
**Join strategy**: REGION→NATION via `range_query("n_regionkey", [region_key])`

### 3.3 SUPPLIER

```
base:  s_suppkey    Numeric         -- 1..10K at SF=1, arithmetic → K=0
fiber: s_name       Categorical
       s_address    Categorical
       s_nationkey  Numeric
       s_phone      Categorical
       s_acctbal    Numeric
       s_comment    Categorical
index: [s_nationkey]
```

**Predicted K**: low (s_acctbal has variance, but with range normalisation K ≈ 0.01)  
**Storage mode**: Sequential (arithmetic keys 1..N)

### 3.4 CUSTOMER

```
base:  c_custkey    Numeric         -- 1..150K at SF=1, arithmetic
fiber: c_name       Categorical
       c_address    Categorical
       c_nationkey  Numeric
       c_phone      Categorical
       c_acctbal    Numeric
       c_mktsegment Categorical     -- 5 values: AUTOMOBILE, BUILDING, etc.
       c_comment    Categorical
index: [c_mktsegment, c_nationkey]
```

**Predicted K**: ~0.05 (c_acctbal has meaningful variance)  
**Storage mode**: Sequential (arithmetic 1..N)  
**Key query**: `filtered_query([Eq("c_mktsegment", "BUILDING")])` → ~20% of rows

### 3.5 PART

```
base:  p_partkey    Numeric         -- 1..200K at SF=1, arithmetic
fiber: p_name       Categorical
       p_mfgr       Categorical     -- 5 values
       p_brand      Categorical     -- 25 values
       p_type       Categorical     -- ~150 values
       p_size       Numeric
       p_container  Categorical
       p_retailprice Numeric
       p_comment    Categorical
index: [p_mfgr, p_brand, p_type, p_container]
```

**Predicted K**: low–mid (p_retailprice variance, p_size distribution)  
**Storage mode**: Sequential  
**Q14 join**: pullback_join(lineitem_filtered, part, "l_partkey", "p_partkey")
  — l_partkey must be a fiber field in LINEITEM; p_partkey is base of PART → O(1)

### 3.6 PARTSUPP

```
base:  ps_id        Numeric (auto-inc)
fiber: ps_partkey   Numeric         -- FK → PART, indexed
       ps_suppkey   Numeric         -- FK → SUPPLIER, indexed
       ps_availqty  Numeric
       ps_supplycost Numeric
       ps_comment   Categorical
index: [ps_partkey, ps_suppkey]
```

**Note**: Composite logical key (partkey, suppkey). Using auto-inc base +
indexed fibers is required so we can do bitmap lookups by either FK.  
**Storage mode**: Sequential (auto-inc 1..N)  
**Predicted K**: mid (supplycost has notable variance across parts)

### 3.7 ORDERS

```
base:  o_orderkey   Numeric         -- sparse in TPC-H (gaps) → K > 0 → Hashed
fiber: o_custkey    Numeric         -- FK → CUSTOMER, indexed
       o_orderstatus Categorical    -- 3 values: F, O, P
       o_totalprice  Numeric
       o_orderdate   Numeric        -- epoch days
       o_orderpriority Categorical  -- 5 values
       o_clerk       Categorical
       o_shippriority Numeric
       o_comment     Categorical
index: [o_custkey, o_orderstatus, o_orderpriority]
```

**Predicted K**: > 0 (o_orderkey has gaps in dbgen output — not arithmetic)  
**Storage mode**: Hashed (GIGI auto-detects non-arithmetic keys after 32 inserts)  
**Q3 join step 1**: `range_query("o_custkey", building_customer_keys)` → bitmap join

### 3.8 LINEITEM

```
base:  l_id         Numeric (auto-inc) -- unique per line
fiber: l_orderkey   Numeric         -- FK → ORDERS, indexed
       l_linenumber Numeric
       l_partkey    Numeric         -- FK → PART, indexed
       l_suppkey    Numeric         -- FK → SUPPLIER, indexed
       l_quantity   Numeric
       l_extendedprice Numeric
       l_discount   Numeric
       l_tax        Numeric
       l_returnflag  Categorical    -- 3 values: A, N, R
       l_linestatus  Categorical    -- 2 values: F, O
       l_shipdate    Numeric        -- epoch days
       l_commitdate  Numeric
       l_receiptdate Numeric
       l_shipinstruct Categorical
       l_shipmode    Categorical
       l_comment     Categorical
index: [l_orderkey, l_returnflag, l_linestatus, l_partkey, l_suppkey]
```

**Predicted K**: mid–high (l_quantity, l_extendedprice, l_discount all vary)  
**Storage mode**: Sequential (auto-inc base avoids the sparse orderkey problem)  
**Most queries start here** — it's the largest table

---

## 4. Join Strategy Reference

| Left             | Join Key      | Right     | Strategy        | Complexity       |
|-----------------|--------------|-----------|-----------------|-----------------|
| LINEITEM        | l_orderkey   | ORDERS    | bitmap join     | O(\|L\| + result) |
| LINEITEM        | l_partkey    | PART      | pullback_join   | O(\|L\|)         |
| LINEITEM        | l_suppkey    | SUPPLIER  | pullback_join   | O(\|L\|)         |
| ORDERS          | o_custkey    | CUSTOMER  | pullback_join   | O(\|O\|)         |
| ORDERS filtered | o_custkey    | CUSTOMER  | bitmap join     | O(\|cust\|)      |
| CUSTOMER filtered| c_custkey   | ORDERS    | range_query FK  | O(\|cust\|)      |
| PARTSUPP        | ps_suppkey   | SUPPLIER  | pullback_join   | O(\|PS\|)        |
| NATION          | n_regionkey  | REGION    | pullback_join   | O(25)            |
| SUPPLIER        | s_nationkey  | NATION    | pullback_join   | O(\|S\|)         |

**Bitmap join**: when the FK is a fiber field that is indexed, use
`range_query(field, key_list)` — O(|key_list|) bitmap lookups.

**Pullback join**: when FK value in left maps to PK (base key) in right,
use `pullback_join(left, right, left_fk, right_pk)` — O(|left|) × O(1).

---

## 5. Query Implementation Table

| Q  | Name                    | Tables       | GIGI Strategy                                 | Tier |
|----|------------------------|-------------|----------------------------------------------|------|
| Q1 | Pricing Summary        | LINEITEM     | filter + filtered_group_by(returnflag/status) | 2    |
| Q2 | Minimum Cost Supplier  | 5 tables     | correlated subquery — needs multi-join        | 3    |
| Q3 | Shipping Priority      | CUST+ORD+LI  | filter→bitmap join→pullback join→agg          | 2    |
| Q4 | Order Priority         | ORD+LI       | EXISTS subquery — filter + correlated check   | 3    |
| Q5 | Local Supplier Volume  | 6 tables     | 6-way chain join + agg                        | 2    |
| Q6 | Revenue Forecast       | LINEITEM     | filter + scan agg (no join)                   | 1    |
| Q7 | Volume Shipping        | 6 tables     | 6-way join + agg                              | 2    |
| Q8 | Market Share           | 8 tables     | full chain + conditional agg                  | 2    |
| Q9 | Product Type Profit    | 6 tables     | 6-way join + group by year                    | 2    |
| Q10| Returned Items         | 4 tables     | filter + 4-way join + agg                     | 2    |
| Q11| Stock Assessment       | 3 tables     | HAVING subquery                               | 3    |
| Q12| Shipping Modes         | ORD+LI       | pullback join + conditional group agg         | 2    |
| Q13| Customer Distribution  | CUST+ORD     | outer join + group + HAVING                   | 3    |
| Q14| Promotion Effect       | LI+PART      | filter + pullback join + conditional agg      | 1    |
| Q15| Top Supplier           | LI+SUP+view  | requires view (max agg → filter)              | 3    |
| Q16| Part/Supplier Relation | PART+PARTSUP  | IN filter + NOT IN filter                     | 3    |
| Q17| Small Quantity Order   | LI+PART      | correlated avg subquery                       | 3    |
| Q18| Large Volume Customer  | CUST+ORD+LI  | HAVING on grouped join                        | 3    |
| Q19| Discounted Revenue     | LI+PART      | complex OR conditions + filter agg            | 2    |
| Q20| Potential Part Promo   | 5 tables     | correlated subquery chain                     | 3    |
| Q21| Suppliers Who Kept     | 4 tables     | EXISTS + NOT EXISTS                           | 3    |
| Q22| Global Sales Oppty     | CUST+ORD     | CASE + subquery avg                           | 3    |

**Tier 1**: GIGI native — `filtered_query` + `fiber_integral` or `filtered_group_by`  
**Tier 2**: Thin Rust wrapper — join + manual aggregation loop over result records  
**Tier 3**: Future work — correlated subqueries, HAVING, set operators

**Initial benchmark targets**: Q1, Q3, Q6, Q14 (cover Tier 1 + 2, all join patterns)

---

## 6. Mathematical Predictions (Testable)

### 6.1 Storage mode predictions

| Table     | Key pattern          | Predicted K | Predicted mode |
|-----------|---------------------|------------|----------------|
| REGION    | 0,1,2,3,4 (step=1)  | 0.0        | Sequential     |
| NATION    | 0..24 (step=1)      | 0.0        | Sequential     |
| SUPPLIER  | 1..N (step=1)       | 0.0        | Sequential     |
| CUSTOMER  | 1..N (step=1)       | 0.0        | Sequential     |
| PART      | 1..N (step=1)       | 0.0        | Sequential     |
| PARTSUPP  | 1..N (auto-inc)     | 0.0        | Sequential     |
| LINEITEM  | 1..N (auto-inc)     | 0.0        | Sequential     |
| ORDERS    | sparse (TPC-H gaps) | > 0.0      | Hashed         |

Note: using auto-inc base for LINEITEM/PARTSUPP forces K=0. If you instead
use l_orderkey as base, orders keys are sparse and K > 0 → Hashed.

### 6.2 RoaringBitmap selectivity predictions

At SF=0.001 (6000 LINEITEM rows):

| Field        | Cardinality | Expected bitmap/value  | Bytes/bitmap (approx) |
|-------------|------------|----------------------|----------------------|
| l_returnflag | 3 (A,N,R) | ~2000 entries each   | ~500 B               |
| l_linestatus | 2 (F,O)   | ~3000 entries each   | ~750 B               |
| l_partkey   | ~2000     | ~3 entries each      | ~30 B                |
| l_orderkey  | ~1500     | ~4 entries each      | ~35 B                |

Total index overhead for LINEITEM at SF=0.001: **< 50 KB**  
vs naive `HashMap<Value, Vec<u64>>`: ~6000 × 8 bytes × 4 indexes ≈ **192 KB**  
RoaringBitmap compression ratio: **~4×** on this data

### 6.3 Curvature predictions for Q6 filtered set

Q6 filter: shipdate in [1994-01-01, 1995-01-01], discount in [0.05, 0.07], qty < 24

Expected fraction of LINEITEM: ~1–2% of rows  
Filtered set is a restriction to an open set U ⊆ B (sheaf restriction).  
**Prediction**: K(U) > K(full) because restricting to a narrow band
increases relative variance. `confidence(Q6_result)` < `confidence(full_lineitem)`.

### 6.4 Q3 join complexity scaling

Q3 chain: CUSTOMER(BUILDING) → ORDERS(date<cutoff) → LINEITEM(date>cutoff)

At SF=0.001 (1500 orders, 6000 lineitems):
- building_customers: ~300 (20% of 1500 CUSTOMER rows)
- building_orders: `range_query("o_custkey", 300 keys)` → ~450 orders (bitmap)
- building_lineitems: `range_query("l_orderkey", 450 keys)` → ~1800 lineitems

**O(|left|) scaling test**: double SF → double all counts → runtime doubles (not quadratic)

---

## 7. TDD Test Plan

### Phase 1 — Math validation (Python, no Rust required)

File: `gigi_tpch_tests.py`

Tests that validate theoretical predictions WITHOUT running the Rust engine:
- Bundle K and storage mode prediction for all 8 tables
- RoaringBitmap size estimation
- Join selectivity computation
- Q6 result correctness against hand-computed reference
- O(|left|) scaling: double data size → verify linear time growth
- Confidence score ordering: K(Q6_filtered) > K(full_lineitem)

### Phase 2 — Rust unit tests (in `src/engine.rs` or `src/bundle.rs`)

Tests that run the actual engine on SF=0.001 data:
- Storage mode detected correctly for each table
- Q6 sum matches Python reference (within floating point tolerance)
- Q3 join count matches Python reference  
- Q14 promo_revenue fraction matches reference
- `pullback_curvature` on ORDERS→LINEITEM returns expected ΔK

### Phase 3 — Rust benchmark

File: `benches/tpch_bench.rs`

- Generate SF=0.001, SF=0.01, SF=0.1 deterministically  
- Run Q1, Q3, Q6, Q14 at each scale  
- Report: latency (ns), throughput (rows/s), storage mode, K, bitmap sizes  
- Assert scaling is linear (ratio check: SF=0.01 / SF=0.001 < 15×)

---

## 8. Benchmark Design

### Metrics reported per query per scale factor

```
Qn | SF    | rows_scanned | rows_returned | wall_ms | ns/row | K     | mode
Q6 | 0.001 |         6000 |           120 |    0.12 |   20.0 | 0.082 | sequential
Q6 | 0.01  |        60000 |          1200 |    1.15 |   19.2 | 0.081 | sequential
Q6 | 0.1   |       600000 |         12000 |   11.4  |   19.0 | 0.080 | sequential
```

### Comparison baseline

Postgres 16 numbers (self-reported from community benchmarks, not our run):
- Q6 at SF=1: ~80–120ms with bitmap index on l_shipdate
- Q3 at SF=1: ~500ms–2s depending on planner choices

GIGI targets:
- Q6: sub-20ms at SF=0.1 (no index needed — pure filter scan, K=0 sequential)
- Q3: sub-100ms at SF=0.1 via bitmap join

### Honest scope

GIGI does NOT implement:
- TPC-H Q2, Q4, Q11, Q13, Q15–Q22 (correlated subqueries, HAVING, set operators)  
- Concurrent query execution  
- TPC-H result validation rules (power/throughput test, refresh streams)

A full TPC-H submission requires handling all 22 queries with ACID refresh
streams and independent auditing. We are running **Q-subset** benchmarks
for analytical comparison, not a certifiable TPC-H submission.

---

## 9. Data Generation

### Python (in-process, for tests)

```python
# Deterministic, no dbgen dependency
rng = DeterministicRNG(seed=42)
lineitem = generate_lineitem(sf=0.001, rng=rng)   # 6000 rows
orders   = generate_orders(sf=0.001, rng=rng)     # 1500 rows
customer = generate_customer(sf=0.001, rng=rng)   # 1500 rows
```

### Rust (in-process, for bench)

```rust
let data = TpchData::generate(sf_factor);  // sf_factor = 0.001, 0.01, 0.1
// Returns pre-populated BundleStore instances for all 8 tables
```

### Official (for SF ≥ 1)

```bash
# TPC-H dbgen (C tool, open source)
./dbgen -s 1.0      # generates .tbl files
# Load into GIGI via CSV import
```

---

## 10. File Index

| File                        | Purpose                              |
|----------------------------|--------------------------------------|
| `GIGI_TPCH_SPEC.md`        | This document                        |
| `gigi_tpch_tests.py`       | Phase 1 math validation (Python)     |
| `benches/tpch_bench.rs`    | Phase 3 Rust benchmark               |
| `test_data/tpch/`          | Pre-generated SF=0.001 CSV snapshots |
| `src/engine.rs` (tests)    | Phase 2 Rust unit tests              |
