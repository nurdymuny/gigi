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

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use super::engine_handle as gauge_engine_handle;
use super::error::GaugeFieldError;
use super::group::Group;
use super::registry as gauge_registry;
use super::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
use super::symplectic_flow as flow_mod;
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

fn not_found(msg: impl Into<String>) -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::NOT_FOUND,
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
    /// TDD-HAL-II.6b: opt-in durable persistence. When `true` the
    /// handler routes through `engine.declare_lattice_durable` (WAL-
    /// logged before in-process registration). When `false` or
    /// omitted, the existing II.6 in-memory-only path runs (Bee's
    /// locked decision 3: persistence is opt-in; default stays
    /// in-memory so existing II.6 clients are unchanged).
    #[serde(default)]
    pub persist: bool,
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

    if req.persist {
        // TDD-HAL-II.6b durable path: WAL-log the declaration before
        // installing in the in-process registry, via the engine handle
        // installed by `gigi_stream::main` (or the test harness's
        // `engine_handle::install`). On crash between log + register
        // the next Engine::open replays the WAL entry and re-installs.
        let lat_name = lat.name.clone();
        let outcome = gauge_engine_handle::with_engine_mut(|engine| {
            engine.declare_lattice_durable(lat)
        });
        match outcome {
            Some(Ok(())) => {
                gauge_engine_handle::mark_lattice_durable(&lat_name);
            }
            Some(Err(e)) => {
                return Err(internal(format!(
                    "gauge: durable lattice declaration failed: {}",
                    e.kind()
                )));
            }
            None => {
                return Err(internal(
                    "gauge: no engine handle installed; cannot honor persist:true",
                ));
            }
        }
    } else {
        lattice_registry::register(lat);
    }
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
    /// TDD-HAL-II.6b: opt-in durable persistence. When `true` the
    /// handler routes through `engine.declare_gauge_field_durable`
    /// (WAL-logged metadata before in-process registration). Default
    /// `false` keeps the II.6 in-memory-only behavior. `persist: true`
    /// against an in-memory lattice fails fast at declaration time
    /// rather than at replay time (see handler dispatch).
    #[serde(default)]
    pub persist: bool,
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
    // TDD-HAL-V.0b — snapshot BEFORE moving into the Arc so the SU(2)-
    // mut sibling map can be populated alongside the dyn read map.
    // Pre-fix this handler only landed in the dyn map; downstream
    // mutators (GIBBS_SAMPLE / SYMPLECTIC_FLOW) could not find the
    // field via `get_su2_mut`. Part V P-1 production receipt surfaced
    // it via HTTP 500 "source field U_p1 is not declared".
    let field_snapshot = field.clone();
    let handle: std::sync::Arc<dyn gauge_registry::GaugeFieldHandle> =
        std::sync::Arc::new(field);

    if req.persist {
        // TDD-HAL-II.6b gate (e): persisting a gauge field on a non-
        // durable lattice would resurrect orphaned on the next reopen
        // (the WAL has a GaugeFieldDeclare but no LatticeDeclare for
        // its base topology, so Pass 2 of replay_gauge_substrate
        // fails to resolve the lattice). Fail fast at declaration.
        if !gauge_engine_handle::is_lattice_durable(&lat.name) {
            return Err(bad_request(format!(
                "gauge: lattice '{}' is not durably persisted; \
                 declare it with persist:true first before persisting a \
                 gauge field on it",
                lat.name
            )));
        }
        // FROM_FIELD + persist=true is rejected because the WAL
        // replay path in `persistence::materialize_field` cannot
        // re-resolve the source field from metadata alone (the
        // source's full buffer is not in the WAL — Bee's locked
        // decision 1: metadata-only WAL variant). P1 follow-up.
        if matches!(handle.init_metadata().0, GaugeFieldInit::FromField(_)) {
            return Err(bad_request(
                "gauge: INIT FROM_FIELD with persist:true is not yet \
                 supported (WAL replay cannot re-resolve the source \
                 field from declaration metadata alone); declare \
                 the source HAAR_RANDOM/IDENTITY first or omit persist",
            ));
        }
        let outcome = gauge_engine_handle::with_engine_mut(|engine| {
            engine.declare_gauge_field_durable(handle.clone())
        });
        match outcome {
            Some(Ok(())) => {}
            Some(Err(e)) => {
                return Err(internal(format!(
                    "gauge: durable gauge-field declaration failed: {}",
                    e.kind()
                )));
            }
            None => {
                return Err(internal(
                    "gauge: no engine handle installed; cannot honor persist:true",
                ));
            }
        }
    } else {
        gauge_registry::register(handle);
    }
    // TDD-HAL-V.0b — re-park into the SU(2)-mut sibling map so
    // GIBBS_SAMPLE / SYMPLECTIC_FLOW can lock the field via
    // `get_su2_mut` without a downstream manual workaround. Done in
    // both persist and non-persist branches; the snapshot is the same
    // source of truth handed to either the engine or the dyn registry.
    gauge_registry::register_su2(field_snapshot);

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

