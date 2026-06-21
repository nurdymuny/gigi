//! TDD-HAL-VI.2 — RED — LOOP_TRANSPORT executor end-to-end smoke.
//!
//! Pre-registration: HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3 §4.4
//! (Zenodo DOI 10.5281/zenodo.20785681). Gate doc:
//! `theory/halcyon/HALCYON_PART_VI_GATES.md` @ 9a73dc0.
//!
//! Scope: prove the verb compiles, parses, executes end-to-end, and
//! returns a `LoopTransportDiagnostics` with the right SHAPE. Does NOT
//! verify any acceptance criterion — that is VI.3 (GC₁..GC₆) and the
//! bit-identity gold fixture is VI.5.
//!
//! Pattern mirrors `tests/halcyon_part_iv_gold.rs` setup (LATTICE +
//! GAUGE_FIELD + E_FIELD declared, register a closed loop, run the
//! verb, then assert shape on the diagnostics). Small N (100) and a
//! single seed for speed — the canonical run is N=10_000 across the
//! full SEEDS bracket.

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::loop_transport::{
    loop_transport, AdiabaticityCheck, LoopTransportDiagnostics,
};
use gigi::parser::{execute, parse, Statement};

/// Set up the smallest closed-loop environment the LOOP_TRANSPORT
/// executor needs: buckyball lattice + SU(2) U field + E field +
/// one closed pentagonal loop.
fn setup_halcyon_canonical_buckyball() -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let lat = "LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat).expect("parse LATTICE");
    execute(&mut engine, &stmt).expect("exec LATTICE");

    // Re-publish through `register_su2` so the executor's mut handle
    // path can pick it up (mirrors halcyon_part_iv_gold.rs).
    let gf = "GAUGE_FIELD U_lt ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(gf).expect("parse GAUGE_FIELD");
    execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");
    {
        let lat = gigi::lattice::registry::get("buckyball").expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            "U_lt".into(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    let ef = "E_FIELD E_lt ON GAUGE_FIELD U_lt INIT ZERO;";
    let stmt = parse(ef).expect("parse E_FIELD");
    execute(&mut engine, &stmt).expect("exec E_FIELD");

    // Register a closed pentagonal loop on the buckyball.
    let loop_decl = "LOOP face0 ON buckyball FACE 0;";
    let stmt = parse(loop_decl).expect("parse LOOP face0");
    execute(&mut engine, &stmt).expect("exec LOOP face0");

    (engine, dir)
}

/// Small-N LOOP_TRANSPORT source — 100 substeps, single seed, β_W
/// starts inside the validated regime, all required clauses present.
fn small_n_source() -> &'static str {
    r#"
        LOOP_TRANSPORT buckyball
          ALONG_LOOP face0
          CONTROL_MANIFOLD (Q, BETA_WILSON)
          ADIABATIC TRUE
          RAMP_RATE_Q 0.04
          RAMP_RATE_BETA_W 0.01
          DRIVE_OMEGA 1.0
          DRIVE_F0 0.01
          N_DISCRETIZATION 100
          PIN_LAMBDA_Q 1.0
          PIN_LAMBDA_BETA_W 1.0
          EPS_Q 0.05
          EPS_BETA_W 0.05
          ALPHA_HALCYON 1.0
          TAU_0 1.0  BETA_TAU 2.0
          MU_BASELINE 1.0  K_SPRING 1.0  C_DAMP 0.1
          BETA_WILSON_START 2.5
          SEEDS [20260616..20260616]
          COMPUTE HOLONOMY_FORWARD
          COMPUTE HOLONOMY_REVERSED
          COMPUTE TRACKING_ERROR_TRACE_Q
          COMPUTE TRACKING_ERROR_TRACE_BETA_W
          COMPUTE ADIABATICITY_CHECK
          RETURN H_forward, H_reversed, sigma_H_blocked,
                 per_seed_H_forward, per_seed_H_reversed,
                 tracking_error_max_Q, tracking_error_max_beta_W,
                 adiabaticity_check;
    "#
}

