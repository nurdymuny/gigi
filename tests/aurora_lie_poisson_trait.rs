//! AURORA Phase 3 — HamiltonianPoissonBracket trait surface (D1).
//!
//! Receipt: this workflow ships the `HamiltonianPoissonBracket` trait
//! + `BracketPhysicsError` enum that AURORA's bracket-step path
//! consumes. Stormer-Verlet KDK fails by construction on shallow
//! water (AURORA reply 3, 2026-06-22: 7x worse Casimir drift than
//! forward Euler); Lie-Poisson is the structure-preserving fix.
//!
//! BIT-IDENTITY: strictly additive — no existing trait is modified.
//!
//! Stability contract: every pub item carries the EVOLVING marker.

#![cfg(feature = "halcyon")]

use std::collections::{BTreeMap, HashMap};

use gigi::gauge::action::{
    BracketPhysicsError, EnergyDecomposition, EnergyError, FactoryError, HamiltonianCapabilities,
    HamiltonianDrift, HamiltonianFactory, HamiltonianForce, HamiltonianHandle,
    HamiltonianPoissonBracket, ProjectionError, ProjectionOperator,
};

// ─────────────────────────────────────────────────────────────────────
// StubHandle — minimal Hamiltonian that wears the full handle surface
// + implements HamiltonianPoissonBracket with a trivial state mutation.
// ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct StubHandle;

impl HamiltonianForce for StubHandle {
    fn force(&self, _state: &[f64]) -> Vec<f64> {
        Vec::new()
    }
}

impl HamiltonianDrift for StubHandle {
    fn drift(&self, state: &[f64], _dt: f64) -> Vec<f64> {
        state.to_vec()
    }
}

impl ProjectionOperator for StubHandle {
    fn project_constraint(&self, _state: &mut [f64]) -> Result<(), ProjectionError> {
        Ok(())
    }
}

impl EnergyDecomposition for StubHandle {
    fn energy_keys(&self) -> &'static [&'static str] {
        &["htotal"]
    }

    fn evaluate(&self, _state: &[f64]) -> Result<BTreeMap<String, f64>, EnergyError> {
        let mut out = BTreeMap::new();
        out.insert("htotal".to_string(), 0.0);
        Ok(out)
    }
}

impl HamiltonianHandle for StubHandle {}