// ── TDD-HAL-III.7 — Read-only Part III verb routes ─────────────
//
// `PLAQUETTE` (III.1), `Q_SURROGATE` (III.2), and a batched
// `observables` endpoint that fans out to both. **No** dedicated
// `gibbs_sample` route (locked decision D5): GIBBS_SAMPLE remains
// reachable only through `POST /v1/gql` (parser::execute); the
// route-table-absence is the load-bearing receipt the test suite
// asserts at gate III.7.
//
// Group-erasure note (locked decision D6/D7): the handlers below
// take the read-only `Arc<dyn GaugeFieldHandle>` from
// `gauge::registry::get` and call the III.1 / III.2 library
// functions, which already dispatch on `handle.group()`. JSON
// envelopes carry no group label — only the `Vec<f64>` / scalar
// `f64` shapes change when U(1) / SU(3) / Z(N) ships.

/// Query string for `GET /v1/gauge_field/{name}/plaquette?reduction=…`.
/// Mirrors the `PlaquetteReduction` enum tags (`per_face` / `mean` /
/// `sum`). Default `mean` so a bare GET returns the canonical scalar
/// the Halcyon mock surfaces.
#[derive(Deserialize)]
pub struct PlaquetteQuery {
    #[serde(default = "default_reduction")]
    pub reduction: String,
}

fn default_reduction() -> String {
    "mean".to_string()
}

/// Per-face envelope: `{"reduction": "per_face", "values": [...]}`.
/// Mirrors the JSON shape the GQL `SELECT PLAQUETTE OF U;` executor
/// arm emits (locked decision D7 — `per_face` is `Vec<f64>` of length
/// `F`, q0 column only).
#[derive(Serialize)]
pub struct PlaquettePerFaceEnvelope {
    pub reduction: &'static str,
    pub values: Vec<f64>,
}

/// Scalar reduction envelope: `{"reduction": "mean"|"sum", "value": …}`.
#[derive(Serialize)]
pub struct PlaquetteScalarEnvelope {
    pub reduction: &'static str,
    pub value: f64,
}

/// Either envelope shape, tagged at the JSON level by the `reduction`
/// field. Axum routing is one handler per URI; the per-face / scalar
/// split is inside the body, not in the route.
#[derive(Serialize)]
#[serde(untagged)]
pub enum PlaquetteEnvelope {
    PerFace(PlaquettePerFaceEnvelope),
    Scalar(PlaquetteScalarEnvelope),
}

/// Resolve `(handle, lattice)` for the named field. Returns a flat 400
/// + typed-error Display on either of the two failure modes (field not
/// declared, lattice gone) so the same `not declared` substring anchor
/// from Part II keeps working.
fn resolve_field_and_lattice(
    name: &str,
) -> Result<(std::sync::Arc<dyn gauge_registry::GaugeFieldHandle>, Lattice), (StatusCode, Json<ErrorEnvelope>)>
{
    let handle = gauge_registry::get(name).ok_or_else(|| {
        bad_request(GaugeFieldError::FieldNotDeclared(name.to_string()).to_string())
    })?;
    let lat = lattice_registry::get(handle.lattice_name()).ok_or_else(|| {
        bad_request(
            GaugeFieldError::LatticeNotDeclared(handle.lattice_name().to_string()).to_string(),
        )
    })?;
    Ok((handle, lat))
}