/// VI.2 smoke — LOOP_TRANSPORT runs end-to-end against the canonical
/// buckyball + closed pentagonal loop, returning a fully-populated
/// `LoopTransportDiagnostics`.
#[test]
fn halcyon_vi_2_smoke_loop_transport_runs_end_to_end() {
    let (_engine, _dir) = setup_halcyon_canonical_buckyball();
    let stmt = parse(small_n_source()).expect("small-N LOOP_TRANSPORT parses");
    assert!(matches!(stmt, Statement::LoopTransport { .. }));

    let diag: LoopTransportDiagnostics =
        loop_transport(&stmt, "U_lt", "E_lt").expect("verb runs end-to-end");

    // SHAPE assertions only — no numerical acceptance lands until VI.3.

    // SEEDS bracket [20260616..20260616] = 1 seed.
    assert_eq!(diag.seeds_used.len(), 1);
    assert_eq!(diag.seeds_used[0], 20_260_616);

    // Per-seed traces have length == #seeds.
    assert_eq!(diag.per_seed_h_forward.len(), 1);
    assert_eq!(diag.per_seed_h_reversed.len(), 1);

    // Aggregates finite.
    assert!(diag.h_forward.is_finite(), "h_forward must be finite");
    assert!(diag.h_reversed.is_finite(), "h_reversed must be finite");
    assert!(
        diag.sigma_h_blocked.is_finite() && diag.sigma_h_blocked >= 0.0,
        "sigma_h_blocked must be non-negative finite"
    );

    // Tracking-error maxes finite + non-negative.
    assert!(
        diag.tracking_error_max_q.is_finite() && diag.tracking_error_max_q >= 0.0,
        "tracking_error_max_q must be non-negative finite"
    );
    assert!(
        diag.tracking_error_max_beta_w.is_finite() && diag.tracking_error_max_beta_w >= 0.0,
        "tracking_error_max_beta_w must be non-negative finite"
    );

    // Adiabaticity verdict carries a finite ratio in either branch.
    match diag.adiabaticity_check {
        AdiabaticityCheck::Acceptable { ratio } => {
            assert!(ratio.is_finite() && ratio < 0.1, "Acceptable ⇒ ratio < 0.1");
        }
        AdiabaticityCheck::AmbiguousForced { ratio } => {
            assert!(
                ratio.is_finite() && ratio >= 0.1,
                "AmbiguousForced ⇒ ratio ≥ 0.1"
            );
        }
    }

    // Smoke runs all substeps to completion (no early-exit in VI.2).
    assert_eq!(diag.n_substeps_completed, 100);
}

/// VI.2 smoke — all 8 RETURN fields of the diagnostics surface are
/// populated. This locks the public contract VI.3/4/5 will index into.
#[test]
fn halcyon_vi_2_smoke_diagnostics_has_all_eight_return_fields() {
    let (_engine, _dir) = setup_halcyon_canonical_buckyball();
    let stmt = parse(small_n_source()).expect("parses");
    let diag = loop_transport(&stmt, "U_lt", "E_lt").expect("runs");

    // Field-shape gauntlet — every v3.1.3 §4.4 RETURN entry has a
    // home on the struct.
    let _h_forward: f64 = diag.h_forward;
    let _h_reversed: f64 = diag.h_reversed;
    let _sigma: f64 = diag.sigma_h_blocked;
    let _per_seed_fwd: &Vec<f64> = &diag.per_seed_h_forward;
    let _per_seed_rev: &Vec<f64> = &diag.per_seed_h_reversed;
    let _err_q: f64 = diag.tracking_error_max_q;
    let _err_bw: f64 = diag.tracking_error_max_beta_w;
    let _verdict: &AdiabaticityCheck = &diag.adiabaticity_check;

    // Echo block (not in RETURN; aids debuggability + this test).
    assert!(!diag.seeds_used.is_empty());
    assert!(diag.n_substeps_completed > 0);
}

/// VI.2 smoke — `AdiabaticityCheck::from_ratio` agrees with the
/// v3.1.3 §4.2 threshold (ratio < 0.1 → Acceptable; ≥ 0.1 → forced).
/// Pure-function gate; no executor needed.
#[test]
fn halcyon_vi_2_adiabaticity_threshold_at_0_1() {
    let acc = AdiabaticityCheck::from_ratio(0.05);
    let amb_eq = AdiabaticityCheck::from_ratio(0.1);
    let amb_hi = AdiabaticityCheck::from_ratio(0.5);

    assert!(matches!(acc, AdiabaticityCheck::Acceptable { .. }));
    assert!(matches!(amb_eq, AdiabaticityCheck::AmbiguousForced { .. }));
    assert!(matches!(amb_hi, AdiabaticityCheck::AmbiguousForced { .. }));

    assert_eq!(acc.ratio(), 0.05);
    assert_eq!(amb_eq.ratio(), 0.1);
    assert_eq!(amb_hi.ratio(), 0.5);

    assert!(acc.is_acceptable());
    assert!(!amb_eq.is_acceptable());
    assert!(!amb_hi.is_acceptable());
}
