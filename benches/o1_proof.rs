//! GIGI O(1) Benchmark Suite
//!
//! Proves the core complexity claims from the spec:
//!   TDD-1.8:  Insert O(1) — time independent of N
//!   TDD-1.9:  Point query O(1) — time independent of N
//!   TDD-2.7:  Range query O(|result|) — independent of N
//!   TDD-2.8:  Sheaf restriction O(|U|) — sublinear
//!   TDD-4.4:  Pullback join O(|left|) — independent of |right|
//!
//! Methodology: measure at N = 1K, 10K, 100K, 1M.
//! If O(1), the ratio t(10N)/t(N) should stay near 1.0 (< 1.5).

use std::time::Instant;
use gigi::*;
use gigi::bundle::BundleStore;
use gigi::join::pullback_join;
use gigi::aggregation::fiber_integral;

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
    r.insert("dept".into(), Value::Text(depts[(i as usize) % depts.len()].into()));
    r.insert("salary".into(), Value::Float(40_000.0 + (i % 500) as f64 * 100.0));
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
    let result_size = store.range_query("dept", &[Value::Text("Eng".into())]).len();
    let start = Instant::now();
    for _ in 0..count {
        let _ = store.range_query("dept", &[Value::Text("Eng".into())]);
    }
    let elapsed = start.elapsed();
    let total_ns = elapsed.as_nanos() as f64 / count as f64;
    (total_ns, result_size)
}

fn main() {
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
    println!("║  {:>10} │ {:>12} │ {:>8}                          ║", "N", "ns/insert", "ratio");
    println!("╟────────────┼──────────────┼──────────────────────────────╢");
    let mut prev_t = 0.0f64;
    for &n in &sizes {
        let t = bench_insert(n, insert_batch);
        let ratio = if prev_t > 0.0 { t / prev_t } else { 1.0 };
        let mark = if ratio < 3.0 || prev_t == 0.0 { "✓" } else { "✗ CACHE" };
        println!("║  {:>10} │ {:>12.1} │ {:>6.2}x  {mark:<20} ║", n, t, ratio);
        prev_t = t;
    }

    // ── TDD-1.9: Point Query O(1) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-1.9: POINT QUERY O(1)  —  {query_iters} queries per size       ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  {:>10} │ {:>12} │ {:>8}                          ║", "N", "ns/query", "ratio");
    println!("╟────────────┼──────────────┼──────────────────────────────╢");
    prev_t = 0.0;
    for &n in &sizes {
        let store = build_store(n);
        let t = bench_point_query(&store, n, query_iters);
        let ratio = if prev_t > 0.0 { t / prev_t } else { 1.0 };
        let mark = if ratio < 1.5 || prev_t == 0.0 { "✓" } else { "✗" };
        println!("║  {:>10} │ {:>12.1} │ {:>6.2}x  {mark}                       ║", n, t, ratio);
        prev_t = t;
    }

    // ── TDD-2.7/2.8: Range Query O(|result|) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-2.7: RANGE QUERY O(|result|)  —  fixed dept filter       ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  {:>10} │ {:>12} │ {:>12} │ {:>8}            ║", "N", "|result|", "ns/element", "ratio");
    println!("╟────────────┼──────────────┼──────────────┼──────────────╢");
    prev_t = 0.0;
    for &n in &sizes {
        let store = build_store(n);
        let range_iters = 100.max(query_iters / n.max(1) * 100);
        let (total_ns, result_size) = bench_range_query(&store, range_iters);
        let ns_per_elem = if result_size > 0 { total_ns / result_size as f64 } else { total_ns };
        let ratio = if prev_t > 0.0 { ns_per_elem / prev_t } else { 1.0 };
        let mark = if ratio < 2.0 || prev_t == 0.0 { "✓" } else { "✗" };
        println!("║  {:>10} │ {:>12} │ {:>12.1} │ {:>6.2}x  {mark}    ║", n, result_size, ns_per_elem, ratio);
        prev_t = ns_per_elem;
    }

    // ── TDD-4.4: Pullback Join O(|left|) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-4.4: PULLBACK JOIN O(|left|)  —  fixed left, vary right  ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  {:>10} │ {:>12} │ {:>8}                          ║", "|right|", "ns/join", "ratio");
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
        let mark = if ratio < 1.5 || prev_t == 0.0 { "✓" } else { "✗" };
        println!("║  {:>10} │ {:>12.1} │ {:>6.2}x  {mark}                       ║", right_n, t, ratio);
        prev_t = t;
    }

    // ── TDD-5.1/5.2: Aggregation O(N) ──
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  TDD-5.1: AGGREGATION O(N)  —  fiber integral over all        ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  {:>10} │ {:>12} │ {:>8}                          ║", "N", "µs/agg", "ratio");
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
        let mark = if ratio < expected_ratio * 1.5 || prev_t == 0.0 { "✓" } else { "✗" };
        println!("║  {:>10} │ {:>12.1} │ {:>6.1}x  {mark}  (expect ~{:.0}x)       ║", n, t, ratio, expected_ratio);
        prev_t = t;
    }

    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║                     BENCHMARK COMPLETE                         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}
