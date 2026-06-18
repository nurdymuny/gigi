//! TDD-HAL-II.6 — HTTP surface for LATTICE + GAUGE_FIELD.
//!
//! Smoke-tests the JSON envelope wire format against an in-process
//! `axum::Router` built by `gigi::gauge::http::build_router()`. Uses
//! `tower::ServiceExt::oneshot` so the suite never opens a TCP listener
//! — same router shape gigi-stream's main() merges into the production
//! app, just without the auth + namespace + rate-limit + CORS layers
//! wrapped around it (those are orthogonal middleware concerns the
//! main binary owns).
//!
//! Locked decisions wired through:
//!
//! - 1. Bit-identity: two `INIT HAAR_RANDOM SEED <s>` declarations with
//!   the same seed produce byte-identical buffers; the wire shape
//!   round-trips that — same JSON payload twice.
//! - 4. Wire format: `{"group", "repr_dim", "n_edges", "data": [[…],…]}`
//!   group-erased; future U(1) / SU(3) / Z(N) only changes the row
//!   width inside `data`.
//! - 5. Typed errors: HAAR_RANDOM without SEED, unknown group, missing
//!   lattice all surface as 400 + flat `{"error": "..."}` envelope. The
//!   error string contains the Halcyon substring anchors `SEED`,
//!   `SU(2)`, `not declared`.
//! - 6. Group erasure: `Group::SU2` is the only variant with live math;
//!   U(1) / SU(3) / Z(N) requests parse but immediately fall into the
//!   `UnsupportedGroup` arm.
//! - 7. Optionality: this file is gated on `halcyon` (the composite
//!   feature pulling in `lattice + gauge`) so the no-default-features
//!   build stays byte-identical.

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
/// the `registry_lock()` trick in `tests/halcyon_part_ii_persistence.rs`.
fn registry_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Drive a single request through the in-process router; collect the
/// response status + body bytes. The body cap of 8 MiB is generous —
/// a 90-edge SU(2) gauge field encodes to ~7 KB of JSON.
async fn oneshot_json(req: Request<Body>) -> (StatusCode, Vec<u8>) {
    // No host state — the global lattice + gauge registries back the
    // HTTP surface, so `Router<()>` is the right type for the test
    // harness.
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

/// Reset the singleton registries to a clean slate. Holding the
/// `registry_lock()` for the duration of the test serializes against
/// the other halcyon tests so cross-file singleton races don't drift.
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

/// TDD-HAL-II.6 — `POST /v1/lattice` registers a buckyball; the
/// subsequent `GET /v1/lattice/{name}` returns the envelope shape with
/// the canonical `n_vertices = 60`, `n_edges = 90`, `n_faces = 32`.
#[tokio::test]
async fn tdd_hal_ii_6_lattice_declare_introspect_round_trip() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();

    // 1. Declare.
    let (status, body) = oneshot_json(post_json(
        "/v1/lattice",
        json!({"name": "buckyball", "topology": "truncated_icosahedron"}),
    ))
    .await;
    assert_eq!(status, StatusCode::OK, "declare body: {}", String::from_utf8_lossy(&body));
    let envelope = parse_json(&body);
    assert_eq!(envelope["name"], "buckyball");
    assert_eq!(envelope["n_vertices"], 60);
    assert_eq!(envelope["n_edges"], 90);
    assert_eq!(envelope["n_faces"], 32);

    // 2. Introspect.
    let (status, body) = oneshot_json(get("/v1/lattice/buckyball")).await;
    assert_eq!(status, StatusCode::OK, "get body: {}", String::from_utf8_lossy(&body));
    let envelope = parse_json(&body);
    assert_eq!(envelope["name"], "buckyball");
    assert_eq!(envelope["n_vertices"], 60);
    assert_eq!(envelope["n_edges"], 90);
    assert_eq!(envelope["n_faces"], 32);
    // Topology hint defaults to "S2" off the buckyball constructor.
    assert_eq!(envelope["topology"], "S2");
    // GQL re-emit form is present (Halcyon's mock round-trips this).
    assert!(envelope["gql"]
        .as_str()
        .unwrap()
        .starts_with("LATTICE buckyball VERTICES 60"));
}

/// TDD-HAL-II.6 — `POST /v1/gauge_field` declares a HAAR_RANDOM SU(2)
/// field on the buckyball; the subsequent `GET /v1/gauge_field/{name}`
/// returns the group-erased envelope with `data` shape `(90, 4)`.
#[tokio::test]
async fn tdd_hal_ii_6_gauge_field_declare_introspect_round_trip() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();

    // Prerequisite: lattice declared via HTTP.
    let (status, _) = oneshot_json(post_json(
        "/v1/lattice",
        json!({"name": "buckyball", "topology": "truncated_icosahedron"}),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);

    // 1. Declare gauge field.
    let (status, body) = oneshot_json(post_json(
        "/v1/gauge_field",
        json!({
            "name": "U",
            "lattice": "buckyball",
            "group": "SU(2)",
            "init": {"kind": "haar_random", "seed": 20260616},
        }),
    ))
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "declare body: {}",
        String::from_utf8_lossy(&body)
    );

    // 2. Introspect.
    let (status, body) = oneshot_json(get("/v1/gauge_field/U")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "get body: {}",
        String::from_utf8_lossy(&body)
    );
    let envelope = parse_json(&body);
    assert_eq!(envelope["group"], "SU(2)");
    assert_eq!(envelope["repr_dim"], 4);
    assert_eq!(envelope["n_edges"], 90);
    assert_eq!(envelope["init_kind"], "HAAR_RANDOM");
    assert_eq!(envelope["init_seed"], 20260616);

    let data = envelope["data"]
        .as_array()
        .expect("`data` must be an array");
    assert_eq!(data.len(), 90, "data must have one row per edge");
    for (e, row) in data.iter().enumerate() {
        let row = row.as_array().unwrap_or_else(|| panic!("row {e} not array"));
        assert_eq!(row.len(), 4, "row {e} must be a quaternion of length 4");
        // SU(2) rows are unit-norm to f64 rounding.
        let n2: f64 = row
            .iter()
            .map(|v| v.as_f64().expect("row entry not f64"))
            .map(|x| x * x)
            .sum();
        assert!(
            (n2 - 1.0).abs() < 1e-12,
            "row {e} not unit-norm: |q|^2 = {n2}"
        );
    }
}

