//! Drift gate: every route the axum router registers must appear in `openapi.json`.
//!
//! `openapi.json` is served verbatim at `GET /v1/openapi.json` (via `include_str!`
//! in `src/bin/gigi_stream.rs`) and customers are pointed at it as *the* route
//! catalog. Nothing generates it — it is hand-maintained, and it had silently
//! drifted 88 routes behind the router by the time this file was written. The
//! whole ML family (`/scan`, `/cluster`, `/infer`, `/texture`, `/cadence`, …)
//! was missing with no test to notice.
//!
//! The defect is not "the ML family is absent" — that is a symptom, and it is
//! fixed. The defect is that *nothing fails* when a shipped verb never reaches
//! the spec. This file is that failure.
//!
//! # How it works
//!
//! [`router_routes`] scans `src/bin/gigi_stream.rs` for `.route("…")` literals,
//! which is the single place every path is registered. That set is diffed
//! against the `paths` keys of `openapi.json`. Axum and OpenAPI spell path
//! parameters the same way (`{name}`), so the comparison is direct.
//!
//! Feature-gated routes (`#[cfg(feature = …)]`) are included: whether a route
//! compiles today is a build question, whether it is documented is not.
//!
//! # The two lists
//!
//! [`INTERNAL_ROUTES`] is permanent: operator and transport surfaces that are
//! deliberately not part of the documented JSON API. It is short, and every
//! entry carries its reason. Adding to it is a real decision — a route hidden
//! here is a route customers cannot discover.
//!
//! [`UNDOCUMENTED_BACKLOG`] is *debt*, not policy. It is the drift that already
//! existed when this gate went in, grandfathered so the gate could land green
//! instead of never landing at all. It is a **ratchet**: a route in this list
//! that has since been documented makes [`undocumented_backlog_has_no_stale_entries`]
//! fail until the entry is deleted. The list can shrink. It must never grow —
//! a newly shipped verb is in neither list, so [`every_router_route_is_documented`]
//! fails on it, which is the entire point.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// Routes deliberately absent from the customer-facing spec, and why.
///
/// Keep this short. Each entry is a promise that the route is not API surface.
const INTERNAL_ROUTES: &[(&str, &str)] = &[
    ("/v1/admin/snapshot", "operator-only: forces a durability snapshot"),
    ("/v1/admin/log-config", "operator-only: runtime log configuration"),
    ("/v1/admin/log-level", "operator-only: runtime log level"),
    ("/v1/metrics", "Prometheus scrape target, not JSON API surface"),
    ("/dashboard", "serves the bundled HTML dashboard, not an API"),
    ("/ws", "WebSocket upgrade; OpenAPI 3.0 cannot describe the frame protocol"),
    ("/v1/ws/dashboard", "WebSocket upgrade (dashboard feed)"),
    ("/v1/ws/{bundle}/dashboard", "WebSocket upgrade (per-bundle feed)"),
    ("/v1/public/gql", "unauthenticated mirror of /v1/gql, enabled per deployment"),
];

/// Pre-existing documentation debt, grandfathered when this gate landed.
///
/// **This list may only shrink.** Document a route in `openapi.json`, then
/// delete its line here. Never add to it — a new route belongs in the spec.
const UNDOCUMENTED_BACKLOG: &[&str] = &[
    // Patterns / hunt
    "/v1/patterns",
    "/v1/patterns/{name}",
    "/v1/bundles/{name}/hunt",
    // Core data surface not yet mirrored into the spec
    "/v1/bundles/{name}/query-stream",
    "/v1/bundles/{name}/record/{id}/vector",
    "/v1/bundles/{name}/drop-field",
    "/v1/bundles/{name}/dhoom",
    "/v1/bundles/{name}/ingest",
    "/v1/bundles/{name}/vector-search",
    // Interactive transactions (the spec documents only /bundles/{name}/transaction)
    "/v1/transactions/begin",
    "/v1/transactions/{tx_id}",
    "/v1/transactions/{tx_id}/write",
    "/v1/transactions/{tx_id}/commit",
    "/v1/transactions/{tx_id}/rollback",
    // GQL + WISH
    "/v1/gql",
    "/v1/wish",
    "/v1/bundles/{name}/wish",
    // Geometry / analytics
    "/v1/bundles/{name}/capacity",
    "/v1/bundles/{name}/horizon",
    "/v1/bundles/{name}/depth",
    "/v1/bundles/{name}/perceive",
    "/v1/bundles/{name}/local_holonomy",
    "/v1/bundles/{name}/windowed_coherence",
    "/v1/bundles/{name}/imagine_coherence",
    "/v1/bundles/{name}/sharded/spectral_gap",
    "/v1/bundles/{name}/sharded/curvature",
    "/v1/bundles/{name}/sharded/holonomy_loop",
    "/v1/bundles/{name}/betti",
    "/v1/bundles/{name}/entropy",
    "/v1/bundles/{name}/free-energy",
    "/v1/bundles/{name}/geodesic",
    "/v1/bundles/{name}/metric",
    "/v1/bundles/{name}/anomalies",
    "/v1/bundles/{name}/anomalies/field",
    "/v1/bundles/{name}/health",
    "/v1/bundles/{name}/predict",
    "/v1/bundles/{name}/verify_invariant",
    "/v1/bundles/{name}/spectral_gap",
    "/v1/bundles/{name}/holonomy_debt",
    "/v1/bundles/{name}/flat_transport",
    "/v1/divergence",
    "/v1/causal_states/commutator",
    "/v1/quantum_cohomology/compose",
    "/v1/quantum_cohomology/capacity",
    // Post-Kähler phase 1
    "/v1/bundles/{name}/fisher_metric",
    "/v1/bundles/{name}/topology/persistence",
    "/v1/bundles/{name}/ml/wasserstein",
    "/v1/bundles/{name}/brain/reeb_flow",
    // Brain primitives (L9–L13)
    "/v1/bundles/{name}/brain/sample",
    "/v1/bundles/{name}/brain/confidence",
    "/v1/bundles/{name}/brain/attend",
    "/v1/bundles/{name}/brain/episodic",
    "/v1/bundles/{name}/brain/semantic",
    "/v1/bundles/{name}/brain/explain",
    "/v1/bundles/{name}/brain/dream",
    "/v1/bundles/{name}/brain/forecast",
    "/v1/bundles/{name}/brain/reconstruct",
    "/v1/bundles/{name}/brain/inpaint",
    "/v1/bundles/{name}/brain/predict",
    "/v1/bundles/{name}/brain/fit_diagnostics",
    "/v1/bundles/{name}/brain/distance_to_fit_mean",
    "/v1/bundles/{name}/brain/confidence_with_explain",
    "/v1/bundles/{name}/brain/sudoku",
    "/v1/bundles/{name}/brain/sample_transport",
    "/v1/bundles/{name}/brain/intent_gate",
];

