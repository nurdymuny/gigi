//! Induced schema mutations must be detected, not persisted.
//!
//! TDD-IDX INV-S, third option. `update_versioned` turned out to call
//! `add_field("_version")` one level down — a schema mutation induced by a
//! record update, invisible to the grep INV-S's dispositions rest on. Handling
//! it by pre-declaring the field works for that one instance and generalises to
//! nothing: any store method may induce a schema change.
//!
//! The two obvious answers both cost something. Enumerating by grep keeps
//! missing indirect callers. Reconciling after the fact and journalling the
//! drift generalises, but inverts F-0's log-before-apply for the induced part.
//!
//! Hallie's third: **reconcile and assert, not reconcile and journal.** Drift
//! between the store's schema and `Engine::schemas` is a bug, not a state worth
//! persisting. Detecting it at the call site costs nothing, leaves F-0 intact,
//! and turns each future induced mutator into something found immediately and
//! then given the `_version` treatment individually.
//!
//! Same move the spec already makes twice: F-6 turning a sentinel into an
//! unrepresentable state, and `FieldDef: PartialEq` turning a future silent
//! divergence into a present loud one.

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;

fn setup(tag: &str) -> (std::path::PathBuf, Engine) {
    let d = std::env::temp_dir().join(format!("gigi_drift_{tag}"));
    let _ = fs::remove_dir_all(&d);
    let mut e = Engine::open(&d).unwrap();
    e.compaction_policy_mut().disabled = true;
    e.create_bundle(
        BundleSchema::new("b")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("tag")),
    )
    .unwrap();
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(1));
    r.insert("tag".into(), Value::Text("x".into()));
    e.insert("b", &r).unwrap();
    (d, e)
}

/// A coherent engine reports no drift. Without this the detector could pass by
/// always reporting drift, which is the failure this suite's own discipline
/// exists to catch.
#[test]
fn a_coherent_engine_reports_no_drift() {
    let (d, e) = setup("clean");
    assert_eq!(e.schema_drift("b"), None, "nothing has diverged");
    let _ = fs::remove_dir_all(&d);
}

/// Mutating the store's schema behind the engine's back is detected, and the
/// report names the field so the induced mutator can be found.
#[test]
fn a_schema_mutation_that_bypasses_the_engine_is_detected() {
    let (d, mut e) = setup("induced");

    // Exactly what an induced mutator does: reach the store directly and change
    // its schema. `update_versioned` did this with `_version`.
    e.bundle_mut("b")
        .unwrap()
        .add_field(FieldDef::numeric("_shadow"));

    let drift = e.schema_drift("b").expect("drift must be detected");
    assert!(
        drift.contains("_shadow"),
        "the report must name the diverging field so the inducing call can be \
         found; got: {drift}"
    );
    let _ = fs::remove_dir_all(&d);
}

/// The path that induced the original finding stays coherent, because
/// `Engine::update_versioned` pre-declares `_version` through the journalling
/// path rather than letting the store add it.
#[test]
fn the_versioned_update_path_stays_coherent() {
    let (d, mut e) = setup("versioned");

    let mut key = Record::new();
    key.insert("id".into(), Value::Integer(1));
    let mut patch = Record::new();
    patch.insert("tag".into(), Value::Text("y".into()));
    e.update_versioned("b", &key, &patch, 0).unwrap();

    assert_eq!(
        e.schema_drift("b"),
        None,
        "update_versioned induces a _version field; pre-declaring it through \
         the journalling path is what keeps the two schema copies in step"
    );
    let _ = fs::remove_dir_all(&d);
}
