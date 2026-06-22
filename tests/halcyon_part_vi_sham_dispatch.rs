//! TDD-HAL-VI.4 — RED — LOOP_TRANSPORT SHAM dispatch contract.
//!
//! Pre-registration: HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3 §5
//! (Zenodo DOI 10.5281/zenodo.20785681). Gate doc:
//! `theory/halcyon/HALCYON_PART_VI_GATES.md` @ 9a73dc0.
//!
//! VI.4 scope (this file):
//!   - The 5 science SHAM flags (FLAT_FIELD, ALPHA_ZERO,
//!     MASS_BASELINE_SCALED, DEGENERATE_LOOP, FROZEN_FIELD) — each must
//!     drive the observable holonomy to zero per its v3.1.3 §5 gate.
//!   - The 1 runtime audit-story flag (EMPTY_LOOP) — substrate-side
//!     companion to GC₄: verb returns H = 0 byte-for-byte.
//!   - The zero-cost-when-off contract: no SHAM clause vs. explicit
//!     empty `SHAM { }` block produce byte-for-byte identical
//!     diagnostics. This is the structural enforcement of the
//!     IV.10 + VI.3 inheritance contract.
//!   - The unknown-flag regression: VI.2's rejection of unknown flags
//!     (e.g. `not_a_real_vi4_flag`) MUST keep firing alongside the
//!     6 newly recognized flag names.
//!
//! Note: OPEN_LOOP is enforced at the VI.2 parser entry (returns
//! `LoopTransportError::LoopNotClosed`), not in this dispatch surface.
//!
//! Sigma strategy per design.sigma_strategy_for_tests:
//!   Tier 1 (default `cargo test`): canonical 8 seeds
//!     {20260616..20260623} per v3.1.3 §4.4. Tests gate against the
//!     load-bearing 10⁻¹⁰ machine-ε floor AND the `|H| < 2σ`
//!     statistical sanity check. The 2σ tolerance allows for σ = 0
//!     when the sham truly zeros every per-seed run.
//!
//! Pattern mirrors `tests/halcyon_part_vi_gc_acceptance.rs` for the
//! environment setup (clear registries, register lattice, build U +
//! E, register loop, build statement, run verb).

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::loop_transport::{
    clear_loops, loop_transport, register_loop, LoopTransportDiagnostics, LoopTransportError,
    RegisteredLoop,
};
use gigi::gauge::{GaugeFieldInit, SU2EField, SU2GaugeField};
use gigi::parser::{
    ControlManifoldSpec, LoopTransportOutputId, LoopTransportReturnId, SeedRange, ShamArg,
    ShamBlock, Statement,
};

// ── Test helpers (mirror VI.3 GC battery scaffolding) ─────────────

mod helpers {
    use super::*;
    use gigi::gauge::registry::{register_su2, register_su2_e};
    use gigi::gauge::EFieldInit;
    use gigi::lattice::registry as lattice_registry;

    pub struct Env {
        pub engine: Engine,
        pub _dir: tempfile::TempDir,
    }

    /// Wipe loop + gauge + lattice registries so a previous test's
    /// state never bleeds into the next.
    pub fn cleanup() {
        clear_loops();
        gigi::gauge::registry::clear();
        gigi::lattice::registry::clear();
    }

    /// Open a fresh engine + tempdir.
    pub fn fresh_env() -> Env {
        let dir = tempfile::tempdir().expect("tempdir");
        let engine = Engine::open(dir.path()).expect("engine open");
        Env { engine, _dir: dir }
    }

    /// Register the canonical halcyon buckyball under name `"bb"`.
    pub fn register_canonical_buckyball(env: &mut Env) {
        let src = "LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
        let stmt = gigi::parser::parse(src).expect("parse buckyball");
        gigi::parser::execute(&mut env.engine, &stmt).expect("exec buckyball");
    }