impl HamiltonianPoissonBracket for StubHandle {
    fn bracket_step(&self, state: &mut [f64], dt: f64) -> Result<(), BracketPhysicsError> {
        if state.is_empty() {
            return Err(BracketPhysicsError::Other("empty state".to_string()));
        }
        state[0] += dt;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn test_hamiltonian_poisson_bracket_trait_can_be_implemented() {
    // Object-safe: must be boxable as dyn.
    let handle: Box<dyn HamiltonianPoissonBracket> = Box::new(StubHandle);
    let mut state = vec![1.0_f64, 2.0, 3.0];
    handle
        .bracket_step(&mut state, 0.5)
        .expect("stub bracket_step never errors on non-empty state");
    assert_eq!(state[0], 1.5, "bracket_step must mutate state in place");
    assert_eq!(state[1], 2.0, "untouched slots stay put");
}

#[test]
fn test_bracket_physics_error_constructs_three_variants() {
    let neg = BracketPhysicsError::NegativeDepth {
        i: 3,
        j: 7,
        h: -1.2e-3,
    };
    let cfl = BracketPhysicsError::CflViolation {
        courant: 1.4,
        max_courant: 1.0,
    };
    let other = BracketPhysicsError::Other("custom reason".to_string());

    // Pattern-match field round-trip.
    match neg {
        BracketPhysicsError::NegativeDepth { i, j, h } => {
            assert_eq!(i, 3);
            assert_eq!(j, 7);
            assert!(h < 0.0);
        }
        _ => panic!("expected NegativeDepth"),
    }
    match cfl {
        BracketPhysicsError::CflViolation {
            courant,
            max_courant,
        } => {
            assert!(courant > max_courant);
        }
        _ => panic!("expected CflViolation"),
    }
    match other {
        BracketPhysicsError::Other(ref reason) => assert_eq!(reason, "custom reason"),
        _ => panic!("expected Other"),
    }

    // Debug renders non-empty + carries discriminant name.
    let dbg_neg = format!("{:?}", BracketPhysicsError::NegativeDepth { i: 0, j: 0, h: 0.0 });
    assert!(dbg_neg.contains("NegativeDepth"));
    let dbg_cfl = format!(
        "{:?}",
        BracketPhysicsError::CflViolation {
            courant: 0.0,
            max_courant: 0.0,
        }
    );
    assert!(dbg_cfl.contains("CflViolation"));
    let dbg_other = format!("{:?}", BracketPhysicsError::Other("x".to_string()));
    assert!(dbg_other.contains("Other"));
}

#[test]
fn test_bracket_physics_error_implements_std_error_pattern() {
    // Coerces into Box<dyn std::error::Error>.
    let err = BracketPhysicsError::NegativeDepth {
        i: 1,
        j: 2,
        h: -0.5,
    };
    let boxed: Box<dyn std::error::Error> = Box::new(err);
    // source() returns None — same trivial pattern as the other
    // three error enums in src/gauge/action.rs.
    assert!(boxed.source().is_none());

    // Display matches the lowercase "subsystem: detail" convention.
    let neg_msg = format!(
        "{}",
        BracketPhysicsError::NegativeDepth {
            i: 3,
            j: 7,
            h: -1.2e-3
        }
    );
    assert!(neg_msg.starts_with("bracket physics: negative depth"));
    assert!(neg_msg.contains("(i, j)=(3, 7)"));

    let cfl_msg = format!(
        "{}",
        BracketPhysicsError::CflViolation {
            courant: 1.4,
            max_courant: 1.0,
        }
    );
    assert!(cfl_msg.starts_with("bracket physics: CFL violation"));
    assert!(cfl_msg.contains("courant=1.4"));
    assert!(cfl_msg.contains("max=1"));

    let other_msg = format!("{}", BracketPhysicsError::Other("custom".to_string()));
    assert_eq!(other_msg, "bracket physics: custom");

    // Send + Sync + Debug + Clone compile-time assertion.
    fn assert_traits<T: Send + Sync + std::fmt::Debug + Clone>() {}
    assert_traits::<BracketPhysicsError>();

    // PartialEq round-trip.
    let a = BracketPhysicsError::Other("same".to_string());
    let b = BracketPhysicsError::Other("same".to_string());
    assert_eq!(a, b);
    let c = BracketPhysicsError::Other("different".to_string());
    assert_ne!(a, c);
}

// ─────────────────────────────────────────────────────────────────────
// D2 — HamiltonianCapabilities + factory capabilities() default method.
// ─────────────────────────────────────────────────────────────────────

/// A pre-Phase-3 stub factory that inherits the default `capabilities()`
/// — same backwards-compat contract as KogutSusskindFactory et al.
#[derive(Debug)]
struct DefaultStubFactory;

impl HamiltonianFactory for DefaultStubFactory {
    fn kind_tag(&self) -> &'static str {
        "DEFAULT_STUB"
    }
    fn group_tag(&self) -> &'static str {
        "R"
    }
    fn from_params(
        &self,
        _params: &HashMap<String, f64>,
    ) -> Result<Box<dyn HamiltonianHandle>, FactoryError> {
        Ok(Box::new(StubHandle))
    }
    // Note: no override of capabilities() — pulls the trait default.
}

/// A Phase-3-aware stub factory modelling AURORA's `ShallowWaterFactory`
/// post-Phase-3: declares BOTH integration paths, so SYMPLECTIC_FLOW can
/// run comparative receipt tests (KDK Casimir drift vs bracket Casimir
/// drift) on the same handle while keeping A17/A18 KDK gates live.
#[derive(Debug)]
struct DualPathStubFactory;

impl HamiltonianFactory for DualPathStubFactory {
    fn kind_tag(&self) -> &'static str {
        "DUAL_PATH_STUB"
    }
    fn group_tag(&self) -> &'static str {
        "R"
    }
    fn from_params(
        &self,
        _params: &HashMap<String, f64>,
    ) -> Result<Box<dyn HamiltonianHandle>, FactoryError> {
        Ok(Box::new(StubHandle))
    }
    fn capabilities(&self) -> HamiltonianCapabilities {
        HamiltonianCapabilities {
            force_drift: true,
            poisson_bracket: true,
        }
    }
}

