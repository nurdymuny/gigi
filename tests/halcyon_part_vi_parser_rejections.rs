//! TDD-HAL-VI.2 — RED — LOOP_TRANSPORT parser-rejection contract.
//!
//! Pre-registration: HALCYON_FALSIFICATION_BATTERY_SPEC_v3.1.3 §2
//! (validated β_W regime) and §4.4 grammar. Gate doc:
//! `theory/halcyon/HALCYON_PART_VI_GATES.md` @ 9a73dc0.
//!
//! Every rejection here is raised PRE-EXECUTOR — the verb either
//! satisfies the validated regime / grammar before the integrator
//! starts, or it doesn't run. This is the audit-story flag stance
//! from the gate doc §SHAM table.
//!
//! Errors are surfaced via the executor pathway (parser front-end
//! returns `Result<Statement, String>`; the structured error variant
//! is what the executor / acceptance battery dispatches on). These
//! tests assert the variant lands at the `Display` boundary — the
//! string form must mention the variant name + the offending value
//! so VI.3 / VI.4 can pattern-match deterministically.

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::loop_transport::LoopTransportError;
use gigi::parser::{execute, parse};

/// Build a fresh engine + register a buckyball lattice + a closed
/// pentagon loop `face0`. Mirrors the smoke test setup. The lattice
/// + LOOP_CLOSED declaration are independent of the LOOP_TRANSPORT
/// verb under test — failures here would be test-infrastructure bugs,
/// not contract violations.
fn engine_with_buckyball_and_closed_loop() -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let lat = "LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat).expect("parse LATTICE");
    execute(&mut engine, &stmt).expect("exec LATTICE");

    let gf = "GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(gf).expect("parse GAUGE_FIELD");
    execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");

    // Register a closed pentagonal loop on the buckyball.
    let loop_decl = "LOOP face0 ON buckyball FACE 0;";
    let stmt = parse(loop_decl).expect("parse LOOP face0");
    execute(&mut engine, &stmt).expect("exec LOOP face0");

    (engine, dir)
}

/// Source template — every required clause present, β_W reachable via
/// the ramp swept by the caller. The caller substitutes a single
/// `{loop_id}` / `{beta_start}` / `{ramp}` / `{sham_body}` to spin
/// each rejection case.
fn lt_src(loop_id: &str, beta_start: f64, ramp_beta_w: f64, sham_body: &str) -> String {
    format!(
        r#"
        LOOP_TRANSPORT buckyball
          ALONG_LOOP {loop_id}
          CONTROL_MANIFOLD (Q, BETA_WILSON)
          ADIABATIC TRUE
          RAMP_RATE_Q 0.04
          RAMP_RATE_BETA_W {ramp_beta_w}
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
          BETA_WILSON_START {beta_start}
          SEEDS [20260616..20260616]
          COMPUTE HOLONOMY_FORWARD
          {sham_body}
          RETURN H_forward;
        "#
    )
}

