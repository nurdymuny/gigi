//! TDD-HAL-III.7 — HTTP routes for the read-only Part III verbs:
//! `PLAQUETTE`, `Q_SURROGATE`, and the batched `observables` endpoint.
//! **No** dedicated route for `GIBBS_SAMPLE` (locked decision D5).
//!
//! Mirrors the Part II HTTP harness (`tests/halcyon_part_ii_http.rs`):
//! tower `oneshot` against the in-process `build_router::<()>()` so the
//! suite never opens a TCP listener.
//!
//! Locked decisions wired through:
//!
//! - D5 (`/v1/gql` soft-edge): the dedicated `POST /v1/gauge_field/{name}/
//!   gibbs_sample` route does **not** exist; test (f) is the load-bearing
//!   route-table-absence receipt. The `/v1/gql` POST endpoint still
//!   reaches GIBBS_SAMPLE via `parser::execute` — that's by design; the
//!   46-minute wall self-enforces.
//! - D6 (`Q_SURROGATE` shape): scalar `f64` under `value`. Mirrors the
//!   Halcyon mock byte-for-byte at the JSON level.
//! - D7 (`PLAQUETTE` shape): `per_face` returns `Vec<f64>` of length
//!   `F = 32` (q0 column only); `mean` / `sum` return scalar `f64` under
//!   `value`. Mirrors mock + spec example shapes.
//! - Optionality: this file is gated on `halcyon` (composite feature
//!   pulling in `lattice + gauge`) so the no-default-features build stays
//!   byte-identical at 852/0.

#![cfg(feature = "halcyon")]

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use gigi::gauge::http::build_router;
use gigi::gauge::registry as gauge_registry;
use gigi::lattice::registry as lattice_registry;
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use tower::ServiceExt;

/// Process-wide mutex serializing every test in this file. The lattice
/// + gauge registries are process singletons; two HTTP-driving tests
/// running in parallel would race the same registered names. Matches
/// the `registry_lock()` trick in `tests/halcyon_part_ii_http.rs`.
fn registry_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Drive a single request through the in-process router; collect the
/// response status + body bytes.
async fn oneshot_json(req: Request<Body>) -> (StatusCode, Vec<u8>) {
    let app: axum::Router = build_router::<()>();
    let resp = app
        .oneshot(req)
        .await
        .expect("router oneshot");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .expect("collect body")
        .to_vec();
    (status, bytes)
}

fn parse_json(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or_else(|e| {
        panic!(
            "body is not JSON: {e}; body = {}",
            String::from_utf8_lossy(body)
        )
    })
}

/// Reset the singleton registries to a clean slate. Mirrors the II.6
/// HTTP harness.
fn clear_registries() {
    gauge_registry::clear();
    lattice_registry::clear();
}

/// Build a `POST <uri>` request with a JSON body.
fn post_json(uri: &str, body: Value) -> Request<Body> {
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

/// Declare buckyball lattice + IDENTITY SU(2) gauge field named `U`.
/// Returns once both registrations land 200 OK; panics on any other
/// status so a misconfigured fixture fails loudly at the test
/// boundary, not later in the assertion.
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
}

/// TDD-HAL-III.7: `GET /v1/gauge_field/U/plaquette?reduction=per_face`
/// returns the per-face q0 column on the IDENTITY field — 32 ones,
/// FP64-exact (locked decision D7).
#[tokio::test]
async fn tdd_hal_iii_7_plaquette_get_per_face() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;

    let (status, body) =
        oneshot_json(get("/v1/gauge_field/U/plaquette?reduction=per_face")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "per_face body: {}",
        String::from_utf8_lossy(&body)
    );
    let envelope = parse_json(&body);
    assert_eq!(envelope["reduction"], "per_face");
    let values = envelope["values"]
        .as_array()
        .expect("`values` must be an array");
    assert_eq!(values.len(), 32, "buckyball has F=32 faces");
    for (i, q) in values.iter().enumerate() {
        let q = q.as_f64().expect("value not f64");
        // Identity quaternion product is FP64-exact 1.0 per face.
        assert_eq!(q, 1.0, "face {i}: expected 1.0 exactly, got {q}");
    }
}