    /// Build an SU(2) field with `INIT IDENTITY` on the named lattice
    /// and register it under `u_name`. Also creates the companion
    /// E field under `e_name` initialised to zero.
    pub fn build_identity_field(lattice_name: &str, u_name: &str, e_name: &str) {
        let lat = lattice_registry::get(lattice_name).expect("lattice declared");
        let u = SU2GaugeField::new(
            u_name.into(),
            &lat,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        let e = SU2EField::new(e_name.into(), &u, EFieldInit::Zero, None)
            .expect("zero e init");
        register_su2(u);
        register_su2_e(std::sync::Arc::new(std::sync::Mutex::new(e)));
    }

    /// Register a face-bounded closed loop.
    pub fn register_face_loop(loop_id: &str, lattice_name: &str, face_idx: usize) {
        let lat = lattice_registry::get(lattice_name).expect("lattice declared");
        let face = lat.faces[face_idx].clone();
        let mut vertices = face.clone();
        vertices.push(face[0]); // close
        let mut edges = Vec::with_capacity(face.len());
        for i in 0..face.len() {
            let a = face[i];
            let b = face[(i + 1) % face.len()];
            edges.push(lat.resolve_edge(a, b).expect("face edge resolves"));
        }
        register_loop(
            loop_id,
            RegisteredLoop {
                lattice_name: lattice_name.into(),
                vertices,
                edges,
            },
        );
    }

    /// Build a `Statement::LoopTransport` programmatically with an
    /// optional SHAM block. The non-sham defaults mirror
    /// VI.3's helpers::build_lt_stmt.
    #[allow(clippy::too_many_arguments)]
    pub fn build_lt_stmt_with_sham(
        lattice: &str,
        loop_id: &str,
        n_disc: usize,
        seed_lo: u64,
        seed_hi: u64,
        alpha_halcyon: f64,
        ramp_rate_beta_w: f64,
        sham: Option<ShamBlock>,
    ) -> Statement {
        Statement::LoopTransport {
            lattice: lattice.into(),
            loop_id: loop_id.into(),
            control_manifold: ControlManifoldSpec::QBetaWilson,
            adiabatic: true,
            ramp_rate_q: 0.04,
            ramp_rate_beta_w,
            drive_omega: 1.0,
            drive_f0: 0.01,
            n_discretization: n_disc,
            pin_lambda_q: 1.0,
            pin_lambda_beta_w: 1.0,
            eps_q: 0.05,
            eps_beta_w: 0.05,
            alpha_halcyon,
            tau_0: 1.0,
            beta_tau: 2.0,
            mu_baseline: 1.0,
            k_spring: 1.0,
            c_damp: 0.1,
            seeds: SeedRange { lo: seed_lo, hi: seed_hi },
            compute: vec![
                LoopTransportOutputId::HolonomyForward,
                LoopTransportOutputId::HolonomyReversed,
                LoopTransportOutputId::TrackingErrorTraceQ,
                LoopTransportOutputId::TrackingErrorTraceBetaW,
                LoopTransportOutputId::AdiabaticityCheck,
            ],
            return_fields: vec![
                LoopTransportReturnId::HForward,
                LoopTransportReturnId::HReversed,
                LoopTransportReturnId::SigmaHBlocked,
                LoopTransportReturnId::PerSeedHForward,
                LoopTransportReturnId::PerSeedHReversed,
                LoopTransportReturnId::TrackingErrorMaxQ,
                LoopTransportReturnId::TrackingErrorMaxBetaW,
                LoopTransportReturnId::AdiabaticityCheck,
            ],
            sham,
        }
    }

    /// Canonical-8 seeds per v3.1.3 §4.4: {20260616..20260623}.
    pub const SEED_LO: u64 = 20_260_616;
    pub const SEED_HI: u64 = 20_260_623;

    /// Small N for fast tests. The science sham gates check
    /// `|H| < 1e-10` which is a structural property of the dispatch,
    /// not a convergence property of N.
    pub const N_DISC: usize = 200;

    /// The 10⁻¹⁰ machine-ε floor per v3.1.3 §5 gate column.
    pub const MACHINE_EPS_GATE: f64 = 1.0e-10;

    /// Setup the standard env for VI.4 SHAM tests: fresh engine,
    /// buckyball lattice, identity U + zero E, face-0 closed loop.
    pub fn setup_canonical() -> Env {
        cleanup();
        let mut env = fresh_env();
        register_canonical_buckyball(&mut env);
        build_identity_field("bb", "U_sh", "E_sh");
        register_face_loop("face0", "bb", 0);
        env
    }

    /// Build a `ShamBlock` from a Vec of (name, ShamArg) pairs.
    pub fn sham(flags: Vec<(&str, ShamArg)>) -> ShamBlock {
        ShamBlock {
            flags: flags.into_iter().map(|(n, a)| (n.to_string(), a)).collect(),
        }
    }

    /// Bare-flag shorthand: `(name, ShamArg::Bool(true))`.
    pub fn bare(name: &str) -> (&str, ShamArg) {
        (name, ShamArg::Bool(true))
    }
}

// ── Bit-identity guard — THE MOST LOAD-BEARING TEST ───────────────

/// VI.4 — zero-cost-when-off contract: explicit empty `SHAM { }` block
/// vs. no SHAM clause must produce byte-for-byte identical
/// diagnostics. Every f64 in the diagnostics envelope is compared via
/// `to_bits()` so any LLVM reordering of the inner loop perturbing
/// the IV.10 + VI.3 inheritance trips this test immediately.
///
/// This is the structural test that protects the IV.10 gold fixture
/// (must stay 4/0 + 1 ignored) and the VI.3 GC battery (6 tests must
/// stay green) regardless of how VI.4 wires the dispatch internally.
#[test]
fn halcyon_vi_4_sham_empty_is_byte_identical_to_no_sham() {
    // Run 1: no SHAM clause (None).
    let mut env = helpers::setup_canonical();
    let stmt_no_sham = helpers::build_lt_stmt_with_sham(
        "bb", "face0", helpers::N_DISC,
        helpers::SEED_LO, helpers::SEED_HI,
        1.0, 0.01, /* sham = */ None,
    );
    let diag_no_sham: LoopTransportDiagnostics =
        loop_transport(&stmt_no_sham, "U_sh", "E_sh").expect("no-sham run");
    let _ = env; // hold tempdir for the duration

    // Run 2: explicit empty `SHAM { }` block (Some(ShamBlock{flags:[]})).
    // Setup a FRESH env so U + E start from the identical initial state
    // (cleanup + build_identity_field rebuilds the identity field).
    let mut env2 = helpers::setup_canonical();
    let stmt_empty_sham = helpers::build_lt_stmt_with_sham(
        "bb", "face0", helpers::N_DISC,
        helpers::SEED_LO, helpers::SEED_HI,
        1.0, 0.01,
        /* sham = */ Some(ShamBlock { flags: vec![] }),
    );
    let diag_empty_sham: LoopTransportDiagnostics =
        loop_transport(&stmt_empty_sham, "U_sh", "E_sh").expect("empty-sham run");
    let _ = env2;

    // Byte-for-byte equality on EVERY f64 in the diagnostics envelope.
    assert_eq!(
        diag_no_sham.h_forward.to_bits(),
        diag_empty_sham.h_forward.to_bits(),
        "h_forward bit-identity (no-sham={} empty-sham={})",
        diag_no_sham.h_forward, diag_empty_sham.h_forward,
    );
    assert_eq!(
        diag_no_sham.h_reversed.to_bits(),
        diag_empty_sham.h_reversed.to_bits(),
        "h_reversed bit-identity"
    );
    assert_eq!(
        diag_no_sham.sigma_h_blocked.to_bits(),
        diag_empty_sham.sigma_h_blocked.to_bits(),
        "sigma_h_blocked bit-identity"
    );
    assert_eq!(
        diag_no_sham.per_seed_h_forward.len(),
        diag_empty_sham.per_seed_h_forward.len(),
        "per_seed_h_forward length"
    );
    for (i, (a, b)) in diag_no_sham
        .per_seed_h_forward
        .iter()
        .zip(diag_empty_sham.per_seed_h_forward.iter())
        .enumerate()
    {
        assert_eq!(
            a.to_bits(), b.to_bits(),
            "per_seed_h_forward[{i}] bit-identity (no-sham={a} empty-sham={b})"
        );
    }
    assert_eq!(
        diag_no_sham.per_seed_h_reversed.len(),
        diag_empty_sham.per_seed_h_reversed.len(),
        "per_seed_h_reversed length"
    );
    for (i, (a, b)) in diag_no_sham
        .per_seed_h_reversed
        .iter()
        .zip(diag_empty_sham.per_seed_h_reversed.iter())
        .enumerate()
    {
        assert_eq!(
            a.to_bits(), b.to_bits(),
            "per_seed_h_reversed[{i}] bit-identity"
        );
    }
    assert_eq!(
        diag_no_sham.tracking_error_max_q.to_bits(),
        diag_empty_sham.tracking_error_max_q.to_bits(),
        "tracking_error_max_q bit-identity"
    );
    assert_eq!(
        diag_no_sham.tracking_error_max_beta_w.to_bits(),
        diag_empty_sham.tracking_error_max_beta_w.to_bits(),
        "tracking_error_max_beta_w bit-identity"
    );
    assert_eq!(
        diag_no_sham.n_substeps_completed,
        diag_empty_sham.n_substeps_completed,
        "n_substeps_completed equality"
    );
    assert_eq!(
        diag_no_sham.seeds_used, diag_empty_sham.seeds_used,
        "seeds_used equality"
    );
}

// ── Regression: unknown SHAM flags STILL rejected ─────────────────

/// VI.4 — regression of VI.2's `UnrecognizedShamFlag` rejection.
/// After VI.4 wires the 6 known flag names, ANY name outside that set
/// (e.g. `not_a_real_vi4_flag`) must still be rejected by the
/// dispatcher. This guards the contract VI.2 set up at
/// `tests/halcyon_part_vi_parser_rejections.rs::halcyon_vi_2_rejects_unrecognized_sham_flag`.
#[test]
fn halcyon_vi_4_sham_unrecognized_flag_still_rejected() {
    let _env = helpers::setup_canonical();
    let stmt = helpers::build_lt_stmt_with_sham(
        "bb", "face0", helpers::N_DISC,
        helpers::SEED_LO, helpers::SEED_LO,
        1.0, 0.01,
        Some(helpers::sham(vec![helpers::bare("not_a_real_vi4_flag")])),
    );
    let err = loop_transport(&stmt, "U_sh", "E_sh")
        .expect_err("unknown SHAM flag must be rejected at executor entry");
    match err {
        LoopTransportError::UnrecognizedShamFlag { name } => {
            assert_eq!(
                name, "not_a_real_vi4_flag",
                "rejection must echo the offending flag name verbatim"
            );
        }
        other => panic!(
            "expected LoopTransportError::UnrecognizedShamFlag, got: {other:?}"
        ),
    }
}

// ── Science SHAM flags (5) ────────────────────────────────────────

/// VI.4 S₁ — FLAT_FIELD: κ_Q ≡ 0 (parameter-space ramp frozen). With
/// no parameter coupling driving the holonomy, |H_S₁| must vanish to
/// machine ε per v3.1.3 §5 S₁ AND satisfy the 2σ_S₁ sanity gate.
#[test]
fn halcyon_vi_4_sham_flat_field_drives_h_to_zero() {
    let _env = helpers::setup_canonical();
    let stmt = helpers::build_lt_stmt_with_sham(
        "bb", "face0", helpers::N_DISC,
        helpers::SEED_LO, helpers::SEED_HI,
        1.0, 0.01,
        Some(helpers::sham(vec![helpers::bare("FLAT_FIELD")])),
    );
    let diag = loop_transport(&stmt, "U_sh", "E_sh")
        .expect("FLAT_FIELD sham must dispatch (currently no-op-rejected by VI.2 executor)");

    // The antisymmetric primary observable per gate doc + design.notes(4):
    // H_geom = ½(h_forward - h_reversed). Per v3.1.3 §5 S₁,
    // |H_S₁| < 1e-10 (load-bearing) AND |H_S₁| < 2σ_S₁ (sanity).
    let h_geom = 0.5 * (diag.h_forward - diag.h_reversed);
    assert!(
        h_geom.abs() < helpers::MACHINE_EPS_GATE,
        "S₁ machine-ε floor: |H_geom|={} >= 1e-10", h_geom.abs()
    );
    // 2σ gate — additive +1e-15 absorbs the σ=0 trivially-passes case.
    assert!(
        h_geom.abs() < 2.0 * diag.sigma_h_blocked + 1.0e-15,
        "S₁ 2σ sanity: |H_geom|={} vs 2σ={}",
        h_geom.abs(), 2.0 * diag.sigma_h_blocked,
    );
}

/// VI.4 S₂ — ALPHA_ZERO: ALPHA_HALCYON = 0 ⇒ dt = 0 ⇒ no field
/// evolution ⇒ H_geom = 0 to machine ε. v3.1.3 §5 S₂ marks
/// |H_S₂| < 1e-10 as the LOAD-BEARING half of the gate (the 2σ check
/// is sanity only).
#[test]
fn halcyon_vi_4_sham_alpha_zero_drives_h_to_zero() {
    let _env = helpers::setup_canonical();
    let stmt = helpers::build_lt_stmt_with_sham(
        "bb", "face0", helpers::N_DISC,
        helpers::SEED_LO, helpers::SEED_HI,
        1.0, 0.01,
        Some(helpers::sham(vec![helpers::bare("ALPHA_ZERO")])),
    );
    let diag = loop_transport(&stmt, "U_sh", "E_sh")
        .expect("ALPHA_ZERO sham must dispatch");

    let h_geom = 0.5 * (diag.h_forward - diag.h_reversed);
    assert!(
        h_geom.abs() < helpers::MACHINE_EPS_GATE,
        "S₂ load-bearing 1e-10 gate: |H_geom|={}", h_geom.abs()
    );
}

/// VI.4 S₃ — MASS_BASELINE_SCALED: substrate must accept μ ∈
/// {0.1, 1.0, 10.0} without error and echo the value through the
/// diagnostics envelope. The POSITIVE-branch baseline-subtracted
/// invariance check is the orchestrator's job per v3.1.3 §5 S₃, not
/// the substrate's — so this test only asserts dispatch + per-μ run
/// completion, not the 10% subtraction invariant.
#[test]
fn halcyon_vi_4_sham_mass_baseline_scaled_accepts_canonical_mu_values() {
    for &mu in &[0.1_f64, 1.0_f64, 10.0_f64] {
        let _env = helpers::setup_canonical();
        let stmt = helpers::build_lt_stmt_with_sham(
            "bb", "face0", helpers::N_DISC,
            helpers::SEED_LO, helpers::SEED_LO, // single seed per μ for speed
            1.0, 0.01,
            Some(helpers::sham(vec![(
                "MASS_BASELINE_SCALED",
                ShamArg::Number(mu),
            )])),
        );
        let diag = loop_transport(&stmt, "U_sh", "E_sh").unwrap_or_else(|e| {
            panic!("MASS_BASELINE_SCALED must accept μ={mu}; got: {e:?}")
        });
        assert!(
            diag.h_forward.is_finite() && diag.h_reversed.is_finite(),
            "S₃ μ={mu} diagnostics must be finite"
        );
    }
}

/// VI.4 S₃ negative — MASS_BASELINE_SCALED with μ ∉ {0.1, 1.0, 10.0}
/// must be rejected at executor entry. The spec only validates the
/// three canonical baselines. (This will materialize as a new
/// `LoopTransportError::InvalidShamArg` variant per design.)
#[test]
fn halcyon_vi_4_sham_mass_baseline_scaled_rejects_off_grid_mu() {
    let _env = helpers::setup_canonical();
    let stmt = helpers::build_lt_stmt_with_sham(
        "bb", "face0", helpers::N_DISC,
        helpers::SEED_LO, helpers::SEED_LO,
        1.0, 0.01,
        Some(helpers::sham(vec![(
            "MASS_BASELINE_SCALED",
            ShamArg::Number(2.5), // not in {0.1, 1.0, 10.0}
        )])),
    );
    let err = loop_transport(&stmt, "U_sh", "E_sh")
        .expect_err("off-grid μ must be rejected");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("MASS_BASELINE_SCALED") || msg.contains("InvalidShamArg")
            || msg.contains("ShamArg") || msg.contains("baseline"),
        "rejection must surface the invalid argument; got: {msg}"
    );
}

