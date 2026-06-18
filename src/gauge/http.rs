//! HTTP surface for LATTICE + GAUGE_FIELD declaration + introspection.
//!
//! Closes TDD-HAL-II.6. Mirrors the GQL parser/executor surface
//! (TDD-HAL-II.5) over a JSON envelope so the Halcyon mock-to-live swap
//! can drive declarations from any HTTP client (the mock has been
//! reading/writing the same envelope shape against `gigi_client/mock.py`
//! since Part I).
//!
//! Wire format (Bee's locked decision 4):
//!
//! - `POST /v1/lattice`
//!   request body: `{"name": "...", "topology": "truncated_icosahedron"}`
//!   response: 200 + `LatticeView` envelope.
//! - `GET /v1/lattice/{name}`
//!   response: 200 + `LatticeView` envelope, or 400 if undeclared (the
//!   error envelope explicitly says "not declared").
//! - `POST /v1/gauge_field`
//!   request body: `{"name": "U", "lattice": "buckyball",
//!                    "group": "SU(2)", "init": {"kind": "haar_random",
//!                    "seed": 20260616}}`
//!   response: 200.
//! - `GET /v1/gauge_field/{name}`
//!   response: 200 + `GaugeFieldView` envelope (`group`, `repr_dim`,
//!   `n_edges`, `data: [[q0,q1,q2,q3], …]`).
//!
//! Group-erasure note: the `GaugeFieldView` JSON shape is group-erased
//! — only the row width inside `data` changes when U(1) / SU(3) / Z(N)
//! ship. The same wire surface holds.
//!
//! Errors map to HTTP 400 with a flat `{"error": "..."}` envelope so
//! Halcyon's substring matches (`SEED`, `SU(2)`, `not declared`) land on
//! a uniform shape. Internal storage failures map to 500.

use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use super::error::GaugeFieldError;
use super::group::Group;
use super::registry as gauge_registry;
use super::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
use crate::lattice::registry as lattice_registry;
use crate::lattice::topology::truncated_icosahedron::buckyball;
use crate::lattice::Lattice;

/// Flat error envelope. Matches the gigi-stream binary's
/// `ErrorResponse` shape (`{"error": "..."}`) so Halcyon's substring
/// checks hit on either surface.
#[derive(Serialize)]
pub struct ErrorEnvelope {
    pub error: String,
}

fn bad_request(msg: impl Into<String>) -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope { error: msg.into() }),
    )
}

fn internal(msg: impl Into<String>) -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorEnvelope { error: msg.into() }),
    )
}

// ── POST /v1/lattice ──────────────────────────────────────────

/// Request body for `POST /v1/lattice`. The Halcyon mock uses
/// `topology="truncated_icosahedron"` (canonical constructor); future
/// constructors land beside it without changing the wire shape.
#[derive(Deserialize)]
pub struct LatticeCreateRequest {
    pub name: String,
    /// Canonical-graph identifier. Lower- or upper-case accepted;
    /// matches the GQL `LATTICE name FROM <CANONICAL>` shorthand.
    pub topology: String,
    /// Optional topology hint string (`"S2"`, `"T2"`, …). Stored
    /// verbatim — the engine does not interpret it.
    #[serde(default)]
    pub topology_hint: Option<String>,
}

/// Wire envelope for a Lattice. Round-trips through `GET /v1/lattice/{name}`.
#[derive(Serialize)]
pub struct LatticeView {
    pub name: String,
    pub n_vertices: usize,
    pub n_edges: usize,
    pub n_faces: usize,
    pub topology: Option<String>,
    /// Canonical GQL re-emit form. Halcyon's mock parses this back via
    /// `Lattice::from_gql` for cross-binding round-trip checks.
    pub gql: String,
}

impl LatticeView {
    fn from_lattice(lat: &Lattice) -> Self {
        Self {
            name: lat.name.clone(),
            n_vertices: lat.n_vertices,
            n_edges: lat.n_edges(),
            n_faces: lat.n_faces(),
            topology: lat.topology.clone(),
            gql: lat.to_gql(),
        }
    }
}

async fn lattice_create(
    Json(req): Json<LatticeCreateRequest>,
) -> Result<(StatusCode, Json<LatticeView>), (StatusCode, Json<ErrorEnvelope>)> {
    let mut lat = match req.topology.to_ascii_uppercase().as_str() {
        "TRUNCATED_ICOSAHEDRON" | "BUCKYBALL" => buckyball(),
        other => {
            return Err(bad_request(format!(
                "gauge: unknown canonical lattice constructor '{other}' \
                 (Part II ships TRUNCATED_ICOSAHEDRON only)"
            )));
        }
    };
    lat.name = req.name.clone();
    if let Some(hint) = req.topology_hint {
        lat.topology = Some(hint);
    }
    let view = LatticeView::from_lattice(&lat);
    lattice_registry::register(lat);
    Ok((StatusCode::OK, Json(view)))
}

