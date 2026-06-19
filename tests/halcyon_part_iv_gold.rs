//! TDD-HAL-IV.10 — Gold gate. Halcyon Gate IV contract via the
//! GIGI-internal canonical reference
//! `tests/fixtures/halcyon/part_iv/symplectic_flow_canonical.json`
//! harvested by gate IV.9.
//!
//! Per Bee's locked decision 1 (inherited): cross-binding bit-identity
//! against Halcyon's NumPy PCG64 mock is impossible by design. The
//! bit-identity contract this gate enforces is **intra-GIGI** — same
//! code, same seed, same OS, same profile → byte-identical (final U,
//! final E, measurement chains) trajectory. The IV.9 fixture is the
//! sentinel; this gate fails loudly the moment the GIGI side drifts
//! from its own past.
//!
//! ── Map onto Halcyon Gate IV ──
//!
//! - **gate_iv_a** — load the IV.9 canonical envelope, replay the same
//!   5-statement GQL block, assert byte-equality on `final_U`,
//!   `final_E`, and the four `measurement_history` chains
//!   (`h_total`, `mean_plaquette`, `gauss_residual_max`,
//!   `q_surrogate`). Run only under `--release` per locked decision
//!   IV-F(b) + the III.8c profile-pin precedent.
//!
//! - **gate_iv_b** — energy drift two-tier (locked decision IV-F).
//!   (a) ACCEPTANCE: `max|ΔH/H_0| < 1e-3` Halcyon bound (debug-safe).
//!   (b) REGRESSION: `max_energy_drift_rel` byte-identical to fixture
//!   (release-only, `f64::to_bits` compare).
//!
//! - **gate_iv_c** — Gauss residual two-tier.
//!   (a) ACCEPTANCE: `||G_cov||_inf < 1e-9` (per-step projection holds).
//!   (b) REGRESSION: `gauss_residual_max` byte-identical to fixture.
//!
//! - **gate_iv_d** — `SELECT H_TOTAL OF (U, E)` returns scalar f64 with
//!   no `PartIvObservableNotReady` error (the IV-J reversal landed in
//!   IV.7; this gate re-asserts at the gold-gate altitude).
//!
//! - **gate_iv_e** — diagnostics envelope shape: every Part IV
//!   `SymplecticFlowDiagnostics` field is populated and surfaced through
//!   the Rows envelope (`seed`, `beta`, `dt`, `n_steps_completed`,
//!   `max_energy_drift_rel`, `gauss_residual_max`,
//!   `cg_iterations_per_step_p99`). `cg_iterations_per_step_p99` is
//!   PRESENT but NOT compared against the fixture (DIAGNOSTIC ONLY per
//!   A2, locked decision).
//!
//! ── Profile pin (III.8c precedent) ──
//!
//! Tests that assert byte-identical reproducibility against the IV.9
//! canonical fixture run only under `--release` (where the fixture was
//! harvested) and are `#[cfg_attr(debug_assertions, ignore)]`. Debug-
//! profile FMA + reassociation accumulate ~few ULPs across 1000 KDK
//! steps × per-step Tikhonov CG, so debug runs would land a different
//! bit pattern and break the gate. Acceptance bounds remain enforceable
//! under any profile.
//!
//! Run:
//! ```text
//! cargo test --features halcyon --test halcyon_part_iv_gold --release
//! ```
//!
//! ── Optionality contract ──
//!
//! Gated on the `halcyon` composite feature so the no-default-features
//! build stays byte-identical at 852/0 (Bee's optionality contract
//! carrying through every Part I/II/III gate, now Part IV).

#![cfg(feature = "halcyon")]

use std::fs;
use std::path::PathBuf;

use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

/// Path to the IV.9 canonical-reference fixture, anchored to the test
/// crate's manifest dir so `cargo test` from anywhere finds it.
fn symplectic_flow_canonical_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join("part_iv")
        .join("symplectic_flow_canonical.json")
}

/// Reset every Part-IV-relevant singleton to a clean slate.
fn clear_registries() {
    gigi::gauge::registry::clear();
    gigi::gauge::registry::clear_e_registry();
    gigi::lattice::registry::clear();
    gigi::gauge::clear_symplectic_flow_diagnostics_cache();
}

