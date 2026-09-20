//! Rotating a bundle's key has to survive the restart it exists to protect.
//!
//! `BundleStore::rotate_key` rebuilds the store around a new key and knows
//! nothing about the log. Both callers stopped there, so a rotation was never
//! journalled: a restart restored the pre-rotation schema. After the key
//! material fix of 2026-09-20 that turned into something sharper than a lost
//! setting, because the schema also kept the seed source it was created with —
//! so a restart would re-derive the OLD key and hand it to records that had
//! been re-encrypted under the new one.
//!
//! `Engine::rotate_key` is the door both callers now use. These pin what it
//! guarantees.
use gigi::crypto::GaugeKey;
use gigi::engine::Engine;
use gigi::types::{
    BundleSchema, EncryptionMode, EncryptionSeedSource, FieldDef, Record, Value,
};
use std::fs;

const SEED_A: &str = "0101010101010101010101010101010101010101010101010101010101010101";
const SEED_B: &str = "0202020202020202020202020202020202020202020202020202020202020202";

fn schema_for(var: &str) -> BundleSchema {
    let mut s = BundleSchema::new("vault")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("secret").with_encryption(EncryptionMode::Opaque))
        .with_seed_source(EncryptionSeedSource::Env(var.into()));
    let seed = gigi::crypto::seed_from_hex(SEED_A).unwrap();
    s.gauge_key = Some(GaugeKey::derive(&seed, &s.fiber_fields));
    s
}

fn record(id: i64, secret: &str) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(id));
    r.insert("secret".into(), Value::Text(secret.into()));
    r
}

#[test]
fn rotation_survives_a_restart_and_rebinds_the_seed() {
    std::env::set_var("GIGI_REKEY_OLD", SEED_A);
    std::env::set_var("GIGI_REKEY_NEW", SEED_B);
    let dir = std::env::temp_dir().join("gigi_rekey_durable");
    let _ = fs::remove_dir_all(&dir);

    {
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(schema_for("GIGI_REKEY_OLD")).unwrap();
        engine.insert("vault", &record(1, "tuna")).unwrap();
        let rotated = engine
            .rotate_key("vault", &EncryptionSeedSource::Env("GIGI_REKEY_NEW".into()))
            .expect("rotation");
        assert_eq!(rotated, 1, "one record re-encrypted");
    }

    let engine = Engine::open(&dir).unwrap();
    let restored = engine.bundle_schema("vault").expect("schema replayed").clone();

    // The rotation is recorded, not just applied.
    assert_eq!(
        restored.seed_source,
        EncryptionSeedSource::Env("GIGI_REKEY_NEW".into()),
        "a restart must not restore the pre-rotation seed source"
    );

    // And the key it yields is the NEW one, not the one it was created with.
    let new_seed = gigi::crypto::seed_from_hex(SEED_B).unwrap();
    let old_seed = gigi::crypto::seed_from_hex(SEED_A).unwrap();
    let expected = GaugeKey::derive(&new_seed, &restored.fiber_fields);
    let stale = GaugeKey::derive(&old_seed, &restored.fiber_fields);
    let got = format!("{:?}", restored.gauge_key.as_ref().unwrap().transforms);
    assert_eq!(got, format!("{:?}", expected.transforms), "key is the rotated one");
    assert_ne!(got, format!("{:?}", stale.transforms), "key is not the original");

    // The data is still readable through it.
    let secrets: Vec<String> = engine
        .bundle("vault")
        .unwrap()
        .records()
        .filter_map(|r| r.get("secret").and_then(|v| v.as_str().map(String::from)))
        .collect();
    drop(engine);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(secrets, vec!["tuna".to_string()]);
}

#[test]
fn rotation_to_a_non_env_seed_is_refused() {
    std::env::set_var("GIGI_REKEY_ONLY", SEED_A);
    let dir = std::env::temp_dir().join("gigi_rekey_refuse");
    let _ = fs::remove_dir_all(&dir);

    let mut engine = Engine::open(&dir).unwrap();
    engine.create_bundle(schema_for("GIGI_REKEY_ONLY")).unwrap();

    for source in [
        EncryptionSeedSource::Random,
        EncryptionSeedSource::Hex(SEED_B.to_string()),
    ] {
        let msg = match engine.rotate_key("vault", &source) {
            Ok(_) => panic!("rotated to a seed the engine cannot resolve again: {source:?}"),
            Err(e) => e.to_string(),
        };
        assert!(
            msg.contains("SEED FROM ENV"),
            "the refusal must name the supported form; got: {msg}"
        );
    }

    // Refused means unchanged, not half-applied.
    let seed = gigi::crypto::seed_from_hex(SEED_A).unwrap();
    let original = GaugeKey::derive(&seed, &engine.bundle_schema("vault").unwrap().fiber_fields);
    let now = format!(
        "{:?}",
        engine.bundle_schema("vault").unwrap().gauge_key.as_ref().unwrap().transforms
    );
    drop(engine);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(now, format!("{:?}", original.transforms), "key changed despite refusal");
}

#[test]
fn rotation_names_a_missing_secret() {
    std::env::set_var("GIGI_REKEY_PRESENT", SEED_A);
    std::env::remove_var("GIGI_REKEY_ABSENT");
    let dir = std::env::temp_dir().join("gigi_rekey_missing");
    let _ = fs::remove_dir_all(&dir);

    let mut engine = Engine::open(&dir).unwrap();
    engine.create_bundle(schema_for("GIGI_REKEY_PRESENT")).unwrap();
    let msg = match engine.rotate_key("vault", &EncryptionSeedSource::Env("GIGI_REKEY_ABSENT".into())) {
        Ok(_) => panic!("rotated using a secret that is not set"),
        Err(e) => e.to_string(),
    };
    drop(engine);
    let _ = fs::remove_dir_all(&dir);
    assert!(
        msg.contains("GIGI_REKEY_ABSENT") && msg.contains("vault"),
        "the refusal must name the bundle and the variable; got: {msg}"
    );
}