async fn lattice_get(
    Path(name): Path<String>,
) -> Result<Json<LatticeView>, (StatusCode, Json<ErrorEnvelope>)> {
    match lattice_registry::get(&name) {
        Some(lat) => Ok(Json(LatticeView::from_lattice(&lat))),
        None => Err(bad_request(format!(
            "gauge: lattice '{name}' is not declared (POST /v1/lattice first)"
        ))),
    }
}

// ── POST /v1/gauge_field ──────────────────────────────────────

/// Init clause for `POST /v1/gauge_field`. Mirrors the GQL `INIT …`
/// surface: `IDENTITY` / `HAAR_RANDOM` (+ seed) / `FROM_FIELD` (+ source
/// name). Tag is `kind` so the JSON shape stays flat and Halcyon's mock
/// can build the payload with one dict literal.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InitSpec {
    Identity,
    HaarRandom {
        #[serde(default)]
        seed: Option<u64>,
    },
    FromField {
        source: String,
    },
}

#[derive(Deserialize)]
pub struct GaugeFieldCreateRequest {
    pub name: String,
    pub lattice: String,
    /// Group label string (Halcyon emits `"SU(2)"`; the parser also
    /// accepts `"SU2"` for symmetry with code-style constants).
    pub group: String,
    pub init: InitSpec,
}

#[derive(Serialize)]
pub struct GaugeFieldCreateResponse {
    pub name: String,
    pub lattice: String,
    pub group: String,
    pub repr_dim: usize,
    pub n_edges: usize,
    pub init_kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_seed: Option<u64>,
}

/// JSON envelope for `GET /v1/gauge_field/{name}`. Group-erased: only
/// the row width of `data` changes when U(1) / SU(3) / Z(N) ships.
#[derive(Serialize)]
pub struct GaugeFieldView {
    pub name: String,
    pub lattice: String,
    pub group: String,
    pub repr_dim: usize,
    pub n_edges: usize,
    pub init_kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_seed: Option<u64>,
    /// Row-major `(n_edges, repr_dim)` buffer. For SU(2) every row is
    /// the scalar-first quaternion `(q0, q1, q2, q3)`.
    pub data: Vec<Vec<f64>>,
}

fn parse_group_label(s: &str) -> Result<Group, (StatusCode, Json<ErrorEnvelope>)> {
    let up = s.trim().to_ascii_uppercase();
    let g = match up.as_str() {
        "SU(2)" | "SU2" => Group::SU2,
        "SU(3)" | "SU3" => Group::SU3,
        "U(1)" | "U1" => Group::U1,
        other => {
            // Z(N) — accept `"Z(N)"` and `"Z(<int>)"` (the latter is
            // closer to what Halcyon's mock emits; the parse error
            // surfaces as a typed UnsupportedGroup once we route it
            // through the executor path).
            if let Some(inner) = other.strip_prefix("Z(").and_then(|s| s.strip_suffix(')')) {
                if let Ok(n) = inner.parse::<u32>() {
                    Group::ZN { n }
                } else {
                    Group::ZN { n: 0 }
                }
            } else {
                return Err(bad_request(format!(
                    "gauge: unknown group label '{s}' (expected SU(2) / SU(3) / U(1) / Z(N))"
                )));
            }
        }
    };
    Ok(g)
}

fn init_kind_label(kind: &GaugeFieldInit) -> &'static str {
    match kind {
        GaugeFieldInit::Identity => "IDENTITY",
        GaugeFieldInit::HaarRandom => "HAAR_RANDOM",
        GaugeFieldInit::FromField(_) => "FROM_FIELD",
    }
}

