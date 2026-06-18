//! TDD-HAL-II.6b — HTTP × durable persistence.
//!
//! Extends the II.6 HTTP surface with an OPTIONAL `persist: bool` field
//! on the `POST /v1/lattice` and `POST /v1/gauge_field` request bodies.
//! When `persist == true` the handler routes through the engine's
//! durable path (`declare_lattice_durable` / `declare_gauge_field_durable`
//! — Part II.4b) so the declaration is WAL-logged before it lands in the
//! in-process registry. When `persist` is absent or `false`, behavior is
//! unchanged from II.6 (in-memory only).
//!
//! Five red tests anchor the contract:
//!
//!   a. `tdd_hal_ii_6b_post_lattice_persist_survives_restart`
//!   b. `tdd_hal_ii_6b_post_gauge_field_persist_survives_restart`
//!   c. `tdd_hal_ii_6b_post_without_persist_field_is_ephemeral`
//!   d. `tdd_hal_ii_6b_post_persist_false_is_ephemeral`
//!   e. `tdd_hal_ii_6b_lattice_persist_required_for_gauge_field_persist`
//!
//! Bee's locked decisions wired through:
//!
//! - 3. Persistence is opt-in. Default stays in-memory; the existing II.6
//!   wire shape (no `persist` field) keeps exactly its previous behavior.
//! - 4. Reach. The HTTP surface and the embedded executor both expose the
//!   same `declare_*_durable` engine methods through their own opt-in
//!   keyword. This test file pins the HTTP half.
//! - 7. Optionality. Gated on `halcyon`; under `--no-default-features` it
//!   does not compile or run.

#![cfg(feature = "halcyon")]

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use gigi::engine::Engine;
use gigi::gauge::engine_handle as gauge_engine_handle;
use gigi::gauge::http::build_router;
use gigi::gauge::registry as gauge_registry;
use gigi::lattice::registry as lattice_registry;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use tower::ServiceExt;

/// Process-wide mutex serializing every test in this file. The lattice
/// + gauge registries are process singletons; two HTTP-driving tests
/// running in parallel would race the same registered names. The
/// `engine_handle` OnceLock module-global is also process-wide so the
/// install/clear flow has to be serialized too. Same trick as
/// `tests/halcyon_part_ii_http.rs` and `tests/halcyon_part_ii_persistence.rs`.
fn registry_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Drive a single request through the in-process router; collect the
/// response status + body bytes.
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

fn parse_json(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or_else(|e| {
        panic!(
            "body is not JSON: {e}; body = {}",
            String::from_utf8_lossy(body)
        )
    })
}

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

/// Reset all process-wide state the HTTP surface touches: the lattice
/// + gauge registries (cleared because Engine::open clears them on
/// replay) AND the engine_handle module-global (cleared so the next
/// install lands on a fresh slot).
fn clear_world() {
    gauge_registry::clear();
    lattice_registry::clear();
    gauge_engine_handle::clear_for_test();
}

/// Install a fresh engine pointing at `dir` and run the WAL replay
/// pass. Returns the Arc so the caller can drop it explicitly when
/// simulating a restart.
fn open_engine_and_install(dir: &Path) -> Arc<RwLock<Engine>> {
    let engine = Engine::open(dir).expect("engine open");
    let handle = Arc::new(RwLock::new(engine));
    gauge_engine_handle::install(handle.clone()).expect("install engine handle");
    handle
}

