//! TDD-HAL-VI.2 — RED — LOOP_TRANSPORT parser grammar acceptance.
//!
//! Pre-registration: HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3 §4.4
//! (Zenodo DOI 10.5281/zenodo.20785681). Gate doc:
//! `theory/halcyon/HALCYON_PART_VI_GATES.md` @ 9a73dc0.
//!
//! Scope:
//!   - `parse()` accepts the full v3.1.3 §4.4 grammar.
//!   - Produces `Statement::LoopTransport` with the right field values.
//!   - Optional `SHAM { ... }` block parses (executor handling deferred
//!     to VI.4; an EMPTY SHAM block is accepted at parser level).
//!
//! Out of scope:
//!   - GC₁..GC₆ acceptance (VI.3).
//!   - SHAM dispatch (VI.4).
//!   - Bit-identity gold fixture (VI.5).
//!
//! These tests intentionally do not compile until the LOOP_TRANSPORT
//! parser arm + supporting types land in `src/parser.rs`. That is the
//! RED state the orchestrator confirms before any impl is written.

#![cfg(feature = "halcyon")]

use gigi::parser::{
    parse, ControlManifoldSpec, LoopTransportOutputId, LoopTransportReturnId, SeedRange, Statement,
};

/// Canonical v3.1.3 §4.4 source — every clause present, canonical
/// ordering. The verb satisfies the letter of the spec or it doesn't
/// ship.
const FULL_V3_1_3_SOURCE: &str = r#"
LOOP_TRANSPORT lattice
  ALONG_LOOP loop_id
  CONTROL_MANIFOLD (Q, BETA_WILSON)
  ADIABATIC TRUE
  RAMP_RATE_Q 0.04
  RAMP_RATE_BETA_W 0.01
  DRIVE_OMEGA 1.0
  DRIVE_F0 0.01
  N_DISCRETIZATION 10000
  PIN_LAMBDA_Q 1.0
  PIN_LAMBDA_BETA_W 1.0
  EPS_Q 0.05
  EPS_BETA_W 0.05
  ALPHA_HALCYON 1.0
  TAU_0 1.0  BETA_TAU 2.0
  MU_BASELINE 1.0  K_SPRING 1.0  C_DAMP 0.1
  SEEDS [20260616..20260623]
  COMPUTE HOLONOMY_FORWARD
  COMPUTE HOLONOMY_REVERSED
  COMPUTE TRACKING_ERROR_TRACE_Q
  COMPUTE TRACKING_ERROR_TRACE_BETA_W
  COMPUTE ADIABATICITY_CHECK
  RETURN H_forward, H_reversed, sigma_H_blocked,
         per_seed_H_forward, per_seed_H_reversed,
         tracking_error_max_Q, tracking_error_max_beta_W,
         adiabaticity_check;
"#;

/// VI.2 grammar acceptance — full v3.1.3 §4.4 source parses and every
/// frozen field lands at the value the spec specifies.
#[test]
fn halcyon_vi_2_grammar_accepts_full_v3_1_3_source() {
    let stmt = parse(FULL_V3_1_3_SOURCE).expect("LOOP_TRANSPORT full v3.1.3 source parses");

    match stmt {
        Statement::LoopTransport {
            lattice,
            loop_id,
            control_manifold,
            adiabatic,
            ramp_rate_q,
            ramp_rate_beta_w,
            drive_omega,
            drive_f0,
            n_discretization,
            pin_lambda_q,
            pin_lambda_beta_w,
            eps_q,
            eps_beta_w,
            alpha_halcyon,
            tau_0,
            beta_tau,
            mu_baseline,
            k_spring,
            c_damp,
            seeds,
            compute,
            return_fields,
            sham,
            beta_wilson_start: _,
        } => {
            assert_eq!(lattice, "lattice");
            assert_eq!(loop_id, "loop_id");
            assert!(matches!(control_manifold, ControlManifoldSpec::QBetaWilson));
            assert!(adiabatic);
            assert_eq!(ramp_rate_q, 0.04);
            assert_eq!(ramp_rate_beta_w, 0.01);
            assert_eq!(drive_omega, 1.0);
            assert_eq!(drive_f0, 0.01);
            assert_eq!(n_discretization, 10_000);
            assert_eq!(pin_lambda_q, 1.0);
            assert_eq!(pin_lambda_beta_w, 1.0);
            assert_eq!(eps_q, 0.05);
            assert_eq!(eps_beta_w, 0.05);
            assert_eq!(alpha_halcyon, 1.0);
            assert_eq!(tau_0, 1.0);
            assert_eq!(beta_tau, 2.0);
            assert_eq!(mu_baseline, 1.0);
            assert_eq!(k_spring, 1.0);
            assert_eq!(c_damp, 0.1);
            assert_eq!(seeds, SeedRange { lo: 20_260_616, hi: 20_260_623 });

            // Every COMPUTE clause shows up in `compute`.
            assert!(compute.contains(&LoopTransportOutputId::HolonomyForward));
            assert!(compute.contains(&LoopTransportOutputId::HolonomyReversed));
            assert!(compute.contains(&LoopTransportOutputId::TrackingErrorTraceQ));
            assert!(compute.contains(&LoopTransportOutputId::TrackingErrorTraceBetaW));
            assert!(compute.contains(&LoopTransportOutputId::AdiabaticityCheck));
            assert_eq!(compute.len(), 5);

            // Every RETURN field shows up in `return_fields`.
            assert!(return_fields.contains(&LoopTransportReturnId::HForward));
            assert!(return_fields.contains(&LoopTransportReturnId::HReversed));
            assert!(return_fields.contains(&LoopTransportReturnId::SigmaHBlocked));
            assert!(return_fields.contains(&LoopTransportReturnId::PerSeedHForward));
            assert!(return_fields.contains(&LoopTransportReturnId::PerSeedHReversed));
            assert!(return_fields.contains(&LoopTransportReturnId::TrackingErrorMaxQ));
            assert!(return_fields.contains(&LoopTransportReturnId::TrackingErrorMaxBetaW));
            assert!(return_fields.contains(&LoopTransportReturnId::AdiabaticityCheck));
            assert_eq!(return_fields.len(), 8);

            // No SHAM block in canonical source.
            assert!(sham.is_none(), "canonical v3.1.3 source has no SHAM block");
        }
        other => panic!("expected Statement::LoopTransport, got {other:?}"),
    }
}