/// Below this, assume the scanner broke rather than that the router shrank.
///
/// Without this floor a refactor of the router-building style would make
/// [`router_routes`] return nothing and every assertion below would pass
/// vacuously — a silent gate is worse than no gate.
const MIN_EXPECTED_ROUTES: usize = 100;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Every path registered with `.route("…")` in the stream binary.
///
/// Deliberately a source scan rather than an axum introspection call: axum's
/// `Router` exposes no path list, and building the real router needs an
/// `Engine`, a `StreamState`, and the full feature matrix. The registration
/// site is a single unambiguous token, so scanning for it is both cheaper and
/// harder to fool than reconstructing the router.
fn router_routes() -> BTreeSet<String> {
    let src = fs::read_to_string(repo_path("src/bin/gigi_stream.rs"))
        .expect("read src/bin/gigi_stream.rs");
    let bytes = src.as_bytes();
    let needle = ".route(";
    let mut out = BTreeSet::new();
    let mut from = 0usize;

    while let Some(rel) = src[from..].find(needle) {
        let mut i = from + rel + needle.len();
        // `.route(` may be followed by a newline and indentation before the path.
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                // No route path contains an escape; bail rather than guess.
                if bytes[i] == b'\\' {
                    break;
                }
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'"' {
                out.insert(src[start..i].to_string());
            }
        }
        from = from + rel + needle.len();
    }

    assert!(
        out.len() >= MIN_EXPECTED_ROUTES,
        "route scanner found only {} routes (expected at least {}). The \
         `.route(\"…\")` registration style in src/bin/gigi_stream.rs probably \
         changed — fix `router_routes()` rather than lowering the floor, or \
         this whole file silently stops checking anything.",
        out.len(),
        MIN_EXPECTED_ROUTES,
    );
    out
}

