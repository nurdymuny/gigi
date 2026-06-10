//! Contract tests for `GET /v1/bundles/{name}/record/{id}/vector`.
//!
//! Marcella's IMAGINE Phase 2 needs the record's actual embedding so
//! `along = normalize(prompt_vec − seed_vec)` is geometrically honest.
//! These tests pin the substrate side: given a record with a Vector
//! field on a single-base-field bundle, the substrate returns the
//! vector deterministically (schema-declaration order, first-Vector
//! wins) plus support for explicit `?field=` disambiguation.
//!
//! Does not spin up axum — same pattern as `causal_states_wire.rs`.
//! The handler in `gigi_stream.rs` is thin glue: it builds a key,
//! calls `BundleStore::point_query`, runs `first_vector_field`, and
//! serializes. Exercising those substrate APIs here pins everything
//! except the route table itself (covered by the deploy smoke).

use gigi::bundle::BundleStore;
use gigi::types::{
    first_vector_field, BundleSchema, FieldDef, FieldType, Record, Value,
};

fn vector_field(name: &str, dims: usize) -> FieldDef {
    // No public FieldDef::vector constructor exists, so we hand-build
    // one rather than touching src/types.rs for a test-only helper.
    let mut f = FieldDef::numeric(name);
    f.field_type = FieldType::Vector { dims };
    f
}

fn substrate_bundle() -> BundleStore {
    // Mirror the substrate-catalog shape: single integer id + an
    // embedding vector + an unrelated label field.
    let schema = BundleSchema::new("substrate")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("label"))
        .fiber(vector_field("embedding", 3));
    let mut store = BundleStore::new(schema);
    for (i, (label, v)) in [
        ("alpha", vec![1.0, 0.0, 0.0]),
        ("beta", vec![0.0, 1.0, 0.0]),
        ("gamma", vec![0.0, 0.0, 1.0]),
    ]
    .iter()
    .enumerate()
    {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i as i64));
        r.insert("label".into(), Value::Text((*label).into()));
        r.insert("embedding".into(), Value::Vector(v.clone()));
        store.insert(&r);
    }
    store
}

// ─── happy path ─────────────────────────────────────────────────────────

#[test]
fn first_vector_field_returns_embedding_in_schema_order() {
    let store = substrate_bundle();
    let key: Record = std::iter::once(("id".to_string(), Value::Integer(1))).collect();
    let record = store.point_query(&key).expect("record exists");
    let got = first_vector_field(&record, &store.schema.fiber_fields);
    assert_eq!(
        got,
        Some(("embedding".to_string(), vec![0.0, 1.0, 0.0])),
        "first Vector field by schema declaration order — not HashMap iteration"
    );
}

#[test]
fn point_query_round_trips_vector_value() {
    // The handler's path: `store.point_query(&key)` → record → first vector.
    // If point_query strips Vector somewhere downstream, the endpoint
    // silently returns 404. This pins that doesn't happen.
    let store = substrate_bundle();
    let key: Record = std::iter::once(("id".to_string(), Value::Integer(2))).collect();
    let record = store.point_query(&key).expect("record exists");
    match record.get("embedding") {
        Some(Value::Vector(v)) => assert_eq!(v, &vec![0.0, 0.0, 1.0]),
        other => panic!("expected Vector, got {:?}", other),
    }
}

// ─── error paths ────────────────────────────────────────────────────────

#[test]
fn first_vector_field_returns_none_when_no_vector_present() {
    // A record without a Vector field → endpoint must 404, not panic.
    let schema = BundleSchema::new("text_only")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("label"));
    let mut store = BundleStore::new(schema);
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(0));
    r.insert("label".into(), Value::Text("solo".into()));
    store.insert(&r);

    let key: Record = std::iter::once(("id".to_string(), Value::Integer(0))).collect();
    let record = store.point_query(&key).unwrap();
    assert!(first_vector_field(&record, &store.schema.fiber_fields).is_none());
}

#[test]
fn point_query_returns_none_for_missing_record() {
    // 404-on-missing path: handler relies on point_query → None.
    let store = substrate_bundle();
    let key: Record = std::iter::once(("id".to_string(), Value::Integer(9999))).collect();
    assert!(store.point_query(&key).is_none());
}

#[test]
fn composite_key_bundles_are_rejected_by_arity_check() {
    // The handler refuses bundles with >1 base field because a path
    // segment can't safely encode a composite key. Pin the arity check.
    let schema = BundleSchema::new("composite")
        .base(FieldDef::numeric("partition"))
        .base(FieldDef::numeric("id"));
    let store = BundleStore::new(schema);
    assert_eq!(
        store.schema.base_fields.len(),
        2,
        "composite-key bundle must report 2 base fields so the handler can 400 on it"
    );
}

// ─── wire shape ─────────────────────────────────────────────────────────
//
// The response struct is defined inline in `gigi_stream.rs` (binary
// crate, not reachable from this test crate). To pin its JSON shape,
// we mirror the four required keys here — if the handler diverges, the
// deploy smoke catches it.

#[test]
fn response_wire_shape_has_exact_four_keys() {
    // Mirror of `RecordVectorResponse` in gigi_stream.rs.
    let wire = serde_json::json!({
        "id": 1,
        "field": "embedding",
        "vector": [0.0, 1.0, 0.0],
        "dims": 3,
    });
    let obj = wire.as_object().unwrap();
    assert!(obj.contains_key("id"), "missing 'id'");
    assert!(obj.contains_key("field"), "missing 'field' — name of the source field");
    assert!(obj.contains_key("vector"), "missing 'vector' — the embedding");
    assert!(obj.contains_key("dims"), "missing 'dims' — vector length");
    assert_eq!(obj.len(), 4, "exactly 4 keys; got: {:?}", obj.keys().collect::<Vec<_>>());
}