#[test]
fn test_capabilities_default_for_existing_factories() {
    // The default impl is the backwards-compat contract for every
    // pre-Phase-3 factory (KogutSusskindFactory etc.): force_drift only.
    let factory = DefaultStubFactory;
    let caps = factory.capabilities();
    assert_eq!(
        caps,
        HamiltonianCapabilities {
            force_drift: true,
            poisson_bracket: false,
        },
        "default capabilities must be {{ force_drift: true, poisson_bracket: false }} \
         so KogutSusskindFactory et al. inherit correct behavior without modification"
    );
    assert!(caps.force_drift);
    assert!(!caps.poisson_bracket);

    // Also exercise via Box<dyn HamiltonianFactory> to confirm
    // object-safety stays intact after the capabilities() addition.
    let boxed: Box<dyn HamiltonianFactory> = Box::new(DefaultStubFactory);
    assert_eq!(
        boxed.capabilities(),
        HamiltonianCapabilities {
            force_drift: true,
            poisson_bracket: false,
        }
    );

    // Copy + Debug derives — pattern-match and log without ceremony.
    let c = caps;
    let _c2 = c; // Copy
    assert_eq!(c, _c2);
    let dbg = format!("{c:?}");
    assert!(dbg.contains("HamiltonianCapabilities"));
    assert!(dbg.contains("force_drift"));
    assert!(dbg.contains("poisson_bracket"));
}

#[test]
fn test_capabilities_can_be_overridden_to_both_true() {
    // Models AURORA's ShallowWaterFactory contract post-Phase-3:
    // keeps force_drift: true (A17/A18 KDK gates stay live) and adds
    // poisson_bracket: true (opts into the non-separable path).
    let factory = DualPathStubFactory;
    let caps = factory.capabilities();
    assert_eq!(
        caps,
        HamiltonianCapabilities {
            force_drift: true,
            poisson_bracket: true,
        }
    );
    assert!(caps.force_drift, "A17/A18 KDK diagnostic gates require force_drift");
    assert!(caps.poisson_bracket, "non-separable physics requires bracket path");

    // Override survives dyn-dispatch.
    let boxed: Box<dyn HamiltonianFactory> = Box::new(DualPathStubFactory);
    let caps_dyn = boxed.capabilities();
    assert!(caps_dyn.force_drift);
    assert!(caps_dyn.poisson_bracket);
}

// ─────────────────────────────────────────────────────────────────────
// D3 — SYMPLECTIC_FLOW capability dispatch.
//
// The dispatch shim is `symplectic_flow_dispatch` in src/gauge/
// symplectic_flow.rs. The existing `symplectic_flow(u_name, e_name,
// ...)` SU(2) Kogut-Susskind path is unchanged (IV.10 + VI.5 gold-
// gate byte-identical kill criterion).
// ─────────────────────────────────────────────────────────────────────

use std::sync::Mutex;

use gigi::gauge::error::GaugeFieldError;
use gigi::gauge::hamiltonian_registry as ham_registry;
use gigi::gauge::symplectic_flow::{
    pick_integrator_path, symplectic_flow_dispatch, IntegratorPath,
};

// ── Global call-order trace shared between handle methods ──
//
// Test stubs push string events into TRACE as they fire, so the order-
// of-operations tests can assert that bracket_step / kick / drift /
// project_constraint ran in the expected sequence. Tests reset the
// trace at entry and serialize via a per-test serial guard.

static TRACE: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn trace_push(s: &str) {
    TRACE.lock().unwrap().push(s.to_string());
}

fn trace_reset() {
    TRACE.lock().unwrap().clear();
}

fn trace_snapshot() -> Vec<String> {
    TRACE.lock().unwrap().clone()
}

// Serial guard — all dispatch tests share the global hamiltonian
// registry + TRACE; running them in parallel would interleave events.
// The harness already runs `-- --test-threads=1` per the workflow but
// we belt-and-brace with a Mutex anyway.
static SERIAL: Mutex<()> = Mutex::new(());

// ── KdkOnlyHandle — tracing variant of StubHandle, force_drift only ──

#[derive(Debug)]
struct KdkOnlyHandle;

impl HamiltonianForce for KdkOnlyHandle {
    fn force(&self, state: &[f64]) -> Vec<f64> {
        trace_push("force");
        vec![0.0; state.len()]
    }
}

impl HamiltonianDrift for KdkOnlyHandle {
    fn drift(&self, state: &[f64], _dt: f64) -> Vec<f64> {
        trace_push("drift");
        state.to_vec()
    }
}

impl ProjectionOperator for KdkOnlyHandle {
    fn project_constraint(&self, _state: &mut [f64]) -> Result<(), ProjectionError> {
        trace_push("project");
        Ok(())
    }
}