/// Drive the locked 5-statement GQL block — the same block IV.9
/// harvested under — through the parser+executor path ONCE. Returns
/// the SYMPLECTIC_FLOW Rows envelope (single record) so each gate can
/// pluck the chain / diagnostic it tests.
///
/// The block:
/// ```text
/// LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';
/// GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;
/// GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 SEED 20260616;
/// E_FIELD E ON GAUGE_FIELD U INIT MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;
/// SYMPLECTIC_FLOW U FROM (U=U, E=E) BETA 2.5 DT 0.02 N_STEPS 1000
///     PROJECT_GAUSS TRUE MEASURE_EVERY 20
///     MEASURE (H_TOTAL, MEAN(PLAQUETTE), GAUSS_RESIDUAL_MAX, Q_SURROGATE)
///     SEED 20260617;
/// ```
fn replay_canonical_block(engine: &mut gigi::engine::Engine) -> gigi::types::Record {
    let lat_decl = "LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat_decl).expect("parse LATTICE");
    execute(engine, &stmt).expect("exec LATTICE");

    let g_decl = "GAUGE_FIELD U ON LATTICE buckyball \
                  GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
    execute(engine, &stmt).expect("exec GAUGE_FIELD");

    // Re-publish through `register_su2` so GIBBS_SAMPLE + SYMPLECTIC_FLOW
    // find the SU(2)-mut handle (D4 fix-up — mirrors III.8b / IV.9).
    {
        let lat = gigi::lattice::registry::get("buckyball").expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            "U".into(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    let thermalize = "GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 SEED 20260616;";
    let stmt = parse(thermalize).expect("parse GIBBS_SAMPLE (thermalize)");
    execute(engine, &stmt).expect("exec GIBBS_SAMPLE (thermalize)");

    let e_decl = "E_FIELD E ON GAUGE_FIELD U INIT MAXWELL_BOLTZMANN \
                  BETA 2.5 SEED 20260617;";
    let stmt = parse(e_decl).expect("parse E_FIELD");
    execute(engine, &stmt).expect("exec E_FIELD");

    let flow = "SYMPLECTIC_FLOW U FROM (U=U, E=E) BETA 2.5 DT 0.02 \
                N_STEPS 1000 PROJECT_GAUSS TRUE MEASURE_EVERY 20 \
                MEASURE (H_TOTAL, MEAN(PLAQUETTE), GAUSS_RESIDUAL_MAX, \
                Q_SURROGATE) SEED 20260617;";
    let stmt = parse(flow).expect("parse SYMPLECTIC_FLOW");
    let rows = match execute(engine, &stmt).expect("exec SYMPLECTIC_FLOW") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows envelope, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "SYMPLECTIC_FLOW returns one row");
    rows.into_iter().next().unwrap()
}

/// Load the IV.9 canonical fixture from disk. Panics with a regen
/// hint if the file is missing — the IV.10 gate is meaningless without
/// the IV.9 sentinel.
fn load_canonical_fixture() -> serde_json::Value {
    let body = fs::read_to_string(symplectic_flow_canonical_path()).unwrap_or_else(|e| {
        panic!(
            "read IV.9 fixture at {}: {e}. Run \
             `cargo test --features halcyon --test harvest_part_iv_canonical \
              --release -- --ignored --nocapture` to regenerate.",
            symplectic_flow_canonical_path().display()
        )
    });
    serde_json::from_str(&body).expect("parse symplectic_flow_canonical.json")
}

/// Pull a `Vec<u64>` from a `"bits"` slot inside a `measurement_history`
/// chain. Panics with a descriptive message if the path is missing.
fn chain_bits(fixture: &serde_json::Value, name: &str) -> Vec<u64> {
    fixture["measurement_history"][name]["bits"]
        .as_array()
        .unwrap_or_else(|| {
            panic!("fixture measurement_history.{name}.bits missing or not an array")
        })
        .iter()
        .map(|b| {
            b.as_u64()
                .unwrap_or_else(|| panic!("fixture measurement_history.{name}.bits entry not u64"))
        })
        .collect()
}

/// TDD-HAL-IV.10: Halcyon Gate IV.A — replay the locked 5-statement
/// GQL block, assert byte-equality between the live run's (final_U,
/// final_E) buffers and the four measurement chains and the IV.9
/// canonical fixture, using `f64::to_bits` as the oracle.
#[test]
#[cfg_attr(debug_assertions, ignore)]
fn tdd_hal_iv_10_a_symplectic_flow_canonical() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    let row = replay_canonical_block(&mut engine);

    let fixture = load_canonical_fixture();

    // ── (1) measurement chains: byte-identical bit-pattern arrays. ──
    let h_label = gigi::gauge::ObservableId::HTotal.label();
    let p_label = gigi::gauge::ObservableId::MeanPlaquette.label();
    let g_label = gigi::gauge::ObservableId::GaussResidualMax.label();
    let q_label = gigi::gauge::ObservableId::QSurrogate.label();

    let h_history: &Vec<f64> = match row.get(h_label) {
        Some(Value::Vector(v)) => v,
        other => panic!("missing/wrong {h_label} chain: {other:?}"),
    };
    let p_history: &Vec<f64> = match row.get(p_label) {
        Some(Value::Vector(v)) => v,
        other => panic!("missing/wrong {p_label} chain: {other:?}"),
    };
    let g_history: &Vec<f64> = match row.get(g_label) {
        Some(Value::Vector(v)) => v,
        other => panic!("missing/wrong {g_label} chain: {other:?}"),
    };
    let q_history: &Vec<f64> = match row.get(q_label) {
        Some(Value::Vector(v)) => v,
        other => panic!("missing/wrong {q_label} chain: {other:?}"),
    };

    let h_bits_fix = chain_bits(&fixture, "h_total");
    let p_bits_fix = chain_bits(&fixture, "mean_plaquette");
    let g_bits_fix = chain_bits(&fixture, "gauss_residual_max");
    let q_bits_fix = chain_bits(&fixture, "q_surrogate");

    assert_eq!(
        h_history.len(),
        h_bits_fix.len(),
        "h_total chain length drift vs fixture"
    );
    for (i, v) in h_history.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            h_bits_fix[i],
            "h_total[{i}] bit pattern drift vs IV.9 fixture: \
             run={:#x} fixture={:#x}",
            v.to_bits(),
            h_bits_fix[i]
        );
    }
    for (i, v) in p_history.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            p_bits_fix[i],
            "mean_plaquette[{i}] bit pattern drift vs IV.9 fixture"
        );
    }
    for (i, v) in g_history.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            g_bits_fix[i],
            "gauss_residual_max[{i}] bit pattern drift vs IV.9 fixture"
        );
    }
    for (i, v) in q_history.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            q_bits_fix[i],
            "q_surrogate[{i}] bit pattern drift vs IV.9 fixture"
        );
    }

    // ── (2) final U buffer: byte-identical to fixture `final_U_bits`. ──
    let final_u: Vec<f64> = {
        let handle = gigi::gauge::registry::get("U").expect("post-flow U");
        handle.as_dense_buffer().data.clone()
    };
    let final_u_bits_fix: Vec<u64> = fixture["final_U_bits"]
        .as_array()
        .expect("final_U_bits array")
        .iter()
        .map(|b| b.as_u64().expect("final_U_bits entry not u64"))
        .collect();
    assert_eq!(
        final_u.len(),
        final_u_bits_fix.len(),
        "final U buffer length drift vs fixture"
    );
    for (i, v) in final_u.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            final_u_bits_fix[i],
            "final_U[{i}] bit pattern drift vs IV.9 fixture"
        );
    }

    // ── (3) final E buffer: byte-identical to fixture `final_E_bits`. ──
    let final_e: Vec<f64> = {
        let handle = gigi::gauge::registry::get_su2_e_mut("E").expect("post-flow E");
        let guard = handle.lock().expect("e field mutex poisoned");
        guard.buffer.data.clone()
    };
    let final_e_bits_fix: Vec<u64> = fixture["final_E_bits"]
        .as_array()
        .expect("final_E_bits array")
        .iter()
        .map(|b| b.as_u64().expect("final_E_bits entry not u64"))
        .collect();
    assert_eq!(
        final_e.len(),
        final_e_bits_fix.len(),
        "final E buffer length drift vs fixture"
    );
    for (i, v) in final_e.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            final_e_bits_fix[i],
            "final_E[{i}] bit pattern drift vs IV.9 fixture"
        );
    }
}

