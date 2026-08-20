//! The 2026-08-20 pre-deploy snapshot incident, reduced to two failing tests.
//!
//! `POST /v1/admin/snapshot` on production failed after 198s with
//!
//!   Snapshot failed: Invalid arithmetic pattern:
//!     Invalid step in 'binary_version@v6.7.0+gemini-drift-v08+0'
//!
//! and left five marcella bundles physically duplicated on disk
//! (`marcella_source_sections.dhoom`: 155,610 -> 325,227 records) while
//! twenty-three others dropped to half their reported count — which turned
//! out to be the *correct* count; the halves were a pre-existing live
//! double-count of the same origin.
//!
//! Two distinct bugs:
//!
//! **1. Header delimiter injection (encoder).** The DHOOM header grammar uses
//! `@start+step` for arithmetic columns, and the decoder splits on the first
//! `+` (`dhoom.rs:456`). The encoder folded a text column whose values contain
//! `+` — `v6.7.0+gemini-drift-v08` — producing a header it cannot itself
//! re-read. The snapshot aborted when reopening its own output.
//!
//! **2. Key-encoding asymmetry (rebase merge).** The rebase dedups base
//! records against the overlay by `pk_string`. Overlay records carry native
//! `Value`s; base records have been through JSON, and `serde_json_to_value`
//! (`engine.rs:3869`) cannot produce `Timestamp` (comes back `Integer`) or
//! `Binary`-vs-`Text` faithfully in every case. A bundle whose PK includes a
//! timestamp — `marcella_genealogy_records.ingested_at` — never matches, so
//! the merge writes base AND overlay: physical duplication, snapshot after
//! snapshot. The same family as the W7 tombstone-key bug, one layer deeper:
//! more than one encoding for the same key.
//!
//! Written before the fixes. Both observed red.

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("gigi_inc0820_{tag}"))
}

// ───────────────────────────── Bug 1: header delimiter injection

/// A text field whose values contain `+` must survive snapshot + reload.
/// This is the production failure in miniature: same value, same shape.
#[test]
fn plus_in_text_value_survives_snapshot_roundtrip() {
    let d = dir("plus");
    let _ = fs::remove_dir_all(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(
            BundleSchema::new("cfs")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("binary_version")),
        )
        .unwrap();
        for i in 0..40i64 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            // 40 identical values: text-arithmetic step-0 folding fires at this size
            // (modal wins at n<10, which is why a small fixture stays green)
            r.insert(
                "binary_version".into(),
                Value::Text("v6.7.0+gemini-drift-v08".into()),
            );
            e.insert("cfs", &r).unwrap();
        }
        e.snapshot().expect(
            "snapshot must not write a header its own decoder rejects \
             (production: 'Invalid step in binary_version@v6.7.0+gemini-drift-v08+0')",
        );
    }

    {
        let e = Engine::open_mmap(&d).expect("the written .dhoom must reopen");
        assert_eq!(e.total_records(), 40, "all records must reload");
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(2));
        let got = e.point_query("cfs", &key).unwrap().expect("record 2");
        assert_eq!(
            got.get("binary_version"),
            Some(&Value::Text("v6.7.0+gemini-drift-v08".into())),
            "the value must round-trip byte-identically"
        );
    }

    let _ = fs::remove_dir_all(&d);
}

/// Every header-structural character, not only `+`. The header grammar uses
/// `{ } , @ | > ^ & #` and `:` — a folded value containing any of them either
/// breaks parsing or silently changes meaning. `,` is the sharpest: it splits
/// field declarations, so a modal default of `"a, b"` truncates the header.
#[test]
fn every_header_delimiter_survives_snapshot_roundtrip() {
    for (i, bad) in [
        "a, b",     // field separator inside a modal default
        "x}y",      // header terminator
        "p|q",      // default marker
        "m@n",      // arithmetic marker
        "u^w",      // delta marker
        "s>t",      // nested marker
        "e&f",      // intern marker
        "g#h",      // computed marker
    ]
    .iter()
    .enumerate()
    {
        let d = dir(&format!("delim{i}"));
        let _ = fs::remove_dir_all(&d);

        {
            let mut e = Engine::open(&d).unwrap();
            e.compaction_policy_mut().disabled = true;
            e.create_bundle(
                BundleSchema::new("b")
                    .base(FieldDef::numeric("id"))
                    .fiber(FieldDef::categorical("v")),
            )
            .unwrap();
            for j in 0..12i64 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(j));
                r.insert("v".into(), Value::Text((*bad).into()));
                e.insert("b", &r).unwrap();
            }
            e.snapshot()
                .unwrap_or_else(|e| panic!("snapshot failed on value {bad:?}: {e}"));
        }

        {
            let e = Engine::open_mmap(&d)
                .unwrap_or_else(|e| panic!("reopen failed on value {bad:?}: {e}"));
            assert_eq!(e.total_records(), 12, "value {bad:?}: records lost");
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(1));
            let got = e.point_query("b", &key).unwrap().expect("record 1");
            assert_eq!(
                got.get("v"),
                Some(&Value::Text((*bad).to_string())),
                "value {bad:?} did not round-trip"
            );
        }

        let _ = fs::remove_dir_all(&d);
    }
}

// ───────────────────────────── Bug 2: key-encoding asymmetry in the merge

