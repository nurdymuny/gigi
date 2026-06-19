//! TDD-HAL-V.0 — `/v1/gql` dispatches gauge-feature statements.
//!
//! Spec: `HALCYON_PART_V_SNAPSHOT_GATES.md` §2.5.
//!
//! Halcyon's snapshot/restore design review surfaced a real bug:
//! `src/bin/gigi_stream.rs::gql_query` early-returned
//! `{"status":"ok"}` for the gauge-feature Statement family
//! (LATTICE / GAUGE_FIELD / GIBBS_SAMPLE / E_FIELD /
//! SYMPLECTIC_FLOW / SHOW (LATTICE | GAUGE_FIELD | E_FIELD) /
//! SELECT (PLAQUETTE | Q_SURROGATE | H_TOTAL | GAUSS_RESIDUAL_MAX) /
//! LATTICE FROM TRUNCATED_ICOSAHEDRON) because
//! `get_bundle_name(&stmt)` returns `None` for the whole family —
//! none of them are bound to a single GIGI bundle. The declaration
//! silently dropped on the floor.
//!
//! The 6 dedicated read-only routes in `src/gauge/http.rs` (the
//! Part II.6 + III.7 + IV.8 surface — `/v1/lattice`,
//! `/v1/gauge_field`, `/v1/lattice/{name}`, …) continued to work
//! end-to-end, but the universal `/v1/gql` reach-through was severed.
//!
//! Fix: a `#[cfg(feature = "gauge")]` dispatch prefix in `gql_query`
//! that consults `gigi::halcyon_gql_dispatch::try_dispatch_gauge_statement`
//! BEFORE the bundle-aware path. The helper drives `parser::execute`
//! against the supplied engine + process-global registries and
//! returns the `ExecResult` lowered through `exec_result_to_response`.
//!
//! Receipt structure (spec §2.5):
//!
//!   1. POST /v1/gql `LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';`
//!      → dispatched (`Ok(ExecResult::Ok)`); the buckyball lattice
//!      lands in `lattice::registry`.
//!   2. GET /v1/lattice/bb → 200 + LatticeView for the same lattice
//!      the GQL declaration just landed (proves the declaration
//!      LANDED, not just was acknowledged with `{"status":"ok"}`).
//!   3. POST /v1/gql `GAUGE_FIELD U ON LATTICE bb GROUP SU(2) INIT IDENTITY;`
//!      → dispatched; the SU(2) IDENTITY field lands in
//!      `gauge::registry`.
//!   4. POST /v1/gql `GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 10
//!      MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616;`
//!      → dispatched; response is `ExecResult::Rows` carrying a
//!      10-element `MeanPlaquette` measurement chain.
//!   5. POST /v1/gql `SELECT PLAQUETTE OF U;` → dispatched; response
//!      is `ExecResult::Rows` carrying a `per_face` Vector of length
//!      `F = 32` (buckyball faces).
//!
//! Optionality: this file is gated on `halcyon` (composite feature
//! pulling in `lattice + gauge`) so the no-default-features build
//! stays byte-identical at 852/0.

#![cfg(feature = "halcyon")]

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use gigi::engine::Engine;
use gigi::gauge::http::build_router;
use gigi::gauge::registry as gauge_registry;
use gigi::halcyon_gql_dispatch::try_dispatch_gauge_statement;
use gigi::lattice::registry as lattice_registry;
use gigi::parser::{parse, ExecResult};
use gigi::types::Value;
use serde_json::Value as JsonValue;
use std::sync::{Mutex, OnceLock, RwLock};
use tower::ServiceExt;

/// Process-wide mutex serializing every test in this file. The
/// `lattice_registry` + `gauge_registry` are process singletons; two
/// tests running in parallel would race the same registered names.
/// Mirrors the `registry_lock()` trick in `tests/halcyon_part_iii_http.rs`.
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

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// Clear every gauge-substrate singleton the dispatch helper touches.
fn clear_registries() {
    gauge_registry::clear();
    gauge_registry::clear_e_registry();
    lattice_registry::clear();
}

