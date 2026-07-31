//! vs_stores ROUND 2 — EMBEDDED lane: gigi as an in-process Rust library.
//!
//! Round 1 measured gigi over HTTP (its deployment shape) against in-process
//! sqlite/duckdb. This binary is the apples-to-apples cell: same dataset, same
//! point-query ids, same warmup(1)+reps(3)/median rule, but gigi linked as a
//! library — `gigi::engine::Engine` opened on a fresh temp dir, no server, no
//! sockets, no JSON wire.
//!
//! Usage:
//!   vs_stores_embedded <data.csv> [n_lookups=2000] [reps=3]
//!   vs_stores_embedded --ids-check <n> <k>     # print the derived id sample
//!                                              # (for cross-checking against
//!                                              # eval_common.point_query_ids)
//!
//! Cells (mirrors benchmarks/vs_stores/run_gigi.py + eval_common.py):
//!   INGEST       fresh Engine in a fresh temp dir per rep; timed window =
//!                engine.batch_insert in 1,000-row batches; rows/sec.
//!   POINT QUERY  2,000 ids derived from seed 20260731+1 by replicating
//!                CPython's `random.Random(seed).sample(range(n_rows), k)`
//!                bit-for-bit (MT19937 + init_by_array + _randbelow + both
//!                sample branches); warm, sequential, per-lookup latency in
//!                MICROSECONDS via std::time::Instant; p50/p95 per rep with
//!                the same linear-interpolation percentile as
//!                eval_common.percentile; median across reps.
//!
//! Prints exactly one JSON blob to stdout. Diagnostics go to stderr.
//!
//! Schema mirrors run_gigi.py: txn_id categorical BASE; merchant, customer
//! categorical fibers; hour, amount numeric fibers.

use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Instant;

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

const SEED_IDS: u64 = 20260731 + 1; // eval_common: random.Random(SEED + 1)
const INSERT_BATCH: usize = 1_000; // eval_common.INSERT_BATCH
const BUNDLE: &str = "bench";
const CSV_HEADER: &str = "txn_id,merchant,customer,hour,amount";

// ─── CPython-compatible MT19937 ─────────────────────────────────────────────
// Reference: CPython Modules/_randommodule.c (init_genrand / init_by_array /
// genrand_uint32) — the exact generator behind `random.Random(int_seed)`.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

struct Mt19937 {
    mt: [u32; N],
    mti: usize,
}