/// (a) `POST /v1/lattice` with `persist: true` survives a process
/// restart. The handler WAL-logs the declaration via the engine's
/// durable path; on the next Engine::open the replay re-installs the
/// lattice into the singleton registry; the fresh router's GET hits
/// that re-installed lattice.
#[tokio::test]
async fn tdd_hal_ii_6b_post_lattice_persist_survives_restart() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let lat_name = "tdd_hal_ii_6b_a_bb";

    // Phase 1 — declare durably, capture the GET envelope, drop engine.
    let snapshot_pre = {
        clear_world();
        let engine = open_engine_and_install(dir.path());
        let (status, body) = oneshot_json(post_json(
            "/v1/lattice",
            json!({
                "name": lat_name,
                "topology": "truncated_icosahedron",
                "persist": true,
            }),
        ))
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "declare body: {}",
            String::from_utf8_lossy(&body)
        );
        let (status, body) = oneshot_json(get(&format!("/v1/lattice/{lat_name}"))).await;
        assert_eq!(status, StatusCode::OK);
        // Engine drop happens at end of scope — but Arc must reach
        // refcount 1 first. The router doesn't hold the engine
        // (option-b module-global); the install side does. Clearing
        // engine_handle drops the install side's Arc.
        gauge_engine_handle::clear_for_test();
        drop(engine);
        body
    };

    // Phase 2 — simulate fresh process: clear registries, reopen
    // engine on same data dir (WAL replay re-populates registries),
    // GET on a fresh router.
    clear_world();
    let _engine = open_engine_and_install(dir.path());
    let (status, snapshot_post) = oneshot_json(get(&format!("/v1/lattice/{lat_name}"))).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "post-restart GET must succeed (lattice replayed): {}",
        String::from_utf8_lossy(&snapshot_post)
    );

    // Receipt: pre- and post-restart envelopes are structurally
    // identical (same name, topology, edge/vertex counts).
    let pre = parse_json(&snapshot_pre);
    let post = parse_json(&snapshot_post);
    assert_eq!(pre["name"], post["name"]);
    assert_eq!(pre["n_vertices"], post["n_vertices"]);
    assert_eq!(pre["n_edges"], post["n_edges"]);
    assert_eq!(pre["n_faces"], post["n_faces"]);
    assert_eq!(pre["topology"], post["topology"]);
}

/// (b) `POST /v1/gauge_field` with `persist: true` survives a process
/// restart with a byte-identical buffer. The intra-binding bit-identity
/// contract (Bee's locked decision 1) lifts from the buffer layer
/// through the WAL replay path to the HTTP envelope.
#[tokio::test]
async fn tdd_hal_ii_6b_post_gauge_field_persist_survives_restart() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let lat_name = "tdd_hal_ii_6b_b_bb";
    let field_name = "tdd_hal_ii_6b_b_U";
    let seed: u64 = 20260616;

    // Phase 1 — declare lattice + gauge field durably, capture buffer.
    let snapshot_pre = {
        clear_world();
        let engine = open_engine_and_install(dir.path());

        let (status, body) = oneshot_json(post_json(
            "/v1/lattice",
            json!({
                "name": lat_name,
                "topology": "truncated_icosahedron",
                "persist": true,
            }),
        ))
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "lattice declare: {}",
            String::from_utf8_lossy(&body)
        );

        let (status, body) = oneshot_json(post_json(
            "/v1/gauge_field",
            json!({
                "name": field_name,
                "lattice": lat_name,
                "group": "SU(2)",
                "init": {"kind": "haar_random", "seed": seed},
                "persist": true,
            }),
        ))
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "field declare: {}",
            String::from_utf8_lossy(&body)
        );

        let (status, body) = oneshot_json(get(&format!("/v1/gauge_field/{field_name}"))).await;
        assert_eq!(status, StatusCode::OK);
        gauge_engine_handle::clear_for_test();
        drop(engine);
        body
    };

    // Phase 2 — fresh process, reopen, GET.
    clear_world();
    let _engine = open_engine_and_install(dir.path());
    let (status, snapshot_post) =
        oneshot_json(get(&format!("/v1/gauge_field/{field_name}"))).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "post-restart GET must succeed (field replayed): {}",
        String::from_utf8_lossy(&snapshot_post)
    );

    // Byte-identity over the wire (matches II.6's intra-gigi
    // byte-equal gate — locked decision 1 lifted through restart).
    assert_eq!(
        snapshot_pre, snapshot_post,
        "post-restart wire payload must be byte-identical to pre-restart"
    );
}

