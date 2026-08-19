//! TDD-IDX W-IDX-1 — schema durability.
//!
//! Spec: `theory/gigi/TDD-IDX_index_set_durability.md`, INV-I and INV-S.
//!
//! The index set is state. So is every other mutable part of `BundleSchema`.
//! These tests pin that a schema mutation is journalled (F-2), reaches
//! `Engine::schemas` (F-3), is applied on replay by a delta that ranges over
//! the same fields the payload carries (F-2b), and is ordered log-before-apply
//! (F-0).
//!
//! Written before the fix. Every one was observed red.

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("gigi_idx_{tag}"))
}

fn cleanup(d: &Path) {
    let _ = fs::remove_dir_all(d);
}

fn schema(name: &str) -> BundleSchema {
    BundleSchema::new(name)
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("tag"))
        .fiber(FieldDef::categorical("topic"))
}

fn rec(i: i64, tag: &str, topic: &str) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(i));
    r.insert("tag".into(), Value::Text(tag.into()));
    r.insert("topic".into(), Value::Text(topic.into()));
    r
}

fn seed(e: &mut Engine, name: &str) {
    e.create_bundle(schema(name)).unwrap();
    for i in 0..4i64 {
        let t = if i % 2 == 0 { "a" } else { "b" };
        e.insert(name, &rec(i, t, "math")).unwrap();
    }
}

fn indexed(e: &Engine, name: &str) -> Vec<String> {
    e.bundle(name).unwrap().schema().indexed_fields.clone()
}

fn fiber_names(fields: &[FieldDef]) -> Vec<String> {
    fields.iter().map(|f| f.name.clone()).collect()
}

// ─────────────────────────────────────────────── T-IDX-4 + T-IDX-5

/// An index declared through the engine must survive a restart, and the
/// engine's own schema map must agree with the store's copy while it is live.
///
/// D-1 + D-2: `add_index` journalled nothing, and `BundleStore` holds a *clone*
/// of the schema (`engine.rs` `BundleStore::new(schema.clone())`) while
/// `Engine::schemas` keeps the original — so the engine map never learned about
/// the index and `compact_wal_to_schemas` re-emitted the stale one.
#[test]
fn index_survives_restart_and_the_two_schema_copies_agree() {
    let d = dir("t4_persist");
    cleanup(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        seed(&mut e, "b");
        e.add_index("b", "tag").unwrap();

        assert!(indexed(&e, "b").contains(&"tag".to_string()));
        // T-IDX-5: the two copies must not diverge.
        assert_eq!(
            e.bundle_schema("b").unwrap().indexed_fields,
            indexed(&e, "b"),
            "Engine::schemas and the store's schema clone disagree"
        );
        e.snapshot().unwrap();
    }

    {
        let e = Engine::open(&d).unwrap();
        assert!(
            indexed(&e, "b").contains(&"tag".to_string()),
            "INDEX LOST ON RESTART — see TDD-IDX D-1/D-2"
        );
        assert_eq!(e.bundle_schema("b").unwrap().indexed_fields, indexed(&e, "b"));
    }

    cleanup(&d);
}

// ─────────────────────────────────────────────────────── T-IDX-6

/// A compaction re-emits `CreateBundle` from `Engine::schemas`. If the index
/// never reached that map, compaction writes the stale schema back and buries
/// the index permanently — worse than merely failing to replay it.
#[test]
fn index_survives_compaction_not_merely_restart() {
    let d = dir("t6_compact");
    cleanup(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        seed(&mut e, "b");
        e.add_index("b", "tag").unwrap();
        e.compact().unwrap();
    }

    {
        let e = Engine::open(&d).unwrap();
        assert!(
            indexed(&e, "b").contains(&"tag".to_string()),
            "compaction re-emitted a schema without the index"
        );
    }

    cleanup(&d);
}

// ────────────────────────────────────────────── T-IDX-15 + T-IDX-19

