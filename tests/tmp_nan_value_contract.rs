//! Probe: Value's Eq / Hash / Ord disagree on NaN.
use gigi::types::Value;
use std::collections::{BTreeMap, HashMap};

#[ignore = "OPEN BUG (found 2026-08-15 while revising TDD-IDX). Value's Eq, \
Hash and Ord disagree on NaN. PartialEq is derived, so Float(NaN) != Float(NaN) \
by IEEE; Hash uses to_bits() so both hash alike; Ord uses total_cmp() so both \
compare Equal. `impl Eq for Value {}` (types.rs:38) therefore asserts a \
reflexivity that does not hold. Consequence: HashMap<Value, _> grows a new \
UNREACHABLE entry per NaN key (get returns None immediately after insert), \
while BTreeMap<Value, _> is fine. field_index is a HashMap<Value, \
RoaringBitmap> (bundle.rs:3298), so every NaN-valued record becomes its own \
singleton bucket and leaks an index entry. Wider than indexing; see \
theory/gigi/TDD-IDX_index_set_durability.md section 2.7 and V-8. Run with \
`cargo test -- --ignored` to see it fail."]
#[test]
fn nan_value_contracts_are_consistent() {
    let a = Value::Float(f64::NAN);
    let b = Value::Float(f64::NAN);

    println!("  PartialEq: a == b  -> {}", a == b);
    println!("  Ord:       a.cmp(b) -> {:?}", a.cmp(&b));

    let mut h: HashMap<Value, i32> = HashMap::new();
    h.insert(a.clone(), 1);
    println!("  HashMap: inserted 1 NaN key, len={}, get(NaN)={:?}",
             h.len(), h.get(&b));
    h.insert(b.clone(), 2);
    println!("  HashMap: after inserting a 2nd NaN key, len={}", h.len());

    let mut t: BTreeMap<Value, i32> = BTreeMap::new();
    t.insert(a.clone(), 1);
    t.insert(b.clone(), 2);
    println!("  BTreeMap: after 2 NaN keys, len={}", t.len());

    assert_eq!(a, b, "Eq is implemented for Value, so NaN must equal itself");
    assert_eq!(h.len(), 1, "a HashMap must not grow an unreachable NaN bucket");
}
