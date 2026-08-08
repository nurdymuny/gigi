//! The field index must address every record distinctly.
//!
//! Roaring bitmaps are u32-only, so the index refers to records by a 32-bit
//! id. That id used to be `bp as u32` — the low half of the 64-bit base
//! point — which meant two records whose base points agreed in their low 32
//! bits shared one bitmap entry and one `bp_reverse` slot. The loser vanished
//! from every indexed query (`COVER ... ON f=v`, neighborhood, spectral,
//! betti, components) while staying perfectly present in `sections()`, so
//! `len()` looked right and nothing raised a word.
//!
//! Birthday bound on 32 bits: ~4.5% chance of at least one collision at
//! 20k records, 25% at 50k, ~69% at 100k, effectively certain past 500k.
//!
//! These gates search for a genuine colliding pair against the engine's own
//! hash and assert both records stay visible. On the pre-fix code the first
//! one fails; there is no way to pass it by truncating.

use gigi::bundle::BundleStore;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn schema() -> BundleSchema {
    let mut s = BundleSchema::new("collide");
    s.base_fields.push(FieldDef::numeric("id"));
    s.fiber_fields.push(FieldDef::categorical("tag"));
    s.fiber_fields.push(FieldDef::numeric("v"));
    s.indexed_fields.push("tag".into());
    s
}

fn rec(id: i64, tag: &str, v: f64) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(id));
    r.insert("tag".into(), Value::Text(tag.into()));
    r.insert("v".into(), Value::Float(v));
    r
}

/// Find two integer keys whose base points share low-32 bits, using the
/// engine's real hash. Returns `None` if none turns up in the search budget
/// (it always does well inside it — that is the point of the bug).
fn find_collision(store: &BundleStore, budget: i64) -> Option<(i64, i64)> {
    use std::collections::HashMap;
    let mut low: HashMap<u32, i64> = HashMap::new();
    for id in 0..budget {
        let key = rec(id, "x", 0.0);
        let bp = store.base_point(&key);
        if let Some(&prev) = low.get(&((bp & 0xFFFF_FFFF) as u32)) {
            // Guard against a full 64-bit collision, which would be a
            // genuinely different (and much rarer) problem.
            let prev_bp = store.base_point(&rec(prev, "x", 0.0));
            if prev_bp != bp {
                return Some((prev, id));
            }
        }
        low.insert((bp & 0xFFFF_FFFF) as u32, id);
    }
    None
}

#[test]
fn colliding_low_words_stay_separately_visible_in_the_index() {
    let probe = BundleStore::new(schema());
    let (a, b) = find_collision(&probe, 400_000)
        .expect("no low-32 collision found in budget — widen the search");

    let mut store = BundleStore::new(schema());
    store.insert(&rec(a, "shared", 1.0));
    store.insert(&rec(b, "shared", 2.0));

    assert_eq!(store.len(), 2, "both records must be stored");

    // The index must return BOTH. Pre-fix this returns one.
    let hits = store.neighborhood("tag", &Value::Text("shared".into()));
    assert_eq!(
        hits.len(),
        2,
        "field index lost a record to a low-32-bit collision \
         (ids {a} and {b}); indexed queries would silently miss it"
    );

    // And each ordinal must resolve back to a distinct, real base point.
    let bp_a = store.base_point(&rec(a, "shared", 1.0));
    let bp_b = store.base_point(&rec(b, "shared", 2.0));
    assert_ne!(bp_a, bp_b);
    let mut resolved: Vec<u64> = hits.clone();
    resolved.sort_unstable();
    let mut want = vec![bp_a, bp_b];
    want.sort_unstable();
    assert_eq!(resolved, want, "resolved base points must be the real ones");
}