/// TDD-HAL-IV.10: Halcyon Gate IV.B — energy drift two-tier
/// (locked decision IV-F).
///   (a) ACCEPTANCE: `max|ΔH/H_0| < 1e-3` Halcyon bound. Always
///       enforced (debug + release).
///   (b) REGRESSION: `max_energy_drift_rel` byte-identical to the IV.9
///       fixture. Release-only — debug-profile FMA + reassociation
///       perturb the diagnostic by a few ULPs across 1000 KDK steps
///       (mirrors the III.8c profile-pin precedent on `P_history`).
///       The byte-equality arm is gated `#[cfg(not(debug_assertions))]`
///       inside the test body so the acceptance tier still runs under
///       debug.
#[test]
fn tdd_hal_iv_10_b_energy_drift_two_tier() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    let row = replay_canonical_block(&mut engine);

    let max_drift = match row.get("max_energy_drift_rel") {
        Some(Value::Float(f)) => *f,
        other => panic!("missing/wrong max_energy_drift_rel: {other:?}"),
    };

    // (a) ACCEPTANCE — Halcyon energy-drift bound. Profile-independent.
    assert!(
        max_drift < 1e-3,
        "max_energy_drift_rel = {max_drift:e} exceeds Halcyon 1e-3 \
         acceptance bound (locked decision IV-F(a))"
    );

    // (b) REGRESSION — byte-identical to fixture. Release-only.
    #[cfg(not(debug_assertions))]
    {
        let fixture = load_canonical_fixture();
        let max_drift_fix = fixture["diagnostics"]["max_energy_drift_rel"]
            .as_f64()
            .expect("diagnostics.max_energy_drift_rel f64");
        assert_eq!(
            max_drift.to_bits(),
            max_drift_fix.to_bits(),
            "max_energy_drift_rel bit pattern drift vs IV.9 fixture: \
             run={:#x} fixture={:#x}",
            max_drift.to_bits(),
            max_drift_fix.to_bits()
        );
    }
}