/// (c) `POST /v1/gauge_field` with NO `persist` field is in-memory only
/// and is GONE after restart. This is the backward-compat receipt: the
/// existing II.6 wire shape (no `persist` field) still works exactly
/// as before — Bee's locked decision 3 in additive form.
#[tokio::test]
async fn tdd_hal_ii_6b_post_without_persist_field_is_ephemeral() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let lat_name = "tdd_hal_ii_6b_c_bb";
    let field_name = "tdd_hal_ii_6b_c_U";

    {
        clear_world();
        let engine = open_engine_and_install(dir.path());

        // Lattice declared in-memory only (no persist field).
        let (status, _) = oneshot_json(post_json(
            "/v1/lattice",
            json!({"name": lat_name, "topology": "truncated_icosahedron"}),
        ))
        .await;
        assert_eq!(status, StatusCode::OK);

        // Field declared in-memory only (no persist field).
        let (status, _) = oneshot_json(post_json(
            "/v1/gauge_field",
            json!({
                "name": field_name,
                "lattice": lat_name,
                "group": "SU(2)",
                "init": {"kind": "haar_random", "seed": 20260616},
            }),
        ))
        .await;
        assert_eq!(status, StatusCode::OK);

        // Sanity: in-memory registry sees the field pre-restart.
        let (status, _) = oneshot_json(get(&format!("/v1/gauge_field/{field_name}"))).await;
        assert_eq!(status, StatusCode::OK);

        gauge_engine_handle::clear_for_test();
        drop(engine);
    }

    // Restart. Engine::open clears the registries; replay finds no
    // LatticeDeclare/GaugeFieldDeclare entries; registries stay empty.
    clear_world();
    let _engine = open_engine_and_install(dir.path());
    let (status, body) = oneshot_json(get(&format!("/v1/gauge_field/{field_name}"))).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "ephemeral field must be gone post-restart: {}",
        String::from_utf8_lossy(&body)
    );
    let envelope = parse_json(&body);
    let err = envelope["error"].as_str().expect("error string");
    assert!(
        err.contains("not declared"),
        "error must mention 'not declared': {err}"
    );
}

/// (d) `POST /v1/gauge_field` with explicit `persist: false` has the
/// same restart-loss behavior as (c). Pins the explicit-false branch.
#[tokio::test]
async fn tdd_hal_ii_6b_post_persist_false_is_ephemeral() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let lat_name = "tdd_hal_ii_6b_d_bb";
    let field_name = "tdd_hal_ii_6b_d_U";

    {
        clear_world();
        let engine = open_engine_and_install(dir.path());

        let (status, _) = oneshot_json(post_json(
            "/v1/lattice",
            json!({
                "name": lat_name,
                "topology": "truncated_icosahedron",
                "persist": false,
            }),
        ))
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = oneshot_json(post_json(
            "/v1/gauge_field",
            json!({
                "name": field_name,
                "lattice": lat_name,
                "group": "SU(2)",
                "init": {"kind": "haar_random", "seed": 20260616},
                "persist": false,
            }),
        ))
        .await;
        assert_eq!(status, StatusCode::OK);

        gauge_engine_handle::clear_for_test();
        drop(engine);
    }

    clear_world();
    let _engine = open_engine_and_install(dir.path());
    let (status, body) = oneshot_json(get(&format!("/v1/gauge_field/{field_name}"))).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "persist:false field must be gone post-restart: {}",
        String::from_utf8_lossy(&body)
    );
}

/// (e) `POST /v1/gauge_field` with `persist: true` against a lattice
/// that was declared in-memory only (no `persist: true` at declare time)
/// is rejected at request time with 400 + a typed error message naming
/// the offending lattice. Fail-fast at declaration, not at replay.
#[tokio::test]
async fn tdd_hal_ii_6b_lattice_persist_required_for_gauge_field_persist() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let lat_name = "tdd_hal_ii_6b_e_bb";
    let field_name = "tdd_hal_ii_6b_e_U";

    clear_world();
    let _engine = open_engine_and_install(dir.path());

    // Lattice declared in-memory only.
    let (status, _) = oneshot_json(post_json(
        "/v1/lattice",
        json!({"name": lat_name, "topology": "truncated_icosahedron"}),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);

    // Persisting a field on a non-durable lattice must fail fast.
    let (status, body) = oneshot_json(post_json(
        "/v1/gauge_field",
        json!({
            "name": field_name,
            "lattice": lat_name,
            "group": "SU(2)",
            "init": {"kind": "identity"},
            "persist": true,
        }),
    ))
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "must fail fast: {}",
        String::from_utf8_lossy(&body)
    );
    let envelope = parse_json(&body);
    let err = envelope["error"].as_str().expect("error string");
    assert!(
        err.contains(lat_name),
        "error must name the offending lattice '{lat_name}': got {err}"
    );
    assert!(
        err.to_lowercase().contains("persist")
            || err.to_lowercase().contains("durabl"),
        "error must explain the persist requirement: got {err}"
    );
}