/// VI.2 parser-rejection — β_W = 2.0 falls below v3.1.3 §2 validated
/// regime [2.5, 3.0]; the parser refuses to lower before executor.
#[test]
fn halcyon_vi_2_rejects_beta_w_below_validated_regime() {
    let (mut engine, _dir) = engine_with_buckyball_and_closed_loop();
    let src = lt_src("face0", /* beta_start = */ 2.0, /* ramp = */ 0.0, "");
    let err = match parse(&src) {
        Ok(stmt) => execute(&mut engine, &stmt)
            .expect_err("β_W = 2.0 must be rejected before integration"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("BetaWilsonOutOfValidatedRegime"),
        "expected BetaWilsonOutOfValidatedRegime, got: {msg}"
    );
    assert!(msg.contains("2.5") && msg.contains("3.0"), "regime bounds must surface in {msg}");
}

/// VI.2 parser-rejection — β_W = 3.5 above v3.1.3 §2 validated regime
/// [2.5, 3.0]; symmetric to the below-regime case.
#[test]
fn halcyon_vi_2_rejects_beta_w_above_validated_regime() {
    let (mut engine, _dir) = engine_with_buckyball_and_closed_loop();
    let src = lt_src("face0", /* beta_start = */ 3.5, /* ramp = */ 0.0, "");
    let err = match parse(&src) {
        Ok(stmt) => execute(&mut engine, &stmt)
            .expect_err("β_W = 3.5 must be rejected before integration"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("BetaWilsonOutOfValidatedRegime"),
        "expected BetaWilsonOutOfValidatedRegime, got: {msg}"
    );
}

/// VI.2 parser-rejection — OPEN_LOOP audit-story flag. A loop whose
/// last vertex ≠ first vertex is rejected with `LoopNotClosed` before
/// the integrator starts (gate doc §SHAM table).
#[test]
fn halcyon_vi_2_rejects_open_loop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let lat = "LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat).expect("parse LATTICE");
    execute(&mut engine, &stmt).expect("exec LATTICE");

    let gf = "GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(gf).expect("parse GAUGE_FIELD");
    execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");

    // Register an OPEN path: edges (0->1), (1->2), (2->3) — head ≠ tail.
    let open = "LOOP open_path ON buckyball EDGES (0, 1, 2, 3);";
    let stmt = parse(open).expect("parse LOOP open_path");
    execute(&mut engine, &stmt).expect("exec LOOP open_path");

    let src = lt_src("open_path", 2.5, 0.0, "");
    let err = match parse(&src) {
        Ok(stmt) => execute(&mut engine, &stmt)
            .expect_err("open loop must be rejected with LoopNotClosed"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("LoopNotClosed"),
        "expected LoopNotClosed, got: {msg}"
    );
}

/// VI.2 parser-rejection — required clause missing. Omit
/// N_DISCRETIZATION; the parser returns a structured failure rather
/// than silently defaulting.
#[test]
fn halcyon_vi_2_rejects_missing_required_clause() {
    let src = r#"
        LOOP_TRANSPORT buckyball
          ALONG_LOOP face0
          CONTROL_MANIFOLD (Q, BETA_WILSON)
          ADIABATIC TRUE
          RAMP_RATE_Q 0.04 RAMP_RATE_BETA_W 0.01
          DRIVE_OMEGA 1.0 DRIVE_F0 0.01
          PIN_LAMBDA_Q 1.0 PIN_LAMBDA_BETA_W 1.0
          EPS_Q 0.05 EPS_BETA_W 0.05
          ALPHA_HALCYON 1.0 TAU_0 1.0 BETA_TAU 2.0
          MU_BASELINE 1.0 K_SPRING 1.0 C_DAMP 0.1
          BETA_WILSON_START 2.5
          SEEDS [20260616..20260616]
          COMPUTE HOLONOMY_FORWARD
          RETURN H_forward;
    "#;
    let err = parse(src).expect_err("missing N_DISCRETIZATION must be rejected at parse time");
    assert!(
        err.contains("N_DISCRETIZATION"),
        "parser error must name the missing clause; got: {err}"
    );
}

/// VI.2 parser-rejection — non-empty SHAM block carries a flag the
/// VI.2 executor cannot dispatch. Per gate doc Locked decisions, VI.2
/// PARSES the SHAM block (forward-compat for VI.4) but rejects any
/// flag with `UnrecognizedShamFlag`.
#[test]
fn halcyon_vi_2_rejects_unrecognized_sham_flag() {
    let (mut engine, _dir) = engine_with_buckyball_and_closed_loop();
    let sham_body = "SHAM { not_a_real_vi4_flag = TRUE }";
    let src = lt_src("face0", 2.5, 0.0, sham_body);
    let err = match parse(&src) {
        Ok(stmt) => execute(&mut engine, &stmt)
            .expect_err("SHAM with unknown flag must be rejected before integration"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("UnrecognizedShamFlag"),
        "expected UnrecognizedShamFlag, got: {msg}"
    );
    assert!(
        msg.contains("not_a_real_vi4_flag"),
        "rejection must name the offending flag; got: {msg}"
    );
}

/// VI.2 type-shape check — the error enum exposes the variants VI.3/4/5
/// will pattern-match against. Asserted via a constructor smoke so the
/// `pub enum LoopTransportError` surface is reachable.
#[test]
fn halcyon_vi_2_error_enum_variants_are_constructible() {
    let _below = LoopTransportError::BetaWilsonOutOfValidatedRegime {
        got: 2.0,
        range: (2.5, 3.0),
    };
    let _above = LoopTransportError::BetaWilsonOutOfValidatedRegime {
        got: 3.5,
        range: (2.5, 3.0),
    };
    let _flag = LoopTransportError::UnrecognizedShamFlag {
        name: "not_a_real_vi4_flag".into(),
    };
}