/// TDD-HAL-IV.10: Halcyon Gate IV.C — Gauss residual two-tier.
///   (a) ACCEPTANCE: `||G_cov||_inf < 1e-9` (per-step projection
///       holds). Always enforced.
///   (b) REGRESSION: `gauss_residual_max` byte-identical to fixture.
///       Release-only (same profile-pin rationale as IV.B).
#[test]
fn tdd_hal_iv_10_c_gauss_residual_two_tier() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    let row = replay_canonical_block(&mut engine);

    let g_res = match row.get("gauss_residual_max") {
        Some(Value::Float(f)) => *f,
        other => panic!("missing/wrong gauss_residual_max diagnostic: {other:?}"),
    };

    // (a) ACCEPTANCE — per-step projection holds. Profile-independent.
    assert!(
        g_res < 1e-9,
        "gauss_residual_max = {g_res:e} exceeds 1e-9 per-step-projection \
         acceptance bound (locked decision IV-F(a))"
    );

    // (b) REGRESSION — byte-identical to fixture. Release-only.
    #[cfg(not(debug_assertions))]
    {
        let fixture = load_canonical_fixture();
        let g_res_fix = fixture["diagnostics"]["gauss_residual_max"]
            .as_f64()
            .expect("diagnostics.gauss_residual_max f64");
        assert_eq!(
            g_res.to_bits(),
            g_res_fix.to_bits(),
            "gauss_residual_max bit pattern drift vs IV.9 fixture: \
             run={:#x} fixture={:#x}",
            g_res.to_bits(),
            g_res_fix.to_bits()
        );
    }
}

/// TDD-HAL-IV.10: Halcyon Gate IV.D — `SELECT H_TOTAL OF (U, E)` returns
/// a scalar f64 (no `PartIvObservableNotReady` error). The III.5 stub
/// reversal landed in IV.7; this gate re-asserts at the gold-gate
/// altitude that the executor path has a positive H_TOTAL surface for
/// an (IDENTITY, MaxwellBoltzmann) pair on the buckyball.
#[test]
fn tdd_hal_iv_10_d_h_total_now_returns() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");

    // Lattice + IDENTITY U + republish for SU(2)-mut surface.
    let lat_decl = "LATTICE iv_10_d_bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat_decl).expect("parse LATTICE");
    execute(&mut engine, &stmt).expect("exec LATTICE");

    let g_decl = "GAUGE_FIELD U_iv_10_d ON LATTICE iv_10_d_bb \
                  GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
    execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");

    {
        let lat = gigi::lattice::registry::get("iv_10_d_bb").expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            "U_iv_10_d".into(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    // E_FIELD MaxwellBoltzmann at β=2.5, fixed seed.
    let e_decl = "E_FIELD E_iv_10_d ON GAUGE_FIELD U_iv_10_d INIT \
                  MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;";
    let stmt = parse(e_decl).expect("parse E_FIELD");
    execute(&mut engine, &stmt).expect("exec E_FIELD");

    // SELECT H_TOTAL OF (U, E); — positive case.
    let stmt = parse("SELECT H_TOTAL OF (U_iv_10_d, E_iv_10_d);")
        .expect("parse SELECT H_TOTAL");
    let rows = match execute(&mut engine, &stmt).expect("exec SELECT H_TOTAL") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows for H_TOTAL, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "SELECT H_TOTAL returns one row");
    let h_total = match rows[0].get("value") {
        Some(Value::Float(v)) => *v,
        other => panic!(
            "missing/wrong value column (expected f64; III.5 stub must be \
             reversed by IV-J): {other:?}"
        ),
    };
    // The Hamiltonian on (IDENTITY, MaxwellBoltzmann at β=2.5) is finite
    // and strictly positive: kinetic = g² · Σ |E_vec|² > 0 (E was drawn
    // from MB, not Zero), potential = (F/g²)·(1 - 1) = 0 on IDENTITY.
    // Together they sum to a finite positive scalar — never NaN, never
    // an error.
    assert!(
        h_total.is_finite(),
        "H_TOTAL must be finite f64 (no PartIvObservableNotReady), got {h_total}"
    );
    assert!(
        h_total > 0.0,
        "H_TOTAL on (IDENTITY, MB β=2.5) is kinetic > 0 + potential = 0, \
         must be strictly positive; got {h_total}"
    );
}