async fn plaquette_get(
    Path(name): Path<String>,
    Query(q): Query<PlaquetteQuery>,
) -> Result<Json<PlaquetteEnvelope>, (StatusCode, Json<ErrorEnvelope>)> {
    let (handle, lat) = resolve_field_and_lattice(&name)?;
    match q.reduction.to_ascii_lowercase().as_str() {
        "per_face" => {
            let values = super::plaquette::plaquette_per_face(handle.as_ref(), &lat)
                .map_err(|e| bad_request(e.to_string()))?;
            Ok(Json(PlaquetteEnvelope::PerFace(PlaquettePerFaceEnvelope {
                reduction: "per_face",
                values,
            })))
        }
        "mean" => {
            let value = super::plaquette::plaquette_mean(handle.as_ref(), &lat)
                .map_err(|e| bad_request(e.to_string()))?;
            Ok(Json(PlaquetteEnvelope::Scalar(PlaquetteScalarEnvelope {
                reduction: "mean",
                value,
            })))
        }
        "sum" => {
            let value = super::plaquette::plaquette_sum(handle.as_ref(), &lat)
                .map_err(|e| bad_request(e.to_string()))?;
            Ok(Json(PlaquetteEnvelope::Scalar(PlaquetteScalarEnvelope {
                reduction: "sum",
                value,
            })))
        }
        other => Err(bad_request(format!(
            "gauge: unknown plaquette reduction '{other}' (expected per_face / mean / sum)"
        ))),
    }
}

/// Scalar `Q_SURROGATE` envelope: `{"value": …}`. Mirrors the Halcyon
/// mock JSON shape byte-for-byte at the JSON level (locked decision D6).
#[derive(Serialize)]
pub struct QSurrogateEnvelope {
    pub value: f64,
}

async fn q_surrogate_get(
    Path(name): Path<String>,
) -> Result<Json<QSurrogateEnvelope>, (StatusCode, Json<ErrorEnvelope>)> {
    let (handle, lat) = resolve_field_and_lattice(&name)?;
    let value = super::q_surrogate::q_surrogate(handle.as_ref(), &lat)
        .map_err(|e| bad_request(e.to_string()))?;
    Ok(Json(QSurrogateEnvelope { value }))
}

/// Body for `POST /v1/gauge_field/{name}/observables`. Read-only
/// despite the verb (II.6c clarification: POST without side effect is
/// the consumer-safe batched-read pattern — accepts a JSON list of
/// observable identifiers in the request body, returns one JSON key
/// per identifier in the response).
#[derive(Deserialize)]
pub struct ObservablesBatchRequest {
    pub observables: Vec<String>,
}

async fn observables_post(
    Path(name): Path<String>,
    Json(req): Json<ObservablesBatchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorEnvelope>)> {
    let (handle, lat) = resolve_field_and_lattice(&name)?;
    let mut out = serde_json::Map::with_capacity(req.observables.len());
    for obs in req.observables {
        match obs.to_ascii_lowercase().as_str() {
            "mean_plaquette" => {
                let v = super::plaquette::plaquette_mean(handle.as_ref(), &lat)
                    .map_err(|e| bad_request(e.to_string()))?;
                out.insert(
                    "mean_plaquette".to_string(),
                    serde_json::Value::from(v),
                );
            }
            "sum_plaquette" => {
                let v = super::plaquette::plaquette_sum(handle.as_ref(), &lat)
                    .map_err(|e| bad_request(e.to_string()))?;
                out.insert("sum_plaquette".to_string(), serde_json::Value::from(v));
            }
            "q_surrogate" => {
                let v = super::q_surrogate::q_surrogate(handle.as_ref(), &lat)
                    .map_err(|e| bad_request(e.to_string()))?;
                out.insert("q_surrogate".to_string(), serde_json::Value::from(v));
            }
            other => {
                return Err(bad_request(format!(
                    "gauge: unknown observable '{other}' (expected mean_plaquette / \
                     sum_plaquette / q_surrogate)"
                )));
            }
        }
    }
    Ok(Json(serde_json::Value::Object(out)))
}

