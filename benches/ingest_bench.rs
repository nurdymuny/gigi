//! GIGI Ingest Benchmark — Hashed vs Sequential (Flat K=0)
//!
//! Measures raw insert throughput to compare:
//!   HashMap mode (curved base)  vs  Vec mode (flat base, memcpy-speed)
//!
//! Methodology: timed insertion of N records (1K → 1M), 3 runs each, best-of-3.

use std::collections::HashMap;
use std::time::Instant;
use gigi::*;
use gigi::bundle::BundleStore;

// ── Schema & Record Factories ───────────────────────────────────────

fn ts_schema() -> BundleSchema {
    BundleSchema::new("timeseries")
        .base(FieldDef::numeric("ts"))
        .fiber(FieldDef::numeric("cpu").with_range(100.0))
        .fiber(FieldDef::numeric("mem").with_range(100.0))
        .fiber(FieldDef::numeric("disk").with_range(100.0))
        .fiber(FieldDef::numeric("net").with_range(1_000_000.0))
}

fn make_record(ts: i64) -> HashMap<String, Value> {
    let mut r = HashMap::new();
    r.insert("ts".into(), Value::Integer(ts));
    r.insert("cpu".into(), Value::Float(20.0 + (ts % 80) as f64));
    r.insert("mem".into(), Value::Float(30.0 + (ts % 60) as f64));
    r.insert("disk".into(), Value::Float(10.0 + (ts % 40) as f64));
    r.insert("net".into(), Value::Float((ts % 100_000) as f64 * 10.0));
    r
}

// ── Benchmark Harness ───────────────────────────────────────────────

struct BenchResult {
    ns_per_insert: f64,
    recs_per_sec: f64,
    mb_per_sec: f64,
}

/// Benchmark: insert `n` records into a Hashed (curved) store.
fn bench_hashed(n: usize, runs: usize) -> BenchResult {
    let schema = ts_schema();
    let record_bytes = 5 * 8; // 5 fields × 8 bytes rough estimate

    let mut best_ns = f64::MAX;
    for _ in 0..runs {
        let mut store = BundleStore::with_geometry(schema.clone(), BaseGeometry::Curved);
        let start = Instant::now();
        for i in 0..n as i64 {
            store.insert(&make_record(i * 10)); // step=10, arithmetic — but forced curved
        }
        let elapsed = start.elapsed().as_nanos() as f64;
        let ns = elapsed / n as f64;
        if ns < best_ns {
            best_ns = ns;
        }
    }

    BenchResult {
        ns_per_insert: best_ns,
        recs_per_sec: 1e9 / best_ns,
        mb_per_sec: (1e9 / best_ns) * record_bytes as f64 / (1024.0 * 1024.0),
    }
}

/// Benchmark: insert `n` records into a Sequential (flat K=0) store.
fn bench_sequential(n: usize, runs: usize) -> BenchResult {
    let schema = ts_schema();
    let record_bytes = 5 * 8;

    let mut best_ns = f64::MAX;
    for _ in 0..runs {
        let mut store = BundleStore::with_geometry(
            schema.clone(),
            BaseGeometry::Flat {
                start: 0,
                step: 10,
                key_field: "ts".into(),
            },
        );
        let start = Instant::now();
        for i in 0..n as i64 {
            store.insert(&make_record(i * 10));
        }
        let elapsed = start.elapsed().as_nanos() as f64;
        let ns = elapsed / n as f64;
        if ns < best_ns {
            best_ns = ns;
        }
    }

    BenchResult {
        ns_per_insert: best_ns,
        recs_per_sec: 1e9 / best_ns,
        mb_per_sec: (1e9 / best_ns) * record_bytes as f64 / (1024.0 * 1024.0),
    }
}