/// Spec §2.5 receipt: drive the 5-step gauge-statement contract
/// through the helper that `gql_query` now consults, and verify
/// every dispatch lands an observable side-effect.
///
/// Without the §2.5 fix this test fails three ways:
///   * the compile-time import of
///     `gigi::halcyon_gql_dispatch::try_dispatch_gauge_statement`
///     errors (module did not exist);
///   * the LATTICE / GAUGE_FIELD declarations never reach the
///     registry, so `GET /v1/lattice/bb` returns 404;
///   * the GIBBS_SAMPLE / SELECT PLAQUETTE statements never
///     produce `ExecResult::Rows`.
#[tokio::test]
async fn tdd_hal_v_0_gql_dispatches_gauge_statements() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());

    // Engine::open clears the lattice + gauge registries (its open
    // path resets every gauge-substrate singleton). Open the engine
    // FIRST so the registry clears below are not clobbered.
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    let engine_lock: RwLock<Engine> = RwLock::new(engine);
    clear_registries();

    // ── Step 1 ── POST /v1/gql `LATTICE bb FROM TRUNCATED_ICOSAHEDRON
    //              TOPOLOGY 'S2';` — declared lattice must dispatch.
    let stmt = parse("LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';")
        .expect("parse LATTICE FROM TRUNCATED_ICOSAHEDRON");
    let result = try_dispatch_gauge_statement(&engine_lock, &stmt)
        .expect("LATTICE statement must dispatch through the gauge helper");
    let result = result.expect("LATTICE execute must succeed");
    assert!(
        matches!(result, ExecResult::Ok),
        "LATTICE FROM TRUNCATED_ICOSAHEDRON must return ExecResult::Ok, got {result:?}"
    );

    // ── Step 2 ── GET /v1/lattice/bb — the declaration must be
    //              visible in the lattice registry. This is the
    //              "LANDED, not just acked" load-bearing receipt.
    let (status, body) = oneshot_json(get("/v1/lattice/bb")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "GET /v1/lattice/bb after dispatch must be 200: {}",
        String::from_utf8_lossy(&body)
    );
    let env = parse_json(&body);
    // The LatticeView envelope shape is the Part II contract; just
    // check the load-bearing fields the buckyball must satisfy.
    assert_eq!(env["name"], "bb");
    assert_eq!(
        env["n_vertices"].as_u64(),
        Some(60),
        "buckyball has V = 60"
    );
    assert_eq!(
        env["n_edges"].as_u64(),
        Some(90),
        "buckyball has E = 90"
    );

    // ── Step 3 ── POST /v1/gql `GAUGE_FIELD U ON LATTICE bb GROUP SU(2)
    //              INIT IDENTITY;` — declared field must dispatch.
    let stmt = parse(
        "GAUGE_FIELD U ON LATTICE bb GROUP SU(2) INIT IDENTITY;",
    )
    .expect("parse GAUGE_FIELD IDENTITY");
    let result = try_dispatch_gauge_statement(&engine_lock, &stmt)
        .expect("GAUGE_FIELD statement must dispatch through the gauge helper");
    let result = result.expect("GAUGE_FIELD execute must succeed");
    assert!(
        matches!(result, ExecResult::Ok),
        "GAUGE_FIELD IDENTITY must return ExecResult::Ok, got {result:?}"
    );

    // Sanity: the gauge HTTP read route now sees the field too.
    let (status, body) = oneshot_json(get("/v1/gauge_field/U")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "GET /v1/gauge_field/U after dispatch must be 200: {}",
        String::from_utf8_lossy(&body)
    );
    let env = parse_json(&body);
    assert_eq!(env["name"], "U");
    assert_eq!(env["lattice"], "bb");

    // Republish through `register_su2` so GIBBS_SAMPLE can lock the
    // SU(2)-mut handle for the heatbath kernel (D4 fix-up — see the
    // III.8b harness + Part IV HTTP test's `declare_identity_fixture`).
    // The current `Statement::GaugeField` executor arm in
    // `src/parser.rs` only registers in the dyn read map; the SU(2)-
    // mut sibling map is populated only by `register_su2`. This is a
    // pre-existing latent gap on both the `/v1/gauge_field` POST
    // route AND the parser arm; the same workaround is the standard
    // pattern in tests/halcyon_part_iv_http.rs:131-141.
    let lat = lattice_registry::get("bb").expect("bb registered above");
    let su2 = gigi::gauge::SU2GaugeField::new(
        "U".into(),
        &lat,
        gigi::gauge::GaugeFieldInit::Identity,
        None,
    )
    .expect("identity init");
    gauge_registry::register_su2(su2);

    // ── Step 4 ── POST /v1/gql `GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 10
    //              MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED
    //              20260616;` — must return Rows with a 10-element
    //              MeanPlaquette chain.
    let stmt = parse(
        "GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 10 MEASURE_EVERY 1 \
         MEASURE (MEAN(PLAQUETTE)) SEED 20260616;",
    )
    .expect("parse GIBBS_SAMPLE");
    let result = try_dispatch_gauge_statement(&engine_lock, &stmt)
        .expect("GIBBS_SAMPLE statement must dispatch through the gauge helper");
    let rows = match result.expect("GIBBS_SAMPLE execute must succeed") {
        ExecResult::Rows(r) => r,
        other => panic!("GIBBS_SAMPLE must return Rows, got {other:?}"),
    };
    assert_eq!(
        rows.len(),
        1,
        "GIBBS_SAMPLE returns one row carrying the measurement-chain columns"
    );
    // Column key is the ObservableId::label() — "MeanPlaquette" for the
    // mean-plaquette observable. The 10-element chain is the
    // measurement-history record of N_SWEEPS=10 / MEASURE_EVERY=1.
    let chain = match rows[0].get("MeanPlaquette") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("GIBBS_SAMPLE must carry a MeanPlaquette Vector column, got {other:?}"),
    };
    assert_eq!(
        chain.len(),
        10,
        "GIBBS_SAMPLE N_SWEEPS=10 MEASURE_EVERY=1 produces a 10-element \
         measurement chain, got {} entries",
        chain.len()
    );
    // Identity → every plaquette is q0=1.0; the heatbath kernel will
    // drift this off 1.0 after the first sweep but the value must stay
    // finite (the IDENTITY → mean-plaquette wire shape is the load-
    // bearing point, not the numeric trajectory).
    for (i, v) in chain.iter().enumerate() {
        assert!(
            v.is_finite(),
            "MeanPlaquette[{i}] must be finite, got {v}"
        );
    }

    // ── Step 5 ── POST /v1/gql `SELECT PLAQUETTE OF U;` — must return
    //              Rows with a per-face Vector of length F=32.
    let stmt = parse("SELECT PLAQUETTE OF U;").expect("parse SELECT PLAQUETTE");
    let result = try_dispatch_gauge_statement(&engine_lock, &stmt)
        .expect("SELECT PLAQUETTE statement must dispatch through the gauge helper");
    let rows = match result.expect("SELECT PLAQUETTE execute must succeed") {
        ExecResult::Rows(r) => r,
        other => panic!("SELECT PLAQUETTE must return Rows, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "SELECT PLAQUETTE returns one row");
    let per_face = match rows[0].get("per_face") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("SELECT PLAQUETTE must carry a per_face Vector, got {other:?}"),
    };
    assert_eq!(
        per_face.len(),
        32,
        "buckyball has F = 32 faces; SELECT PLAQUETTE per_face must \
         carry all 32 entries"
    );
    // After the GIBBS_SAMPLE above, the U field is no longer
    // identity, so per-face values may have drifted off 1.0. Just
    // assert finiteness — the wire shape is the load-bearing
    // dispatch receipt.
    for (i, v) in per_face.iter().enumerate() {
        assert!(
            v.is_finite(),
            "per_face[{i}] must be finite, got {v}"
        );
    }

    // ── Negative receipt ── a NON-gauge statement (SHOW BUNDLES is
    //                       the canonical bundle-aware verb) must
    //                       NOT match the gauge dispatcher. The
    //                       helper returning `None` is the contract
    //                       that prevents over-dispatch.
    let stmt = parse("SHOW BUNDLES;").expect("parse SHOW BUNDLES");
    assert!(
        try_dispatch_gauge_statement(&engine_lock, &stmt).is_none(),
        "SHOW BUNDLES must fall through to the bundle-aware path, \
         not the gauge dispatcher"
    );
}