// ── TDD-HAL-IV.8 — Read-only Part IV verb routes ───────────────
//
// `SHOW E_FIELD` (IV.7) → `GET /v1/e_field/{name}` + optional
// `?with_buffer=true` for the full Lie buffer column.
// `SELECT H_TOTAL OF (U, E)` (IV.7) → `GET /v1/gauge_field/{name}/
// h_total?e_field=<name>` returning `{ h_total, kinetic, potential }`.
// `SELECT GAUSS_RESIDUAL_MAX OF (U, E)` (IV.7) → `GET /v1/gauge_field/
// {name}/gauss_residual_max?e_field=<name>[&reduction=covariant|flat]`.
// `GET /v1/symplectic_flow/diagnostics/{run_id}` — reads from the
// process-local LRU cache populated by every `symplectic_flow` call.
//
// **No** dedicated POST routes for `E_FIELD` (locked decision IV-I) or
// `SYMPLECTIC_FLOW` (IV.6 locked decision); both verbs remain reachable
// only through `POST /v1/gql` (parser::execute). The route-table-
// absence is the load-bearing receipt the test suite asserts at gate
// IV.8.

/// Query string for `GET /v1/e_field/{name}`. `with_buffer=true`
/// materializes the full `(n_edges, 4)` Lie buffer under the `buffer`
/// column; default (`false` / omitted) emits metadata only.
#[derive(Deserialize)]
pub struct EFieldQuery {
    #[serde(default)]
    pub with_buffer: bool,
}

/// Wire envelope for `GET /v1/e_field/{name}`. Mirrors the GQL `SHOW
/// E_FIELD` shape — lowercased `init_kind` to match the HTTP convention
/// (`identity` / `haar_random` on the gauge field route) the consumer
/// already has in hand.
#[derive(Serialize)]
pub struct EFieldView {
    pub name: String,
    pub source_gauge_field: String,
    pub source_lattice: String,
    pub group: String,
    pub repr_dim: usize,
    pub n_edges: usize,
    pub init_kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_beta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_from: Option<String>,
    /// Row-major `(n_edges, 4)` Lie buffer (q0=0 invariant on every
    /// row). Only present when the query param `with_buffer=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer: Option<Vec<Vec<f64>>>,
}

fn e_field_init_kind_label(kind: &super::e_field::EFieldInit) -> &'static str {
    match kind {
        super::e_field::EFieldInit::Zero => "zero",
        super::e_field::EFieldInit::MaxwellBoltzmann { .. } => "maxwell_boltzmann",
        super::e_field::EFieldInit::FromField(_) => "from_field",
    }
}

async fn e_field_get(
    Path(name): Path<String>,
    Query(q): Query<EFieldQuery>,
) -> Result<Json<EFieldView>, (StatusCode, Json<ErrorEnvelope>)> {
    let handle = gauge_registry::get_su2_e(&name).ok_or_else(|| {
        bad_request(GaugeFieldError::EFieldNotDeclared(name.clone()).to_string())
    })?;
    let buf = handle.as_dense_buffer();
    let (kind, init_seed) = handle.init_metadata();
    let init_kind = e_field_init_kind_label(&kind);
    let (init_beta, init_from) = match &kind {
        super::e_field::EFieldInit::Zero => (None, None),
        super::e_field::EFieldInit::MaxwellBoltzmann { beta } => (Some(*beta), None),
        super::e_field::EFieldInit::FromField(src) => (None, Some(src.clone())),
    };
    let buffer = if q.with_buffer {
        let d = buf.repr_dim;
        if buf.data.len() != buf.n_edges * d {
            return Err(internal(format!(
                "gauge: e field buffer shape mismatch (expected {} f64s, got {})",
                buf.n_edges * d,
                buf.data.len()
            )));
        }
        let mut rows: Vec<Vec<f64>> = Vec::with_capacity(buf.n_edges);
        for e in 0..buf.n_edges {
            rows.push(buf.data[e * d..(e + 1) * d].to_vec());
        }
        Some(rows)
    } else {
        None
    };
    Ok(Json(EFieldView {
        name: handle.name().to_string(),
        source_gauge_field: handle.source_gauge_field().to_string(),
        source_lattice: handle.source_lattice().to_string(),
        group: handle.group().label().to_string(),
        repr_dim: buf.repr_dim,
        n_edges: buf.n_edges,
        init_kind,
        init_seed,
        init_beta,
        init_from,
        buffer,
    }))
}