#[test]
fn colliding_records_are_both_reachable_by_graph_verbs() {
    let probe = BundleStore::new(schema());
    let (a, b) = find_collision(&probe, 400_000).expect("collision");

    let mut store = BundleStore::new(schema());
    // Two records sharing a tag, plus a third on its own tag: the graph is
    // one edge plus an isolated vertex ⇒ β₀ = 2, β₁ = 0.
    store.insert(&rec(a, "shared", 1.0));
    store.insert(&rec(b, "shared", 2.0));
    store.insert(&rec(a.wrapping_add(7), "alone", 3.0));

    let (b0, b1) = gigi::spectral::betti_numbers(&store);
    assert_eq!(store.len(), 3);
    assert_eq!(
        (b0, b1),
        (2, 0),
        "graph verbs saw a collapsed vertex set: β₀={b0}, β₁={b1}"
    );
}

#[test]
fn delete_then_reinsert_does_not_resurrect_a_stale_ordinal() {
    let mut store = BundleStore::new(schema());
    store.insert(&rec(1, "a", 1.0));
    store.insert(&rec(2, "a", 2.0));
    assert_eq!(store.neighborhood("tag", &Value::Text("a".into())).len(), 2);

    let mut k = Record::new();
    k.insert("id".into(), Value::Integer(1));
    assert!(store.delete(&k), "delete should report success");
    assert_eq!(
        store.neighborhood("tag", &Value::Text("a".into())).len(),
        1,
        "deleted record must leave the index"
    );

    // Re-inserting the same key must come back cleanly, and the survivor
    // must not have been disturbed.
    store.insert(&rec(1, "a", 9.0));
    let hits = store.neighborhood("tag", &Value::Text("a".into()));
    assert_eq!(hits.len(), 2, "re-inserted record must be indexed again");
    let mut ids: Vec<i64> = store
        .records()
        .filter(|r| hits.contains(&store.base_point(r)))
        .filter_map(|r| r.get("id").and_then(|v| v.as_i64()))
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, vec![1, 2]);
}

#[test]
fn truncate_resets_ordinals_without_stranding_bitmap_entries() {
    let mut store = BundleStore::new(schema());
    for i in 0..50 {
        store.insert(&rec(i, "t", i as f64));
    }
    assert_eq!(store.neighborhood("tag", &Value::Text("t".into())).len(), 50);

    store.truncate();
    assert_eq!(store.len(), 0);
    assert!(store.neighborhood("tag", &Value::Text("t".into())).is_empty());

    // Ordinals restart from zero; the cleared index must not let a fresh
    // ordinal collide with a stale entry.
    for i in 100..110 {
        store.insert(&rec(i, "t", i as f64));
    }
    let hits = store.neighborhood("tag", &Value::Text("t".into()));
    assert_eq!(hits.len(), 10, "post-truncate index is inconsistent");
    let live: std::collections::HashSet<u64> =
        store.records().map(|r| store.base_point(&r)).collect();
    for bp in &hits {
        assert!(
            live.contains(bp),
            "index points at base point {bp} with no record behind it"
        );
    }
}

#[test]
fn scale_every_record_reaches_the_index() {
    // 60k records is past the point where the truncating scheme reliably
    // dropped one (it did so at 30k on the SBF-5 fixture).
    let mut store = BundleStore::new(schema());
    for i in 0..60_000i64 {
        store.insert(&rec(i, if i % 2 == 0 { "even" } else { "odd" }, i as f64));
    }
    assert_eq!(store.len(), 60_000);
    let even = store.neighborhood("tag", &Value::Text("even".into())).len();
    let odd = store.neighborhood("tag", &Value::Text("odd".into())).len();
    println!("scale gate: {even} even + {odd} odd = {} indexed", even + odd);
    assert_eq!(
        even + odd,
        60_000,
        "{} record(s) missing from the index",
        60_000 - (even + odd)
    );
    // And the graph verbs must see one component (two buckets are disjoint,
    // so β₀ = 2 exactly — not 3, which is what a dropped record produced).
    let (b0, _) = gigi::spectral::betti_numbers(&store);
    assert_eq!(b0, 2, "β₀={b0} — a record fell out of every bucket");
}