/// The mirror case, and the one v2 got wrong: a *removal* must also survive.
///
/// `drop_field` removes the field and cascades to `indexed_fields`. Under
/// add-only replay the removal is journalled and never applied, so the field
/// and its index both come back. T-IDX-19 additionally pins that the two schema
/// copies agree as an ORDERED sequence — fiber layout is positional, so set
/// equality would pass while every read after the removed slot returned the
/// wrong column.
#[test]
fn dropped_field_and_its_index_stay_dropped_across_restart() {
    let d = dir("t15_drop");
    cleanup(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        seed(&mut e, "b");
        e.add_index("b", "tag").unwrap();
        assert!(e.drop_field("b", "tag").unwrap(), "drop_field should report true");

        assert!(!indexed(&e, "b").contains(&"tag".to_string()));
        assert!(!fiber_names(&e.bundle("b").unwrap().schema().fiber_fields)
            .contains(&"tag".to_string()));
        // NO snapshot. A compaction collapses the WAL to a single final
        // CreateBundle, so replay takes the Vacant branch and constructs the
        // store from the finished schema — the delta never runs. Snapshotting
        // here would make this test green with the delta removed entirely,
        // which the mechanism-removal pass demonstrated.

    }

    {
        let e = Engine::open(&d).unwrap();
        let store_fibers = fiber_names(&e.bundle("b").unwrap().schema().fiber_fields);
        let engine_fibers = fiber_names(&e.bundle_schema("b").unwrap().fiber_fields);

        assert!(
            !store_fibers.contains(&"tag".to_string()),
            "dropped field returned after restart: {store_fibers:?}"
        );
        assert!(
            !indexed(&e, "b").contains(&"tag".to_string()),
            "dropped field's index returned after restart"
        );
        // T-IDX-19: ordered sequence, not set.
        assert_eq!(
            store_fibers, engine_fibers,
            "Engine::schemas and the store disagree on fiber field ORDER; \
             fiber access is positional, so this is a silent misread"
        );
    }

    cleanup(&d);
}

// ────────────────────────────────────────────────────── T-IDX-18

/// A field added through the engine must survive a restart exactly ONCE.
///
/// `add_field` pushes unconditionally (`bundle.rs`) with no idempotence guard,
/// so a replay that applies the journalled payload rather than a delta against
/// the store's current schema duplicates the field and shifts every position
/// after it. The record values are asserted too: a shifted layout shows up
/// there, not in the field list.
#[test]
fn added_field_survives_restart_exactly_once() {
    let d = dir("t18_addfield");
    cleanup(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        seed(&mut e, "b");
        e.add_field("b", FieldDef::categorical("region")).unwrap();
        // NO snapshot — see the note in the drop_field test above.
    }

    {
        let e = Engine::open(&d).unwrap();
        let names = fiber_names(&e.bundle("b").unwrap().schema().fiber_fields);
        assert_eq!(
            names.iter().filter(|n| *n == "region").count(),
            1,
            "field duplicated across replay: {names:?}"
        );

        // Positional integrity: the original values must still read back.
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(2));
        let got = e.point_query("b", &key).unwrap().expect("record 2");
        assert_eq!(got.get("tag"), Some(&Value::Text("a".into())), "fiber layout shifted");
        assert_eq!(got.get("topic"), Some(&Value::Text("math".into())), "fiber layout shifted");
    }

    cleanup(&d);
}

// ────────────────────────────────────────────────────── T-IDX-16

/// A no-op `add_index` must append nothing to the WAL. Unconditional logging
/// grows the log without changing state, which is the cost side of F-2's
/// "re-emit CreateBundle" design and the reason the early-return matters.
#[test]
fn repeated_add_index_appends_no_wal_entry() {
    let d = dir("t16_noop");
    cleanup(&d);

    let wal = d.join("gigi.wal");
    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        seed(&mut e, "b");
        e.add_index("b", "tag").unwrap();

        let before = fs::metadata(&wal).unwrap().len();
        for _ in 0..5 {
            e.add_index("b", "tag").unwrap();
        }
        let after = fs::metadata(&wal).unwrap().len();

        assert_eq!(
            before, after,
            "five no-op add_index calls grew the WAL by {} bytes",
            after - before
        );
    }

    cleanup(&d);
}