impl EnergyDecomposition for KdkOnlyHandle {
    fn energy_keys(&self) -> &'static [&'static str] {
        &["htotal"]
    }
    fn evaluate(&self, _state: &[f64]) -> Result<BTreeMap<String, f64>, EnergyError> {
        let mut out = BTreeMap::new();
        // Constant — receipt test asserts drift == 0.0 for the no-op
        // integrator path on both KDK and bracket sides.
        out.insert("htotal".to_string(), 42.0);
        Ok(out)
    }
}

impl HamiltonianHandle for KdkOnlyHandle {}

#[derive(Debug)]
struct KdkOnlyFactory;

impl HamiltonianFactory for KdkOnlyFactory {
    fn kind_tag(&self) -> &'static str {
        "KOGUT_SUSSKIND_STUB"
    }
    fn group_tag(&self) -> &'static str {
        "SU2"
    }
    fn from_params(
        &self,
        _params: &HashMap<String, f64>,
    ) -> Result<Box<dyn HamiltonianHandle>, FactoryError> {
        Ok(Box::new(KdkOnlyHandle))
    }
    // Default capabilities() — { force_drift: true, poisson_bracket: false }.
}

// ── BracketOnlyHandle — tracing handle that implements both KDK +
//    bracket but its factory advertises only poisson_bracket. ──

#[derive(Debug)]
struct BracketHandle;

impl HamiltonianForce for BracketHandle {
    fn force(&self, state: &[f64]) -> Vec<f64> {
        trace_push("force");
        vec![0.0; state.len()]
    }
}

impl HamiltonianDrift for BracketHandle {
    fn drift(&self, state: &[f64], _dt: f64) -> Vec<f64> {
        trace_push("drift");
        state.to_vec()
    }
}

impl ProjectionOperator for BracketHandle {
    fn project_constraint(&self, _state: &mut [f64]) -> Result<(), ProjectionError> {
        trace_push("project");
        Ok(())
    }
}

impl EnergyDecomposition for BracketHandle {
    fn energy_keys(&self) -> &'static [&'static str] {
        &["htotal"]
    }
    fn evaluate(&self, _state: &[f64]) -> Result<BTreeMap<String, f64>, EnergyError> {
        let mut out = BTreeMap::new();
        out.insert("htotal".to_string(), 7.0);
        Ok(out)
    }
}

impl HamiltonianHandle for BracketHandle {
    fn as_poisson_bracket(&self) -> Option<&dyn HamiltonianPoissonBracket> {
        Some(self)
    }
}

impl HamiltonianPoissonBracket for BracketHandle {
    fn bracket_step(&self, _state: &mut [f64], _dt: f64) -> Result<(), BracketPhysicsError> {
        trace_push("bracket_step");
        Ok(())
    }
}

#[derive(Debug)]
struct BracketOnlyFactory;

impl HamiltonianFactory for BracketOnlyFactory {
    fn kind_tag(&self) -> &'static str {
        "SHALLOW_WATER_STUB"
    }
    fn group_tag(&self) -> &'static str {
        "R"
    }
    fn from_params(
        &self,
        _params: &HashMap<String, f64>,
    ) -> Result<Box<dyn HamiltonianHandle>, FactoryError> {
        Ok(Box::new(BracketHandle))
    }
    fn capabilities(&self) -> HamiltonianCapabilities {
        HamiltonianCapabilities {
            force_drift: true,
            poisson_bracket: true,
        }
    }
}

// ── NoPathFactory — declares no integration path, error case. ──

#[derive(Debug)]
struct NoPathFactory;

impl HamiltonianFactory for NoPathFactory {
    fn kind_tag(&self) -> &'static str {
        "NO_PATH_STUB"
    }
    fn group_tag(&self) -> &'static str {
        "R"
    }
    fn from_params(
        &self,
        _params: &HashMap<String, f64>,
    ) -> Result<Box<dyn HamiltonianHandle>, FactoryError> {
        Ok(Box::new(KdkOnlyHandle))
    }
    fn capabilities(&self) -> HamiltonianCapabilities {
        HamiltonianCapabilities {
            force_drift: false,
            poisson_bracket: false,
        }
    }
}

// ── Tests ──

