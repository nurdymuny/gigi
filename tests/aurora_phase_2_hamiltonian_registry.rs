//! AURORA Phase 2 — hamiltonian_registry + WAL HamiltonianDeclare
//! event (RED-first tests).
//!
//! Receipt: this workflow ships the register/get_factory API +
//! WAL emission of HamiltonianDeclare. Per Q5 eager-init contract,
//! the registry starts empty and is only populated by explicit
//! register() calls from the host binary's main(). No lazy_static,
//! no OnceCell auto-init, no thread-local first-use registration.
//!
//! BIT-IDENTITY: strictly additive. No existing module modified
//! beyond the new pub mod + WAL op constant + WalEntry variant.
//!
//! WAL replay handling of HamiltonianDeclare is EXPLICITLY out of
//! scope here — the event is emitted and persisted, but engine
//! replay is a follow-up workflow.

#![cfg(feature = "halcyon")]

use std::collections::{BTreeMap, HashMap};

use gigi::gauge::action::{
    EnergyDecomposition, EnergyError, FactoryError, HamiltonianDrift, HamiltonianFactory,
    HamiltonianForce, HamiltonianHandle, ProjectionError, ProjectionOperator,
};
use gigi::gauge::hamiltonian_registry::{self, RegistryError};
use gigi::wal::{WalEntry, WalReader, WalWriter};

// ─────────────────────────────────────────────────────────────────────
// Stub factory used by registry tests
// ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct StubHamiltonian;

impl HamiltonianForce for StubHamiltonian {
    fn force(&self, _state: &[f64]) -> Vec<f64> {
        Vec::new()
    }
}
impl HamiltonianDrift for StubHamiltonian {
    fn drift(&self, state: &[f64], _dt: f64) -> Vec<f64> {
        state.to_vec()
    }
}
impl ProjectionOperator for StubHamiltonian {
    fn project_constraint(&self, _state: &mut [f64]) -> Result<(), ProjectionError> {
        Ok(())
    }
}
impl EnergyDecomposition for StubHamiltonian {
    fn energy_keys(&self) -> &'static [&'static str] {
        &["e_stub"]
    }
    fn evaluate(&self, _state: &[f64]) -> Result<BTreeMap<String, f64>, EnergyError> {
        let mut m = BTreeMap::new();
        m.insert("e_stub".to_string(), 0.0);
        Ok(m)
    }
}
impl HamiltonianHandle for StubHamiltonian {}

struct StubFactory {
    kind: &'static str,
    group: &'static str,
}

impl HamiltonianFactory for StubFactory {
    fn kind_tag(&self) -> &'static str {
        self.kind
    }
    fn group_tag(&self) -> &'static str {
        self.group
    }
    fn from_params(
        &self,
        _params: &HashMap<String, f64>,
    ) -> Result<Box<dyn HamiltonianHandle>, FactoryError> {
        Ok(Box::new(StubHamiltonian))
    }
}