/// Query string for `GET /v1/gauge_field/{name}/h_total`. The `e_field`
/// parameter is mandatory — `H_total` is the joint (U, E) observable
/// (locked decision IV-J).
#[derive(Deserialize)]
pub struct HTotalQuery {
    pub e_field: String,
}

/// Wire envelope for `H_total`. `kinetic = g²·Σ|E|²`,
/// `potential = (F/g²)·(1 - ⟨P⟩)`, `h_total = kinetic + potential`. The
/// β consumed here comes from the E field's `MaxwellBoltzmann` init
/// metadata when present, else 1.0 (matches the executor's HTotal arm —
/// the diagnostic surface; in-flow HTotal uses the flow's β).
#[derive(Serialize)]
pub struct HTotalEnvelope {
    pub gauge_field: String,
    pub e_field: String,
    pub kinetic: f64,
    pub potential: f64,
    pub h_total: f64,
}

async fn h_total_get(
    Path(u_name): Path<String>,
    Query(q): Query<HTotalQuery>,
) -> Result<Json<HTotalEnvelope>, (StatusCode, Json<ErrorEnvelope>)> {
    // Resolve U through the SU(2)-mut surface (the only one that exposes
    // the concrete SU2GaugeField the H_total formula reads). Mirrors
    // the executor's SELECT H_TOTAL arm.
    let u_handle = gauge_registry::get(&u_name).ok_or_else(|| {
        bad_request(GaugeFieldError::FieldNotDeclared(u_name.clone()).to_string())
    })?;
    if !matches!(u_handle.group(), Group::SU2) {
        return Err(bad_request(GaugeFieldError::UnsupportedGroup(u_handle.group()).to_string()));
    }
    let u_arc = gauge_registry::get_su2_mut(&u_name).ok_or_else(|| {
        bad_request(GaugeFieldError::FieldNotDeclared(u_name.clone()).to_string())
    })?;
    let e_arc = gauge_registry::get_su2_e_mut(&q.e_field).ok_or_else(|| {
        bad_request(GaugeFieldError::EFieldNotDeclared(q.e_field.clone()).to_string())
    })?;
    let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
    let e_guard = e_arc.lock().expect("e field mutex poisoned");
    if e_guard.source_lattice != u_guard.lattice_name {
        return Err(bad_request(
            GaugeFieldError::EFieldSourceMismatch {
                e_lattice: e_guard.source_lattice.clone(),
                u_lattice: u_guard.lattice_name.clone(),
            }
            .to_string(),
        ));
    }
    let lat = lattice_registry::get(&u_guard.lattice_name).ok_or_else(|| {
        bad_request(
            GaugeFieldError::LatticeNotDeclared(u_guard.lattice_name.clone()).to_string(),
        )
    })?;
    let beta = match &e_guard.init_kind {
        super::e_field::EFieldInit::MaxwellBoltzmann { beta } => *beta,
        _ => 1.0_f64,
    };
    let g2 = 4.0_f64 / beta;
    let mut kin = 0.0_f64;
    for edge in 0..e_guard.buffer.n_edges {
        let row = e_guard.read_element_q(edge);
        kin += row[1] * row[1] + row[2] * row[2] + row[3] * row[3];
    }
    let kinetic = g2 * kin;
    let p_mean = super::plaquette::plaquette_mean(&*u_guard, &lat)
        .map_err(|e| bad_request(e.to_string()))?;
    let potential = (lat.n_faces() as f64) * (1.0_f64 - p_mean) / g2;
    let h_total = kinetic + potential;
    Ok(Json(HTotalEnvelope {
        gauge_field: u_name,
        e_field: q.e_field,
        kinetic,
        potential,
        h_total,
    }))
}

