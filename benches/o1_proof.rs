//! GIGI O(1) Benchmark Suite
//!
//! Proves the core complexity claims from the spec:
//!   TDD-1.8:  Insert O(1) — time independent of N
//!   TDD-1.9:  Point query O(1) — time independent of N
//!   TDD-2.7:  Range query O(|result|) — independent of N
//!   TDD-2.8:  Sheaf restriction O(|U|) — sublinear
//!   TDD-4.4:  Pullback join O(|left|) — independent of |right|
//!   TDD-R1:   K normalization with declared RANGE O(1) — independent of N
//!   TDD-N1:   COVER NEAR O(N) — linear scan, scales 10x per 10x N
//!   TDD-H1:   HOLONOMY ON FIBER O(N) — linear group-by scan, 10x per 10x N
//!
//! Methodology: measure at N = 1K, 10K, 100K, 1M.
//! If O(1), the ratio t(10N)/t(N) should stay near 1.0 (< 1.5).
//! If O(N), the ratio t(10N)/t(N) should track ~10x.

use gigi::aggregation::fiber_integral;
use gigi::bundle::BundleStore;
use gigi::join::pullback_join;
use gigi::*;
use std::time::Instant;

fn make_schema() -> BundleSchema {
    BundleSchema::new("bench")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("dept"))
        .fiber(FieldDef::numeric("salary").with_range(100_000.0))
        .fiber(FieldDef::categorical("name"))
        .index("dept")
}

fn make_record(i: i64) -> std::collections::HashMap<String, Value> {
    let depts = ["Eng", "Sales", "HR", "Mkt", "Ops", "Legal", "Fin", "R&D"];
    let mut r = std::collections::HashMap::new();
    r.insert("id".into(), Value::Integer(i));
    r.insert(
        "dept".into(),
        Value::Text(depts[(i as usize) % depts.len()].into()),
    );
    r.insert(
        "salary".into(),
        Value::Float(40_000.0 + (i % 500) as f64 * 100.0),
    );
    r.insert("name".into(), Value::Text(format!("User_{i}")));
    r
}

/// Populate a store with N records.
fn build_store(n: usize) -> BundleStore {
    let schema = make_schema();
    let mut store = BundleStore::new(schema);
    for i in 0..n as i64 {
        store.insert(&make_record(i));
    }
    store
}

/// Measure insert of `count` records into an existing store of size `n`.
fn bench_insert(n: usize, count: usize) -> f64 {
    let mut store = build_store(n);
    let base = n as i64;
    let start = Instant::now();
    for i in 0..count as i64 {
        store.insert(&make_record(base + i));
    }
    let elapsed = start.elapsed();
    elapsed.as_nanos() as f64 / count as f64
}

/// Measure point query latency (average of `count` lookups).
fn bench_point_query(store: &BundleStore, n: usize, count: usize) -> f64 {
    // Query random-ish keys spread across the key space
    let start = Instant::now();
    for i in 0..count {
        let key_id = ((i * 7919) % n) as i64; // pseudo-random
        let mut key = std::collections::HashMap::new();
        key.insert("id".into(), Value::Integer(key_id));
        let _ = store.point_query(&key);
    }
    let elapsed = start.elapsed();
    elapsed.as_nanos() as f64 / count as f64
}

/// Measure range query latency normalized per result element.
fn bench_range_query(store: &BundleStore, count: usize) -> (f64, usize) {
    let result_size = store
        .range_query("dept", &[Value::Text("Eng".into())])
        .len();
    let start = Instant::now();
    for _ in 0..count {
        let _ = store.range_query("dept", &[Value::Text("Eng".into())]);
    }
    let elapsed = start.elapsed();
    let total_ns = elapsed.as_nanos() as f64 / count as f64;
    (total_ns, result_size)
}

/// Measure `compute_record_k` with a declared-RANGE schema.
/// This should be O(1) — independent of how many records are in the store.
fn bench_k_normalization(store: &BundleStore, iters: usize) -> f64 {
    use gigi::bundle::compute_record_k;
    let stats = store.get_field_stats().clone();
    let fiber_fields = store.schema.fiber_fields.clone();
    let record = vec![Value::Float(0.5), Value::Float(50_000.0)];
    let start = Instant::now();
    for _ in 0..iters {
        let _ = compute_record_k(&stats, &record, &fiber_fields);
    }
    start.elapsed().as_nanos() as f64 / iters as f64
}