/// VI.2 grammar acceptance — the spec freezes (Q, BETA_WILSON) as the
/// control manifold; the parser must capture that exactly.
#[test]
fn halcyon_vi_2_grammar_control_manifold_is_q_beta_wilson() {
    let stmt = parse(FULL_V3_1_3_SOURCE).expect("parses");
    if let Statement::LoopTransport { control_manifold, .. } = stmt {
        assert!(matches!(control_manifold, ControlManifoldSpec::QBetaWilson));
    } else {
        panic!("expected Statement::LoopTransport");
    }
}

/// VI.2 grammar acceptance — single-seed bracket `[s..s]` collapses to
/// SeedRange { lo: s, hi: s } (inclusive both ends).
#[test]
fn halcyon_vi_2_grammar_single_seed_bracket() {
    let src = r#"
        LOOP_TRANSPORT lat
          ALONG_LOOP face0
          CONTROL_MANIFOLD (Q, BETA_WILSON)
          ADIABATIC TRUE
          RAMP_RATE_Q 0.04 RAMP_RATE_BETA_W 0.01
          DRIVE_OMEGA 1.0 DRIVE_F0 0.01
          N_DISCRETIZATION 100
          PIN_LAMBDA_Q 1.0 PIN_LAMBDA_BETA_W 1.0
          EPS_Q 0.05 EPS_BETA_W 0.05
          ALPHA_HALCYON 1.0 TAU_0 1.0 BETA_TAU 2.0
          MU_BASELINE 1.0 K_SPRING 1.0 C_DAMP 0.1
          SEEDS [20260616..20260616]
          COMPUTE HOLONOMY_FORWARD
          RETURN H_forward;
    "#;
    let stmt = parse(src).expect("single-seed bracket parses");
    if let Statement::LoopTransport { seeds, .. } = stmt {
        assert_eq!(seeds, SeedRange { lo: 20_260_616, hi: 20_260_616 });
    } else {
        panic!("expected Statement::LoopTransport");
    }
}

/// VI.2 grammar acceptance — empty `SHAM { }` block parses (executor
/// handling deferred to VI.4; an EMPTY SHAM block carries no flags so
/// VI.2 accepts it).
#[test]
fn halcyon_vi_2_grammar_empty_sham_block_parses() {
    let src = r#"
        LOOP_TRANSPORT lat
          ALONG_LOOP face0
          CONTROL_MANIFOLD (Q, BETA_WILSON)
          ADIABATIC TRUE
          RAMP_RATE_Q 0.04 RAMP_RATE_BETA_W 0.01
          DRIVE_OMEGA 1.0 DRIVE_F0 0.01
          N_DISCRETIZATION 100
          PIN_LAMBDA_Q 1.0 PIN_LAMBDA_BETA_W 1.0
          EPS_Q 0.05 EPS_BETA_W 0.05
          ALPHA_HALCYON 1.0 TAU_0 1.0 BETA_TAU 2.0
          MU_BASELINE 1.0 K_SPRING 1.0 C_DAMP 0.1
          SEEDS [20260616..20260616]
          COMPUTE HOLONOMY_FORWARD
          SHAM { }
          RETURN H_forward;
    "#;
    let stmt = parse(src).expect("LOOP_TRANSPORT with empty SHAM block parses");
    if let Statement::LoopTransport { sham, .. } = stmt {
        let sham = sham.expect("SHAM block recorded on the Statement variant");
        assert!(sham.flags.is_empty(), "empty SHAM block carries no flags");
    } else {
        panic!("expected Statement::LoopTransport");
    }
}

/// VI.2 grammar acceptance — ADIABATIC FALSE round-trips as the bool
/// `false` (the verdict path branches on this; see v3.1.3 §4.2).
#[test]
fn halcyon_vi_2_grammar_adiabatic_false() {
    let src = r#"
        LOOP_TRANSPORT lat
          ALONG_LOOP face0
          CONTROL_MANIFOLD (Q, BETA_WILSON)
          ADIABATIC FALSE
          RAMP_RATE_Q 0.04 RAMP_RATE_BETA_W 0.01
          DRIVE_OMEGA 1.0 DRIVE_F0 0.01
          N_DISCRETIZATION 100
          PIN_LAMBDA_Q 1.0 PIN_LAMBDA_BETA_W 1.0
          EPS_Q 0.05 EPS_BETA_W 0.05
          ALPHA_HALCYON 1.0 TAU_0 1.0 BETA_TAU 2.0
          MU_BASELINE 1.0 K_SPRING 1.0 C_DAMP 0.1
          SEEDS [20260616..20260616]
          COMPUTE HOLONOMY_FORWARD
          RETURN H_forward;
    "#;
    let stmt = parse(src).expect("ADIABATIC FALSE parses");
    if let Statement::LoopTransport { adiabatic, .. } = stmt {
        assert!(!adiabatic);
    } else {
        panic!("expected Statement::LoopTransport");
    }
}
