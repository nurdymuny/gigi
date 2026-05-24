//! L3 real-data smoke test (per bee's "test with real data" rule
//! and per IMPLEMENTATION_PLAN.md L3 def-of-done).
//!
//! L3 introduces a cached spectral-gap snapshot on each bundle store
//! (`BundleStore::spectral_gap_cached`) and a Marcella-facing HTTP
//! surface (`GET /v1/bundles/<name>/spectral_gap`) that serves it.
//!
//! This test exercises the producer side on the same 20-record
//! sensor dataset the other Kähler real-data smokes use. It builds
//! a bundle with indexed categorical fields (`status`, `unit`) — the
//! ones that actually have spectral structure in the data — and
//! asserts:
//!
//! 1. The first call to `spectral_gap_cached()` returns a real
//!    snapshot (Some), with all the Marcella v2 §4 contract fields
//!    populated and in mathematically valid ranges.
//! 2. The second call returns the SAME snapshot (cache hit, not
//!    recomputed — same `cached_at`, same `lambda_2`).
//! 3. Inserting a new record invalidates the cache (consumption
//!    draft v2 §4 "Marcella detects drift on cached_at change").
//! 4. After invalidation, the next read recomputes and the snapshot
//!    becomes stable again on a third read.
//! 5. The mathematical invariants Marcella depends on hold on real
//!    data (Cheeger sandwich, mixing-time formula, λ₂ in [0, 2]).
//!
//! Negative case: a 1-record subset still returns None (single
//! record → no spectral structure to report).

#![cfg(feature = "kahler")]

use gigi::types::{BundleSchema, FieldDef, Value};
use gigi::BundleStore;
use std::collections::HashMap;
use std::fs;

fn load_sensor_records() -> Vec<HashMap<String, Value>> {
    let path = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{}/test_data/sensor_data.json", d))
        .expect("CARGO_MANIFEST_DIR not set");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path, e));
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse sensor_data.json");
    parsed
        .as_array()
        .expect("array")
        .iter()
        .map(|item| {
            let obj = item.as_object().expect("object");
            let mut rec = HashMap::new();
            for (k, v) in obj {
                let val = match v {
                    serde_json::Value::String(s) => Value::Text(s.clone()),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Value::Integer(i)
                        } else {
                            Value::Float(n.as_f64().expect("f64"))
                        }
                    }
                    serde_json::Value::Bool(b) => Value::Bool(*b),
                    _ => panic!("unexpected JSON variant"),
                };
                rec.insert(k.clone(), val);
            }
            rec
        })
        .collect()
}

/// Build the sensor schema with two indexed categorical fields. We
/// index `status` and `unit` — both have a small number of distinct
/// values across the 20 records, so the field-index graph is
/// connected with non-trivial spectral structure.
fn sensor_schema() -> BundleSchema {
    BundleSchema::new("sensor_spectral")
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
        .index("status")
        .index("unit")
}

