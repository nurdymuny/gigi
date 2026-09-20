//! The write-ahead log carries derived key material in cleartext, beside the
//! ciphertext it protects.
//!
//! `encode_schema` (src/wal.rs) serializes `BundleSchema::gauge_key` verbatim:
//! the 32-byte Opaque AES key, the 32-byte Indexed CMAC key, the Affine scale
//! and offset, the Probabilistic bucket key, and the full Isometric matrix.
//! Anyone holding the data directory therefore holds both halves, and every
//! statement about what an encryption mode conceals at rest is true only
//! against an observer who does not have the directory.
//!
//! It is written because it cannot currently be re-derived. `GaugeKey::derive`
//! is deterministic in (seed, fiber field definitions), but `decode_field_def`
//! discards `encryption` and `encryption_group` on load and hardcodes
//! `EncryptionMode::None`, and `BundleSchema` does not record which seed source
//! it was built from. So the log stores the answer because it cannot reconstruct
//! the question.
//!
//! Found 2026-09-20 while answering HELICITY engineering's request for an
//! operation/disclosure matrix for KITT. They have said they will not publish
//! that matrix while this holds.
use gigi::crypto::{FieldTransform, GaugeKey};
use gigi::engine::Engine;
use gigi::types::{BundleSchema, EncryptionMode, EncryptionSeedSource, FieldDef};
use std::fs;

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.len() <= haystack.len() && haystack.windows(needle.len()).any(|w| w == needle)
}

const SEED_HEX: &str = "0707070707070707070707070707070707070707070707070707070707070707";

/// Build an encrypted bundle and return (data dir, derived key).
///
/// `source` decides whether the key is re-derivable on load. Only `Env` is:
/// the seed lives outside the data directory, so the log can record the
/// variable name instead of the key.
fn encrypted_bundle(
    dir_tag: &str,
    source: EncryptionSeedSource,
) -> (std::path::PathBuf, GaugeKey) {
    let dir = std::env::temp_dir().join(dir_tag);
    let _ = fs::remove_dir_all(&dir);

    let mut schema = BundleSchema::new("vault")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("amount").with_encryption(EncryptionMode::Opaque))
        .fiber(FieldDef::categorical("counterparty").with_encryption(EncryptionMode::Indexed))
        .with_seed_source(source);

    let seed = [7u8; 32];
    let key = GaugeKey::derive(&seed, &schema.fiber_fields);
    schema.gauge_key = Some(key.clone());

    {
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(schema).unwrap();
    }
    (dir, key)
}

/// Which of the derived secrets are byte-for-byte recoverable from the log.
fn leaked_from(wal: &[u8], key: &GaugeKey) -> Vec<String> {
    let mut leaked = Vec::new();
    for (i, t) in key.transforms.iter().enumerate() {
        match t {
            FieldTransform::Opaque { key } => {
                if contains(wal, key) {
                    leaked.push(format!("transform {i}: 32-byte Opaque AES key"));
                }
            }
            FieldTransform::Indexed { key } => {
                if contains(wal, key) {
                    leaked.push(format!("transform {i}: 32-byte Indexed CMAC key"));
                }
            }
            FieldTransform::Affine { scale, offset } => {
                if contains(wal, &scale.to_le_bytes()) && contains(wal, &offset.to_le_bytes()) {
                    leaked.push(format!("transform {i}: Affine scale and offset"));
                }
            }
            _ => {}
        }
    }
    leaked
}

#[test]
fn wal_does_not_carry_derived_key_material() {
    std::env::set_var("GIGI_TEST_SEED_A", SEED_HEX);
    let (dir, key) = encrypted_bundle(
        "gigi_wal_key_material_env",
        EncryptionSeedSource::Env("GIGI_TEST_SEED_A".into()),
    );
    let wal = fs::read(dir.join("gigi.wal")).expect("wal written");
    let leaked = leaked_from(&wal, &key);
    let _ = fs::remove_dir_all(&dir);
    assert!(
        leaked.is_empty(),
        "derived key material is recoverable from gigi.wal alone: {}. The log must record how to re-derive the key, never the key itself.",
        leaked.join("; ")
    );
}

