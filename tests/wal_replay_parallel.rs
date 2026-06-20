//! Parallel WAL replay byte-identity contract.
//!
//! GIGI's per-startup WAL replay is on the critical path of every
//! restart. Production observation (2026-06-20 hygiene audit) shows
//! the sequential `do_replay` takes ~11 minutes against 4529 bundles
//! and 12.16M records on every gigi-stream restart, during which all
//! requests 503. The fix is per-bundle parallelism with the three
//! gauge passes preserved as serial barriers.
//!
//! This test is the bit-identity gate that the parallel path MUST
//! pass before it ships. The rule (per Sprint B revert lesson, t002
//! in claude_substrate_v0): ship only if BOTH bit-identity and
//! wall-clock win. A parallel scheme that wins on wall-clock but
//! diverges on bit-identity is rejected without negotiation.
//!
//! Contract (locked):
//!
//!   For any data directory `D` with a valid WAL, the post-replay
//!   `Engine` state produced by `ReplayMode::Sequential` MUST equal
//!   the state produced by `ReplayMode::Parallel` on:
//!     1. sorted set of bundle names
//!     2. per-bundle record count
//!     3. per-bundle record-content SHA-256 (sorted by base point
//!        + fiber values, stable encoding)
//!
//! Equivalently: the sequential and parallel paths must produce the
//! same `state_hash`. The hash uses LE u64 bundle counts plus per-
//! bundle sorted-record-content SHA-256 — finer-grained than the
//! benchmark's count-only floor, so it catches the case where a
//! parallel scheme drops or reorders records inside a single bundle.
//!
//! The fixture is built in-process: small enough to run inside the
//! 902/0 suite, broad enough to exercise CreateBundle / Insert /
//! Update / Delete / DropBundle / Checkpoint across multiple
//! bundles. The cross-bundle independence claim is exercised by 8
//! parallel bundles each with their own write pattern.

use std::path::Path;

use gigi::engine::{Engine, ReplayMode};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use sha2::{Digest, Sha256};

/// Build a representative WAL into `dir`. Drops the engine cleanly on
/// return so the WAL is fsync'd and ready for replay-by-reopen.
fn build_fixture(dir: &Path) {
    let mut engine = Engine::open(dir).expect("engine open (build)");

    // 8 bundles with different write patterns. Per-bundle independence
    // is the central correctness claim of the parallel scheme; running
    // multiple bundles with overlapping insert / update / delete
    // sequences in the same WAL stresses that.
    let bundles = [
        "alpha", "beta", "gamma", "delta",
        "epsilon", "zeta", "eta", "theta",
    ];
    for name in &bundles {
        let schema = BundleSchema::new(name)
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("label"))
            .fiber(FieldDef::numeric("value").with_range(10_000.0));
        engine.create_bundle(schema).expect("create_bundle");
    }

    // Phase A — inserts across all bundles, interleaved so the WAL
    // entry order is genuinely cross-bundle. 200 records per bundle.
    for i in 0..200i64 {
        for (b_idx, name) in bundles.iter().enumerate() {
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(i + (b_idx as i64) * 10_000));
            rec.insert(
                "label".into(),
                Value::Text(format!("{name}_{i}")),
            );
            rec.insert(
                "value".into(),
                Value::Float((i as f64) * 1.5 + (b_idx as f64)),
            );
            engine.insert(name, &rec).expect("insert");
        }
    }

    // Checkpoint in the middle so the replay exercises the
    // pre-checkpoint snapshot-load path (no-op here since we haven't
    // snapshotted, but the WalEntry::Checkpoint marker is present).
    engine.checkpoint().expect("checkpoint");

    // Phase B — updates on a subset of bundles. Tests intra-bundle
    // WAL-order preservation: update must apply after the original
    // insert.
    for i in 0..50i64 {
        for (b_idx, name) in bundles.iter().take(4).enumerate() {
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(i + (b_idx as i64) * 10_000));
            let mut patches = Record::new();
            patches.insert(
                "value".into(),
                Value::Float((i as f64) * 100.0),
            );
            engine.update(name, &key, &patches).expect("update");
        }
    }

    // Phase C — deletes on a different subset. Stresses that delete
    // applied after insert produces the right final count.
    for i in 100..120i64 {
        for (b_idx, name) in bundles.iter().skip(4).enumerate() {
            let real_idx = b_idx + 4;
            let mut key = Record::new();
            key.insert(
                "id".into(),
                Value::Integer(i + (real_idx as i64) * 10_000),
            );
            engine.delete(name, &key).expect("delete");
        }
    }

    // Phase D — drop one bundle entirely. The replay must observe the
    // DropBundle entry and NOT include the dropped bundle in the
    // post-replay state.
    engine.drop_bundle("theta").expect("drop_bundle theta");

    // Drop the engine to flush the WAL writer.
    drop(engine);
}