#[test]
fn real_sensor_data_spectral_gap_cache_lifecycle() {
    let records = load_sensor_records();
    assert_eq!(records.len(), 20, "sensor_data.json must have 20 records");

    let mut store = BundleStore::new(sensor_schema());
    for rec in &records {
        store.insert(rec);
    }
    assert_eq!(store.len(), 20);

    // ── First read: cache miss, compute snapshot ──
    let snap1 = store
        .spectral_gap_cached()
        .expect("≥ 2 records → spectral_gap_cached must return Some");

    // λ₂ ∈ [0, 2] (normalized Laplacian spectrum). Real categorical
    // indices give a connected graph so λ₂ > 0.
    assert!(
        snap1.lambda_2 > 0.0 && snap1.lambda_2 <= 2.0,
        "λ₂ on real sensor data must be in (0, 2]; got {}",
        snap1.lambda_2
    );

    // Cheeger sandwich: λ₂/2 ≤ h(G) ≤ √(2 λ₂).
    assert!(
        (snap1.cheeger_lower - snap1.lambda_2 / 2.0).abs() < 1e-12,
        "cheeger_lower contract: λ₂/2 = {}; got {}",
        snap1.lambda_2 / 2.0,
        snap1.cheeger_lower
    );
    let expected_upper = (2.0 * snap1.lambda_2).sqrt();
    assert!(
        (snap1.cheeger_upper - expected_upper).abs() < 1e-12,
        "cheeger_upper contract: √(2λ₂) = {}; got {}",
        expected_upper,
        snap1.cheeger_upper
    );

    // Mixing time formula on real data.
    let expected_mix = ((1.0 / snap1.lambda_2) * (1.0_f64 / 1e-3).ln()).ceil() as u64;
    assert_eq!(
        snap1.mix_time, expected_mix,
        "mix_time on real data must match Marcella formula"
    );

    // cached_at must be non-empty (Marcella keys drift detection
    // off this string).
    assert!(
        !snap1.cached_at.is_empty(),
        "cached_at must be non-empty on real data"
    );

    // ── Second read: cache hit ──
    let snap2 = store
        .spectral_gap_cached()
        .expect("cache hit must still return Some");
    assert_eq!(
        snap1, snap2,
        "two reads with no intervening writes must return identical snapshots"
    );

    // ── Insert: cache invalidates ──
    // Build a synthetic record that conforms to the sensor schema.
    let mut new_rec = HashMap::new();
    new_rec.insert("sensor_id".into(), Value::Text("sensor_99".into()));
    new_rec.insert("timestamp".into(), Value::Integer(9_999));
    new_rec.insert("temperature".into(), Value::Float(25.0));
    new_rec.insert("humidity".into(), Value::Float(50.0));
    new_rec.insert("pressure".into(), Value::Float(1013.25));
    new_rec.insert("unit".into(), Value::Text("celsius".into()));
    new_rec.insert("status".into(), Value::Text("normal".into()));
    store.insert(&new_rec);

    // ── Third read: recomputes a fresh snapshot, then stable again ──
    // We observe invalidation through the public surface (Marcella's
    // surface): the post-insert read must produce a snapshot whose
    // λ₂ differs from the pre-insert λ₂ (the graph grew by one
    // vertex with new categorical values). If the cache hadn't been
    // invalidated, snap3 would still be the cached snap1.
    let snap3 = store
        .spectral_gap_cached()
        .expect("post-insert read must compute a fresh Some");
    let snap4 = store
        .spectral_gap_cached()
        .expect("fourth read is a cache hit");
    assert_eq!(
        snap3, snap4,
        "after invalidation + recompute, cache must be stable again"
    );
    assert!(
        (snap3.lambda_2 - snap1.lambda_2).abs() > 1e-9,
        "insert must invalidate the cache: post-insert λ₂ ({}) should differ from \
         pre-insert λ₂ ({}); identical values mean the stale cache was served",
        snap3.lambda_2,
        snap1.lambda_2
    );

    // Sanity: the recomputed snapshot is still in valid ranges.
    assert!(
        (0.0..=2.0).contains(&snap3.lambda_2),
        "λ₂ after insert out of range: {}",
        snap3.lambda_2
    );
    assert!(
        (snap3.cheeger_lower - snap3.lambda_2 / 2.0).abs() < 1e-12,
        "cheeger_lower contract holds after insert too"
    );

    // ── Diagnostic print (visible under `cargo test -- --nocapture`) ──
    println!(
        "L3 sensor smoke: before-insert λ₂ = {:.6}, mix_time = {}, cheeger ∈ [{:.6}, {:.6}]",
        snap1.lambda_2, snap1.mix_time, snap1.cheeger_lower, snap1.cheeger_upper
    );
    println!(
        "  after-insert λ₂ = {:.6}, mix_time = {}, cheeger ∈ [{:.6}, {:.6}]",
        snap3.lambda_2, snap3.mix_time, snap3.cheeger_lower, snap3.cheeger_upper
    );
}

#[test]
fn real_sensor_data_single_record_subset_returns_none() {
    // Negative case: load the 20 records, but only insert one — the
    // spectral graph is degenerate, Marcella sees a 404 instead of
    // a meaningless λ₂.
    let records = load_sensor_records();
    let mut store = BundleStore::new(sensor_schema());
    store.insert(&records[0]);
    assert_eq!(store.len(), 1);

    assert!(
        store.spectral_gap_cached().is_none(),
        "single-record bundle on real data: spectral_gap_cached must return None \
         (so the HTTP endpoint serves 404)"
    );
}