/// TDD-HAL-II.6 — two HAAR_RANDOM declarations with the same seed round-
/// trip to byte-identical JSON over the wire. This is Bee's locked
/// decision 1 (intra-binding bit-identity) lifted from the buffer layer
/// up to the HTTP envelope.
#[tokio::test]
async fn tdd_hal_ii_6_gauge_field_buffer_intra_gigi_byte_equal() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());

    let mut snapshots: Vec<Vec<u8>> = Vec::new();
    for _ in 0..2 {
        clear_registries();
        // Lattice.
        let (status, _) = oneshot_json(post_json(
            "/v1/lattice",
            json!({"name": "bb_eq", "topology": "truncated_icosahedron"}),
        ))
        .await;
        assert_eq!(status, StatusCode::OK);
        // Gauge field (same name + same seed).
        let (status, _) = oneshot_json(post_json(
            "/v1/gauge_field",
            json!({
                "name": "U_eq",
                "lattice": "bb_eq",
                "group": "SU(2)",
                "init": {"kind": "haar_random", "seed": 20260616},
            }),
        ))
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = oneshot_json(get("/v1/gauge_field/U_eq")).await;
        assert_eq!(status, StatusCode::OK);
        snapshots.push(body);
    }
    assert_eq!(
        snapshots[0], snapshots[1],
        "intra-binding bit-identity: same seed must produce byte-identical JSON over the wire"
    );
}

/// TDD-HAL-II.6 — `POST /v1/gauge_field` referencing an undeclared
/// lattice returns 400 with `{"error": "...not declared..."}`. Halcyon's
/// G2.B regex anchor `not declared` lands on the Display.
#[tokio::test]
async fn tdd_hal_ii_6_lattice_not_declared_error() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();

    let (status, body) = oneshot_json(post_json(
        "/v1/gauge_field",
        json!({
            "name": "U_orphan",
            "lattice": "nope_no_such_lattice",
            "group": "SU(2)",
            "init": {"kind": "identity"},
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let envelope = parse_json(&body);
    let err = envelope["error"]
        .as_str()
        .expect("error must be a string");
    assert!(
        err.contains("not declared"),
        "error must mention 'not declared' (Halcyon G2.B anchor): got {err}"
    );
}

/// TDD-HAL-II.6 — `POST /v1/gauge_field` with `group: "U(1)"` returns
/// 400 + the typed `UnsupportedGroup` error. Halcyon's G2.D regex
/// anchor `SU\(2\)` lands on the Display (Bee's locked decision 5).
#[tokio::test]
async fn tdd_hal_ii_6_unsupported_group_error() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();

    // Lattice must be declared so we can prove the failure is the
    // group, not the lattice resolution.
    let _ = oneshot_json(post_json(
        "/v1/lattice",
        json!({"name": "bb_u1", "topology": "truncated_icosahedron"}),
    ))
    .await;

    let (status, body) = oneshot_json(post_json(
        "/v1/gauge_field",
        json!({
            "name": "U_u1",
            "lattice": "bb_u1",
            "group": "U(1)",
            "init": {"kind": "identity"},
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let envelope = parse_json(&body);
    let err = envelope["error"]
        .as_str()
        .expect("error must be a string");
    assert!(
        err.contains("SU(2)"),
        "error must mention 'SU(2)' (Halcyon G2.D anchor): got {err}"
    );
}

/// TDD-HAL-II.6 — `POST /v1/gauge_field` with HAAR_RANDOM and no seed
/// returns 400 + the typed `SeedRequired` error. Halcyon's `match=SEED`
/// substring check lands on the Display (Bee's locked decision 1).
#[tokio::test]
async fn tdd_hal_ii_6_seed_required_error() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    clear_registries();

    let _ = oneshot_json(post_json(
        "/v1/lattice",
        json!({"name": "bb_no_seed", "topology": "truncated_icosahedron"}),
    ))
    .await;

    let (status, body) = oneshot_json(post_json(
        "/v1/gauge_field",
        json!({
            "name": "U_no_seed",
            "lattice": "bb_no_seed",
            "group": "SU(2)",
            "init": {"kind": "haar_random"},
        }),
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let envelope = parse_json(&body);
    let err = envelope["error"]
        .as_str()
        .expect("error must be a string");
    assert!(
        err.to_uppercase().contains("SEED"),
        "error must mention 'SEED' (Halcyon match=SEED anchor): got {err}"
    );
}
