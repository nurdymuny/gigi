//! TDD-HAL-IV.8 — HTTP routes for the read-only Part IV verbs:
//! `SHOW E_FIELD`, `SELECT H_TOTAL OF (U, E)`, `SELECT
//! GAUSS_RESIDUAL_MAX OF (U, E)`, and `GET
//! /v1/symplectic_flow/diagnostics/{run_id}`.
//!
//! Mirrors the Part III HTTP harness (`tests/halcyon_part_iii_http.rs`):
//! tower `oneshot` against the in-process `build_router::<()>()` so the
//! suite never opens a TCP listener.
//!
//! Locked decisions wired through:
//!
//! - **IV-I** (E_FIELD embedded-only): the dedicated `POST /v1/e_field`
//!   route does **not** exist. Test (h) is the load-bearing IV-I
//!   route-table-absence receipt. The `parser::execute` path stays the
//!   single declarer of E fields.
//! - **IV.6** (SYMPLECTIC_FLOW embedded-only): the dedicated `POST
//!   /v1/gauge_field/{name}/symplectic_flow` route does **not** exist.
//!   Test (g) is the load-bearing receipt.
//! - **IV-H**: SHOW E_FIELD / SELECT H_TOTAL / SELECT GAUSS_RESIDUAL_MAX
//!   / GET /v1/symplectic_flow/diagnostics/{run_id} all HTTP-safe; this
//!   gate makes them addressable.
//! - **IV-J** (III.5 stub reversed): a positive `H_TOTAL` read at
//!   (IDENTITY, Zero) returns kinetic = 0.0 and a finite Wilson
//!   potential — no `PartIvObservableNotReady` error.
//!
//! Optionality: this file is gated on `halcyon` (composite feature
//! pulling in `lattice + gauge`) so the no-default-features build stays
//! byte-identical at 852/0.

#![cfg(feature = "halcyon")]

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use gigi::gauge::http::build_router;
use gigi::gauge::registry as gauge_registry;
use gigi::gauge::symplectic_flow as flow_mod;
use gigi::lattice::registry as lattice_registry;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;
use serde_json::{json, Value as JsonValue};
use std::sync::{Mutex, OnceLock};
use tower::ServiceExt;

/// Process-wide mutex serializing every test in this file. The lattice
/// + gauge + symplectic-flow caches are process singletons; two HTTP-
/// driving tests running in parallel would race the same registered
/// names + run_id keys. Matches the `registry_lock()` trick in
/// `tests/halcyon_part_iii_http.rs`.
fn registry_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

async fn oneshot_json(req: Request<Body>) -> (StatusCode, Vec<u8>) {
    let app: axum::Router = build_router::<()>();
    let resp = app.oneshot(req).await.expect("router oneshot");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .expect("collect body")
        .to_vec();
    (status, bytes)
}

fn parse_json(body: &[u8]) -> JsonValue {
    serde_json::from_slice(body).unwrap_or_else(|e| {
        panic!(
            "body is not JSON: {e}; body = {}",
            String::from_utf8_lossy(body)
        )
    })
}

/// Reset every Part-IV-relevant singleton to a clean slate.
fn clear_registries() {
    gauge_registry::clear();
    gauge_registry::clear_e_registry();
    lattice_registry::clear();
    flow_mod::clear_diagnostics_cache();
}

fn post_json(uri: &str, body: JsonValue) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// Declare buckyball lattice + IDENTITY SU(2) gauge field named `U`
/// over HTTP, then re-publish through `register_su2` so the
/// SU(2)-mut surface used by SYMPLECTIC_FLOW finds the field. Mirrors
/// the III.8b gold harness fix-up.
async fn declare_identity_fixture() {
    let (status, body) = oneshot_json(post_json(
        "/v1/lattice",
        json!({"name": "buckyball", "topology": "truncated_icosahedron"}),
    ))
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "fixture: lattice declaration failed: {}",
        String::from_utf8_lossy(&body)
    );
    let (status, body) = oneshot_json(post_json(
        "/v1/gauge_field",
        json!({
            "name": "U",
            "lattice": "buckyball",
            "group": "SU(2)",
            "init": {"kind": "identity"},
        }),
    ))
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "fixture: gauge field declaration failed: {}",
        String::from_utf8_lossy(&body)
    );
    // Republish through `register_su2` so symplectic_flow can lock the
    // SU(2)-mut handle (D4 fix-up — see III.8b harness).
    let lat = lattice_registry::get("buckyball").expect("lattice declared above");
    let su2 = gigi::gauge::SU2GaugeField::new(
        "U".into(),
        &lat,
        gigi::gauge::GaugeFieldInit::Identity,
        None,
    )
    .expect("identity init");
    gauge_registry::register_su2(su2);
}

/// Declare an `E_FIELD E ON GAUGE_FIELD U INIT ZERO;` through the
/// embedded parser+executor path — there is NO HTTP route for E_FIELD
/// declaration (locked decision IV-I). Owns its own temp engine so the
/// tests that don't otherwise need an engine still get a clean
/// `parser::execute` surface.
fn declare_zero_e_field_embedded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    declare_zero_e_field_embedded_with_engine(&mut engine);
}