/// VI.4 S₅ — DEGENERATE_LOOP: γ_unit replaced by a zero-area loop
/// (out-and-back along a single edge). The walk_loop returns the
/// SU(2) identity; H_geom must vanish to machine ε AND pass the 2σ_S₅
/// gate per v3.1.3 §5 S₅.
#[test]
fn halcyon_vi_4_sham_degenerate_loop_drives_h_to_zero() {
    let _env = helpers::setup_canonical();
    let stmt = helpers::build_lt_stmt_with_sham(
        "bb", "face0", helpers::N_DISC,
        helpers::SEED_LO, helpers::SEED_HI,
        1.0, 0.01,
        Some(helpers::sham(vec![helpers::bare("DEGENERATE_LOOP")])),
    );
    let diag = loop_transport(&stmt, "U_sh", "E_sh")
        .expect("DEGENERATE_LOOP sham must dispatch");

    let h_geom = 0.5 * (diag.h_forward - diag.h_reversed);
    assert!(
        h_geom.abs() < helpers::MACHINE_EPS_GATE,
        "S₅ machine-ε floor: |H_geom|={}", h_geom.abs()
    );
    assert!(
        h_geom.abs() < 2.0 * diag.sigma_h_blocked + 1.0e-15,
        "S₅ 2σ sanity: |H_geom|={} vs 2σ={}",
        h_geom.abs(), 2.0 * diag.sigma_h_blocked,
    );
}

