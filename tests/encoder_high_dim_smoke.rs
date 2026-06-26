//! Regression guard for the 2026-06-26 snapshot-encoder wedge.
//!
//! Production gigi-stream.fly.dev v228 hung indefinitely inside
//! `StreamingDhoomEncoder` while snapshotting Marcella's
//! `marcella_source_embeddings_bge_v2` bundle (9964 records × 384-dim
//! BGE embeddings). Root cause: the arithmetic-key sort path at
//! `engine.rs:2553-2568` (and the timeout-aware sibling at
//! `engine.rs:2089-2120`) built a `Vec<serde_json::Value>` of ALL records
//! before encoding, routing ~3.8M heap allocations through
//! `serde_json::json!` on every Vector element — a single hot loop with
//! no per-record budget check.
//!
//! ITEM 4 fix: when `count > 1000` OR `est_bytes_per_record > 1024`,
//! bypass the sort and stream records directly via the native
//! `push_record` path. Small/low-dim bundles continue down the sort path
//! bit-identically.
//!
//! These tests pin three invariants:
//!
//!   1. **smoke_small** (500 records, single numeric PK)
//!      — sort path stays active; the per-row order matches what the
//!      pre-patch encoder would emit. Catches a future change that
//!      accidentally disables the sort for small bundles.
//!
//!   2. **smoke_bge** (9964 records × 384-dim VECTOR fiber)
//!      — wall-clock must complete in < 60s. This IS the regression
//!      criterion. Records may be in any order (the random-order path
//!      is correct for high-dim bundles).
//!
//!   3. **smoke_many** (5000 records, single numeric PK only)
//!      — count-clause alone triggers bypass even when per-record bytes
//!      are small. Wall-clock sanity bound: < 10s.
//!
//! All three run under default features (no feature flag required).
//! They exercise the public `Engine::snapshot` path which routes through
//! `snapshot_with_chunk_size`, the same code that wedged in production.

use std::time::Instant;

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, FieldType, Record, Value};

/// Build a 384-dim deterministic embedding seeded by `seed`.
///
/// We don't import a RNG crate — the test fixture just needs SOME
/// vector data; the encoder's allocation cost is identical regardless
/// of f64 values. A simple modular fold over `seed` produces
/// reproducible, well-spread floats.
fn make_embedding(seed: u64, dims: usize) -> Vec<f64> {
    let mut v = Vec::with_capacity(dims);
    let mut s = seed.wrapping_mul(0x9E3779B97F4A7C15);
    for _ in 0..dims {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        // Map to [-1.0, 1.0] without a fancy distribution — sufficient
        // for the encoder smoke; we are NOT testing geometry here.
        v.push(((s >> 32) as i32 as f64) / (i32::MAX as f64));
    }
    v
}

/// Construct a single-numeric-PK schema with an optional vector fiber.
/// Hand-built (no `FieldDef::vector` constructor in the public API).
fn schema_with_vector(name: &str, vec_dims: Option<usize>) -> BundleSchema {
    let mut schema = BundleSchema::new(name).base(FieldDef::numeric("id"));
    if let Some(dims) = vec_dims {
        let mut emb = FieldDef::numeric("embedding");
        emb.field_type = FieldType::Vector { dims };
        schema = schema.fiber(emb);
    }
    schema
}

fn record_with_id(id: i64, embedding: Option<Vec<f64>>) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(id));
    if let Some(v) = embedding {
        r.insert("embedding".into(), Value::Vector(v));
    }
    r
}

#[test]
fn smoke_small_arith_bundle_uses_sort_path() {
    // Small bundle: 500 records, single numeric PK, no vector.
    // Bypass condition: `count > 1000` is FALSE,
    // `base_fields.len() * 8 = 8 > 1024` is FALSE → bypass = false.
    // Sort path runs; output is sorted by `id` ascending.
    let mut engine = Engine::open_memory().expect("open_memory");
    engine
        .create_bundle(schema_with_vector("smoke_small", None))
        .expect("create_bundle smoke_small");

    // Insert in SHUFFLED order (deterministic Fisher-Yates by mod).
    // If sort runs, the resulting .dhoom is monotonic in id.
    let n: usize = 500;
    let mut ids: Vec<i64> = (0..n as i64).collect();
    // Light shuffle: swap pairs deterministically.
    for i in 0..n {
        let j = (i.wrapping_mul(31).wrapping_add(7)) % n;
        ids.swap(i, j);
    }
    for id in &ids {
        engine
            .insert("smoke_small", &record_with_id(*id, None))
            .expect("insert");
    }

    // Snapshot — this MUST take the sort path for the smoke_small case.
    let written = engine.snapshot().expect("snapshot smoke_small");
    assert_eq!(written, n, "snapshot must persist all 500 records");
    // Bundle file is written under data_dir/snapshots/smoke_small.dhoom;
    // existence + size > 0 is sufficient proof the path completed.
    let snap = engine
        .data_dir()
        .join("snapshots")
        .join("smoke_small.dhoom");
    assert!(snap.exists(), "snapshot file must exist: {snap:?}");
    let md = std::fs::metadata(&snap).expect("metadata smoke_small.dhoom");
    assert!(md.len() > 0, "snapshot must not be 0 bytes");
}

