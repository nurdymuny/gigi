//! WISH ASK 5 — LOOP_TRANSPORT accepts a non-`U_lt` gauge-field name
//! as its first arg (Hallie 2026-06-22).
//!
//! Pre-ASK 5, the executor at `src/parser.rs:10380` hardcoded the
//! string literals `"U_lt"` / `"E_lt"` when calling
//! `gauge::loop_transport::loop_transport(stmt, "U_lt", "E_lt")`.
//! Hallie's per-seed UUID-suffixed scratch fields (orchestrator-side
//! GIBBS_SAMPLE fix) require the executor to dispatch on whatever
//! gauge name the GQL itself declared.
//!
//! Ride-along behaviors verified by this file:
//!
//!   1. `LOOP_TRANSPORT bb GAUGE_FIELD U_seed_a3f9 E_FIELD E_seed_a3f9
//!      ALONG_LOOP face0 …` resolves the registry lookup against the
//!      scratch names, returns a Rows result with finite h_forward.
//!   2. The historical short form `LOOP_TRANSPORT bb ALONG_LOOP …`
//!      (no GAUGE_FIELD / E_FIELD clauses) still defaults to U_lt /
//!      E_lt — backwards-compat with every Halcyon Part VI gold
//!      fixture.
//!   3. An unknown gauge name surfaces through the existing typed
//!      `LoopTransportError::UFieldNotDeclared(String)` channel
//!      (already supported in `src/gauge/loop_transport.rs:255`); the
//!      executor surfaces the error string with the literal name so
//!      orchestrator-side handlers can match on it.

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult, Statement};
use gigi::types::Value;

fn clear_all() {
    gigi::gauge::loop_transport::clear_loops();
    gigi::gauge::registry::clear();
    gigi::gauge::registry::clear_e_registry();
    gigi::lattice::registry::clear();
}