/// The `paths` keys of the checked-in `openapi.json`.
fn openapi_paths() -> BTreeSet<String> {
    let raw = fs::read_to_string(repo_path("openapi.json")).expect("read openapi.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("openapi.json is valid JSON");
    doc.get("paths")
        .and_then(|p| p.as_object())
        .expect("openapi.json has a `paths` object")
        .keys()
        .cloned()
        .collect()
}

fn internal() -> BTreeSet<String> {
    INTERNAL_ROUTES.iter().map(|(p, _)| p.to_string()).collect()
}

fn backlog() -> BTreeSet<String> {
    UNDOCUMENTED_BACKLOG.iter().map(|p| p.to_string()).collect()
}

/// **The gate.** A route that is registered, not internal, and not
/// grandfathered must be in `openapi.json`.
///
/// This is what fails when the next verb ships without a spec entry.
#[test]
fn every_router_route_is_documented() {
    let routes = router_routes();
    let spec = openapi_paths();
    let internal = internal();
    let backlog = backlog();

    let missing: Vec<&String> = routes
        .iter()
        .filter(|r| !spec.contains(*r) && !internal.contains(*r) && !backlog.contains(*r))
        .collect();

    assert!(
        missing.is_empty(),
        "\n{} route(s) are registered in the axum router but missing from \
         openapi.json:\n{}\n\nopenapi.json is served verbatim at GET \
         /v1/openapi.json and customers read it as the route catalog. Add each \
         path (request + response schema, `{{\"error\": \"…\"}}` for failures), \
         or — if the route is genuinely not API surface — add it to \
         INTERNAL_ROUTES in this file with a reason. Do not add it to \
         UNDOCUMENTED_BACKLOG; that list is closed.\n",
        missing.len(),
        missing
            .iter()
            .map(|r| format!("  - {r}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// The ratchet: once a backlog route is documented, its entry must go.
///
/// Without this the backlog would rot into a permanent allowlist that hides
/// fresh drift behind stale names.
#[test]
fn undocumented_backlog_has_no_stale_entries() {
    let spec = openapi_paths();
    let now_documented: Vec<&&str> = UNDOCUMENTED_BACKLOG
        .iter()
        .filter(|p| spec.contains(**p))
        .collect();

    assert!(
        now_documented.is_empty(),
        "\n{} route(s) are in UNDOCUMENTED_BACKLOG but are now documented in \
         openapi.json. Delete these lines from the list — it is a ratchet, and \
         leaving them would let it shadow real drift:\n{}\n",
        now_documented.len(),
        now_documented
            .iter()
            .map(|r| format!("  - {r}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Every backlog entry must still be a real route.
///
/// Catches renames and deletions that would otherwise leave dead names in the
/// list forever.
#[test]
fn undocumented_backlog_matches_real_routes() {
    let routes = router_routes();
    let dangling: Vec<&&str> = UNDOCUMENTED_BACKLOG
        .iter()
        .filter(|p| !routes.contains(**p))
        .collect();

    assert!(
        dangling.is_empty(),
        "\nUNDOCUMENTED_BACKLOG names {} route(s) the router no longer \
         registers — renamed or removed. Delete them:\n{}\n",
        dangling.len(),
        dangling
            .iter()
            .map(|r| format!("  - {r}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Internal exclusions must be real, and must not also be documented.
#[test]
fn internal_routes_are_real_and_undocumented() {
    let routes = router_routes();
    let spec = openapi_paths();

    for (path, reason) in INTERNAL_ROUTES {
        assert!(
            routes.contains(*path),
            "INTERNAL_ROUTES lists `{path}` ({reason}) but the router does not \
             register it — renamed or removed. Delete the entry.",
        );
        assert!(
            !spec.contains(*path),
            "`{path}` is marked internal ({reason}) but IS documented in \
             openapi.json. Pick one: remove it from INTERNAL_ROUTES if it is \
             really API surface, or drop it from the spec.",
        );
    }
}

/// Reverse drift: the spec must not advertise routes that do not exist.
#[test]
fn every_openapi_path_exists_in_the_router() {
    let routes = router_routes();
    let spec = openapi_paths();

    let phantom: Vec<&String> = spec.iter().filter(|p| !routes.contains(*p)).collect();

    assert!(
        phantom.is_empty(),
        "\nopenapi.json documents {} path(s) the router does not register. A \
         customer calling these gets a 404:\n{}\n",
        phantom.len(),
        phantom
            .iter()
            .map(|r| format!("  - {r}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Pins the ML family specifically, so the fix that motivated this file cannot
/// silently regress.
#[test]
fn the_ml_family_is_documented() {
    let spec = openapi_paths();

    let ml = [
        "/v1/ml",
        "/v1/bundles/{name}/scan",
        "/v1/bundles/{name}/scan/fit",
        "/v1/bundles/{name}/cluster",
        "/v1/bundles/{name}/infer",
        "/v1/bundles/{name}/reduce",
        "/v1/bundles/{name}/factorize",
        "/v1/bundles/{name}/changepoints",
        "/v1/bundles/{name}/prescribe",
        "/v1/bundles/{name}/circulation",
        "/v1/bundles/{name}/solve",
        "/v1/bundles/{name}/texture",
        "/v1/bundles/{name}/precedence",
        "/v1/bundles/{name}/cadence",
    ];

    let missing: Vec<&str> = ml.iter().copied().filter(|p| !spec.contains(*p)).collect();
    assert!(
        missing.is_empty(),
        "ML routes vanished from openapi.json: {missing:?}",
    );
}

/// `cadence.index_blocked` is genuinely absent under two blocks and is emitted
/// as `null`, not a fabricated `0.0`. A spec that calls it a plain number tells
/// a typed client to expect a value that will not arrive.
#[test]
fn cadence_index_blocked_is_documented_nullable() {
    let raw = fs::read_to_string(repo_path("openapi.json")).expect("read openapi.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");

    let field = doc
        .pointer(
            "/paths/~1v1~1bundles~1{name}~1cadence/post/responses/200/content/\
             application~1json/schema/properties/index_blocked",
        )
        .expect("cadence 200 response documents `index_blocked`");

    assert_eq!(
        field.get("nullable").and_then(|v| v.as_bool()),
        Some(true),
        "`index_blocked` must be documented `nullable: true` — the handler \
         emits JSON null when there are under two blocks. Got: {field}",
    );
}