/// Query string for `GET /v1/gauge_field/{name}/gauss_residual_max`.
/// `reduction` defaults to `covariant` (Halcyon production-canonical
/// per locked decision IV-G); `flat` is the debug path.
#[derive(Deserialize)]
pub struct GaussResidualQuery {
    pub e_field: String,
    #[serde(default = "default_gauss_reduction")]
    pub reduction: String,
}

fn default_gauss_reduction() -> String {
    "covariant".to_string()
}

#[derive(Serialize)]
pub struct GaussResidualEnvelope {
    pub gauge_field: String,
    pub e_field: String,
    pub reduction: &'static str,
    pub gauss_residual_max: f64,
}

async fn gauss_residual_max_get(
    Path(u_name): Path<String>,
    Query(q): Query<GaussResidualQuery>,
) -> Result<Json<GaussResidualEnvelope>, (StatusCode, Json<ErrorEnvelope>)> {
    let u_handle = gauge_registry::get(&u_name).ok_or_else(|| {
        bad_request(GaugeFieldError::FieldNotDeclared(u_name.clone()).to_string())
    })?;
    if !matches!(u_handle.group(), Group::SU2) {
        return Err(bad_request(GaugeFieldError::UnsupportedGroup(u_handle.group()).to_string()));
    }
    let u_arc = gauge_registry::get_su2_mut(&u_name).ok_or_else(|| {
        bad_request(GaugeFieldError::FieldNotDeclared(u_name.clone()).to_string())
    })?;
    let e_arc = gauge_registry::get_su2_e_mut(&q.e_field).ok_or_else(|| {
        bad_request(GaugeFieldError::EFieldNotDeclared(q.e_field.clone()).to_string())
    })?;
    let u_guard = u_arc.lock().expect("su2 field mutex poisoned");
    let e_guard = e_arc.lock().expect("e field mutex poisoned");
    if e_guard.source_lattice != u_guard.lattice_name {
        return Err(bad_request(
            GaugeFieldError::EFieldSourceMismatch {
                e_lattice: e_guard.source_lattice.clone(),
                u_lattice: u_guard.lattice_name.clone(),
            }
            .to_string(),
        ));
    }
    let lat = lattice_registry::get(&u_guard.lattice_name).ok_or_else(|| {
        bad_request(
            GaugeFieldError::LatticeNotDeclared(u_guard.lattice_name.clone()).to_string(),
        )
    })?;
    let vinc = super::gauss::build_vertex_edge_incidence(&lat);
    let (reduction_label, residuals) = match q.reduction.to_ascii_lowercase().as_str() {
        "covariant" => (
            "covariant",
            super::gauss::compute_gauss_residual_covariant(
                &*u_guard, &*e_guard, &lat, &vinc,
            )
            .map_err(|e| bad_request(e.to_string()))?,
        ),
        "flat" => (
            "flat",
            super::gauss::compute_gauss_residual_flat(&*e_guard, &lat, &vinc)
                .map_err(|e| bad_request(e.to_string()))?,
        ),
        other => {
            return Err(bad_request(format!(
                "gauge: unknown gauss-residual reduction '{other}' (expected covariant / flat)"
            )));
        }
    };
    let max = super::gauss::max_inf_norm(&residuals);
    Ok(Json(GaussResidualEnvelope {
        gauge_field: u_name,
        e_field: q.e_field,
        reduction: reduction_label,
        gauss_residual_max: max,
    }))
}

/// JSON envelope for the diagnostics block — flat copy of
/// `SymplecticFlowDiagnostics`. Serializable shape; the cache stores
/// the full `SymplecticFlowResponse` and we project it into JSON here.
#[derive(Serialize)]
pub struct SymplecticFlowDiagnosticsView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub beta: f64,
    pub dt: f64,
    pub n_steps_completed: usize,
    pub cg_iterations_per_step_p99: f64,
    pub max_energy_drift_rel: f64,
    pub gauss_residual_max: f64,
}

#[derive(Serialize)]
pub struct SymplecticFlowDiagnosticsEnvelope {
    pub run_id: String,
    pub field: String,
    pub e_field: String,
    pub measurement_history: std::collections::HashMap<String, Vec<f64>>,
    pub diagnostics: SymplecticFlowDiagnosticsView,
}

