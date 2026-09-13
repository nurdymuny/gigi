//! The diagnostic that decides Halcyon's Q3: after reload, is a field that
//! was never written ABSENT, or PRESENT-BUT-NULL?
//!
//! It is present-but-null. That matters because a Null in a `numeric` field
//! is what produces "values in field 'v0' do not match its schema type" --
//! the exact refusal Marcella's intent_gate returned. A genuinely missing
//! field produces a different error. So the refusal text is evidence about
//! which of the two happened, and this test is the receipt for reading it
//! that way.
use std::fs;
use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

#[test]
fn unwritten_fields_reload_as_null_not_absent() {
    let d = std::env::temp_dir().join("gigi_null_probe");
    let _ = fs::remove_dir_all(&d);
    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(
            BundleSchema::new("sp")
                .base(FieldDef::categorical("record_id"))
                .fiber(FieldDef::categorical("tier"))
                .fiber(FieldDef::numeric("v0")),
        ).unwrap();
        // 3 records with NO v0 written (the "bad batch" shape)
        for i in 0..3i64 {
            let mut r = Record::new();
            r.insert("record_id".into(), Value::Text(format!("bad_{i}")));
            r.insert("tier".into(), Value::Text("bad".into()));
            e.insert("sp", &r).unwrap();
        }
        // 3 records WITH v0
        for i in 0..3i64 {
            let mut r = Record::new();
            r.insert("record_id".into(), Value::Text(format!("good_{i}")));
            r.insert("tier".into(), Value::Text("good".into()));
            r.insert("v0".into(), Value::Float(i as f64 + 0.5));
            e.insert("sp", &r).unwrap();
        }
        e.snapshot().unwrap();
    }
    let e = Engine::open_mmap(&d).unwrap();
    let store = e.bundle("sp").unwrap();
    let mut absent = 0usize;
    let mut null = 0usize;
    let mut real = 0usize;
    for r in store.records() {
        assert!(r.get("record_id").is_some(), "key must survive reload");
        match r.get("v0") {
            None => absent += 1,
            Some(Value::Null) => null += 1,
            Some(_) => real += 1,
        }
    }
    assert_eq!(absent, 0, "a declared field must never come back ABSENT");
    assert_eq!(null, 3, "the 3 records written without v0 must read back Null");
    assert_eq!(real, 3, "the 3 records written with v0 must keep their value");
    let _ = fs::remove_dir_all(&d);
}