/// TDD-HAL-III.7: `GET /v1/gauge_field/U/plaquette?reduction=mean`
/// returns the scalar mean — 1.0 on IDENTITY (locked decision D7).
#[tokio::test]
async fn tdd_hal_iii_7_plaquette_get_mean() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;

    let (status, body) =
        oneshot_json(get("/v1/gauge_field/U/plaquette?reduction=mean")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "mean body: {}",
        String::from_utf8_lossy(&body)
    );
    let envelope = parse_json(&body);
    assert_eq!(envelope["reduction"], "mean");
    assert_eq!(
        envelope["value"].as_f64().expect("value not f64"),
        1.0,
        "identity mean must be 1.0 exactly"
    );
}

/// TDD-HAL-III.7: `GET /v1/gauge_field/U/observables/q_surrogate`
/// returns the scalar Q_surrogate — 0.0 on IDENTITY (every face
/// holonomy is `q0 = 1`, `arccos(1) = 0`; locked decision D6).
#[tokio::test]
async fn tdd_hal_iii_7_q_surrogate_get() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;

    let (status, body) =
        oneshot_json(get("/v1/gauge_field/U/observables/q_surrogate")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "q_surrogate body: {}",
        String::from_utf8_lossy(&body)
    );
    let envelope = parse_json(&body);
    let v = envelope["value"].as_f64().expect("value not f64");
    assert!(
        v.abs() < 1e-12,
        "identity Q_surrogate must be ≈ 0, got {v}"
    );
}

/// TDD-HAL-III.7: `POST /v1/gauge_field/U/observables` with a list
/// of observable identifiers returns each as a JSON key. Read-only
/// despite the verb (II.6c clarification: POST without side effect
/// is the consumer-safe batched-read pattern).
#[tokio::test]
async fn tdd_hal_iii_7_observables_batched_post() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;

    let (status, body) = oneshot_json(post_json(
        "/v1/gauge_field/U/observables",
        json!({"observables": ["mean_plaquette", "q_surrogate"]}),
    ))
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "observables body: {}",
        String::from_utf8_lossy(&body)
    );
    let envelope = parse_json(&body);
    assert_eq!(
        envelope["mean_plaquette"].as_f64().expect("mean_plaquette not f64"),
        1.0
    );
    let q = envelope["q_surrogate"].as_f64().expect("q_surrogate not f64");
    assert!(q.abs() < 1e-12, "identity Q_surrogate must be ≈ 0, got {q}");
}

/// TDD-HAL-III.7: `GET /v1/gauge_field/{undeclared}/plaquette`
/// returns 400 + flat `{"error": "...not declared..."}` envelope,
/// matching the Part II typed-error pattern.
#[tokio::test]
async fn tdd_hal_iii_7_plaquette_404_when_field_undeclared() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();

    let (status, body) =
        oneshot_json(get("/v1/gauge_field/nonexistent/plaquette?reduction=mean")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let envelope = parse_json(&body);
    let err = envelope["error"]
        .as_str()
        .expect("error must be a string");
    assert!(
        err.contains("not declared"),
        "error must mention 'not declared': got {err}"
    );
}

/// TDD-HAL-III.7: `POST /v1/gauge_field/U/gibbs_sample` (any body)
/// returns 404 — **the dedicated GIBBS_SAMPLE route does not exist.**
/// Load-bearing D5 enforcement receipt: GIBBS_SAMPLE is reachable
/// only through `/v1/gql` (parser::execute), never as a dedicated
/// HTTP verb.
#[tokio::test]
async fn tdd_hal_iii_7_no_dedicated_gibbs_sample_route() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();
    declare_identity_fixture().await;

    let (status, _body) = oneshot_json(post_json(
        "/v1/gauge_field/U/gibbs_sample",
        json!({"beta": 2.5, "n_sweeps": 1, "seed": 20260616}),
    ))
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "POST /v1/gauge_field/U/gibbs_sample must NOT be a registered route (D5)"
    );
}