/// VI.4 S₆ — FROZEN_FIELD: skip drift_step so U is static across all
/// substeps. On a cold-start identity U the walk_loop reads identity →
/// H = 0; H_geom must vanish to machine ε AND pass the 2σ_S₆ gate per
/// v3.1.3 §5 S₆.
#[test]
fn halcyon_vi_4_sham_frozen_field_drives_h_to_zero() {
    let _env = helpers::setup_canonical();
    let stmt = helpers::build_lt_stmt_with_sham(
        "bb", "face0", helpers::N_DISC,
        helpers::SEED_LO, helpers::SEED_HI,
        1.0, 0.01,
        Some(helpers::sham(vec![helpers::bare("FROZEN_FIELD")])),
    );
    let diag = loop_transport(&stmt, "U_sh", "E_sh")
        .expect("FROZEN_FIELD sham must dispatch");

    let h_geom = 0.5 * (diag.h_forward - diag.h_reversed);
    assert!(
        h_geom.abs() < helpers::MACHINE_EPS_GATE,
        "S₆ machine-ε floor: |H_geom|={}", h_geom.abs()
    );
    assert!(
        h_geom.abs() < 2.0 * diag.sigma_h_blocked + 1.0e-15,
        "S₆ 2σ sanity: |H_geom|={} vs 2σ={}",
        h_geom.abs(), 2.0 * diag.sigma_h_blocked,
    );
}