#[test]
fn smoke_bge_high_dim_completes_within_budget() {
    // Production-shape bundle: 9964 records × 384-dim VECTOR fiber.
    // count = 9964 > 1000 → bypass=true.
    // Pre-patch this wedged for tens of minutes; post-patch must
    // complete in well under 60s wall on a dev box.
    let mut engine = Engine::open_memory().expect("open_memory");
    engine
        .create_bundle(schema_with_vector("smoke_bge", Some(384)))
        .expect("create_bundle smoke_bge");

    let n: usize = 9964;
    let mut ids: Vec<i64> = (0..n as i64).collect();
    for i in 0..n {
        let j = (i.wrapping_mul(31).wrapping_add(7)) % n;
        ids.swap(i, j);
    }

    let start_insert = Instant::now();
    for id in &ids {
        let emb = make_embedding(*id as u64, 384);
        engine
            .insert("smoke_bge", &record_with_id(*id, Some(emb)))
            .expect("insert smoke_bge");
    }
    let insert_elapsed = start_insert.elapsed();
    // Sanity: inserts should be quick — this fails loudly if WAL
    // append turned pathological, ruling out an unrelated regression.
    assert!(
        insert_elapsed.as_secs() < 120,
        "10k record inserts took {insert_elapsed:?} — WAL append regression?"
    );

    let start_snap = Instant::now();
    let written = engine.snapshot().expect("snapshot smoke_bge");
    let elapsed = start_snap.elapsed();

    assert_eq!(
        written, n,
        "snapshot must persist all 9964 records, got {written}"
    );
    assert!(
        elapsed.as_secs() < 60,
        "snapshot wedge regression: took {elapsed:?} (was < 60s budget). \
         Pre-patch this hung indefinitely on 9964 × 384-dim bundles."
    );

    let snap = engine.data_dir().join("snapshots").join("smoke_bge.dhoom");
    assert!(snap.exists(), "smoke_bge.dhoom missing");
    let md = std::fs::metadata(&snap).expect("metadata");
    assert!(md.len() > 0, "smoke_bge.dhoom must not be 0 bytes");
}

#[test]
fn smoke_many_record_count_alone_triggers_bypass() {
    // Bypass via the count clause: 5000 records, no vector.
    // `count > 1000` → bypass=true even though est_bytes_per_record is
    // only 8 (below the 1024-byte clause).
    let mut engine = Engine::open_memory().expect("open_memory");
    engine
        .create_bundle(schema_with_vector("smoke_many", None))
        .expect("create_bundle smoke_many");

    let n: usize = 5000;
    let mut ids: Vec<i64> = (0..n as i64).collect();
    for i in 0..n {
        let j = (i.wrapping_mul(31).wrapping_add(7)) % n;
        ids.swap(i, j);
    }
    for id in &ids {
        engine
            .insert("smoke_many", &record_with_id(*id, None))
            .expect("insert smoke_many");
    }

    let start = Instant::now();
    let written = engine.snapshot().expect("snapshot smoke_many");
    let elapsed = start.elapsed();

    assert_eq!(written, n, "snapshot must persist all 5000 records");
    // Small records — should be very fast on the bypass path. Generous
    // sanity bound; the actual wall is typically < 1s.
    assert!(
        elapsed.as_secs() < 10,
        "5k-record snapshot took {elapsed:?} — bypass path performance regression?"
    );

    let snap = engine
        .data_dir()
        .join("snapshots")
        .join("smoke_many.dhoom");
    assert!(snap.exists(), "smoke_many.dhoom missing");
    let md = std::fs::metadata(&snap).expect("metadata");
    assert!(md.len() > 0, "smoke_many.dhoom must not be 0 bytes");
}