#[test]
fn test_symplectic_flow_dispatches_to_kdk_for_kogut_susskind_factory() {
    let _g = SERIAL.lock().unwrap();
    ham_registry::clear();
    trace_reset();

    let factory_name = "kogut_susskind_stub_dispatch";
    ham_registry::register(factory_name, Box::new(KdkOnlyFactory), None, 0)
        .expect("register factory");

    let params = HashMap::new();
    let mut state = vec![1.0, 2.0, 3.0];
    let diag = symplectic_flow_dispatch(
        factory_name,
        &params,
        &mut state,
        1, /* n_steps */
        0.1, /* dt */
        false, /* project */
        "handle_a",
        None,
    )
    .expect("dispatch succeeds for KDK factory");

    assert_eq!(diag.path, IntegratorPath::StormerVerletKdk);
    assert_eq!(diag.path.wal_tag(), "stormer_verlet_kdk");
    assert_eq!(diag.factory_name, factory_name);
    assert_eq!(diag.handle_name, "handle_a");
    assert_eq!(diag.n_steps_completed, 1);

    let tr = trace_snapshot();
    assert!(
        tr.contains(&"force".to_string()) && tr.contains(&"drift".to_string()),
        "KDK path must call force + drift, got: {tr:?}"
    );
    assert!(
        !tr.contains(&"bracket_step".to_string()),
        "KDK path must not call bracket_step, got: {tr:?}"
    );

    ham_registry::clear();
}

#[test]
fn test_symplectic_flow_dispatches_to_bracket_step_for_stub_bracket_factory() {
    let _g = SERIAL.lock().unwrap();
    ham_registry::clear();
    trace_reset();

    let factory_name = "shallow_water_stub_dispatch";
    ham_registry::register(factory_name, Box::new(BracketOnlyFactory), None, 0)
        .expect("register factory");

    let params = HashMap::new();
    let mut state = vec![1.0, 2.0, 3.0];
    let diag = symplectic_flow_dispatch(
        factory_name,
        &params,
        &mut state,
        3, /* n_steps */
        0.05,
        false,
        "handle_b",
        None,
    )
    .expect("dispatch succeeds for bracket factory");

    assert_eq!(diag.path, IntegratorPath::LiePoissonBracket);
    assert_eq!(diag.path.wal_tag(), "bracket_step");
    assert_eq!(diag.n_steps_completed, 3);

    let tr = trace_snapshot();
    let bracket_count = tr.iter().filter(|e| e.as_str() == "bracket_step").count();
    assert_eq!(
        bracket_count, 3,
        "bracket_step must fire once per step, got: {tr:?}"
    );
    assert!(
        !tr.iter().any(|e| e.as_str() == "force" || e.as_str() == "drift"),
        "bracket path must not call force/drift, got: {tr:?}"
    );

    ham_registry::clear();
}

#[test]
fn test_symplectic_flow_projection_runs_after_bracket_step() {
    let _g = SERIAL.lock().unwrap();
    ham_registry::clear();
    trace_reset();

    let factory_name = "bracket_projection_dispatch";
    ham_registry::register(factory_name, Box::new(BracketOnlyFactory), None, 0)
        .expect("register factory");

    let params = HashMap::new();
    let mut state = vec![1.0, 2.0];
    symplectic_flow_dispatch(
        factory_name,
        &params,
        &mut state,
        2,
        0.1,
        true, /* project = true */
        "handle_c",
        None,
    )
    .expect("dispatch succeeds");

    let tr = trace_snapshot();
    // Filter out evaluate calls (energy receipt is sampled per step but
    // not via the trace) — the trace only carries force/drift/bracket/
    // project events. Expected order across 2 steps:
    //   bracket_step, project, bracket_step, project
    let relevant: Vec<&str> = tr
        .iter()
        .filter(|e| matches!(e.as_str(), "bracket_step" | "project" | "force" | "drift"))
        .map(|s| s.as_str())
        .collect();
    assert_eq!(
        relevant,
        vec!["bracket_step", "project", "bracket_step", "project"],
        "bracket -> project per step, got: {tr:?}"
    );

    ham_registry::clear();
}

#[test]
fn test_symplectic_flow_projection_runs_after_kdk() {
    let _g = SERIAL.lock().unwrap();
    ham_registry::clear();
    trace_reset();

    let factory_name = "kdk_projection_dispatch";
    ham_registry::register(factory_name, Box::new(KdkOnlyFactory), None, 0)
        .expect("register factory");

    let params = HashMap::new();
    let mut state = vec![1.0, 2.0];
    symplectic_flow_dispatch(
        factory_name,
        &params,
        &mut state,
        2,
        0.1,
        true,
        "handle_d",
        None,
    )
    .expect("dispatch succeeds");

    let tr = trace_snapshot();
    let relevant: Vec<&str> = tr
        .iter()
        .filter(|e| matches!(e.as_str(), "bracket_step" | "project" | "force" | "drift"))
        .map(|s| s.as_str())
        .collect();
    // KDK ordering per step: force (kick0), drift, force (kick1), project.
    // Two steps -> 8 events total.
    assert_eq!(
        relevant,
        vec![
            "force", "drift", "force", "project", "force", "drift", "force", "project"
        ],
        "KDK -> project per step, got: {tr:?}"
    );

    ham_registry::clear();
}