/// Same as `declare_zero_e_field_embedded` but reuses an already-open
/// engine so the diagnostics test can chain `Engine::open` (which
/// clears the registries) → `declare_identity_fixture` (HTTP) →
/// E_FIELD ZERO → SYMPLECTIC_FLOW in the right order. Without this
/// the implicit `Engine::open` inside the no-arg helper would clobber
/// the just-declared U.
fn declare_zero_e_field_embedded_with_engine(engine: &mut gigi::engine::Engine) {
    let stmt =
        parse("E_FIELD E ON GAUGE_FIELD U INIT ZERO;").expect("parse E_FIELD ZERO");
    execute(engine, &stmt).expect("exec E_FIELD ZERO");
}

/// TDD-HAL-IV.8: GET /v1/e_field/{name}?with_buffer=true returns the per-edge Lie buffer + SHOW E_FIELD WITH BUFFER metadata.
#[tokio::test]
async fn tdd_hal_iv_8_e_field_get_buffer() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;
    declare_zero_e_field_embedded();

    let (status, body) = oneshot_json(get("/v1/e_field/E?with_buffer=true")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "e_field body: {}",
        String::from_utf8_lossy(&body)
    );
    let env = parse_json(&body);
    assert_eq!(env["name"], "E");
    assert_eq!(env["source_gauge_field"], "U");
    assert_eq!(env["init_kind"], "zero");
    assert_eq!(env["group"], "SU(2)");
    assert_eq!(env["repr_dim"].as_u64(), Some(4));
    assert_eq!(env["n_edges"].as_u64(), Some(90));
    let buffer = env["buffer"]
        .as_array()
        .expect("buffer must be array");
    assert_eq!(buffer.len(), 90);
    for (i, row) in buffer.iter().enumerate() {
        let r = row.as_array().expect("row must be array");
        assert_eq!(r.len(), 4, "row {i} not 4-wide");
        for (j, v) in r.iter().enumerate() {
            let f = v.as_f64().expect("not f64");
            assert_eq!(f, 0.0, "row {i} col {j} expected 0.0, got {f}");
        }
    }
}

/// TDD-HAL-IV.8: `GET /v1/gauge_field/U/h_total?e_field=E` on
/// (IDENTITY, Zero) returns kinetic = 0 and a finite Wilson potential
/// (no PartIvObservableNotReady — locked decision IV-J).
#[tokio::test]
async fn tdd_hal_iv_8_h_total_get_at_identity_zero_e() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;
    declare_zero_e_field_embedded();

    let (status, body) =
        oneshot_json(get("/v1/gauge_field/U/h_total?e_field=E")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "h_total body: {}",
        String::from_utf8_lossy(&body)
    );
    let env = parse_json(&body);
    let kinetic = env["kinetic"].as_f64().expect("kinetic not f64");
    let potential = env["potential"].as_f64().expect("potential not f64");
    let h_total = env["h_total"].as_f64().expect("h_total not f64");
    assert_eq!(kinetic, 0.0, "E=Zero ⇒ kinetic = 0 exactly");
    // IDENTITY field ⇒ every face holonomy is q0 = 1.0; Wilson
    // potential `(F/g²)·(1 - ⟨P⟩) = (F/g²)·(1 - 1) = 0`. Both
    // kinetic and potential are exactly zero at (IDENTITY, Zero).
    assert_eq!(
        potential, 0.0,
        "IDENTITY ⇒ ⟨P⟩ = 1 ⇒ Wilson potential = 0 exactly"
    );
    assert_eq!(h_total, 0.0, "h_total = kinetic + potential = 0");
}

/// TDD-HAL-IV.8: `GET /v1/gauge_field/U/gauss_residual_max?e_field=E`
/// at (IDENTITY, Zero) returns the covariant reduction by default,
/// max residual = 0.
#[tokio::test]
async fn tdd_hal_iv_8_gauss_residual_get_covariant() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;
    declare_zero_e_field_embedded();

    let (status, body) =
        oneshot_json(get("/v1/gauge_field/U/gauss_residual_max?e_field=E")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "gauss_residual body: {}",
        String::from_utf8_lossy(&body)
    );
    let env = parse_json(&body);
    assert_eq!(env["reduction"], "covariant");
    let max = env["gauss_residual_max"]
        .as_f64()
        .expect("gauss_residual_max not f64");
    assert_eq!(max, 0.0, "E=Zero on IDENTITY ⇒ covariant residual = 0");
}

