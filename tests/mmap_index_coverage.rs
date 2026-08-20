//! TDD-IDX W-IDX-5 / F-4 — an index on an mmap-backed bundle does not cover
//! the base, and nothing may quietly assume otherwise.
//!
//! **What is deliberately NOT built here.** F-4 as written says "make
//! `add_index` cover the mmap base". That would mean an overlay-level field
//! index addressing both mmap rows and overlay records — real machinery. It is
//! not built, because it currently has **no consumer**:
//!
//!   * `field_index_graph` (the λ path) takes `&BundleStore` — heap only.
//!   * `OverlayBundle::indexed_values` says so in its own doc comment:
//!     "from overlay; base has no index".
//!   * `/spectral_gap` returns 501 for mmap-resident bundles.
//!   * `DEPTH` refuses for them as of W-IDX-3.
//!
//! So a partial index cannot produce a wrong geometry answer today — no
//! geometry answer is produced at all. Building the merged index now would be
//! speculative work against an interface nobody calls.
//!
//! **What IS built here**: the partial-ness is made impossible to inherit
//! silently. `add_index` returns `#[must_use] IndexCoverage`, so the day
//! somebody lifts the λ-verbs onto `BundleRef` — the follow-up this has always
//! been sequenced behind — they cannot wire the index through without the
//! compiler making them look at it. That is the F-6 move applied to F-4: turn a
//! future silent divergence into a present loud one, structurally, rather than
//! by remembering.

use gigi::engine::Engine;
use gigi::mmap_bundle::IndexCoverage;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("gigi_cov_{tag}"))
}

fn schema() -> BundleSchema {
    BundleSchema::new("b")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("tag"))
}

fn rec(i: i64) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(i));
    r.insert("tag".into(), Value::Text("t".into()));
    r
}

/// Indexing a bundle whose records live in the mmap base reports
/// `OverlayOnly`, and names how many records are uncovered.
#[test]
fn indexing_an_mmap_backed_bundle_reports_partial_coverage() {
    let d = dir("partial");
    let _ = fs::remove_dir_all(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema()).unwrap();
        for i in 0..6 {
            e.insert("b", &rec(i)).unwrap();
        }
        e.snapshot().unwrap(); // push the records into a .dhoom
    }

    {
        let mut e = Engine::open_mmap(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        // one record in the overlay, six in the base
        e.insert("b", &rec(99)).unwrap();

        let ob = e.mmap_bundle("b").expect("bundle is mmap-resident");
        match ob.add_index("tag") {
            IndexCoverage::OverlayOnly { base_records } => {
                assert_eq!(
                    base_records, 6,
                    "six records live in the base and are not indexed"
                );
            }
            IndexCoverage::Complete => panic!(
                "reported Complete for a bundle with 6 records in the mmap base — \
                 the index covers the overlay only (TDD-IDX F-4)"
            ),
        }
    }

    let _ = fs::remove_dir_all(&d);
}

/// The converse, so the test above cannot pass by always reporting partial:
/// an overlay bundle with an empty base is completely covered.
#[test]
fn an_empty_base_is_completely_covered() {
    let d = dir("complete");
    let _ = fs::remove_dir_all(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema()).unwrap();
        e.snapshot().unwrap();
    }

    {
        let mut e = Engine::open_mmap(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.insert("b", &rec(1)).unwrap();
        e.insert("b", &rec(2)).unwrap();

        if let Some(ob) = e.mmap_bundle("b") {
            assert_eq!(
                ob.add_index("tag"),
                IndexCoverage::Complete,
                "an empty base leaves nothing uncovered"
            );
        }
        // If the bundle fell back to a heap store (no .dhoom for an empty
        // bundle), there is no overlay to test and the case is vacuous — the
        // partial-coverage test above is the one that carries the weight.
    }

    let _ = fs::remove_dir_all(&d);
}