/// A bundle whose PK includes a TIMESTAMP must not duplicate across
/// snapshot cycles. This is `marcella_genealogy_records` in miniature:
/// base records live in JSON (Timestamp -> Integer on read-back), overlay
/// records carry native `Timestamp`, and a key comparison that sees two
/// different types writes both.
///
/// The shape: snapshot once (records -> base), write the same records again
/// (upsert semantics -> overlay shadows base), snapshot again (the rebase
/// merge), reload, count.
#[test]
fn timestamp_keyed_bundle_does_not_duplicate_across_snapshots() {
    let d = dir("ts_dup");
    let _ = fs::remove_dir_all(&d);

    let rec = |i: i64| {
        let mut r = Record::new();
        r.insert("ingested_at".into(), Value::Timestamp(1_750_000_000_000 + i));
        r.insert("content".into(), Value::Text(format!("record {i}")));
        r
    };

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(
            BundleSchema::new("g")
                .base(FieldDef::timestamp("ingested_at", 1.0))
                .fiber(FieldDef::categorical("content")),
        )
        .unwrap();
        for i in 0..3 {
            e.insert("g", &rec(i)).unwrap();
        }
        e.snapshot().unwrap(); // records -> .dhoom base
    }

    {
        let mut e = Engine::open_mmap(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        // Same records again: overlay now shadows base, count must stay 3.
        for i in 0..3 {
            e.insert("g", &rec(i)).unwrap();
        }
        assert_eq!(
            e.total_records(),
            3,
            "LIVE double-count: overlay fails to shadow base when the PK is a \
             Timestamp (base side round-tripped to Integer). This is the origin \
             of openssl_crypto_v05 reporting 18,288 over a 9,145-record disk."
        );
        e.snapshot().unwrap(); // the rebase merge
    }

    {
        let e = Engine::open_mmap(&d).unwrap();
        assert_eq!(
            e.total_records(),
            3,
            "DISK duplication: the rebase merge wrote base AND overlay copies \
             of the same records. This is marcella_source_sections going \
             155,610 -> 325,227 on 2026-08-20."
        );
    }

    let _ = fs::remove_dir_all(&d);
}
// ───────────────── the real thing: the actual 400 production records
//
// The synthetic fixtures above did NOT reproduce the fold — the encoder's
// text-arithmetic classification depends on record shape in ways a 2-field
// fixture misses. This fixture IS the production bundle that aborted the
// snapshot (exported 2026-08-20, cfs_psp_v01, 400 records, 95 columns), and
// with the guard removed it fails with the production error byte-for-byte.
use serde_json::Value as JsonValue;

#[test]
fn encode_the_real_cfs_psp_records() {
    // The fixture is REAL production data and is deliberately not committed —
    // this repo is public, and it went out once by mistake (f597329; removed
    // from the tip, history left per standing decision). The synthetic tests
    // above carry the same mechanisms in CI; this one runs wherever the
    // fixture exists locally.
    let Ok(raw) = std::fs::read_to_string("tests/fixtures_cfs_psp_incident.json") else {
        eprintln!("fixture absent (real production data, not committed) - skipping");
        return;
    };
    let doc: JsonValue = serde_json::from_str(&raw).unwrap();
    let recs = doc.get("records").cloned().unwrap_or(doc);
    let mut wrapped = serde_json::Map::new();
    wrapped.insert("cfs_psp_v01".into(), recs);
    match gigi::dhoom::encode(&JsonValue::Object(wrapped)) {
        Ok(s) => {
            let header = s.lines().next().unwrap_or("");
            println!("HEADER: {}", &header[..header.len().min(400)]);
            // and can we re-read it?
            match gigi::dhoom::decode(&s) {
                Ok(_) => println!("decode: OK"),
                Err(e) => panic!("ENCODER WROTE WHAT DECODER REJECTS: {e}"),
            }
        }
        Err(e) => panic!("encode failed outright: {e}"),
    }
}

// ───────────────────────────── Bug 3: body newline shatter

/// A text value containing embedded newlines must survive snapshot + reload
/// as ONE record.
///
/// This is Bee's authored genealogy record in miniature. DHOOM body rows are
/// line-delimited; the writer quoted newline-containing values CSV-style but
/// left the newline LITERAL inside the quotes, and every reader iterates
/// `body.lines()` — so the first time `marcella_genealogy_records` was ever
/// encoded to disk (the 2026-08-20 rebase), the record containing her
/// grandmother's story shattered into a dozen fragment-records: "the cedar
/// chest", "found the name on my grandmother's nursing degree", each line a
/// counterfeit record. The WAL held the intact original, which is what the
/// restore recovered.
#[test]
fn multiline_text_survives_snapshot_as_one_record() {
    let d = dir("newline");
    let _ = fs::remove_dir_all(&d);

    let story = "I was named for my grandmother.\nMy grandmother was a nurse,\n\
                 who kept her middle name hidden her whole life.\nYears later,\n\
                 she gave that runtime the reclaimed name.";

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(
            BundleSchema::new("g")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("content")),
        )
        .unwrap();
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("content".into(), Value::Text(story.into()));
        e.insert("g", &r).unwrap();
        // a second, single-line record so a shatter changes the count
        let mut r2 = Record::new();
        r2.insert("id".into(), Value::Integer(2));
        r2.insert("content".into(), Value::Text("single line".into()));
        e.insert("g", &r2).unwrap();
        e.snapshot().unwrap();
    }

    {
        let e = Engine::open_mmap(&d).expect("reopen");
        assert_eq!(
            e.total_records(),
            2,
            "a newline-containing record shattered into fragment-records"
        );
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let got = e.point_query("g", &key).unwrap().expect("record 1");
        assert_eq!(
            got.get("content"),
            Some(&Value::Text(story.into())),
            "the multi-line content must round-trip byte-identically"
        );
    }

    let _ = fs::remove_dir_all(&d);
}
