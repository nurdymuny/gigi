//! AURORA Phase 2 — Hamiltonian trait surface (RED-first tests).
//!
//! Receipt: this workflow ships the four pub traits + HamiltonianFactory
//! that AURORA's ShallowWaterFactory is already written against
//! (kind_tag/group_tag/from_params, EnergyDecomposition with 7 keys).
//!
//! BIT-IDENTITY: these tests are strictly additive — no existing module
//! is modified. The new trait surface lives in `src/gauge/action.rs`
//! and must be group-agnostic (AURORA's group_tag="R" is the canary).
//!
//! Stability contract: every pub item in `src/gauge/action.rs` carries
//! the EVOLVING marker (docs/STABILITY_GUARANTEES.md) so AURORA can
//! pin gigi by commit hash and get a stable contract surface.

#![cfg(feature = "halcyon")]

use std::collections::{BTreeMap, HashMap};

use gigi::gauge::action::{
    EnergyDecomposition, EnergyError, FactoryError, HamiltonianDrift, HamiltonianFactory,
    HamiltonianForce, HamiltonianHandle, ProjectionError, ProjectionOperator,
};

// ─────────────────────────────────────────────────────────────────────
// NoOpHamiltonian — minimal stub that implements the full surface.
// Used to prove the trait hierarchy compiles in isolation.
// ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct NoOpHamiltonian;

impl HamiltonianForce for NoOpHamiltonian {
    fn force(&self, _state: &[f64]) -> Vec<f64> {
        Vec::new()
    }
}

impl HamiltonianDrift for NoOpHamiltonian {
    fn drift(&self, state: &[f64], _dt: f64) -> Vec<f64> {
        state.to_vec()
    }
}

impl ProjectionOperator for NoOpHamiltonian {
    fn project_constraint(&self, _state: &mut [f64]) -> Result<(), ProjectionError> {
        Ok(())
    }
}

impl EnergyDecomposition for NoOpHamiltonian {
    fn energy_keys(&self) -> &'static [&'static str] {
        &[]
    }
    fn evaluate(&self, _state: &[f64]) -> Result<BTreeMap<String, f64>, EnergyError> {
        Ok(BTreeMap::new())
    }
}

impl HamiltonianHandle for NoOpHamiltonian {}

struct NoOpHamiltonianFactory;

impl HamiltonianFactory for NoOpHamiltonianFactory {
    fn kind_tag(&self) -> &'static str {
        "NO_OP"
    }
    fn group_tag(&self) -> &'static str {
        "NONE"
    }
    fn from_params(
        &self,
        _params: &HashMap<String, f64>,
    ) -> Result<Box<dyn HamiltonianHandle>, FactoryError> {
        Ok(Box::new(NoOpHamiltonian))
    }
}

// ─────────────────────────────────────────────────────────────────────
// MockShallowWaterFactory — mirrors AURORA's exact shape (kind_tag=
// "SHALLOW_WATER", group_tag="R", params {g, omega, a}) so signature
// alignment is verified without depending on the aurora crate.
// ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct ShallowWaterMock {
    keys: &'static [&'static str],
}

impl HamiltonianForce for ShallowWaterMock {
    fn force(&self, _state: &[f64]) -> Vec<f64> {
        Vec::new()
    }
}

impl HamiltonianDrift for ShallowWaterMock {
    fn drift(&self, state: &[f64], _dt: f64) -> Vec<f64> {
        state.to_vec()
    }
}

impl ProjectionOperator for ShallowWaterMock {
    fn project_constraint(&self, _state: &mut [f64]) -> Result<(), ProjectionError> {
        Ok(())
    }
}

impl EnergyDecomposition for ShallowWaterMock {
    fn energy_keys(&self) -> &'static [&'static str] {
        self.keys
    }
    fn evaluate(&self, _state: &[f64]) -> Result<BTreeMap<String, f64>, EnergyError> {
        let mut m = BTreeMap::new();
        for k in self.keys {
            m.insert((*k).to_string(), 0.0);
        }
        Ok(m)
    }
}

impl HamiltonianHandle for ShallowWaterMock {}

struct MockShallowWaterFactory;

impl HamiltonianFactory for MockShallowWaterFactory {
    fn kind_tag(&self) -> &'static str {
        "SHALLOW_WATER"
    }
    fn group_tag(&self) -> &'static str {
        "R"
    }
    fn from_params(
        &self,
        params: &HashMap<String, f64>,
    ) -> Result<Box<dyn HamiltonianHandle>, FactoryError> {
        if !params.contains_key("g") {
            return Err(FactoryError::MissingParam { name: "g" });
        }
        Ok(Box::new(ShallowWaterMock {
            keys: &[
                "casimir_energy",
                "casimir_mass",
                "casimir_pv_l1",
                "casimir_pv_l2",
                "kelvin_eq",
                "kelvin_n30",
                "kelvin_s30",
            ],
        }))
    }
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