fn stub_factory(kind: &'static str, group: &'static str) -> Box<dyn HamiltonianFactory> {
    Box::new(StubFactory { kind, group })
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

/// register() then with_factory() round-trips a stub.
#[test]
fn test_registry_register_then_lookup() {
    hamiltonian_registry::clear();
    hamiltonian_registry::register("rrl_alpha", stub_factory("STUB_A", "R"), None, 1)
        .expect("first register must succeed");

    let kind = hamiltonian_registry::with_factory("rrl_alpha", |f| f.kind_tag().to_string())
        .expect("lookup must return Some");
    assert_eq!(kind, "STUB_A");
}

/// Unknown name returns None — never a silent default.
#[test]
fn test_registry_unknown_name_returns_none() {
    hamiltonian_registry::clear();
    let got = hamiltonian_registry::with_factory("does_not_exist", |f| f.kind_tag().to_string());
    assert!(got.is_none(), "unknown name must return None");
    assert!(!hamiltonian_registry::contains("does_not_exist"));
}

/// Duplicate register() with the same name returns
/// RegistryError::DuplicateName — last-write-wins is a footgun the
/// substrate explicitly refuses.
#[test]
fn test_registry_duplicate_name_rejects() {
    hamiltonian_registry::clear();
    hamiltonian_registry::register("dup", stub_factory("FIRST", "R"), None, 1)
        .expect("first register must succeed");

    let err = hamiltonian_registry::register("dup", stub_factory("SECOND", "R"), None, 2)
        .expect_err("second register must reject");
    match err {
        RegistryError::DuplicateName { name } => assert_eq!(name, "dup"),
        other => panic!("expected DuplicateName, got {other:?}"),
    }

    // The first factory remains in place — duplicate is rejected, not
    // overwritten.
    let kind = hamiltonian_registry::with_factory("dup", |f| f.kind_tag().to_string()).unwrap();
    assert_eq!(kind, "FIRST", "duplicate must not overwrite");
}

/// get_factory("foo").kind_tag() round-trips the registered tag.
#[test]
fn test_registry_factory_kind_tag_round_trip() {
    hamiltonian_registry::clear();
    hamiltonian_registry::register("foo", stub_factory("STUB", "R"), None, 1)
        .expect("register must succeed");
    let kind = hamiltonian_registry::with_factory("foo", |f| f.kind_tag()).unwrap();
    assert_eq!(kind, "STUB");
    let group = hamiltonian_registry::with_factory("foo", |f| f.group_tag()).unwrap();
    assert_eq!(group, "R");
}

/// register() with a WalWriter emits a HamiltonianDeclare entry with
/// the correct (name, kind_tag, group_tag, registered_at) tuple.
#[test]
fn test_wal_hamiltonian_declare_event_emitted() {
    hamiltonian_registry::clear();

    let dir = tempdir_for_test("aurora_phase_2_wal_emit");
    let wal_path = dir.join("test.wal");
    let mut writer = WalWriter::open(&wal_path).expect("open wal");

    hamiltonian_registry::register(
        "shallow_water_demo",
        stub_factory("SHALLOW_WATER", "R"),
        Some(&mut writer),
        42,
    )
    .expect("register must succeed");
    writer.sync().expect("sync");
    drop(writer);

    let mut reader = WalReader::open(&wal_path).expect("open reader");
    let entries = reader.read_all().expect("read_all");
    let declare = entries
        .into_iter()
        .find_map(|e| match e {
            WalEntry::HamiltonianDeclare {
                name,
                kind_tag,
                group_tag,
                registered_at,
            } => Some((name, kind_tag, group_tag, registered_at)),
            _ => None,
        })
        .expect("WAL must contain a HamiltonianDeclare entry");

    assert_eq!(declare.0, "shallow_water_demo");
    assert_eq!(declare.1, "SHALLOW_WATER");
    assert_eq!(declare.2, "R");
    assert_eq!(declare.3, 42);
}

/// Q5 eager-init contract: clear() then with_factory() returns None
/// for any name. No lazy auto-population, no first-use hook, no
/// thread-local magic.
#[test]
fn test_registry_eager_init_no_auto_populate() {
    hamiltonian_registry::clear();
    assert!(hamiltonian_registry::with_factory("anything", |_| ()).is_none());
    assert!(hamiltonian_registry::with_factory("SHALLOW_WATER", |_| ()).is_none());
    assert!(hamiltonian_registry::with_factory("KOGUT_SUSSKIND", |_| ()).is_none());
    assert!(hamiltonian_registry::list_registered().is_empty());
}

/// list_registered returns (name, kind_tag, group_tag) triples for
/// every active registration — useful for introspection HTTP endpoints
/// and debug dumps.
#[test]
fn test_registry_list_registered_after_register() {
    hamiltonian_registry::clear();
    hamiltonian_registry::register("ham_a", stub_factory("KIND_A", "R"), None, 1).unwrap();
    hamiltonian_registry::register("ham_b", stub_factory("KIND_B", "SU2"), None, 2).unwrap();

    let mut listed = hamiltonian_registry::list_registered();
    listed.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].0, "ham_a");
    assert_eq!(listed[0].1, "KIND_A");
    assert_eq!(listed[0].2, "R");
    assert_eq!(listed[1].0, "ham_b");
    assert_eq!(listed[1].1, "KIND_B");
    assert_eq!(listed[1].2, "SU2");
}

// ─────────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────────

fn tempdir_for_test(tag: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!("gigi_aurora_p2_{tag}_{nanos}"));
    std::fs::create_dir_all(&p).expect("create temp dir");
    p
}