/// Benchmark: insert `n` records with auto-detection (starts Hashed, switches to Sequential).
fn bench_autodetect(n: usize, runs: usize) -> BenchResult {
    let schema = ts_schema();
    let record_bytes = 5 * 8;

    let mut best_ns = f64::MAX;
    for _ in 0..runs {
        let mut store = BundleStore::new(schema.clone());
        let start = Instant::now();
        for i in 0..n as i64 {
            store.insert(&make_record(i * 10));
        }
        let elapsed = start.elapsed().as_nanos() as f64;
        let ns = elapsed / n as f64;
        if ns < best_ns {
            best_ns = ns;
        }
    }

    BenchResult {
        ns_per_insert: best_ns,
        recs_per_sec: 1e9 / best_ns,
        mb_per_sec: (1e9 / best_ns) * record_bytes as f64 / (1024.0 * 1024.0),
    }
}

/// Benchmark: point query on Hashed vs Sequential stores.
fn bench_point_queries(n: usize, queries: usize, runs: usize) -> (f64, f64) {
    let schema = ts_schema();

    // Build Hashed store
    let mut hashed = BundleStore::with_geometry(schema.clone(), BaseGeometry::Curved);
    for i in 0..n as i64 {
        hashed.insert(&make_record(i * 10));
    }

    // Build Sequential store
    let mut sequential = BundleStore::with_geometry(
        schema.clone(),
        BaseGeometry::Flat { start: 0, step: 10, key_field: "ts".into() },
    );
    for i in 0..n as i64 {
        sequential.insert(&make_record(i * 10));
    }

    let mut best_h = f64::MAX;
    let mut best_s = f64::MAX;

    for _ in 0..runs {
        // Query Hashed
        let start = Instant::now();
        for q in 0..queries {
            let key_val = ((q * 7919) % n) as i64 * 10;
            let mut key = HashMap::new();
            key.insert("ts".into(), Value::Integer(key_val));
            let _ = hashed.point_query(&key);
        }
        let ns = start.elapsed().as_nanos() as f64 / queries as f64;
        if ns < best_h { best_h = ns; }

        // Query Sequential
        let start = Instant::now();
        for q in 0..queries {
            let key_val = ((q * 7919) % n) as i64 * 10;
            let mut key = HashMap::new();
            key.insert("ts".into(), Value::Integer(key_val));
            let _ = sequential.point_query(&key);
        }
        let ns = start.elapsed().as_nanos() as f64 / queries as f64;
        if ns < best_s { best_s = ns; }
    }

    (best_h, best_s)
}

/// Benchmark: batch_insert `n` records into a Hashed (curved) store.
fn bench_batch_hashed(n: usize, runs: usize) -> BenchResult {
    let schema = ts_schema();
    let record_bytes = 5 * 8;

    let records: Vec<HashMap<String, Value>> = (0..n as i64)
        .map(|i| make_record(i * 10))
        .collect();

    let mut best_ns = f64::MAX;
    for _ in 0..runs {
        let mut store = BundleStore::with_geometry(schema.clone(), BaseGeometry::Curved);
        let start = Instant::now();
        store.batch_insert(&records);
        let elapsed = start.elapsed().as_nanos() as f64;
        let ns = elapsed / n as f64;
        if ns < best_ns {
            best_ns = ns;
        }
    }

    BenchResult {
        ns_per_insert: best_ns,
        recs_per_sec: 1e9 / best_ns,
        mb_per_sec: (1e9 / best_ns) * record_bytes as f64 / (1024.0 * 1024.0),
    }
}

/// Benchmark: batch_insert `n` records into a Sequential (flat K=0) store.
fn bench_batch_sequential(n: usize, runs: usize) -> BenchResult {
    let schema = ts_schema();
    let record_bytes = 5 * 8;

    let records: Vec<HashMap<String, Value>> = (0..n as i64)
        .map(|i| make_record(i * 10))
        .collect();

    let mut best_ns = f64::MAX;
    for _ in 0..runs {
        let mut store = BundleStore::with_geometry(
            schema.clone(),
            BaseGeometry::Flat { start: 0, step: 10, key_field: "ts".into() },
        );
        let start = Instant::now();
        store.batch_insert(&records);
        let elapsed = start.elapsed().as_nanos() as f64;
        let ns = elapsed / n as f64;
        if ns < best_ns {
            best_ns = ns;
        }
    }

    BenchResult {
        ns_per_insert: best_ns,
        recs_per_sec: 1e9 / best_ns,
        mb_per_sec: (1e9 / best_ns) * record_bytes as f64 / (1024.0 * 1024.0),
    }
}