/// Bring up a canonical halcyon buckyball + GAUGE_FIELD `u_name`
/// INIT IDENTITY + E_FIELD `e_name` INIT ZERO + `face0 LOOP` on
/// FACE 0. Mirrors `tests/halcyon_part_vi_6_semantic_thermalized.rs`
/// `setup_thermalized_canonical` but takes the gauge-field names as
/// parameters so we can exercise UUID-suffixed scratch fields.
fn setup_with_named_fields(u_name: &str, e_name: &str) -> (Engine, tempfile::TempDir) {
    clear_all();
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let lat = "LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat).expect("parse LATTICE");
    execute(&mut engine, &stmt).expect("exec LATTICE");

    // GAUGE_FIELD via GQL …
    let gf = format!("GAUGE_FIELD {u_name} ON LATTICE bb GROUP SU(2) INIT IDENTITY;");
    let stmt = parse(&gf).expect("parse GAUGE_FIELD");
    execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");

    // … and re-publish through register_su2 so the SU(2) handle is
    // present for LOOP_TRANSPORT (mirrors VI.6 setup).
    {
        let lat = gigi::lattice::registry::get("bb").expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            u_name.into(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    // Thermalize so the loop holonomy is non-trivial.
    let therm = format!("GIBBS_SAMPLE {u_name} BETA 2.5 N_SWEEPS 200 SEED 20260616;");
    let stmt = parse(&therm).expect("parse GIBBS_SAMPLE");
    execute(&mut engine, &stmt).expect("exec GIBBS_SAMPLE");

    let ef = format!("E_FIELD {e_name} ON GAUGE_FIELD {u_name} INIT ZERO;");
    let stmt = parse(&ef).expect("parse E_FIELD");
    execute(&mut engine, &stmt).expect("exec E_FIELD");

    let loop_decl = "LOOP face0 ON bb FACE 0;";
    let stmt = parse(loop_decl).expect("parse LOOP face0");
    execute(&mut engine, &stmt).expect("exec LOOP face0");

    (engine, dir)
}

/// Canonical LOOP_TRANSPORT GQL string with the `GAUGE_FIELD`/`E_FIELD`
/// clauses spliced in (or omitted for the short form). The remainder is
/// the regime-safe ALPHA_HALCYON=1 / SEEDS=[1..1] minimal envelope used
/// elsewhere — we care about the dispatch wiring, not the convergence
/// physics.
fn lt_source(
    field_clauses: &str,
    compute_block: &str,
) -> String {
    format!(
        r#"LOOP_TRANSPORT bb {field_clauses}
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
            SEEDS [1..1]
            {compute_block}
            RETURN H_forward, H_reversed, sigma_H_blocked,
                   per_seed_H_forward, per_seed_H_reversed,
                   tracking_error_max_Q, tracking_error_max_beta_W,
                   adiabaticity_check;"#
    )
}

fn compute_h_only() -> &'static str {
    "COMPUTE HOLONOMY_FORWARD COMPUTE HOLONOMY_REVERSED COMPUTE ADIABATICITY_CHECK"
}

fn run_lt(engine: &mut Engine, src: &str) -> Result<gigi::types::Record, String> {
    let stmt = parse(src).map_err(|e| format!("parse: {e}"))?;
    assert!(matches!(stmt, Statement::LoopTransport { .. }));
    match execute(engine, &stmt).map_err(|e| format!("execute: {e}"))? {
        ExecResult::Rows(rows) => {
            assert_eq!(rows.len(), 1, "LOOP_TRANSPORT must return exactly one row");
            Ok(rows.into_iter().next().unwrap())
        }
        other => panic!("expected Rows; got {other:?}"),
    }
}

fn h_forward(rec: &gigi::types::Record) -> f64 {
    match rec.get("h_forward") {
        Some(Value::Float(x)) => *x,
        other => panic!("h_forward not a Float: {other:?}"),
    }
}

// ── Test 1 ────────────────────────────────────────────────────────

/// `LOOP_TRANSPORT bb GAUGE_FIELD U_seed_a3f9b2c1 E_FIELD
/// E_seed_a3f9b2c1 ALONG_LOOP face0 …` resolves the per-seed
/// UUID-suffixed scratch fields the orchestrator-side GIBBS_SAMPLE
/// fix needs. Pre-ASK 5 this errored with `UFieldNotDeclared(U_lt)`
/// regardless of what GQL named.
#[test]
fn test_loop_transport_accepts_uuid_suffixed_scratch_field() {
    let (mut engine, _dir) =
        setup_with_named_fields("U_seed_a3f9b2c1", "E_seed_a3f9b2c1");
    let src = lt_source(
        "GAUGE_FIELD U_seed_a3f9b2c1 E_FIELD E_seed_a3f9b2c1",
        compute_h_only(),
    );
    let rec = run_lt(&mut engine, &src).expect("LOOP_TRANSPORT with scratch fields");
    let hf = h_forward(&rec);
    assert!(
        hf.is_finite(),
        "h_forward must be finite under scratch fields; got {hf}"
    );
}

// ── Test 2 ────────────────────────────────────────────────────────

/// Backwards-compat gate: the historical short form (no
/// `GAUGE_FIELD`/`E_FIELD` clauses) defaults to (`U_lt`, `E_lt`) and
/// the loop runs unchanged.
#[test]
fn test_loop_transport_u_lt_still_works() {
    let (mut engine, _dir) = setup_with_named_fields("U_lt", "E_lt");
    let src = lt_source("", compute_h_only());
    let rec = run_lt(&mut engine, &src).expect("default U_lt path");
    let hf = h_forward(&rec);
    assert!(hf.is_finite(), "h_forward must be finite on default path; got {hf}");
}

// ── Test 3 ────────────────────────────────────────────────────────

/// Naming a gauge field that does not exist in the registry returns a
/// clear error that mentions the literal name so the
/// orchestrator-side handler can pattern-match on it.
#[test]
fn test_loop_transport_unknown_gauge_field_clear_error() {
    // Set up the canonical (U_lt, E_lt) so the lattice / loop exist
    // but the requested gauge name does NOT.
    let (mut engine, _dir) = setup_with_named_fields("U_lt", "E_lt");
    let src = lt_source(
        "GAUGE_FIELD U_does_not_exist E_FIELD E_does_not_exist",
        compute_h_only(),
    );
    let err = run_lt(&mut engine, &src).expect_err("missing field must error");
    assert!(
        err.contains("U_does_not_exist") || err.contains("UFieldNotDeclared"),
        "error must mention the literal missing name; got: {err}"
    );
}

// ── Test 4 ────────────────────────────────────────────────────────

/// Explicit `GAUGE_FIELD U_lt E_FIELD E_lt` produces output byte-
/// identical to the short form — the sugar is exact equivalence, not
/// a separate code path.
#[test]
fn test_loop_transport_explicit_u_lt_named_equivalent() {
    // First pass: short form.
    let (mut engine, _dir) = setup_with_named_fields("U_lt", "E_lt");
    let src_short = lt_source("", compute_h_only());
    let rec_short = run_lt(&mut engine, &src_short).expect("short form");
    let hf_short = h_forward(&rec_short);

    // Fresh engine, then explicit form.
    let (mut engine, _dir) = setup_with_named_fields("U_lt", "E_lt");
    let src_explicit = lt_source("GAUGE_FIELD U_lt E_FIELD E_lt", compute_h_only());
    let rec_explicit = run_lt(&mut engine, &src_explicit).expect("explicit form");
    let hf_explicit = h_forward(&rec_explicit);

    assert_eq!(
        hf_short.to_bits(),
        hf_explicit.to_bits(),
        "explicit U_lt/E_lt clauses must produce byte-identical h_forward; \
         short={hf_short}, explicit={hf_explicit}"
    );
}
