//! VI.2b — HTTP /v1/gql dispatch for `LoopDecl` + `LoopTransport`.
//!
//! Regression guard. VI.2 (commit `777c7ad`) shipped the
//! `Statement::LoopTransport` executor arm with the full
//! `ExecResult::Rows([diagnostics])` envelope, but the HTTP
//! `gigi-stream` binary routes gauge-feature statements through
//! `halcyon_gql_dispatch::try_dispatch_gauge_statement` (per the Part V
//! P-1 drop-bug fix at `src/bin/gigi_stream.rs:12136-12146`). The
//! VI.2 ship omitted `LoopDecl` + `LoopTransport` from that
//! dispatcher's match list, so /v1/gql for `LOOP_TRANSPORT` returned
//! the default `{"status":"ok"}` envelope — the executor arm never
//! fired.
//!
//! This test asserts the dispatcher recognizes both statements after
//! VI.2b. Without the fix, `try_dispatch_gauge_statement` returns
//! `None` and the HTTP layer drops the statement on the floor.

#![cfg(all(feature = "gauge", feature = "halcyon"))]

use std::sync::RwLock;

use gigi::engine::Engine;
use gigi::halcyon_gql_dispatch::try_dispatch_gauge_statement;
use gigi::parser::{parse, ExecResult};

fn setup_engine() -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    (engine, dir)
}

/// VI.2b core: `LoopDecl` statements route through the gauge
/// dispatcher (return Some), not the bundle-aware path (would return
/// None and drop on the floor).
#[test]
fn vi_2b_loop_decl_routes_through_gauge_dispatcher() {
    let (engine, _dir) = setup_engine();
    let engine_lock = RwLock::new(engine);

    // First declare the lattice the loop references; the
    // try_dispatch path needs every statement to round-trip cleanly
    // (declare lattice → declare loop). Declare via the dispatcher
    // so we exercise the same path the HTTP server uses.
    let lat = "LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat).expect("parse LATTICE");
    let res = try_dispatch_gauge_statement(&engine_lock, &stmt);
    assert!(
        res.is_some(),
        "LATTICE FROM canonical must dispatch through gauge layer"
    );
    res.unwrap().expect("LATTICE FROM exec ok");

    // LOOP face0 ON buckyball FACE 0; — the VI.2-grammar LOOP
    // declaration. Without the VI.2b fix, this returns None and
    // the HTTP layer emits {"status":"ok"} silently.
    let loop_src = "LOOP face0 ON buckyball FACE 0;";
    let stmt = parse(loop_src).expect("parse LOOP face0");
    let res = try_dispatch_gauge_statement(&engine_lock, &stmt);
    assert!(
        res.is_some(),
        "LOOP declaration must dispatch through gauge layer; \
         got None — Statement::LoopDecl missing from gauge \
         dispatcher's match arm (VI.2b regression)"
    );
    res.unwrap().expect("LOOP face0 exec ok");
}

/// VI.2b core: `LoopTransport` statements route through the gauge
/// dispatcher (return Some), so the executor arm at parser.rs:10338
/// fires + emits the full LoopTransportDiagnostics Row envelope.
///
/// Without VI.2b, the HTTP layer returns {"status":"ok"} and Halcyon's
/// `LiveLoopTransportClient` reads no diagnostics — exactly what
/// Halcyon hit when running run_holonomy_battery.py at the live
/// binding on 2026-06-21.
#[test]
fn vi_2b_loop_transport_routes_through_gauge_dispatcher() {
    let (engine, _dir) = setup_engine();
    let engine_lock = RwLock::new(engine);

    // Precondition: lattice + U_lt gauge field + E_lt + face0 loop
    // (the executor-arm convention from parser.rs:10338 hardcodes
    // the U_lt / E_lt naming).
    for stmt_src in [
        "LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';",
        "GAUGE_FIELD U_lt ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;",
        "E_FIELD E_lt ON GAUGE_FIELD U_lt INIT ZERO;",
        "LOOP face0 ON buckyball FACE 0;",
    ] {
        let stmt = parse(stmt_src).expect("parse precondition");
        let res = try_dispatch_gauge_statement(&engine_lock, &stmt);
        assert!(
            res.is_some(),
            "precondition statement must dispatch through gauge layer: {stmt_src}"
        );
        res.unwrap().expect("precondition exec ok");
    }

    // Small-N LOOP_TRANSPORT (N=100, 1 seed) — same shape the VI.2
    // executor_smoke test uses, but routed through the HTTP
    // dispatcher path instead of the direct function call.
    let lt_src = r#"LOOP_TRANSPORT buckyball
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
        SEEDS [20260616..20260616]
        COMPUTE HOLONOMY_FORWARD
        COMPUTE HOLONOMY_REVERSED
        COMPUTE ADIABATICITY_CHECK
        RETURN H_FORWARD, H_REVERSED, SIGMA_H_BLOCKED,
               PER_SEED_H_FORWARD, PER_SEED_H_REVERSED,
               TRACKING_ERROR_MAX_Q, TRACKING_ERROR_MAX_BETA_W,
               ADIABATICITY_CHECK;"#;

    let stmt = parse(lt_src).expect("parse LOOP_TRANSPORT");
    let res = try_dispatch_gauge_statement(&engine_lock, &stmt);
    assert!(
        res.is_some(),
        "LOOP_TRANSPORT must dispatch through gauge layer; \
         got None — Statement::LoopTransport missing from gauge \
         dispatcher's match arm (VI.2b regression — HTTP would \
         return empty status:ok)"
    );

    // The executor arm returns ExecResult::Rows with all 9 fields.
    // Verify the dispatcher surfaces it.
    let exec_res = res.unwrap().expect("LOOP_TRANSPORT exec ok");
    match exec_res {
        ExecResult::Rows(rows) => {
            assert_eq!(rows.len(), 1, "LOOP_TRANSPORT returns exactly one row");
            let row = &rows[0];
            // The 9 fields VI.2's executor arm builds (parser.rs:10338-10401)
            for field in [
                "h_forward",
                "h_reversed",
                "sigma_h_blocked",
                "per_seed_h_forward",
                "per_seed_h_reversed",
                "tracking_error_max_q",
                "tracking_error_max_beta_w",
                "adiabaticity_verdict",
                "adiabaticity_ratio",
                "n_substeps_completed",
            ] {
                assert!(
                    row.contains_key(field),
                    "LOOP_TRANSPORT response missing field '{field}'; \
                     row keys: {:?}",
                    row.keys().collect::<Vec<_>>()
                );
            }
        }
        other => panic!(
            "LOOP_TRANSPORT must return ExecResult::Rows (the executor arm \
             at parser.rs:10401 emits this); got {other:?} — VI.2b regression"
        ),
    }
}