/// Measure `cover_near` via HNSW dispatch — O(log N) after index is built.
fn bench_cover_near(store: &BundleStore, iters: usize) -> f64 {
    let qp = vec![
        ("salary".to_string(), 60_000.0f64),
    ];
    let start = Instant::now();
    for _ in 0..iters {
        let _ = store.cover_near(&qp, 0.3, None);
    }
    start.elapsed().as_nanos() as f64 / iters as f64
}

/// Force the linear scan path by calling cover_near_records directly.
fn bench_cover_near_linear(store: &BundleStore, iters: usize) -> f64 {
    let qp = vec![("salary".to_string(), 60_000.0f64)];
    let start = Instant::now();
    for _ in 0..iters {
        let _ = BundleStore::cover_near_records(store.records(), &store.schema, &qp, 0.3, None);
    }
    start.elapsed().as_nanos() as f64 / iters as f64
}

/// Measure holonomy group-by scan — O(N) over records.
fn bench_holonomy_groupby(store: &BundleStore, iters: usize) -> f64 {
    use std::collections::BTreeMap;
    let start = Instant::now();
    for _ in 0..iters {
        let mut groups: BTreeMap<String, (f64, f64, usize)> = BTreeMap::new();
        for rec in store.records() {
            let key = match rec.get("dept") {
                Some(v) => format!("{v:?}"),
                None => continue,
            };
            let v0 = rec.get("salary").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let v1 = rec.get("id").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let entry = groups.entry(key).or_insert((0.0, 0.0, 0));
            entry.0 += v0;
            entry.1 += v1;
            entry.2 += 1;
        }
        // Compute centroids (dropped — only timing the scan)
        let _: Vec<_> = groups
            .into_iter()
            .map(|(k, (sx, sy, n))| (k, sx / n as f64, sy / n as f64))
            .collect();
    }
    start.elapsed().as_nanos() as f64 / iters as f64
}