async fn gauge_field_create(
    Json(req): Json<GaugeFieldCreateRequest>,
) -> Result<(StatusCode, Json<GaugeFieldCreateResponse>), (StatusCode, Json<ErrorEnvelope>)> {
    // 1. Group dispatch — non-SU(2) variants surface the typed
    //    UnsupportedGroup error (Halcyon G2.D regex anchor `SU\(2\)`
    //    matches against its Display).
    let group = parse_group_label(&req.group)?;
    if !matches!(group, Group::SU2) {
        return Err(bad_request(GaugeFieldError::UnsupportedGroup(group).to_string()));
    }

    // 2. Resolve the bound lattice. The `LatticeNotDeclared` Display
    //    contains the literal "not declared" so the Halcyon G2.B
    //    regex anchor `not declared` matches.
    let lat = match lattice_registry::get(&req.lattice) {
        Some(l) => l,
        None => {
            return Err(bad_request(
                GaugeFieldError::LatticeNotDeclared(req.lattice.clone()).to_string(),
            ));
        }
    };

    // 3. Build the init kind + optional seed from the JSON tag.
    let (init_kind, seed) = match req.init {
        InitSpec::Identity => (GaugeFieldInit::Identity, None),
        InitSpec::HaarRandom { seed } => {
            // Bee's locked decision 1: HAAR_RANDOM requires a SEED for
            // the intra-binding bit-identity contract. The typed error
            // surface lifts that to the HTTP boundary.
            if seed.is_none() {
                return Err(bad_request(GaugeFieldError::SeedRequired.to_string()));
            }
            (GaugeFieldInit::HaarRandom, seed)
        }
        InitSpec::FromField { source } => (GaugeFieldInit::FromField(source), None),
    };

    // 4. Materialize. FROM_FIELD takes a separate path because the
    //    constructor doesn't have a registry handle.
    let field = match &init_kind {
        GaugeFieldInit::FromField(src) => {
            let src_handle = match gauge_registry::get(src) {
                Some(h) => h,
                None => {
                    return Err(bad_request(
                        GaugeFieldError::FieldNotDeclared(src.clone()).to_string(),
                    ));
                }
            };
            if src_handle.lattice_name() != lat.name {
                return Err(bad_request(format!(
                    "gauge: INIT FROM_FIELD source '{}' lives on lattice '{}', not '{}'",
                    src,
                    src_handle.lattice_name(),
                    lat.name
                )));
            }
            let src_buf = src_handle.as_dense_buffer().clone();
            SU2GaugeField {
                name: req.name.clone(),
                lattice_name: lat.name.clone(),
                buffer: src_buf,
                init_kind: init_kind.clone(),
                init_seed: None,
            }
        }
        _ => SU2GaugeField::new(req.name.clone(), &lat, init_kind.clone(), seed)
            .map_err(|e| bad_request(e.to_string()))?,
    };

    let repr_dim = field.buffer.repr_dim;
    let n_edges = field.buffer.n_edges;
    let init_kind_str = init_kind_label(&field.init_kind);
    let init_seed_back = field.init_seed;
    let handle: std::sync::Arc<dyn gauge_registry::GaugeFieldHandle> =
        std::sync::Arc::new(field);
    gauge_registry::register(handle);

    Ok((
        StatusCode::OK,
        Json(GaugeFieldCreateResponse {
            name: req.name,
            lattice: lat.name,
            group: group.label().to_string(),
            repr_dim,
            n_edges,
            init_kind: init_kind_str,
            init_seed: init_seed_back,
        }),
    ))
}

async fn gauge_field_get(
    Path(name): Path<String>,
) -> Result<Json<GaugeFieldView>, (StatusCode, Json<ErrorEnvelope>)> {
    let handle = match gauge_registry::get(&name) {
        Some(h) => h,
        None => {
            return Err(bad_request(
                GaugeFieldError::FieldNotDeclared(name.clone()).to_string(),
            ));
        }
    };
    let buf = handle.as_dense_buffer();
    let (kind, init_seed) = handle.init_metadata();
    let init_kind = init_kind_label(&kind);
    // Row-major split for the wire — JSON `data` is `[[q0,q1,q2,q3], …]`
    // so the consumer never has to know `repr_dim` to decode a row.
    let mut data: Vec<Vec<f64>> = Vec::with_capacity(buf.n_edges);
    let d = buf.repr_dim;
    if buf.data.len() != buf.n_edges * d {
        return Err(internal(format!(
            "gauge: buffer shape mismatch (expected {} f64s, got {})",
            buf.n_edges * d,
            buf.data.len()
        )));
    }
    for e in 0..buf.n_edges {
        let row = buf.data[e * d..(e + 1) * d].to_vec();
        data.push(row);
    }
    Ok(Json(GaugeFieldView {
        name: handle.name().to_string(),
        lattice: handle.lattice_name().to_string(),
        group: handle.group().label().to_string(),
        repr_dim: d,
        n_edges: buf.n_edges,
        init_kind,
        init_seed,
        data,
    }))
}

/// Build the LATTICE + GAUGE_FIELD HTTP router. The gigi-stream binary
/// merges this into its main app; the test harness drives the same
/// router directly via `tower::ServiceExt::oneshot` (no listener).
///
/// The router carries no state — the lattice + gauge registries are
/// process singletons (mirrors `lattice::registry` / `gauge::registry`),
/// so the HTTP surface threads through them directly. State threading
/// becomes a refactor concern once persistence land on the BundleStore
/// path that's already wired through `engine.declare_gauge_field_durable`
/// (II.4b).
///
/// Generic in the host app's `State` type so the gigi-stream binary
/// (which has `Router<Arc<StreamState>>`) can `merge` this in without
/// state-type juggling. The handlers themselves don't read host state
/// — they hit the global lattice + gauge registries — so any concrete
/// `S: Clone + Send + Sync + 'static` works.
pub fn build_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/v1/lattice", post(lattice_create))
        .route("/v1/lattice/{name}", get(lattice_get))
        .route("/v1/gauge_field", post(gauge_field_create))
        .route("/v1/gauge_field/{name}", get(gauge_field_get))
}