/// Compile-time check: `Box<dyn HamiltonianFactory>` is constructible.
/// Proves the trait is object-safe (no associated types leak through).
#[test]
fn test_hamiltonian_factory_trait_shape() {
    let factory: Box<dyn HamiltonianFactory> = Box::new(NoOpHamiltonianFactory);
    assert_eq!(factory.kind_tag(), "NO_OP");
    assert_eq!(factory.group_tag(), "NONE");
}

/// HamiltonianHandle requires all four sub-traits. The bound check
/// happens at compile time — if any sub-trait is missing from the
/// super-trait, this fn fails to type-check.
#[test]
fn test_hamiltonian_handle_subtrait_bounds() {
    fn assert_force<T: HamiltonianForce>() {}
    fn assert_drift<T: HamiltonianDrift>() {}
    fn assert_projection<T: ProjectionOperator>() {}
    fn assert_energy<T: EnergyDecomposition>() {}
    fn assert_handle<T: HamiltonianHandle>() {}

    assert_force::<NoOpHamiltonian>();
    assert_drift::<NoOpHamiltonian>();
    assert_projection::<NoOpHamiltonian>();
    assert_energy::<NoOpHamiltonian>();
    assert_handle::<NoOpHamiltonian>();
}

/// EnergyDecomposition::energy_keys returns a stable slice. The
/// AURORA ShallowWater contract is 7 keys in a fixed order; this
/// test pins that contract at the trait surface level.
#[test]
fn test_energy_decomposition_keys_iteration() {
    let factory = MockShallowWaterFactory;
    let mut params = HashMap::new();
    params.insert("g".to_string(), 9.81);
    let handle = factory.from_params(&params).expect("factory must build");
    let keys = handle.energy_keys();
    assert_eq!(keys.len(), 7, "ShallowWater contract: 7 energy keys");
    assert_eq!(keys[0], "casimir_energy");
    assert_eq!(keys[6], "kelvin_s30");

    // BTreeMap evaluation is deterministic — same keys, sorted.
    let map = handle.evaluate(&[]).expect("evaluate must succeed");
    let names: Vec<&String> = map.keys().collect();
    assert_eq!(names.len(), 7);
}

/// NoOpHamiltonian implements all four sub-traits and HamiltonianHandle.
/// Round-trip through `Box<dyn HamiltonianHandle>` proves the super-trait
/// itself is object-safe.
#[test]
fn test_stub_hamiltonian_compiles_against_traits() {
    let h: Box<dyn HamiltonianHandle> = Box::new(NoOpHamiltonian);
    assert_eq!(h.force(&[]).len(), 0);
    assert_eq!(h.drift(&[1.0, 2.0], 0.1), vec![1.0, 2.0]);
    let mut s = [0.0_f64; 4];
    h.project_constraint(&mut s).expect("noop must not diverge");
    assert!(h.energy_keys().is_empty());
}

/// AURORA's exact factory shape — kind_tag "SHALLOW_WATER", group_tag
/// "R", from_params taking &HashMap<String, f64> — must compile through
/// the trait surface. If this test compiles, AURORA's ShallowWaterFactory
/// will compile against this gigi commit.
#[test]
fn test_aurora_shallow_water_factory_signature_alignment() {
    let factory: Box<dyn HamiltonianFactory> = Box::new(MockShallowWaterFactory);
    assert_eq!(factory.kind_tag(), "SHALLOW_WATER");
    assert_eq!(factory.group_tag(), "R");

    let mut params = HashMap::new();
    params.insert("g".to_string(), 9.81);
    params.insert("omega".to_string(), 7.2921e-5);
    params.insert("a".to_string(), 6.371e6);
    let handle = factory.from_params(&params).expect("must build");
    assert_eq!(handle.energy_keys().len(), 7);
}

/// Missing required params surfaces as a typed FactoryError — never a
/// panic, never a silent default. No tunable tolerances.
#[test]
fn test_factory_missing_param_returns_typed_error() {
    let factory = MockShallowWaterFactory;
    let params = HashMap::new();
    let err = factory.from_params(&params).expect_err("must reject");
    match err {
        FactoryError::MissingParam { name } => assert_eq!(name, "g"),
        other => panic!("expected MissingParam, got {other:?}"),
    }
}