/// The key survives the round trip, re-derived rather than read.
///
/// Without this, the test above could be satisfied by simply dropping the key
/// and leaving every encrypted bundle unreadable after a restart.
#[test]
fn env_sourced_key_is_rederived_on_reopen() {
    std::env::set_var("GIGI_TEST_SEED_B", SEED_HEX);
    let (dir, key) = encrypted_bundle(
        "gigi_wal_key_rederive",
        EncryptionSeedSource::Env("GIGI_TEST_SEED_B".into()),
    );

    let engine = Engine::open(&dir).unwrap();
    let restored = engine
        .bundle_schema("vault")
        .expect("schema replayed")
        .clone();
    drop(engine);
    let _ = fs::remove_dir_all(&dir);

    let gk = restored.gauge_key.expect("gauge key re-derived on load");
    // FieldTransform carries raw key bytes and derives Debug but not PartialEq;
    // the debug form compares every field of every variant.
    assert_eq!(
        format!("{:?}", gk.transforms),
        format!("{:?}", key.transforms),
        "re-derived key does not match the original"
    );
    assert_eq!(
        restored.fiber_fields[0].encryption,
        EncryptionMode::Opaque,
        "restored schema must carry the real per-field encryption mode, not plaintext"
    );
    assert_eq!(restored.fiber_fields[1].encryption, EncryptionMode::Indexed);
}

/// The seed is load-bearing, which is what proves the key is re-derived rather
/// than read back.
///
/// Without this, the two tests above pass just as well when the key is written
/// to the log: one checks that no material is present, the other that the key
/// still works, and neither alone forces re-derivation. Removing the variable
/// separates them — if the key were in the log, this reopen would succeed.
#[test]
fn reopen_without_the_seed_refuses_loudly() {
    std::env::set_var("GIGI_TEST_SEED_C", SEED_HEX);
    let (dir, _key) = encrypted_bundle(
        "gigi_wal_key_no_seed",
        EncryptionSeedSource::Env("GIGI_TEST_SEED_C".into()),
    );
    std::env::remove_var("GIGI_TEST_SEED_C");

    // Engine is not Debug, so no expect_err here.
    let msg = match Engine::open(&dir) {
        Ok(_) => panic!("engine opened an encrypted bundle with no seed available"),
        Err(e) => e.to_string(),
    };
    let _ = fs::remove_dir_all(&dir);

    assert!(
        msg.contains("vault") && msg.contains("GIGI_TEST_SEED_C"),
        "the refusal must name the bundle and the variable so the operator knows what to restore; got: {msg}"
    );
}

/// A seed that only the creating process knows cannot be re-derived at load,
/// so the sole way to make such a bundle durable would be to journal its key
/// beside the ciphertext. The engine refuses to create one instead.
///
/// Decided 2026-09-20: require the env seed, refuse the other two.
#[test]
fn encrypted_bundle_with_a_non_env_seed_is_refused() {
    let cases = [
        ("random", EncryptionSeedSource::Random),
        ("hex", EncryptionSeedSource::Hex(SEED_HEX.to_string())),
    ];
    for (tag, source) in cases {
        let dir = std::env::temp_dir().join(format!("gigi_wal_refuse_{tag}"));
        let _ = fs::remove_dir_all(&dir);

        let mut schema = BundleSchema::new("vault")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("amount").with_encryption(EncryptionMode::Opaque))
            .with_seed_source(source);
        schema.gauge_key = Some(GaugeKey::derive(&[7u8; 32], &schema.fiber_fields));

        let mut engine = Engine::open(&dir).unwrap();
        let msg = match engine.create_bundle(schema) {
            Ok(()) => panic!("{tag} seed: engine created a bundle whose key it cannot re-derive"),
            Err(e) => e.to_string(),
        };
        drop(engine);

        // The log must not carry the key of a bundle that was refused, either.
        let wal = fs::read(dir.join("gigi.wal")).unwrap_or_default();
        let key = GaugeKey::derive(&[7u8; 32], &[FieldDef::numeric("amount")
            .with_encryption(EncryptionMode::Opaque)]);
        let leaked = leaked_from(&wal, &key);
        let _ = fs::remove_dir_all(&dir);

        assert!(
            msg.contains("SEED FROM ENV"),
            "{tag} seed: the refusal must name the supported form; got: {msg}"
        );
        assert!(
            leaked.is_empty(),
            "{tag} seed: refused bundle still wrote key material: {}",
            leaked.join("; ")
        );
    }
}

/// A bundle that encrypts nothing is unaffected by the seed rule.
#[test]
fn plaintext_bundle_is_not_subject_to_the_seed_rule() {
    let dir = std::env::temp_dir().join("gigi_wal_plaintext");
    let _ = fs::remove_dir_all(&dir);
    let schema = BundleSchema::new("open")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("amount"));
    let mut engine = Engine::open(&dir).unwrap();
    let created = engine.create_bundle(schema);
    drop(engine);
    let _ = fs::remove_dir_all(&dir);
    assert!(created.is_ok(), "plaintext bundle refused: {created:?}");
}