// ── Pretty Printer ──────────────────────────────────────────────────

fn fmt_rps(rps: f64) -> String {
    if rps >= 1e6 {
        format!("{:.2}M", rps / 1e6)
    } else if rps >= 1e3 {
        format!("{:.0}K", rps / 1e3)
    } else {
        format!("{:.0}", rps)
    }
}

fn fmt_mb(mb: f64) -> String {
    if mb >= 1024.0 {
        format!("{:.1} GB/s", mb / 1024.0)
    } else {
        format!("{:.1} MB/s", mb)
    }
}

fn main() {
    let sizes = [1_000usize, 10_000, 100_000, 1_000_000];
    let runs = 3;

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║           GIGI INGEST BENCHMARK — Single vs Batch, Hashed vs Vec            ║");
    println!("║           5 fiber fields × 8 bytes ≈ 40 bytes/record                       ║");
    println!("║           Best of {runs} runs per configuration                                  ║");
    println!("╠══════════════════════════════════════════════════════════════════════════════╣");

    // Run all benchmarks once, cache results
    let mut results: Vec<(usize, BenchResult, BenchResult, BenchResult, BenchResult, BenchResult)> = Vec::new();

    for &n in &sizes {
        eprint!("  Benchmarking N={n}...");
        let h  = bench_hashed(n, runs);
        let s  = bench_sequential(n, runs);
        let a  = bench_autodetect(n, runs);
        let bh = bench_batch_hashed(n, runs);
        let bs = bench_batch_sequential(n, runs);
        eprintln!(" done");
        results.push((n, h, s, a, bh, bs));
    }

    // ── SINGLE INSERT: Hashed vs Vec ──
    println!("║                                                                            ║");
    println!("║  SINGLE INSERT — HashMap vs Vec                                            ║");
    println!("╠═══════════╤══════════════════════╤══════════════════════╤════════════════════╣");
    println!("║           │      HashMap (K>0)   │     Vec (K=0 flat)  │      Speedup       ║");
    println!("║     N     │   ns/ins │    rec/s   │   ns/ins │   rec/s  │                    ║");
    println!("╟───────────┼──────────┼───────────┼──────────┼──────────┼────────────────────╢");
    for (n, h, s, _, _, _) in &results {
        let speedup = h.ns_per_insert / s.ns_per_insert;
        println!(
            "║ {:>9} │ {:>8.1} │ {:>9} │ {:>8.1} │ {:>8} │ {:>5.2}x {:>11} ║",
            n, h.ns_per_insert, fmt_rps(h.recs_per_sec),
            s.ns_per_insert, fmt_rps(s.recs_per_sec), speedup,
            if speedup >= 1.5 { "faster ✓" } else if speedup >= 1.0 { "faster" } else { "slower ✗" }
        );
    }

    // ── BATCH INSERT: Hashed vs Vec ──
    println!("╠═══════════╤══════════════════════╤══════════════════════╤════════════════════╣");
    println!("║           │  batch HashMap (K>0) │   batch Vec (K=0)   │      Speedup       ║");
    println!("║     N     │   ns/ins │    rec/s   │   ns/ins │   rec/s  │                    ║");
    println!("╟───────────┼──────────┼───────────┼──────────┼──────────┼────────────────────╢");
    for (n, _, _, _, bh, bs) in &results {
        let speedup = bh.ns_per_insert / bs.ns_per_insert;
        println!(
            "║ {:>9} │ {:>8.1} │ {:>9} │ {:>8.1} │ {:>8} │ {:>5.2}x {:>11} ║",
            n, bh.ns_per_insert, fmt_rps(bh.recs_per_sec),
            bs.ns_per_insert, fmt_rps(bs.recs_per_sec), speedup,
            if speedup >= 1.5 { "faster ✓" } else if speedup >= 1.0 { "faster" } else { "slower ✗" }
        );
    }

    // ── BATCH vs SINGLE (HashMap) ──
    println!("╠═══════════╤══════════════════════╤══════════════════════╤════════════════════╣");
    println!("║           │  single insert (K>0) │  batch_insert (K>0) │    batch speedup   ║");
    println!("║     N     │   ns/ins │    rec/s   │   ns/ins │   rec/s  │                    ║");
    println!("╟───────────┼──────────┼───────────┼──────────┼──────────┼────────────────────╢");
    for (n, h, _, _, bh, _) in &results {
        let speedup = h.ns_per_insert / bh.ns_per_insert;
        println!(
            "║ {:>9} │ {:>8.1} │ {:>9} │ {:>8.1} │ {:>8} │ {:>5.2}x {:>11} ║",
            n, h.ns_per_insert, fmt_rps(h.recs_per_sec),
            bh.ns_per_insert, fmt_rps(bh.recs_per_sec), speedup,
            if speedup >= 1.1 { "faster ✓" } else { "≈ same" }
        );
    }

    // ── BATCH vs SINGLE (Vec) ──
    println!("╠═══════════╤══════════════════════╤══════════════════════╤════════════════════╣");
    println!("║           │  single insert (K=0) │  batch_insert (K=0) │    batch speedup   ║");
    println!("║     N     │   ns/ins │    rec/s   │   ns/ins │   rec/s  │                    ║");
    println!("╟───────────┼──────────┼───────────┼──────────┼──────────┼────────────────────╢");
    for (n, _, s, _, _, bs) in &results {
        let speedup = s.ns_per_insert / bs.ns_per_insert;
        println!(
            "║ {:>9} │ {:>8.1} │ {:>9} │ {:>8.1} │ {:>8} │ {:>5.2}x {:>11} ║",
            n, s.ns_per_insert, fmt_rps(s.recs_per_sec),
            bs.ns_per_insert, fmt_rps(bs.recs_per_sec), speedup,
            if speedup >= 1.1 { "faster ✓" } else { "≈ same" }
        );
    }

    // ── BANDWIDTH ──
    println!("╠═══════════╤═══════════════════════════════════════════════════════════════════╣");
    println!("║           │              Effective Ingest Bandwidth                          ║");
    println!("║     N     │  single HashMap │  single Vec  │  batch Vec   │   Druid ref     ║");
    println!("╟───────────┼─────────────────┼──────────────┼──────────────┼─────────────────╢");
    for (n, h, s, _, _, bs) in &results {
        println!(
            "║ {:>9} │ {:>15} │ {:>12} │ {:>12} │ ~200 MB/s (ref) ║",
            n, fmt_mb(h.mb_per_sec), fmt_mb(s.mb_per_sec), fmt_mb(bs.mb_per_sec),
        );
    }

    println!("╠══════════════════════════════════════════════════════════════════════════════╣");
    println!("║  K=0 detection: arithmetic keys → auto-switch HashMap → Vec                ║");
    println!("║  batch_insert(): deferred detection + single promotion pass                ║");
    println!("║  Geometry: K=0 ⇒ E = B × F ⇒ index(k) = (k-start)/step                   ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    println!();

    // ── Summary ──
    if let Some((_, h, s, _, bh, bs)) = results.last() {
        println!("SUMMARY @ 1M records:");
        println!(
            "  Single:  HashMap {:.0} ns ({}) │ Vec {:.0} ns ({})",
            h.ns_per_insert, fmt_rps(h.recs_per_sec),
            s.ns_per_insert, fmt_rps(s.recs_per_sec),
        );
        println!(
            "  Batch:   HashMap {:.0} ns ({}) │ Vec {:.0} ns ({})",
            bh.ns_per_insert, fmt_rps(bh.recs_per_sec),
            bs.ns_per_insert, fmt_rps(bs.recs_per_sec),
        );
        println!(
            "  Batch speedup:  HashMap {:.2}x │ Vec {:.2}x",
            h.ns_per_insert / bh.ns_per_insert,
            s.ns_per_insert / bs.ns_per_insert,
        );
    }
    println!();
}
