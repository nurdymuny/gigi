//! The log must not carry the plaintext of an encrypted bundle.
//!
//! It used to. `Engine::insert` journalled the caller's record and then handed
//! it to the store, which is where the gauge transform is applied, so the log
//! entry was written from the untransformed record. An observer holding the
//! data directory did not need the key, because there was nothing they needed
//! to decrypt.
//!
//! Fixed 2026-09-20 by sealing before journalling: the log carries the
//! transformed values and replay unseals them before the store sees them. The
//! stored copy and the logged copy are independent, since an Opaque field draws
//! a fresh nonce each time -- the log is a recipe for reconstructing the
//! record, not a mirror of how it is stored.
//!
//! Found by probing the whole data directory for a canary rather than reading
//! the code. An earlier probe of `gigi.wal` alone reported clean and was wrong:
//! it compared a 20-byte window against a 21-byte needle, which can never
//! match. These walk every file under the data root.
use gigi::crypto::GaugeKey;
use gigi::engine::Engine;
use gigi::types::{BundleSchema, EncryptionMode, EncryptionSeedSource, FieldDef, Record, Value};
use std::fs;

const CANARY: &[u8; 21] = b"CANARY-SWORDFISH-9931";
const SEED_HEX: &str = "0707070707070707070707070707070707070707070707070707070707070707";

/// Every file under `dir` that contains the canary, by file name.
fn files_containing_canary(dir: &std::path::Path) -> Vec<String> {
    let mut hits = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let bytes = fs::read(&path).unwrap_or_default();
            if bytes.windows(CANARY.len()).any(|w| w == CANARY) {
                hits.push(path.file_name().unwrap().to_string_lossy().into_owned());
            }
        }
    }
    hits
}

#[test]
fn wal_does_not_carry_plaintext_fiber_values() {
    std::env::set_var("GIGI_PLAINTEXT_PROBE_SEED", SEED_HEX);
    let dir = std::env::temp_dir().join("gigi_wal_plaintext_fiber");
    let _ = fs::remove_dir_all(&dir);

    let mut schema = BundleSchema::new("vault")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("secret").with_encryption(EncryptionMode::Opaque))
        .with_seed_source(EncryptionSeedSource::Env("GIGI_PLAINTEXT_PROBE_SEED".into()));
    schema.gauge_key = Some(GaugeKey::derive(&[7u8; 32], &schema.fiber_fields));

    {
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(schema).unwrap();
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("secret".into(), Value::Text(String::from_utf8_lossy(CANARY).into_owned()));
        engine.insert("vault", &r).unwrap();
    }

    let hits = files_containing_canary(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(
        hits.is_empty(),
        "the plaintext fiber value of an encrypted bundle is on disk in: {hits:?}"
    );
}

/// Sealing is only correct if replay can undo it.
///
/// Without this, the test above passes just as well when replay is broken and
/// the records come back as ciphertext or not at all.
#[test]
fn a_sealed_record_survives_a_restart() {
    std::env::set_var("GIGI_ROUNDTRIP_SEED", SEED_HEX);
    let dir = std::env::temp_dir().join("gigi_wal_sealed_roundtrip");
    let _ = fs::remove_dir_all(&dir);

    let mut schema = BundleSchema::new("vault")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("secret").with_encryption(EncryptionMode::Opaque))
        .with_seed_source(EncryptionSeedSource::Env("GIGI_ROUNDTRIP_SEED".into()));
    schema.gauge_key = Some(GaugeKey::derive(&[7u8; 32], &schema.fiber_fields));

    {
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(schema).unwrap();
        for (id, secret) in [(1i64, "tuna"), (2, "marlin")] {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(id));
            r.insert("secret".into(), Value::Text(secret.into()));
            engine.insert("vault", &r).unwrap();
        }
    }

    let engine = Engine::open(&dir).unwrap();
    let mut secrets: Vec<String> = engine
        .bundle("vault")
        .unwrap()
        .records()
        .filter_map(|r| r.get("secret").and_then(|v| v.as_str().map(String::from)))
        .collect();
    drop(engine);
    let _ = fs::remove_dir_all(&dir);
    secrets.sort();
    assert_eq!(
        secrets,
        vec!["marlin".to_string(), "tuna".to_string()],
        "replay must unseal what insert sealed"
    );
}

/// The update path writes its patch fields to the log too.
#[ignore = "OPEN BUG (found 2026-09-20, same family as the insert leak fixed that day). Engine::update journals the patch record before the store applies the gauge transform, so a patch that touches an encrypted fiber field puts that value in the log in the clear. Not fixed with the insert path because a patch is partial, and a field in an Isometric group cannot be sealed on its own -- the group's members are transformed together, so sealing one member of a partial patch needs the others, which the patch does not carry. Wants a decision about partial groups before it wants code. Run with `cargo test -- --ignored`."]
#[test]
fn an_update_does_not_carry_plaintext_either() {
    std::env::set_var("GIGI_UPDATE_PROBE_SEED", SEED_HEX);
    let dir = std::env::temp_dir().join("gigi_wal_update_plaintext");
    let _ = fs::remove_dir_all(&dir);

    let mut schema = BundleSchema::new("vault")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("secret").with_encryption(EncryptionMode::Opaque))
        .with_seed_source(EncryptionSeedSource::Env("GIGI_UPDATE_PROBE_SEED".into()));
    schema.gauge_key = Some(GaugeKey::derive(&[7u8; 32], &schema.fiber_fields));

    {
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(schema).unwrap();
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("secret".into(), Value::Text("placeholder".into()));
        engine.insert("vault", &r).unwrap();

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let mut patch = Record::new();
        patch.insert("secret".into(), Value::Text(String::from_utf8_lossy(CANARY).into_owned()));
        let _ = engine.update("vault", &key, &patch);
    }

    let hits = files_containing_canary(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(hits.is_empty(), "an updated plaintext value is on disk in: {hits:?}");
}