/// Render a single `Record` (= `HashMap<String, Value>`) into a
/// canonical string. `Record` is a `HashMap` so its `Debug`
/// representation is iteration-order-dependent and therefore unsafe
/// to feed into a hash directly — two byte-identical records can
/// print with their fields in different orders depending on
/// insertion history. We sort field names lexicographically and
/// emit `key=value` pairs joined by `|`. `Value`'s `Debug` is
/// stable.
fn canonical_record(rec: &Record) -> String {
    let mut pairs: Vec<(&str, &Value)> = rec.iter().map(|(k, v)| (k.as_str(), v)).collect();
    pairs.sort_by_key(|(k, _)| *k);
    let mut out = String::new();
    for (k, v) in pairs {
        out.push_str(k);
        out.push('=');
        out.push_str(&format!("{:?}", v));
        out.push('|');
    }
    out
}

/// Deterministic state hash. Sorted by bundle name; for each bundle
/// emits (name bytes, LE u64 count, per-record canonical-content
/// SHA-256). Records inside a bundle are sorted by their canonical
/// string so HashMap iteration order on the records iterator does
/// not perturb the output. This is strictly stronger than the
/// benchmark hash (which only counts records per bundle) and will
/// catch any parallel scheme that drops, duplicates, or
/// content-perturbs records inside a bundle.
fn state_hash(engine: &Engine) -> String {
    let mut names: Vec<&str> = engine.bundle_names();
    names.sort();

    let mut hasher = Sha256::new();
    for name in &names {
        hasher.update(name.as_bytes());
        let bundle = engine.bundle(name).expect("bundle present");
        let len = bundle.len() as u64;
        hasher.update(len.to_le_bytes());

        let mut record_strings: Vec<String> = bundle
            .records()
            .map(|rec| canonical_record(&rec))
            .collect();
        record_strings.sort();
        for s in &record_strings {
            hasher.update(s.as_bytes());
            hasher.update(b"\n");
        }
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest.iter() {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Parallel WAL replay must produce post-replay engine state that is
/// byte-identical to the sequential replay on the same WAL. This is
/// the non-negotiable bit-identity gate for the parallel-replay
/// change. Failure means revert.
#[test]
fn tdd_parallel_replay_byte_identical_to_sequential() {
    let dir = tempfile::tempdir().expect("tempdir");
    build_fixture(dir.path());

    // Sequential replay (reference path — must remain exactly the
    // pre-change behaviour).
    let seq_engine = Engine::open_with_replay_mode(dir.path(), ReplayMode::Sequential)
        .expect("engine open sequential");
    let seq_hash = state_hash(&seq_engine);
    let seq_names = {
        let mut n = seq_engine.bundle_names();
        n.sort();
        n.into_iter().map(|s| s.to_string()).collect::<Vec<_>>()
    };
    let seq_total = seq_engine.total_records();
    drop(seq_engine);

    // Parallel replay (the new fast path).
    let par_engine = Engine::open_with_replay_mode(dir.path(), ReplayMode::Parallel)
        .expect("engine open parallel");
    let par_hash = state_hash(&par_engine);
    let par_names = {
        let mut n = par_engine.bundle_names();
        n.sort();
        n.into_iter().map(|s| s.to_string()).collect::<Vec<_>>()
    };
    let par_total = par_engine.total_records();
    drop(par_engine);

    assert_eq!(
        seq_names, par_names,
        "bundle name set diverged across replay modes:\n  sequential: {:?}\n  parallel:   {:?}",
        seq_names, par_names
    );
    assert_eq!(
        seq_total, par_total,
        "total record count diverged across replay modes: sequential={}, parallel={}",
        seq_total, par_total
    );
    assert_eq!(
        seq_hash, par_hash,
        "state hash diverged across replay modes:\n  sequential: {}\n  parallel:   {}",
        seq_hash, par_hash
    );
}

/// Parallel replay must be deterministic across runs on the same
/// WAL. This guards against the failure mode where parallel
/// scheduling itself perturbs final state (e.g. a HashMap iteration
/// order leaking into a registry insert order that affects
/// observable state). The state hash must match across two
/// independent parallel replays of the same WAL.
#[test]
fn tdd_parallel_replay_deterministic_across_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    build_fixture(dir.path());

    let engine_a = Engine::open_with_replay_mode(dir.path(), ReplayMode::Parallel)
        .expect("engine open parallel A");
    let hash_a = state_hash(&engine_a);
    drop(engine_a);

    let engine_b = Engine::open_with_replay_mode(dir.path(), ReplayMode::Parallel)
        .expect("engine open parallel B");
    let hash_b = state_hash(&engine_b);
    drop(engine_b);

    assert_eq!(
        hash_a, hash_b,
        "parallel replay non-deterministic: run A = {}, run B = {}",
        hash_a, hash_b
    );
}
