//! Parallel read-scaling measurement for the "no parallelism" caveat.
//!
//! GIGI has no intra-query parallelism (no rayon, no par_iter, no
//! worker pool inside a single query). It DOES have inter-query
//! parallelism: `ConcurrentEngine` (src/concurrent.rs) wraps the
//! engine in `Arc<RwLock<Engine>>`, and the production server
//! (`src/bin/gigi_stream.rs`, `#[tokio::main]` multi-thread runtime,
//! `StreamState::engine_read`) serves every read handler under a
//! shared read lock. That means N concurrent readers execute point
//! queries simultaneously on N OS threads.
//!
//! This test measures that: same total query count, 1 thread vs N
//! threads, and reports the speedup. It is `#[ignore]`d because it is
//! a timing measurement, not a correctness gate — timing assertions
//! are flaky in CI. Run it explicitly:
//!
//! ```text
//! cargo test --release --test parallel_read_scaling -- --ignored --nocapture
//! ```

use gigi::bundle::{BundleStore, QueryCondition};
use gigi::concurrent::ConcurrentEngine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::sync::Arc;
use std::time::Instant;

const N_RECORDS: i64 = 200_000;
const TOTAL_QUERIES: i64 = 400_000;

fn build_engine(dir: &std::path::Path) -> ConcurrentEngine {
    let _ = std::fs::remove_dir_all(dir);
    let engine = ConcurrentEngine::open(dir).unwrap();
    let schema = BundleSchema::new("scaling")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("name"))
        .fiber(FieldDef::numeric("val").with_range(1_000_000.0));
    engine.create_bundle(schema).unwrap();

    let records: Vec<Record> = (0..N_RECORDS)
        .map(|i| {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("row_{i}")));
            r.insert("val".into(), Value::Float(i as f64));
            r
        })
        .collect();
    engine.write(|e| e.batch_insert("scaling", &records)).unwrap().unwrap();
    engine
}

fn run_with_threads(engine: &ConcurrentEngine, threads: i64) -> f64 {
    let per_thread = TOTAL_QUERIES / threads;
    let engine = Arc::new(engine.clone());
    let start = Instant::now();
    let mut handles = Vec::new();
    for t in 0..threads {
        let eng = Arc::clone(&engine);
        handles.push(std::thread::spawn(move || {
            let mut hits = 0u64;
            for i in 0..per_thread {
                let id = (t * per_thread + i) % N_RECORDS;
                let mut key = Record::new();
                key.insert("id".into(), Value::Integer(id));
                if eng.point_query("scaling", &key).unwrap().is_some() {
                    hits += 1;
                }
            }
            hits
        }));
    }
    let mut total_hits = 0u64;
    for h in handles {
        total_hits += h.join().unwrap();
    }
    let elapsed = start.elapsed().as_secs_f64();
    assert!(total_hits > 0, "queries must actually hit records");
    (threads * per_thread) as f64 / elapsed
}

