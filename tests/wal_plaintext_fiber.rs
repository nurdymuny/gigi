//! The write-ahead log holds the PLAINTEXT fiber values of an encrypted bundle.
//!
//! `Engine::insert` journals the caller's record and then hands it to
//! `BundleStore::insert`, which is where the gauge transform is applied. The log
//! entry is written first and is written from the pre-transform record, so for
//! an encrypted bundle the log carries exactly the values the ciphertext was
//! meant to conceal.
//!
//! This is larger than the key material that used to sit beside it, which was
//! fixed on 2026-09-20. An observer holding the data directory does not need the
//! key, because they do not need to decrypt anything.
//!
//! Not fixed here because the fix is a design decision, not a patch. The log has
//! to carry something replay can use. Either it carries ciphertext and replay
//! stops re-encrypting, which changes what a replayed insert means and
//! interacts with rekeying, or the log payload is itself encrypted, which puts
//! key handling on the durability path. Both are choices about the engine's
//! contract.
//!
//! Found 2026-09-20 while fixing the rekey path, by probing the whole data
//! directory for a canary value rather than reading the code. An earlier probe
//! of `gigi.wal` alone reported clean and was wrong: it compared a 20-byte
//! window against a 21-byte needle, which can never match. The reproduction
//! below searches every file under the data root.
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

#[ignore = "OPEN BUG (found 2026-09-20). Engine::insert journals the caller's record BEFORE BundleStore::insert applies the gauge transform, so gigi.wal carries the plaintext fiber values of an encrypted bundle. An observer with the data directory does not need the key. Not a patch: the log must carry something replay can use, so the choice is between journalling ciphertext (and stopping replay from re-encrypting, which interacts with rekey) or encrypting the log payload (which puts key handling on the durability path). Needs a decision before it needs code. Run with `cargo test -- --ignored`."]
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
