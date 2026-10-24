//! Registry claim values must remain exact text across snapshot/restart.
//! A numeric-looking string is not a request to change its stored type.

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

#[test]
fn registry_numeric_text_survives_snapshot_and_mmap() {
    let dir = tempfile::tempdir().unwrap();
    let values = [
        "1",
        "1.0",
        "001",
        "0.12345678901234567890123456789",
        "2e3",
        "r",
        "T",
        "F",
        "null",
    ];
    {
        let mut engine = Engine::open(dir.path()).unwrap();
        engine.compaction_policy_mut().disabled = true;
        engine
            .create_bundle(
                BundleSchema::new("claims")
                    .base(FieldDef::categorical("id"))
                    .fiber(FieldDef::categorical("value"))
                    .fiber(FieldDef::categorical("constant")),
            )
            .unwrap();
        for (index, text) in values.iter().enumerate() {
            let mut record = Record::new();
            record.insert("id".into(), Value::Text(format!("claim-{index}")));
            record.insert("value".into(), Value::Text(text.to_string()));
            // Repeated numeric-looking text exercises header-default folding.
            record.insert("constant".into(), Value::Text("1.0".into()));
            engine.insert("claims", &record).unwrap();
        }
        engine.snapshot().unwrap();
    }
    for mmap in [false, true] {
        let engine = if mmap {
            Engine::open_mmap(dir.path())
        } else {
            Engine::open(dir.path())
        }
        .unwrap();
        for (index, text) in values.iter().enumerate() {
            let mut key = Record::new();
            key.insert("id".into(), Value::Text(format!("claim-{index}")));
            let row = engine.point_query("claims", &key).unwrap().unwrap();
            assert_eq!(
                row.get("value"),
                Some(&Value::Text(text.to_string())),
                "mmap={mmap}, claim={index}"
            );
            assert_eq!(
                row.get("constant"),
                Some(&Value::Text("1.0".into())),
                "mmap={mmap}, constant"
            );
        }
    }
}

#[test]
fn escaped_strings_and_real_typed_values_remain_distinct() {
    let source = serde_json::json!({"claims": [
        {"id": "a", "text": "1", "typed": 1},
        {"id": "b", "text": "T", "typed": true},
        {"id": "c", "text": "null", "typed": null},
        {"id": "d", "text": "\u{001f}[1,2]", "typed": [1, 2]},
        {"id": "e", "text": "line one\nline two", "typed": 3.25}
    ]});
    let encoded = gigi::dhoom::encode(&source).unwrap();
    let decoded = gigi::dhoom::decode(&encoded).unwrap();
    assert_eq!(decoded, source);
}