#[test]
fn test_symplectic_flow_receipt_check_holds_for_both_paths() {
    // Both stubs publish constant htotal -> drift == 0.0 from either
    // path. This is the receipt-contract regression: the substrate's
    // post-flow drift envelope is path-agnostic.
    let _g = SERIAL.lock().unwrap();
    ham_registry::clear();
    trace_reset();

    ham_registry::register("kdk_receipt", Box::new(KdkOnlyFactory), None, 0).unwrap();
    let mut s1 = vec![1.0, 2.0];
    let d1 = symplectic_flow_dispatch(
        "kdk_receipt",
        &HashMap::new(),
        &mut s1,
        4,
        0.1,
        false,
        "h",
        None,
    )
    .unwrap();
    assert_eq!(d1.max_energy_drift_rel, 0.0, "KDK constant H -> zero drift");
    assert_eq!(d1.path, IntegratorPath::StormerVerletKdk);

    ham_registry::register("bracket_receipt", Box::new(BracketOnlyFactory), None, 0).unwrap();
    let mut s2 = vec![1.0, 2.0];
    let d2 = symplectic_flow_dispatch(
        "bracket_receipt",
        &HashMap::new(),
        &mut s2,
        4,
        0.1,
        false,
        "h",
        None,
    )
    .unwrap();
    assert_eq!(d2.max_energy_drift_rel, 0.0, "bracket constant H -> zero drift");
    assert_eq!(d2.path, IntegratorPath::LiePoissonBracket);

    ham_registry::clear();
}

#[test]
fn test_no_integration_path_clear_error() {
    let _g = SERIAL.lock().unwrap();
    ham_registry::clear();
    trace_reset();

    let factory_name = "no_path_dispatch";
    ham_registry::register(factory_name, Box::new(NoPathFactory), None, 0).unwrap();

    let mut state = vec![1.0_f64];
    let err = symplectic_flow_dispatch(
        factory_name,
        &HashMap::new(),
        &mut state,
        1,
        0.1,
        false,
        "h",
        None,
    )
    .expect_err("must error with NoIntegrationPath");

    match err {
        GaugeFieldError::NoIntegrationPath {
            factory,
            force_drift,
            poisson_bracket,
        } => {
            assert_eq!(factory, factory_name);
            assert!(!force_drift);
            assert!(!poisson_bracket);
        }
        other => panic!("expected NoIntegrationPath, got {other:?}"),
    }

    // No call to force/drift/bracket/project happened — the dispatcher
    // refuses before doing any work.
    let tr = trace_snapshot();
    assert!(
        tr.iter().all(|e| !matches!(
            e.as_str(),
            "force" | "drift" | "bracket_step" | "project"
        )),
        "no integrator step must run when path is refused, got: {tr:?}"
    );

    ham_registry::clear();
}

#[test]
fn test_pick_integrator_path_capability_lies_fall_back_to_kdk() {
    // A factory that LIES — declares poisson_bracket=true, but the
    // handle's as_poisson_bracket() returns None. The dispatcher must
    // fall back to KDK rather than panic.
    #[derive(Debug)]
    struct LyingFactory;
    impl HamiltonianFactory for LyingFactory {
        fn kind_tag(&self) -> &'static str {
            "LYING_STUB"
        }
        fn group_tag(&self) -> &'static str {
            "R"
        }
        fn from_params(
            &self,
            _: &HashMap<String, f64>,
        ) -> Result<Box<dyn HamiltonianHandle>, FactoryError> {
            Ok(Box::new(KdkOnlyHandle))
        }
        fn capabilities(&self) -> HamiltonianCapabilities {
            // LIE: declares bracket path but handle doesn't impl it.
            HamiltonianCapabilities {
                force_drift: true,
                poisson_bracket: true,
            }
        }
    }

    let caps = LyingFactory.capabilities();
    let handle = KdkOnlyHandle;
    let path = pick_integrator_path("lying", caps, &handle).expect("falls back, no error");
    assert_eq!(
        path,
        IntegratorPath::StormerVerletKdk,
        "lying factory must fall back to KDK rather than panic"
    );
}