/// TDD-HAL-IV.10: Halcyon Gate IV.E — the SYMPLECTIC_FLOW Rows envelope
/// surfaces every `SymplecticFlowDiagnostics` field (seed, beta, dt,
/// n_steps_completed, max_energy_drift_rel, gauss_residual_max,
/// cg_iterations_per_step_p99). `cg_iterations_per_step_p99` is
/// PRESENT but NOT compared against the fixture (DIAGNOSTIC ONLY per
/// the A2 matrix; locked decision IV-F + A2 doc).
#[test]
fn tdd_hal_iv_10_e_diagnostics_envelope_shape() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");

    // Cheap flow (3 KDK steps) — this gate is about the envelope shape,
    // not about the trajectory. Avoid the 1000-step canonical replay.
    let lat_decl = "LATTICE iv_10_e_bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat_decl).expect("parse LATTICE");
    execute(&mut engine, &stmt).expect("exec LATTICE");

    let g_decl = "GAUGE_FIELD U_iv_10_e ON LATTICE iv_10_e_bb \
                  GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
    execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");

    {
        let lat = gigi::lattice::registry::get("iv_10_e_bb").expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            "U_iv_10_e".into(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    let e_decl = "E_FIELD E_iv_10_e ON GAUGE_FIELD U_iv_10_e INIT \
                  MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;";
    let stmt = parse(e_decl).expect("parse E_FIELD");
    execute(&mut engine, &stmt).expect("exec E_FIELD");

    let flow = "SYMPLECTIC_FLOW U_iv_10_e FROM (U=U_iv_10_e, E=E_iv_10_e) \
                BETA 2.5 DT 0.02 N_STEPS 3 PROJECT_GAUSS TRUE \
                MEASURE_EVERY 1 MEASURE (H_TOTAL) SEED 20260617;";
    let stmt = parse(flow).expect("parse SYMPLECTIC_FLOW");
    let rows = match execute(&mut engine, &stmt).expect("exec SYMPLECTIC_FLOW") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows envelope, got {other:?}"),
    };
    let row = &rows[0];

    // All 7 diagnostics fields present + typed.
    match row.get("seed") {
        Some(Value::Integer(n)) => assert_eq!(*n, 20260617),
        other => panic!("missing/wrong seed: {other:?}"),
    }
    match row.get("beta") {
        Some(Value::Float(b)) => assert_eq!(*b, 2.5),
        other => panic!("missing/wrong beta: {other:?}"),
    }
    match row.get("dt") {
        Some(Value::Float(d)) => assert_eq!(*d, 0.02),
        other => panic!("missing/wrong dt: {other:?}"),
    }
    match row.get("n_steps_completed") {
        Some(Value::Integer(n)) => assert_eq!(*n, 3),
        other => panic!("missing/wrong n_steps_completed: {other:?}"),
    }
    match row.get("max_energy_drift_rel") {
        Some(Value::Float(f)) => assert!(
            f.is_finite(),
            "max_energy_drift_rel must be finite, got {f}"
        ),
        other => panic!("missing/wrong max_energy_drift_rel: {other:?}"),
    }
    match row.get("gauss_residual_max") {
        Some(Value::Float(f)) => assert!(
            f.is_finite() && *f >= 0.0,
            "gauss_residual_max must be finite non-negative, got {f}"
        ),
        other => panic!("missing/wrong gauss_residual_max: {other:?}"),
    }
    // cg_iterations_per_step_p99 — PRESENT, finite, non-negative. NOT
    // compared against any fixture (DIAGNOSTIC ONLY per A2 matrix).
    match row.get("cg_iterations_per_step_p99") {
        Some(Value::Float(f)) => assert!(
            f.is_finite() && *f >= 0.0,
            "cg_iterations_per_step_p99 must be finite non-negative, got {f}"
        ),
        other => panic!(
            "missing/wrong cg_iterations_per_step_p99 \
             (DIAGNOSTIC ONLY but must be present): {other:?}"
        ),
    }
}