// ── Audit-story (runtime) SHAM flag — EMPTY_LOOP ──────────────────

/// VI.4 — EMPTY_LOOP runtime companion to GC₄: integrator runs zero
/// substeps; verb returns H=0 byte-for-byte. Per design.per_flag_
/// implementation[EMPTY_LOOP]: per-seed arrays are populated (with 0.0
/// entries per seed), n_substeps_completed = 0, sigma = 0,
/// h_forward = h_reversed = 0. ALL f64 values must be the literal
/// floating-point zero (to_bits() == 0u64).
#[test]
fn halcyon_vi_4_sham_empty_loop_returns_zero_byte_for_byte() {
    let _env = helpers::setup_canonical();
    let stmt = helpers::build_lt_stmt_with_sham(
        "bb", "face0", helpers::N_DISC,
        helpers::SEED_LO, helpers::SEED_HI,
        1.0, 0.01,
        Some(helpers::sham(vec![helpers::bare("EMPTY_LOOP")])),
    );
    let diag = loop_transport(&stmt, "U_sh", "E_sh")
        .expect("EMPTY_LOOP sham must dispatch (short-circuits before any KDK)");

    // Byte-for-byte literal zero in the aggregates.
    assert_eq!(
        diag.h_forward.to_bits(), 0u64,
        "EMPTY_LOOP: h_forward must be literal +0.0 byte-for-byte, got {}",
        diag.h_forward
    );
    assert_eq!(
        diag.h_reversed.to_bits(), 0u64,
        "EMPTY_LOOP: h_reversed must be literal +0.0 byte-for-byte"
    );
    assert_eq!(
        diag.sigma_h_blocked.to_bits(), 0u64,
        "EMPTY_LOOP: sigma must be literal +0.0 byte-for-byte"
    );

    // Diagnostics envelope shape preserved (per-seed arrays length =
    // n_seeds, entries all literal zero, n_substeps_completed = 0).
    let expected_seed_count = (helpers::SEED_HI - helpers::SEED_LO + 1) as usize;
    assert_eq!(
        diag.per_seed_h_forward.len(), expected_seed_count,
        "EMPTY_LOOP must preserve per_seed_h_forward length"
    );
    assert_eq!(
        diag.per_seed_h_reversed.len(), expected_seed_count,
        "EMPTY_LOOP must preserve per_seed_h_reversed length"
    );
    for (i, &v) in diag.per_seed_h_forward.iter().enumerate() {
        assert_eq!(
            v.to_bits(), 0u64,
            "EMPTY_LOOP: per_seed_h_forward[{i}] must be literal +0.0"
        );
    }
    for (i, &v) in diag.per_seed_h_reversed.iter().enumerate() {
        assert_eq!(
            v.to_bits(), 0u64,
            "EMPTY_LOOP: per_seed_h_reversed[{i}] must be literal +0.0"
        );
    }
    assert_eq!(
        diag.n_substeps_completed, 0,
        "EMPTY_LOOP: n_substeps_completed must be 0 (zero substeps run)"
    );
}