/// TDD-HAL-IV.8: `?reduction=flat` query param returns the flat
/// reduction; at U=IDENTITY (Ad(I) = I) the flat and covariant
/// reductions agree (both 0).
#[tokio::test]
async fn tdd_hal_iv_8_gauss_residual_get_flat_optional() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;
    declare_zero_e_field_embedded();

    let (status, body) = oneshot_json(get(
        "/v1/gauge_field/U/gauss_residual_max?e_field=E&reduction=flat",
    ))
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "gauss_residual flat body: {}",
        String::from_utf8_lossy(&body)
    );
    let env = parse_json(&body);
    assert_eq!(env["reduction"], "flat");
    let max = env["gauss_residual_max"]
        .as_f64()
        .expect("gauss_residual_max not f64");
    assert_eq!(max, 0.0, "Zero E ⇒ flat residual = 0");
}

/// TDD-HAL-IV.8: a SYMPLECTIC_FLOW run kicked off through the embedded
/// parser+executor path lands its diagnostics in the process-local LRU
/// under `run_id`. `GET /v1/symplectic_flow/diagnostics/{run_id}` reads
/// the full response back.
#[tokio::test]
async fn tdd_hal_iv_8_diagnostics_get() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();

    // `Engine::open` clears the lattice + gauge registries, so we open
    // the engine FIRST and then declare U + E inside its lifetime.
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    declare_identity_fixture().await;
    declare_zero_e_field_embedded_with_engine(&mut engine);

    // Run a tiny flow through the embedded path (3 KDK steps so the
    // test is cheap; project_gauss is OFF because the buckyball
    // covariant residual is already 0 at (IDENTITY, Zero) and the
    // projector would no-op).
    let stmt = parse(
        "SYMPLECTIC_FLOW U FROM (U=U, E=E) \
         BETA 2.5 DT 0.05 N_STEPS 3;",
    )
    .expect("parse SYMPLECTIC_FLOW");
    let rows = match execute(&mut engine, &stmt).expect("exec SYMPLECTIC_FLOW") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    let run_id = match rows[0].get("run_id") {
        Some(Value::Text(s)) => s.clone(),
        other => panic!("missing/wrong run_id column: {other:?}"),
    };

    let (status, body) = oneshot_json(get(&format!(
        "/v1/symplectic_flow/diagnostics/{run_id}"
    )))
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "diagnostics body: {}",
        String::from_utf8_lossy(&body)
    );
    let env = parse_json(&body);
    assert_eq!(env["run_id"], run_id);
    assert_eq!(env["field"], "U");
    assert_eq!(env["e_field"], "E");
    let diag = &env["diagnostics"];
    assert_eq!(diag["beta"].as_f64(), Some(2.5));
    assert_eq!(diag["dt"].as_f64(), Some(0.05));
    assert_eq!(diag["n_steps_completed"].as_u64(), Some(3));
    // cg_iterations_per_step_p99 must be present (DIAGNOSTIC ONLY —
    // never compared in A2 rows, but always serialized).
    assert!(
        diag["cg_iterations_per_step_p99"].is_number(),
        "diagnostics must carry cg_iterations_per_step_p99"
    );
    assert!(diag["max_energy_drift_rel"].is_number());
    assert!(diag["gauss_residual_max"].is_number());
}

/// TDD-HAL-IV.8: unknown run_id returns 404 + flat error envelope.
#[tokio::test]
async fn tdd_hal_iv_8_diagnostics_404_on_unknown_run_id() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();

    let (status, body) = oneshot_json(get(
        "/v1/symplectic_flow/diagnostics/nonexistent-run-id",
    ))
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let env = parse_json(&body);
    let err = env["error"].as_str().expect("error must be a string");
    assert!(
        err.contains("nonexistent-run-id") || err.contains("not found"),
        "error must mention the missing run_id or 'not found': got {err}"
    );
}

/// TDD-HAL-IV.8: `POST /v1/gauge_field/U/symplectic_flow` returns 404.
/// **LOAD-BEARING D5+IV-I receipt** — SYMPLECTIC_FLOW is reachable only
/// through `parser::execute` (the `/v1/gql` POST), never as a dedicated
/// HTTP verb. This is the route-table-absence assertion.
#[tokio::test]
async fn tdd_hal_iv_8_no_dedicated_symplectic_flow_route() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;
    declare_zero_e_field_embedded();

    let (status, _body) = oneshot_json(post_json(
        "/v1/gauge_field/U/symplectic_flow",
        json!({
            "e_field": "E", "beta": 2.5, "dt": 0.05, "n_steps": 3,
            "seed": 20260616,
        }),
    ))
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "POST /v1/gauge_field/U/symplectic_flow must NOT be a \
         registered route (IV.6 + locked decision)"
    );
}

/// TDD-HAL-IV.8: `POST /v1/e_field` returns 404. **Load-bearing
/// locked-decision IV-I receipt** — E_FIELD declaration is reachable
/// only through `parser::execute`, never as a dedicated HTTP verb.
#[tokio::test]
async fn tdd_hal_iv_8_no_dedicated_e_field_declare_route() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;

    let (status, _body) = oneshot_json(post_json(
        "/v1/e_field",
        json!({
            "name": "E",
            "source_gauge_field": "U",
            "init": {"kind": "zero"},
        }),
    ))
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "POST /v1/e_field must NOT be a registered route (locked \
         decision IV-I — E_FIELD embedded-only)"
    );
}