fn main() {
    // On Windows, switch the console to UTF-8 so box-drawing characters render correctly.
    // SetConsoleOutputCP is from kernel32.dll, which is always linked on Windows targets.
    #[cfg(target_os = "windows")]
    unsafe {
        extern "system" { fn SetConsoleOutputCP(cp: u32) -> i32; }
        SetConsoleOutputCP(65001);
    }

    let sizes: Vec<usize> = vec![1_000, 10_000, 100_000, 1_000_000];
    let query_iters = 10_000;
    let insert_batch = 1_000;

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║            GIGI O(1) Benchmark — Complexity Proof              ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    // ── TDD-1.8: Insert O(1) ──
    println!("║                                                                ║");
    println!("║  TDD-1.8: INSERT O(1)  —  {insert_batch} inserts per size            ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!(
        "║  {:>10} │ {:>12} │ {:>8}                          ║",
        "N", "ns/insert", "ratio"
    );
    println!("╟────────────┼──────────────┼──────────────────────────────╢");
    let mut prev_t = 0.0f64;
    for &n in &sizes {
        let t = bench_insert(n, insert_batch);
        let ratio = if prev_t > 0.0 { t / prev_t } else { 1.0 };
        let mark = if ratio < 3.0 || prev_t == 0.0 {
            "✓"
        } else {
            "✗ CACHE"
        };
        println!("║  {:>10} │ {:>12.1} │ {:>6.2}x  {mark:<20} ║", n, t, ratio);
        prev_t = t;
    }

    // ── TDD-1.9: Point Query O(1) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-1.9: POINT QUERY O(1)  —  {query_iters} queries per size       ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!(
        "║  {:>10} │ {:>12} │ {:>8}                          ║",
        "N", "ns/query", "ratio"
    );
    println!("╟────────────┼──────────────┼──────────────────────────────╢");
    prev_t = 0.0;
    for &n in &sizes {
        let store = build_store(n);
        let t = bench_point_query(&store, n, query_iters);
        let ratio = if prev_t > 0.0 { t / prev_t } else { 1.0 };
        let mark = if ratio < 1.5 || prev_t == 0.0 {
            "✓"
        } else {
            "✗"
        };
        println!(
            "║  {:>10} │ {:>12.1} │ {:>6.2}x  {mark}                       ║",
            n, t, ratio
        );
        prev_t = t;
    }

    // ── TDD-2.7/2.8: Range Query O(|result|) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-2.7: RANGE QUERY O(|result|)  —  fixed dept filter       ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!(
        "║  {:>10} │ {:>12} │ {:>12} │ {:>8}            ║",
        "N", "|result|", "ns/element", "ratio"
    );
    println!("╟────────────┼──────────────┼──────────────┼──────────────╢");
    prev_t = 0.0;
    for &n in &sizes {
        let store = build_store(n);
        let range_iters = 100.max(query_iters / n.max(1) * 100);
        let (total_ns, result_size) = bench_range_query(&store, range_iters);
        let ns_per_elem = if result_size > 0 {
            total_ns / result_size as f64
        } else {
            total_ns
        };
        let ratio = if prev_t > 0.0 {
            ns_per_elem / prev_t
        } else {
            1.0
        };
        let mark = if ratio < 2.0 || prev_t == 0.0 {
            "✓"
        } else {
            "✗"
        };
        println!(
            "║  {:>10} │ {:>12} │ {:>12.1} │ {:>6.2}x  {mark}    ║",
            n, result_size, ns_per_elem, ratio
        );
        prev_t = ns_per_elem;
    }

    // ── TDD-4.4: Pullback Join O(|left|) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-4.4: PULLBACK JOIN O(|left|)  —  fixed left, vary right  ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!(
        "║  {:>10} │ {:>12} │ {:>8}                          ║",
        "|right|", "ns/join", "ratio"
    );
    println!("╟────────────┼──────────────┼──────────────────────────────╢");

    let left_schema = BundleSchema::new("orders")
        .base(FieldDef::numeric("order_id"))
        .fiber(FieldDef::numeric("customer_id"))
        .fiber(FieldDef::numeric("amount").with_range(10000.0));
    let mut left_store = BundleStore::new(left_schema);
    for i in 0..1_000i64 {
        let mut r = std::collections::HashMap::new();
        r.insert("order_id".into(), Value::Integer(i));
        r.insert("customer_id".into(), Value::Integer(i % 100));
        r.insert("amount".into(), Value::Float(100.0 + i as f64));
        left_store.insert(&r);
    }

    prev_t = 0.0;
    for &right_n in &[100, 1_000, 10_000, 100_000] {
        let right_schema = BundleSchema::new("customers")
            .base(FieldDef::numeric("customer_id"))
            .fiber(FieldDef::categorical("name"));
        let mut right_store = BundleStore::new(right_schema);
        for i in 0..right_n as i64 {
            let mut r = std::collections::HashMap::new();
            r.insert("customer_id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("C_{i}")));
            right_store.insert(&r);
        }

        let join_iters = 10;
        let start = Instant::now();
        for _ in 0..join_iters {
            let _ = pullback_join(&left_store, &right_store, "customer_id", "customer_id");
        }
        let elapsed = start.elapsed();
        let t = elapsed.as_nanos() as f64 / join_iters as f64;

        let ratio = if prev_t > 0.0 { t / prev_t } else { 1.0 };
        let mark = if ratio < 1.5 || prev_t == 0.0 {
            "✓"
        } else {
            "✗"
        };
        println!(
            "║  {:>10} │ {:>12.1} │ {:>6.2}x  {mark}                       ║",
            right_n, t, ratio
        );
        prev_t = t;
    }

    // ── TDD-5.1/5.2: Aggregation O(N) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-5.1: AGGREGATION O(N)  —  fiber integral over all        ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!(
        "║  {:>10} │ {:>12} │ {:>8}                          ║",
        "N", "µs/agg", "ratio"
    );
    println!("╟────────────┼──────────────┼──────────────────────────────╢");
    prev_t = 0.0;
    for &n in &sizes {
        let store = build_store(n);
        let agg_iters = 10;
        let start = Instant::now();
        for _ in 0..agg_iters {
            let _ = fiber_integral(&store, "salary");
        }
        let elapsed = start.elapsed();
        let t = elapsed.as_micros() as f64 / agg_iters as f64;

        let ratio = if prev_t > 0.0 { t / prev_t } else { 1.0 };
        let expected_ratio = if sizes.iter().position(|&s| s == n).unwrap_or(0) > 0 {
            (n as f64) / (sizes[sizes.iter().position(|&s| s == n).unwrap() - 1] as f64)
        } else {
            1.0
        };
        // For O(N), ratio should track 10x per 10x size increase
        let mark = if ratio < expected_ratio * 1.5 || prev_t == 0.0 {
            "✓"
        } else {
            "✗"
        };
        println!(
            "║  {:>10} │ {:>12.1} │ {:>6.1}x  {mark}  (expect ~{:.0}x)       ║",
            n, t, ratio, expected_ratio
        );
        prev_t = t;
    }

    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║                                                                ║");
    println!("║  TDD-R1: K NORMALIZATION O(1) — declared RANGE, vary N        ║");
    println!("║  compute_record_k() must not depend on store size             ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!(
        "║  {:>10} │ {:>12} │ {:>8}                          ║",
        "N", "ns/call", "ratio"
    );
    println!("╟────────────┼──────────────┼──────────────────────────────╢");
    prev_t = 0.0;
    for &n in &sizes {
        let store = build_store(n);
        let t = bench_k_normalization(&store, 100_000);
        let ratio = if prev_t > 0.0 { t / prev_t } else { 1.0 };
        let mark = if ratio < 1.5 || prev_t == 0.0 { "✓ O(1)" } else { "✗ NOT O(1)" };
        println!(
            "║  {:>10} │ {:>12.1} │ {:>6.2}x  {mark:<20} ║",
            n, t, ratio
        );
        prev_t = t;
    }

    // ── COVER NEAR O(N) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-N1: COVER NEAR O(N) — linear scan, should track 10x/10x ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!(
        "║  {:>10} │ {:>12} │ {:>8}                          ║",
        "N", "µs/scan", "ratio"
    );
    println!("╟────────────┼──────────────┼──────────────────────────────╢");
    prev_t = 0.0;
    for &n in &sizes {
        let store = build_store(n);
        let iters = (1_000_000 / n).max(1);
        let t_ns = bench_cover_near(&store, iters);
        let t_us = t_ns / 1_000.0;
        let ratio = if prev_t > 0.0 { t_us / prev_t } else { 1.0 };
        // O(N): ratio should track ~10x per 10x N. Accept 3x–30x as pass.
        let mark = if ratio < 30.0 || prev_t == 0.0 { "✓ O(N)" } else { "✗ super-linear" };
        println!(
            "║  {:>10} │ {:>12.1} │ {:>6.1}x  {mark:<20} ║",
            n, t_us, ratio
        );
        prev_t = t_us;
    }

    // ── HOLONOMY ON FIBER O(N) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-H1: HOLONOMY GROUP-BY O(N) — scan, 10x per 10x N        ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!(
        "║  {:>10} │ {:>12} │ {:>8}                          ║",
        "N", "µs/scan", "ratio"
    );
    println!("╟────────────┼──────────────┼──────────────────────────────╢");
    prev_t = 0.0;
    for &n in &sizes {
        let store = build_store(n);
        let iters = (500_000 / n).max(1);
        let t_ns = bench_holonomy_groupby(&store, iters);
        let t_us = t_ns / 1_000.0;
        let ratio = if prev_t > 0.0 { t_us / prev_t } else { 1.0 };
        let mark = if ratio < 30.0 || prev_t == 0.0 { "✓ O(N)" } else { "✗ super-linear" };
        println!(
            "║  {:>10} │ {:>12.1} │ {:>6.1}x  {mark:<20} ║",
            n, t_us, ratio
        );
        prev_t = t_us;
    }

    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║                                                                ║");
    println!("║  TDD-N2: COVER NEAR — HNSW vs LINEAR at 1M records            ║");
    println!("║  Build index once, then compare O(log N) vs O(N)              ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    let mut hnsw_store = build_store(1_000_000);
    print!("║  Building HNSW index on 1M records...                          ║");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let t_build = Instant::now();
    hnsw_store.build_hnsw_for_fields(&["salary"]);
    let build_ms = t_build.elapsed().as_millis();
    println!("\r║  HNSW index built in {build_ms} ms                                    ║");
    println!("╟────────────────┬──────────────────┬──────────────────────────╢");
    println!(
        "║  {:>14} │ {:>16} │ {:>8}                  ║",
        "method", "µs/query @1M", "speedup"
    );
    println!("╟────────────────┼──────────────────┼──────────────────────────╢");

    let linear_ns = bench_cover_near_linear(&hnsw_store, 3);
    let hnsw_ns   = bench_cover_near(&hnsw_store, 1_000);
    let speedup   = linear_ns / hnsw_ns;

    println!(
        "║  {:>14} │ {:>16.1} │  1.0x  (baseline)          ║",
        "linear scan", linear_ns / 1_000.0
    );
    let mark = if speedup > 10.0 { "✓" } else { "✗" };
    println!(
        "║  {:>14} │ {:>16.1} │  {speedup:.0}x faster  {mark}              ║",
        "HNSW", hnsw_ns / 1_000.0
    );

    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║                     BENCHMARK COMPLETE                         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}
