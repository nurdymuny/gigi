//! TDD-IDX W-IDX-3 / F-6 — "undefined" must not travel as a measurement.
//!
//! `spectral_gap` returned `0.0` in three situations that are not the same
//! thing, and nothing in the return type distinguished them:
//!
//!   1. the graph has no edges (no indexed fields)          — undefined
//!   2. the graph is disconnected                           — undefined
//!   3. the bundle is mmap-resident, so `as_heap()` is None  — not measured at all
//!
//! and one where it is a real answer:
//!
//!   4. a connected graph whose smallest non-zero eigenvalue happens to be small
//!
//! `DEPTH` classifies `lambda1 < lambda1_topological` as level IV — "topological
//! encoding, infinite erasure energy, the manifold topology has changed" — with
//! full confidence. So all three non-measurements produce the single most
//! alarming answer the verb can give, and a caller doing everything right cannot
//! tell them from case 4.
//!
//! Written before the fix. Observed red.

use gigi::engine::Engine;
use gigi::spectral::{self, SpectralGap};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("gigi_ref_{tag}"))
}

/// `n` records, one categorical field, all sharing ONE value. Indexing that
/// field makes the graph `K_n` — connected, with a well-defined λ₁ of n/(n−1).
fn single_value_bundle(n: i64) -> BundleSchema {
    BundleSchema::new("b")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("cohort"))
}

fn rec(i: i64, cohort: &str) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(i));
    r.insert("cohort".into(), Value::Text(cohort.into()));
    r
}

// ───────────────────────────────────────────────────────── T-IDX-11

/// No indexed fields: the graph has no edges, so every record is its own
/// component and there is no non-zero eigenvalue to be the smallest. The
/// result must say so, carrying the count that makes it diagnosable.
#[test]
fn no_indexed_fields_is_undefined_not_zero() {
    let d = dir("t11_noedges");
    let _ = fs::remove_dir_all(&d);
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(single_value_bundle(5)).unwrap();
    for i in 0..5 {
        e.insert("b", &rec(i, "only")).unwrap();
    }

    let store = e.bundle("b").unwrap();
    let gap = spectral::spectral_gap(store.as_heap().unwrap());

    match gap {
        SpectralGap::Undefined { components, records } => {
            assert_eq!(components, 5, "one component per isolated record");
            assert_eq!(records, 5);
        }
        SpectralGap::Measured(v) => panic!(
            "a graph with no edges reported a MEASURED gap of {v} — this is the \
             sentinel travelling as a measurement (TDD-IDX §2.6)"
        ),
    }
    let _ = fs::remove_dir_all(&d);
}

// ───────────────────────────────────────────────────────── T-IDX-12

/// The matched pair for T-IDX-11, and the one that catches the plausible-but-
/// wrong fix. ONE indexed field whose values are all identical gives `K_n`,
/// which is connected and has λ₁ = n/(n−1) exactly. A gate implemented as
/// "refuse unless at least two fields are indexed" — the E17 audit's operational
/// recipe, compiled into a precondition — would wrongly refuse this.
#[test]
fn one_indexed_field_with_one_value_is_measured() {
    let d = dir("t12_kn");
    let _ = fs::remove_dir_all(&d);
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(single_value_bundle(5)).unwrap();
    for i in 0..5 {
        e.insert("b", &rec(i, "only")).unwrap();
    }
    e.add_index("b", "cohort").unwrap();

    let store = e.bundle("b").unwrap();
    let gap = spectral::spectral_gap(store.as_heap().unwrap());

    match gap {
        SpectralGap::Measured(v) => {
            let expected = 5.0_f64 / 4.0;
            assert!(
                (v - expected).abs() < 1e-9,
                "K_5 has λ₁ = 5/4 = {expected}; got {v}"
            );
        }
        SpectralGap::Undefined { components, .. } => panic!(
            "K_5 is connected and its gap is well-defined, but the result was \
             Undefined with {components} components — a field-count gate would \
             produce exactly this (TDD-IDX §2.3)"
        ),
    }
    let _ = fs::remove_dir_all(&d);
}

// ───────────────────────────────────────────────────── T-IDX-11c

/// Disconnected: one indexed field with several distinct values gives a
/// disjoint union of cliques. Undefined, with the component count equal to the
/// number of distinct values.
#[test]
fn disjoint_cliques_are_undefined_with_the_right_component_count() {
    let d = dir("t11c_cliques");
    let _ = fs::remove_dir_all(&d);
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(single_value_bundle(6)).unwrap();
    for i in 0..6 {
        e.insert("b", &rec(i, if i < 3 { "x" } else { "y" })).unwrap();
    }
    e.add_index("b", "cohort").unwrap();

    let store = e.bundle("b").unwrap();
    match spectral::spectral_gap(store.as_heap().unwrap()) {
        SpectralGap::Undefined { components, records } => {
            assert_eq!(components, 2, "two distinct values -> two cliques");
            assert_eq!(records, 6);
        }
        SpectralGap::Measured(v) => panic!(
            "two disjoint K_3 cliques are disconnected; reported Measured({v})"
        ),
    }
    let _ = fs::remove_dir_all(&d);
}

// ───────────────────────────────────────── the escape hatch is explicit

/// `or_zero()` still exists, because 24 call sites wanted the old `f64` and
/// deciding what each verb should do on a degenerate graph is a per-verb
/// product question this spec cannot answer. The point of the type is not that
/// nobody may collapse it — it is that collapsing it is now a visible, greppable
/// act rather than the default.
#[test]
fn or_zero_preserves_the_old_behaviour_explicitly() {
    let d = dir("t_orzero");
    let _ = fs::remove_dir_all(&d);
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(single_value_bundle(4)).unwrap();
    for i in 0..4 {
        e.insert("b", &rec(i, "only")).unwrap();
    }
    let store = e.bundle("b").unwrap();
    assert_eq!(
        spectral::spectral_gap(store.as_heap().unwrap()).or_zero(),
        0.0,
        "or_zero must reproduce exactly what the old f64 return gave"
    );
    let _ = fs::remove_dir_all(&d);
}