async fn symplectic_flow_diagnostics_get(
    Path(run_id): Path<String>,
) -> Result<Json<SymplecticFlowDiagnosticsEnvelope>, (StatusCode, Json<ErrorEnvelope>)> {
    let resp = flow_mod::get_diagnostics(&run_id).ok_or_else(|| {
        not_found(format!(
            "gauge: symplectic_flow run_id '{run_id}' not found in the \
             process-local diagnostics cache (the cache holds the last \
             32 runs of this process lifetime; restarts clear it)"
        ))
    })?;
    let mut measurement_history: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::with_capacity(resp.measurement_history.len());
    for (obs, chain) in resp.measurement_history.iter() {
        measurement_history.insert(obs.label().to_string(), chain.clone());
    }
    Ok(Json(SymplecticFlowDiagnosticsEnvelope {
        run_id: resp.run_id.clone(),
        field: resp.field.clone(),
        e_field: resp.e_field.clone(),
        measurement_history,
        diagnostics: SymplecticFlowDiagnosticsView {
            seed: resp.diagnostics.seed,
            beta: resp.diagnostics.beta,
            dt: resp.diagnostics.dt,
            n_steps_completed: resp.diagnostics.n_steps_completed,
            cg_iterations_per_step_p99: resp.diagnostics.cg_iterations_per_step_p99,
            max_energy_drift_rel: resp.diagnostics.max_energy_drift_rel,
            gauss_residual_max: resp.diagnostics.gauss_residual_max,
        },
    }))
}

/// Build the LATTICE + GAUGE_FIELD HTTP router. The gigi-stream binary
/// merges this into its main app; the test harness drives the same
/// router directly via `tower::ServiceExt::oneshot` (no listener).
///
/// The router carries no state — the lattice + gauge registries are
/// process singletons (mirrors `lattice::registry` / `gauge::registry`)
/// and the engine handle used by II.6b's `persist:true` branch is
/// installed via `gauge::engine_handle::install` (option-b
/// module-global), so the HTTP surface threads through them directly.
///
/// Generic in the host app's `State` type so the gigi-stream binary
/// (which has `Router<Arc<StreamState>>`) can `merge` this in without
/// state-type juggling. The handlers themselves don't read host state
/// — they hit the global lattice + gauge registries (in-memory path)
/// or the module-global engine handle (durable path) — so any concrete
/// `S: Clone + Send + Sync + 'static` works.
///
/// TDD-HAL-III.7 additions: `PLAQUETTE` + `Q_SURROGATE` + batched
/// `observables` routes wired alongside the II.6 declaration surface.
/// **No** dedicated `gibbs_sample` route (locked decision D5) — the
/// route table is the load-bearing receipt.
pub fn build_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/v1/lattice", post(lattice_create))
        .route("/v1/lattice/{name}", get(lattice_get))
        .route("/v1/gauge_field", post(gauge_field_create))
        .route("/v1/gauge_field/{name}", get(gauge_field_get))
        // TDD-HAL-III.7 read-only verb routes.
        .route("/v1/gauge_field/{name}/plaquette", get(plaquette_get))
        .route(
            "/v1/gauge_field/{name}/observables/q_surrogate",
            get(q_surrogate_get),
        )
        .route("/v1/gauge_field/{name}/observables", post(observables_post))
        // TDD-HAL-IV.8 read-only Part IV verb routes.
        //
        // **No** POST routes for `/v1/e_field` (locked decision IV-I —
        // E_FIELD embedded-only) or
        // `/v1/gauge_field/{name}/symplectic_flow` (IV.6 locked —
        // SYMPLECTIC_FLOW embedded-only). The route-table absence is
        // the load-bearing receipt the IV.8 test suite asserts.
        .route("/v1/e_field/{name}", get(e_field_get))
        .route("/v1/gauge_field/{name}/h_total", get(h_total_get))
        .route(
            "/v1/gauge_field/{name}/gauss_residual_max",
            get(gauss_residual_max_get),
        )
        .route(
            "/v1/symplectic_flow/diagnostics/{run_id}",
            get(symplectic_flow_diagnostics_get),
        )
}
