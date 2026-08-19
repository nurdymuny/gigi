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

/// Hallie's second review: the defect is not "NaN breaks Eq". Ord and PartialEq
/// are two independent, disagreeing definitions of equality, and they break
/// different containers in different directions.
#[ignore = "OPEN BUG (found 2026-08-15, Hallie review 2 of TDD-IDX). Ord and PartialEq are two independent, disagreeing definitions of equality on Value. Integer(1).cmp(Float(1.0)) is Equal while == is false, so a BTreeMap OVERWRITES across numeric types while a HashMap keeps them separate. Worse: there is no (Binary, Binary) arm in Ord::cmp (types.rs:82 fallthrough), so ALL Binary values compare Equal and a BTreeMap collapses every Binary key into one entry. This is the general form of the NaN defect and points the opposite way. See TDD-IDX section 2.7. Run with `cargo test -- --ignored`."]
#[test]
fn ord_and_partialeq_agree_on_equality() {
    use std::cmp::Ordering;
    use gigi::types::Value;

    // 1. cross-type numeric: Ord says Equal, PartialEq says not-equal
    let i = Value::Integer(1);
    let f = Value::Float(1.0);
    println!("  Integer(1) vs Float(1.0): cmp={:?} eq={}", i.cmp(&f), i == f);
    let mut t: BTreeMap<Value, &str> = BTreeMap::new();
    t.insert(i.clone(), "int");
    t.insert(f.clone(), "float");
    println!("  BTreeMap after both: len={} -> {:?}", t.len(), t.values().collect::<Vec<_>>());
    let mut h: HashMap<Value, &str> = HashMap::new();
    h.insert(i.clone(), "int");
    h.insert(f.clone(), "float");
    println!("  HashMap  after both: len={}", h.len());

    // 2. Binary has no (Binary, Binary) arm -> falls through to type_order
    let b1 = Value::Binary(vec![1, 2, 3]);
    let b2 = Value::Binary(vec![9, 9, 9]);
    println!("  Binary([1,2,3]) vs Binary([9,9,9]): cmp={:?} eq={}", b1.cmp(&b2), b1 == b2);
    let mut tb: BTreeMap<Value, &str> = BTreeMap::new();
    tb.insert(b1.clone(), "first");
    tb.insert(b2.clone(), "second");
    println!("  BTreeMap with 2 distinct Binary keys: len={}", tb.len());

    assert_ne!(i.cmp(&f), Ordering::Equal, "Ord must not equate Integer and Float");
    assert_ne!(b1.cmp(&b2), Ordering::Equal, "Ord must not equate distinct Binary values");
    assert_eq!(tb.len(), 2, "BTreeMap must keep distinct Binary keys distinct");
}
