//! TDD-IDX W-IDX-4 / F-5 — `bundle_version = H(records, index_set)`.
//!
//! F-1 through F-3 make a stale geometry answer *not happen*. F-5 makes one
//! *detectable*, which is a different job and the one a diagnosis product needs:
//! a λ quoted in a report last week is only trustworthy if you can ask whether
//! the bundle it was computed on is still the same bundle.
//!
//! The version must therefore be derived from content, not from a counter.
//! `mutation_counter` restarts at zero, so a version built on it would report
//! "changed" after every reboot and "unchanged" for two engines holding
//! different data.
//!
//! Written before the fix. Observed red.

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("gigi_ver_{tag}"))
}

fn schema() -> BundleSchema {
    BundleSchema::new("b")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("tag"))
}

fn rec(i: i64, tag: &str) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(i));
    r.insert("tag".into(), Value::Text(tag.into()));
    r
}

fn seed(e: &mut Engine) {
    e.create_bundle(schema()).unwrap();
    for i in 0..4 {
        e.insert("b", &rec(i, if i % 2 == 0 { "a" } else { "b" })).unwrap();
    }
}

// ───────────────────────────────────────────────────────── T-IDX-10

/// The headline: declaring an index changes the version even though not one
/// record moved. This is the whole point — `index_set` decides what the
/// λ-verbs measure (§2.2), so a λ computed before it is not comparable to one
/// computed after, and a version keyed only on records would call them equal.
#[test]
fn version_changes_when_only_the_index_set_changes() {
    let d = dir("t10_index");
    let _ = fs::remove_dir_all(&d);
    let mut e = Engine::open(&d).unwrap();
    seed(&mut e);

    let before = e.bundle_version("b").unwrap();
    e.add_index("b", "tag").unwrap();
    let after = e.bundle_version("b").unwrap();

    assert_ne!(
        before, after,
        "index_set changed and records did not, but the version did not move — \
         a version keyed on H(records) alone cannot detect this (TDD-IDX §2.5)"
    );
    let _ = fs::remove_dir_all(&d);
}

/// And the converse, so the test above cannot pass by the version simply
/// changing on every call.
#[test]
fn version_is_stable_when_nothing_changes() {
    let d = dir("t10_stable");
    let _ = fs::remove_dir_all(&d);
    let mut e = Engine::open(&d).unwrap();
    seed(&mut e);

    let a = e.bundle_version("b").unwrap();
    let b = e.bundle_version("b").unwrap();
    assert_eq!(a, b, "two reads with no mutation between them must agree");
    let _ = fs::remove_dir_all(&d);
}

/// Records changing must move it too — otherwise it is a schema version
/// wearing a bundle version's name.
#[test]
fn version_changes_when_records_change() {
    let d = dir("t10_records");
    let _ = fs::remove_dir_all(&d);
    let mut e = Engine::open(&d).unwrap();
    seed(&mut e);

    let before = e.bundle_version("b").unwrap();
    e.insert("b", &rec(99, "c")).unwrap();
    let after = e.bundle_version("b").unwrap();

    assert_ne!(before, after, "a new record must move the version");
    let _ = fs::remove_dir_all(&d);
}

// ───────────────────────────────── the property a counter cannot give

/// Restart-stability. This is why the version is a content hash and not
/// `mutation_counter`: the counter restarts at zero, so a counter-derived
/// version would report "changed" after every reboot — making it useless for
/// the one question it exists to answer, which spans restarts by construction.
#[test]
fn version_survives_a_restart_unchanged() {
    let d = dir("t10_restart");
    let _ = fs::remove_dir_all(&d);

    let before = {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        seed(&mut e);
        e.add_index("b", "tag").unwrap();
        e.bundle_version("b").unwrap()
    };

    let after = {
        let e = Engine::open(&d).unwrap();
        e.bundle_version("b").unwrap()
    };

    assert_eq!(
        before, after,
        "the same records and the same index set produced a different version \
         across a restart — the version is derived from session state, not content"
    );
    let _ = fs::remove_dir_all(&d);
}

/// Two bundles with identical content and identical index sets must version
/// identically, even in different engines. Without this the value cannot be
/// compared between a report and a live system, which is its only use.
#[test]
fn identical_content_versions_identically_across_engines() {
    let d1 = dir("t10_eq1");
    let d2 = dir("t10_eq2");
    let _ = fs::remove_dir_all(&d1);
    let _ = fs::remove_dir_all(&d2);

    let mut e1 = Engine::open(&d1).unwrap();
    seed(&mut e1);
    e1.add_index("b", "tag").unwrap();

    // e2 reaches the SAME content by a DIFFERENT path: an extra record, then
    // its removal. This is the discriminating case, and the plain
    // same-sequence version of this test could not see it — replay applies the
    // same mutations in the same order, so a version keyed on `mutation_counter`
    // lands on the same number and looks content-derived when it is not.
    // Found by the mechanism-removal pass: mixing the counter into the hash
    // left every other test in this file green.
    let mut e2 = Engine::open(&d2).unwrap();
    seed(&mut e2);
    e2.insert("b", &rec(1234, "scratch")).unwrap();
    let mut scratch_key = Record::new();
    scratch_key.insert("id".into(), Value::Integer(1234));
    assert!(e2.delete("b", &scratch_key).unwrap(), "scratch record removed");
    e2.add_index("b", "tag").unwrap();

    assert_eq!(
        e1.bundle_version("b").unwrap(),
        e2.bundle_version("b").unwrap(),
        "same records and same index set, reached by different mutation paths, \
         produced different versions — the hash is keyed on session state, not content"
    );

    // and a difference in ONE record must separate them
    e2.insert("b", &rec(7, "z")).unwrap();
    assert_ne!(
        e1.bundle_version("b").unwrap(),
        e2.bundle_version("b").unwrap()
    );

    let _ = fs::remove_dir_all(&d1);
    let _ = fs::remove_dir_all(&d2);
}