// ────────────────────────────────────────────────────── T-IDX-8

/// A re-emitted `CreateBundle` for a populated bundle must not drop its
/// records. `or_insert_with` is what makes the re-emit safe; `insert` would
/// replace the store and silently empty it.
#[test]
fn reemitted_create_bundle_does_not_drop_records() {
    let d = dir("t8_records");
    cleanup(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        seed(&mut e, "b");
        e.add_index("b", "tag").unwrap();
        e.add_index("b", "topic").unwrap();
    }

    {
        let e = Engine::open(&d).unwrap();
        assert_eq!(e.total_records(), 4, "re-emitted CreateBundle dropped records");
    }

    cleanup(&d);
}

// ────────────────────────────────────────────────────── T-IDX-17

/// INV-S — every field of `BundleSchema` must carry a written disposition:
/// the replay delta applies it, no mutator exists for it, or it is out of
/// scope with its consequence named.
///
/// This destructure is exhaustive **on purpose**: no `..` pattern. Adding a
/// ninth field to `BundleSchema` breaks this test's compilation, which is the
/// point — a (b) disposition ("no mutator exists") is a claim about today and
/// expires silently without a compile-time check.
#[test]
fn every_bundle_schema_field_has_an_inv_s_disposition() {
    let s = schema("disposition");
    let BundleSchema {
        // (b) — no post-construction mutator
        name: _,
        base_fields: _,
        adjacencies: _,
        h1_threshold: _,
        invariants: _,
        // (a) — the replay delta applies these
        fiber_fields: _,
        indexed_fields: _,
        // (c) — out of scope, consequence named in TDD-IDX §8
        gauge_key: _,
        // (c) — feature-gated NINTH field, found by this very test on its
        // first run under `--features kahler`. `with_kahler` is a consuming
        // builder, so there is no post-construction mutator and (b) would
        // apply — except that `kahler` does not appear in the WAL schema
        // payload at all (`grep kahler src/wal.rs` → nothing), so it is not
        // journalled and does not survive a restart. That is a durability gap
        // in the Kähler feature, not an indexing one; out of scope here, and
        // flagged in TDD-IDX §8.
        #[cfg(feature = "kahler")]
        kahler: _,
    } = s;
}

// ─────────────────────────────────────────────── T-IDX-4b (F-2, isolated)

/// The same as T-IDX-4 but with **no snapshot and no compaction** between the
/// mutation and the restart.
///
/// Added after the mechanism-removal pass, which found that deleting the WAL
/// append from `journal_schema` left every other test in this file green. The
/// reason is that they all snapshot before dropping the engine, and
/// `compact_wal_to_schemas` re-emits `CreateBundle` from `Engine::schemas` —
/// which the F-3 write had already updated. So a compaction was silently
/// standing in for the journalling, and the tests could not tell the two apart.
///
/// This is the only case where the WAL append is the sole durable record of the
/// change, which makes it the only test that gates F-2 on its own. It is also
/// the realistic one: a crash does not run a compaction first.
#[test]
fn index_survives_restart_with_no_compaction_in_between() {
    let d = dir("t4b_nocompact");
    cleanup(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        seed(&mut e, "b");
        e.add_index("b", "tag").unwrap();
        // deliberately NO snapshot() and NO compact() — drop straight out
    }

    {
        let e = Engine::open(&d).unwrap();
        assert!(
            indexed(&e, "b").contains(&"tag".to_string()),
            "the index was never journalled — a compaction was covering for it \
             in the other tests. See TDD-IDX F-2."
        );
    }

    cleanup(&d);
}