#[test]
#[ignore = "timing measurement, not a correctness gate"]
fn parallel_point_query_scaling() {
    let dir = std::env::temp_dir().join("gigi_parallel_read_scaling");
    let engine = build_engine(&dir);

    // Warm-up so page-cache / allocator effects don't land on thread 1.
    let _ = run_with_threads(&engine, 1);

    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    println!("available_parallelism = {cpus}");
    println!("records = {N_RECORDS}, total point queries per run = {TOTAL_QUERIES}");

    let mut baseline = 0.0f64;
    for threads in [1i64, 2, 4, 8] {
        let qps = run_with_threads(&engine, threads);
        if threads == 1 {
            baseline = qps;
        }
        println!(
            "threads={threads:<2} throughput={qps:>12.0} q/s   speedup={:.2}x",
            qps / baseline
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// ── Optimizer effect: is there a physical query optimizer on the hot path? ──
//
// `BundleStore::filtered_query_ex` (src/bundle.rs:2339) runs
// `partition_conditions` → `intersect_bitmaps`, and takes a streaming
// early-terminating path when no sort is requested. This measures the two
// decisions that path makes:
//
//   1. indexed predicate  → Roaring bitmap lookup, no record touched
//      non-indexed        → residual filter over every record
//   2. LIMIT with no sort → stop at offset+limit instead of buffering O(N)
//
// Same selectivity in both arms so the only difference is the plan.

const OPT_N: i64 = 500_000;

fn build_opt_store() -> BundleStore {
    let schema = BundleSchema::new("opt")
        .base(FieldDef::numeric("id"))
        // indexed — eligible for the bitmap path
        .fiber(FieldDef::categorical("dept"))
        // NOT indexed — identical values, forced through the residual filter
        .fiber(FieldDef::categorical("dept_unindexed"))
        .fiber(FieldDef::numeric("val").with_range(1_000_000.0))
        .index("dept");
    let mut store = BundleStore::with_geometry(
        schema,
        gigi::bundle::BaseGeometry::Flat {
            start: 0,
            step: 1,
            key_field: "id".into(),
        },
    );
    let records: Vec<Record> = (0..OPT_N)
        .map(|i| {
            let bucket = format!("d{}", i % 100); // 1% selectivity per value
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("dept".into(), Value::Text(bucket.clone()));
            r.insert("dept_unindexed".into(), Value::Text(bucket));
            r.insert("val".into(), Value::Float(i as f64));
            r
        })
        .collect();
    store.batch_insert(&records);
    store
}

fn time_query(store: &BundleStore, conds: &[QueryCondition], limit: Option<usize>, iters: u32) -> (f64, usize) {
    // warm
    let warm = store.filtered_query(conds, None, false, limit, None);
    let start = Instant::now();
    let mut sink = 0usize;
    for _ in 0..iters {
        sink += store.filtered_query(conds, None, false, limit, None).len();
    }
    let us = start.elapsed().as_secs_f64() * 1e6 / iters as f64;
    assert!(sink > 0);
    (us, warm.len())
}

#[test]
#[ignore = "timing measurement, not a correctness gate"]
fn optimizer_index_vs_scan() {
    let store = build_opt_store();
    println!("records = {OPT_N}, 100 distinct values → 1% selectivity per predicate");

    let indexed = [QueryCondition::Eq("dept".into(), Value::Text("d7".into()))];
    let unindexed = [QueryCondition::Eq(
        "dept_unindexed".into(),
        Value::Text("d7".into()),
    )];

    // Confirm the executor really classifies them differently.
    let (idx_conds, idx_residual) = store.partition_conditions(&indexed);
    let (un_conds, un_residual) = store.partition_conditions(&unindexed);
    println!(
        "plan(dept)            → bitmap={} residual={}  est_card={}",
        idx_conds.len(),
        idx_residual.len(),
        idx_conds
            .first()
            .map(|c| store.estimate_selectivity(c))
            .unwrap_or(0)
    );
    println!(
        "plan(dept_unindexed)  → bitmap={} residual={}",
        un_conds.len(),
        un_residual.len()
    );

    let (t_idx, n_idx) = time_query(&store, &indexed, None, 20);
    let (t_scan, n_scan) = time_query(&store, &unindexed, None, 20);
    assert_eq!(n_idx, n_scan, "both arms must return the same rows");
    println!("full result set ({n_idx} rows):");
    println!("  bitmap index path  {t_idx:>10.1} µs");
    println!("  residual scan path {t_scan:>10.1} µs   → optimizer wins {:.1}x", t_scan / t_idx);

    // LIMIT early termination on the *non*-indexed predicate: no index can
    // help, so any speedup here is the streaming fast path alone.
    let (t_scan_all, _) = time_query(&store, &unindexed, None, 20);
    let (t_scan_lim, n_lim) = time_query(&store, &unindexed, Some(10), 20);
    println!("LIMIT 10 on the non-indexed predicate ({n_lim} rows):");
    println!("  no LIMIT           {t_scan_all:>10.1} µs");
    println!("  LIMIT 10           {t_scan_lim:>10.1} µs   → early exit {:.1}x", t_scan_all / t_scan_lim);
}