impl Mt19937 {
    fn init_genrand(s: u32) -> Self {
        let mut mt = [0u32; N];
        mt[0] = s;
        for i in 1..N {
            mt[i] = 1_812_433_253u32
                .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        Self { mt, mti: N }
    }

    /// `init_by_array` — CPython seeds `Random(int)` with the absolute value
    /// of the int split into little-endian 32-bit words.
    fn from_int_seed(seed: u64) -> Self {
        let key: Vec<u32> = if seed == 0 {
            vec![0]
        } else {
            let mut v = Vec::new();
            let mut s = seed;
            while s > 0 {
                v.push((s & 0xffff_ffff) as u32);
                s >>= 32;
            }
            v
        };
        let mut r = Self::init_genrand(19_650_218);
        let (mut i, mut j) = (1usize, 0usize);
        let mut k = N.max(key.len());
        while k > 0 {
            r.mt[i] = (r.mt[i]
                ^ (r.mt[i - 1] ^ (r.mt[i - 1] >> 30)).wrapping_mul(1_664_525))
            .wrapping_add(key[j])
            .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                r.mt[0] = r.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            r.mt[i] = (r.mt[i]
                ^ (r.mt[i - 1] ^ (r.mt[i - 1] >> 30)).wrapping_mul(1_566_083_941))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                r.mt[0] = r.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        r.mt[0] = 0x8000_0000;
        r.mti = N;
        r
    }

    fn genrand_u32(&mut self) -> u32 {
        if self.mti >= N {
            for kk in 0..N - M {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] =
                    self.mt[kk + M] ^ (y >> 1) ^ if y & 1 == 1 { MATRIX_A } else { 0 };
            }
            for kk in N - M..N - 1 {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] =
                    self.mt[kk + M - N] ^ (y >> 1) ^ if y & 1 == 1 { MATRIX_A } else { 0 };
            }
            let y = (self.mt[N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
            self.mt[N - 1] = self.mt[M - 1] ^ (y >> 1) ^ if y & 1 == 1 { MATRIX_A } else { 0 };
            self.mti = 0;
        }
        let mut y = self.mt[self.mti];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `getrandbits(k)` for k in 1..=32 (all we need for n <= 2^32).
    fn getrandbits(&mut self, k: u32) -> u32 {
        assert!(k >= 1 && k <= 32, "getrandbits: k out of range");
        self.genrand_u32() >> (32 - k)
    }

    /// `Random._randbelow_with_getrandbits(n)` — rejection sampling.
    fn randbelow(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        let k = 32 - n.leading_zeros(); // n.bit_length()
        let mut r = self.getrandbits(k);
        while r >= n {
            r = self.getrandbits(k);
        }
        r
    }
}

/// CPython `random.sample(range(n), k)` — both branches (pool copy vs
/// selection set), selected by the same setsize rule as Lib/random.py.
fn py_sample_range(rng: &mut Mt19937, n: usize, k: usize) -> Vec<usize> {
    assert!(k <= n, "sample larger than population");
    let mut result = vec![0usize; k];
    let mut setsize: f64 = 21.0;
    if k > 5 {
        // setsize += 4 ** ceil(log(k * 3, 4)); 3k is never an exact power of 4
        // (powers of 4 aren't divisible by 3) so the float log has margin.
        let exp = ((k as f64) * 3.0).ln() / 4.0f64.ln();
        setsize += 4.0f64.powi(exp.ceil() as i32);
    }
    if (n as f64) <= setsize {
        // pool branch: partial Fisher-Yates on an explicit pool
        let mut pool: Vec<usize> = (0..n).collect();
        for i in 0..k {
            let j = rng.randbelow((n - i) as u32) as usize;
            result[i] = pool[j];
            pool[j] = pool[n - i - 1];
        }
    } else {
        // selection-set branch: rejection into a set
        let mut selected: HashSet<u32> = HashSet::new();
        for i in 0..k {
            let mut j = rng.randbelow(n as u32);
            while selected.contains(&j) {
                j = rng.randbelow(n as u32);
            }
            selected.insert(j);
            result[i] = j as usize; // population is range(n)
        }
    }
    result
}

fn txn_id(i: usize) -> String {
    format!("T{i:06}") // eval_common.txn_id: f"T{i:06d}"
}

/// The 2,000 (or k) point-lookup ids — same stream, same order as
/// eval_common.point_query_ids().
fn point_query_ids(n_rows: usize, k: usize) -> Vec<String> {
    let mut rng = Mt19937::from_int_seed(SEED_IDS);
    py_sample_range(&mut rng, n_rows, k)
        .into_iter()
        .map(txn_id)
        .collect()
}

// ─── stats (same conventions as eval_common) ────────────────────────────────

/// Linear-interpolation percentile, identical to eval_common.percentile.
fn percentile(xs: &[f64], p: f64) -> f64 {
    let mut s = xs.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(!s.is_empty(), "empty sample");
    let k = (s.len() - 1) as f64 * (p / 100.0);
    let (lo, hi) = (k.floor() as usize, k.ceil() as usize);
    if lo == hi {
        s[k as usize]
    } else {
        s[lo] + (s[hi] - s[lo]) * (k - lo as f64)
    }
}

/// Median, identical to eval_common.median.
fn median(xs: &[f64]) -> f64 {
    let mut s = xs.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

fn round_to(x: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    (x * f).round() / f
}

// ─── dataset ────────────────────────────────────────────────────────────────

fn read_csv(path: &PathBuf) -> Vec<Record> {
    let f = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("cannot open {}: {e}", path.display()));
    let mut lines = BufReader::new(f).lines();
    let header = lines
        .next()
        .expect("empty csv")
        .expect("csv header read failed");
    assert_eq!(
        header.trim(),
        CSV_HEADER,
        "unexpected csv header (want `{CSV_HEADER}`)"
    );
    let mut rows = Vec::new();
    for line in lines {
        let line = line.expect("csv line read failed");
        if line.is_empty() {
            continue;
        }
        let mut it = line.split(',');
        let t = it.next().expect("txn_id");
        let m = it.next().expect("merchant");
        let c = it.next().expect("customer");
        let h: f64 = it.next().expect("hour").parse().expect("hour parse");
        let a: f64 = it.next().expect("amount").parse().expect("amount parse");
        let mut rec = Record::new();
        rec.insert("txn_id".into(), Value::Text(t.to_string()));
        rec.insert("merchant".into(), Value::Text(m.to_string()));
        rec.insert("customer".into(), Value::Text(c.to_string()));
        rec.insert("hour".into(), Value::Float(h));
        rec.insert("amount".into(), Value::Float(a));
        rows.push(rec);
    }
    rows
}

// ─── engine fixture (idioms from src/ml/test_support.rs) ────────────────────

/// Schema-parity note (round-2 fairness audit, FAIL-1): the HTTP
/// create_bundle route indexes every key field (`schema.index(key)` in
/// src/bin/gigi_stream.rs, "Also index keys"), so round 1's HTTP bundle
/// carried a secondary txn_id index and paid its maintenance inside the
/// ingest clock. The embedded lane must match — hence `.index("txn_id")`.
/// The audit measured the omission at ~11% of embedded ingest throughput;
/// point queries are unaffected (BundleStore::point_query uses the base
/// hash, never field_index).
fn bench_schema() -> BundleSchema {
    BundleSchema::new(BUNDLE)
        .base(FieldDef::categorical("txn_id"))
        .fiber(FieldDef::categorical("merchant"))
        .fiber(FieldDef::categorical("customer"))
        .fiber(FieldDef::numeric("hour"))
        .fiber(FieldDef::numeric("amount"))
        .index("txn_id")
}

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "gigi_vs_stores_embedded_{}_{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// One ingest rep: fresh Engine in a fresh temp dir, timed window = the
/// batched batch_insert loop. Returns (rows_per_sec, dir, engine) — the
/// caller keeps the last rep's engine for the point-query phase.
fn ingest_once(records: &[Record], tag: &str) -> (f64, PathBuf, Engine) {
    let dir = fresh_dir(tag);
    let mut engine = Engine::open(&dir).expect("Engine::open");
    engine.create_bundle(bench_schema()).expect("create_bundle");

    let t0 = Instant::now();
    let mut inserted = 0usize;
    for chunk in records.chunks(INSERT_BATCH) {
        inserted += engine.batch_insert(BUNDLE, chunk).expect("batch_insert");
    }
    let dur = t0.elapsed().as_secs_f64();

    // verification, outside the timed window (mirrors run_gigi.ingest_once)
    assert_eq!(inserted, records.len(), "ingest verification failed");
    (records.len() as f64 / dur, dir, engine)
}

// ─── main ───────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --ids-check <n> <k>: print the derived sample for cross-validation
    // against python eval_common.point_query_ids (no dataset needed).
    if args.get(1).map(String::as_str) == Some("--ids-check") {
        let n: usize = args[2].parse().expect("n");
        let k: usize = args[3].parse().expect("k");
        let ids = point_query_ids(n, k);
        let idx_sum: u64 = ids.iter().map(|t| t[1..].parse::<u64>().unwrap()).sum();
        println!(
            "{}",
            serde_json::json!({
                "n": n, "k": k,
                "first10": &ids[..ids.len().min(10)],
                "last": ids.last(),
                "index_sum": idx_sum,
                "all": ids,
            })
        );
        return;
    }

    if args.len() < 2 {
        eprintln!("usage: vs_stores_embedded <data.csv> [n_lookups=2000] [reps=3]");
        eprintln!("       vs_stores_embedded --ids-check <n> <k>");
        std::process::exit(2);
    }
    let csv_path = PathBuf::from(&args[1]);
    let n_lookups: usize = args
        .get(2)
        .map(|s| s.parse().expect("n_lookups"))
        .unwrap_or(2_000);
    let reps: usize = args.get(3).map(|s| s.parse().expect("reps")).unwrap_or(3);

    eprintln!("reading {} ...", csv_path.display());
    let records = read_csv(&csv_path); // untimed, mirrors eval_common.read_transactions
    let n_rows = records.len();
    assert!(
        n_lookups <= n_rows,
        "n_lookups ({n_lookups}) > n_rows ({n_rows}) — sample larger than population"
    );
    eprintln!("{n_rows} rows");

    // ── A. INGEST — 1 untimed warmup + `reps` timed, fresh engine each ──
    eprintln!("A. ingest (1 warmup + {reps} reps, {n_rows} rows each) ...");
    let mut ingest_reps: Vec<f64> = Vec::new();
    let mut keep: Option<(PathBuf, Engine)> = None;
    for i in 0..=reps {
        let (rps, dir, engine) = ingest_once(&records, &format!("rep{i}"));
        if i == 0 {
            // warmup: result discarded, engine dropped, dir removed
            drop(engine);
            let _ = std::fs::remove_dir_all(&dir);
            continue;
        }
        ingest_reps.push(rps);
        eprintln!("   rep {i}: {:.1} rows/sec", rps);
        if let Some((old_dir, old_engine)) = keep.take() {
            drop(old_engine);
            let _ = std::fs::remove_dir_all(&old_dir);
        }
        keep = Some((dir, engine));
    }
    let (final_dir, engine) = keep.expect("at least one timed ingest rep");

    // ── B. POINT QUERY — same ids as round 1 (seed SEED+1 stream) ──
    let ids = point_query_ids(n_rows, n_lookups);
    let ids_first10: Vec<String> = ids.iter().take(10).cloned().collect();
    let ids_index_sum: u64 = ids.iter().map(|t| t[1..].parse::<u64>().unwrap()).sum();

    // key records prebuilt outside the timed window (the embedded analogue
    // of round 1 pre-quoting the URL ids; disclosed below)
    let keys: Vec<Record> = ids
        .iter()
        .map(|t| {
            let mut k = Record::new();
            k.insert("txn_id".into(), Value::Text(t.clone()));
            k
        })
        .collect();

    let store = engine.bundle(BUNDLE).expect("bundle 'bench' missing");

    // correctness check once, untimed (mirrors run_gigi.py)
    let probe = store.point_query(&keys[0]).expect("probe id not found");
    assert_eq!(
        probe.get("txn_id"),
        Some(&Value::Text(ids[0].clone())),
        "probe returned wrong record"
    );

    eprintln!("B. point queries ({n_lookups} lookups; 1 warmup pass + {reps} timed passes) ...");
    let mut point_reps: Vec<Vec<f64>> = Vec::new(); // per-rep per-lookup micros
    for pass in 0..=reps {
        let mut lats_us = Vec::with_capacity(n_lookups);
        let mut found = 0usize;
        for key in &keys {
            let t0 = Instant::now();
            let hit = store.point_query(key);
            let us = t0.elapsed().as_secs_f64() * 1e6;
            if hit.is_some() {
                found += 1;
            }
            lats_us.push(us);
        }
        assert_eq!(found, n_lookups, "point query miss — ids not all present");
        if pass > 0 {
            point_reps.push(lats_us);
        }
    }

    let pq_reps: Vec<serde_json::Value> = point_reps
        .iter()
        .map(|lats| {
            serde_json::json!({
                "p50_us": round_to(percentile(lats, 50.0), 3),
                "p95_us": round_to(percentile(lats, 95.0), 3),
            })
        })
        .collect();
    let p50s: Vec<f64> = pq_reps.iter().map(|r| r["p50_us"].as_f64().unwrap()).collect();
    let p95s: Vec<f64> = pq_reps.iter().map(|r| r["p95_us"].as_f64().unwrap()).collect();

    // ── report: one JSON blob to stdout ──
    let out = serde_json::json!({
        "system": "gigi_embedded",
        "transport": "in-process — gigi::engine::Engine linked as a Rust library, release build, no HTTP",
        "csv": csv_path.display().to_string(),
        "n_rows": n_rows,
        "protocol": {
            "seed_ids": SEED_IDS,
            "id_derivation": "CPython random.Random(20260732).sample(range(n_rows), n_lookups) replicated bit-for-bit (MT19937 init_by_array + _randbelow + Lib/random.py sample)",
            "insert_batch": INSERT_BATCH,
            "n_point_queries": n_lookups,
            "warmups_per_cell": 1,
            "reps_per_cell": reps,
            "cell_statistic": "median of reps",
            "percentile": "linear interpolation, same convention as eval_common.percentile",
        },
        "ids_first10": ids_first10,
        "ids_index_sum": ids_index_sum,
        "cells": {
            "ingest": {
                "rows": n_rows,
                "unit": "rows_per_sec",
                "reps": ingest_reps.iter().map(|x| round_to(*x, 1)).collect::<Vec<_>>(),
                "median": round_to(median(&ingest_reps), 1),
            },
            "point_query": {
                "n_queries": n_lookups,
                "unit": "us",
                "reps": pq_reps,
                "p50_us_median": round_to(median(&p50s), 3),
                "p95_us_median": round_to(median(&p95s), 3),
            },
        },
        "disclosures": [
            "Embedded lane: all cells in-process via the gigi Rust library — no HTTP round trip, no JSON wire. This is the apples-to-apples counterpart of round 1's in-process sqlite/duckdb cells.",
            "INGEST timed window = the engine.batch_insert loop only (1,000-row batches), including gigi's WAL append + any checkpoint the engine triggers — its real embedded write path. Record structs are built from the CSV before the timed window, mirroring round 1 where python rows were parsed pre-window.",
            "SCHEMA PARITY (post-audit fix): the bundle declares .index(\"txn_id\"), matching round 1's HTTP create_bundle path which indexes every key field — secondary-index maintenance is inside the embedded ingest clock, same as every other system (round-1 fairness rule #2). The audit measured the earlier omission at ~11% of ingest throughput; point queries never touch field_index and are unaffected.",
            "INGEST runs on a fresh Engine in a fresh temp dir per rep (fresh-bundle-per-rep rule from round 1).",
            "POINT QUERY timed window = BundleStore::point_query per lookup only; key Records are prebuilt untimed (the embedded analogue of round 1 pre-quoting URL ids). No response deserialization exists in-process — disclosed as inherent to the embedded shape, not hidden.",
            "Point-query ids are the SAME ids in the SAME order as round 1 (seed 20260732 stream) when n_rows=100000 and n_lookups=2000; cross-check with --ids-check against eval_common.point_query_ids.",
        ],
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());

    drop(store);
    drop(engine);
    let _ = std::fs::remove_dir_all(&final_dir);
}
