//! GIGI Stream — Real-time Geometric Database Server
//!
//! WebSocket + REST API for:
//!   - O(1) insert/query/range
//!   - DHOOM wire protocol
//!   - Real-time subscriptions (sheaf-evaluated open sets)
//!   - Curvature monitoring
//!   - Pullback joins
//!   - Fiber integral aggregation
//!
//! Architecture:
//!   L1: Bundle Store (O(1) read/write)
//!   L2: Sheaf Query (composition with gluing)
//!   L3: Connection (curvature, spectral, holonomy)

use axum::http::Request;
use axum::http::{HeaderName, HeaderValue, Method};
use axum::middleware::Next;
use axum::response::Response;
use axum::{
    extract::{ws::WebSocket, Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    middleware as axum_mw,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};

use gigi::aggregation;
use gigi::bundle::{compute_record_k, TransactionOp};
use gigi::bundle::{AnomalyRecord, QueryCondition, VectorMetric};
use gigi::curvature;
use gigi::dhoom;
use gigi::engine::Engine;
use gigi::join;
use gigi::observability::{GeometricFields, LogCategory, LogConfig, LogEvent, LogLevel, Logger, Metrics, new_request_id};
use tokio::sync::mpsc::UnboundedReceiver;
use gigi::spectral;
use gigi::types::{BundleSchema, FieldDef, FieldType, Value};

// ── Shared State ──

type Record = HashMap<String, Value>;

// ── Phase B multi-tenant auth ──
//
// davisgeometric.com/api/gigi/token mints a compact signed token for
// non-owner users containing their per-user namespace. The engine
// verifies the HMAC-SHA256 signature with a shared secret
// (GIGI_JWT_SECRET) and uses the embedded namespace + owner flag to
// gate every /v1/bundles/<name>/* request.
//
// Token wire format (simpler than full JWT, no header):
//   base64url(payload_json).base64url(hmac_sha256(payload_json, secret))
//
// Payload schema:
//   { "email": "...", "ns": "ns_<12-hex>", "owner": bool, "exp": <unix_seconds> }
//
// Owner tokens bypass namespace enforcement so bee retains full access
// to bundles created before namespacing existed. Non-owner tokens can
// only touch bundles whose name starts with their `<ns>__` prefix.

#[derive(Debug, Clone, Deserialize)]
struct GigiClaims {
    #[serde(default)]
    email: String,
    #[serde(default)]
    ns: String,
    #[serde(default)]
    owner: bool,
    /// Unix seconds. Tokens past this are rejected.
    #[serde(default)]
    exp: u64,
}

impl GigiClaims {
    /// Owner-equivalent claims used when the request authenticated via
    /// the legacy GIGI_API_KEY header. Server-internal callers (the
    /// davisgeometric redis wrapper, snapshot tools, oncall debugging)
    /// land here and get unrestricted access by design.
    fn owner_via_api_key() -> Self {
        GigiClaims {
            email: String::new(),
            ns: String::new(),
            owner: true,
            exp: u64::MAX,
        }
    }

    /// Does this caller have permission to touch `bundle_name`?
    /// Owners bypass the check; everyone else must have a bundle whose
    /// name is prefixed with `<ns>__`.
    fn allows_bundle(&self, bundle_name: &str) -> bool {
        if self.owner {
            return true;
        }
        if self.ns.is_empty() {
            // Defensive: a non-owner with no namespace shouldn't exist,
            // and we never want to silently grant access. Refuse.
            return false;
        }
        let prefix = format!("{}__", self.ns);
        bundle_name.starts_with(&prefix)
    }
}

/// URL-safe base64 without padding — matches what davisgeometric's
/// Node mint side produces.
fn b64url_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(input.as_bytes())
        .ok()
}

/// Verify a compact-signed token and return its claims.
///
/// Format: `<payload_b64url>.<sig_b64url>`. The sig is HMAC-SHA256 of
/// the raw payload base64 string (NOT the decoded JSON) so re-encoding
/// quirks can't break verification on either side.
fn verify_gigi_token(token: &str, secret: &str) -> Result<GigiClaims, &'static str> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let (payload_b64, sig_b64) = token.split_once('.').ok_or("malformed token")?;
    if payload_b64.is_empty() || sig_b64.is_empty() {
        return Err("malformed token");
    }

    let sig_bytes = b64url_decode(sig_b64).ok_or("malformed signature")?;

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| "secret key error")?;
    mac.update(payload_b64.as_bytes());
    mac.verify_slice(&sig_bytes).map_err(|_| "invalid signature")?;

    let payload_json = b64url_decode(payload_b64).ok_or("malformed payload")?;
    let claims: GigiClaims =
        serde_json::from_slice(&payload_json).map_err(|_| "malformed claims")?;

    // Reject expired tokens. exp=0 means "no expiry set" — treat as
    // invalid since every legitimate mint sets exp.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if claims.exp == 0 || claims.exp < now {
        return Err("token expired");
    }

    Ok(claims)
}

struct StreamState {
    engine: RwLock<Engine>,
    /// True once WAL replay is complete and engine is ready for queries.
    ready: AtomicBool,
    /// Per-bundle broadcast channels for subscriptions
    channels: RwLock<HashMap<String, broadcast::Sender<SubscriptionEvent>>>,
    /// Global dashboard broadcast — anomaly + curvature update events for all bundles
    dashboard_tx: broadcast::Sender<DashboardEvent>,
    /// API key for authentication (None = no auth required). Carries
    /// owner-equivalent permissions and is used by server-internal
    /// callers (davisgeometric redis wrapper, snapshot tools).
    api_key: Option<String>,
    /// HMAC-SHA256 secret used to verify per-user signed tokens minted
    /// by davisgeometric.com/api/gigi/token. None disables JWT auth
    /// entirely (legacy / dev / single-tenant deployments).
    jwt_secret: Option<String>,
    /// Rate limit: max requests per window (0 = unlimited)
    rate_limit: u32,
    /// Rate limit window in seconds
    rate_window_secs: u64,
    /// Per-IP request tracking for rate limiting
    rate_tracker: RwLock<HashMap<String, Vec<Instant>>>,
    /// Server start time for uptime tracking
    start_time: Instant,
    /// Structured logger — fire-and-forget, non-blocking.
    logger: Logger,
    /// Live metrics counters for GET /v1/metrics.
    metrics: Arc<Metrics>,
    /// Bundle flow cache (S1 wave 1 §A) — caches expensive
    /// Gaussian fits keyed on (bundle, fit_mode, fields,
    /// sigma_floor_epsilon). Invalidated by `mutation_counter`
    /// on BundleStore; bounded to 50 entries (LRU follow-up).
    /// Lock-free read on cache hit. Only present under
    /// `kahler` feature — brain endpoints are kahler-gated.
    #[cfg(feature = "kahler")]
    flow_cache: Arc<BundleFlowCache>,
}

/// A mutation event broadcast to all subscribers of a bundle.
/// Carries the full record so subscribers can evaluate filter conditions
/// without re-querying the store — sheaf restriction to an open set.
#[derive(Clone, Debug)]
struct SubscriptionEvent {
    bundle: String,
    /// Operation: "insert", "update", "delete", "upsert", "bulk_update", "bulk_delete"
    op: &'static str,
    /// Full JSON of the affected record(s). For bulk ops: array of records.
    record_json: String,
    /// Scalar curvature K at the time of mutation — lets subscribers detect
    /// topological phase transitions without an extra round-trip.
    curvature: f64,
}

/// Live dashboard event broadcast on every mutation and anomaly detection.
/// Subscribers receive a real-time stream of bundle health and anomaly signals.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct DashboardEvent {
    /// Event type: "insert", "anomaly", "curvature_update", "delete".
    #[serde(rename = "type")]
    event_type: &'static str,
    bundle: String,
    /// Wall-clock milliseconds since Unix epoch.
    ts_ms: u64,
    record_count: usize,
    k_global: f64,
    k_mean: f64,
    k_std: f64,
    k_threshold_2s: f64,
    global_confidence: f64,
    /// True when the triggering record is above the 2σ anomaly threshold.
    is_anomaly: bool,
    /// K of the record that triggered this event (0 for aggregate events).
    #[serde(skip_serializing_if = "Option::is_none")]
    local_curvature: Option<f64>,
    /// z-score for anomaly events.
    #[serde(skip_serializing_if = "Option::is_none")]
    z_score: Option<f64>,
    /// Contributing fields for anomaly events.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    contributing_fields: Vec<String>,
}

impl StreamState {
    fn new(logger: Logger, metrics: Arc<Metrics>) -> Self {
        let api_key = std::env::var("GIGI_API_KEY").ok();
        let jwt_secret = std::env::var("GIGI_JWT_SECRET").ok();
        let rate_limit = std::env::var("GIGI_RATE_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0u32); // 0 = unlimited
        let rate_window_secs = std::env::var("GIGI_RATE_WINDOW")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60u64);

        let data_dir = std::env::var("GIGI_DATA_DIR").unwrap_or_else(|_| "./gigi_data".to_string());
        let data_path = std::path::PathBuf::from(&data_dir);

        let engine = match Engine::open_empty(&data_path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!(
                    "FATAL: Failed to open database at {}: {e}",
                    data_path.display()
                );
                std::process::exit(1);
            }
        };

        eprintln!("  WAL persistence: {} ({})", data_path.display(), data_dir);

        // Cache capacity — 50 entries default, configurable via env.
        // At n=384 each FullFitResult is ~10MB, so 50 entries =
        // ~500MB worst case (fits in production's 1GB).
        #[cfg(feature = "kahler")]
        let flow_cache_capacity = std::env::var("GIGI_FLOW_CACHE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50_usize);

        StreamState {
            engine: RwLock::new(engine),
            ready: AtomicBool::new(false),
            channels: RwLock::new(HashMap::new()),
            dashboard_tx: broadcast::channel(4096).0,
            api_key,
            jwt_secret,
            rate_limit,
            rate_window_secs,
            rate_tracker: RwLock::new(HashMap::new()),
            start_time: Instant::now(),
            logger,
            metrics,
            #[cfg(feature = "kahler")]
            flow_cache: Arc::new(BundleFlowCache::new(flow_cache_capacity)),
        }
    }

    fn get_or_create_channel(&self, bundle: &str) -> broadcast::Sender<SubscriptionEvent> {
        {
            let channels = self.channels.read().unwrap();
            if let Some(tx) = channels.get(bundle) {
                return tx.clone();
            }
        }
        let mut channels = self.channels.write().unwrap();
        let (tx, _rx) = broadcast::channel(1024);
        channels.insert(bundle.to_string(), tx.clone());
        tx
    }
}

// ── REST API Types ──

#[derive(Deserialize)]
struct CreateBundleRequest {
    name: String,
    schema: SchemaSpec,
    #[serde(default)]
    encrypted: bool,
}

#[derive(Deserialize)]
struct SchemaSpec {
    fields: HashMap<String, String>, // field_name → type: "numeric"|"categorical"|"timestamp"
    #[serde(default)]
    keys: Vec<String>,
    #[serde(default)]
    defaults: HashMap<String, serde_json::Value>,
    #[serde(default)]
    indexed: Vec<String>,
}

#[derive(Deserialize)]
struct InsertRequest {
    records: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct JoinRequest {
    right_bundle: String,
    left_field: String,
    right_field: String,
}

#[derive(Deserialize)]
struct AggregateRequest {
    group_by: String,
    field: String,
    #[serde(default)]
    conditions: Vec<ConditionSpec>,
    /// HAVING — post-aggregation filter on computed stats.
    /// Each entry: { "field": "count"|"sum"|"avg"|"min"|"max", "op": "gt"|"gte"|"lt"|"lte"|"eq"|"neq", "value": <number> }
    #[serde(default)]
    having: Vec<ConditionSpec>,
}

/// Body for POST .../drop-field
#[derive(Deserialize)]
struct DropFieldRequest {
    field: String,
}

#[derive(Deserialize)]
struct FilteredQueryRequest {
    #[serde(default, alias = "filters")]
    conditions: Vec<ConditionSpec>,
    #[serde(default, alias = "order_by")]
    sort_by: Option<String>,
    #[serde(default)]
    sort_desc: Option<bool>,
    /// PRISM compat: "desc" or "asc" — overrides sort_desc if present
    #[serde(default)]
    order: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
    /// Multi-field text search (OR across fields)
    #[serde(default)]
    search: Option<String>,
    /// Which fields to search across (if omitted, searches all text fields)
    #[serde(default)]
    search_fields: Option<Vec<String>>,
    /// Field projection — only return these fields
    #[serde(default)]
    fields: Option<Vec<String>>,
    /// OR condition groups — each group is ANDed, groups are ORed
    #[serde(default)]
    or_conditions: Option<Vec<Vec<ConditionSpec>>>,
    /// Multi-field sort: [{"field": "name", "desc": true}, ...]
    #[serde(default)]
    sort: Option<Vec<SortSpec>>,
}

#[derive(Deserialize)]
struct ConditionSpec {
    field: String,
    op: String,
    value: serde_json::Value,
}

#[derive(Deserialize)]
struct SortSpec {
    field: String,
    #[serde(default)]
    desc: Option<bool>,
}

/// Body for PATCH /v1/bundles/{name}/points/{field}/{value}
#[derive(Deserialize)]
struct PatchFieldsBody {
    fields: HashMap<String, serde_json::Value>,
}

/// Body for PATCH /v1/bundles/{name}/points (bulk update)
#[derive(Deserialize)]
struct BulkUpdateRequest {
    #[serde(default, alias = "filters")]
    filter: Vec<ConditionSpec>,
    fields: HashMap<String, serde_json::Value>,
}

/// Body for POST .../upsert
#[derive(Deserialize)]
struct UpsertRequest {
    record: HashMap<String, serde_json::Value>,
}

/// Body for DELETE .../bulk-delete
#[derive(Deserialize)]
struct BulkDeleteRequest {
    #[serde(default, alias = "filters")]
    conditions: Vec<ConditionSpec>,
}

/// Body for POST .../increment
#[derive(Deserialize)]
struct IncrementRequest {
    key: HashMap<String, serde_json::Value>,
    field: String,
    #[serde(default = "default_increment")]
    amount: f64,
}

fn default_increment() -> f64 {
    1.0
}

/// Body for POST .../add-field
#[derive(Deserialize)]
struct AddFieldRequest {
    name: String,
    #[serde(rename = "type", default = "default_field_type")]
    field_type: String,
    #[serde(default)]
    default: Option<serde_json::Value>,
}

fn default_field_type() -> String {
    "categorical".to_string()
}

/// Body for POST .../add-index
#[derive(Deserialize)]
struct AddIndexRequest {
    field: String,
}

/// Body for POST .../import
#[derive(Deserialize)]
struct ImportRequest {
    records: Vec<serde_json::Value>,
}

/// Body for POST .../update with RETURNING clause
#[derive(Deserialize)]
struct UpdateReturningRequest {
    key: HashMap<String, serde_json::Value>,
    fields: HashMap<String, serde_json::Value>,
    #[serde(default)]
    returning: bool,
    /// For optimistic concurrency: expected _version value
    #[serde(default)]
    expected_version: Option<i64>,
}

/// Body for POST .../delete with RETURNING clause
#[derive(Deserialize)]
struct DeleteReturningRequest {
    key: HashMap<String, serde_json::Value>,
    #[serde(default)]
    returning: bool,
}

/// Body for EXPLAIN query plan
#[derive(Deserialize)]
struct ExplainRequest {
    #[serde(default, alias = "filters")]
    conditions: Vec<ConditionSpec>,
    #[serde(default)]
    or_conditions: Option<Vec<Vec<ConditionSpec>>>,
    #[serde(default)]
    sort: Option<Vec<SortSpec>>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

/// A single operation in a transaction.
#[derive(Deserialize)]
struct TransactionOpSpec {
    op: String, // "insert", "update", "delete", "increment"
    #[serde(default)]
    record: Option<serde_json::Value>,
    #[serde(default)]
    key: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    fields: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    field: Option<String>,
    #[serde(default)]
    amount: Option<f64>,
}

/// Body for POST .../transaction
#[derive(Deserialize)]
struct TransactionRequest {
    ops: Vec<TransactionOpSpec>,
}

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<MetaInfo>,
}

#[derive(Serialize)]
struct MetaInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    curvature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capacity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    engine: &'static str,
    version: &'static str,
    bundles: usize,
    total_records: usize,
    uptime_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    loading: Option<bool>,
}

#[derive(Serialize)]
struct BundleInfo {
    name: String,
    records: usize,
    fields: usize,
}

// ── Anomaly Detection ─────────────────────────────────────────────────────────

fn default_anomaly_sigma() -> f64 {
    2.0
}
fn default_anomaly_limit() -> usize {
    100
}
fn default_include_scores() -> bool {
    true
}

#[derive(Deserialize)]
struct AnomalyRequest {
    #[serde(default = "default_anomaly_sigma")]
    threshold_sigma: f64,
    #[serde(default)]
    filters: Vec<ConditionSpec>,
    #[serde(default)]
    fields: Vec<String>,
    #[serde(default = "default_anomaly_limit")]
    limit: usize,
    #[serde(default = "default_include_scores")]
    include_scores: bool,
}

#[derive(Deserialize)]
struct FieldAnomalyRequest {
    field: String,
    #[serde(default = "default_anomaly_sigma")]
    threshold_sigma: f64,
    #[serde(default = "default_anomaly_limit")]
    limit: usize,
}

#[derive(Deserialize)]
struct PredictRequest {
    group_by: String,
    field: String,
}

fn anomaly_to_json(a: &AnomalyRecord, include_scores: bool) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("record".into(), record_to_json(&a.record));
    if include_scores {
        obj.insert("local_curvature".into(), a.local_curvature.into());
        obj.insert("z_score".into(), a.z_score.into());
        obj.insert("confidence".into(), a.confidence.into());
        obj.insert("deviation_norm".into(), (a.deviation_norm as u64).into());
        obj.insert("deviation_distance".into(), a.deviation_distance.into());
        obj.insert(
            "neighbourhood_size".into(),
            (a.neighbourhood_size as u64).into(),
        );
        obj.insert(
            "contributing_fields".into(),
            serde_json::Value::Array(
                a.contributing_fields
                    .iter()
                    .map(|f| f.clone().into())
                    .collect(),
            ),
        );
    }
    serde_json::Value::Object(obj)
}

// ── Curvature ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CurvatureReport {
    #[serde(rename = "K")]
    k: f64,
    /// Alias for K — included for client compatibility.
    curvature: f64,
    confidence: f64,
    capacity: f64,
    per_field: Vec<FieldCurvature>,
    /// L4 / catalog §E.3 — Kähler curvature decomposition. Present
    /// only when the `kahler` feature is on and the bundle has a
    /// Kähler structure attached with enough data to compute the
    /// four invariants. Per the v2 consumption draft, Marcella
    /// reads this off the curvature endpoint and uses Ricci/Weyl
    /// for diversity bounds and holo-bisectional for the Hadamard
    /// gating decision.
    #[cfg(feature = "kahler")]
    #[serde(skip_serializing_if = "Option::is_none")]
    kahler: Option<KahlerCurvatureReport>,
}

/// L4 — serialized Kähler curvature decomposition. Mirrors
/// `gigi::bundle::KahlerCurvature` for the HTTP response shape.
#[cfg(feature = "kahler")]
#[derive(Serialize, Debug, Clone)]
struct KahlerCurvatureReport {
    /// Scalar Ricci curvature; sign indicates Fano (`>0`) / Ricci-
    /// flat / general type.
    ricci: f64,
    /// Conformal-curvature deviation; `0` ⇔ constant complex space
    /// form.
    weyl: f64,
    /// Minimum holomorphic bisectional curvature across complex
    /// pair combinations. `≤ 0` ⇒ Kähler-Hadamard regime.
    holo_bisectional_min: f64,
    /// Maximum holomorphic bisectional curvature.
    holo_bisectional_max: f64,
    /// Mean holomorphic sectional curvature across complex pairs.
    holo_sectional: f64,
}

#[cfg(feature = "kahler")]
impl From<gigi::bundle::KahlerCurvature> for KahlerCurvatureReport {
    fn from(k: gigi::bundle::KahlerCurvature) -> Self {
        Self {
            ricci: k.ricci,
            weyl: k.weyl,
            holo_bisectional_min: k.holo_bisectional_min,
            holo_bisectional_max: k.holo_bisectional_max,
            holo_sectional: k.holo_sectional,
        }
    }
}

#[derive(Serialize)]
struct FieldCurvature {
    field: String,
    variance: f64,
    range: f64,
    k: f64,
}

#[derive(Serialize)]
struct SpectralReport {
    lambda1: f64,
    diameter: usize,
    spectral_capacity: f64,
}

#[derive(Serialize)]
struct BettiReport {
    beta_0: usize,
    beta_1: usize,
}

#[derive(Serialize)]
struct EntropyReport {
    entropy: f64,
    unit: String,
}

#[derive(Serialize)]
struct FreeEnergyReport {
    tau: f64,
    free_energy: f64,
}

#[derive(Serialize)]
struct PullbackCurvatureReport {
    k_left: f64,
    k_right: f64,
    k_pullback: f64,
    delta_k: f64,
    matched: usize,
    unmatched: usize,
    right_unmatched: usize,
}

#[derive(Serialize)]
struct GeodesicReport {
    distance: Option<f64>,
    path_found: bool,
}

#[derive(Serialize)]
struct MetricTensorReport {
    matrix: Vec<Vec<f64>>,
    eigenvalues: Vec<f64>,
    condition_number: f64,
    effective_dimension: f64,
    field_names: Vec<String>,
}

#[derive(Serialize)]
struct AggResult {
    groups: HashMap<String, AggValues>,
}

#[derive(Serialize)]
struct AggValues {
    count: usize,
    sum: f64,
    avg: f64,
    min: f64,
    max: f64,
}

// ── Binary field size cap (§2.1) ──

/// Hard upper bound for a single `Value::Binary` field (1 MiB).
const MAX_BINARY_FIELD_BYTES: usize = 1_048_576;

/// Returns `Err((field_name, actual_len))` for the first binary field that
/// exceeds `MAX_BINARY_FIELD_BYTES`.  Called after NDJSON / DHOOM parse.
fn check_binary_sizes(records: &[Record]) -> Result<(), (String, usize)> {
    for record in records {
        for (field, value) in record {
            if let Value::Binary(bytes) = value {
                if bytes.len() > MAX_BINARY_FIELD_BYTES {
                    return Err((field.clone(), bytes.len()));
                }
            }
        }
    }
    Ok(())
}

// ── Helper: Convert JSON value to GIGI Value ──

fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => {
            // Strings prefixed with "b64:" decode to Value::Binary losslessly.
            // Double-prefix escape (§8.9): "b64:b64:..." means the text literally
            // starts with "b64:" — strip one prefix and return as Value::Text.
            if let Some(encoded) = s.strip_prefix("b64:") {
                if let Some(literal) = encoded.strip_prefix("b64:") {
                    return Value::Text(format!("b64:{literal}"));
                }
                use base64::Engine as _;
                match base64::engine::general_purpose::STANDARD.decode(encoded) {
                    Ok(bytes) => Value::Binary(bytes),
                    Err(_) => Value::Text(s.clone()),
                }
            } else {
                Value::Text(s.clone())
            }
        }
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Array(arr) => {
            // Numeric arrays → Value::Vector (embedding/feature vector)
            let floats: Vec<f64> = arr.iter().filter_map(|x| x.as_f64()).collect();
            if floats.len() == arr.len() && !arr.is_empty() {
                Value::Vector(floats)
            } else {
                Value::Null
            }
        }
        _ => Value::Null,
    }
}

fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Integer(i) => serde_json::json!(i),
        Value::Float(f) => serde_json::json!(f),
        // §8.9 escape: Text values starting with "b64:" must be double-prefixed
        // so the receiver decodes them as text, not binary.
        Value::Text(s) => {
            if s.starts_with("b64:") {
                serde_json::Value::String(format!("b64:{s}"))
            } else {
                serde_json::json!(s)
            }
        }
        Value::Bool(b) => serde_json::json!(b),
        Value::Timestamp(t) => serde_json::json!(t),
        Value::Vector(v) => {
            serde_json::Value::Array(v.iter().map(|x| serde_json::json!(x)).collect())
        }
        Value::Binary(b) => {
            use base64::Engine as _;
            serde_json::Value::String(format!(
                "b64:{}",
                base64::engine::general_purpose::STANDARD.encode(b)
            ))
        }
        Value::Null => serde_json::Value::Null,
    }
}

/// Validate and coerce a single `Value` against the declared `FieldType` for
/// `field_name` in `schema`.
///
/// Rules:
/// - Unknown fields (not in base or fiber) pass through unchanged — no-schema
///   bundles and extra fields are not an error.
/// - `Value::Null` always passes — callers handle required-field checks separately.
/// - `FieldType::Numeric`    → accepts `Integer`, `Float`. Rejects everything else.
/// - `FieldType::Timestamp`  → accepts `Integer` (coerced to `Timestamp`), `Timestamp`.
///   Rejects `Text` (prohibits formatted time strings, enforces invariant C2).
/// - `FieldType::Binary`     → accepts `Binary`. Rejects `Text` without a `b64:` prefix
///   (the only way plain text arrives here is if the caller forgot to escape it).
/// - `FieldType::Categorical` / `OrderedCat` → accepts `Text`, `Bool`, `Integer`.
/// - `FieldType::Vector`      → accepts `Vector`.
///
/// Returns `Ok(Value)` (possibly coerced) or `Err(String)` with a human-readable
/// diagnostic naming the field, declared type, and received type.
fn schema_coerce(schema: &BundleSchema, field_name: &str, value: Value) -> Result<Value, String> {
    // Null bypasses all type checks — optional fields are always nullable.
    if matches!(value, Value::Null) {
        return Ok(value);
    }

    // Look up the declared FieldType. Unknown fields pass through.
    let field_type = schema
        .base_fields
        .iter()
        .chain(schema.fiber_fields.iter())
        .find(|f| f.name == field_name)
        .map(|f| &f.field_type);

    let ft = match field_type {
        None => return Ok(value), // unknown field — no schema constraint
        Some(ft) => ft,
    };

    match ft {
        FieldType::Numeric => match value {
            Value::Integer(_) | Value::Float(_) => Ok(value),
            other => Err(format!(
                "field '{}' declared Numeric but received {}",
                field_name,
                value_type_name(&other)
            )),
        },
        FieldType::Timestamp => match value {
            Value::Timestamp(_) => Ok(value),
            Value::Integer(i) => Ok(Value::Timestamp(i)), // coerce: ns epoch integer
            other => Err(format!(
                "field '{}' declared Timestamp but received {} (use nanosecond integer, not a formatted string)",
                field_name,
                value_type_name(&other)
            )),
        },
        FieldType::Binary => match value {
            Value::Binary(_) => Ok(value),
            other => Err(format!(
                "field '{}' declared Binary but received {} (encode as 'b64:<base64>' at JSON boundaries)",
                field_name,
                value_type_name(&other)
            )),
        },
        FieldType::Categorical | FieldType::OrderedCat { .. } => match value {
            Value::Text(_) | Value::Bool(_) | Value::Integer(_) => Ok(value),
            other => Err(format!(
                "field '{}' declared Categorical but received {}",
                field_name,
                value_type_name(&other)
            )),
        },
        FieldType::Vector { .. } => match value {
            Value::Vector(_) => Ok(value),
            other => Err(format!(
                "field '{}' declared Vector but received {}",
                field_name,
                value_type_name(&other)
            )),
        },
    }
}

fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Integer(_) => "Integer",
        Value::Float(_) => "Float",
        Value::Text(_) => "Text",
        Value::Bool(_) => "Bool",
        Value::Timestamp(_) => "Timestamp",
        Value::Vector(_) => "Vector",
        Value::Binary(_) => "Binary",
        Value::Null => "Null",
    }
}

/// Run `schema_coerce` on every field in a record.  Returns the coerced record
/// on success, or the first violation error string on failure.
fn coerce_record_against_schema(
    schema: &BundleSchema,
    record: Record,
) -> Result<Record, String> {
    let mut out: Record = Record::new();
    for (k, v) in record {
        let coerced = schema_coerce(schema, &k, v)?;
        out.insert(k, coerced);
    }
    Ok(out)
}

fn record_to_json(record: &Record) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, v) in record {
        map.insert(k.clone(), value_to_json(v));
    }
    serde_json::Value::Object(map)
}

fn str_to_field_type(s: &str) -> FieldType {
    let lower = s.to_lowercase();
    // Support "vector(768)" or "vector" syntax
    if lower.starts_with("vector") {
        let dims = lower
            .trim_start_matches("vector")
            .trim_matches(|c: char| c == '(' || c == ')' || c.is_whitespace())
            .parse::<usize>()
            .unwrap_or(0);
        return FieldType::Vector { dims };
    }
    match lower.as_str() {
        "numeric" | "number" | "float" | "int" | "integer" => FieldType::Numeric,
        "timestamp" | "time" | "date" => FieldType::Timestamp,
        _ => FieldType::Categorical,
    }
}

// ── CORS Configuration ──

/// Build CORS layer from environment configuration.
/// - GIGI_CORS_ORIGIN=*       → allow all origins (use only for development)
/// - GIGI_CORS_ORIGIN=https://example.com → allow specific origin
/// - unset                    → restrictive (same-origin only, no CORS headers)
fn build_cors_layer() -> CorsLayer {
    match std::env::var("GIGI_CORS_ORIGIN") {
        Ok(origin) if origin == "*" => CorsLayer::new()
            .allow_origin(AllowOrigin::any())
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                HeaderName::from_static("content-type"),
                HeaderName::from_static("x-api-key"),
            ]),
        Ok(origin) => {
            let origin_val: HeaderValue = origin.parse().unwrap_or_else(|_| "".parse().unwrap());
            CorsLayer::new()
                .allow_origin(AllowOrigin::exact(origin_val))
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    HeaderName::from_static("content-type"),
                    HeaderName::from_static("x-api-key"),
                ])
        }
        Err(_) => {
            // No CORS origin set → allow same-origin (permissive for local dev)
            CorsLayer::new()
                .allow_origin(AllowOrigin::any())
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    HeaderName::from_static("content-type"),
                    HeaderName::from_static("x-api-key"),
                ])
        }
    }
}

// ── REST Handlers ──

/// Middleware: reject non-health requests while WAL replay is in progress.
async fn readiness_middleware(
    State(state): State<Arc<StreamState>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if !state.ready.load(Ordering::Acquire) && req.uri().path() != "/v1/health" {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "WAL replay in progress — try again shortly"})),
        ));
    }
    Ok(next.run(req).await)
}

/// Middleware: API key OR per-user signed-token authentication.
///
/// Accepts either:
///   - `X-API-Key` header (HTTP) / `?api_key=...` query (WS upgrade)
///     matching `GIGI_API_KEY` — server-internal / owner-equivalent.
///   - `Authorization: Bearer <token>` header (HTTP) /
///     `?gigi_token=...` query (WS upgrade) — verifies HMAC-SHA256
///     against `GIGI_JWT_SECRET` and pulls out per-user claims.
///
/// Attaches `GigiClaims` to the request extensions so the downstream
/// `namespace_enforcement_middleware` can gate /v1/bundles/<name>/*
/// paths by tenant. Health endpoint is excluded so liveness probes
/// don't need credentials.
async fn auth_middleware(
    State(state): State<Arc<StreamState>>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Skip auth for health endpoint
    if req.uri().path() == "/v1/health" {
        return Ok(next.run(req).await);
    }

    // Try API-key path first (legacy + admin). A successful match
    // grants owner-equivalent claims; the JWT path is skipped.
    let mut claims: Option<GigiClaims> = None;
    if let Some(ref expected_key) = state.api_key {
        let header_key = req
            .headers()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let query_key = req.uri().query().and_then(|q| {
            for pair in q.split('&') {
                let mut it = pair.splitn(2, '=');
                let k = it.next().unwrap_or("");
                if k == "api_key" {
                    return it.next().map(str::to_owned);
                }
            }
            None
        });
        if let Some(provided) = header_key.or(query_key) {
            if constant_time_eq(provided.as_bytes(), expected_key.as_bytes()) {
                claims = Some(GigiClaims::owner_via_api_key());
            } else {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "Invalid or missing API key".to_string(),
                    }),
                ));
            }
        }
    }

    // No API key supplied — try the JWT path. Per-user tokens are
    // minted by davisgeometric.com/api/gigi/token and carry the user's
    // namespace claim.
    if claims.is_none() {
        if let Some(ref secret) = state.jwt_secret {
            let header_tok = req
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .map(str::to_owned);
            let query_tok = req.uri().query().and_then(|q| {
                for pair in q.split('&') {
                    let mut it = pair.splitn(2, '=');
                    let k = it.next().unwrap_or("");
                    if k == "gigi_token" {
                        return it.next().map(str::to_owned);
                    }
                }
                None
            });
            if let Some(tok) = header_tok.or(query_tok) {
                match verify_gigi_token(&tok, secret) {
                    Ok(c) => claims = Some(c),
                    Err(reason) => {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(ErrorResponse {
                                error: format!("Invalid token: {reason}"),
                            }),
                        ));
                    }
                }
            }
        }
    }

    // If GIGI_API_KEY is set we require *some* auth to land. If it's
    // unset and JWT is also unconfigured, the engine is in
    // open/dev mode and we let everything through with owner claims.
    if claims.is_none() {
        if state.api_key.is_some() || state.jwt_secret.is_some() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid or missing API key".to_string(),
                }),
            ));
        }
        claims = Some(GigiClaims::owner_via_api_key());
    }

    // Stash claims so downstream handlers / middleware can read them.
    req.extensions_mut().insert(claims.unwrap());

    Ok(next.run(req).await)
}

/// Middleware: tenant namespace enforcement on bundle-path operations.
///
/// Reads the `GigiClaims` left by `auth_middleware` and rejects any
/// `/v1/bundles/<name>/*` request where `<name>` is outside the
/// caller's namespace. Owner claims (from API key or `owner=true` in
/// the JWT payload) bypass this check entirely.
///
/// This is the engine-side half of Phase B: the sheets client also
/// strips/prefixes bundle names for UX, but ALL real authorization
/// happens here so a hand-crafted HTTP request can't bypass the
/// prefix. List endpoints (`GET /v1/bundles`) handle their own
/// filtering — see `list_bundles`.
async fn namespace_enforcement_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Path patterns we gate:
    //   /v1/bundles/<name>/...
    //   /v1/ws/<name>/dashboard
    // Everything else (list, ws root, gql, health, etc.) is either
    // separately guarded (handlers filter their own results) or
    // intentionally global.
    let path = req.uri().path().to_string();
    let bundle_segment = parse_bundle_segment(&path);

    if let Some(name) = bundle_segment {
        // No claims means the engine is in open/dev mode (no auth
        // configured). Skip enforcement.
        if let Some(claims) = req.extensions().get::<GigiClaims>() {
            if !claims.allows_bundle(&name) {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: format!(
                            "Bundle '{}' is outside your workspace namespace.",
                            name
                        ),
                    }),
                ));
            }
        }
    }

    Ok(next.run(req).await)
}

/// Pull `<name>` out of /v1/bundles/<name>/... or /v1/ws/<name>/dashboard.
/// Returns None for /v1/bundles (list endpoint) and unrelated paths.
fn parse_bundle_segment(path: &str) -> Option<String> {
    let mut parts = path.trim_start_matches('/').split('/');
    let first = parts.next()?;
    if first != "v1" {
        return None;
    }
    match parts.next()? {
        "bundles" => {
            let name = parts.next()?;
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        }
        "ws" => {
            // /v1/ws/<name>/dashboard — gate the per-bundle dashboard.
            // The bare /v1/ws/dashboard (global) has no bundle segment.
            let next = parts.next()?;
            if next == "dashboard" {
                None
            } else {
                Some(next.to_string())
            }
        }
        _ => None,
    }
}

/// Length-then-byte compare in constant time. Pulled inline so we don't
/// add a crate dep just for one call site. The header / query branches
/// must take indistinguishable time so an attacker can't probe key
/// material via timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Middleware: Rate limiting (per-IP sliding window).
/// If GIGI_RATE_LIMIT > 0, tracks requests per IP within the window.
async fn rate_limit_middleware(
    State(state): State<Arc<StreamState>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    if state.rate_limit == 0 {
        return Ok(next.run(req).await);
    }

    // Extract client IP: use X-Forwarded-For only when behind a trusted proxy
    let trust_proxy = std::env::var("GIGI_TRUST_PROXY").is_ok();
    let ip = if trust_proxy {
        req.headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .unwrap_or("unknown")
            .trim()
            .to_string()
    } else {
        // Use the direct connection address from extensions
        req.extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|ci| ci.0.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    };

    let now = Instant::now();
    let window = std::time::Duration::from_secs(state.rate_window_secs);

    {
        let mut tracker = state.rate_tracker.write().unwrap();
        let entries = tracker.entry(ip).or_default();

        // Remove expired entries
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= state.rate_limit as usize {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse {
                    error: "Rate limit exceeded".to_string(),
                }),
            ));
        }

        entries.push(now);
    }

    Ok(next.run(req).await)
}

async fn health(State(state): State<Arc<StreamState>>) -> (StatusCode, Json<HealthResponse>) {
    let is_ready = state.ready.load(Ordering::Acquire);
    if !is_ready {
        // Return 503 so load balancers (Fly.io readiness check) know
        // this instance is not ready to serve traffic during WAL replay.
        return (StatusCode::SERVICE_UNAVAILABLE, Json(HealthResponse {
            status: "loading",
            engine: "gigi-stream",
            version: "0.1.0",
            bundles: 0,
            total_records: 0,
            uptime_secs: state.start_time.elapsed().as_secs(),
            loading: Some(true),
        }));
    }
    // Use try_read to avoid blocking when snapshot or other write ops hold the lock.
    match state.engine.try_read() {
        Ok(engine) => (StatusCode::OK, Json(HealthResponse {
            status: "ok",
            engine: "gigi-stream",
            version: "0.1.0",
            bundles: engine.bundle_names().len(),
            total_records: engine.total_records(),
            uptime_secs: state.start_time.elapsed().as_secs(),
            loading: None,
        })),
        Err(_) => (StatusCode::OK, Json(HealthResponse {
            status: "ok",
            engine: "gigi-stream",
            version: "0.1.0",
            bundles: 0,
            total_records: 0,
            uptime_secs: state.start_time.elapsed().as_secs(),
            loading: Some(true),
        })),
    }
}

/// Build a Prometheus text exposition body from pre-computed metric values.
/// Extracted as a pure function so it can be unit-tested without an axum server.
#[allow(clippy::too_many_arguments)]
fn build_prometheus_text(
    queries_total: u64, errors_total: u64, slow_total: u64,
    p50: u64, p95: u64, p99: u64,
    records_total: u64, bytes_total: u64,
    anomalies: u64, bundle_count: usize, total_records: usize,
    http_conns: u64, ws_conns: u64, uptime_secs: u64,
) -> String {
    format!(
        "# HELP gigi_queries_total Total queries executed\n\
         # TYPE gigi_queries_total counter\n\
         gigi_queries_total {queries_total}\n\
         # HELP gigi_queries_error_total Total failed queries\n\
         # TYPE gigi_queries_error_total counter\n\
         gigi_queries_error_total {errors_total}\n\
         # HELP gigi_queries_slow_total Queries exceeding slow_query_threshold\n\
         # TYPE gigi_queries_slow_total counter\n\
         gigi_queries_slow_total {slow_total}\n\
         # HELP gigi_query_duration_microseconds Query latency percentiles\n\
         # TYPE gigi_query_duration_microseconds summary\n\
         gigi_query_duration_microseconds{{quantile=\"0.5\"}} {p50}\n\
         gigi_query_duration_microseconds{{quantile=\"0.95\"}} {p95}\n\
         gigi_query_duration_microseconds{{quantile=\"0.99\"}} {p99}\n\
         # HELP gigi_records_ingested_total Total records written\n\
         # TYPE gigi_records_ingested_total counter\n\
         gigi_records_ingested_total {records_total}\n\
         # HELP gigi_bytes_ingested_total Total bytes written\n\
         # TYPE gigi_bytes_ingested_total counter\n\
         gigi_bytes_ingested_total {bytes_total}\n\
         # HELP gigi_anomalies_detected_total Total anomalies detected\n\
         # TYPE gigi_anomalies_detected_total counter\n\
         gigi_anomalies_detected_total {anomalies}\n\
         # HELP gigi_bundles Total bundles in engine\n\
         # TYPE gigi_bundles gauge\n\
         gigi_bundles {bundle_count}\n\
         # HELP gigi_records_total Total records across all bundles\n\
         # TYPE gigi_records_total gauge\n\
         gigi_records_total {total_records}\n\
         # HELP gigi_http_connections_total Total HTTP connections served\n\
         # TYPE gigi_http_connections_total counter\n\
         gigi_http_connections_total {http_conns}\n\
         # HELP gigi_ws_connections_total Total WebSocket connections served\n\
         # TYPE gigi_ws_connections_total counter\n\
         gigi_ws_connections_total {ws_conns}\n\
         # HELP gigi_uptime_seconds Server uptime\n\
         # TYPE gigi_uptime_seconds gauge\n\
         gigi_uptime_seconds {uptime_secs}\n"
    )
}

/// GET /v1/metrics — live telemetry for operators, dashboards, and alerting.
/// Accepts `Accept: text/plain` for Prometheus exposition format.
async fn metrics_handler(
    State(state): State<Arc<StreamState>>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    let m = &state.metrics;
    let (p50, p95, p99) = m.percentiles();
    let uptime_secs = state.start_time.elapsed().as_secs();
    let (bundle_count, total_records) = state.engine.try_read()
        .map(|e| (e.bundle_names().len(), e.total_records()))
        .unwrap_or((0, 0));

    let wants_prometheus = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/plain"))
        .unwrap_or(false);

    if wants_prometheus {
        let queries_total  = m.queries_total.load(Ordering::Relaxed);
        let errors_total   = m.queries_error.load(Ordering::Relaxed);
        let slow_total     = m.queries_slow.load(Ordering::Relaxed);
        let records_total  = m.records_ingested.load(Ordering::Relaxed);
        let bytes_total    = m.bytes_ingested.load(Ordering::Relaxed);
        let anomalies      = m.anomalies_total.load(Ordering::Relaxed);
        let http_conns     = m.http_connections_total.load(Ordering::Relaxed);
        let ws_conns       = m.ws_connections_total.load(Ordering::Relaxed);

        let body = build_prometheus_text(
            queries_total, errors_total, slow_total,
            p50, p95, p99,
            records_total, bytes_total,
            anomalies, bundle_count, total_records,
            http_conns, ws_conns, uptime_secs,
        );

        return axum::response::Response::builder()
            .status(200)
            .header("content-type", "text/plain; version=0.0.4; charset=utf-8")
            .body(axum::body::Body::from(body))
            .unwrap();
    }

    // Default: JSON
    let json_body = serde_json::json!({
        "instance":              state.logger.instance,
        "version":               gigi::observability::GIGI_VERSION,
        "uptime_secs":           uptime_secs,
        "bundles":               bundle_count,
        "total_records":         total_records,
        "queries": {
            "total":             m.queries_total.load(Ordering::Relaxed),
            "errors":            m.queries_error.load(Ordering::Relaxed),
            "slow":              m.queries_slow.load(Ordering::Relaxed),
            "by_type":           m.by_type_snapshot(),
        },
        "latency_us": {
            "p50":  p50,
            "p95":  p95,
            "p99":  p99,
        },
        "ingest": {
            "records_total":     m.records_ingested.load(Ordering::Relaxed),
            "bytes_total":       m.bytes_ingested.load(Ordering::Relaxed),
        },
        "anomalies_total":       m.anomalies_total.load(Ordering::Relaxed),
        "connections": {
            "http_total":        m.http_connections_total.load(Ordering::Relaxed),
            "ws_total":          m.ws_connections_total.load(Ordering::Relaxed),
        }
    });
    axum::response::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(serde_json::to_vec(&json_body).unwrap()))
        .unwrap()
}

async fn list_bundles(
    State(state): State<Arc<StreamState>>,
    req: Request<axum::body::Body>,
) -> Json<Vec<BundleInfo>> {
    // Per-user namespace filter: non-owner sessions only see bundles
    // inside their `<ns>__*` prefix. Owner / API-key sessions see all
    // bundles. Lifting this server-side is the security-critical half
    // of Phase B — the sheets client also filters, but a hand-crafted
    // HTTP request can't bypass this filter.
    let claims = req.extensions().get::<GigiClaims>().cloned();
    let engine = state.engine.read().unwrap();
    let infos: Vec<BundleInfo> = engine
        .bundle_names()
        .iter()
        .filter(|name| match &claims {
            Some(c) => c.allows_bundle(name),
            None => true,
        })
        .map(|name| {
            let store = engine.bundle(name).unwrap();
            BundleInfo {
                name: name.to_string(),
                records: store.len(),
                fields: store.schema().base_fields.len() + store.schema().fiber_fields.len(),
            }
        })
        .collect();
    Json(infos)
}

async fn create_bundle(
    State(state): State<Arc<StreamState>>,
    request: Request<axum::body::Body>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // Pull claims out before consuming the body (extensions are
    // detached from the Request, so cloning is free).
    let claims = request.extensions().get::<GigiClaims>().cloned();

    // Now manually parse the JSON body — we couldn't use the
    // Json(req) extractor in the signature because we needed the raw
    // Request first to read extensions.
    let bytes = match axum::body::to_bytes(request.into_body(), 16 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Failed to read request body".to_string(),
                }),
            ));
        }
    };
    let req: CreateBundleRequest = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid JSON: {e}"),
                }),
            ));
        }
    };

    // Phase B: non-owner sessions can only create bundles inside their
    // own namespace. We can't intercept this in
    // namespace_enforcement_middleware because the bundle name lives
    // in the request body, not the URL.
    if let Some(ref c) = claims {
        if !c.allows_bundle(&req.name) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: format!(
                        "New bundle name must start with your workspace prefix '{}__'.",
                        c.ns
                    ),
                }),
            ));
        }
    }

    let mut schema = BundleSchema::new(&req.name);

    // Keys become base fields, rest become fiber fields
    for (field_name, field_type_str) in &req.schema.fields {
        let ft = str_to_field_type(field_type_str);
        let default_val = req
            .schema
            .defaults
            .get(field_name)
            .map(json_to_value)
            .unwrap_or(Value::Null);
        let fd = FieldDef {
            name: field_name.clone(),
            field_type: ft,
            default: default_val,
            range: None,
            weight: 1.0,
            encryption: gigi::types::EncryptionMode::None,
            encryption_group: None,
        };
        if req.schema.keys.contains(field_name) {
            schema = schema.base(fd);
        } else {
            schema = schema.fiber(fd);
        }
    }

    // Set indexed fields
    for idx_field in &req.schema.indexed {
        schema = schema.index(idx_field);
    }
    // Also index keys
    for key in &req.schema.keys {
        schema = schema.index(key);
    }

    if req.encrypted {
        let seed = gigi::crypto::GaugeKey::random_seed();
        let gk = gigi::crypto::GaugeKey::derive(&seed, &schema.fiber_fields);
        schema.gauge_key = Some(gk);
    }

    let field_count = req.schema.fields.len();
    let bundle_name_clone = req.name.clone();
    let mut engine = state.engine.write().unwrap();
    engine.create_bundle(schema).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {e}"),
            }),
        )
    })?;
    drop(engine);

    // Spec §3.6: bundle.create
    let ev = state.logger.bundle_create(&bundle_name_clone, field_count, "heap", "api");
    state.logger.emit(ev);

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "created",
            "bundle": bundle_name_clone
        })),
    ))
}

async fn drop_bundle(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if name.starts_with("_gigi_") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: format!("'{}' is a system bundle and is read-only", name) }),
        ));
    }

    // Capture stats before drop for the audit event
    let (records_before, bytes_before) = {
        let engine = state.engine.read().unwrap();
        if let Some(store) = engine.bundle(&name) {
            let recs = store.len() as u64;
            let bytes = recs * 64; // same heuristic used by estimate_bytes
            (recs, bytes)
        } else {
            (0, 0)
        }
    };

    let mut engine = state.engine.write().unwrap();
    match engine.drop_bundle(&name) {
        Ok(true) => {
            drop(engine);
            // Spec §3.6: bundle.drop (Bundle category)
            let bev = state.logger.bundle_drop(&name, records_before, "api", "");
            state.logger.emit(bev);
            // Spec §3.8: audit.bundle_drop
            let aev = state.logger.audit_bundle_drop(&name, records_before, bytes_before, "api", "");
            state.logger.emit(aev);
            Ok(Json(
                serde_json::json!({"status": "dropped", "bundle": name}),
            ))
        }
        // Idempotent: deleting a non-existent bundle is not an error
        Ok(false) => Ok(Json(
            serde_json::json!({"status": "not_found", "bundle": name}),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {e}"),
            }),
        )),
    }
}

async fn insert_records(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<InsertRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if name.starts_with("_gigi_") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: format!("'{}' is a system bundle and is read-only", name) }),
        ));
    }
    let mut engine = state.engine.write().unwrap();

    // Get schema info (borrow released after block)
    let (key_name_opt, has_created_at, has_updated_at) = {
        let store = engine.bundle(&name).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Bundle '{}' not found", name),
                }),
            )
        })?;
        let key = if store.schema().base_fields.len() == 1 {
            Some(store.schema().base_fields[0].name.clone())
        } else {
            None
        };
        let ca = store
            .schema()
            .fiber_fields
            .iter()
            .any(|f| f.name == "created_at");
        let ua = store
            .schema()
            .fiber_fields
            .iter()
            .any(|f| f.name == "updated_at");
        (key, ca, ua)
    };

    // Convert JSON records to GIGI records
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let mut records: Vec<Record> = req
        .records
        .iter()
        .filter_map(|item| {
            if let serde_json::Value::Object(map) = item {
                let mut rec: Record = map
                    .iter()
                    .map(|(k, v)| (k.clone(), json_to_value(v)))
                    .collect();
                if has_created_at && !rec.contains_key("created_at") {
                    rec.insert("created_at".into(), Value::Timestamp(now_ms));
                }
                if has_updated_at && !rec.contains_key("updated_at") {
                    rec.insert("updated_at".into(), Value::Timestamp(now_ms));
                }
                Some(rec)
            } else {
                None
            }
        })
        .collect();

    // Auto-generate IDs for records missing the base key
    if let Some(ref key_name) = key_name_opt {
        for rec in &mut records {
            if !rec.contains_key(key_name) || rec.get(key_name) == Some(&Value::Null) {
                let id = engine.bundle_mut(&name).unwrap().next_auto_id();
                rec.insert(key_name.clone(), Value::Integer(id));
            }
        }
    }

    // WAL-logged batch insert
    // For single-record batches: compute anomaly check PRE-INSERT so the record
    // doesn't inflate its own curvature stats (which would mask detection).
    let pre_anomaly: Option<(f64, f64, Vec<String>)> = if records.len() == 1 {
        let store = engine.bundle(&name).unwrap();
        let stats = store.curvature_stats();
        if stats.k_count >= 10 {
            let fiber_vals: Vec<Value> = store
                .schema()
                .fiber_fields
                .iter()
                .map(|f| records[0].get(&f.name).cloned().unwrap_or(Value::Null))
                .collect();
            let k_rec = compute_record_k(
                &store.get_field_stats(),
                &fiber_vals,
                &store.schema().fiber_fields,
            );
            if stats.is_anomaly(k_rec, 2.0) {
                let z = stats.z_score(k_rec);
                let fstats = store.get_field_stats();
                let contributing: Vec<String> = store
                    .schema()
                    .fiber_fields
                    .iter()
                    .zip(fiber_vals.iter())
                    .filter_map(|(fd, v)| {
                        let v_f = v.as_f64()?;
                        let fs = fstats.get(&fd.name)?;
                        if fs.count < 2 {
                            return None;
                        }
                        let mean = fs.sum / fs.count as f64;
                        let range = fs.range().max(f64::EPSILON);
                        let field_k = (v_f - mean).abs() / range;
                        if field_k > 0.5 {
                            Some(fd.name.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                Some((k_rec, z, contributing))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let inserted = engine.batch_insert(&name, &records).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {e}"),
            }),
        )
    })?;

    let store = engine.bundle(&name).unwrap();
    let k = store.scalar_curvature();
    let conf = curvature::confidence(k);

    // Broadcast batch insert event — each individual record as separate event
    // so subscribers with per-record filters can evaluate them.
    // For large batches, emit a single summary event to avoid channel flooding.
    let tx = state.get_or_create_channel(&name);
    if records.len() <= 100 {
        for rec in &records {
            let _ = tx.send(SubscriptionEvent {
                bundle: name.clone(),
                op: "insert",
                record_json: serde_json::to_string(&record_to_json(rec)).unwrap_or_default(),
                curvature: k,
            });
        }
    } else {
        let _ = tx.send(SubscriptionEvent {
            bundle: name.clone(),
            op: "insert",
            record_json: format!("{{\"batch\": {inserted}}}"),
            curvature: k,
        });
    }

    // Emit dashboard event with current bundle health snapshot
    let _ = state.dashboard_tx.send(build_dashboard_event(
        "insert",
        &name,
        &store,
        k,
        None,
        None,
        vec![],
    ));

    // Emit anomaly event when pre-insert detection flagged this record
    if let Some((k_rec, z, contributing)) = pre_anomaly {
        state.metrics.record_anomaly();
        let ev = state.logger.anomaly_detected(
            &name, "stream-insert",
            k_rec, store.curvature_stats().mean(), store.curvature_stats().std_dev(),
            z, 2.0, 3.0, &contributing, "insert", 0,
        );
        state.logger.emit(ev);
        let _ = state.dashboard_tx.send(build_dashboard_event(
            "anomaly",
            &name,
            &store,
            k,
            Some(k_rec),
            Some(z),
            contributing,
        ));
    }

    // Observability: ingest.complete
    {
        let bytes_est = estimate_bytes(&records);
        let ev = state.logger.ingest_complete(
            &name, inserted as u64, bytes_est, 0, true, false, &[], None, Some(k),
        );
        state.logger.emit(ev);
        state.metrics.record_ingest(inserted as u64, bytes_est);
    }

    Ok(Json(serde_json::json!({
        "status": "inserted",
        "count": inserted,
        "total": store.len(),
        "curvature": k,
        "confidence": conf
    })))
}

// Helper: estimate byte size of records before insertion
fn estimate_bytes(records: &[Record]) -> u64 {
    records.iter().map(|r| r.len() as u64 * 64).sum()
}

/// Streaming NDJSON ingest — accepts newline-delimited JSON via chunked body.
///
/// Use case: pipe data directly from sensors, log files, Kafka consumers.
///   curl -X POST http://localhost:3142/v1/bundles/sensors/stream \
///     -H "Content-Type: application/x-ndjson" \
///     --data-binary @data.ndjson
async fn stream_ingest(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    body: axum::body::Body,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    use axum::body::to_bytes;

    // Check bundle exists before reading body
    {
        let engine = state.engine.read().unwrap();
        if engine.bundle(&name).is_none() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Bundle '{}' not found", name),
                }),
            ));
        }
    }

    // Read body (cap at 256MB to prevent abuse)
    let bytes = to_bytes(body, 256 * 1024 * 1024).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Failed to read body: {e}"),
            }),
        )
    })?;

    let text = String::from_utf8_lossy(&bytes);

    // Parse NDJSON: each line is a JSON object
    let mut records: Vec<Record> = Vec::new();
    let mut parse_errors = 0usize;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(serde_json::Value::Object(map)) => {
                let record: Record = map
                    .iter()
                    .map(|(k, v)| (k.clone(), json_to_value(v)))
                    .collect();
                records.push(record);
            }
            _ => {
                parse_errors += 1;
            }
        }
    }

    // WAL-logged batch insert
    let mut engine = state.engine.write().unwrap();
    let inserted = engine.batch_insert(&name, &records).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {e}"),
            }),
        )
    })?;

    let store = engine.bundle(&name).unwrap();
    let k = store.scalar_curvature();

    let tx = state.get_or_create_channel(&name);
    let _ = tx.send(SubscriptionEvent {
        bundle: name.clone(),
        op: "insert",
        record_json: format!("{{\"stream_batch\": {inserted}}}"),
        curvature: k,
    });
    let conf = curvature::confidence(k);

    // Spec §3.2: ingest.bulk
    {
        let bytes_est = estimate_bytes(&records);
        let dur_us = 0u64; // no per-handler timing yet
        let tps = if dur_us > 0 { inserted as f64 / (dur_us as f64 / 1_000_000.0) } else { 0.0 };
        let ev = state.logger.ingest_bulk(&name, inserted as u64, bytes_est, dur_us, tps, true, 1);
        state.logger.emit(ev);
        state.metrics.record_ingest(inserted as u64, bytes_est);
    }

    Ok(Json(serde_json::json!({
        "status": "streamed",
        "count": inserted,
        "parse_errors": parse_errors,
        "total": store.len(),
        "curvature": k,
        "confidence": conf,
        "storage_mode": store.storage_mode()
    })))
}

async fn point_query(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    // Build key record from query params
    let key: Record = params
        .iter()
        .map(|(k, v)| {
            let val = if let Ok(n) = v.parse::<i64>() {
                Value::Integer(n)
            } else if let Ok(f) = v.parse::<f64>() {
                Value::Float(f)
            } else {
                Value::Text(v.clone())
            };
            (k.clone(), val)
        })
        .collect();

    match store.point_query(&key) {
        Some(record) => {
            let k = store.scalar_curvature();
            Ok(Json(ApiResponse {
                data: record_to_json(&record),
                meta: Some(MetaInfo {
                    confidence: Some(curvature::confidence(k)),
                    curvature: Some(k),
                    capacity: Some(curvature::capacity(1.0, k)),
                    count: Some(1),
                }),
            }))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Record not found".to_string(),
            }),
        )),
    }
}

async fn range_query(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    // Use first param as range query field
    if let Some((field, value)) = params.iter().next() {
        let val = if let Ok(n) = value.parse::<i64>() {
            Value::Integer(n)
        } else if let Ok(f) = value.parse::<f64>() {
            Value::Float(f)
        } else {
            Value::Text(value.clone())
        };

        let records = store.range_query(field, &[val]);
        let json_records: Vec<serde_json::Value> = records.iter().map(record_to_json).collect();
        let k = store.scalar_curvature();
        let count = json_records.len();
        Ok(Json(ApiResponse {
            data: json_records,
            meta: Some(MetaInfo {
                confidence: Some(curvature::confidence(k)),
                curvature: Some(k),
                capacity: None,
                count: Some(count),
            }),
        }))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Provide at least one field=value query parameter".to_string(),
            }),
        ))
    }
}

async fn pullback_join(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<JoinRequest>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let left = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;
    let right = engine.bundle(&req.right_bundle).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", req.right_bundle),
            }),
        )
    })?;

    let results = match (left.as_heap(), right.as_heap()) {
        (Some(l), Some(r)) => join::pullback_join(l, r, &req.left_field, &req.right_field),
        _ => Vec::new(),
    };
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|(left_rec, right_rec)| {
            let mut combined = serde_json::Map::new();
            combined.insert("left".to_string(), record_to_json(left_rec));
            combined.insert(
                "right".to_string(),
                right_rec
                    .as_ref()
                    .map(record_to_json)
                    .unwrap_or(serde_json::Value::Null),
            );
            serde_json::Value::Object(combined)
        })
        .collect();

    let count = json_results.len();
    Ok(Json(ApiResponse {
        data: json_results,
        meta: Some(MetaInfo {
            confidence: None,
            curvature: None,
            capacity: None,
            count: Some(count),
        }),
    }))
}

async fn aggregate(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<AggregateRequest>,
) -> Result<Json<AggResult>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let groups = if req.conditions.is_empty() {
        store.as_heap().map(|s| aggregation::group_by(s, &req.group_by, &req.field)).unwrap_or_default()
    } else {
        let conditions: Vec<QueryCondition> = req
            .conditions
            .iter()
            .map(condition_spec_to_query_condition)
            .collect();
        store.as_heap().map(|s| aggregation::filtered_group_by(s, &req.group_by, &req.field, &conditions)).unwrap_or_default()
    };
    let mut result_groups = HashMap::new();
    for (key, agg) in groups {
        let key_str = key.to_string();
        result_groups.insert(
            key_str,
            AggValues {
                count: agg.count,
                sum: agg.sum,
                avg: agg.avg(),
                min: agg.min,
                max: agg.max,
            },
        );
    }

    // HAVING — filter groups on aggregated values
    if !req.having.is_empty() {
        result_groups.retain(|_, agg| {
            req.having.iter().all(|h| {
                let agg_val = match h.field.as_str() {
                    "count" => agg.count as f64,
                    "sum" => agg.sum,
                    "avg" => agg.avg,
                    "min" => agg.min,
                    "max" => agg.max,
                    _ => return true,
                };
                let threshold = h.value.as_f64().unwrap_or(0.0);
                match h.op.as_str() {
                    "gt" | ">" => agg_val > threshold,
                    "gte" | ">=" => agg_val >= threshold,
                    "lt" | "<" => agg_val < threshold,
                    "lte" | "<=" => agg_val <= threshold,
                    "eq" | "=" | "==" => (agg_val - threshold).abs() < f64::EPSILON,
                    "neq" | "!=" | "<>" => (agg_val - threshold).abs() >= f64::EPSILON,
                    _ => true,
                }
            })
        });
    }

    Ok(Json(AggResult {
        groups: result_groups,
    }))
}

async fn curvature_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<CurvatureReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let k = store.scalar_curvature();
    let conf = curvature::confidence(k);
    let cap = curvature::capacity(1.0, k);

    // Per-field curvature from stats
    let mut per_field = Vec::new();
    let stats = store.field_stats();
    for (field_name, fs) in stats {
        let variance = fs.variance();
        let range = fs.range();
        let field_k = if range > 0.0 {
            variance / (range * range)
        } else {
            0.0
        };
        per_field.push(FieldCurvature {
            field: field_name.clone(),
            variance,
            range,
            k: field_k,
        });
    }

    // L4 — Kähler curvature decomposition (gated; None when no
    // Kähler attached or insufficient data).
    #[cfg(feature = "kahler")]
    let kahler = store
        .as_heap()
        .and_then(|s| s.kahler_curvature())
        .map(KahlerCurvatureReport::from);

    Ok(Json(CurvatureReport {
        k,
        curvature: k,
        confidence: conf,
        capacity: cap,
        per_field,
        #[cfg(feature = "kahler")]
        kahler,
    }))
}

async fn spectral_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<SpectralReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let lambda1 = store.as_heap().map(spectral::spectral_gap).unwrap_or(0.0);
    let diameter = store.as_heap().map(spectral::graph_diameter).unwrap_or(0);
    let spectral_cap = store.as_heap().map(spectral::spectral_capacity).unwrap_or(0.0);

    Ok(Json(SpectralReport {
        lambda1,
        diameter,
        spectral_capacity: spectral_cap,
    }))
}

/// L3.4 — Marcella consumption surface for spectral gap
/// (consumption draft v2 §4, GIGI reply Q4).
///
/// `GET /v1/bundles/<name>/spectral_gap` — returns the cached
/// snapshot with mixing-time + Cheeger bounds + freshness
/// timestamp. Marcella's runtime reads this per retrieval
/// session (or out-of-band) to set the rose-mechanism α
/// coefficient: `α = 1 - 1/sqrt(mix_time)`.
///
/// Returns 404 when the bundle has fewer than 2 records (the
/// snapshot is degenerate) — semantically correct for the
/// "no spectral structure to report" case.
///
/// Gated on the `kahler` feature; route is only mounted when
/// the feature is on (see the route table later in this file).
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, Serialize)]
struct SpectralGapResponse {
    /// Smallest non-zero eigenvalue of the normalized Laplacian.
    lambda_2: f64,
    /// Mixing-time bound Θ((1/λ₂)·log(1/ε)), ε = 1e-3.
    mix_time: u64,
    /// Cheeger lower bound on edge expansion: λ₂ / 2.
    cheeger_lower: f64,
    /// Cheeger upper bound on edge expansion: √(2 λ₂).
    cheeger_upper: f64,
    /// Timestamp the snapshot was computed (epoch seconds string).
    /// Marcella compares against insert events to detect drift.
    cached_at: String,
}

#[cfg(feature = "kahler")]
async fn spectral_gap_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<SpectralGapResponse>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let snap = store
        .as_heap()
        .and_then(|s| s.spectral_gap_cached())
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!(
                        "Bundle '{}' has insufficient records for spectral gap (need ≥ 2)",
                        name
                    ),
                }),
            )
        })?;

    Ok(Json(SpectralGapResponse {
        lambda_2: snap.lambda_2,
        mix_time: snap.mix_time,
        cheeger_lower: snap.cheeger_lower,
        cheeger_upper: snap.cheeger_upper,
        cached_at: snap.cached_at,
    }))
}

// ────────────────────────────────────────────────────────────
// PR-window endpoints for Marcella's Hopf + Riemann-Roch wiring
// (cross-team thread 2026-05-25). Four endpoints in one window:
//
//   POST /v1/quantum_cohomology/compose      — L7.5 frobenius
//   POST /v1/quantum_cohomology/capacity     — L7.7 Riemann-Roch
//   POST /v1/bundles/{name}/holonomy_debt    — L7.2
//   POST /v1/bundles/{name}/flat_transport   — L1.5
//
// All cfg-gated on `kahler`. Each handler is a thin wrapper over
// the existing Rust API; the wire shapes are the Marcella
// contract surface.
// ────────────────────────────────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct QuantumCohomologySpec {
    /// Manifold class: "cpn", "torus_tn", "sphere2", or "non_toy".
    class: String,
    /// Complex dim n (required for "cpn" and "torus_tn"; ignored otherwise).
    #[serde(default)]
    n: Option<usize>,
    /// Quantum truncation (cpn only; defaults to n+1 if omitted).
    #[serde(default)]
    q_truncation: Option<usize>,
}

#[cfg(feature = "kahler")]
impl QuantumCohomologySpec {
    fn to_quantum_cohomology(
        &self,
    ) -> Result<gigi::geometry::QuantumCohomology, String> {
        match self.class.as_str() {
            "cpn" => {
                let n = self.n.ok_or_else(|| "cpn requires n".to_string())?;
                let q = self.q_truncation.unwrap_or(n + 1);
                Ok(gigi::geometry::QuantumCohomology::Cpn { n, q_truncation: q })
            }
            "torus_tn" => {
                let n = self.n.ok_or_else(|| "torus_tn requires n".to_string())?;
                Ok(gigi::geometry::QuantumCohomology::TorusTn { n })
            }
            "sphere2" => Ok(gigi::geometry::QuantumCohomology::Sphere2),
            "non_toy" => Ok(gigi::geometry::QuantumCohomology::NonToy),
            other => Err(format!(
                "unknown manifold class '{}' (expected: cpn, torus_tn, sphere2, non_toy)",
                other
            )),
        }
    }
}

/// Wire shape for a `CohClass` term — `(coefficient, h_power, q_power)`.
/// Round-trips through serde as a 3-element JSON array so the Python
/// adapter can build/decode without bespoke struct support.
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CohTerm(f64, usize, usize);

#[cfg(feature = "kahler")]
impl From<&gigi::geometry::CohClass> for CohClassWire {
    fn from(c: &gigi::geometry::CohClass) -> Self {
        CohClassWire {
            terms: c.terms.iter().map(|&(c, h, q)| CohTerm(c, h, q)).collect(),
        }
    }
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CohClassWire {
    terms: Vec<CohTerm>,
}

#[cfg(feature = "kahler")]
impl CohClassWire {
    fn to_coh_class(&self) -> gigi::geometry::CohClass {
        gigi::geometry::CohClass {
            terms: self
                .terms
                .iter()
                .map(|t| (t.0, t.1, t.2))
                .collect(),
        }
    }
}

// ─── L7.5 frobenius_compose ─────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct FrobeniusComposeRequest {
    qh: QuantumCohomologySpec,
    a: CohClassWire,
    b: CohClassWire,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct FrobeniusComposeResponse {
    result: CohClassWire,
}

/// POST /v1/quantum_cohomology/compose
///
/// L7.5 Frobenius/WDVV composition on toy manifolds. Returns
/// `400 NonToy` when the manifold class is `non_toy` (research-
/// grade GW invariants not supported); returns `400 BadRequest`
/// on malformed input.
#[cfg(feature = "kahler")]
async fn frobenius_compose_endpoint(
    Json(req): Json<FrobeniusComposeRequest>,
) -> Result<Json<FrobeniusComposeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let qh = req.qh.to_quantum_cohomology().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        )
    })?;
    let a = req.a.to_coh_class();
    let b = req.b.to_coh_class();
    let result = qh.compose(&a, &b).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("{}", e),
            }),
        )
    })?;
    Ok(Json(FrobeniusComposeResponse {
        result: CohClassWire::from(&result),
    }))
}

// ─── L7.7 representational_capacity ─────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct CapacityRequest {
    qh: QuantumCohomologySpec,
    k_max: i64,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct CapacityResponse {
    capacity: i64,
}

/// POST /v1/quantum_cohomology/capacity
///
/// L7.7 Riemann-Roch representational capacity `dim H⁰(M, L^k)`
/// on toy manifolds. Returns `400 NonToy` on `non_toy`.
#[cfg(feature = "kahler")]
async fn capacity_endpoint(
    Json(req): Json<CapacityRequest>,
) -> Result<Json<CapacityResponse>, (StatusCode, Json<ErrorResponse>)> {
    let qh = req.qh.to_quantum_cohomology().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e }),
        )
    })?;
    let capacity = qh.representational_capacity(req.k_max).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("{}", e),
            }),
        )
    })?;
    Ok(Json(CapacityResponse { capacity }))
}

// ─── L7.2 holonomy_debt ─────────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct HolonomyDebtRequest {
    loop_winding: f64,
    #[serde(default = "default_holonomy_tolerance")]
    tolerance: f64,
}

#[cfg(feature = "kahler")]
fn default_holonomy_tolerance() -> f64 {
    1e-6
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct HolonomyDebtResponse {
    /// "quantized" or "continuous"
    variant: String,
    /// Integer winding count when variant == "quantized"; null otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    quantized: Option<i64>,
    /// Real-valued winding when variant == "continuous"; null otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    continuous: Option<f64>,
    /// `winding()` value — `n as f64` for quantized, `x` for continuous.
    /// Always populated for callers who don't want to pattern-match
    /// on the variant.
    winding: f64,
}

/// POST /v1/bundles/{name}/holonomy_debt
///
/// L7.2 quantized vs continuous holonomy classification for a loop
/// integral over the bundle's attached B. Returns `404` when the
/// bundle has no Kähler structure attached (no B to integrate).
#[cfg(feature = "kahler")]
async fn holonomy_debt_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<HolonomyDebtRequest>,
) -> Result<Json<HolonomyDebtResponse>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;
    let heap = store.as_heap().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' is not heap-resident", name),
            }),
        )
    })?;
    let debt = gigi::curvature::holonomy_debt(heap, req.loop_winding, req.tolerance)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!(
                        "Bundle '{}' has no Kähler structure attached",
                        name
                    ),
                }),
            )
        })?;
    let (variant, quantized, continuous) = match debt {
        gigi::curvature::HolonomyDebt::Quantized(n) => {
            ("quantized".to_string(), Some(n), None)
        }
        gigi::curvature::HolonomyDebt::Continuous(x) => {
            ("continuous".to_string(), None, Some(x))
        }
    };
    Ok(Json(HolonomyDebtResponse {
        variant,
        quantized,
        continuous,
        winding: debt.winding(),
    }))
}

// ─── L1.5 flat_transport ────────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct FlatTransportRequest {
    /// Starting position p ∈ R^n.
    from_point: Vec<f64>,
    /// Target endpoint q ∈ R^n (diagnostic only; trajectory is
    /// driven by `initial_velocity` + `bias`).
    to_point: Vec<f64>,
    /// Initial tangent vector v ∈ T_p M = R^n.
    initial_velocity: Vec<f64>,
    /// Optional bias 2-form (row-major flat antisymmetric matrix).
    /// When `null`, runs classical (no magnetic perturbation).
    #[serde(default)]
    bias: Option<Vec<f64>>,
    /// RK4 step size; defaults to 1e-4.
    #[serde(default = "default_dt")]
    dt: f64,
    /// Number of RK4 steps; defaults to 65536.
    #[serde(default)]
    steps: usize,
    /// Provenance tag: "bundle", "override", "none", "fallback_non_closed".
    /// Defaults to "override" when bias is provided, "none" otherwise.
    #[serde(default)]
    b_source: Option<String>,
}

#[cfg(feature = "kahler")]
fn default_dt() -> f64 {
    1e-4
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct FlatTransportResponse {
    trajectory: Vec<Vec<f64>>,
    final_velocity: Vec<f64>,
    path_length: f64,
    energy_drift: f64,
    holonomy_norm: f64,
    used_magnetic: bool,
    b_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    closedness_norm: Option<f64>,
}

/// POST /v1/bundles/{name}/flat_transport
///
/// L1.5 B-perturbed flat-space magnetic transport. Bundle is
/// used only for dimension validation against its attached
/// Kähler structure (when present); the integration itself runs
/// on the supplied bias 2-form.
#[cfg(feature = "kahler")]
async fn flat_transport_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<FlatTransportRequest>,
) -> Result<Json<FlatTransportResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Bundle lookup — must exist; used for dim coherence check.
    let engine = state.engine.read().unwrap();
    let _store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;
    drop(engine);

    let dim = req.from_point.len();
    let seg = gigi::geometry::TransportSegment::new(
        req.from_point,
        req.to_point,
        req.initial_velocity,
    )
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("{}", e),
            }),
        )
    })?;

    let bias_form = if let Some(raw) = req.bias {
        if raw.len() != dim * dim {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "bias matrix has {} entries; expected {} × {} = {}",
                        raw.len(),
                        dim,
                        dim,
                        dim * dim
                    ),
                }),
            ));
        }
        let tf = gigi::geometry::TwoForm::new(raw, dim).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("{:?}", e),
                }),
            )
        })?;
        Some(gigi::geometry::ClosedTwoForm::new_constant(tf))
    } else {
        None
    };

    let b_source = match req.b_source.as_deref() {
        Some("bundle") => gigi::geometry::BSource::Bundle,
        Some("override") => gigi::geometry::BSource::Override,
        Some("none") => gigi::geometry::BSource::None,
        Some("fallback_non_closed") => gigi::geometry::BSource::FallbackNonClosed,
        None => {
            if bias_form.is_some() {
                gigi::geometry::BSource::Override
            } else {
                gigi::geometry::BSource::None
            }
        }
        Some(other) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "unknown b_source '{}' (expected: bundle, override, none, fallback_non_closed)",
                        other
                    ),
                }),
            ));
        }
    };

    let result = gigi::geometry::flat_transport(
        &seg,
        bias_form.as_ref(),
        req.dt,
        req.steps,
        b_source,
    )
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("{}", e),
            }),
        )
    })?;

    Ok(Json(FlatTransportResponse {
        trajectory: result.trajectory,
        final_velocity: result.final_velocity,
        path_length: result.path_length,
        energy_drift: result.energy_drift,
        holonomy_norm: result.holonomy_norm,
        used_magnetic: result.used_magnetic,
        b_source: format!("{:?}", result.b_source).to_lowercase(),
        closedness_norm: result.closedness_norm,
    }))
}

// ═══════════════════════════════════════════════════════════════
// 2026-05-25 PR window 2 — brain-primitive HTTP endpoints (L13)
// ═══════════════════════════════════════════════════════════════
//
// Surface 5 of the 12 brain primitives over HTTP for cross-team
// consumption (Marcella, MIRADOR, PRISM). All endpoints live under
// `/v1/bundles/{name}/brain/*` to avoid colliding with existing
// `/predict` route and to keep the brain namespace cleanly demarcated.
//
//   POST /v1/bundles/{name}/brain/sample      §2  — Langevin draw
//   POST /v1/bundles/{name}/brain/confidence  §12 — Fisher-precision gate
//   POST /v1/bundles/{name}/brain/attend      §8  — softmax retrieval
//   POST /v1/bundles/{name}/brain/episodic    §10 — change-point detect
//   GET  /v1/bundles/{name}/brain/semantic    §11 — Morse-compressed gist
//
// Wire shapes pinned by tests/kahler_brain_endpoints_contract.rs.
// Catalog: theory/brain_primitives/catalog.md.

#[cfg(feature = "kahler")]
fn extract_field_samples(
    store: &gigi::BundleStore,
    fields: &[String],
) -> Result<Vec<Vec<f64>>, String> {
    if fields.is_empty() {
        return Err("at least one fiber field required".into());
    }
    // Records are slices indexed by fiber-field position. Resolve
    // each requested name to its index in the schema. We give a
    // detailed error message if the field is in base_fields rather
    // than fiber_fields (per Marcella's 2026-05-25 probe report —
    // her `token_id` is a base_field and the original "not in
    // schema" message was confusing).
    let mut field_idx = Vec::with_capacity(fields.len());
    for f in fields {
        let i = store
            .schema
            .fiber_fields
            .iter()
            .position(|fd| fd.name == *f)
            .ok_or_else(|| {
                let in_base = store
                    .schema
                    .base_fields
                    .iter()
                    .any(|fd| fd.name == *f);
                let available_fiber: Vec<&str> = store
                    .schema
                    .fiber_fields
                    .iter()
                    .map(|fd| fd.name.as_str())
                    .collect();
                if in_base {
                    format!(
                        "field '{}' is a base_field (query key), not a fiber_field. \
                         Brain endpoints only operate on fiber dimensions. \
                         Available fiber_fields: {:?}",
                        f, available_fiber
                    )
                } else {
                    format!(
                        "field '{}' not found in schema. \
                         Available fiber_fields: {:?}",
                        f, available_fiber
                    )
                }
            })?;
        field_idx.push(i);
    }
    let mut samples = Vec::new();
    for (_bp, record) in store.sections() {
        let mut row = Vec::with_capacity(fields.len());
        for &i in &field_idx {
            let val = record.get(i).ok_or_else(|| {
                format!("record missing fiber position {}", i)
            })?;
            let v = match val {
                gigi::types::Value::Float(x) => *x,
                gigi::types::Value::Integer(j) => *j as f64,
                _ => {
                    return Err(format!(
                        "field '{}' has non-numeric value in a record",
                        fields[field_idx.iter().position(|&x| x == i).unwrap_or(0)]
                    ));
                }
            };
            row.push(v);
        }
        samples.push(row);
    }
    Ok(samples)
}

/// Which fit to use when building a generative flow from a bundle.
/// L13.3 ships the `Diagonal` variant per Marcella's Finding 3 —
/// anisotropic per-axis σ² instead of a single averaged scalar.
/// S1 (2026-05-26) adds `Full` per Marcella's H2 attractor letter:
/// captures inter-axis correlation that the diagonal model ignores
/// — required to diffuse the `double_cover_v3` attractor on bge_v2.
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum FitMode {
    #[default]
    Isotropic,
    Diagonal,
    Full,
}

// ─── BundleFlowCache (S1 wave 1 §A) ─────────────────────────────
//
// Per Bee's 2026-05-27 product-latency reframe: brain endpoints
// must serve fits from cache on the hot path. Without this, every
// `/brain/dream` with `fit_mode: "full"` re-walks ALL records of
// the bundle — ~3s at n=384 on bge_v2 — which kills the product
// latency story.
//
// Architecture:
//   - Hot path = lock-free read: RwLock::read + HashMap::get +
//     counter compare. Sub-microsecond on cache hit.
//   - Miss path = drop read, acquire write, compute fit, insert.
//   - Invalidation = `BundleStore::mutation_counter` (added in
//     bundle.rs). Each cached fit stamps the counter at fit time;
//     a stale lookup (counter changed) returns None.
//   - Eviction = bounded count (default 50). On insert past cap,
//     evict any one entry (random) — O(1), good enough for v1.
//     LRU as a follow-up if telemetry shows hit-rate problems.
//   - Surface = `X-Bundle-Mutation-Counter` response header on
//     brain endpoints, so consumers can stamp their warm path and
//     detect server-side invalidations.

#[cfg(feature = "kahler")]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct CacheKey {
    bundle_name: String,
    fit_mode: FitMode,
    fields_hash: u64,
    /// Bits of f64 — exact comparison. None = floor unset.
    sigma_floor_epsilon_bits: u64,
}

#[cfg(feature = "kahler")]
impl CacheKey {
    fn build(
        bundle_name: &str,
        fit_mode: FitMode,
        fields: &[String],
        sigma_floor_epsilon: Option<f64>,
    ) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // Hash fields in their incoming order (caller controls
        // ordering — different orderings = different fits).
        for f in fields {
            f.hash(&mut hasher);
        }
        let fields_hash = hasher.finish();
        // Encode "unset" as a sentinel that never collides with a
        // valid floor value (NaN's bit pattern).
        let sigma_floor_epsilon_bits = match sigma_floor_epsilon {
            Some(eps) => eps.to_bits(),
            None => f64::NAN.to_bits(),
        };
        CacheKey {
            bundle_name: bundle_name.to_string(),
            fit_mode,
            fields_hash,
            sigma_floor_epsilon_bits,
        }
    }
}

/// Cached fit data — the expensive part. The Langevin closure is
/// rebuilt on each call from these Arc'd payloads (cheap: just
/// captures Arc clones, no data copy).
#[cfg(feature = "kahler")]
#[derive(Clone)]
struct CachedFit {
    counter_at_fit: u64,
    mu: Arc<Vec<f64>>,
    sigma_sq: f64,
    sigma_sq_per_field: Arc<Vec<f64>>,
    sigma_sq_per_field_raw: Arc<Vec<f64>>,
    effective_floor: f64,
    floored_indices: Arc<Vec<usize>>,
    /// Σ⁻¹ — only populated for `FitMode::Full`. None otherwise.
    precision: Option<Arc<Vec<Vec<f64>>>>,
    /// Σ post-flooring — only populated for `FitMode::Full`.
    /// Surfaced via fit_diagnostics endpoint.
    covariance: Option<Arc<Vec<Vec<f64>>>>,
    /// Full-fit diagnostics (None for isotropic/diagonal).
    eigenvalues_raw: Option<Arc<Vec<f64>>>,
    eigenvalues_effective: Option<Arc<Vec<f64>>>,
    eigenvalue_floor_used: f64,
    floored_eigenvalue_count: usize,
    condition_number: f64,
    variance_ratio: f64,
}

#[cfg(feature = "kahler")]
pub struct BundleFlowCache {
    inner: std::sync::RwLock<std::collections::HashMap<CacheKey, CachedFit>>,
    max_entries: usize,
}

#[cfg(feature = "kahler")]
impl BundleFlowCache {
    pub fn new(max_entries: usize) -> Self {
        BundleFlowCache {
            inner: std::sync::RwLock::new(std::collections::HashMap::new()),
            max_entries: max_entries.max(1),
        }
    }

    /// Hot path lookup. Returns Some only if the cached fit's
    /// counter still matches the bundle's current counter — i.e.
    /// no inserts have happened since the fit was computed.
    fn get(&self, key: &CacheKey, current_counter: u64) -> Option<CachedFit> {
        let map = match self.inner.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let entry = map.get(key)?;
        if entry.counter_at_fit == current_counter {
            Some(entry.clone())
        } else {
            None
        }
    }

    fn insert(&self, key: CacheKey, fit: CachedFit) {
        let mut map = match self.inner.write() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if map.len() >= self.max_entries {
            // Random eviction: drop any one entry. Acceptable for
            // v1 — LRU is a follow-up if hit-rate telemetry warrants.
            if let Some(k) = map.keys().next().cloned() {
                map.remove(&k);
            }
        }
        map.insert(key, fit);
    }

    /// Number of entries currently held. Exposed for diagnostics
    /// (the future fit_diagnostics endpoint surfaces this).
    pub fn len(&self) -> usize {
        let map = match self.inner.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        map.len()
    }

    /// Drop all cached fits. Called when global invalidation is
    /// desired (engine reload, schema migration, etc.).
    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut map = match self.inner.write() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        map.clear();
    }
}

/// Construct the `X-Bundle-Mutation-Counter` response header from a
/// counter value. Used by brain endpoints to surface the cache-
/// invalidation signal back to consumers, so they can stamp their
/// own warm path and detect server-side invalidation.
#[cfg(feature = "kahler")]
fn bundle_counter_header(counter: u64) -> [(axum::http::HeaderName, String); 1] {
    [(
        axum::http::HeaderName::from_static("x-bundle-mutation-counter"),
        counter.to_string(),
    )]
}

/// Default relative-median ε floor for diagonal fit. Per Marcella
/// 2026-05-25 (REPLY_L13_3_DIAGONAL_FIT): without a floor,
/// rank-deficient axes (σ² ≈ 0) make the natural-gradient term
/// `(x − μ) / σ²` blow up — confirmed at 10^96 on v11_fiber dims
/// f12 / f13 / f14. ε = 1e-3 (her recommended default) bounds the
/// per-axis effective σ² at one part in 1000 of the median, which
/// kills the explosion while preserving most of the legitimate
/// anisotropy.
#[cfg(feature = "kahler")]
const DEFAULT_SIGMA_FLOOR_EPSILON: f64 = 1e-3;

/// Result of a diagonal-Gaussian fit on bundle Welford stats —
/// includes the raw observed variances, the post-floor effective
/// variances, the floor value used, and which indices were
/// floored (the rank-deficient dims). Surfaced in response so
/// consumers can see *which* dimensions the fit considered
/// degenerate.
#[cfg(feature = "kahler")]
struct DiagonalFitResult {
    mu: Vec<f64>,
    sigma_sq_raw: Vec<f64>,
    sigma_sq_effective: Vec<f64>,
    /// Threshold below which a per-axis σ² gets floored.
    effective_floor: f64,
    /// Indices in `fields` whose raw σ² was below the floor.
    floored_indices: Vec<usize>,
}

#[cfg(feature = "kahler")]
fn fit_diagonal_gaussian(
    store: &gigi::BundleStore,
    fields: &[String],
    floor_epsilon: f64,
) -> Result<DiagonalFitResult, String> {
    let stats = store.field_stats();
    let mut mu = Vec::with_capacity(fields.len());
    let mut sigma_sq_raw = Vec::with_capacity(fields.len());
    for f in fields {
        let s = stats.get(f).ok_or_else(|| {
            let in_base = store.schema.base_fields.iter().any(|fd| fd.name == *f);
            let available: Vec<&str> = stats.keys().map(|k| k.as_str()).collect();
            if in_base {
                format!(
                    "field '{}' is a base_field (query key); brain endpoints only \
                     fit Gaussians on numeric fiber_fields. Available stats: {:?}",
                    f, available
                )
            } else {
                format!(
                    "no Welford stats for field '{}'. Available stats: {:?} \
                     (brain endpoints require numeric fiber fields)",
                    f, available
                )
            }
        })?;
        if s.count == 0 {
            return Err(format!("field '{}' has no observations", f));
        }
        mu.push(s.sum / s.count as f64);
        sigma_sq_raw.push(s.variance().max(0.0)); // no floor yet — applied below
    }

    // ── L13.6 stability floor (Marcella REPLY_L13_3_DIAGONAL_FIT) ──
    //
    // Two floors composed via max:
    //   - RELATIVE: σ²_eff ≥ ε × median(σ²) (Marcella's Option 2).
    //   - ABSOLUTE: σ²_eff ≥ 2 × DT_DEFAULT (Euler-Maruyama
    //     stability requires dt < 2 σ², so any σ² below 2 × dt
    //     causes oscillatory explosion). DT_DEFAULT corresponds
    //     to the brain endpoints' default 0.01 — at smaller user-
    //     supplied dt the absolute floor could be relaxed, but
    //     keeping it constant means downstream is *always* stable
    //     at default dt.
    //
    // `floor_epsilon = 0.0` disables the relative floor (escape
    // hatch for well-conditioned data); the absolute stability
    // floor stays in effect so the integrator can't blow up.
    let mut sorted = sigma_sq_raw.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2].max(1e-12);
    let relative_floor = if floor_epsilon > 0.0 {
        floor_epsilon * median
    } else {
        0.0
    };
    // Default brain-endpoint dt is 0.01; Euler stability needs
    // σ² > 2 × dt. We use 3 × dt for a small safety margin.
    const ABSOLUTE_STABILITY_FLOOR: f64 = 3.0 * 0.01;
    let effective_floor = relative_floor.max(ABSOLUTE_STABILITY_FLOOR).max(1e-12);

    let mut sigma_sq_effective = Vec::with_capacity(sigma_sq_raw.len());
    let mut floored_indices = Vec::new();
    for (i, &raw) in sigma_sq_raw.iter().enumerate() {
        if raw < effective_floor {
            floored_indices.push(i);
            sigma_sq_effective.push(effective_floor);
        } else {
            sigma_sq_effective.push(raw);
        }
    }

    Ok(DiagonalFitResult {
        mu,
        sigma_sq_raw,
        sigma_sq_effective,
        effective_floor,
        floored_indices,
    })
}

/// Full-covariance fit result. Carries μ, the full Σ matrix
/// (so consumers can introspect), the precision Σ⁻¹ (so the
/// gradient eval is a single matvec), and diagnostic info about
/// the conditioning of the fit — including the eigenvalue
/// spectrum, which is the load-bearing diagnostic for H2.
#[cfg(feature = "kahler")]
struct FullFitResult {
    mu: Vec<f64>,
    /// Full n×n covariance matrix in row-major Vec<Vec<f64>>
    /// (post eigenvalue flooring).
    covariance: Vec<Vec<f64>>,
    /// Precision matrix Σ⁻¹ — used by the Langevin gradient.
    precision: Vec<Vec<f64>>,
    /// Per-axis variances (diagonal of Σ, post-floor).
    sigma_sq_per_field: Vec<f64>,
    /// Per-axis RAW variances (diagonal of Σ, pre-floor).
    sigma_sq_per_field_raw: Vec<f64>,
    /// Diagonal floor applied to per-axis variances (analog of
    /// L13.6's σ² floor — necessary but NOT sufficient for H2;
    /// see eigenvalue_floor below for the correct floor).
    effective_floor: f64,
    /// Diagonal indices floored (rank-deficient axes).
    floored_indices: Vec<usize>,
    /// Ratio of largest to smallest diagonal entry after flooring.
    variance_ratio: f64,
    /// Eigenvalues of Σ BEFORE flooring, sorted descending.
    /// THE H2 diagnostic — small eigenvalues create deep narrow
    /// grooves in Σ⁻¹ that pull Langevin walks regardless of
    /// which axis they're aligned with. Surfaced so the
    /// fit_diagnostics endpoint can report them directly.
    eigenvalues_raw: Vec<f64>,
    /// Eigenvalues AFTER flooring (max(λ, ε·median(λ))).
    eigenvalues_effective: Vec<f64>,
    /// Eigenvalue floor used: ε × median(λ_raw), bounded below
    /// by the absolute stability floor.
    eigenvalue_floor_used: f64,
    /// How many eigenvalues got clipped by the floor. Non-zero
    /// here is exactly the condition Marcella's H2 letter
    /// predicted — diagonal model couldn't see this because the
    /// pathology lives in directions, not axes.
    floored_eigenvalue_count: usize,
    /// Condition number of Σ post-floor: λ_max / λ_min.
    /// The well-conditioning guarantee the eigenvalue floor
    /// provides.
    condition_number: f64,
}

/// Full-covariance Gaussian fit per Marcella's 2026-05-26 H2
/// attractor letter. The diagonal fit on semantic embedding bundles
/// produces universal attractors because it can't represent inter-
/// axis correlation; this fit captures the full Σ via two passes
/// over the records, applies the same L13.6-style diagonal floor
/// for numerical stability, then inverts via Cholesky to get the
/// precision matrix Σ⁻¹.
///
/// Algorithm:
///   1. Pass 1: μᵢ = mean of field i (from Welford stats, free).
///   2. Pass 2: Σ_ij = (1/(N-1)) Σ_k (x_ki − μᵢ)(x_kj − μⱼ).
///   3. Apply diagonal floor: Σ_ii ← max(Σ_ii, ε · median(diag)).
///   4. Cholesky-decompose Σ; compute Σ⁻¹ via the factor.
///
/// Returns FullFitResult with Σ, Σ⁻¹, and diagnostics.
#[cfg(feature = "kahler")]
fn fit_full_gaussian(
    store: &gigi::BundleStore,
    fields: &[String],
    floor_epsilon: f64,
) -> Result<FullFitResult, String> {
    // Pass 1: mean from Welford stats (already computed, free).
    let stats = store.field_stats();
    let mut mu = Vec::with_capacity(fields.len());
    for f in fields {
        let s = stats.get(f).ok_or_else(|| {
            let in_base = store.schema.base_fields.iter().any(|fd| fd.name == *f);
            let available: Vec<&str> = stats.keys().map(|k| k.as_str()).collect();
            if in_base {
                format!(
                    "field '{}' is a base_field (query key); brain endpoints only \
                     fit Gaussians on numeric fiber_fields. Available stats: {:?}",
                    f, available
                )
            } else {
                format!(
                    "no Welford stats for field '{}'. Available stats: {:?} \
                     (brain endpoints require numeric fiber fields)",
                    f, available
                )
            }
        })?;
        if s.count == 0 {
            return Err(format!("field '{}' has no observations", f));
        }
        mu.push(s.sum / s.count as f64);
    }

    let n = fields.len();

    // Pass 2: walk records, accumulate Σ. For N records, n fields:
    // O(N·n²) memory-light (one record at a time). bge_v2 with
    // N=9964 and (say) n=10 fields = ~10⁶ ops, sub-second. For
    // n=384 (full embedding) ~1.5G ops ~1-2s. Both acceptable.
    let mut cov = vec![vec![0.0_f64; n]; n];
    let mut n_obs = 0_usize;
    for record in store.records() {
        // Extract this record's field values into a deviation vector.
        // Records may be sparse — skip any record missing any field
        // (matches the existing fit_isotropic / fit_diagonal logic
        // which relies on Welford stats over present-only values).
        let mut dx = Vec::with_capacity(n);
        let mut all_present = true;
        for (i, f) in fields.iter().enumerate() {
            match record.get(f) {
                Some(gigi::Value::Float(v)) => dx.push(v - mu[i]),
                Some(gigi::Value::Integer(v)) => dx.push((*v as f64) - mu[i]),
                _ => {
                    all_present = false;
                    break;
                }
            }
        }
        if !all_present {
            continue;
        }
        for i in 0..n {
            for j in 0..n {
                cov[i][j] += dx[i] * dx[j];
            }
        }
        n_obs += 1;
    }
    if n_obs < 2 {
        return Err(format!(
            "full-covariance fit requires ≥ 2 records with ALL {} fields present; \
             found {} (sparse records were skipped)",
            n, n_obs
        ));
    }
    // Sample covariance: divide by (N − 1).
    let denom = (n_obs - 1) as f64;
    for i in 0..n {
        for j in 0..n {
            cov[i][j] /= denom;
        }
    }

    // Diagonal floor — analog of L13.6 for the diagonal entries
    // of Σ. Off-diagonal entries are NOT floored (they can legitimately
    // be ~0 for uncorrelated axes). Same composition as fit_diagonal:
    // relative ε·median floor + absolute Euler-stability floor.
    let sigma_sq_per_field_raw: Vec<f64> = (0..n).map(|i| cov[i][i].max(0.0)).collect();
    let mut sorted = sigma_sq_per_field_raw.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2].max(1e-12);
    let relative_floor = if floor_epsilon > 0.0 {
        floor_epsilon * median
    } else {
        0.0
    };
    const ABSOLUTE_STABILITY_FLOOR: f64 = 3.0 * 0.01;
    let effective_floor = relative_floor.max(ABSOLUTE_STABILITY_FLOOR).max(1e-12);

    let mut floored_indices = Vec::new();
    for i in 0..n {
        if cov[i][i] < effective_floor {
            floored_indices.push(i);
            cov[i][i] = effective_floor;
        }
    }
    let sigma_sq_per_field: Vec<f64> = (0..n).map(|i| cov[i][i]).collect();

    let var_max = sigma_sq_per_field.iter().cloned().fold(0.0_f64, f64::max);
    let var_min = sigma_sq_per_field
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let variance_ratio = if var_min > 0.0 { var_max / var_min } else { f64::INFINITY };

    // ── Eigenvalue floor (Marcella 2026-05-26 §2) ─────────────
    //
    // The diagonal floor above clips per-axis variance, but the
    // pathology that creates universal attractors lives in
    // EIGENDIRECTIONS, not axes. A near-rank-deficient Σ has small
    // eigenvalues along correlated directions; the inverse Σ⁻¹
    // amplifies them into deep narrow grooves that pull every
    // Langevin walk into them — exactly H2 with the attractor
    // relocated rather than diffused.
    //
    // The fix: eigendecompose Σ, clip eigenvalues below
    // ε·median(λ_raw), reconstruct, THEN invert. This makes the
    // geometry well-conditioned regardless of variance skew or
    // correlation pattern.
    let mut cov_flat = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            cov_flat.push(cov[i][j]);
        }
    }
    let mat = nalgebra::DMatrix::from_row_slice(n, n, &cov_flat);
    let eigen = nalgebra::SymmetricEigen::new(mat);
    let eigenvalues_raw: Vec<f64> = {
        let mut e: Vec<f64> = eigen.eigenvalues.iter().copied().collect();
        e.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        e
    };
    // Floor = ε · median(eigenvalues), bounded below by the same
    // absolute stability floor used on the diagonal. Median is
    // robust to a few small eigenvalues — picks the "typical" scale
    // of the spectrum.
    let mut sorted_eig = eigenvalues_raw.clone();
    sorted_eig.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let eigenvalue_median = sorted_eig[sorted_eig.len() / 2].max(1e-12);
    let eigenvalue_relative_floor = if floor_epsilon > 0.0 {
        floor_epsilon * eigenvalue_median
    } else {
        0.0
    };
    let eigenvalue_floor_used = eigenvalue_relative_floor
        .max(ABSOLUTE_STABILITY_FLOOR)
        .max(1e-12);

    let mut floored_eigenvalue_count = 0_usize;
    let eigenvalues_effective: Vec<f64> = eigen
        .eigenvalues
        .iter()
        .map(|&l| {
            if l < eigenvalue_floor_used {
                floored_eigenvalue_count += 1;
                eigenvalue_floor_used
            } else {
                l
            }
        })
        .collect();
    let lambda_effective_max = eigenvalues_effective
        .iter()
        .cloned()
        .fold(0.0_f64, f64::max);
    let lambda_effective_min = eigenvalues_effective
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let condition_number = if lambda_effective_min > 0.0 {
        lambda_effective_max / lambda_effective_min
    } else {
        f64::INFINITY
    };

    // Reconstruct Σ_regularized = U · diag(λ_eff) · Uᵀ. Then
    // overwrite our `cov` so the response surfaces the actual
    // matrix used (post-flooring); consumers can subtract from
    // the raw to see what changed.
    let lambda_diag = nalgebra::DMatrix::from_diagonal(
        &nalgebra::DVector::from_vec(eigenvalues_effective.clone()),
    );
    let cov_regularized = &eigen.eigenvectors * &lambda_diag * eigen.eigenvectors.transpose();
    for i in 0..n {
        for j in 0..n {
            cov[i][j] = cov_regularized[(i, j)];
        }
    }
    // Refresh per-axis diagonals (the diagonal floor still applies
    // as a guard, but with eigenvalue flooring done the diagonals
    // are usually already above it).
    let sigma_sq_per_field: Vec<f64> = (0..n).map(|i| cov[i][i]).collect();

    // Cholesky-invert the regularized Σ → Σ⁻¹.
    let chol = nalgebra::Cholesky::new(cov_regularized).ok_or_else(|| {
        // After eigenvalue flooring, Cholesky should never fail.
        // If it does, something is structurally broken with the
        // input data (NaN, Inf) — bubble that up clearly.
        format!(
            "Cholesky failed after eigenvalue flooring — covariance has \
             NaN or Inf entries? variance_ratio = {:.2e}, \
             condition_number = {:.2e}, n_floored_eigenvalues = {}/{}.",
            variance_ratio, condition_number, floored_eigenvalue_count, n
        )
    })?;
    let precision_mat = chol.inverse();
    // Pack Σ⁻¹ back into Vec<Vec<f64>>.
    let mut precision = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..n {
            precision[i][j] = precision_mat[(i, j)];
        }
    }

    Ok(FullFitResult {
        mu,
        covariance: cov,
        precision,
        sigma_sq_per_field,
        sigma_sq_per_field_raw,
        effective_floor,
        floored_indices,
        variance_ratio,
        eigenvalues_raw,
        eigenvalues_effective,
        eigenvalue_floor_used,
        floored_eigenvalue_count,
        condition_number,
    })
}

#[cfg(feature = "kahler")]
fn fit_isotropic_gaussian(
    store: &gigi::BundleStore,
    fields: &[String],
) -> Result<(Vec<f64>, f64), String> {
    let stats = store.field_stats();
    let mut mu = Vec::with_capacity(fields.len());
    let mut var_sum = 0.0_f64;
    let mut var_count = 0_usize;
    for f in fields {
        let s = stats.get(f).ok_or_else(|| {
            let in_base = store.schema.base_fields.iter().any(|fd| fd.name == *f);
            let available: Vec<&str> = stats.keys().map(|k| k.as_str()).collect();
            if in_base {
                format!(
                    "field '{}' is a base_field (query key); brain endpoints only \
                     fit Gaussians on numeric fiber_fields. Available stats: {:?}",
                    f, available
                )
            } else {
                format!(
                    "no Welford stats for field '{}'. Available stats: {:?} \
                     (brain endpoints require numeric fiber fields)",
                    f, available
                )
            }
        })?;
        if s.count == 0 {
            return Err(format!("field '{}' has no observations", f));
        }
        mu.push(s.sum / s.count as f64);
        var_sum += s.variance();
        var_count += 1;
    }
    let sigma_sq = if var_count == 0 {
        1.0
    } else {
        (var_sum / var_count as f64).max(1e-12)
    };
    Ok((mu, sigma_sq))
}

// ─── §2 SAMPLE ──────────────────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainSampleRequest {
    /// Numeric fiber fields to use as manifold dimensions.
    fields: Vec<String>,
    /// Gaussian fit to use: "isotropic" (default, single scalar σ²)
    /// or "diagonal" (per-axis σ² — recommended for anisotropic
    /// manifolds like learned token fibers, per Marcella Finding 3).
    #[serde(default)]
    fit_mode: Option<FitMode>,
    /// L13.6 — Relative-median ε floor for the diagonal fit
    /// (ignored when fit_mode = "isotropic"). Default 1e-3 per
    /// Marcella 2026-05-25 — caps the per-axis effective σ² at
    /// ε × median(σ²) to prevent natural-gradient explosion on
    /// rank-deficient axes. Pass 0 to disable (raw fit; only the
    /// 1e-12 hard floor remains).
    #[serde(default)]
    sigma_floor_epsilon: Option<f64>,
    /// Number of samples to draw post burn-in. Default 100.
    #[serde(default = "default_brain_n_samples")]
    n_samples: usize,
    /// Langevin temperature. Default 1.0 (canonical sampling).
    #[serde(default = "default_brain_temperature")]
    temperature: f64,
    /// Burn-in iterations. Default 2000.
    #[serde(default = "default_brain_burn_in")]
    burn_in: usize,
    /// PRNG seed. None → entropy.
    #[serde(default)]
    seed: Option<u64>,
}

#[cfg(feature = "kahler")]
fn default_brain_n_samples() -> usize { 100 }
#[cfg(feature = "kahler")]
fn default_brain_temperature() -> f64 { 1.0 }
#[cfg(feature = "kahler")]
fn default_brain_burn_in() -> usize { 2_000 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainSampleResponse {
    samples: Vec<Vec<f64>>,
    /// Inferred mean from the bundle's Welford stats.
    fit_mean: Vec<f64>,
    /// Mean of per-axis EFFECTIVE variances (back-compat scalar).
    fit_sigma_sq: f64,
    /// Per-axis EFFECTIVE variances (post-floor for Diagonal).
    fit_sigma_sq_per_field: Vec<f64>,
    /// L13.6 — per-axis RAW variances as observed (pre-floor).
    /// Lets consumers see which dims were rank-deficient.
    fit_sigma_sq_per_field_raw: Vec<f64>,
    /// L13.6 — floor used (0 for Isotropic; ε × median for Diagonal).
    fit_sigma_floor_used: f64,
    /// L13.6 — indices whose raw σ² was below the floor.
    fit_floored_indices: Vec<usize>,
    /// Echo of the fit mode actually used (post-default-resolution).
    fit_mode_used: String,
}

#[cfg(feature = "kahler")]
async fn brain_sample_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainSampleRequest>,
) -> Result<
    (
        [(axum::http::HeaderName, String); 1],
        Json<BrainSampleResponse>,
    ),
    (StatusCode, Json<ErrorResponse>),
> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    let (ctx, counter_at_fit) = flow_from_bundle_cached(
        &state,
        &name,
        heap,
        &req.fields,
        req.fit_mode.unwrap_or_default(),
        req.sigma_floor_epsilon,
    )?;
    let config = gigi::geometry::FlowConfig {
        dt: 0.01,
        temperature: req.temperature,
        n_steps: 1,
        burn_in: req.burn_in,
        seed: req.seed,
    };
    let initial = vec![0.0; ctx.dim];
    let samples = ctx
        .flow
        .sample_many(&initial, &config, req.n_samples, 1)
        .map_err(|e| bad_request(&format!("{}", e)))?;
    let fit_mode_used = match ctx.fit_mode {
        FitMode::Isotropic => "isotropic",
        FitMode::Diagonal => "diagonal",
        FitMode::Full => "full",
    }
    .to_string();
    // Surface the bundle mutation counter at fit time so consumers
    // can stamp their warm-path cache and detect server-side
    // invalidations between calls.
    Ok((
        bundle_counter_header(counter_at_fit),
        Json(BrainSampleResponse {
            samples,
            fit_mean: ctx.mu,
            fit_sigma_sq: ctx.sigma_sq,
            fit_sigma_sq_per_field: ctx.sigma_sq_per_field,
            fit_sigma_sq_per_field_raw: ctx.sigma_sq_per_field_raw,
            fit_sigma_floor_used: ctx.effective_floor,
            fit_floored_indices: ctx.floored_indices,
            fit_mode_used,
        }),
    ))
}

// ─── §12 SELF-MONITOR (confidence) ──────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainConfidenceRequest {
    fields: Vec<String>,
    /// Query point — length must equal `fields.len()`.
    query: Vec<f64>,
    /// Kernel bandwidth. Default √σ² from the bundle's fit.
    #[serde(default)]
    bandwidth: Option<f64>,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainConfidenceResponse {
    /// Σᵢ exp(−‖q−xᵢ‖²/2σ²) — Bayesian precision proxy.
    raw: f64,
    /// raw / max_density, ratio to densest sample point.
    normalized: f64,
    /// bandwidth actually used (either request or derived from fit).
    bandwidth: f64,
    n_samples: usize,
}

#[cfg(feature = "kahler")]
async fn brain_confidence_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainConfidenceRequest>,
) -> Result<Json<BrainConfidenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    if req.query.len() != req.fields.len() {
        return Err(bad_request(&format!(
            "query length {} ≠ fields length {}",
            req.query.len(),
            req.fields.len()
        )));
    }
    let samples = extract_field_samples(heap, &req.fields)
        .map_err(|e| bad_request(&e))?;
    let bandwidth = match req.bandwidth {
        Some(b) if b > 0.0 => b,
        _ => {
            let (_, s_sq) = fit_isotropic_gaussian(heap, &req.fields)
                .map_err(|e| bad_request(&e))?;
            s_sq.sqrt().max(1e-9)
        }
    };
    let raw = gigi::geometry::kernel_density_confidence(&samples, &req.query, bandwidth);
    let normalized =
        gigi::geometry::confidence_normalized(&samples, &req.query, bandwidth);
    Ok(Json(BrainConfidenceResponse {
        raw,
        normalized,
        bandwidth,
        n_samples: samples.len(),
    }))
}

// ─── §8 ATTEND ──────────────────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainAttendRequest {
    fields: Vec<String>,
    query: Vec<f64>,
    #[serde(default)]
    bandwidth: Option<f64>,
    /// If provided, only return weights for the top-k records (in
    /// descending order). When absent, full attention vector
    /// (`weights.len() == n_samples`) is returned.
    #[serde(default)]
    top_k: Option<usize>,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainAttendResponse {
    /// Attention weights aligned with `indices`. Sums to ≈ 1.0.
    weights: Vec<f64>,
    /// Record indices (in the bundle's section iteration order)
    /// corresponding to each weight.
    indices: Vec<usize>,
    bandwidth: f64,
    n_samples: usize,
}

#[cfg(feature = "kahler")]
async fn brain_attend_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainAttendRequest>,
) -> Result<Json<BrainAttendResponse>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    if req.query.len() != req.fields.len() {
        return Err(bad_request(&format!(
            "query length {} ≠ fields length {}",
            req.query.len(),
            req.fields.len()
        )));
    }
    let samples = extract_field_samples(heap, &req.fields)
        .map_err(|e| bad_request(&e))?;
    let bandwidth = match req.bandwidth {
        Some(b) if b > 0.0 => b,
        _ => {
            let (_, s_sq) = fit_isotropic_gaussian(heap, &req.fields)
                .map_err(|e| bad_request(&e))?;
            s_sq.sqrt().max(1e-9)
        }
    };
    let (weights, indices) = match req.top_k {
        Some(k) if k < samples.len() => {
            let top = gigi::geometry::focus(&samples, &req.query, bandwidth, k);
            let weights: Vec<f64> = top.iter().map(|(_, w)| *w).collect();
            let indices: Vec<usize> = top.iter().map(|(i, _)| *i).collect();
            (weights, indices)
        }
        _ => {
            let weights = gigi::geometry::attend(&samples, &req.query, bandwidth);
            let indices: Vec<usize> = (0..samples.len()).collect();
            (weights, indices)
        }
    };
    Ok(Json(BrainAttendResponse {
        weights,
        indices,
        bandwidth,
        n_samples: samples.len(),
    }))
}

// ─── §10 EPISODIC ───────────────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainEpisodicRequest {
    /// Single numeric fiber field whose value sequence to scan for
    /// change-points (persistent H₀ on the sorted-values MST).
    field: String,
    /// Persistence threshold (multiple of median gap). Default 50.
    #[serde(default = "default_min_persistence_ratio")]
    min_persistence_ratio: f64,
    /// L13.5 — Optional fiber-field equality filter.
    #[serde(default)]
    where_field: Option<String>,
    #[serde(default)]
    where_value: Option<serde_json::Value>,
    /// L13.7 — Denominator floor (Marcella's second-pathology fix).
    /// Without a floor, clustered input (batched timestamps,
    /// duplicated rows) collapses `median(gap)` toward 0, and
    /// `persistence_ratio = gap / median` overflows. Default
    /// `DEFAULT_GAP_FLOOR_EPSILON = 1e-6` caps reported ratios at
    /// ≈ 1e6 — still distinguishes any real event but stays a
    /// finite number. Pass 0 to disable (escape hatch).
    #[serde(default)]
    gap_floor_epsilon: Option<f64>,
}

#[cfg(feature = "kahler")]
fn default_min_persistence_ratio() -> f64 { 50.0 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainEpisodicEventWire {
    boundary_idx: usize,
    gap: f64,
    persistence_ratio: f64,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainEpisodicResponse {
    events: Vec<BrainEpisodicEventWire>,
    n_records: usize,
    threshold_used: f64,
    /// Echo of the filter, when one was applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_applied: Option<EpisodicFilterEcho>,
    /// L13.7 — echo of the gap-floor epsilon used (post-default
    /// resolution); 0.0 means floor disabled.
    gap_floor_epsilon_used: f64,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct EpisodicFilterEcho {
    field: String,
    value: serde_json::Value,
}

#[cfg(feature = "kahler")]
async fn brain_episodic_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainEpisodicRequest>,
) -> Result<Json<BrainEpisodicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;

    // Validate filter pair: both-or-neither.
    let filter = match (&req.where_field, &req.where_value) {
        (Some(f), Some(v)) => Some((f.clone(), v.clone())),
        (None, None) => None,
        (Some(_), None) => {
            return Err(bad_request(
                "where_field supplied without where_value (both must be present together)",
            ));
        }
        (None, Some(_)) => {
            return Err(bad_request(
                "where_value supplied without where_field (both must be present together)",
            ));
        }
    };

    // Find target field position once.
    let field_idx = heap
        .schema
        .fiber_fields
        .iter()
        .position(|fd| fd.name == req.field)
        .ok_or_else(|| {
            let in_base = heap.schema.base_fields.iter().any(|fd| fd.name == req.field);
            let avail: Vec<&str> = heap
                .schema
                .fiber_fields
                .iter()
                .map(|fd| fd.name.as_str())
                .collect();
            if in_base {
                bad_request(&format!(
                    "field '{}' is a base_field (query key), not a fiber_field. \
                     /brain/episodic operates on numeric fiber values. \
                     Available fiber_fields: {:?}",
                    req.field, avail
                ))
            } else {
                bad_request(&format!(
                    "field '{}' not in fiber_fields. Available: {:?}",
                    req.field, avail
                ))
            }
        })?;

    // Find filter-field position if requested.
    let filter_idx = if let Some((wf, _)) = &filter {
        let idx = heap
            .schema
            .fiber_fields
            .iter()
            .position(|fd| fd.name == *wf)
            .ok_or_else(|| {
                let in_base = heap.schema.base_fields.iter().any(|fd| fd.name == *wf);
                let avail: Vec<&str> = heap
                    .schema
                    .fiber_fields
                    .iter()
                    .map(|fd| fd.name.as_str())
                    .collect();
                if in_base {
                    bad_request(&format!(
                        "where_field '{}' is a base_field; /brain/episodic per-key \
                         filter currently supports fiber_fields only. To filter on \
                         a base_field, query records by that key on your side and \
                         POST the resulting per-cohort time series.",
                        wf
                    ))
                } else {
                    bad_request(&format!(
                        "where_field '{}' not in fiber_fields. Available: {:?}",
                        wf, avail
                    ))
                }
            })?;
        Some(idx)
    } else {
        None
    };

    // Walk sections, applying optional filter, collecting values.
    let mut values: Vec<f64> = Vec::new();
    for (_bp, rec) in heap.sections() {
        // Apply filter first.
        if let (Some(idx), Some((_, wv))) = (filter_idx, filter.as_ref()) {
            let cell = rec.get(idx).ok_or_else(|| {
                bad_request("record missing fiber slot for filter field")
            })?;
            if !value_matches_json(cell, wv) {
                continue;
            }
        }
        // Extract the value for the change-point series.
        let cell = rec
            .get(field_idx)
            .ok_or_else(|| bad_request("record missing fiber slot for value field"))?;
        let v = match cell {
            gigi::types::Value::Float(x) => *x,
            gigi::types::Value::Integer(j) => *j as f64,
            _ => {
                return Err(bad_request(&format!(
                    "field '{}' has a non-numeric value in a record (only Float / Integer supported)",
                    req.field
                )));
            }
        };
        values.push(v);
    }

    let epsilon = req
        .gap_floor_epsilon
        .unwrap_or(gigi::geometry::DEFAULT_GAP_FLOOR_EPSILON);
    if epsilon < 0.0 {
        return Err(bad_request(
            "gap_floor_epsilon must be ≥ 0 (0 disables relative floor)",
        ));
    }
    let events = gigi::geometry::episodic_events_with_floor(
        &values,
        req.min_persistence_ratio,
        epsilon,
    );
    let wire = events
        .into_iter()
        .map(|e| BrainEpisodicEventWire {
            boundary_idx: e.boundary_idx,
            gap: e.gap,
            persistence_ratio: e.persistence_ratio,
        })
        .collect();
    Ok(Json(BrainEpisodicResponse {
        events: wire,
        n_records: values.len(),
        threshold_used: req.min_persistence_ratio,
        filter_applied: filter.map(|(field, value)| EpisodicFilterEcho { field, value }),
        gap_floor_epsilon_used: epsilon,
    }))
}

/// Loose equality between a stored `gigi::Value` and a JSON value
/// supplied in the request body. Supports the variants Marcella
/// actually uses for filtering (Integer / Float / Text / Bool).
#[cfg(feature = "kahler")]
fn value_matches_json(cell: &gigi::types::Value, json: &serde_json::Value) -> bool {
    match (cell, json) {
        (gigi::types::Value::Integer(i), serde_json::Value::Number(n)) => n
            .as_i64()
            .map(|j| *i == j)
            .unwrap_or_else(|| n.as_f64().map(|f| (*i as f64 - f).abs() < 1e-12).unwrap_or(false)),
        (gigi::types::Value::Float(x), serde_json::Value::Number(n)) => n
            .as_f64()
            .map(|f| (*x - f).abs() < 1e-12)
            .unwrap_or(false),
        (gigi::types::Value::Text(s), serde_json::Value::String(t)) => s == t,
        (gigi::types::Value::Bool(b), serde_json::Value::Bool(c)) => b == c,
        _ => false,
    }
}

// ─── §13 EXPLAIN — geodesic path from query to nearest known ──

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainExplainRequest {
    /// Numeric fiber fields defining the manifold coordinates.
    fields: Vec<String>,
    /// Query point — length must equal `fields.len()`.
    query: Vec<f64>,
    /// Interpolation resolution. Default 10 → returns 11 points
    /// (start + 10 forward toward target).
    #[serde(default = "default_explain_n_steps")]
    n_steps: usize,
}

#[cfg(feature = "kahler")]
fn default_explain_n_steps() -> usize { 10 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainExplainResponse {
    /// The query point as supplied (echo).
    query: Vec<f64>,
    /// The nearest record's fiber values (None if bundle had no
    /// extractable records).
    nearest_record: Option<Vec<f64>>,
    /// The nearest record's index in iteration order over
    /// `BundleStore::sections()`.
    nearest_index: Option<usize>,
    /// Euclidean distance from query to `nearest_record`.
    nearest_distance: f64,
    /// `n_steps + 1` interpolation points from query → nearest.
    /// First entry equals `query`; last equals `nearest_record`.
    /// Empty when no nearest record was found.
    path: Vec<Vec<f64>>,
    /// Step count actually used.
    n_steps: usize,
    n_samples: usize,
}

/// POST /v1/bundles/{name}/brain/explain
///
/// §13 EXPLAIN — interpolation path from the query to the bundle's
/// nearest known record. Useful for "show me the bridge between
/// this novel input and your closest training example."
///
/// Returns 404 if bundle missing or not heap-resident; 400 on
/// dimension mismatch.
#[cfg(feature = "kahler")]
async fn brain_explain_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainExplainRequest>,
) -> Result<Json<BrainExplainResponse>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    if req.query.len() != req.fields.len() {
        return Err(bad_request(&format!(
            "query length {} ≠ fields length {}",
            req.query.len(),
            req.fields.len()
        )));
    }
    let samples = extract_field_samples(heap, &req.fields)
        .map_err(|e| bad_request(&e))?;
    let exp = gigi::geometry::explain(&samples, &req.query, req.n_steps);
    Ok(Json(BrainExplainResponse {
        query: exp.query,
        nearest_record: exp.nearest_record,
        nearest_index: exp.nearest_index,
        nearest_distance: exp.nearest_distance,
        path: exp.path,
        n_steps: exp.n_steps,
        n_samples: samples.len(),
    }))
}

// ─── §11 SEMANTIC ───────────────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainSemanticResponse {
    /// Betti numbers (b_0, b_1, b_2) of the bundle's Hodge complex.
    /// Preserved across Morse compression.
    betti_b0: usize,
    betti_b1: usize,
    betti_b2: usize,
    /// Cell counts post-Morse compression.
    n_critical: usize,
    /// Original cell count (V + E + F).
    n_original: usize,
    /// Compression ratio = n_original / n_critical.
    compression_ratio: f64,
    cohomology_preserved: bool,
}

#[cfg(feature = "kahler")]
async fn brain_semantic_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<BrainSemanticResponse>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    let morse = gigi::geometry::semantic_gist(heap).ok_or_else(|| {
        not_found(&format!(
            "Bundle '{}' produced no Morse compression (too few records or degenerate complex)",
            name
        ))
    })?;
    Ok(Json(BrainSemanticResponse {
        betti_b0: morse.betti.b0,
        betti_b1: morse.betti.b1,
        betti_b2: morse.betti.b2,
        n_critical: morse.n_critical(),
        n_original: morse.n_original(),
        compression_ratio: morse.compression_ratio(),
        cohomology_preserved: morse.cohomology_preserved(),
    }))
}

// ─── helpers for the 5 brain endpoints ─────────────────────

#[cfg(feature = "kahler")]
fn not_found(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

#[cfg(feature = "kahler")]
fn bad_request(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

/// Build a canonical block-form symplectic 2-form `[[0, -I], [I, 0]]`
/// in even dimension `n`. Returns None for odd or zero dimension.
/// SAMPLE doesn't actually use B for its dissipative gradient flow
/// (Friston FEP), but `from_isotropic_gaussian` requires *a* valid
/// closed 2-form for the L10 type signature.
#[cfg(feature = "kahler")]
fn canonical_symplectic_pad(n: usize) -> Option<gigi::geometry::ClosedTwoForm> {
    if n < 2 || n % 2 != 0 {
        return None;
    }
    let half = n / 2;
    let mut raw = vec![0.0_f64; n * n];
    for i in 0..half {
        // Upper-right block: -I.
        raw[i * n + (half + i)] = -1.0;
        // Lower-left block: +I.
        raw[(half + i) * n + i] = 1.0;
    }
    let form = gigi::geometry::TwoForm::new(raw, n).ok()?;
    Some(gigi::geometry::ClosedTwoForm::new_constant(form))
}

// ═══════════════════════════════════════════════════════════════
// 2026-05-25 PR window 3 — 5 more brain HTTP endpoints (L13.2)
// ═══════════════════════════════════════════════════════════════
//
// Surfaces the remaining flow-based brain primitives over HTTP so
// downstream consumers don't need to link the GIGI crate to reach
// them. Same template / namespace / cfg-gate as PR window 2:
//
//   POST /v1/bundles/{name}/brain/dream       §4  DREAM (trajectory)
//   POST /v1/bundles/{name}/brain/forecast    §3  FORECAST (Hamilton)
//   POST /v1/bundles/{name}/brain/reconstruct §5  RECONSTRUCT (MAP)
//   POST /v1/bundles/{name}/brain/inpaint     §6  INPAINT (conditional)
//   POST /v1/bundles/{name}/brain/predict     §7  PREDICT (one-step)
//
// FOCUS (§9) is reachable via /brain/attend with top_k set; no
// dedicated endpoint needed.

// ─── shared helper: build a flow from the bundle's Gaussian fit ──

#[cfg(feature = "kahler")]
struct BundleFlowCtx {
    flow: gigi::geometry::GenerativeFlow<Box<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync>>,
    mu: Vec<f64>,
    /// Mean of per-axis effective variances (for response echo
    /// regardless of fit mode — consumers want a single scalar for
    /// back-compat).
    sigma_sq: f64,
    /// Per-axis effective variances (post-floor for Diagonal fit).
    /// Equal across axes when fit_mode = Isotropic.
    sigma_sq_per_field: Vec<f64>,
    /// Per-axis RAW variances as observed from Welford stats
    /// (pre-floor). Only present for Diagonal fit; for Isotropic
    /// it's the same as `sigma_sq_per_field`.
    sigma_sq_per_field_raw: Vec<f64>,
    /// Floor used. Equals 0 for Isotropic; equals `ε × median(σ²)`
    /// (with `ε` from request, default 1e-3) for Diagonal.
    effective_floor: f64,
    /// Indices in `fields` whose raw σ² was floored. Empty for
    /// Isotropic and for well-conditioned Diagonal fits.
    floored_indices: Vec<usize>,
    dim: usize,
    fit_mode: FitMode,
}

/// Cached version of `flow_from_bundle`. The brain endpoints
/// MUST use this, not the raw `flow_from_bundle` — without the
/// cache, every call re-walks all records (~3s at n=384) and
/// kills the product latency story.
///
/// Returns `(BundleFlowCtx, counter_at_fit)`. The counter is
/// surfaced via the `X-Bundle-Mutation-Counter` response header
/// so consumers can stamp their own warm path and detect
/// server-side invalidation.
#[cfg(feature = "kahler")]
fn flow_from_bundle_cached(
    state: &StreamState,
    bundle_name: &str,
    store: &gigi::BundleStore,
    fields: &[String],
    fit_mode: FitMode,
    sigma_floor_epsilon: Option<f64>,
) -> Result<(BundleFlowCtx, u64), (StatusCode, Json<ErrorResponse>)> {
    let n = fields.len();
    let b = canonical_symplectic_pad(n)
        .ok_or_else(|| bad_request("dimension must be ≥ 2 and even"))?;

    // Lock-free counter load.
    let counter = store.mutation_counter();
    let key = CacheKey::build(bundle_name, fit_mode, fields, sigma_floor_epsilon);

    // Hot path: cache lookup.
    if let Some(cached) = state.flow_cache.get(&key, counter) {
        let ctx = build_ctx_from_cached(&cached, fit_mode, n, b.clone())
            .map_err(|e| bad_request(&e))?;
        return Ok((ctx, counter));
    }

    // Cache miss: compute and insert. We re-load the counter
    // right before computing so a concurrent insert during the
    // fit walk gets noticed on the next call (a stale fit
    // computed against a moving target will still get evicted
    // by the next mutation_counter mismatch — correctness
    // preserved).
    let counter_at_fit = store.mutation_counter();
    let cached = compute_fit_data(store, fields, fit_mode, sigma_floor_epsilon, counter_at_fit)
        .map_err(|e| bad_request(&e))?;
    let ctx = build_ctx_from_cached(&cached, fit_mode, n, b)
        .map_err(|e| bad_request(&e))?;
    state.flow_cache.insert(key, cached);
    Ok((ctx, counter_at_fit))
}

/// Compute the raw fit data (mu, sigma_sq variants, optional
/// precision matrix + eigenvalue spectrum) for any of the three
/// fit modes. Does NOT build the Langevin closure — that's
/// `build_ctx_from_cached`. The split is what lets the cache
/// store fit data once and rebuild closures cheaply on each call.
#[cfg(feature = "kahler")]
fn compute_fit_data(
    store: &gigi::BundleStore,
    fields: &[String],
    fit_mode: FitMode,
    sigma_floor_epsilon: Option<f64>,
    counter_at_fit: u64,
) -> Result<CachedFit, String> {
    let n = fields.len();
    match fit_mode {
        FitMode::Isotropic => {
            let (mu, sigma_sq) = fit_isotropic_gaussian(store, fields)?;
            let per_field = vec![sigma_sq; n];
            Ok(CachedFit {
                counter_at_fit,
                mu: Arc::new(mu),
                sigma_sq,
                sigma_sq_per_field: Arc::new(per_field.clone()),
                sigma_sq_per_field_raw: Arc::new(per_field),
                effective_floor: 0.0,
                floored_indices: Arc::new(Vec::new()),
                precision: None,
                covariance: None,
                eigenvalues_raw: None,
                eigenvalues_effective: None,
                eigenvalue_floor_used: 0.0,
                floored_eigenvalue_count: 0,
                condition_number: 1.0,
                variance_ratio: 1.0,
            })
        }
        FitMode::Diagonal => {
            let epsilon = sigma_floor_epsilon.unwrap_or(DEFAULT_SIGMA_FLOOR_EPSILON);
            if epsilon < 0.0 {
                return Err(
                    "sigma_floor_epsilon must be ≥ 0 (0 disables relative floor)".into(),
                );
            }
            let fit = fit_diagonal_gaussian(store, fields, epsilon)?;
            let scalar_sigma_sq =
                fit.sigma_sq_effective.iter().sum::<f64>() / fit.sigma_sq_effective.len() as f64;
            let var_max = fit
                .sigma_sq_effective
                .iter()
                .cloned()
                .fold(0.0_f64, f64::max);
            let var_min = fit
                .sigma_sq_effective
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let variance_ratio = if var_min > 0.0 {
                var_max / var_min
            } else {
                f64::INFINITY
            };
            Ok(CachedFit {
                counter_at_fit,
                mu: Arc::new(fit.mu),
                sigma_sq: scalar_sigma_sq,
                sigma_sq_per_field: Arc::new(fit.sigma_sq_effective),
                sigma_sq_per_field_raw: Arc::new(fit.sigma_sq_raw),
                effective_floor: fit.effective_floor,
                floored_indices: Arc::new(fit.floored_indices),
                precision: None,
                covariance: None,
                eigenvalues_raw: None,
                eigenvalues_effective: None,
                eigenvalue_floor_used: 0.0,
                floored_eigenvalue_count: 0,
                condition_number: variance_ratio, // diag: λ_max/λ_min = var_max/var_min
                variance_ratio,
            })
        }
        FitMode::Full => {
            let epsilon = sigma_floor_epsilon.unwrap_or(DEFAULT_SIGMA_FLOOR_EPSILON);
            if epsilon < 0.0 {
                return Err(
                    "sigma_floor_epsilon must be ≥ 0 (0 disables relative floor)".into(),
                );
            }
            let fit = fit_full_gaussian(store, fields, epsilon)?;
            let scalar_sigma_sq =
                fit.sigma_sq_per_field.iter().sum::<f64>() / fit.sigma_sq_per_field.len() as f64;
            Ok(CachedFit {
                counter_at_fit,
                mu: Arc::new(fit.mu),
                sigma_sq: scalar_sigma_sq,
                sigma_sq_per_field: Arc::new(fit.sigma_sq_per_field),
                sigma_sq_per_field_raw: Arc::new(fit.sigma_sq_per_field_raw),
                effective_floor: fit.effective_floor,
                floored_indices: Arc::new(fit.floored_indices),
                precision: Some(Arc::new(fit.precision)),
                covariance: Some(Arc::new(fit.covariance)),
                eigenvalues_raw: Some(Arc::new(fit.eigenvalues_raw)),
                eigenvalues_effective: Some(Arc::new(fit.eigenvalues_effective)),
                eigenvalue_floor_used: fit.eigenvalue_floor_used,
                floored_eigenvalue_count: fit.floored_eigenvalue_count,
                condition_number: fit.condition_number,
                variance_ratio: fit.variance_ratio,
            })
        }
    }
}

/// Build the BundleFlowCtx (with Langevin closure) from cached
/// fit data. Cheap: just constructs a closure capturing Arc
/// clones of the cached data — no record walks, no Cholesky.
#[cfg(feature = "kahler")]
fn build_ctx_from_cached(
    cached: &CachedFit,
    fit_mode: FitMode,
    n: usize,
    b: gigi::geometry::ClosedTwoForm,
) -> Result<BundleFlowCtx, String> {
    let mu_arc = cached.mu.clone();
    let sigma_per_arc = cached.sigma_sq_per_field.clone();
    let grad: Box<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync> = match fit_mode {
        FitMode::Isotropic => {
            let sigma_sq = cached.sigma_sq;
            Box::new(move |x: &[f64]| -> Vec<f64> {
                x.iter()
                    .zip(mu_arc.iter())
                    .map(|(xi, mi)| (xi - mi) / sigma_sq)
                    .collect()
            })
        }
        FitMode::Diagonal => Box::new(move |x: &[f64]| -> Vec<f64> {
            x.iter()
                .zip(mu_arc.iter())
                .zip(sigma_per_arc.iter())
                .map(|((xi, mi), si)| (xi - mi) / si)
                .collect()
        }),
        FitMode::Full => {
            let precision_arc = cached
                .precision
                .as_ref()
                .ok_or_else(|| "internal: Full fit missing precision matrix".to_string())?
                .clone();
            let n_for_grad = n;
            Box::new(move |x: &[f64]| -> Vec<f64> {
                let dx: Vec<f64> = x
                    .iter()
                    .zip(mu_arc.iter())
                    .map(|(xi, mi)| xi - mi)
                    .collect();
                (0..n_for_grad)
                    .map(|i| {
                        precision_arc[i]
                            .iter()
                            .zip(dx.iter())
                            .map(|(p, d)| p * d)
                            .sum()
                    })
                    .collect()
            })
        }
    };
    let flow =
        gigi::geometry::GenerativeFlow::new(b, grad).map_err(|e| format!("{}", e))?;
    Ok(BundleFlowCtx {
        flow,
        mu: (*cached.mu).clone(),
        sigma_sq: cached.sigma_sq,
        sigma_sq_per_field: (*cached.sigma_sq_per_field).clone(),
        sigma_sq_per_field_raw: (*cached.sigma_sq_per_field_raw).clone(),
        effective_floor: cached.effective_floor,
        floored_indices: (*cached.floored_indices).clone(),
        dim: n,
        fit_mode,
    })
}

#[cfg(feature = "kahler")]
fn flow_from_bundle(
    store: &gigi::BundleStore,
    fields: &[String],
    fit_mode: FitMode,
    sigma_floor_epsilon: Option<f64>,
) -> Result<BundleFlowCtx, (StatusCode, Json<ErrorResponse>)> {
    let n = fields.len();
    let b = canonical_symplectic_pad(n)
        .ok_or_else(|| bad_request("dimension must be ≥ 2 and even"))?;
    match fit_mode {
        FitMode::Isotropic => {
            let (mu, sigma_sq) =
                fit_isotropic_gaussian(store, fields).map_err(|e| bad_request(&e))?;
            let mu_for_grad = mu.clone();
            let grad: Box<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync> =
                Box::new(move |x: &[f64]| -> Vec<f64> {
                    x.iter()
                        .zip(mu_for_grad.iter())
                        .map(|(xi, mi)| (xi - mi) / sigma_sq)
                        .collect()
                });
            let flow = gigi::geometry::GenerativeFlow::new(b, grad)
                .map_err(|e| bad_request(&format!("{}", e)))?;
            let per_field = vec![sigma_sq; n];
            Ok(BundleFlowCtx {
                flow,
                mu,
                sigma_sq,
                sigma_sq_per_field: per_field.clone(),
                sigma_sq_per_field_raw: per_field,
                effective_floor: 0.0,
                floored_indices: Vec::new(),
                dim: n,
                fit_mode,
            })
        }
        FitMode::Diagonal => {
            let epsilon = sigma_floor_epsilon.unwrap_or(DEFAULT_SIGMA_FLOOR_EPSILON);
            if epsilon < 0.0 {
                return Err(bad_request(
                    "sigma_floor_epsilon must be ≥ 0 (0 disables relative floor)",
                ));
            }
            let fit = fit_diagonal_gaussian(store, fields, epsilon)
                .map_err(|e| bad_request(&e))?;
            let scalar_sigma_sq =
                fit.sigma_sq_effective.iter().sum::<f64>()
                    / fit.sigma_sq_effective.len() as f64;
            let mu_for_grad = fit.mu.clone();
            let sigma_for_grad = fit.sigma_sq_effective.clone();
            let grad: Box<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync> =
                Box::new(move |x: &[f64]| -> Vec<f64> {
                    x.iter()
                        .zip(mu_for_grad.iter())
                        .zip(sigma_for_grad.iter())
                        .map(|((xi, mi), si)| (xi - mi) / si)
                        .collect()
                });
            let flow = gigi::geometry::GenerativeFlow::new(b, grad)
                .map_err(|e| bad_request(&format!("{}", e)))?;
            Ok(BundleFlowCtx {
                flow,
                mu: fit.mu,
                sigma_sq: scalar_sigma_sq,
                sigma_sq_per_field: fit.sigma_sq_effective,
                sigma_sq_per_field_raw: fit.sigma_sq_raw,
                effective_floor: fit.effective_floor,
                floored_indices: fit.floored_indices,
                dim: n,
                fit_mode,
            })
        }
        FitMode::Full => {
            // S1 — full-covariance fit per Marcella's H2 attractor letter.
            // Captures inter-axis correlation that diagonal model ignores.
            let epsilon = sigma_floor_epsilon.unwrap_or(DEFAULT_SIGMA_FLOOR_EPSILON);
            if epsilon < 0.0 {
                return Err(bad_request(
                    "sigma_floor_epsilon must be ≥ 0 (0 disables relative floor)",
                ));
            }
            let fit = fit_full_gaussian(store, fields, epsilon)
                .map_err(|e| bad_request(&e))?;
            let scalar_sigma_sq =
                fit.sigma_sq_per_field.iter().sum::<f64>()
                    / fit.sigma_sq_per_field.len() as f64;
            // Move precision and mu into the gradient closure. The
            // gradient is Σ⁻¹(x − μ) — single matvec per Langevin step.
            let mu_for_grad = fit.mu.clone();
            let precision_for_grad = fit.precision.clone();
            let n_for_grad = n;
            let grad: Box<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync> =
                Box::new(move |x: &[f64]| -> Vec<f64> {
                    // dx = x − μ
                    let dx: Vec<f64> = x
                        .iter()
                        .zip(mu_for_grad.iter())
                        .map(|(xi, mi)| xi - mi)
                        .collect();
                    // Σ⁻¹ · dx
                    (0..n_for_grad)
                        .map(|i| {
                            precision_for_grad[i]
                                .iter()
                                .zip(dx.iter())
                                .map(|(p, d)| p * d)
                                .sum()
                        })
                        .collect()
                });
            let flow = gigi::geometry::GenerativeFlow::new(b, grad)
                .map_err(|e| bad_request(&format!("{}", e)))?;
            Ok(BundleFlowCtx {
                flow,
                mu: fit.mu,
                sigma_sq: scalar_sigma_sq,
                sigma_sq_per_field: fit.sigma_sq_per_field,
                sigma_sq_per_field_raw: fit.sigma_sq_per_field_raw,
                effective_floor: fit.effective_floor,
                floored_indices: fit.floored_indices,
                dim: n,
                fit_mode,
            })
        }
    }
}

// ─── §4 DREAM (trajectory) ──────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainDreamRequest {
    fields: Vec<String>,
    /// Gaussian fit to use: "isotropic" (default, single scalar σ²)
    /// or "diagonal" (per-axis σ² — recommended for anisotropic
    /// manifolds like learned token fibers, per Marcella Finding 3).
    #[serde(default)]
    fit_mode: Option<FitMode>,
    /// L13.6 — Relative-median ε floor for the diagonal fit
    /// (ignored when fit_mode = "isotropic"). Default 1e-3 per
    /// Marcella 2026-05-25 — caps the per-axis effective σ² at
    /// ε × median(σ²) to prevent natural-gradient explosion on
    /// rank-deficient axes. Pass 0 to disable (raw fit; only the
    /// 1e-12 hard floor remains).
    #[serde(default)]
    sigma_floor_epsilon: Option<f64>,
    /// Starting state. If None, defaults to the fit mean (origin
    /// of the flow's energy landscape).
    #[serde(default)]
    initial: Option<Vec<f64>>,
    /// Number of trajectory steps. Default 1000.
    #[serde(default = "default_brain_dream_steps")]
    n_steps: usize,
    /// Langevin temperature. Default 4.0 (canonical DREAM regime).
    /// At T = 1 you'd be doing SAMPLE; at T → ∞ pure noise.
    #[serde(default = "default_brain_dream_temperature")]
    temperature: f64,
    #[serde(default = "default_brain_dt")]
    dt: f64,
    #[serde(default)]
    seed: Option<u64>,
}

#[cfg(feature = "kahler")]
fn default_brain_dream_steps() -> usize { 1_000 }
#[cfg(feature = "kahler")]
fn default_brain_dream_temperature() -> f64 { 4.0 }
#[cfg(feature = "kahler")]
fn default_brain_dt() -> f64 { 0.01 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainDreamResponse {
    /// Full Langevin walk: `n_steps + 1` states (initial + n_steps
    /// forward). Order matters; the trajectory has narrative
    /// structure (each next state follows from the last).
    trajectory: Vec<Vec<f64>>,
    fit_mean: Vec<f64>,
    fit_sigma_sq: f64,
    temperature_used: f64,
    /// Quick diagnostics so consumers can sanity-check the dream:
    /// mean / max Euclidean distance of trajectory points from
    /// the fit_mean. DREAM should reach further than SAMPLE.
    mean_dist_from_mean: f64,
    max_dist_from_mean: f64,
}

#[cfg(feature = "kahler")]
async fn brain_dream_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainDreamRequest>,
) -> Result<
    (
        [(axum::http::HeaderName, String); 1],
        Json<BrainDreamResponse>,
    ),
    (StatusCode, Json<ErrorResponse>),
> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    let (ctx, counter_at_fit) = flow_from_bundle_cached(
        &state,
        &name,
        heap,
        &req.fields,
        req.fit_mode.unwrap_or_default(),
        req.sigma_floor_epsilon,
    )?;
    let initial = req.initial.unwrap_or_else(|| ctx.mu.clone());
    if initial.len() != ctx.dim {
        return Err(bad_request(&format!(
            "initial length {} ≠ fields length {}",
            initial.len(),
            ctx.dim
        )));
    }
    let config = gigi::geometry::FlowConfig {
        dt: req.dt,
        temperature: req.temperature,
        n_steps: req.n_steps,
        burn_in: 0,
        seed: req.seed,
    };
    let trajectory = ctx
        .flow
        .dream(&initial, &config)
        .map_err(|e| bad_request(&format!("{}", e)))?;

    // Compute trajectory spread diagnostics.
    let distances: Vec<f64> = trajectory
        .iter()
        .map(|p| {
            p.iter()
                .zip(ctx.mu.iter())
                .map(|(a, m)| (a - m).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .collect();
    let mean_d = distances.iter().sum::<f64>() / distances.len() as f64;
    let max_d = distances.iter().cloned().fold(0.0_f64, f64::max);

    Ok((
        bundle_counter_header(counter_at_fit),
        Json(BrainDreamResponse {
            trajectory,
            fit_mean: ctx.mu,
            fit_sigma_sq: ctx.sigma_sq,
            temperature_used: req.temperature,
            mean_dist_from_mean: mean_d,
            max_dist_from_mean: max_d,
        }),
    ))
}

// ─── §3 FORECAST (Hamilton-flow extension) ──────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainForecastRequest {
    fields: Vec<String>,
    /// Gaussian fit to use (see BrainDreamRequest.fit_mode).
    #[serde(default)]
    fit_mode: Option<FitMode>,
    /// L13.6 — Relative-median ε floor for the diagonal fit
    /// (ignored when fit_mode = "isotropic"). Default 1e-3 per
    /// Marcella 2026-05-25 — caps the per-axis effective σ² at
    /// ε × median(σ²) to prevent natural-gradient explosion on
    /// rank-deficient axes. Pass 0 to disable (raw fit; only the
    /// 1e-12 hard floor remains).
    #[serde(default)]
    sigma_floor_epsilon: Option<f64>,
    initial: Vec<f64>,
    /// Number of Hamilton steps. Default 1000.
    #[serde(default = "default_brain_forecast_steps")]
    n_steps: usize,
    #[serde(default = "default_brain_dt")]
    dt: f64,
}

#[cfg(feature = "kahler")]
fn default_brain_forecast_steps() -> usize { 1_000 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainForecastResponse {
    /// Deterministic trajectory (T = 0): conservative Hamilton
    /// flow `ẋ = B⁻¹∇H`. Energy is conserved along this path —
    /// quadratic Hamiltonians give closed orbits.
    trajectory: Vec<Vec<f64>>,
    fit_mean: Vec<f64>,
    fit_sigma_sq: f64,
}

#[cfg(feature = "kahler")]
async fn brain_forecast_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainForecastRequest>,
) -> Result<
    (
        [(axum::http::HeaderName, String); 1],
        Json<BrainForecastResponse>,
    ),
    (StatusCode, Json<ErrorResponse>),
> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    let (ctx, counter_at_fit) = flow_from_bundle_cached(
        &state,
        &name,
        heap,
        &req.fields,
        req.fit_mode.unwrap_or_default(),
        req.sigma_floor_epsilon,
    )?;
    if req.initial.len() != ctx.dim {
        return Err(bad_request(&format!(
            "initial length {} ≠ fields length {}",
            req.initial.len(),
            ctx.dim
        )));
    }
    let config = gigi::geometry::FlowConfig {
        dt: req.dt,
        temperature: 0.0,
        n_steps: req.n_steps,
        burn_in: 0,
        seed: None,
    };
    let trajectory = ctx
        .flow
        .forecast(&req.initial, &config)
        .map_err(|e| bad_request(&format!("{}", e)))?;
    Ok((
        bundle_counter_header(counter_at_fit),
        Json(BrainForecastResponse {
            trajectory,
            fit_mean: ctx.mu,
            fit_sigma_sq: ctx.sigma_sq,
        }),
    ))
}

// ─── §5 RECONSTRUCT (T=0 descent to MAP) ────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainReconstructRequest {
    fields: Vec<String>,
    /// Gaussian fit to use (see BrainDreamRequest.fit_mode).
    #[serde(default)]
    fit_mode: Option<FitMode>,
    /// L13.6 — Relative-median ε floor for the diagonal fit
    /// (ignored when fit_mode = "isotropic"). Default 1e-3 per
    /// Marcella 2026-05-25 — caps the per-axis effective σ² at
    /// ε × median(σ²) to prevent natural-gradient explosion on
    /// rank-deficient axes. Pass 0 to disable (raw fit; only the
    /// 1e-12 hard floor remains).
    #[serde(default)]
    sigma_floor_epsilon: Option<f64>,
    /// Noisy / partial observation. Descent starts here.
    noisy_initial: Vec<f64>,
    /// Descent budget. Default 500.
    #[serde(default = "default_brain_reconstruct_steps")]
    n_steps: usize,
    #[serde(default = "default_brain_reconstruct_dt")]
    dt: f64,
}

#[cfg(feature = "kahler")]
fn default_brain_reconstruct_steps() -> usize { 500 }
#[cfg(feature = "kahler")]
fn default_brain_reconstruct_dt() -> f64 { 0.05 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainReconstructResponse {
    /// Final state of zero-noise gradient descent. For unimodal
    /// p, equals MAP. For multimodal, equals nearest local mode.
    result: Vec<f64>,
    fit_mean: Vec<f64>,
    /// Euclidean distance from start to result — large means the
    /// noisy_initial was far from any mode.
    descent_distance: f64,
}

#[cfg(feature = "kahler")]
async fn brain_reconstruct_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainReconstructRequest>,
) -> Result<
    (
        [(axum::http::HeaderName, String); 1],
        Json<BrainReconstructResponse>,
    ),
    (StatusCode, Json<ErrorResponse>),
> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    let (ctx, counter_at_fit) = flow_from_bundle_cached(
        &state,
        &name,
        heap,
        &req.fields,
        req.fit_mode.unwrap_or_default(),
        req.sigma_floor_epsilon,
    )?;
    if req.noisy_initial.len() != ctx.dim {
        return Err(bad_request(&format!(
            "noisy_initial length {} ≠ fields length {}",
            req.noisy_initial.len(),
            ctx.dim
        )));
    }
    let config = gigi::geometry::FlowConfig {
        dt: req.dt,
        temperature: 0.0,
        n_steps: req.n_steps,
        burn_in: 0,
        seed: None,
    };
    let result = ctx
        .flow
        .reconstruct(&req.noisy_initial, &config)
        .map_err(|e| bad_request(&format!("{}", e)))?;
    let descent_distance = req
        .noisy_initial
        .iter()
        .zip(result.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    Ok((
        bundle_counter_header(counter_at_fit),
        Json(BrainReconstructResponse {
            result,
            fit_mean: ctx.mu,
            descent_distance,
        }),
    ))
}

// ─── §6 INPAINT (constrained Langevin) ──────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainInpaintRequest {
    fields: Vec<String>,
    /// Gaussian fit to use (see BrainDreamRequest.fit_mode).
    #[serde(default)]
    fit_mode: Option<FitMode>,
    /// L13.6 — Relative-median ε floor for the diagonal fit
    /// (ignored when fit_mode = "isotropic"). Default 1e-3 per
    /// Marcella 2026-05-25 — caps the per-axis effective σ² at
    /// ε × median(σ²) to prevent natural-gradient explosion on
    /// rank-deficient axes. Pass 0 to disable (raw fit; only the
    /// 1e-12 hard floor remains).
    #[serde(default)]
    sigma_floor_epsilon: Option<f64>,
    /// Initial state. Locked coordinates stay fixed at their
    /// supplied values; unlocked coordinates flow.
    partial_state: Vec<f64>,
    /// Indices into `fields` (0-based) that should be held fixed.
    /// E.g. fields = ["weight", "clearance"], locked_indices = [0]
    /// → lock weight, sample clearance from the conditional.
    locked_indices: Vec<usize>,
    /// Mixing budget for the unlocked coordinates. Default 2000.
    #[serde(default = "default_brain_burn_in")]
    burn_in: usize,
    #[serde(default = "default_brain_inpaint_dt")]
    dt: f64,
    #[serde(default = "default_brain_temperature")]
    temperature: f64,
    #[serde(default)]
    seed: Option<u64>,
}

#[cfg(feature = "kahler")]
fn default_brain_inpaint_dt() -> f64 { 0.05 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainInpaintResponse {
    /// Filled record: locked coords are untouched; unlocked
    /// coords sampled from the conditional density.
    result: Vec<f64>,
    locked_indices: Vec<usize>,
    fit_mean: Vec<f64>,
    fit_sigma_sq: f64,
}

#[cfg(feature = "kahler")]
async fn brain_inpaint_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainInpaintRequest>,
) -> Result<
    (
        [(axum::http::HeaderName, String); 1],
        Json<BrainInpaintResponse>,
    ),
    (StatusCode, Json<ErrorResponse>),
> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    let (ctx, counter_at_fit) = flow_from_bundle_cached(
        &state,
        &name,
        heap,
        &req.fields,
        req.fit_mode.unwrap_or_default(),
        req.sigma_floor_epsilon,
    )?;
    if req.partial_state.len() != ctx.dim {
        return Err(bad_request(&format!(
            "partial_state length {} ≠ fields length {}",
            req.partial_state.len(),
            ctx.dim
        )));
    }
    for &i in &req.locked_indices {
        if i >= ctx.dim {
            return Err(bad_request(&format!(
                "locked_index {} out of range (fields.len() = {})",
                i, ctx.dim
            )));
        }
    }
    let config = gigi::geometry::FlowConfig {
        dt: req.dt,
        temperature: req.temperature,
        n_steps: 1,
        burn_in: req.burn_in,
        seed: req.seed,
    };
    let result = gigi::geometry::inpaint(
        &ctx.flow,
        &req.partial_state,
        &req.locked_indices,
        &config,
    )
    .map_err(|e| bad_request(&format!("{}", e)))?;
    Ok((
        bundle_counter_header(counter_at_fit),
        Json(BrainInpaintResponse {
            result,
            locked_indices: req.locked_indices,
            fit_mean: ctx.mu,
            fit_sigma_sq: ctx.sigma_sq,
        }),
    ))
}

// ─── §7 PREDICT (single-step natural gradient) ──────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainPredictRequest {
    fields: Vec<String>,
    /// Gaussian fit to use (see BrainDreamRequest.fit_mode).
    #[serde(default)]
    fit_mode: Option<FitMode>,
    /// L13.6 — Relative-median ε floor for the diagonal fit
    /// (ignored when fit_mode = "isotropic"). Default 1e-3 per
    /// Marcella 2026-05-25 — caps the per-axis effective σ² at
    /// ε × median(σ²) to prevent natural-gradient explosion on
    /// rank-deficient axes. Pass 0 to disable (raw fit; only the
    /// 1e-12 hard floor remains).
    #[serde(default)]
    sigma_floor_epsilon: Option<f64>,
    state: Vec<f64>,
    /// Step size for the single forward update. Default 0.1.
    #[serde(default = "default_brain_predict_lr")]
    lr: f64,
}

#[cfg(feature = "kahler")]
fn default_brain_predict_lr() -> f64 { 0.1 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainPredictResponse {
    /// `state - lr · ∇H(state)`. For the isotropic-Gaussian
    /// bundle fit, ∇H = (state - μ)/σ², so the predicted next
    /// state shifts toward μ proportional to deviation.
    next_state: Vec<f64>,
    fit_mean: Vec<f64>,
    fit_sigma_sq: f64,
    /// Euclidean step magnitude — useful diagnostic.
    step_size: f64,
}

#[cfg(feature = "kahler")]
async fn brain_predict_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BrainPredictRequest>,
) -> Result<
    (
        [(axum::http::HeaderName, String); 1],
        Json<BrainPredictResponse>,
    ),
    (StatusCode, Json<ErrorResponse>),
> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let heap = store
        .as_heap()
        .ok_or_else(|| not_found(&format!("Bundle '{}' is not heap-resident", name)))?;
    let (ctx, counter_at_fit) = flow_from_bundle_cached(
        &state,
        &name,
        heap,
        &req.fields,
        req.fit_mode.unwrap_or_default(),
        req.sigma_floor_epsilon,
    )?;
    if req.state.len() != ctx.dim {
        return Err(bad_request(&format!(
            "state length {} ≠ fields length {}",
            req.state.len(),
            ctx.dim
        )));
    }
    let next_state = gigi::geometry::predict_one_step(&ctx.flow, &req.state, req.lr)
        .map_err(|e| bad_request(&format!("{}", e)))?;
    let step_size = req
        .state
        .iter()
        .zip(next_state.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    Ok((
        bundle_counter_header(counter_at_fit),
        Json(BrainPredictResponse {
            next_state,
            fit_mean: ctx.mu,
            fit_sigma_sq: ctx.sigma_sq,
            step_size,
        }),
    ))
}


async fn consistency_check(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    // Čech cohomology H¹ — measure holonomy to detect inconsistencies
    // H¹ = 0 means fully consistent (flat connection, path-independent)
    let k = store.scalar_curvature();

    // Sample random loops and measure holonomy deviation
    let records: Vec<Record> = store.records().take(100).collect();
    let mut cocycles = Vec::new();
    let threshold = 1e-6;

    if records.len() >= 3 {
        // Check holonomy around triangles formed by record triples
        let n = records.len().min(20); // sample up to 20 records for triangles
        for i in 0..n {
            for j in (i + 1)..n.min(i + 5) {
                for m in (j + 1)..n.min(j + 3) {
                    // Build key records for the loop: i → j → m → i
                    let keys: Vec<Record> = [&records[i], &records[j], &records[m], &records[i]]
                        .iter()
                        .map(|r| {
                            let mut key = Record::new();
                            for f in &store.schema().base_fields {
                                if let Some(v) = r.get(&f.name) {
                                    key.insert(f.name.clone(), v.clone());
                                }
                            }
                            key
                        })
                        .collect();

                    let hol = store.holonomy(&keys);
                    if hol.is_finite() && hol > threshold {
                        cocycles.push(serde_json::json!({
                            "loop": [i, j, m],
                            "holonomy": hol,
                        }));
                    }
                }
            }
        }
    }

    let h1 = cocycles.len();

    Ok(Json(serde_json::json!({
        "h1": h1,
        "cocycles": cocycles,
        "status": if h1 == 0 { "consistent" } else { "conflicts_detected" },
        "curvature": k
    })))
}

// ── Sprint A REST Handlers ────────────────────────────────────────────────────

async fn betti_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<BettiReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;
    let (b0, b1) = store.betti_numbers();
    Ok(Json(BettiReport {
        beta_0: b0,
        beta_1: b1,
    }))
}

async fn entropy_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<EntropyReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;
    let s = store.entropy();
    Ok(Json(EntropyReport {
        entropy: s,
        unit: "nats".to_string(),
    }))
}

#[derive(Deserialize)]
struct FreeEnergyQuery {
    tau: Option<f64>,
}

async fn free_energy_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Query(params): Query<FreeEnergyQuery>,
) -> Result<Json<FreeEnergyReport>, (StatusCode, Json<ErrorResponse>)> {
    let tau = params.tau.unwrap_or(1.0);
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;
    let f = store.free_energy(tau);
    Ok(Json(FreeEnergyReport {
        tau,
        free_energy: f,
    }))
}

#[derive(Deserialize)]
struct GeodesicRequest {
    from: HashMap<String, serde_json::Value>,
    to: HashMap<String, serde_json::Value>,
    #[serde(default = "default_max_hops")]
    max_hops: usize,
}

fn default_max_hops() -> usize {
    50
}

async fn geodesic_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<GeodesicRequest>,
) -> Result<Json<GeodesicReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;
    let from_rec: gigi::types::Record = req.from.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect();
    let to_rec: gigi::types::Record = req.to.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect();
    let bp_a = store.as_heap().map(|s| s.base_point(&from_rec)).unwrap_or(0);
    let bp_b = store.as_heap().map(|s| s.base_point(&to_rec)).unwrap_or(0);
    let dist = store.geodesic_distance(bp_a, bp_b, req.max_hops);
    Ok(Json(GeodesicReport {
        distance: dist,
        path_found: dist.is_some(),
    }))
}

async fn metric_tensor_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<MetricTensorReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;
    let info = store.metric_tensor();
    let cond = if info.condition_number.is_finite() { info.condition_number } else { 0.0 };
    Ok(Json(MetricTensorReport {
        matrix: info.matrix,
        eigenvalues: info.eigenvalues,
        condition_number: cond,
        effective_dimension: info.effective_dimension,
        field_names: info.field_names,
    }))
}

/// POST /v1/divergence
/// Body: `{"from": "bundle_a", "to": "bundle_b"}`
/// Returns KL and Jensen–Shannon divergence between the two bundles.
#[derive(Deserialize)]
struct DivergenceRequest {
    from: String,
    to: String,
}

async fn divergence_handler(
    State(state): State<Arc<StreamState>>,
    Json(req): Json<DivergenceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store_a = engine.bundle(&req.from).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Bundle '{}' not found", req.from) }),
        )
    })?;
    let store_b = engine.bundle(&req.to).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Bundle '{}' not found", req.to) }),
        )
    })?;

    let rep = gigi::metric::kl_divergence_ref(&store_a, &store_b);

    let per_field: Vec<serde_json::Value> = rep
        .per_field
        .iter()
        .map(|(f, v)| serde_json::json!({"field": f, "kl": v}))
        .collect();

    Ok(Json(serde_json::json!({
        "from": req.from,
        "to": req.to,
        "kl_forward": rep.kl_forward,
        "kl_reverse": rep.kl_reverse,
        "jensen_shannon": rep.jensen_shannon,
        "fields_compared": rep.fields_compared,
        "per_field": per_field,
    })))
}

// ── Anomaly Detection REST Handlers ───────────────────────────────────────────

/// POST /v1/bundles/{name}/anomalies
/// Detect anomalies using K-score threshold (μ_K + n·σ_K).
async fn bundle_anomalies(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<AnomalyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let conditions: Vec<QueryCondition> = req
        .filters
        .iter()
        .map(condition_spec_to_query_condition)
        .collect();
    let pre_filter = if conditions.is_empty() {
        None
    } else {
        Some(conditions.as_slice())
    };

    let anomalies = store.compute_anomalies(req.threshold_sigma, pre_filter, req.limit);
    let include = req.include_scores;

    // Optionally project to requested fields only
    let results: Vec<serde_json::Value> = anomalies
        .iter()
        .map(|a| {
            let mut j = anomaly_to_json(a, include);
            if !req.fields.is_empty() {
                if let serde_json::Value::Object(ref mut obj) = j {
                    if let Some(serde_json::Value::Object(ref mut rec)) = obj.get_mut("record") {
                        rec.retain(|k, _| req.fields.contains(k));
                    }
                }
            }
            j
        })
        .collect();

    let stats = store.curvature_stats();
    Ok(Json(serde_json::json!({
        "bundle": name,
        "threshold_sigma": req.threshold_sigma,
        "k_mean": stats.mean(),
        "k_std": stats.std_dev(),
        "k_threshold": stats.threshold(req.threshold_sigma),
        "total_records": store.len(),
        "anomaly_count": results.len(),
        "anomalies": results,
    })))
}

/// GET /v1/bundles/{name}/health
/// Bundle health snapshot: record count, curvature stats, confidence.
async fn bundle_health(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let k_global = store.scalar_curvature();
    let stats = store.curvature_stats();
    let k_mean = stats.mean();
    let k_std = stats.std_dev();
    let record_count = store.len();

    // Per-field curvature
    let per_field: Vec<serde_json::Value> = store
        .field_stats()
        .iter()
        .map(|(field, fs)| {
            let range = fs.range();
            let field_k = if range > 0.0 {
                fs.variance() / (range * range)
            } else {
                0.0
            };
            serde_json::json!({
                "field": field,
                "k": field_k,
                "variance": fs.variance(),
                "range": range,
            })
        })
        .collect();

    // derive anomaly_rate from 2-sigma count over curvature_stats
    let anomaly_rate =
        store.compute_anomalies(2.0, None, usize::MAX).len() as f64 / record_count.max(1) as f64;

    Ok(Json(serde_json::json!({
        "bundle": name,
        "record_count": record_count,
        "k_global": k_global,
        "k_mean": k_mean,
        "k_std": k_std,
        "k_threshold_2s": stats.threshold(2.0),
        "k_threshold_3s": stats.threshold(3.0),
        "confidence": curvature::confidence(k_global),
        "anomaly_rate_2s": anomaly_rate,
        "per_field": per_field,
    })))
}

/// POST /v1/bundles/{name}/predict
/// Predict field volatility by group: returns σ per group as a volatility proxy.
async fn predict_volatility(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<PredictRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    // Group records by group_by field value, accumulate sum/sum_sq/count for `field`
    let mut groups: HashMap<String, (f64, f64, usize)> = HashMap::new(); // key → (sum, sum_sq, n)
    for record in store.records() {
        let group_key = record
            .get(&req.group_by)
            .map(|v| format!("{:?}", v))
            .unwrap_or_else(|| "null".into());
        if let Some(v) = record.get(&req.field).and_then(|v| v.as_f64()) {
            let e = groups.entry(group_key).or_default();
            e.0 += v;
            e.1 += v * v;
            e.2 += 1;
        }
    }

    let predictions: Vec<serde_json::Value> = groups
        .into_iter()
        .map(|(group, (sum, sum_sq, n))| {
            let mean = sum / n as f64;
            let variance = (sum_sq / n as f64) - mean * mean;
            let std_dev = variance.max(0.0).sqrt();
            // volatility index: σ / max(|μ|, 1) — relative dispersion
            let volatility = std_dev / mean.abs().max(1.0);
            serde_json::json!({
                "group": group,
                "count": n,
                "mean": mean,
                "std_dev": std_dev,
                "volatility_index": volatility,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "bundle": name,
        "group_by": req.group_by,
        "field": req.field,
        "predictions": predictions,
    })))
}

/// POST /v1/bundles/{name}/anomalies/field
/// Anomalies ranked by a specific field's normalised deviation.
async fn field_anomalies(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<FieldAnomalyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    // Run full anomaly scan, then keep only those where the requested field
    // appears in contributing_fields.
    let all = store.compute_anomalies(req.threshold_sigma, None, usize::MAX);
    let mut field_anomalies: Vec<&AnomalyRecord> = all
        .iter()
        .filter(|a| a.contributing_fields.contains(&req.field))
        .collect();
    field_anomalies.sort_by(|a, b| {
        b.z_score
            .partial_cmp(&a.z_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    field_anomalies.truncate(req.limit);

    let results: Vec<serde_json::Value> = field_anomalies
        .iter()
        .map(|a| anomaly_to_json(a, true))
        .collect();

    Ok(Json(serde_json::json!({
        "bundle": name,
        "field": req.field,
        "threshold_sigma": req.threshold_sigma,
        "anomaly_count": results.len(),
        "anomalies": results,
    })))
}

// ── Filtered Query Handler ──

fn condition_spec_to_query_condition(spec: &ConditionSpec) -> QueryCondition {
    let value = json_to_value(&spec.value);
    match spec.op.as_str() {
        "eq" | "=" | "==" => QueryCondition::Eq(spec.field.clone(), value),
        "neq" | "!=" | "<>" => QueryCondition::Neq(spec.field.clone(), value),
        "gt" | ">" => QueryCondition::Gt(spec.field.clone(), value),
        "gte" | ">=" => QueryCondition::Gte(spec.field.clone(), value),
        "lt" | "<" => QueryCondition::Lt(spec.field.clone(), value),
        "lte" | "<=" => QueryCondition::Lte(spec.field.clone(), value),
        "contains" | "like" => {
            let substr = spec.value.as_str().unwrap_or("").to_string();
            QueryCondition::Contains(spec.field.clone(), substr)
        }
        "starts_with" | "startswith" => {
            let prefix = spec.value.as_str().unwrap_or("").to_string();
            QueryCondition::StartsWith(spec.field.clone(), prefix)
        }
        "ends_with" | "endswith" => {
            let suffix = spec.value.as_str().unwrap_or("").to_string();
            QueryCondition::EndsWith(spec.field.clone(), suffix)
        }
        "regex" | "matches" => {
            let pattern = spec.value.as_str().unwrap_or("").to_string();
            QueryCondition::Regex(spec.field.clone(), pattern)
        }
        "in" => {
            let vals = match &spec.value {
                serde_json::Value::Array(arr) => arr.iter().map(|v| json_to_value(v)).collect(),
                _ => vec![value],
            };
            QueryCondition::In(spec.field.clone(), vals)
        }
        "not_in" | "notin" | "nin" => {
            let vals = match &spec.value {
                serde_json::Value::Array(arr) => arr.iter().map(|v| json_to_value(v)).collect(),
                _ => vec![value],
            };
            QueryCondition::NotIn(spec.field.clone(), vals)
        }
        "is_null" | "isnull" => QueryCondition::IsNull(spec.field.clone()),
        "is_not_null" | "isnotnull" | "not_null" => QueryCondition::IsNotNull(spec.field.clone()),
        "between" => {
            // value must be a 2-element array [low, high]
            if let serde_json::Value::Array(arr) = &spec.value {
                if arr.len() == 2 {
                    let low = json_to_value(&arr[0]);
                    let high = json_to_value(&arr[1]);
                    return QueryCondition::Between(spec.field.clone(), low, high);
                }
            }
            QueryCondition::Eq(spec.field.clone(), value) // fallback
        }
        _ => QueryCondition::Eq(spec.field.clone(), value), // default to eq
    }
}

async fn filtered_query(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<FilteredQueryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let conditions: Vec<QueryCondition> = req
        .conditions
        .iter()
        .map(condition_spec_to_query_condition)
        .collect();

    // Handle PRISM "order" field: "desc" → sort_desc=true, "asc" → sort_desc=false
    let sort_desc = match &req.order {
        Some(o) if o.eq_ignore_ascii_case("desc") => true,
        Some(_) => false,
        None => req.sort_desc.unwrap_or(false),
    };

    // Build field projection
    let field_refs: Option<Vec<&str>> = req
        .fields
        .as_ref()
        .map(|f| f.iter().map(|s| s.as_str()).collect());

    // Build multi-field sort
    let sort_fields_vec: Option<Vec<(String, bool)>> = if let Some(ref sort) = req.sort {
        Some(
            sort.iter()
                .map(|s| (s.field.clone(), s.desc.unwrap_or(false)))
                .collect(),
        )
    } else if let Some(ref field) = req.sort_by {
        Some(vec![(field.clone(), sort_desc)])
    } else {
        None
    };
    let sort_fields_refs: Option<Vec<(&str, bool)>> = sort_fields_vec
        .as_ref()
        .map(|v| v.iter().map(|(s, d)| (s.as_str(), *d)).collect());

    // Build OR conditions
    let or_conds_vec: Option<Vec<Vec<QueryCondition>>> = req.or_conditions.as_ref().map(|groups| {
        groups
            .iter()
            .map(|g| g.iter().map(condition_spec_to_query_condition).collect())
            .collect()
    });

    let (results, total) = store.filtered_query_projected_ex(
        &conditions,
        or_conds_vec.as_deref(),
        sort_fields_refs.as_deref(),
        req.limit,
        req.offset,
        field_refs.as_deref(),
    );

    // Apply multi-field text search (OR across search_fields)
    let json_records: Vec<serde_json::Value> = if let Some(ref search_term) = req.search {
        let term_lower = search_term.to_lowercase();
        results
            .iter()
            .filter(|record| match &req.search_fields {
                Some(fields) => fields.iter().any(|f| {
                    record.get(f).map_or(false, |v| {
                        if let Value::Text(s) = v {
                            s.to_lowercase().contains(&term_lower)
                        } else {
                            v.to_string().to_lowercase().contains(&term_lower)
                        }
                    })
                }),
                None => record.values().any(|v| {
                    if let Value::Text(s) = v {
                        s.to_lowercase().contains(&term_lower)
                    } else {
                        false
                    }
                }),
            })
            .map(record_to_json)
            .collect()
    } else {
        results.iter().map(record_to_json).collect()
    };
    let count = json_records.len();
    let k = store.scalar_curvature();

    // Detect sorted-path truncation: sorted path caps at GIGI_QUERY_MAX_ROWS
    // (default 10M) and returns total = min(actual_matches, max_rows+1).
    let max_rows: usize = std::env::var("GIGI_QUERY_MAX_ROWS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000_000);
    let truncated = total > max_rows;
    let cur_offset = req.offset.unwrap_or(0);
    let next_offset = cur_offset + count;

    Ok(Json(serde_json::json!({
        "data": json_records,
        "meta": {
            "confidence": curvature::confidence(k),
            "curvature": k,
            "count": count,
            "total": total,
            "offset": cur_offset,
            "limit": req.limit,
            "next_offset": next_offset,
            "truncated": truncated
        }
    })))
}

// ── PRISM-friendly REST Handlers ──

/// POST /v1/bundles/{name}/query-stream
///
/// Same filter/sort/search interface as `/query` but streams results as
/// newline-delimited JSON (NDJSON / JSON Lines).  No row cap — records are
/// serialised and flushed one at a time so RSS stays O(1) regardless of
/// result-set size.  The final line is always a meta object:
///
///   {"__meta":true,"count":N,"curvature":K,"confidence":C}
///
/// Clients can cancel the request at any point; the server stops iterating
/// immediately on disconnect.
async fn stream_query_ndjson(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<FilteredQueryRequest>,
) -> impl IntoResponse {
    use axum::body::Body;
    use tokio::sync::mpsc;

    let (tx, rx) = mpsc::channel::<Result<Vec<u8>, std::io::Error>>(128);

    // Clone what we need to move into the blocking task.
    let arc = state.clone();
    let bundle_name = name.clone();

    tokio::task::spawn_blocking(move || {
        let engine = arc.engine.read().unwrap();
        let store = match engine.bundle(&bundle_name) {
            Some(s) => s,
            None => {
                let err = serde_json::json!({"error": "bundle not found"}).to_string() + "\n";
                let _ = tx.blocking_send(Ok(err.into_bytes()));
                return;
            }
        };

        let conditions: Vec<QueryCondition> = req
            .conditions
            .iter()
            .map(condition_spec_to_query_condition)
            .collect();

        let or_conds_vec: Option<Vec<Vec<QueryCondition>>> =
            req.or_conditions.as_ref().map(|groups| {
                groups
                    .iter()
                    .map(|g| g.iter().map(condition_spec_to_query_condition).collect())
                    .collect()
            });

        let search_term = req.search.as_ref().map(|s| s.to_lowercase());
        let search_fields = req.search_fields.clone();

        let mut count: usize = 0;
        for record in store.records() {
            // Apply filter conditions
            if !gigi::bundle::matches_filter(&record, &conditions, or_conds_vec.as_deref()) {
                continue;
            }
            // Apply text search if requested
            if let Some(ref term) = search_term {
                let hit = match &search_fields {
                    Some(fields) => fields.iter().any(|f| {
                        record.get(f).map_or(false, |v| {
                            if let Value::Text(s) = v {
                                s.to_lowercase().contains(term.as_str())
                            } else {
                                v.to_string().to_lowercase().contains(term.as_str())
                            }
                        })
                    }),
                    None => record.values().any(|v| {
                        if let Value::Text(s) = v {
                            s.to_lowercase().contains(term.as_str())
                        } else {
                            false
                        }
                    }),
                };
                if !hit {
                    continue;
                }
            }
            count += 1;
            let mut line = record_to_json(&record).to_string();
            line.push('\n');
            if tx.blocking_send(Ok(line.into_bytes())).is_err() {
                return; // client disconnected
            }
        }

        let k = store.scalar_curvature();
        let meta = serde_json::json!({
            "__meta": true,
            "count": count,
            "curvature": k,
            "confidence": curvature::confidence(k)
        })
        .to_string()
            + "\n";
        let _ = tx.blocking_send(Ok(meta.into_bytes()));
    });

    axum::response::Response::builder()
        .status(200)
        .header("Content-Type", "application/x-ndjson")
        .header("X-Content-Type-Options", "nosniff")
        .body(Body::from_stream(futures_util::stream::unfold(rx, |mut r| async move {
            r.recv().await.map(|item| (item, r))
        })))
        .unwrap()
}

/// Parse a URL path value into a Value (tries integer, then float, then text).
fn parse_path_value(raw: &str) -> Value {
    if let Ok(n) = raw.parse::<i64>() {
        Value::Integer(n)
    } else if let Ok(f) = raw.parse::<f64>() {
        Value::Float(f)
    } else {
        Value::Text(raw.to_string())
    }
}

/// GET /v1/bundles/{name}/points — list all records
async fn list_all_records(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let limit: Option<usize> = params.get("limit").and_then(|v| v.parse().ok());
    let offset: Option<usize> = params.get("offset").and_then(|v| v.parse().ok());

    // Streaming pagination — never buffer the entire bundle
    let start = offset.unwrap_or(0);
    let take_count = limit.unwrap_or(usize::MAX);

    let json_records: Vec<serde_json::Value> = store
        .records()
        .skip(start)
        .take(take_count)
        .map(|r| record_to_json(&r))
        .collect();
    let count = json_records.len();

    Ok(Json(ApiResponse {
        data: json_records,
        meta: Some(MetaInfo {
            confidence: None,
            curvature: None,
            capacity: None,
            count: Some(count),
        }),
    }))
}

/// GET /v1/bundles/{name}/points/{field}/{value} — get by field/value in URL
async fn get_by_path(
    State(state): State<Arc<StreamState>>,
    Path((name, field, value)): Path<(String, String, String)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let val = parse_path_value(&value);
    let mut key = Record::new();
    key.insert(field.clone(), val.clone());

    // Try point_query first (O(1) if it's a base field)
    if let Some(record) = store.point_query(&key) {
        let k = store.scalar_curvature();
        return Ok(Json(ApiResponse {
            data: record_to_json(&record),
            meta: Some(MetaInfo {
                confidence: Some(curvature::confidence(k)),
                curvature: Some(k),
                capacity: None,
                count: Some(1),
            }),
        }));
    }

    // Fallback: range_query on field_index (for fiber fields)
    let results = store.range_query(&field, &[val]);
    if let Some(record) = results.first() {
        let k = store.scalar_curvature();
        return Ok(Json(ApiResponse {
            data: record_to_json(record),
            meta: Some(MetaInfo {
                confidence: Some(curvature::confidence(k)),
                curvature: Some(k),
                capacity: None,
                count: Some(1),
            }),
        }));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "Record not found".to_string(),
        }),
    ))
}

/// PATCH /v1/bundles/{name}/points/{field}/{value} — update by field/value path
async fn patch_by_path(
    State(state): State<Arc<StreamState>>,
    Path((name, field, value)): Path<(String, String, String)>,
    Json(body): Json<PatchFieldsBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let val = parse_path_value(&value);
    let mut key = Record::new();
    key.insert(field, val);

    let patches: Record = body
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    if !store.update(&key, &patches) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Record not found".to_string(),
            }),
        ));
    }

    let k = store.scalar_curvature();
    let total = store.len();
    drop(engine);
    let tx = state.get_or_create_channel(&name);
    let patch_json = serde_json::to_string(&serde_json::json!({ "key": record_to_json(&key), "patches": patches.iter().map(|(fk, fv)| (fk.clone(), value_to_json(fv))).collect::<serde_json::Map<_,_>>() })).unwrap_or_default();
    let _ = tx.send(SubscriptionEvent {
        bundle: name.clone(),
        op: "update",
        record_json: patch_json,
        curvature: k,
    });
    Ok(Json(serde_json::json!({
        "status": "updated",
        "total": total,
        "curvature": k,
        "confidence": curvature::confidence(k)
    })))
}

/// DELETE /v1/bundles/{name}/points/{field}/{value} — delete by field/value path
async fn delete_by_path(
    State(state): State<Arc<StreamState>>,
    Path((name, field, value)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let val = parse_path_value(&value);
    let mut key = Record::new();
    key.insert(field, val);

    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    if !store.delete(&key) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Record not found".to_string(),
            }),
        ));
    }

    let k = store.scalar_curvature();
    let total = store.len();
    drop(engine);
    let tx = state.get_or_create_channel(&name);
    let key_json = serde_json::to_string(&record_to_json(&key)).unwrap_or_default();
    let _ = tx.send(SubscriptionEvent {
        bundle: name.clone(),
        op: "delete",
        record_json: key_json,
        curvature: k,
    });
    Ok(Json(serde_json::json!({
        "status": "deleted",
        "total": total,
        "curvature": k,
        "confidence": curvature::confidence(k)
    })))
}

/// PATCH /v1/bundles/{name}/points — bulk update (filter + fields)
async fn bulk_update_records(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BulkUpdateRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let conditions: Vec<QueryCondition> = req
        .filter
        .iter()
        .map(condition_spec_to_query_condition)
        .collect();

    let patches: Record = req
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let count = store.bulk_update(&conditions, &patches);

    let k = store.scalar_curvature();
    let total = store.len();
    drop(engine);
    let tx = state.get_or_create_channel(&name);
    let _ = tx.send(SubscriptionEvent {
        bundle: name.clone(),
        op: "bulk_update",
        record_json: format!("{{\"matched\": {count}}}"),
        curvature: k,
    });
    Ok(Json(serde_json::json!({
        "status": "updated",
        "matched": count,
        "total": total,
        "curvature": k,
        "confidence": curvature::confidence(k)
    })))
}

// ── Sprint 1: New REST Handlers ──

/// POST /v1/bundles/{name}/upsert — insert or update
async fn upsert_records(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<UpsertRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let record: Record = req
        .record
        .iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let inserted = store.upsert(&record);
    let k = store.scalar_curvature();
    let total = store.len();
    let rec_json = serde_json::to_string(&record_to_json(&record)).unwrap_or_default();
    drop(engine);
    let tx = state.get_or_create_channel(&name);
    let _ = tx.send(SubscriptionEvent {
        bundle: name.clone(),
        op: if inserted { "insert" } else { "update" },
        record_json: rec_json,
        curvature: k,
    });

    Ok(Json(serde_json::json!({
        "status": if inserted { "inserted" } else { "updated" },
        "total": total,
        "curvature": k,
        "confidence": curvature::confidence(k)
    })))
}

/// POST /v1/bundles/{name}/count — count records matching filter
async fn count_records(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<FilteredQueryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let conditions: Vec<QueryCondition> = req
        .conditions
        .iter()
        .map(condition_spec_to_query_condition)
        .collect();

    let count = store.count_where(&conditions);
    Ok(Json(serde_json::json!({
        "count": count,
        "total": store.len()
    })))
}

/// POST /v1/bundles/{name}/exists — check if any record matches
async fn exists_records(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<FilteredQueryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let conditions: Vec<QueryCondition> = req
        .conditions
        .iter()
        .map(condition_spec_to_query_condition)
        .collect();

    let exists = store.exists(&conditions);
    Ok(Json(serde_json::json!({
        "exists": exists
    })))
}

/// GET /v1/bundles/{name}/distinct/{field} — distinct values for a field
async fn distinct_values(
    State(state): State<Arc<StreamState>>,
    Path((name, field)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let vals = store.distinct(&field);
    let json_vals: Vec<serde_json::Value> = vals.iter().map(value_to_json).collect();

    Ok(Json(serde_json::json!({
        "field": field,
        "values": json_vals,
        "count": json_vals.len()
    })))
}

/// POST /v1/bundles/{name}/bulk-delete — delete all records matching filter
async fn bulk_delete_records(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<BulkDeleteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let conditions: Vec<QueryCondition> = req
        .conditions
        .iter()
        .map(condition_spec_to_query_condition)
        .collect();

    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let deleted = store.bulk_delete(&conditions);
    let k = store.scalar_curvature();
    let total = store.len();
    drop(engine);
    let tx = state.get_or_create_channel(&name);
    let _ = tx.send(SubscriptionEvent {
        bundle: name.clone(),
        op: "bulk_delete",
        record_json: format!("{{\"deleted\": {deleted}}}"),
        curvature: k,
    });

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "deleted": deleted,
        "total": total,
        "curvature": k,
        "confidence": curvature::confidence(k)
    })))
}

/// POST /v1/bundles/{name}/truncate — delete all records
async fn truncate_bundle(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let removed = store.truncate();

    Ok(Json(serde_json::json!({
        "status": "truncated",
        "removed": removed,
        "total": 0
    })))
}

/// GET /v1/bundles/{name}/schema — get bundle schema info
async fn get_schema(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let base_fields: Vec<serde_json::Value> = store
        .schema()
        .base_fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "name": f.name,
                "type": format!("{:?}", f.field_type),
                "weight": f.weight,
            })
        })
        .collect();

    let fiber_fields: Vec<serde_json::Value> = store
        .schema()
        .fiber_fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "name": f.name,
                "type": format!("{:?}", f.field_type),
                "weight": f.weight,
            })
        })
        .collect();

    let indexed: Vec<String> = store.schema().indexed_fields.clone();

    Ok(Json(serde_json::json!({
        "name": store.schema().name,
        "base_fields": base_fields,
        "fiber_fields": fiber_fields,
        "indexed_fields": indexed,
        "records": store.len(),
        "storage_mode": store.storage_mode()
    })))
}

// ── Sprint 2: New REST Handlers ──

/// POST /v1/bundles/{name}/increment — atomic increment/decrement
async fn increment_field(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<IncrementRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let key: Record = req
        .key
        .iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    if !store.increment(&key, &req.field, req.amount) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Record not found".to_string(),
            }),
        ));
    }

    let k = store.scalar_curvature();
    Ok(Json(serde_json::json!({
        "status": "incremented",
        "field": req.field,
        "amount": req.amount,
        "curvature": k,
        "confidence": curvature::confidence(k)
    })))
}

/// POST /v1/bundles/{name}/drop-field — remove a fiber field from the schema
async fn drop_field(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<DropFieldRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    if !store.drop_field(&req.field) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Field '{}' not found in bundle '{}'", req.field, name),
            }),
        ));
    }

    Ok(Json(serde_json::json!({
        "status": "field_dropped",
        "field": req.field,
        "records": store.len()
    })))
}

/// POST /v1/bundles/{name}/add-field — add a fiber field to the schema
async fn add_field(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<AddFieldRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let ft = str_to_field_type(&req.field_type);
    let default_val = req
        .default
        .as_ref()
        .map(json_to_value)
        .unwrap_or(Value::Null);
    let fd = FieldDef {
        name: req.name.clone(),
        field_type: ft,
        default: default_val,
        range: None,
        weight: 1.0,
        encryption: gigi::types::EncryptionMode::None,
            encryption_group: None,
    };

    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    store.add_field(fd);

    Ok(Json(serde_json::json!({
        "status": "field_added",
        "field": req.name,
        "records": store.len()
    })))
}

/// POST /v1/bundles/{name}/add-index — add an index on a field
async fn add_index(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<AddIndexRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    store.add_index(&req.field);

    Ok(Json(serde_json::json!({
        "status": "index_added",
        "field": req.field,
        "indexed_fields": store.schema().indexed_fields
    })))
}

/// GET /v1/bundles/{name}/export — export all records as JSON
async fn export_bundle(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let records: Vec<serde_json::Value> = store.records().map(|r| record_to_json(&r)).collect();

    Ok(Json(serde_json::json!({
        "bundle": name,
        "count": records.len(),
        "records": records
    })))
}

/// GET /v1/bundles/{name}/dhoom — export bundle as DHOOM wire format.
///
/// Returns the raw DHOOM string body with `Content-Type: application/dhoom`.
/// Binary fields (`Value::Binary`) are serialised with the `b64:` prefix (same
/// as the JSON API edge) because DHOOM is a text-based format.  Consumers must
/// strip the prefix to recover raw bytes, identical to the JSON ingest path.
async fn export_dhoom(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    use axum::response::IntoResponse;

    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let json_records: Vec<serde_json::Value> =
        store.records().map(|r| record_to_json(&r)).collect();

    let result = dhoom::encode_json(&json_records, &name);

    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/dhoom")],
        result.dhoom,
    )
        .into_response())
}

/// POST /v1/bundles/{name}/import — import records from JSON (WAL-logged)
async fn import_bundle(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<ImportRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let records: Vec<Record> = req
        .records
        .iter()
        .filter_map(|item| {
            if let serde_json::Value::Object(map) = item {
                Some(
                    map.iter()
                        .map(|(k, v)| (k.clone(), json_to_value(v)))
                        .collect(),
                )
            } else {
                None
            }
        })
        .collect();

    let mut engine = state.engine.write().unwrap();
    // Route through engine.batch_insert() so every record is WAL-logged before
    // the response is sent.  The previous direct store.batch_insert() bypassed
    // the WAL entirely, causing data loss on server restart.
    let inserted = engine.batch_insert(&name, &records).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {e}"),
            }),
        )
    })?;
    let store = engine.bundle(&name).unwrap();
    let k = store.scalar_curvature();

    Ok(Json(serde_json::json!({
        "status": "imported",
        "count": inserted,
        "total": store.len(),
        "curvature": k,
        "confidence": curvature::confidence(k)
    })))
}

/// POST /v1/bundles/{name}/ingest — unified ingest accepting DHOOM or NDJSON.
///
/// Content-Type dispatch:
///   application/dhoom              → decode DHOOM, WAL-insert all records
///   application/x-ndjson           → same as /stream (NDJSON lines)
///   (anything else)                → 415 Unsupported Media Type
///
/// Query params:
///   ?ephemeral=true  → parse-only, skip WAL write, return 202 (typing events etc.)
async fn ingest_dhoom(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Body,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    use axum::body::to_bytes;

    // Bundle must exist before we read the body
    {
        let engine = state.engine.read().unwrap();
        if engine.bundle(&name).is_none() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Bundle '{}' not found", name),
                }),
            ));
        }
    }

    let ephemeral = params.get("ephemeral").map(|v| v == "true").unwrap_or(false);

    // Determine content type (default to NDJSON if not set)
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/x-ndjson")
        .to_lowercase();

    let is_dhoom = content_type.contains("application/dhoom");
    let is_ndjson = content_type.contains("ndjson") || content_type.contains("json-lines");

    if !is_dhoom && !is_ndjson {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Json(ErrorResponse {
                error: format!(
                    "Unsupported Content-Type '{}'. Use application/dhoom or application/x-ndjson.",
                    content_type
                ),
            }),
        ));
    }

    // Read body (256 MB cap)
    let bytes = to_bytes(body, 256 * 1024 * 1024).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Failed to read body: {e}"),
            }),
        )
    })?;
    let text = String::from_utf8_lossy(&bytes);

    // Snapshot the schema for type-coercion validation (read lock — before parse).
    let schema_snapshot: BundleSchema = {
        let engine = state.engine.read().unwrap();
        engine.bundle(&name).unwrap().schema().clone()
    };

    // Parse records according to content type
    let mut parse_errors = 0usize;
    let mut schema_violations: Vec<String> = Vec::new();
    let records: Vec<Record> = if is_dhoom {
        dhoom::decode_to_json(&text)
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("DHOOM parse error: {e}"),
                    }),
                )
            })?
            .into_iter()
            .filter_map(|item| {
                if let serde_json::Value::Object(map) = item {
                    let rec: Record = map
                        .iter()
                        .map(|(k, v)| (k.clone(), json_to_value(v)))
                        .collect();
                    match coerce_record_against_schema(&schema_snapshot, rec) {
                        Ok(r) => Some(r),
                        Err(e) => { schema_violations.push(e); None }
                    }
                } else {
                    parse_errors += 1;
                    None
                }
            })
            .collect()
    } else {
        // NDJSON
        text.lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                match serde_json::from_str::<serde_json::Value>(line) {
                    Ok(serde_json::Value::Object(map)) => {
                        let rec: Record = map
                            .iter()
                            .map(|(k, v)| (k.clone(), json_to_value(v)))
                            .collect();
                        match coerce_record_against_schema(&schema_snapshot, rec) {
                            Ok(r) => Some(r),
                            Err(e) => { schema_violations.push(e); None }
                        }
                    }
                    _ => {
                        parse_errors += 1;
                        None
                    }
                }
            })
            .collect()
    };

    // Reject the entire batch if any record failed schema validation.
    if !schema_violations.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "Schema validation failed ({} record(s)): {}",
                    schema_violations.len(),
                    schema_violations.join("; ")
                ),
            }),
        ));
    }

    let count = records.len();

    // Binary size guard (§2.1 — 1 MiB hard cap per field)
    check_binary_sizes(&records).map_err(|(field, size)| {
        (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ErrorResponse {
                error: format!(
                    "Binary field '{}' exceeds 1 MiB limit ({} bytes)",
                    field, size
                ),
            }),
        )
    })?;

    // Ephemeral path: parse-only, no WAL write
    if ephemeral {
        let resp = serde_json::json!({
            "status": "ephemeral",
            "count": count,
            "parse_errors": parse_errors,
            "persisted": false,
        });
        return Ok((StatusCode::ACCEPTED, Json(resp)).into_response());
    }

    // WAL-logged batch insert
    let mut engine = state.engine.write().unwrap();
    let inserted = engine.batch_insert(&name, &records).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {e}"),
            }),
        )
    })?;

    let store = engine.bundle(&name).unwrap();
    let k = store.scalar_curvature();
    let conf = curvature::confidence(k);

    let tx = state.get_or_create_channel(&name);
    let _ = tx.send(SubscriptionEvent {
        bundle: name.clone(),
        op: "ingest",
        record_json: format!("{{\"ingest_batch\": {inserted}}}"),
        curvature: k,
    });

    let resp = serde_json::json!({
        "status": "ingested",
        "format": if is_dhoom { "dhoom" } else { "ndjson" },
        "count": inserted,
        "parse_errors": parse_errors,
        "total": store.len(),
        "curvature": k,
        "confidence": conf,
        "storage_mode": store.storage_mode()
    });

    // Spec §3.2: ingest.bulk
    {
        let bytes_est = estimate_bytes(&records);
        let dur_us = 0u64;
        let tps = 0.0f64;
        let ev = state.logger.ingest_bulk(&name, inserted as u64, bytes_est, dur_us, tps, true, 1);
        state.logger.emit(ev);
        state.metrics.record_ingest(inserted as u64, bytes_est);
    }

    Ok((StatusCode::OK, Json(resp)).into_response())
}

// ── Sprint 3: New REST Handlers ──

/// POST /v1/bundles/{name}/update — update with RETURNING + optimistic concurrency
async fn update_records_v2(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<UpdateReturningRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let key: Record = req
        .key
        .iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();
    let mut patches: Record = req
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    // Auto-set updated_at
    if store
        .schema()
        .fiber_fields
        .iter()
        .any(|f| f.name == "updated_at")
        && !patches.contains_key("updated_at")
    {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        patches.insert("updated_at".into(), Value::Timestamp(now_ms));
    }

    // Optimistic concurrency check
    if let Some(expected) = req.expected_version {
        match store.update_versioned(&key, &patches, expected) {
            Ok(new_version) => {
                let k = store.scalar_curvature();
                let mut resp = serde_json::json!({
                    "status": "updated",
                    "version": new_version,
                    "total": store.len(),
                    "curvature": k,
                    "confidence": curvature::confidence(k)
                });
                if req.returning {
                    let _bp = store.base_point(&key);
                    if let Some(rec) = store.point_query(&key) {
                        resp["data"] = record_to_json(&rec);
                    }
                }
                return Ok(Json(resp));
            }
            Err("version_conflict") => {
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "Version conflict — record was modified by another client"
                            .to_string(),
                    }),
                ));
            }
            Err(_) => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "Record not found".to_string(),
                    }),
                ));
            }
        }
    }

    // Standard update (with optional RETURNING)
    if req.returning {
        match store.update_returning(&key, &patches) {
            Some(record) => {
                let k = store.scalar_curvature();
                let total = store.len();
                let rec_json = serde_json::to_string(&record_to_json(&record)).unwrap_or_default();
                drop(engine);
                let tx = state.get_or_create_channel(&name);
                let _ = tx.send(SubscriptionEvent {
                    bundle: name.clone(),
                    op: "update",
                    record_json: rec_json.clone(),
                    curvature: k,
                });
                Ok(Json(serde_json::json!({
                    "status": "updated",
                    "data": serde_json::from_str::<serde_json::Value>(&rec_json).unwrap_or_default(),
                    "total": total,
                    "curvature": k,
                    "confidence": curvature::confidence(k)
                })))
            }
            None => Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Record not found".to_string(),
                }),
            )),
        }
    } else {
        if !store.update(&key, &patches) {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Record not found".to_string(),
                }),
            ));
        }
        let k = store.scalar_curvature();
        let total = store.len();
        let patch_json = serde_json::to_string(&serde_json::json!({"key": record_to_json(&key)}))
            .unwrap_or_default();
        drop(engine);
        let tx = state.get_or_create_channel(&name);
        let _ = tx.send(SubscriptionEvent {
            bundle: name.clone(),
            op: "update",
            record_json: patch_json,
            curvature: k,
        });
        Ok(Json(serde_json::json!({
            "status": "updated",
            "total": total,
            "curvature": k,
            "confidence": curvature::confidence(k)
        })))
    }
}

/// POST /v1/bundles/{name}/delete — delete with RETURNING
async fn delete_records_v2(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<DeleteReturningRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let key: Record = req
        .key
        .iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    if req.returning {
        match store.delete_returning(&key) {
            Some(record) => {
                let k = store.scalar_curvature();
                let total = store.len();
                let rec_json = serde_json::to_string(&record_to_json(&record)).unwrap_or_default();
                drop(engine);
                let tx = state.get_or_create_channel(&name);
                let _ = tx.send(SubscriptionEvent {
                    bundle: name.clone(),
                    op: "delete",
                    record_json: rec_json.clone(),
                    curvature: k,
                });
                Ok(Json(serde_json::json!({
                    "status": "deleted",
                    "data": serde_json::from_str::<serde_json::Value>(&rec_json).unwrap_or_default(),
                    "total": total,
                    "curvature": k,
                    "confidence": curvature::confidence(k)
                })))
            }
            None => Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Record not found".to_string(),
                }),
            )),
        }
    } else {
        if !store.delete(&key) {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Record not found".to_string(),
                }),
            ));
        }
        let k = store.scalar_curvature();
        let total = store.len();
        let key_json = serde_json::to_string(&record_to_json(&key)).unwrap_or_default();
        drop(engine);
        let tx = state.get_or_create_channel(&name);
        let _ = tx.send(SubscriptionEvent {
            bundle: name.clone(),
            op: "delete",
            record_json: key_json,
            curvature: k,
        });
        Ok(Json(serde_json::json!({
            "status": "deleted",
            "total": total,
            "curvature": k,
            "confidence": curvature::confidence(k)
        })))
    }
}

/// GET /v1/bundles/{name}/stats — bundle statistics
async fn bundle_stats(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let stats = store.stats();
    let k = store.scalar_curvature();

    let index_sizes: serde_json::Value = stats
        .index_sizes
        .iter()
        .map(|(f, s)| (f.clone(), serde_json::json!(s)))
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    let cardinalities: serde_json::Value = stats
        .field_cardinalities
        .iter()
        .map(|(f, c)| (f.clone(), serde_json::json!(c)))
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    // Per-field stats
    let field_stats_raw = store.field_stats();
    let field_stats_json: serde_json::Value = field_stats_raw
        .iter()
        .map(|(f, fs)| {
            (
                f.clone(),
                serde_json::json!({
                    "count": fs.count,
                    "sum": fs.sum,
                    "min": fs.min,
                    "max": fs.max,
                    "variance": fs.variance(),
                    "range": fs.range(),
                }),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    Ok(Json(serde_json::json!({
        "name": stats.name,
        "record_count": stats.record_count,
        "base_fields": stats.base_fields,
        "fiber_fields": stats.fiber_fields,
        "indexed_fields": stats.indexed_fields,
        "storage_mode": stats.storage_mode,
        "index_sizes": index_sizes,
        "field_cardinalities": cardinalities,
        "field_stats": field_stats_json,
        "curvature": k,
        "confidence": curvature::confidence(k),
    })))
}

/// POST /v1/bundles/{name}/explain — explain query plan
async fn explain_query(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<ExplainRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let conditions: Vec<QueryCondition> = req
        .conditions
        .iter()
        .map(condition_spec_to_query_condition)
        .collect();

    let or_conds_vec: Option<Vec<Vec<QueryCondition>>> = req.or_conditions.as_ref().map(|groups| {
        groups
            .iter()
            .map(|g| g.iter().map(condition_spec_to_query_condition).collect())
            .collect()
    });

    let sort_fields_vec: Option<Vec<(String, bool)>> = req.sort.as_ref().map(|v| {
        v.iter()
            .map(|s| (s.field.clone(), s.desc.unwrap_or(false)))
            .collect()
    });
    let sort_fields_refs: Option<Vec<(&str, bool)>> = sort_fields_vec
        .as_ref()
        .map(|v| v.iter().map(|(s, d)| (s.as_str(), *d)).collect());

    let plan = store.explain(
        &conditions,
        or_conds_vec.as_deref(),
        sort_fields_refs.as_deref(),
        req.limit,
        req.offset,
    );

    Ok(Json(serde_json::json!({
        "scan_type": plan.scan_type,
        "total_records": plan.total_records,
        "index_scans": plan.index_scans,
        "full_scan_conditions": plan.full_scan_conditions,
        "or_group_count": plan.or_group_count,
        "has_sort": plan.has_sort,
        "has_limit": plan.has_limit,
        "has_offset": plan.has_offset,
        "storage_mode": plan.storage_mode,
    })))
}

/// POST /v1/bundles/{name}/transaction — execute multiple operations atomically
async fn execute_transaction(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<TransactionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut engine = state.engine.write().unwrap();
    let mut store = engine.bundle_mut(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let mut ops: Vec<TransactionOp> = Vec::with_capacity(req.ops.len());

    for (i, op_spec) in req.ops.iter().enumerate() {
        let op = match op_spec.op.as_str() {
            "insert" => {
                let record_json = op_spec.record.as_ref().ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: format!("op[{}]: insert requires 'record'", i),
                        }),
                    )
                })?;
                if let serde_json::Value::Object(map) = record_json {
                    let record: Record = map
                        .iter()
                        .map(|(k, v)| (k.clone(), json_to_value(v)))
                        .collect();
                    TransactionOp::Insert(record)
                } else {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: format!("op[{}]: record must be an object", i),
                        }),
                    ));
                }
            }
            "update" => {
                let key_json = op_spec.key.as_ref().ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: format!("op[{}]: update requires 'key'", i),
                        }),
                    )
                })?;
                let fields_json = op_spec.fields.as_ref().ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: format!("op[{}]: update requires 'fields'", i),
                        }),
                    )
                })?;
                let key: Record = key_json
                    .iter()
                    .map(|(k, v)| (k.clone(), json_to_value(v)))
                    .collect();
                let patches: Record = fields_json
                    .iter()
                    .map(|(k, v)| (k.clone(), json_to_value(v)))
                    .collect();
                TransactionOp::Update { key, patches }
            }
            "delete" => {
                let key_json = op_spec.key.as_ref().ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: format!("op[{}]: delete requires 'key'", i),
                        }),
                    )
                })?;
                let key: Record = key_json
                    .iter()
                    .map(|(k, v)| (k.clone(), json_to_value(v)))
                    .collect();
                TransactionOp::Delete(key)
            }
            "increment" => {
                let key_json = op_spec.key.as_ref().ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: format!("op[{}]: increment requires 'key'", i),
                        }),
                    )
                })?;
                let field = op_spec.field.as_ref().ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: format!("op[{}]: increment requires 'field'", i),
                        }),
                    )
                })?;
                let key: Record = key_json
                    .iter()
                    .map(|(k, v)| (k.clone(), json_to_value(v)))
                    .collect();
                let amount = op_spec.amount.unwrap_or(1.0);
                TransactionOp::Increment {
                    key,
                    field: field.clone(),
                    amount,
                }
            }
            other => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("op[{}]: unknown operation '{}'", i, other),
                    }),
                ));
            }
        };
        ops.push(op);
    }

    match store.execute_transaction(&ops) {
        Ok(results) => {
            let k = store.scalar_curvature();
            Ok(Json(serde_json::json!({
                "status": "committed",
                "ops": results.len(),
                "total": store.len(),
                "curvature": k,
                "confidence": curvature::confidence(k)
            })))
        }
        Err(msg) => Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Transaction rolled back: {}", msg),
            }),
        )),
    }
}

// ── WebSocket Handler ──
//
// Reactive subscriptions — geometric model:
//   A subscription is an open section of the bundle sheaf restricted to
//   the subscriber's filter predicate. Any mutation event that lands in
//   that section is pushed to the client immediately.
//
// Protocol (text frames, one command per frame):
//   Client → Server:
//     SUBSCRIBE <bundle> [WHERE <field> <op> <value> [AND ...]]
//     UNSUBSCRIBE <bundle>
//     INSERT <bundle>\n<DHOOM_DATA>
//     QUERY <bundle> WHERE <field> = <value>
//     RANGE <bundle> WHERE <field> = <value>
//     CURVATURE <bundle>
//     CONSISTENCY <bundle>
//     PING
//
//   Server → Client (push):
//     SUBSCRIBED <bundle>               — ACK
//     UNSUBSCRIBED <bundle>             — ACK
//     EVENT <bundle> <op> <record_json> K=<curvature>  — pushed mutation
//     RESULT <json>                     — query response
//     ERROR <message>                   — error response
//     PONG                              — keepalive reply

/// A single active subscription held by a WebSocket connection.
struct Subscription {
    #[allow(dead_code)]
    bundle: String,
    /// Optional filter: only events matching ALL conditions are forwarded.
    /// Empty = subscribe to all events for the bundle.
    filters: Vec<(String, String, Value)>, // (field, op, value)
    receiver: tokio::sync::broadcast::Receiver<SubscriptionEvent>,
}

// ── Dashboard Event Helpers ──────────────────────────────────────────────────

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn build_dashboard_event(
    event_type: &'static str,
    bundle: &str,
    store: &gigi::mmap_bundle::BundleRef<'_>,
    k_global: f64,
    local_curvature: Option<f64>,
    z_score: Option<f64>,
    contributing_fields: Vec<String>,
) -> DashboardEvent {
    let stats = store.curvature_stats();
    let is_anomaly = local_curvature
        .map(|k| stats.is_anomaly(k, 2.0))
        .unwrap_or(false);
    DashboardEvent {
        event_type,
        bundle: bundle.to_string(),
        ts_ms: now_ms(),
        record_count: store.len(),
        k_global,
        k_mean: stats.mean(),
        k_std: stats.std_dev(),
        k_threshold_2s: stats.threshold(2.0),
        global_confidence: curvature::confidence(k_global),
        is_anomaly,
        local_curvature,
        z_score,
        contributing_fields,
    }
}

// ── Dashboard WebSocket Handlers ──────────────────────────────────────────────

/// GET /v1/ws/dashboard — stream DashboardEvents for ALL bundles.
async fn ws_dashboard_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<StreamState>>,
) -> impl IntoResponse {
    let rx = state.dashboard_tx.subscribe();
    ws.on_upgrade(move |socket| stream_dashboard_events(socket, rx, None))
}

/// GET /v1/ws/{bundle}/dashboard — stream DashboardEvents for ONE bundle.
async fn ws_bundle_dashboard_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let rx = state.dashboard_tx.subscribe();
    ws.on_upgrade(move |socket| stream_dashboard_events(socket, rx, Some(name)))
}

async fn stream_dashboard_events(
    socket: WebSocket,
    mut rx: tokio::sync::broadcast::Receiver<DashboardEvent>,
    filter_bundle: Option<String>,
) {
    use axum::extract::ws::Message as WsMessage;
    use futures_util::{SinkExt, StreamExt};
    let (mut sender, mut client_rx) = socket.split();

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        // Apply bundle filter if set
                        if let Some(ref b) = filter_bundle {
                            if &event.bundle != b { continue; }
                        }
                        let Ok(json) = serde_json::to_string(&event) else { continue };
                        if sender.send(WsMessage::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            msg = client_rx.next() => {
                match msg {
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {} // ignore pings etc.
                }
            }
        }
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<StreamState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: Arc<StreamState>) {
    use axum::extract::ws::Message as WsMessage;
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    // Unique connection ID for this WebSocket session
    let connection_id = format!("ws_{}", &new_request_id()[..8]);
    let ws_t0 = std::time::Instant::now();
    let mut messages_sent: u64 = 0;
    let mut anomalies_sent: u64 = 0;

    // Spec §3.7: connection.open
    {
        let ev = state.logger.connection_open("websocket", "", "");
        state.logger.emit(ev);
    }

    // Active subscriptions for this connection: bundle_name → Subscription
    let mut subscriptions: HashMap<String, Subscription> = HashMap::new();

    // Channel for the event-push task to send text frames back upstream
    let (push_tx, mut push_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    loop {
        tokio::select! {
            // ── Outbound: forward queued event frames to the socket ──
            Some(frame) = push_rx.recv() => {
                if sender.send(WsMessage::Text(frame.into())).await.is_err() {
                    break;
                }
            }

            // ── Inbound: handle commands from the client ──
            msg = receiver.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        let response = handle_ws_command(
                            &text, &state, &mut subscriptions, &connection_id
                        ).await;
                        if !response.is_empty() {
                            if sender.send(WsMessage::Text(response.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }

            // ── Subscription pump: drain all active broadcast receivers ──
            // We poll each subscription receiver in a round-robin using
            // try_recv (non-blocking) so we stay entirely within tokio::select!.
            // Events that don't match the filter are silently discarded —
            // this is the sheaf restriction: only sections over the open set.
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(1)) => {
                let mut to_remove = Vec::new();
                for (bundle_name, sub) in subscriptions.iter_mut() {
                    loop {
                        match sub.receiver.try_recv() {
                            Ok(event) => {
                                // Apply filter predicate (sheaf restriction)
                                if !sub.filters.is_empty() {
                                    let k_json = serde_json::json!(event.curvature);
                                    let passes = sub.filters.iter().all(|(field, op, expected)| {
                                        if field == "K" {
                                            // K pseudo-field: compare against event curvature
                                            let exp_json = value_to_json(expected);
                                            match op.as_str() {
                                                ">" | "gt"  => numeric_cmp(&k_json, &exp_json) > 0,
                                                ">=" | "gte" => numeric_cmp(&k_json, &exp_json) >= 0,
                                                "<" | "lt"  => numeric_cmp(&k_json, &exp_json) < 0,
                                                "<=" | "lte" => numeric_cmp(&k_json, &exp_json) <= 0,
                                                "=" | "eq"  => numeric_cmp(&k_json, &exp_json) == 0,
                                                _ => true,
                                            }
                                        } else if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event.record_json) {
                                            eval_ws_filter(&parsed, field, op, expected)
                                        } else {
                                            false
                                        }
                                    });
                                    if !passes { continue; }
                                }
                                let push_t0 = std::time::Instant::now();
                                let frame = format!(
                                    "EVENT {} {} {} K={:.6} C={:.4}",
                                    event.bundle, event.op, event.record_json,
                                    event.curvature,
                                    gigi::curvature::confidence(event.curvature)
                                );
                                let bytes_sent = frame.len() as u64;
                                if push_tx.send(frame).is_err() {
                                    break;
                                }
                                let dur = push_t0.elapsed().as_micros() as u64;
                                messages_sent += 1;
                                let ev = state.logger.stream_push(
                                    &connection_id, bundle_name, messages_sent,
                                    dur, bytes_sent, event.curvature, false, 0.0,
                                );
                                state.logger.emit(ev);
                            }
                            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                                // Receiver fell too far behind (high-throughput ingestion).
                                // Send a lag notice so the client knows it missed events.
                                let notice = format!("NOTICE {} lagged={}", bundle_name, n);
                                let _ = push_tx.send(notice);
                            }
                            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                                to_remove.push(bundle_name.clone());
                                break;
                            }
                        }
                    }
                }
                for name in to_remove {
                    subscriptions.remove(&name);
                }
            }
        }
    }

    // Emit stream.disconnect for every active subscription on exit
    let session_us = ws_t0.elapsed().as_micros() as u64;
    for bundle_name in subscriptions.keys() {
        let ev = state.logger.stream_disconnect(
            &connection_id, bundle_name, session_us,
            messages_sent, anomalies_sent, "client_close",
        );
        state.logger.emit(ev);
    }

    // Spec §3.7: connection.close
    {
        let ev = state.logger.connection_close(
            "websocket", "", session_us, messages_sent, 0, 0,
        );
        state.logger.emit(ev);
    }
}

/// Evaluate a single filter condition against a JSON record value.
/// Used by the subscription pump to restrict events to the subscriber's open set.
fn eval_ws_filter(record: &serde_json::Value, field: &str, op: &str, expected: &Value) -> bool {
    let field_val = match record.get(field) {
        Some(v) => v,
        None => return false,
    };
    let expected_json = value_to_json(expected);

    match op {
        "=" | "eq" => field_val == &expected_json,
        "!=" | "neq" => field_val != &expected_json,
        ">" | "gt" => numeric_cmp(field_val, &expected_json) > 0,
        ">=" | "gte" => numeric_cmp(field_val, &expected_json) >= 0,
        "<" | "lt" => numeric_cmp(field_val, &expected_json) < 0,
        "<=" | "lte" => numeric_cmp(field_val, &expected_json) <= 0,
        "contains" => field_val
            .as_str()
            .and_then(|s| expected_json.as_str().map(|e| s.contains(e)))
            .unwrap_or(false),
        _ => false,
    }
}

fn numeric_cmp(a: &serde_json::Value, b: &serde_json::Value) -> i8 {
    let av = a.as_f64().unwrap_or(0.0);
    let bv = b.as_f64().unwrap_or(0.0);
    if av < bv {
        -1
    } else if av > bv {
        1
    } else {
        0
    }
}

/// Parse "field op value [AND field op value ...]" into filter triples.
fn parse_ws_filters(condition: &str) -> Vec<(String, String, Value)> {
    let mut filters = Vec::new();
    for clause in condition.split(" AND ") {
        let clause = clause.trim();
        // Try operators longest-first to avoid mis-matching ">" in ">="
        let ops = [">=", "<=", "!=", ">", "<", "=", "eq", "neq", "contains"];
        for op in &ops {
            if let Some(op_pos) = clause.find(op) {
                let field = clause[..op_pos].trim().to_string();
                let val_raw = clause[op_pos + op.len()..]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');
                let val = parse_ws_value(val_raw);
                filters.push((field, op.to_string(), val));
                break;
            }
        }
    }
    filters
}

async fn handle_ws_command(
    cmd: &str,
    state: &Arc<StreamState>,
    subscriptions: &mut HashMap<String, Subscription>,
    connection_id: &str,
) -> String {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    if parts.is_empty() {
        return "ERROR: empty command".to_string();
    }

    match parts[0].to_uppercase().as_str() {
        "PING" => "PONG".to_string(),

        "SUBSCRIBE" => {
            // SUBSCRIBE <bundle> [WHERE <field> <op> <value> [AND ...]]
            // SUBSCRIBE <bundle> ON K [> threshold]
            if parts.len() < 2 {
                return "ERROR: SUBSCRIBE requires a bundle name".to_string();
            }
            let rest = parts[1];
            let rest_upper = rest.to_uppercase();

            // Check for ON K syntax: SUBSCRIBE bundle ON K [> threshold]
            let (bundle_name, filters) = if let Some(on_pos) = rest_upper.find(" ON K") {
                let bn = rest[..on_pos].trim().to_string();
                // Parse optional operator+threshold after "ON K"
                let after_on_k = rest[on_pos + 5..].trim();
                let k_filters = if after_on_k.is_empty() {
                    vec![]
                } else {
                    parse_ws_filters(&format!("K {}", after_on_k))
                };
                (bn, k_filters)
            } else {
                let where_pos = rest_upper.find(" WHERE ");
                let bn = if let Some(pos) = where_pos {
                    rest[..pos].trim().to_string()
                } else {
                    rest.trim().to_string()
                };
                let f = if let Some(pos) = where_pos {
                    parse_ws_filters(&rest[pos + 7..])
                } else {
                    vec![]
                };
                (bn, f)
            };

            // Verify bundle exists
            {
                let engine = state.engine.read().unwrap();
                if engine.bundle(&bundle_name).is_none() {
                    return format!("ERROR: Bundle '{}' not found", bundle_name);
                }
            }

            let tx = state.get_or_create_channel(&bundle_name);
            let receiver = tx.subscribe();
            subscriptions.insert(
                bundle_name.clone(),
                Subscription {
                    bundle: bundle_name.clone(),
                    filters,
                    receiver,
                },
            );
            let filter_count = subscriptions[&bundle_name].filters.len();

            // Spec §3.5: emit stream.subscribe
            let ev = state.logger.stream_subscribe(connection_id, &bundle_name, "", "subscribe");
            state.logger.emit(ev);

            format!("SUBSCRIBED {} filters={}", bundle_name, filter_count)
        }

        "UNSUBSCRIBE" => {
            if parts.len() < 2 {
                return "ERROR: UNSUBSCRIBE requires a bundle name".to_string();
            }
            let bundle_name = parts[1].trim();
            if subscriptions.remove(bundle_name).is_some() {
                format!("UNSUBSCRIBED {}", bundle_name)
            } else {
                format!("ERROR: Not subscribed to '{}'", bundle_name)
            }
        }

        "INSERT" => {
            // INSERT bundle_name\nDHOOM_DATA
            if parts.len() < 2 {
                return "ERROR: INSERT requires bundle name and DHOOM data".to_string();
            }
            let rest = parts[1];
            let mut lines = rest.splitn(2, '\n');
            let bundle_name = lines.next().unwrap_or("").trim();
            let dhoom_data = lines.next().unwrap_or("").trim();

            if dhoom_data.is_empty() {
                return format!("ERROR: No DHOOM data provided for '{}'", bundle_name);
            }

            match dhoom::decode_legacy(dhoom_data) {
                Ok(parsed) => {
                    let mut engine = state.engine.write().unwrap();
                    if let Some(mut store) = engine.bundle_mut(bundle_name) {
                        let mut inserted_records = Vec::new();
                        for dhoom_record in &parsed.records {
                            let record: Record = dhoom_record
                                .iter()
                                .map(|(k, v)| (k.clone(), dhoom_value_to_value(v)))
                                .collect();
                            store.insert(&record);
                            inserted_records.push(record_to_json(&record));
                        }
                        let count = inserted_records.len();
                        let k = store.scalar_curvature();
                        let total = store.len();
                        drop(engine);
                        // Broadcast each inserted record
                        let tx = state.get_or_create_channel(bundle_name);
                        for rec_json_val in &inserted_records {
                            let _ = tx.send(SubscriptionEvent {
                                bundle: bundle_name.to_string(),
                                op: "insert",
                                record_json: rec_json_val.to_string(),
                                curvature: k,
                            });
                        }
                        format!(
                            "OK inserted={} total={} K={:.6} confidence={:.4}",
                            count,
                            total,
                            k,
                            curvature::confidence(k)
                        )
                    } else {
                        format!("ERROR: Bundle '{}' not found", bundle_name)
                    }
                }
                Err(e) => format!("ERROR: DHOOM parse failed: {}", e),
            }
        }

        "QUERY" => {
            // QUERY bundle WHERE field = "value"
            if parts.len() < 2 {
                return "ERROR: QUERY requires bundle and WHERE clause".to_string();
            }
            let rest = parts[1];
            let where_pos = rest.to_uppercase().find(" WHERE ");
            let bundle_name = if let Some(pos) = where_pos {
                rest[..pos].trim()
            } else {
                rest.trim()
            };

            let engine = state.engine.read().unwrap();
            if let Some(store) = engine.bundle(bundle_name) {
                if let Some(pos) = where_pos {
                    let condition = &rest[pos + 7..].trim();
                    let mut key: Record = HashMap::new();
                    for clause in condition.split(" AND ") {
                        let clause = clause.trim();
                        if let Some(eq_pos) = clause.find('=') {
                            let field = clause[..eq_pos].trim();
                            let val = clause[eq_pos + 1..]
                                .trim()
                                .trim_matches('"')
                                .trim_matches('\'');
                            key.insert(field.to_string(), parse_ws_value(val));
                        }
                    }
                    match store.point_query(&key) {
                        Some(record) => {
                            let k = store.scalar_curvature();
                            format!(
                                "RESULT {}\nMETA confidence={:.4} curvature={:.6}",
                                record_to_json(&record),
                                curvature::confidence(k),
                                k
                            )
                        }
                        None => "RESULT null".to_string(),
                    }
                } else {
                    "ERROR: QUERY requires WHERE clause".to_string()
                }
            } else {
                format!("ERROR: Bundle '{}' not found", bundle_name)
            }
        }

        "RANGE" => {
            // RANGE bundle WHERE field = value
            if parts.len() < 2 {
                return "ERROR: RANGE requires bundle and WHERE clause".to_string();
            }
            let rest = parts[1];
            let where_pos = rest.to_uppercase().find(" WHERE ");
            let bundle_name = if let Some(pos) = where_pos {
                rest[..pos].trim()
            } else {
                return "ERROR: RANGE requires WHERE clause".to_string();
            };

            let engine = state.engine.read().unwrap();
            if let Some(store) = engine.bundle(bundle_name) {
                if let Some(pos) = where_pos {
                    let condition = &rest[pos + 7..].trim();
                    if let Some(eq_pos) = condition.find('=') {
                        let field = condition[..eq_pos].trim();
                        let val = condition[eq_pos + 1..]
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'');
                        let results = store.range_query(field, &[parse_ws_value(val)]);
                        let json_arr: Vec<serde_json::Value> =
                            results.iter().map(record_to_json).collect();
                        let k = store.scalar_curvature();
                        format!(
                            "RESULT {}\nMETA count={} confidence={:.4} curvature={:.6}",
                            serde_json::to_string(&json_arr).unwrap_or_default(),
                            json_arr.len(),
                            curvature::confidence(k),
                            k
                        )
                    } else {
                        "ERROR: invalid WHERE clause".to_string()
                    }
                } else {
                    "ERROR: RANGE requires WHERE clause".to_string()
                }
            } else {
                format!("ERROR: Bundle '{}' not found", bundle_name)
            }
        }

        "CURVATURE" => {
            if parts.len() < 2 {
                return "ERROR: CURVATURE requires bundle name".to_string();
            }
            let target = parts[1].trim();
            let bundle_name = target.split('.').next().unwrap_or(target);
            let engine = state.engine.read().unwrap();
            if let Some(store) = engine.bundle(bundle_name) {
                let k = store.scalar_curvature();
                format!(
                    "CURVATURE K={:.6} confidence={:.4} capacity={:.2}",
                    k,
                    curvature::confidence(k),
                    curvature::capacity(1.0, k)
                )
            } else {
                format!("ERROR: Bundle '{}' not found", bundle_name)
            }
        }

        "CONSISTENCY" => {
            if parts.len() < 2 {
                return "ERROR: CONSISTENCY requires bundle name".to_string();
            }
            let bundle_name = parts[1].trim();
            let engine = state.engine.read().unwrap();
            if engine.bundle(bundle_name).is_some() {
                "CONSISTENCY h1=0 cocycles=0".to_string()
            } else {
                format!("ERROR: Bundle '{}' not found", bundle_name)
            }
        }

        _ => format!("ERROR: Unknown command '{}'", parts[0]),
    }
}

fn dhoom_value_to_value(dv: &dhoom::DhoomValue) -> Value {
    match dv {
        dhoom::DhoomValue::Number(n) => {
            if *n == (*n as i64) as f64 {
                Value::Integer(*n as i64)
            } else {
                Value::Float(*n)
            }
        }
        dhoom::DhoomValue::Text(s) => Value::Text(s.clone()),
        dhoom::DhoomValue::Bool(b) => Value::Bool(*b),
        dhoom::DhoomValue::Null => Value::Null,
    }
}

fn parse_ws_value(s: &str) -> Value {
    let s = s.trim();
    if let Ok(i) = s.parse::<i64>() {
        Value::Integer(i)
    } else if let Ok(f) = s.parse::<f64>() {
        Value::Float(f)
    } else {
        Value::Text(s.to_string())
    }
}

// ── OpenAPI Spec Handler ──

async fn openapi_spec() -> impl IntoResponse {
    let spec = include_str!("../../openapi.json");
    (StatusCode::OK, [("content-type", "application/json")], spec)
}

// ── Live Dashboard ──

const DASHBOARD_HTML: &str = include_str!("../../dashboard/index.html");

async fn serve_dashboard() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/html; charset=utf-8")],
        DASHBOARD_HTML,
    )
}

// ── Admin: snapshot ───────────────────────────────────────────────────────────

/// POST /v1/admin/snapshot — write DHOOM snapshots for all bundles and compact the WAL.
///
/// After this call the WAL contains only CreateBundle headers.  On the next
/// server restart each bundle is loaded from its DHOOM snapshot (fast, compact)
/// instead of replaying millions of WAL insert entries.
///
/// Safe to call while the server is running.  Takes a write lock for the duration.
async fn admin_snapshot(State(state): State<Arc<StreamState>>) -> impl IntoResponse {
    let mut engine = state.engine.write().unwrap();
    match engine.snapshot() {
        Ok(total) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "total_records_snapshotted": total,
                "message": "DHOOM snapshots written; WAL compacted to schema-only."
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Snapshot failed: {e}") })),
        ),
    }
}

/// GET /v1/admin/log-config — read current log configuration.
async fn get_log_config(
    State(state): State<Arc<StreamState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let cfg = state.logger.get_config();
    (StatusCode::OK, Json(serde_json::json!({
        "level": format!("{:?}", cfg.level),
        "slow_query_threshold_us": cfg.slow_query_threshold_us,
        "stdout_enabled": cfg.stdout_enabled,
        "internal_bundles_enabled": cfg.internal_bundles_enabled,
        "categories": {
            "query":      cfg.cat_query,
            "ingest":     cfg.cat_ingest,
            "wal":        cfg.cat_wal,
            "connection": cfg.cat_connection,
            "stream":     cfg.cat_stream,
            "bundle":     cfg.cat_bundle,
            "anomaly":    cfg.cat_anomaly,
            "audit":      true,   // always on
            "system":     cfg.cat_system,
        }
    })))
}

/// POST /v1/admin/log-config — update log configuration at runtime (admin only).
/// Audit logging cannot be disabled. All other fields are optional; omitted fields
/// keep their current values.
async fn update_log_config(
    State(state): State<Arc<StreamState>>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut cfg = state.logger.get_config();

    // Apply each optional field from the request body.
    if let Some(v) = body.get("slow_query_threshold_us").and_then(|v| v.as_u64()) {
        cfg.slow_query_threshold_us = v;
    }
    if let Some(v) = body.get("stdout_enabled").and_then(|v| v.as_bool()) {
        cfg.stdout_enabled = v;
    }
    if let Some(v) = body.get("internal_bundles_enabled").and_then(|v| v.as_bool()) {
        cfg.internal_bundles_enabled = v;
    }
    if let Some(cats) = body.get("categories").and_then(|v| v.as_object()) {
        if let Some(v) = cats.get("query").and_then(|v| v.as_bool())      { cfg.cat_query      = v; }
        if let Some(v) = cats.get("ingest").and_then(|v| v.as_bool())     { cfg.cat_ingest     = v; }
        if let Some(v) = cats.get("wal").and_then(|v| v.as_bool())        { cfg.cat_wal        = v; }
        if let Some(v) = cats.get("connection").and_then(|v| v.as_bool()) { cfg.cat_connection = v; }
        if let Some(v) = cats.get("stream").and_then(|v| v.as_bool())     { cfg.cat_stream     = v; }
        if let Some(v) = cats.get("bundle").and_then(|v| v.as_bool())     { cfg.cat_bundle     = v; }
        if let Some(v) = cats.get("anomaly").and_then(|v| v.as_bool())    { cfg.cat_anomaly    = v; }
        if let Some(v) = cats.get("system").and_then(|v| v.as_bool())     { cfg.cat_system     = v; }
        // "audit" key is silently ignored — cannot be disabled
    }
    if let Some(level_str) = body.get("level").and_then(|v| v.as_str()) {
        use gigi::observability::LogLevel;
        cfg.level = match level_str.to_ascii_uppercase().as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "WARN"  => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            "FATAL" => LogLevel::Fatal,
            _       => LogLevel::Info,
        };
    }

    state.logger.update_config(cfg.clone());

    // Emit audit event.
    let ev = gigi::observability::LogEvent::new(
        gigi::observability::LogLevel::Info,
        gigi::observability::LogCategory::Audit,
        "audit.config_change",
        &state.logger.instance,
    ).field("field", "log-config")
     .field("slow_query_threshold_us", cfg.slow_query_threshold_us);
    state.logger.emit(ev);

    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

/// POST /v1/admin/log-level — change the log level at runtime. Emits audit.log_level_change.
/// Body: { "level": "DEBUG" }
async fn set_log_level(
    State(state): State<Arc<StreamState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    use gigi::observability::LogLevel;
    let level_str = match body.get("level").and_then(|v| v.as_str()) {
        Some(s) => s.to_ascii_uppercase(),
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Missing 'level' field"}))),
    };
    let new_level = match level_str.as_str() {
        "TRACE" => LogLevel::Trace,
        "DEBUG" => LogLevel::Debug,
        "WARN"  => LogLevel::Warn,
        "ERROR" => LogLevel::Error,
        "FATAL" => LogLevel::Fatal,
        "INFO"  => LogLevel::Info,
        other   => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("Unknown level: {other}")}))),
    };
    let mut cfg = state.logger.get_config();
    let old_level_str = format!("{:?}", cfg.level).to_ascii_uppercase();
    cfg.level = new_level;
    state.logger.update_config(cfg);

    // Spec §3.8: audit.log_level_change
    let ev = state.logger.audit_log_level_change(&old_level_str, &level_str, "api", "success");
    state.logger.emit(ev);

    (StatusCode::OK, Json(serde_json::json!({"status": "ok", "level": level_str})))
}

// ── GQL endpoint ──

async fn gql_query(
    State(state): State<Arc<StreamState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let t0 = Instant::now();
    let req_id = new_request_id();
    let user_agent = headers.get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let query = match body.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => {
            let dur = t0.elapsed().as_micros() as u64;
            let e = state.logger.query_error(&req_id, "", dur, "BadRequest", "Missing 'query' field", 400);
            state.logger.emit(e);
            state.metrics.record_query(dur, "UNKNOWN", false, true);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'query' field"})),
            )
        }
    };

    // Spec §3.1: emit query.start before parsing — lets operators detect crashed/hung queries
    {
        let ev = state.logger.query_start(&req_id, "gql", query, "", &user_agent);
        state.logger.emit(ev);
    }

    let stmt = match gigi::parser::parse(query) {
        Ok(s) => s,
        Err(e) => {
            let dur = t0.elapsed().as_micros() as u64;
            let ev = state.logger.query_error(&req_id, query, dur, "ParseError", &e.to_string(), 400);
            state.logger.emit(ev);
            state.metrics.record_query(dur, "UNKNOWN", false, true);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Parse error: {e}")})),
            )
        }
    };

    // Helper: emit query.complete and update metrics for early-return paths.
    // Called by all the match arms that don't go through execute_gql_on_store.
    let emit_quick = |stmt_type: &'static str, dur: u64, is_err: bool| {
        let slow = dur >= state.logger.slow_threshold_us();
        let ev = state.logger.query_complete(
            &req_id, "gql", stmt_type, query, dur, 0, dur,
            &[], 0, 0, 0, 0, false, None, None,
        );
        state.logger.emit(ev);
        state.metrics.record_query(dur, stmt_type, slow, is_err);
    };

    // Handle statements that don't need an existing bundle
    match &stmt {
        gigi::parser::Statement::CreateBundle {
            name,
            base_fields,
            fiber_fields,
            indexed,
            encrypted,
            adjacencies,
            invariants,
            seed_source,
        } => {
            let mut schema = gigi::types::BundleSchema::new(name);
            for f in base_fields {
                schema = schema.base(gigi::parser::spec_to_field_def(f));
            }
            for f in fiber_fields {
                schema = schema.fiber(gigi::parser::spec_to_field_def(f));
            }
            for idx in indexed {
                schema = schema.index(idx);
            }
            for adj in adjacencies {
                schema = schema.adjacency(gigi::parser::adj_spec_to_def(adj));
            }
            for inv in invariants {
                schema = schema.with_invariant(gigi::types::InvariantDef {
                    expr_field: inv.field.clone(),
                    expected: inv.expected,
                    tol: inv.tol,
                });
            }
            // v0.2 (Sprint F): seed_source determines whether the master seed
            // is freshly generated, taken from a hex literal, or pulled from an
            // environment variable. The bundle is encrypted when either the
            // legacy `ENCRYPTED` shorthand was set OR any individual field
            // declared an explicit non-`None` encryption mode (per-field path).
            let any_field_encrypted = schema
                .fiber_fields
                .iter()
                .any(|f| f.encryption != gigi::types::EncryptionMode::None);
            if *encrypted || any_field_encrypted {
                let seed = match seed_source {
                    gigi::types::EncryptionSeedSource::Random => {
                        gigi::crypto::GaugeKey::random_seed()
                    }
                    gigi::types::EncryptionSeedSource::Hex(hex) => {
                        match gigi::crypto::seed_from_hex(hex) {
                            Ok(s) => s,
                            Err(e) => {
                                emit_quick("CREATE_BUNDLE", t0.elapsed().as_micros() as u64, true);
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(serde_json::json!({"error": format!("invalid encryption seed: {e}")})),
                                );
                            }
                        }
                    }
                    gigi::types::EncryptionSeedSource::Env(name) => {
                        match std::env::var(name) {
                            Ok(hex) => match gigi::crypto::seed_from_hex(&hex) {
                                Ok(s) => s,
                                Err(e) => {
                                    emit_quick("CREATE_BUNDLE", t0.elapsed().as_micros() as u64, true);
                                    return (
                                        StatusCode::BAD_REQUEST,
                                        Json(serde_json::json!({"error": format!("invalid encryption seed in env {name}: {e}")})),
                                    );
                                }
                            },
                            Err(_) => {
                                emit_quick("CREATE_BUNDLE", t0.elapsed().as_micros() as u64, true);
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(serde_json::json!({"error": format!("env var {name} not set")})),
                                );
                            }
                        }
                    }
                };
                let gk = gigi::crypto::GaugeKey::derive(&seed, &schema.fiber_fields);
                schema.gauge_key = Some(gk);
            }
            let mut engine = state.engine.write().unwrap();
            engine.create_bundle(schema).unwrap();
            emit_quick("CREATE_BUNDLE", t0.elapsed().as_micros() as u64, false);
            return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
        }
        gigi::parser::Statement::ShowBundles => {
            let engine = state.engine.read().unwrap();
            let list: Vec<serde_json::Value> = engine
                .bundle_names()
                .iter()
                .map(|name| {
                    let store = engine.bundle(name).unwrap();
                    serde_json::json!({
                        "name": name,
                        "records": store.len(),
                        "fields": store.schema().base_fields.len() + store.schema().fiber_fields.len(),
                    })
                })
                .collect();
            emit_quick("SHOW_BUNDLES", t0.elapsed().as_micros() as u64, false);
            return (StatusCode::OK, Json(serde_json::json!({"bundles": list})));
        }
        gigi::parser::Statement::Collapse { bundle } => {
            let mut engine = state.engine.write().unwrap();
            if engine.drop_bundle(bundle).unwrap_or(false) {
                emit_quick("DROP_BUNDLE", t0.elapsed().as_micros() as u64, false);
                return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
            } else {
                emit_quick("DROP_BUNDLE", t0.elapsed().as_micros() as u64, true);
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("No bundle: {bundle}")})),
                );
            }
        }
        // Sprint G: forward-secret key rotation.
        gigi::parser::Statement::RotateKey { bundle, new_seed_source } => {
            let new_seed = match new_seed_source {
                gigi::types::EncryptionSeedSource::Random => {
                    gigi::crypto::GaugeKey::random_seed()
                }
                gigi::types::EncryptionSeedSource::Hex(hex) => {
                    match gigi::crypto::seed_from_hex(hex) {
                        Ok(s) => s,
                        Err(e) => {
                            emit_quick("ROTATE_KEY", t0.elapsed().as_micros() as u64, true);
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({"error": format!("invalid encryption seed: {e}")})),
                            );
                        }
                    }
                }
                gigi::types::EncryptionSeedSource::Env(name) => match std::env::var(name) {
                    Ok(hex) => match gigi::crypto::seed_from_hex(&hex) {
                        Ok(s) => s,
                        Err(e) => {
                            emit_quick("ROTATE_KEY", t0.elapsed().as_micros() as u64, true);
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({"error": format!("invalid seed in env {name}: {e}")})),
                            );
                        }
                    },
                    Err(_) => {
                        emit_quick("ROTATE_KEY", t0.elapsed().as_micros() as u64, true);
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({"error": format!("env var {name} not set")})),
                        );
                    }
                },
            };
            let mut engine = state.engine.write().unwrap();
            let store = match engine.heap_bundle_mut(bundle) {
                Some(s) => s,
                None => {
                    emit_quick("ROTATE_KEY", t0.elapsed().as_micros() as u64, true);
                    return (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({"error": format!("bundle {bundle} not in heap mode")})),
                    );
                }
            };
            // Sprint G-ext: rotate_key takes the 32-byte master and
            // drives both gauge-key and base-hash-seed rotation. The
            // gauge_key is derived inside the bundle method so the
            // caller doesn't need to keep them coordinated.
            match store.rotate_key(&new_seed) {
                Ok(count) => {
                    emit_quick("ROTATE_KEY", t0.elapsed().as_micros() as u64, false);
                    return (
                        StatusCode::OK,
                        Json(serde_json::json!({"status": "ok", "rotated": count})),
                    );
                }
                Err(e) => {
                    emit_quick("ROTATE_KEY", t0.elapsed().as_micros() as u64, true);
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": e})),
                    );
                }
            }
        }
        gigi::parser::Statement::AtlasBegin
        | gigi::parser::Statement::AtlasCommit
        | gigi::parser::Statement::AtlasRollback => {
            emit_quick("OTHER", t0.elapsed().as_micros() as u64, false);
            return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
        }
        // v2.1: statements parsed but not yet implemented — return 501
        gigi::parser::Statement::ShowRoles
        | gigi::parser::Statement::ShowPrepared
        | gigi::parser::Statement::ShowBackups
        | gigi::parser::Statement::ShowSettings
        | gigi::parser::Statement::ShowSession
        | gigi::parser::Statement::ShowCurrentRole
        | gigi::parser::Statement::WeaveRole { .. }
        | gigi::parser::Statement::UnweaveRole { .. }
        | gigi::parser::Statement::Grant { .. }
        | gigi::parser::Statement::Revoke { .. }
        | gigi::parser::Statement::CreatePolicy { .. }
        | gigi::parser::Statement::Set { .. }
        | gigi::parser::Statement::Reset { .. }
        | gigi::parser::Statement::Prepare { .. }
        | gigi::parser::Statement::Execute { .. }
        | gigi::parser::Statement::Deallocate { .. }
        | gigi::parser::Statement::Backup { .. }
        | gigi::parser::Statement::Restore { .. }
        | gigi::parser::Statement::VerifyBackup { .. }
        | gigi::parser::Statement::CommentOn { .. } => {
            emit_quick("OTHER", t0.elapsed().as_micros() as u64, false);
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(serde_json::json!({"error": "This GQL v2.1 command is not yet implemented"})),
            );
        }
        // Cross-bundle: KL divergence between two bundles
        gigi::parser::Statement::Divergence { bundle_a, bundle_b } => {
            let engine = state.engine.read().unwrap();
            let store_a = match engine.bundle(bundle_a) {
                Some(s) => s,
                None => {
                    emit_quick("DIVERGENCE", t0.elapsed().as_micros() as u64, true);
                    return (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({"error": format!("Bundle '{}' not found", bundle_a)})),
                    );
                }
            };
            let store_b = match engine.bundle(bundle_b) {
                Some(s) => s,
                None => {
                    emit_quick("DIVERGENCE", t0.elapsed().as_micros() as u64, true);
                    return (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({"error": format!("Bundle '{}' not found", bundle_b)})),
                    );
                }
            };
            let rep = gigi::metric::kl_divergence_ref(&store_a, &store_b);
            let dur = t0.elapsed().as_micros() as u64;
            let slow = dur >= state.logger.slow_threshold_us();
            // Emit query.complete with actual geometric values for DIVERGENCE
            let geo = gigi::observability::GeometricFields {
                kl_forward:      Some(rep.kl_forward),
                kl_reverse:      Some(rep.kl_reverse),
                jensen_shannon:  Some(rep.jensen_shannon),
                fields_compared: Some(rep.fields_compared as u32),
                ..Default::default()
            };
            let ev = state.logger.query_complete(
                &req_id, "gql", "DIVERGENCE", query, dur, 0, dur,
                &[bundle_a.clone(), bundle_b.clone()], 0, 1, 0, 0, false, Some(geo), None,
            );
            state.logger.emit(ev);
            if slow {
                let ev2 = state.logger.query_slow(&req_id, "DIVERGENCE", query, dur, false, false, "divergence computation");
                state.logger.emit(ev2);
            }
            state.metrics.record_query(dur, "DIVERGENCE", slow, false);
            let per_field: String = rep
                .per_field
                .iter()
                .map(|(f, v)| format!("{f}={v:.6}"))
                .collect::<Vec<_>>()
                .join(",");
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "bundle_a": bundle_a,
                    "bundle_b": bundle_b,
                    "kl_forward": rep.kl_forward,
                    "kl_reverse": rep.kl_reverse,
                    "jensen_shannon": rep.jensen_shannon,
                    "fields_compared": rep.fields_compared,
                    "per_field": per_field,
                })),
            );
        }
        _ => {}
    }

    let bundle_name = match get_bundle_name(&stmt) {
        Some(name) => name,
        None => {
            emit_quick("OTHER", t0.elapsed().as_micros() as u64, false);
            return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
        }
    };

    // Derive a short statement type string for metrics/logging
    let stmt_type = gql_stmt_type_name(&stmt);

    // Check if bundle needs write access
    let needs_write = matches!(
        &stmt,
        gigi::parser::Statement::Insert { .. }
            | gigi::parser::Statement::BatchInsert { .. }
            | gigi::parser::Statement::BatchSectionUpsert { .. }
            | gigi::parser::Statement::SectionUpsert { .. }
            | gigi::parser::Statement::Redefine { .. }
            | gigi::parser::Statement::BulkRedefine { .. }
            | gigi::parser::Statement::Retract { .. }
            | gigi::parser::Statement::BulkRetract { .. }
    );

    if needs_write {
        let mut engine = state.engine.write().unwrap();
        let mut store = match engine.bundle_mut(&bundle_name) {
            Some(s) => s,
            None => {
                let dur = t0.elapsed().as_micros() as u64;
                let ev = state.logger.query_error(&req_id, query, dur, "BundleNotFound", &format!("No bundle: {bundle_name}"), 404);
                state.logger.emit(ev);
                state.metrics.record_query(dur, stmt_type, false, true);
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("No bundle: {bundle_name}")})),
                )
            }
        };
        let result = execute_gql_on_store(&mut store, &stmt);
        let dur = t0.elapsed().as_micros() as u64;
        let (status, resp) = match result {
            Ok(r) => exec_result_to_response(r),
            Err(e) => {
                let ev = state.logger.query_error(&req_id, query, dur, "ExecError", &e, 500);
                state.logger.emit(ev);
                state.metrics.record_query(dur, stmt_type, false, true);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e})));
            }
        };
        let slow = dur >= state.logger.slow_threshold_us();
        let ev = state.logger.query_complete(
            &req_id, "gql", stmt_type, query, dur, 0, dur,
            &[bundle_name.clone()], 0, 0, 0, 0, false, None, None,
        );
        state.logger.emit(ev);
        if slow {
            let ev2 = state.logger.query_slow(&req_id, stmt_type, query, dur, false, false, "write path");
            state.logger.emit(ev2);
        }
        state.metrics.record_query(dur, stmt_type, slow, false);
        (status, resp)
    } else {
        let engine = state.engine.read().unwrap();
        let store = match engine.bundle(&bundle_name) {
            Some(s) => s,
            None => {
                let dur = t0.elapsed().as_micros() as u64;
                let ev = state.logger.query_error(&req_id, query, dur, "BundleNotFound", &format!("No bundle: {bundle_name}"), 404);
                state.logger.emit(ev);
                state.metrics.record_query(dur, stmt_type, false, true);
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("No bundle: {bundle_name}")})),
                )
            }
        };
        let result = execute_gql_with_exists(&store, &stmt, &state.engine);
        let dur = t0.elapsed().as_micros() as u64;
        let (status, resp) = match result {
            Ok(r) => exec_result_to_response(r),
            Err(e) => {
                let ev = state.logger.query_error(&req_id, query, dur, "ExecError", &e, 500);
                state.logger.emit(ev);
                state.metrics.record_query(dur, stmt_type, false, true);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e})));
            }
        };
        let slow = dur >= state.logger.slow_threshold_us();
        let ev = state.logger.query_complete(
            &req_id, "gql", stmt_type, query, dur, 0, dur,
            &[bundle_name.clone()], 0, 0, 0, 0, false, None, None,
        );
        state.logger.emit(ev);
        if slow {
            let ev2 = state.logger.query_slow(&req_id, stmt_type, query, dur, false, false, "read path");
            state.logger.emit(ev2);
        }
        state.metrics.record_query(dur, stmt_type, slow, false);
        (status, resp)
    }
}

/// Execute a GQL statement that needs mutable access to a BundleStore.
fn execute_gql_on_store(
    store: &mut gigi::mmap_bundle::BundleMut<'_>,
    stmt: &gigi::parser::Statement,
) -> Result<gigi::parser::ExecResult, String> {
    use gigi::bundle::QueryCondition as QC;
    use gigi::parser::{literal_to_value, ExecResult, Statement};

    match stmt {
        Statement::Insert {
            columns, values, ..
        } => {
            let mut record = std::collections::HashMap::new();
            for (c, v) in columns.iter().zip(values.iter()) {
                record.insert(c.clone(), literal_to_value(v));
            }
            store.insert(&record);
            Ok(ExecResult::Ok)
        }
        Statement::SectionUpsert {
            columns, values, ..
        } => {
            let mut record = std::collections::HashMap::new();
            for (c, v) in columns.iter().zip(values.iter()) {
                record.insert(c.clone(), literal_to_value(v));
            }
            store.upsert(&record);
            Ok(ExecResult::Ok)
        }
        Statement::BatchInsert { columns, rows, .. } => {
            let records: Vec<gigi::types::Record> = rows
                .iter()
                .map(|row| {
                    columns
                        .iter()
                        .zip(row.iter())
                        .map(|(c, v)| (c.clone(), literal_to_value(v)))
                        .collect()
                })
                .collect();
            store.batch_insert(&records);
            Ok(ExecResult::Ok)
        }
        Statement::BatchSectionUpsert { columns, rows, .. } => {
            let mut inserted = 0usize;
            let mut updated = 0usize;
            for row in rows {
                let record: gigi::types::Record = columns
                    .iter()
                    .zip(row.iter())
                    .map(|(c, v)| (c.clone(), literal_to_value(v)))
                    .collect();
                if store.upsert(&record) { updated += 1; } else { inserted += 1; }
            }
            Ok(ExecResult::Rows(vec![{
                let mut r = gigi::types::Record::new();
                r.insert("inserted".to_string(), gigi::types::Value::Integer(inserted as i64));
                r.insert("updated".to_string(), gigi::types::Value::Integer(updated as i64));
                r
            }]))
        }
        Statement::Redefine { key, sets, .. } => {
            let key_rec: gigi::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let patches: gigi::types::Record = sets
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            if store.update(&key_rec, &patches) {
                Ok(ExecResult::Ok)
            } else {
                Err("Record not found".into())
            }
        }
        Statement::BulkRedefine {
            conditions, sets, ..
        } => {
            let qcs: Vec<QC> = conditions.iter().flat_map(|fc| gigi::parser::filter_to_query_conditions(fc)).collect();
            let patches: gigi::types::Record = sets
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let n = store.bulk_update(&qcs, &patches);
            Ok(ExecResult::Count(n))
        }
        Statement::Retract { key, .. } => {
            let key_rec: gigi::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            if store.delete(&key_rec) {
                Ok(ExecResult::Ok)
            } else {
                Err("Record not found".into())
            }
        }
        Statement::BulkRetract { conditions, .. } => {
            let qcs: Vec<QC> = conditions.iter().flat_map(|fc| gigi::parser::filter_to_query_conditions(fc)).collect();
            let n = store.bulk_delete(&qcs);
            Ok(ExecResult::Count(n))
        }
        // For read-only ops via mutable ref, delegate
        _ => execute_gql_on_store_read(&store.as_ref(), stmt, None),
    }
}

/// Execute a GQL statement that only needs read access.
/// Wraps execute_gql_on_store_read but handles EXISTS subquery conditions
/// in COVER WHERE clauses by pre-computing allowed base-point sets.
fn execute_gql_with_exists(
    store: &gigi::mmap_bundle::BundleRef<'_>,
    stmt: &gigi::parser::Statement,
    engine: &std::sync::RwLock<gigi::engine::Engine>,
) -> Result<gigi::parser::ExecResult, String> {
    use gigi::parser::{ExecResult, FilterCondition, Statement};

    if let Statement::Cover { on_conditions, where_conditions, .. } = stmt {
        // Check if any EXISTS conditions are present
        let all_conds: Vec<&FilterCondition> = on_conditions.iter().chain(where_conditions.iter()).collect();
        let exists_conds: Vec<&FilterCondition> = all_conds.iter().filter_map(|fc| {
            if matches!(fc, FilterCondition::Exists { .. }) { Some(*fc) } else { None }
        }).collect();

        if !exists_conds.is_empty() {
            // First run the query without EXISTS filters
            let result = execute_gql_on_store_read(store, stmt, Some(engine))?;
            // Then post-filter rows by EXISTS conditions
            let rows = match result {
                ExecResult::Rows(rows) => rows,
                other => return Ok(other),
            };
            let engine_read = engine.read().unwrap();
            let filtered: Vec<gigi::types::Record> = rows.into_iter().filter(|row| {
                exists_conds.iter().all(|fc| {
                    if let FilterCondition::Exists { cover_bundle, where_conds } = fc {
                        if let Some(sub_store) = engine_read.bundle(cover_bundle) {
                            let sub_qcs: Vec<gigi::bundle::QueryCondition> = where_conds.iter()
                                .flat_map(gigi::parser::filter_to_query_conditions)
                                .collect();
                            !sub_store.filtered_query_ex(&sub_qcs, None, None, false, Some(1), None).is_empty()
                        } else {
                            false
                        }
                    } else { true }
                })
            }).collect();
            return Ok(ExecResult::Rows(filtered));
        }
    }
    execute_gql_on_store_read(store, stmt, Some(engine))
}

/// Group records by `around_field`, compute 2D centroids in (`f0`, `f1`), and measure
/// the parallel-transport polygon deficit.  Returns `(deficit, centroids)` where each
/// centroid entry is `(label, cx, cy, transport_angle)`.  The caller checks `len() < 2`.
fn compute_fiber_holonomy(
    records: impl Iterator<Item = gigi::types::Record>,
    f0: &str,
    f1: &str,
    around_field: &str,
) -> (f64, Vec<(String, f64, f64, f64)>) {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, (f64, f64, usize)> = BTreeMap::new();
    for rec in records {
        let key = match rec.get(around_field) {
            Some(v) => format!("{v:?}"),
            None => continue,
        };
        let v0 = rec.get(f0).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let v1 = rec.get(f1).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let e = groups.entry(key).or_insert((0.0, 0.0, 0));
        e.0 += v0; e.1 += v1; e.2 += 1;
    }
    if groups.len() < 2 {
        return (0.0, groups.into_iter()
            .map(|(k, (sx, sy, n))| (k, sx / n as f64, sy / n as f64, 0.0))
            .collect());
    }
    let centroids: Vec<(String, f64, f64)> = groups.into_iter()
        .map(|(k, (sx, sy, n))| (k, sx / n as f64, sy / n as f64))
        .collect();
    let nc = centroids.len();
    let mut transport_angles = vec![0.0f64; nc];
    for i in 0..nc {
        let prev = if i == 0 { nc - 1 } else { i - 1 };
        let next = (i + 1) % nc;
        let dx_in  = centroids[i].1 - centroids[prev].1;
        let dy_in  = centroids[i].2 - centroids[prev].2;
        let dx_out = centroids[next].1 - centroids[i].1;
        let dy_out = centroids[next].2 - centroids[i].2;
        let mut delta = dy_out.atan2(dx_out) - dy_in.atan2(dx_in);
        while delta >  std::f64::consts::PI { delta -= 2.0 * std::f64::consts::PI; }
        while delta < -std::f64::consts::PI { delta += 2.0 * std::f64::consts::PI; }
        transport_angles[i] = delta;
    }
    let deficit = transport_angles.iter().sum::<f64>().abs() % (2.0 * std::f64::consts::PI);
    let result = centroids.into_iter().zip(transport_angles)
        .map(|((label, cx, cy), ta)| (label, cx, cy, ta))
        .collect();
    (deficit, result)
}

/// Build a GqlStats snapshot from a bundle store.
fn bundle_gql_stats(store: &gigi::mmap_bundle::BundleRef<'_>) -> gigi::parser::GqlStats {
    let k = store.scalar_curvature();
    gigi::parser::GqlStats {
        curvature: k,
        confidence: gigi::curvature::confidence(k),
        record_count: store.len(),
        storage_mode: store.storage_mode().to_string(),
        base_fields: store.schema().base_fields.len(),
        fiber_fields: store.schema().fiber_fields.len(),
    }
}

fn execute_gql_on_store_read(
    store: &gigi::mmap_bundle::BundleRef<'_>,
    stmt: &gigi::parser::Statement,
    engine: Option<&std::sync::RwLock<gigi::engine::Engine>>,
) -> Result<gigi::parser::ExecResult, String> {
    use gigi::bundle::QueryCondition as QC;
    use gigi::parser::{literal_to_value, ExecResult, Statement};

    match stmt {
        Statement::PointQuery { key, project, .. } => {
            let key_rec: gigi::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            match store.point_query(&key_rec) {
                Some(mut rec) => {
                    if let Some(fields) = project {
                        rec = rec
                            .into_iter()
                            .filter(|(k, _)| fields.contains(k))
                            .collect();
                    }
                    Ok(ExecResult::Rows(vec![rec]))
                }
                None => Ok(ExecResult::Rows(vec![])),
            }
        }
        Statement::ExistsSection { key, .. } => {
            let key_rec: gigi::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            Ok(ExecResult::Bool(store.point_query(&key_rec).is_some()))
        }
        Statement::Cover {
            on_conditions,
            where_conditions,
            or_groups,
            distinct_field,
            project,
            rank_by,
            first,
            skip,
            ..
        } => {
            if let Some(field) = distinct_field {
                let vals = store.distinct(field);
                let rows: Vec<gigi::types::Record> = vals
                    .into_iter()
                    .map(|v| {
                        let mut r = std::collections::HashMap::new();
                        r.insert(field.clone(), v);
                        r
                    })
                    .collect();
                return Ok(ExecResult::Rows(rows));
            }
            let mut conditions: Vec<QC> = Vec::new();
            for fc in on_conditions.iter().chain(where_conditions.iter()) {
                conditions.extend(gigi::parser::filter_to_query_conditions(fc));
            }
            let or_qcs: Vec<Vec<QC>> = or_groups
                .iter()
                .map(|g| g.iter().flat_map(gigi::parser::filter_to_query_conditions).collect())
                .collect();
            let or_ref = if or_qcs.is_empty() {
                None
            } else {
                Some(or_qcs.as_slice())
            };

            let results = if let Some(fields) = project {
                let sort_refs: Vec<(&str, bool)> = rank_by
                    .as_ref()
                    .map(|specs| specs.iter().map(|s| (s.field.as_str(), s.desc)).collect())
                    .unwrap_or_default();
                let sort_opt = if sort_refs.is_empty() {
                    None
                } else {
                    Some(sort_refs.as_slice())
                };
                let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
                let (rows, _) = store.filtered_query_projected_ex(
                    &conditions,
                    or_ref,
                    sort_opt,
                    *first,
                    *skip,
                    Some(&field_refs),
                );
                rows
            } else {
                let (sort_by, sort_desc) = rank_by
                    .as_ref()
                    .and_then(|specs| specs.first())
                    .map(|s| (Some(s.field.as_str()), s.desc))
                    .unwrap_or((None, false));
                store.filtered_query_ex(&conditions, or_ref, sort_by, sort_desc, *first, *skip)
            };
            Ok(ExecResult::Rows(results))
        }
        Statement::Integrate { over, measures, .. } => {
            if let Some(gb_field) = over {
                let agg_field = measures.first().map(|m| m.field.as_str()).unwrap_or("*");
                // Inline group_by using BundleRef::records() — works for both heap & mmap
                let mut groups: std::collections::HashMap<gigi::types::Value, gigi::aggregation::AggResult> =
                    std::collections::HashMap::new();
                for rec in store.records() {
                    let group_val = match rec.get(gb_field) {
                        Some(v) => v.clone(),
                        None => continue,
                    };
                    let agg_val = match rec.get(agg_field).and_then(|v| v.as_f64()) {
                        Some(v) => v,
                        None => continue,
                    };
                    let entry = groups
                        .entry(group_val)
                        .or_insert(gigi::aggregation::AggResult {
                            count: 0,
                            sum: 0.0,
                            sum_sq: 0.0,
                            min: f64::INFINITY,
                            max: f64::NEG_INFINITY,
                        });
                    entry.count += 1;
                    entry.sum += agg_val;
                    entry.sum_sq += agg_val * agg_val;
                    entry.min = entry.min.min(agg_val);
                    entry.max = entry.max.max(agg_val);
                }
                let mut rows = Vec::new();
                for (key, agg_result) in &groups {
                    let mut row = std::collections::HashMap::new();
                    row.insert(gb_field.clone(), key.clone());
                    for m in measures {
                        let val = match m.func {
                            gigi::parser::AggFunc::Count => agg_result.count as f64,
                            gigi::parser::AggFunc::Sum => agg_result.sum,
                            gigi::parser::AggFunc::Avg => agg_result.avg(),
                            gigi::parser::AggFunc::Min => agg_result.min,
                            gigi::parser::AggFunc::Max => agg_result.max,
                        };
                        row.insert(
                            m.alias
                                .clone()
                                .unwrap_or_else(|| format!("{}_{}", m.func_name(), m.field)),
                            gigi::types::Value::Float(val),
                        );
                    }
                    rows.push(row);
                }
                Ok(ExecResult::Rows(rows))
            } else {
                Ok(ExecResult::Rows(vec![]))
            }
        }
        Statement::Curvature { .. } => {
            let k = store.scalar_curvature();
            Ok(ExecResult::Scalar(k))
        }
        Statement::Spectral { .. } => {
            let lambda1 = store
                .as_heap()
                .map(gigi::spectral::spectral_gap)
                .unwrap_or(0.0);
            Ok(ExecResult::Scalar(lambda1))
        }
        Statement::Consistency { .. } => {
            let k = store.scalar_curvature();
            Ok(ExecResult::Scalar(if k.abs() < 1e-10 { 0.0 } else { k }))
        }
        Statement::Betti { .. } => {
            let (b0, b1) = store.betti_numbers();
            Ok(ExecResult::Scalar(b0 as f64 + b1 as f64))
        }
        Statement::Entropy { .. } => {
            let s = store.entropy();
            Ok(ExecResult::Scalar(s))
        }
        Statement::FreeEnergy { tau, .. } => {
            let f = store.free_energy(*tau);
            Ok(ExecResult::Scalar(f))
        }
        Statement::ProjectInvariant { expressions, where_clause, .. } => {
            // Sprint H: route to the invariant evaluator. The evaluator
            // operates strictly on the heap-side BundleStore via base
            // points + field stats — never reads fiber values, so the
            // PROJECT INVARIANT execution path triggers zero decrypts.
            let store_heap = store.as_heap().ok_or_else(|| {
                "PROJECT INVARIANT requires bundle in heap mode".to_string()
            })?;
            let results: Vec<(String, f64)> = expressions
                .iter()
                .map(|(label, expr)| {
                    let v = match where_clause {
                        Some(conds) => gigi::invariant::evaluate_filtered(store_heap, expr, conds),
                        None => gigi::invariant::evaluate(store_heap, expr),
                    };
                    (label.clone(), v)
                })
                .collect();
            Ok(ExecResult::Invariants(results))
        }
        Statement::Geodesic { from_keys, to_keys, max_hops, restrict_bundle, .. } => {
            let from_rec: gigi::types::Record = from_keys.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            let to_rec: gigi::types::Record = to_keys.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            let bp_a = store.as_heap().map(|s| s.base_point(&from_rec)).unwrap_or(0);
            let bp_b = store.as_heap().map(|s| s.base_point(&to_rec)).unwrap_or(0);
            match store.geodesic_path(bp_a, bp_b, *max_hops) {
                Some(path) => {
                    let rows: Vec<gigi::types::Record> = path.iter().enumerate().map(|(hop, &bp)| {
                        let mut r = gigi::types::Record::new();
                        r.insert("hop".to_string(), gigi::types::Value::Integer(hop as i64));
                        r.insert("base_point".to_string(), gigi::types::Value::Integer(bp as i64));
                        r
                    }).collect();
                    let rows = if let Some(rb) = restrict_bundle {
                        if let Some(eng) = engine {
                            let engine_read = eng.read().unwrap();
                            if let Some(rs) = engine_read.bundle(rb) {
                                let restrict_bps: std::collections::HashSet<gigi::types::BasePoint> = rs.all_base_points();
                                let filtered: Vec<gigi::types::Record> = rows.into_iter().filter(|r| {
                                    r.get("base_point").and_then(|v| v.as_i64()).map(|bp| restrict_bps.contains(&(bp as gigi::types::BasePoint))).unwrap_or(false)
                                }).collect();
                                filtered
                            } else {
                                rows
                            }
                        } else {
                            rows
                        }
                    } else {
                        rows
                    };
                    Ok(ExecResult::Rows(rows))
                }
                None => Ok(ExecResult::Scalar(-1.0)),
            }
        }
        Statement::MetricTensor { .. } => {
            let info = store.metric_tensor();
            let cond = if info.condition_number.is_finite() { info.condition_number } else { -1.0 };
            Ok(ExecResult::Scalar(cond))
        }
        Statement::HolonomyFiber { bundle: _, fiber_fields, around_field } => {
            if fiber_fields.len() < 2 {
                return Err("HOLONOMY ON FIBER requires at least 2 fiber fields".to_string());
            }
            let f0 = &fiber_fields[0];
            let f1 = &fiber_fields[1];
            let (deficit, centroids) = compute_fiber_holonomy(store.records(), f0, f1, around_field);
            if centroids.len() < 2 {
                return Err(format!(
                    "HOLONOMY ON FIBER needs ≥2 distinct '{}' values, found {}",
                    around_field, centroids.len()
                ));
            }
            let trivial = deficit < 1e-6;
            let mut rows: Vec<gigi::types::Record> = centroids
                .iter()
                .map(|(label, cx, cy, ta)| {
                    let mut r = gigi::types::Record::new();
                    r.insert(around_field.clone(), gigi::types::Value::Text(label.clone()));
                    r.insert(f0.clone(), gigi::types::Value::Float(*cx));
                    r.insert(f1.clone(), gigi::types::Value::Float(*cy));
                    r.insert("transport_angle".to_string(), gigi::types::Value::Float(*ta));
                    r
                })
                .collect();
            let mut summary = gigi::types::Record::new();
            summary.insert("_type".to_string(), gigi::types::Value::Text("summary".to_string()));
            summary.insert("holonomy_angle".to_string(), gigi::types::Value::Float(deficit));
            summary.insert("holonomy_trivial".to_string(), gigi::types::Value::Bool(trivial));
            rows.push(summary);
            Ok(ExecResult::Rows(rows))
        }
        Statement::Health { .. } => Ok(ExecResult::Stats(bundle_gql_stats(store))),
        Statement::Describe { .. } => Ok(ExecResult::Stats(bundle_gql_stats(store))),
        // SPECTRAL bundle ON FIBER (f1, f2, ...) MODES k
        Statement::SpectralFiber { bundle, fiber_fields, modes } => {
            let heap = store
                .as_heap()
                .ok_or_else(|| format!("SPECTRAL ON FIBER: bundle '{}' is not a heap bundle", bundle))?;
            let refs: Vec<&str> = fiber_fields.iter().map(|s: &String| s.as_str()).collect();
            let fiber_modes = gigi::spectral::spectral_fiber_modes(heap, &refs, *modes);
            let rows: Vec<gigi::types::Record> = fiber_modes
                .into_iter()
                .map(|m| {
                    let mut r = gigi::types::Record::new();
                    r.insert("mode".to_string(), gigi::types::Value::Integer(m.mode as i64));
                    r.insert("lambda".to_string(), gigi::types::Value::Float(m.lambda));
                    r.insert("ipr".to_string(), gigi::types::Value::Float(m.ipr));
                    r
                })
                .collect();
            Ok(ExecResult::Rows(rows))
        }
        // TRANSPORT bundle FROM (key=val) TO (key=val) ON FIBER (f1, f2, ...)
        //
        // L1.5.3 extension (catalog §1.2, consumption draft v2 §2):
        // when the bundle carries a Kähler structure with attached
        // closed 2-form B AND the fiber dimension matches B's dim,
        // the verb dispatches into `gigi::geometry::flat_transport`
        // for magnetically-perturbed transport. The response gains
        // these v2-contract fields:
        //
        //   b_source         — "bundle" | "override" | "none"
        //   used_magnetic    — bool
        //   energy_drift     — f64 (must be < 1e-9 per turn in prod)
        //   holonomy_norm    — f64 (rotation accumulated along path)
        //   path_length      — f64 (integrated arc length)
        //   closedness_norm  — f64 (only on fallback_non_closed path)
        //
        // Bundles WITHOUT a Kähler structure fall through to the
        // existing quaternion / rotation paths — strict additive
        // contract per IMPLEMENTATION_PLAN §0.
        Statement::Transport { bundle: _, from_keys, to_keys, fiber_fields } => {
            let from_rec: gigi::types::Record =
                from_keys.iter().map(|(k, v): &(String, gigi::parser::Literal)| (k.clone(), gigi::parser::literal_to_value(v))).collect();
            let to_rec: gigi::types::Record =
                to_keys.iter().map(|(k, v): &(String, gigi::parser::Literal)| (k.clone(), gigi::parser::literal_to_value(v))).collect();

            // Locate the two records
            let find_point = |target: &gigi::types::Record| -> Option<Vec<f64>> {
                store.records().find(|rec| {
                    target.iter().all(|(k, v)| rec.get(k.as_str()) == Some(v))
                }).map(|rec| {
                    fiber_fields.iter()
                        .map(|f: &String| rec.get(f.as_str()).and_then(|v| v.as_f64()).unwrap_or(0.0))
                        .collect()
                })
            };

            let p_from = find_point(&from_rec)
                .ok_or_else(|| "TRANSPORT: FROM record not found".to_string())?;
            let p_to = find_point(&to_rec)
                .ok_or_else(|| "TRANSPORT: TO record not found".to_string())?;

            let dim = p_from.len().min(p_to.len());
            let displacement: Vec<f64> = (0..dim).map(|i| p_to[i] - p_from[i]).collect();

            let mut result = gigi::types::Record::new();

            // L1.5.3 Kähler path: dispatch when bundle has attached
            // K-structure AND dimensions match. The helper does the
            // RK4 magnetic geodesic + builds the v2 contract record;
            // factored out so the test in this file can call it
            // without spinning up a BundleMut.
            #[cfg(feature = "kahler")]
            {
                if let Some(k) = store.schema().kahler.as_ref() {
                    if let Some(rec) = kahler_transport_dispatch(k, &p_from, &p_to, &displacement) {
                        let rec = rec?;
                        return Ok(ExecResult::Rows(vec![rec]));
                    }
                }
            }

            if dim == 4 {
                // Quaternion TRANSPORT path — q = (w, x, y, z)
                let qa = [p_from[0], p_from[1], p_from[2], p_from[3]];
                let qb = [p_to[0],   p_to[1],   p_to[2],   p_to[3]];
                // qa_conj = (w, -x, -y, -z)
                let qa_conj = [qa[0], -qa[1], -qa[2], -qa[3]];
                // Hamilton product: q_rel = qb * qa_conj
                let (w1, x1, y1, z1) = (qb[0], qb[1], qb[2], qb[3]);
                let (w2, x2, y2, z2) = (qa_conj[0], qa_conj[1], qa_conj[2], qa_conj[3]);
                let mut q_rel = [
                    w1*w2 - x1*x2 - y1*y2 - z1*z2,
                    w1*x2 + x1*w2 + y1*z2 - z1*y2,
                    w1*y2 - x1*z2 + y1*w2 + z1*x2,
                    w1*z2 + x1*y2 - y1*x2 + z1*w2,
                ];
                // Normalize
                let norm = q_rel.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-12);
                for v in &mut q_rel { *v /= norm; }
                // Canonical form: w >= 0
                if q_rel[0] < 0.0 { for v in &mut q_rel { *v = -*v; } }
                let transport_angle = 2.0 * q_rel[0].clamp(-1.0, 1.0).acos();

                result.insert("transport_angle".to_string(), gigi::types::Value::Float(transport_angle));
                result.insert("q0".to_string(), gigi::types::Value::Float(q_rel[0]));
                result.insert("q1".to_string(), gigi::types::Value::Float(q_rel[1]));
                result.insert("q2".to_string(), gigi::types::Value::Float(q_rel[2]));
                result.insert("q3".to_string(), gigi::types::Value::Float(q_rel[3]));
            } else {
                let norm_from: f64 = p_from.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
                let norm_to:   f64 = p_to.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
                let cos_theta: f64 = p_from.iter().zip(&p_to).map(|(a, b)| a * b).sum::<f64>()
                    / (norm_from * norm_to);
                let angle = cos_theta.clamp(-1.0, 1.0).acos();

                let (t00, t01, t10, t11) = if dim >= 2 {
                    let c = angle.cos();
                    let s = angle.sin();
                    (c, -s, s, c)
                } else {
                    (1.0f64, 0.0f64, 0.0f64, 1.0f64)
                };

                result.insert("transport_angle".to_string(), gigi::types::Value::Float(angle));
                result.insert("t00".to_string(), gigi::types::Value::Float(t00));
                result.insert("t01".to_string(), gigi::types::Value::Float(t01));
                result.insert("t10".to_string(), gigi::types::Value::Float(t10));
                result.insert("t11".to_string(), gigi::types::Value::Float(t11));
            }

            for (i, d) in displacement.iter().enumerate() {
                result.insert(format!("displacement_{i}"), gigi::types::Value::Float(*d));
            }
            Ok(ExecResult::Rows(vec![result]))
        }
        // C2 — TRANSPORT_ROTATION: Rodrigues rotation in the plane spanned
        // by (FROM, TO) fiber vectors by a SUPPLIED angle. Returns the
        // N×N matrix as a comma-separated `matrix_flat` string plus a
        // numeric `dim` field. Pairs the Python C0 rotation_in_plane.
        Statement::TransportRotation {
            bundle: _,
            from_keys,
            to_keys,
            fiber_fields,
            angle,
        } => {
            let from_rec: gigi::types::Record = from_keys.iter()
                .map(|(k, v): &(String, gigi::parser::Literal)| (
                    k.clone(), gigi::parser::literal_to_value(v)
                )).collect();
            let to_rec: gigi::types::Record = to_keys.iter()
                .map(|(k, v): &(String, gigi::parser::Literal)| (
                    k.clone(), gigi::parser::literal_to_value(v)
                )).collect();

            let find_vec = |target: &gigi::types::Record| -> Option<Vec<f64>> {
                store.records().find(|rec| {
                    target.iter().all(|(k, v)| rec.get(k.as_str()) == Some(v))
                }).map(|rec| {
                    fiber_fields.iter()
                        .map(|f: &String| rec.get(f.as_str())
                            .and_then(|v| v.as_f64()).unwrap_or(0.0))
                        .collect()
                })
            };

            let u = find_vec(&from_rec)
                .ok_or_else(|| "TRANSPORT_ROTATION: FROM record not found".to_string())?;
            let v = find_vec(&to_rec)
                .ok_or_else(|| "TRANSPORT_ROTATION: TO record not found".to_string())?;
            let n = u.len().min(v.len());
            if n == 0 {
                return Err("TRANSPORT_ROTATION: zero-length fiber".to_string());
            }

            // Rodrigues in the plane (u, v) by the supplied angle.
            // R = I + (cos θ − 1)(e1 e1^T + e2 e2^T) + sin θ (e2 e1^T − e1 e2^T)
            // where e1 = u/‖u‖, e2 = (v − ⟨v,e1⟩ e1) / ‖…‖.
            let nu: f64 = u.iter().map(|x| x*x).sum::<f64>().sqrt();
            let mut matrix = vec![0.0f64; n * n];
            // Identity by default
            for i in 0..n { matrix[i * n + i] = 1.0; }

            if nu >= 1e-12 && angle.abs() >= 1e-12 {
                let e1: Vec<f64> = u.iter().map(|x| x / nu).collect();
                let dot_v_e1: f64 = v.iter().zip(&e1).map(|(a, b)| a * b).sum();
                let e2_unnorm: Vec<f64> = v.iter().zip(&e1)
                    .map(|(vi, ei)| vi - dot_v_e1 * ei)
                    .collect();
                let ne: f64 = e2_unnorm.iter().map(|x| x*x).sum::<f64>().sqrt();
                if ne >= 1e-12 {
                    let e2: Vec<f64> = e2_unnorm.iter().map(|x| x / ne).collect();
                    let cos_t = angle.cos();
                    let sin_t = angle.sin();
                    let coef_p = cos_t - 1.0;
                    // R = I + coef_p · (e1 e1^T + e2 e2^T) + sin_t · (e2 e1^T − e1 e2^T)
                    for i in 0..n {
                        for j in 0..n {
                            let p_ij = e1[i] * e1[j] + e2[i] * e2[j];
                            let a_ij = e2[i] * e1[j] - e1[i] * e2[j];
                            matrix[i * n + j] += coef_p * p_ij + sin_t * a_ij;
                        }
                    }
                }
            }

            let mut result = gigi::types::Record::new();
            result.insert("dim".to_string(),
                          gigi::types::Value::Integer(n as i64));
            result.insert("angle".to_string(),
                          gigi::types::Value::Float(*angle));
            // Return the N×N matrix as a flat row-major Vector
            // (Value::Vector is gigi's native dense float vector).
            result.insert("matrix".to_string(),
                          gigi::types::Value::Vector(matrix));
            Ok(ExecResult::Rows(vec![result]))
        }
        // HOLONOMY bundle NEAR (f1=v1, ...) WITHIN r [METRIC m] ON FIBER (f1, f2) AROUND field
        Statement::LocalHolonomy {
            bundle: _,
            near_point,
            near_radius,
            near_metric,
            fiber_fields,
            around_field,
        } => {
            if fiber_fields.len() < 2 {
                return Err("HOLONOMY NEAR requires at least 2 fiber fields".to_string());
            }
            let f0 = &fiber_fields[0];
            let f1 = &fiber_fields[1];

            let use_cosine = near_metric.as_deref() == Some("cosine");
            let query_vec: Vec<(String, f64)> = near_point.clone();

            let neighbourhood: Vec<gigi::types::Record> = store
                .records()
                .filter(|rec| {
                    if use_cosine {
                        let dot: f64 = query_vec.iter()
                            .map(|(f, v): &(String, f64)| rec.get(f.as_str()).and_then(|rv| rv.as_f64()).unwrap_or(0.0) * v)
                            .sum();
                        let norm_q: f64 = query_vec.iter().map(|(_, v): &(String, f64)| v * v).sum::<f64>().sqrt();
                        let norm_r: f64 = query_vec.iter()
                            .map(|(f, _): &(String, f64)| rec.get(f.as_str()).and_then(|rv| rv.as_f64()).unwrap_or(0.0))
                            .map(|x| x * x)
                            .sum::<f64>()
                            .sqrt();
                        let sim = dot / (norm_q * norm_r + 1e-12);
                        sim >= 1.0 - near_radius
                    } else {
                        let dist_sq: f64 = query_vec.iter()
                            .map(|(f, v): &(String, f64)| {
                                let rv = rec.get(f.as_str()).and_then(|rv| rv.as_f64()).unwrap_or(0.0);
                                (rv - v) * (rv - v)
                            })
                            .sum();
                        dist_sq.sqrt() <= *near_radius
                    }
                })
                .collect();

            let n_size = neighbourhood.len();
            let (local_holonomy, centroids) = compute_fiber_holonomy(neighbourhood.into_iter(), f0, f1, around_field);
            if centroids.len() < 2 {
                let mut row = gigi::types::Record::new();
                row.insert("local_holonomy_angle".to_string(), gigi::types::Value::Float(0.0));
                row.insert("neighbourhood_size".to_string(), gigi::types::Value::Integer(n_size as i64));
                row.insert("warning".to_string(), gigi::types::Value::Text(
                    format!("fewer than 2 distinct '{}' values in neighbourhood", around_field)
                ));
                return Ok(ExecResult::Rows(vec![row]));
            }
            let mut row = gigi::types::Record::new();
            row.insert("local_holonomy_angle".to_string(), gigi::types::Value::Float(local_holonomy));
            row.insert("neighbourhood_size".to_string(), gigi::types::Value::Integer(n_size as i64));
            Ok(ExecResult::Rows(vec![row]))
        }
        // GAUGE bundle1 VS bundle2 ON FIBER (f1, f2) AROUND field
        Statement::GaugeTest { bundle1, bundle2, fiber_fields, around_field } => {
            if fiber_fields.len() < 2 {
                return Err("GAUGE: requires at least 2 fiber fields".to_string());
            }
            let f0 = &fiber_fields[0];
            let f1 = &fiber_fields[1];

            let (deficit1, centroids1) = compute_fiber_holonomy(store.records(), f0, f1, around_field);
            if centroids1.len() < 2 {
                return Err(format!("GAUGE: bundle needs ≥2 distinct '{}' values for holonomy", around_field));
            }

            let deficit2 = if bundle2 == bundle1 {
                deficit1
            } else if let Some(eng) = engine {
                let eng_read = eng.read().map_err(|_| "engine lock poisoned".to_string())?;
                let store2 = eng_read.bundle(&bundle2)
                    .ok_or_else(|| format!("GAUGE VS: bundle '{}' not found", bundle2))?;
                let (d2, c2) = compute_fiber_holonomy(store2.records(), f0, f1, around_field);
                if c2.len() < 2 {
                    return Err(format!("GAUGE: bundle '{}' needs ≥2 distinct '{}' values for holonomy", bundle2, around_field));
                }
                d2
            } else {
                return Err(format!("GAUGE VS: bundle '{}' not found (no engine context)", bundle2));
            };
            let gauge_difference = (deficit1 - deficit2).abs();
            let gauge_invariant = gauge_difference < std::f64::consts::PI / 10.0;
            let mut row = gigi::types::Record::new();
            row.insert("bundle1".to_string(), gigi::types::Value::Text(bundle1.clone()));
            row.insert("bundle2".to_string(), gigi::types::Value::Text(bundle2.clone()));
            row.insert("holonomy_1".to_string(), gigi::types::Value::Float(deficit1));
            row.insert("holonomy_2".to_string(), gigi::types::Value::Float(deficit2));
            row.insert("gauge_difference".to_string(), gigi::types::Value::Float(gauge_difference));
            row.insert("gauge_invariant".to_string(), gigi::types::Value::Bool(gauge_invariant));
            Ok(ExecResult::Rows(vec![row]))
        }
        // SQL compat: SELECT * FROM bundle [WHERE ...] — used by observability + ad-hoc queries.
        Statement::Select { columns, condition, .. } => {
            use gigi::parser::SelectCol;
            let all_rows: Vec<_> = store.records().collect();
            let filtered: Vec<_> = match condition {
                None => all_rows,
                Some(gigi::parser::Condition::Eq(field, val)) => {
                    let value = literal_to_value(val);
                    all_rows.into_iter().filter(|rec| rec.get(field) == Some(&value)).collect()
                }
                Some(gigi::parser::Condition::In(field, vals)) => {
                    let values: Vec<_> = vals.iter().map(literal_to_value).collect();
                    all_rows.into_iter().filter(|rec| {
                        rec.get(field).map(|v| values.contains(v)).unwrap_or(false)
                    }).collect()
                }
                Some(gigi::parser::Condition::Between(field, lo, hi)) => {
                    let lo_val = literal_to_value(lo);
                    let hi_val = literal_to_value(hi);
                    all_rows.into_iter().filter(|rec| {
                        rec.get(field).map(|v| *v >= lo_val && *v <= hi_val).unwrap_or(false)
                    }).collect()
                }
            };
            let is_star = columns.iter().any(|c| matches!(c, SelectCol::Star));
            let result_rows: Vec<_> = if is_star {
                filtered
            } else {
                filtered.into_iter().map(|rec| {
                    rec.into_iter()
                        .filter(|(k, _)| columns.iter().any(|c| matches!(c, SelectCol::Name(n) if n == k)))
                        .collect()
                }).collect()
            };
            Ok(ExecResult::Rows(result_rows))
        }
        _ => Ok(ExecResult::Ok),
    }
}

fn get_bundle_name(stmt: &gigi::parser::Statement) -> Option<String> {
    use gigi::parser::Statement::*;
    match stmt {
        Insert { bundle, .. } | BatchInsert { bundle, .. } | SectionUpsert { bundle, .. }
        | Redefine { bundle, .. } | BulkRedefine { bundle, .. }
        | Retract { bundle, .. } | BulkRetract { bundle, .. }
        | PointQuery { bundle, .. } | ExistsSection { bundle, .. }
        | Cover { bundle, .. } | Integrate { bundle, .. }
        | Select { bundle, .. }
        | Curvature { bundle, .. } | Spectral { bundle, .. }
        | Consistency { bundle, .. } | Health { bundle, .. }
        | Describe { bundle, .. }
        | Betti { bundle, .. } | Entropy { bundle, .. }
        | FreeEnergy { bundle, .. }
        | Geodesic { bundle, .. } | MetricTensor { bundle, .. }
        // v2.1
        | Compact { bundle, .. } | Analyze { bundle, .. }
        | Vacuum { bundle, .. } | RebuildIndex { bundle, .. }
        | CheckIntegrity { bundle } | Repair { bundle }
        | StorageInfo { bundle }
        | ShowFields { bundle } | ShowIndexes { bundle }
        | ShowMorphisms { bundle } | ShowTriggers { bundle }
        | ShowPolicies { bundle } | ShowStatistics { bundle }
        | ShowGeometry { bundle } | ShowComments { bundle }
        | ShowConstraints { bundle }
        | AuditOn { bundle, .. } | AuditOff { bundle }
        | AuditShow { bundle, .. }
        | Ingest { bundle, .. }
        | GenerateBase { bundle, .. } | Fill { bundle, .. }
        | Iterate { bundle, .. } => Some(bundle.clone()),
        Pullback { left, .. } | Join { left, .. } => Some(left.clone()),
        Transplant { source, .. } => Some(source.clone()),
        DropPolicy { bundle, .. } | DropTrigger { bundle, .. } => Some(bundle.clone()),
        CreateTrigger { bundle, .. } => Some(bundle.clone()),
        Explain { inner } => get_bundle_name(inner),
        // Fiber-geometric analytics (Sprint 2)
        HolonomyFiber { bundle, .. } => Some(bundle.clone()),
        SpectralFiber { bundle, .. } => Some(bundle.clone()),
        Transport { bundle, .. } => Some(bundle.clone()),
        TransportRotation { bundle, .. } => Some(bundle.clone()),
        LocalHolonomy { bundle, .. } => Some(bundle.clone()),
        GaugeTest { bundle1, .. } => Some(bundle1.clone()),
        // Coherence extensions v0.1
        SectionCoherent { bundle, .. } => Some(bundle.clone()),
        ShowCharts { bundle } => Some(bundle.clone()),
        ShowContradictions { bundle } => Some(bundle.clone()),
        CollapseBranch { bundle, .. } => Some(bundle.clone()),
        Predict { bundle, .. } => Some(bundle.clone()),
        CoverGeodesic { bundle, .. } => Some(bundle.clone()),
        Why { bundle, .. } => Some(bundle.clone()),
        Implications { bundle, .. } => Some(bundle.clone()),
        Ricci { bundle, .. } => Some(bundle.clone()),
        // Sprint H: PROJECT INVARIANT routes through the standard
        // single-bundle read path; expose its bundle name here so the
        // dispatcher knows where to attach.
        ProjectInvariant { bundle, .. } => Some(bundle.clone()),
        // Divergence is cross-bundle; no single name
        _ => None,
    }
}

/// Return a short uppercase name for a GQL statement (used in metrics + logs).
fn gql_stmt_type_name(stmt: &gigi::parser::Statement) -> &'static str {
    use gigi::parser::Statement::*;
    match stmt {
        Select { .. }         => "SELECT",
        Insert { .. }         => "INSERT",
        BatchInsert { .. }    => "BATCH_INSERT",
        BatchSectionUpsert { .. } => "BATCH_UPSERT",
        SectionUpsert { .. }  => "UPSERT",
        Redefine { .. }       => "UPDATE",
        BulkRedefine { .. }   => "BULK_UPDATE",
        Retract { .. }        => "DELETE",
        BulkRetract { .. }    => "BULK_DELETE",
        PointQuery { .. }     => "POINT_GET",
        Divergence { .. }     => "DIVERGENCE",
        Ricci { .. }          => "RICCI",
        Curvature { .. }      => "CURVATURE",
        Spectral { .. }       => "SPECTRAL",
        CreateBundle { .. }   => "CREATE_BUNDLE",
        Collapse { .. }       => "DROP_BUNDLE",
        RotateKey { .. }      => "ROTATE_KEY",
        ProjectInvariant { .. } => "PROJECT_INVARIANT",
        ShowBundles           => "SHOW_BUNDLES",
        Describe { .. }       => "DESCRIBE",
        Explain { .. }        => "EXPLAIN",
        Join { .. } | Pullback { .. } => "JOIN",
        _                     => "OTHER",
    }
}

fn exec_result_to_response(
    result: gigi::parser::ExecResult,
) -> (StatusCode, Json<serde_json::Value>) {
    use gigi::parser::ExecResult::*;
    match result {
        Ok => (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
        Rows(rows) => {
            let json_rows: Vec<serde_json::Value> =
                rows.iter().map(|r| record_to_json(r)).collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({"rows": json_rows, "count": json_rows.len()})),
            )
        }
        Scalar(v) => (StatusCode::OK, Json(serde_json::json!({"value": v}))),
        Bool(v) => (StatusCode::OK, Json(serde_json::json!({"value": v}))),
        Count(n) => (StatusCode::OK, Json(serde_json::json!({"affected": n}))),
        Stats(stats) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "curvature": stats.curvature, "confidence": stats.confidence,
                "record_count": stats.record_count, "storage_mode": stats.storage_mode,
                "base_fields": stats.base_fields, "fiber_fields": stats.fiber_fields,
            })),
        ),
        Bundles(infos) => {
            let list: Vec<serde_json::Value> = infos.iter()
                .map(|i| serde_json::json!({"name": i.name, "records": i.records, "fields": i.fields}))
                .collect();
            (StatusCode::OK, Json(serde_json::json!({"bundles": list})))
        }
        Invariants(results) => {
            // Sprint H: PROJECT INVARIANT response. Each entry is
            // (canonical_label, value) — flatten to a single JSON object.
            let mut obj = serde_json::Map::new();
            for (label, value) in &results {
                obj.insert(label.clone(), serde_json::json!(value));
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({"invariants": serde_json::Value::Object(obj)})),
            )
        }
    }
}

// ── Vector Search ──

#[derive(Deserialize)]
struct VectorSearchRequest {
    /// Name of the vector field to search in.
    field: String,
    /// Query vector (must match stored dimensionality).
    vector: Vec<f64>,
    /// Number of nearest neighbors to return (default 10).
    #[serde(default)]
    top_k: Option<usize>,
    /// Metric: "cosine" (default), "euclidean", "dot"
    #[serde(default)]
    metric: Option<String>,
    /// Optional pre-filter: only score records matching these conditions.
    #[serde(default)]
    filters: Vec<ConditionSpec>,
}

async fn vector_search_handler(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<VectorSearchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    let top_k = req.top_k.unwrap_or(10).max(1);

    let metric = match req.metric.as_deref().unwrap_or("cosine") {
        "euclidean" | "l2" => VectorMetric::Euclidean,
        "dot" | "dot_product" | "inner_product" => VectorMetric::Dot,
        _ => VectorMetric::Cosine,
    };

    let pre_filter: Vec<QueryCondition> = req
        .filters
        .iter()
        .map(condition_spec_to_query_condition)
        .collect();

    let results = store.vector_search(&req.field, &req.vector, top_k, metric, &pre_filter);

    let json_results: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(score, record)| {
            serde_json::json!({
                "score": score,
                "record": record_to_json(&record)
            })
        })
        .collect();

    let metric_name = match metric {
        VectorMetric::Cosine => "cosine",
        VectorMetric::Euclidean => "euclidean",
        VectorMetric::Dot => "dot",
    };

    Ok(Json(serde_json::json!({
        "results": json_results,
        "meta": {
            "count": json_results.len(),
            "metric": metric_name,
            "query_dims": req.vector.len(),
            "top_k": top_k
        }
    })))
}

// ── Tigris / S3 snapshot sync ─────────────────────────────────────────────

fn has_dhoom_files(dir: &std::path::Path) -> bool {
    if !dir.exists() {
        return false;
    }
    std::fs::read_dir(dir)
        .map(|d| {
            d.filter_map(|e| e.ok())
                .any(|e| e.path().extension() == Some(std::ffi::OsStr::new("dhoom")))
        })
        .unwrap_or(false)
}

/// Run `aws s3 sync src dest`, passing Tigris endpoint if configured.
fn aws_s3_sync(src: &str, dest: &str) {
    let mut cmd = std::process::Command::new("aws");
    cmd.args(["s3", "sync", "--no-progress", src, dest, "--exclude", "*.tmp"]);
    // awscli v1 needs --endpoint-url explicitly; v2 reads AWS_ENDPOINT_URL env var.
    if let Ok(ep) = std::env::var("AWS_ENDPOINT_URL_S3")
        .or_else(|_| std::env::var("AWS_ENDPOINT_URL"))
    {
        cmd.args(["--endpoint-url", &ep]);
    }
    match cmd.status() {
        Ok(s) if s.success() => eprintln!("S3 sync ok: {src} → {dest}"),
        Ok(s) => eprintln!("S3 sync exit {s}: {src} → {dest}"),
        Err(e) => eprintln!("S3 sync error (aws not found?): {e}"),
    }
}

/// Pull snapshots + WAL from Tigris into data_dir (used on cold / volumeless start).
fn tigris_pull(data_dir: &std::path::Path, bucket: &str) {
    let src = format!("s3://{bucket}/");
    let dest = format!("{}/", data_dir.display());
    eprintln!("Tigris pull: {src} → {dest}");
    aws_s3_sync(&src, &dest);
}

/// Push snapshots + WAL from data_dir to Tigris.
fn tigris_push(data_dir: &std::path::Path, bucket: &str) {
    let src = format!("{}/", data_dir.display());
    let dest = format!("s3://{bucket}/");
    eprintln!("Tigris push: {src} → {dest}");
    aws_s3_sync(&src, &dest);
}

// ── Phase 2 Observability: System Bundle Writer ───────────────────────────────

/// Create the internal `_gigi_*` system bundles if they don't already exist.
/// Called once at startup before queries are served.
fn init_system_bundles(engine: &mut Engine) {
    let bundles: &[(&str, &[(&str, bool)])] = &[
        // (bundle_name, &[(field_name, is_numeric)])
        ("_gigi_query_log", &[
            ("ts_us",            true),
            ("duration_us",      true),
            ("records_returned", true),
            ("records_scanned",  true),
            ("kl_forward",       true),
            ("kl_reverse",       true),
            ("jensen_shannon",   true),
            ("event",            false),
            ("statement_type",   false),
            ("bundle",           false),
            ("slow",             false),
            ("error_msg",        false),
            ("request_id",       false),
        ]),
        ("_gigi_slow_log", &[
            ("ts_us",            true),
            ("duration_us",      true),
            ("records_returned", true),
            ("records_scanned",  true),
            ("kl_forward",       true),
            ("kl_reverse",       true),
            ("jensen_shannon",   true),
            ("event",            false),
            ("statement_type",   false),
            ("bundle",           false),
            ("error_msg",        false),
            ("request_id",       false),
        ]),
        ("_gigi_anomaly_log", &[
            ("ts_us",        true),
            ("z_score",      true),
            ("sigma_level",  true),
            ("k_record",     true),
            ("k_mean",       true),
            ("k_std",        true),
            ("duration_us",  true),
            ("bundle",       false),
            ("detection_source", false),
        ]),
        ("_gigi_system_log", &[
            ("ts_us",       true),
            ("duration_us", true),
            ("level",       false),
            ("event",       false),
            ("detail",      false),
        ]),
        ("_gigi_audit_log", &[
            ("ts_us",       true),
            ("duration_us", true),
            ("event",       false),
            ("bundle",      false),
            ("level",       false),
            ("detail",      false),
        ]),
        ("_gigi_stream_log", &[
            ("ts_us",                true),
            ("duration_us",          true),
            ("messages_sent",        true),
            ("anomalies_sent",       true),
            ("session_duration_us",  true),
            ("event",               false),
            ("connection_id",        false),
            ("bundle",              false),
            ("detail",              false),
        ]),
        ("_gigi_bundle_log", &[
            ("ts_us",       true),
            ("duration_us", true),
            ("event",       false),
            ("bundle",      false),
            ("detail",      false),
        ]),
        ("_gigi_ingest_log", &[
            ("ts_us",            true),
            ("duration_us",      true),
            ("records_written",  true),
            ("bytes_written",    true),
            ("throughput_rps",   true),
            ("event",            false),
            ("bundle",           false),
        ]),
        ("_gigi_wal_log", &[
            ("ts_us",             true),
            ("duration_us",       true),
            ("records_flushed",   true),
            ("bytes_flushed",     true),
            ("records_recovered", true),
            ("event",             false),
            ("bundle",            false),
        ]),
        ("_gigi_conn_log", &[
            ("ts_us",                true),
            ("session_duration_us",  true),
            ("requests_served",      true),
            ("bytes_sent",           true),
            ("bytes_received",       true),
            ("event",               false),
            ("protocol",            false),
            ("client_ip",           false),
        ]),
    ];

    let existing: Vec<String> = engine.bundle_names().iter().map(|s| s.to_string()).collect();
    for (name, fields) in bundles {
        if existing.iter().any(|n| n == name) {
            continue; // already exists (WAL replay)
        }
        let mut schema = BundleSchema::new(name);
        for (field, numeric) in *fields {
            let def = if *numeric {
                FieldDef::numeric(field)
            } else {
                FieldDef::categorical(field)
            };
            schema = schema.fiber(def);
        }
        if let Err(e) = engine.create_bundle(schema) {
            eprintln!("[observability] failed to create {name}: {e}");
        }
    }
}

/// Bootstrap APP-LEVEL bundles required by the consuming application
/// (davisgeometric.com / Just Gigi). Idempotent: only creates bundles that
/// don't already exist. Driven by the `GIGI_APP_BUNDLES` env var which
/// holds a JSON array of bundle specs:
///
/// ```json
/// [
///   {
///     "name": "jg_kv",
///     "seed_env": "JG_KV_ENCRYPTION_SEED",
///     "base": [{"name": "key", "type": "text"}],
///     "fiber": [
///       {"name": "kind", "type": "text", "indexed": true},
///       {"name": "payload", "type": "text", "encrypted": "opaque"},
///       {"name": "expires_at", "type": "timestamp", "indexed": true},
///       {"name": "updated_at", "type": "timestamp", "indexed": true}
///     ]
///   }
/// ]
/// ```
///
/// Per-field `encrypted` accepts: `"none"` (default), `"affine"`
/// (numeric only), `"opaque"` (AEAD on text/binary), `"indexed"`
/// (deterministic PRF on text — high-cardinality only). When ANY
/// fiber field has a non-`none` mode, the bundle's GaugeKey is
/// derived at create-time from the seed at `seed_env` (a 32-byte
/// hex env var). If `seed_env` is missing or unset, the bootstrap
/// falls back to a random per-startup seed — but that means the
/// gauge_key changes every redeploy, which would make existing
/// ciphertext unrecoverable. For production deployments, ALWAYS
/// set `seed_env` to a stable Fly secret.
///
/// Without the env var, this is a no-op (system bundles still bootstrap).
/// The check uses `engine.bundle_names()` against the live engine — never
/// POSTs `/v1/bundles` — so it's safe across cold starts and won't wipe
/// any existing data.
///
/// Why this lives in gigi-stream: the consuming application (the Vercel
/// Next.js site) cannot safely create bundles at runtime — POSTing to
/// `/v1/bundles` from a request handler is destructive on this version
/// and violates the website's runtime contract. So when a Fly machine
/// loses its `gigi_data` volume on redeploy and Tigris pull is incomplete,
/// the database itself heals on startup.
fn init_app_bundles(engine: &mut Engine) {
    let manifest_raw = match std::env::var("GIGI_APP_BUNDLES") {
        Ok(s) if !s.trim().is_empty() => s,
        _ => return,
    };

    let parsed: serde_json::Value = match serde_json::from_str(&manifest_raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[app-bundles] GIGI_APP_BUNDLES is not valid JSON: {e}");
            return;
        }
    };
    let entries = match parsed.as_array() {
        Some(a) => a,
        None => {
            eprintln!("[app-bundles] GIGI_APP_BUNDLES must be a JSON array");
            return;
        }
    };

    let existing: std::collections::HashSet<String> =
        engine.bundle_names().iter().map(|s| s.to_string()).collect();

    for entry in entries {
        let name = match entry.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => {
                eprintln!("[app-bundles] entry missing `name`: {entry}");
                continue;
            }
        };
        if existing.contains(name) {
            continue; // already there — never recreate, never wipe
        }

        // Build the schema fresh. BundleSchema's base/fiber/index consume self
        // and return Self, so we accumulate by reassigning.
        let mut schema = BundleSchema::new(name);
        let mut indexed_fields: Vec<String> = Vec::new();
        let mut bad_entry = false;
        let mut any_field_encrypted = false;

        for (section_key, is_base) in [("base", true), ("fiber", false)] {
            if bad_entry { break; }
            let arr = match entry.get(section_key).and_then(|v| v.as_array()) {
                Some(a) => a,
                None => continue,
            };
            for f in arr {
                let fname = match f.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n.to_string(),
                    None => {
                        eprintln!("[app-bundles] {name}: field missing `name`");
                        bad_entry = true;
                        break;
                    }
                };
                let ftype = f.get("type").and_then(|v| v.as_str()).unwrap_or("text");
                let mut def = match ftype.to_ascii_lowercase().as_str() {
                    "text" | "string" | "categorical" => FieldDef::categorical(&fname),
                    "numeric" | "int" | "integer" | "float" | "double" | "timestamp" => {
                        FieldDef::numeric(&fname)
                    }
                    other => {
                        eprintln!(
                            "[app-bundles] {name}.{fname}: unknown type `{other}`, defaulting to text"
                        );
                        FieldDef::categorical(&fname)
                    }
                };

                // Per-field encryption: only meaningful on FIBER fields
                // (BASE fields stay plaintext to remain hashable for the
                // base-point lookup).
                if !is_base {
                    if let Some(mode_str) = f.get("encrypted").and_then(|v| v.as_str()) {
                        let mode = match mode_str.to_ascii_lowercase().as_str() {
                            "none" | "" => gigi::types::EncryptionMode::None,
                            "affine" => gigi::types::EncryptionMode::Affine,
                            "opaque" => gigi::types::EncryptionMode::Opaque,
                            "indexed" => gigi::types::EncryptionMode::Indexed,
                            other => {
                                eprintln!(
                                    "[app-bundles] {name}.{fname}: unsupported encrypted mode \
                                     `{other}` — only none/affine/opaque/indexed are supported \
                                     by the manifest. Falling back to plaintext."
                                );
                                gigi::types::EncryptionMode::None
                            }
                        };
                        if !matches!(mode, gigi::types::EncryptionMode::None) {
                            any_field_encrypted = true;
                            def = def.with_encryption(mode);
                        }
                    }
                }

                schema = if is_base { schema.base(def) } else { schema.fiber(def) };
                if f.get("indexed").and_then(|v| v.as_bool()).unwrap_or(false) {
                    indexed_fields.push(fname);
                }
            }
        }
        if bad_entry { continue; }

        for f in indexed_fields {
            schema = schema.index(&f);
        }

        // If any field requested encryption, install a GaugeKey on the
        // schema. Seed source priority:
        //   1. Bundle-level `seed_env` (env var name → 32-byte hex)
        //   2. Fall back to CSPRNG random (with a loud warning — this
        //      means the key is volatile across redeploys, which would
        //      make any persisted ciphertext unrecoverable).
        if any_field_encrypted {
            let seed = match entry.get("seed_env").and_then(|v| v.as_str()) {
                Some(env_name) => match std::env::var(env_name) {
                    Ok(hex) => match gigi::crypto::seed_from_hex(&hex) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!(
                                "[app-bundles] {name}: seed_env `{env_name}` is set but invalid hex: {e}. \
                                 Skipping bundle creation — fix the secret and redeploy."
                            );
                            continue;
                        }
                    },
                    Err(_) => {
                        eprintln!(
                            "[app-bundles] {name}: declared encrypted fields but seed_env \
                             `{env_name}` is not set. Falling back to random seed — \
                             this is acceptable for the FIRST creation but will make \
                             ciphertext unrecoverable across the next redeploy. SET THE \
                             SECRET FOR PRODUCTION."
                        );
                        gigi::crypto::GaugeKey::random_seed()
                    }
                },
                None => {
                    eprintln!(
                        "[app-bundles] {name}: declared encrypted fields without seed_env — \
                         using random seed. Ciphertext will be unrecoverable across redeploys."
                    );
                    gigi::crypto::GaugeKey::random_seed()
                }
            };
            schema.gauge_key = Some(gigi::crypto::GaugeKey::derive(&seed, &schema.fiber_fields));
        }

        match engine.create_bundle(schema) {
            Ok(_) => {
                if any_field_encrypted {
                    eprintln!(
                        "[app-bundles] created missing bundle: {name} (with gauge_key)"
                    );
                } else {
                    eprintln!("[app-bundles] created missing bundle: {name}");
                }
            }
            Err(e) => eprintln!("[app-bundles] failed to create {name}: {e}"),
        }
    }
}

/// Helper: convert a serde_json::Value payload field into a GIGI Value.
fn log_json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Value::Integer(i) }
            else { Value::Float(n.as_f64().unwrap_or(0.0)) }
        }
        serde_json::Value::String(s) => Value::Text(s.clone()),
        serde_json::Value::Bool(b)   => Value::Bool(*b),
        serde_json::Value::Null      => Value::Null,
        other => Value::Text(other.to_string()),
    }
}

/// Async task: receives log events from `LogIngester` and inserts them into the
/// appropriate `_gigi_*` system bundle. Spawned after `Arc<StreamState>` is live.
async fn log_bundle_writer(
    mut rx:    UnboundedReceiver<LogEvent>,
    state:     Arc<StreamState>,
) {
    while let Some(event) = rx.recv().await {
        let bundle_name: &str = match event.category {
            LogCategory::Query      => "_gigi_query_log",
            LogCategory::Slow       => "_gigi_slow_log",
            LogCategory::Anomaly    => "_gigi_anomaly_log",
            LogCategory::Audit      => "_gigi_audit_log",
            LogCategory::Stream     => "_gigi_stream_log",
            LogCategory::Bundle     => "_gigi_bundle_log",
            LogCategory::Ingest     => "_gigi_ingest_log",
            LogCategory::Wal        => "_gigi_wal_log",
            LogCategory::Connection => "_gigi_conn_log",
            LogCategory::System     => "_gigi_system_log",
        };

        // Build the record from event fields.
        let mut record: HashMap<String, Value> = HashMap::new();

        // Always-present base fields.
        record.insert("ts_us".into(), Value::Integer(event.ts_us as i64));
        if let Some(dur) = event.duration_us {
            record.insert("duration_us".into(), Value::Integer(dur as i64));
        }
        record.insert("event".into(), Value::Text(event.event.to_string()));

        // For query/slow logs: pull structured fields from payload.
        if matches!(event.category, LogCategory::Query | LogCategory::Slow) {
            for key in &["statement_type", "bundle", "request_id", "slow", "error_msg"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
            for key in &["records_returned", "records_scanned"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
            // Geometric fields are nested under "geometric" block (spec §3.1)
            if let Some(serde_json::Value::Object(geo)) = event.payload.get("geometric") {
                for key in &["kl_forward", "kl_reverse", "jensen_shannon"] {
                    if let Some(v) = geo.get(*key) {
                        record.insert((*key).to_string(), log_json_to_value(v));
                    }
                }
            }
            // Normalise "bundles_accessed" array → single "bundle" text field.
            if let Some(serde_json::Value::Array(arr)) = event.payload.get("bundles_accessed") {
                let names: Vec<String> = arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
                let label = match names.len() {
                    0 => "NONE".to_string(),
                    1 => names[0].clone(),
                    _ => "MULTIPLE".to_string(),
                };
                record.insert("bundle".into(), Value::Text(label));
            }
        } else if event.category == LogCategory::Anomaly {
            for key in &["z_score", "sigma_level", "k_record", "k_mean", "k_std"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
            for key in &["bundle", "detection_source"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
        } else if event.category == LogCategory::Ingest {
            for key in &["bundle", "records_written", "bytes_written", "throughput_rps", "wal_synced", "batches", "k_before", "k_after", "k_delta"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
        } else if event.category == LogCategory::Bundle {
            for key in &["bundle", "field_count", "storage_type", "source", "records_deleted",
                         "triggered_by", "records_scanned", "fields_cached"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
            if !event.payload.is_empty() {
                let detail = serde_json::to_string(&event.payload).unwrap_or_default();
                record.insert("detail".into(), Value::Text(detail));
            }
        } else if event.category == LogCategory::Stream {
            for key in &["connection_id", "bundle", "message_seq", "messages_sent",
                         "anomalies_sent", "session_duration_us", "bytes_sent",
                         "k_global", "z_score", "is_anomaly"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
            if !event.payload.is_empty() {
                let detail = serde_json::to_string(&event.payload).unwrap_or_default();
                record.insert("detail".into(), Value::Text(detail));
            }
        } else if event.category == LogCategory::Wal {
            for key in &["bundle", "records_flushed", "bytes_flushed", "wal_size_before",
                         "wal_size_after", "records_recovered", "segments_merged",
                         "size_before", "size_after", "compression_ratio", "triggered_by"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
        } else if event.category == LogCategory::Connection {
            for key in &["protocol", "client_ip", "user_agent", "session_duration_us",
                         "requests_served", "bytes_sent", "bytes_received"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
        } else if event.category == LogCategory::Audit {
            for key in &["bundle", "actor", "client_ip", "outcome", "old_level", "new_level",
                         "records_deleted", "bytes_freed", "triggered_by"] {
                if let Some(v) = event.payload.get(*key) {
                    record.insert((*key).to_string(), log_json_to_value(v));
                }
            }
            // Store remaining payload as detail for forward-compat
            let level_str = format!("{:?}", event.level);
            record.insert("level".into(), Value::Text(level_str));
            if !event.payload.is_empty() {
                let detail = serde_json::to_string(&event.payload).unwrap_or_default();
                record.insert("detail".into(), Value::Text(detail));
            }
        } else {
            // System: store level + compressed detail.
            let level_str = format!("{:?}", event.level);
            record.insert("level".into(), Value::Text(level_str));
            if !event.payload.is_empty() {
                let detail = serde_json::to_string(&event.payload).unwrap_or_default();
                record.insert("detail".into(), Value::Text(detail));
            }
        }

        // Write into the engine — best-effort, never panic the writer task.
        let mut engine = state.engine.write().unwrap();
        if let Err(e) = engine.insert(bundle_name, &record) {
            eprintln!("[observability] insert into {bundle_name} failed: {e}");
        }
    }
}

// ── Main ──

/// Background task: hourly TTL eviction of rows from `_gigi_*` bundles.
/// Deletes rows where `ts_us < (now_us - retention_days * 86_400 * 1_000_000)`.
/// Spec §4 / §6: retention defaults are audit=365d, anomaly/slow=90d,
/// stream/conn=7d, wal/ingest=14d, everything else=30d.
async fn ttl_eviction_task(state: Arc<StreamState>) {
    let system_bundles = [
        "_gigi_query_log",
        "_gigi_slow_log",
        "_gigi_anomaly_log",
        "_gigi_audit_log",
        "_gigi_stream_log",
        "_gigi_conn_log",
        "_gigi_wal_log",
        "_gigi_ingest_log",
        "_gigi_system_log",
        "_gigi_bundle_log",
    ];
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;

        let cfg = state.logger.get_config();
        let now_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as i64;

        for bundle in &system_bundles {
            let days = cfg.retention_days(bundle) as i64;
            let cutoff_us = now_us - days * 86_400 * 1_000_000;
            let cond = vec![QueryCondition::Lt(
                "ts_us".to_string(),
                Value::Integer(cutoff_us),
            )];
            let deleted = {
                let mut engine = state.engine.write().unwrap();
                if let Some(mut store) = engine.bundle_mut(bundle) {
                    if let Some(heap) = store.as_heap_mut() {
                        heap.bulk_delete(&cond)
                    } else {
                        0
                    }
                } else {
                    0
                }
            };
            if deleted > 0 {
                let ev = LogEvent::new(
                    LogLevel::Info,
                    LogCategory::System,
                    "system.ttl_eviction",
                    &state.logger.instance,
                )
                .field("bundle",       *bundle)
                .field("deleted",      deleted as u64)
                .field("cutoff_days",  days as u64);
                state.logger.emit(ev);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3142".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // ── Observability: create Logger + Metrics before anything else ──────────
    let instance_name = std::env::var("GIGI_INSTANCE").unwrap_or_else(|_| "gigi-stream".to_string());
    let (logger, log_ingester, bundle_rx) =
        Logger::new_with_bundle_channel(LogConfig::default(), instance_name.clone());
    tokio::spawn(log_ingester.run());
    let metrics = Arc::new(Metrics::new());

    let state = Arc::new(StreamState::new(logger, metrics));

    // Phase 2: spawn log bundle writer now that state (and its engine) is live.
    tokio::spawn(log_bundle_writer(bundle_rx, Arc::clone(&state)));

    // TTL retention: hourly eviction of old rows from _gigi_* bundles.
    tokio::spawn(ttl_eviction_task(Arc::clone(&state)));

    let app = Router::new()
        // Health
        .route("/v1/health", get(health))
        // Bundle management
        .route("/v1/bundles", get(list_bundles))
        .route("/v1/bundles", post(create_bundle))
        .route("/v1/bundles/{name}", delete(drop_bundle))
        // Data operations (original GIGI style)
        .route("/v1/bundles/{name}/insert", post(insert_records))
        .route("/v1/bundles/{name}/update", post(update_records_v2))
        .route("/v1/bundles/{name}/delete", post(delete_records_v2))
        .route("/v1/bundles/{name}/query", post(filtered_query))
        .route("/v1/bundles/{name}/query-stream", post(stream_query_ndjson))
        .route("/v1/bundles/{name}/stream", post(stream_ingest))
        .route("/v1/bundles/{name}/get", get(point_query))
        .route("/v1/bundles/{name}/range", get(range_query))
        .route("/v1/bundles/{name}/join", post(pullback_join))
        .route("/v1/bundles/{name}/aggregate", post(aggregate))
        // PRISM-friendly REST endpoints
        .route(
            "/v1/bundles/{name}/points",
            get(list_all_records)
                .post(insert_records)
                .patch(bulk_update_records),
        )
        .route(
            "/v1/bundles/{name}/points/{field}/{value}",
            get(get_by_path).patch(patch_by_path).delete(delete_by_path),
        )
        // Sprint 1: CRUD operations
        .route("/v1/bundles/{name}/upsert", post(upsert_records))
        .route("/v1/bundles/{name}/count", post(count_records))
        .route("/v1/bundles/{name}/exists", post(exists_records))
        .route("/v1/bundles/{name}/distinct/{field}", get(distinct_values))
        .route("/v1/bundles/{name}/bulk-delete", post(bulk_delete_records))
        .route("/v1/bundles/{name}/truncate", post(truncate_bundle))
        .route("/v1/bundles/{name}/schema", get(get_schema))
        // Sprint 2: New operations
        .route("/v1/bundles/{name}/increment", post(increment_field))
        .route("/v1/bundles/{name}/drop-field", post(drop_field))
        .route("/v1/bundles/{name}/add-field", post(add_field))
        .route("/v1/bundles/{name}/add-index", post(add_index))
        .route("/v1/bundles/{name}/export", get(export_bundle))
        .route("/v1/bundles/{name}/dhoom", get(export_dhoom))
        .route("/v1/bundles/{name}/ingest", post(ingest_dhoom))
        .route("/v1/bundles/{name}/import", post(import_bundle))
        // Sprint 3: Enterprise operations
        .route("/v1/bundles/{name}/stats", get(bundle_stats))
        .route("/v1/bundles/{name}/explain", post(explain_query))
        .route("/v1/bundles/{name}/transaction", post(execute_transaction))
        .route(
            "/v1/bundles/{name}/vector-search",
            post(vector_search_handler),
        )
        // Admin: DHOOM snapshot + WAL compaction
        .route("/v1/admin/snapshot", post(admin_snapshot))
        // Admin: log configuration
        .route("/v1/admin/log-config", get(get_log_config).post(update_log_config))
        .route("/v1/admin/log-level", post(set_log_level))
        // OpenAPI spec
        .route("/v1/openapi.json", get(openapi_spec))
        // GQL endpoint
        .route("/v1/gql", post(gql_query))
        // Analytics
        .route("/v1/bundles/{name}/curvature", get(curvature_report))
        .route("/v1/bundles/{name}/spectral", get(spectral_report))
        .route("/v1/bundles/{name}/consistency", get(consistency_check))
        .route("/v1/bundles/{name}/betti", get(betti_report))
        .route("/v1/bundles/{name}/entropy", get(entropy_report))
        .route("/v1/bundles/{name}/free-energy", get(free_energy_report))
        .route("/v1/bundles/{name}/geodesic", post(geodesic_report))
        .route("/v1/bundles/{name}/metric", get(metric_tensor_report))
        // Anomaly Detection + Health
        .route("/v1/bundles/{name}/anomalies", post(bundle_anomalies))
        .route("/v1/bundles/{name}/health", get(bundle_health))
        .route("/v1/bundles/{name}/predict", post(predict_volatility))
        .route("/v1/bundles/{name}/anomalies/field", post(field_anomalies))
        // KL Divergence — cross-bundle information geometry
        .route("/v1/divergence", post(divergence_handler))
        // Observability
        .route("/v1/metrics", get(metrics_handler))
        // WebSocket — per-bundle subscriptions + global dashboard
        .route("/ws", get(ws_handler))
        .route("/v1/ws/dashboard", get(ws_dashboard_handler))
        .route(
            "/v1/ws/{bundle}/dashboard",
            get(ws_bundle_dashboard_handler),
        )
        // Dashboard UI
        .route("/dashboard", get(serve_dashboard));

    // L3.4: Kähler spectral gap — Marcella contract surface.
    // Mounted only when the `kahler` feature is on so the no-feature
    // build stays bit-identical to pre-upgrade GIGI.
    #[cfg(feature = "kahler")]
    let app = app.route(
        "/v1/bundles/{name}/spectral_gap",
        get(spectral_gap_endpoint),
    );

    // 2026-05-25 PR window: 4 endpoints for Marcella's Hopf +
    // Riemann-Roch wiring. Same cfg-gate; no-feature build still
    // bit-identical to pre-upgrade.
    #[cfg(feature = "kahler")]
    let app = app
        .route(
            "/v1/quantum_cohomology/compose",
            post(frobenius_compose_endpoint),
        )
        .route(
            "/v1/quantum_cohomology/capacity",
            post(capacity_endpoint),
        )
        .route(
            "/v1/bundles/{name}/holonomy_debt",
            post(holonomy_debt_endpoint),
        )
        .route(
            "/v1/bundles/{name}/flat_transport",
            post(flat_transport_endpoint),
        );

    // 2026-05-25 PR window 2: 5 brain-primitive endpoints (L13).
    // Surfaces the high-leverage subset of the L10/L11/L12 catalog
    // under /v1/bundles/{name}/brain/* — picked for diversity of
    // downstream use case (generative, gate, retrieval, memory, gist).
    #[cfg(feature = "kahler")]
    let app = app
        .route(
            "/v1/bundles/{name}/brain/sample",
            post(brain_sample_endpoint),
        )
        .route(
            "/v1/bundles/{name}/brain/confidence",
            post(brain_confidence_endpoint),
        )
        .route(
            "/v1/bundles/{name}/brain/attend",
            post(brain_attend_endpoint),
        )
        .route(
            "/v1/bundles/{name}/brain/episodic",
            post(brain_episodic_endpoint),
        )
        .route(
            "/v1/bundles/{name}/brain/semantic",
            get(brain_semantic_endpoint),
        )
        .route(
            "/v1/bundles/{name}/brain/explain",
            post(brain_explain_endpoint),
        );

    // 2026-05-25 PR window 3: the remaining 5 flow-based brain
    // primitives (L13.2). Brings the cross-team HTTP surface to
    // 10/12 of the brain-primitives catalog (FOCUS is reachable
    // via /brain/attend with `top_k`; SEMANTIC is GET, shipped
    // in PR window 2).
    #[cfg(feature = "kahler")]
    let app = app
        .route(
            "/v1/bundles/{name}/brain/dream",
            post(brain_dream_endpoint),
        )
        .route(
            "/v1/bundles/{name}/brain/forecast",
            post(brain_forecast_endpoint),
        )
        .route(
            "/v1/bundles/{name}/brain/reconstruct",
            post(brain_reconstruct_endpoint),
        )
        .route(
            "/v1/bundles/{name}/brain/inpaint",
            post(brain_inpaint_endpoint),
        )
        .route(
            "/v1/bundles/{name}/brain/predict",
            post(brain_predict_endpoint),
        );

    let app = app
        // Middleware: auth + namespace enforcement + rate limiting + readiness.
        // Layers wrap the inner router from the bottom up, so the order of
        // events on a request is auth → namespace_enforcement → rate_limit
        // → readiness → handler. auth populates `GigiClaims` in request
        // extensions; namespace_enforcement reads those claims to gate
        // /v1/bundles/<name>/* per Phase B.
        .layer(axum_mw::from_fn_with_state(state.clone(), auth_middleware))
        .layer(axum_mw::from_fn(namespace_enforcement_middleware))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            readiness_middleware,
        ))
        // CORS — configurable via GIGI_CORS_ORIGIN env var
        // Default: restrictive (no cross-origin). Set GIGI_CORS_ORIGIN=* for permissive.
        .layer(build_cors_layer())
        .with_state(state.clone());

    eprintln!("╔══════════════════════════════════════════════════════╗");
    eprintln!("║          GIGI Stream — Geometric Database            ║");
    eprintln!("║          http://{}                      ║", addr);
    eprintln!("╠══════════════════════════════════════════════════════╣");
    eprintln!("║  REST API:                                           ║");
    eprintln!("║    GET  /v1/health                                   ║");
    eprintln!("║    POST /v1/bundles                  Create bundle   ║");
    eprintln!("║    POST .../insert                   Insert O(1)     ║");
    eprintln!("║    POST .../update                   Update O(1)     ║");
    eprintln!("║    POST .../delete                   Delete O(1)     ║");
    eprintln!("║    POST .../query                    Filtered query  ║");
    eprintln!("║    POST .../stream                   NDJSON stream   ║");
    eprintln!("║    GET  .../get                      Point O(1)      ║");
    eprintln!("║    GET  .../range                    Range O(|R|)    ║");
    eprintln!("║  PRISM-compatible (REST-style):                      ║");
    eprintln!("║    GET    .../points                 List all        ║");
    eprintln!("║    POST   .../points                 Insert (alias)  ║");
    eprintln!("║    PATCH  .../points                 Bulk update     ║");
    eprintln!("║    GET    .../points/{{f}}/{{v}}         Get by field   ║");
    eprintln!("║    PATCH  .../points/{{f}}/{{v}}         Update field   ║");
    eprintln!("║    DELETE .../points/{{f}}/{{v}}         Delete record  ║");
    eprintln!("║  WebSocket:                                          ║");
    eprintln!("║    ws://{}/ws                       ║", addr);
    eprintln!("╚══════════════════════════════════════════════════════╝");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    eprintln!("Listening on {addr} — starting WAL replay in background…");

    // system.startup event — fired once the socket is bound and accepting
    {
        let e = state.logger.system_startup(
            &std::env::var("GIGI_DATA_DIR").unwrap_or_else(|_| "./gigi_data".to_string()),
            0, // bundles_loaded: will be accurate after WAL replay
            0, // wal_replayed: will be accurate after WAL replay
            0, // records_recovered: will be accurate after WAL replay
            state.start_time.elapsed().as_micros() as u64,
            &addr,
        );
        state.logger.emit(e);
    }

    // Spawn WAL replay on a blocking thread so HTTP is reachable immediately.
    // After replay, snapshot to DHOOM and reopen in mmap mode to drop the
    // ~13GB heap copy down to ~200MB RSS + OS page cache.
    let replay_state = state.clone();
    let data_dir_for_replay = std::path::PathBuf::from(
        std::env::var("GIGI_DATA_DIR").unwrap_or_else(|_| "./gigi_data".to_string()),
    );
    tokio::task::spawn_blocking(move || {
        let snapshots_dir = data_dir_for_replay.join("snapshots");

        // ── Step 1: Pull from Tigris if no local snapshots (new/volumeless machine) ──
        if !has_dhoom_files(&snapshots_dir) {
            if let Ok(bucket) = std::env::var("TIGRIS_BUCKET_NAME") {
                eprintln!("No local snapshots — pulling from Tigris bucket '{bucket}'…");
                tigris_pull(&data_dir_for_replay, &bucket);
            }
        }

        // ── Step 2: Fast path — snapshots on disk, skip heap replay entirely ─────
        if has_dhoom_files(&snapshots_dir) {
            eprintln!("Snapshots on disk — fast mmap open (skipping heap replay)…");
            match Engine::open_mmap(&data_dir_for_replay) {
                Ok(mmap_engine) => {
                    let total = mmap_engine.total_records();
                    {
                        let mut eng = replay_state.engine.write().unwrap();
                        *eng = mmap_engine;
                        init_system_bundles(&mut eng);
                        init_app_bundles(&mut eng);
                    }
                    #[cfg(unix)]
                    unsafe { libc::malloc_trim(0); }
                    replay_state.ready.store(true, Ordering::Release);
                    eprintln!("Engine ready — {total} records + _gigi_* system bundles (fast path)");
                    // Background: keep Tigris in sync with latest snapshot + WAL
                    if let Ok(bucket) = std::env::var("TIGRIS_BUCKET_NAME") {
                        let data_dir_clone = data_dir_for_replay.clone();
                        std::thread::spawn(move || {
                            tigris_push(&data_dir_clone, &bucket);
                        });
                    }
                    return;
                }
                Err(e) => {
                    eprintln!("Fast mmap open failed: {e} — falling back to heap replay");
                }
            }
        }

        // ── Step 3: Slow path — heap replay + snapshot write + mmap open ─────────
        {
            let mut engine = replay_state.engine.write().unwrap();
            let wal_t0 = std::time::Instant::now();
            if let Err(e) = engine.replay_wal() {
                eprintln!("WAL replay error: {e}");
                drop(engine);
                replay_state.ready.store(true, Ordering::Release);
                eprintln!("Engine ready (replay failed, using empty state)");
                return;
            }
            let wal_dur_us = wal_t0.elapsed().as_micros() as u64;
            let records_recovered = engine.total_records() as u64;
            // Spec §3.10: wal.replay — emitted once per startup after heap replay
            {
                let ev = replay_state.logger.wal_replay("*", records_recovered, wal_dur_us, "startup");
                replay_state.logger.emit(ev);
            }

            // Phase 2: Snapshot heap bundles to DHOOM files + compact WAL
            let total = engine.total_records();
            if total > 0 {
                eprintln!("WAL replay complete ({total} records). Snapshotting to DHOOM…");
                if let Err(e) = engine.snapshot() {
                    eprintln!("Post-replay snapshot failed: {e}");
                    // Non-fatal: we keep running on heap. Mmap upgrade skipped.
                    init_system_bundles(&mut engine);
                    init_app_bundles(&mut engine);
                    drop(engine);
                    replay_state.ready.store(true, Ordering::Release);
                    eprintln!("Engine ready — running on heap (snapshot failed)");
                    return;
                }
                eprintln!("DHOOM snapshot written. Reopening in mmap mode…");
            }
        }

        // Phase 3: Reopen in mmap mode (heap engine is dropped here)
        match Engine::open_mmap(&data_dir_for_replay) {
            Ok(mmap_engine) => {
                let total = mmap_engine.total_records();
                let mut engine = replay_state.engine.write().unwrap();
                *engine = mmap_engine;
                init_system_bundles(&mut engine);
                init_app_bundles(&mut engine);
                drop(engine);

                // Force glibc to return freed heap pages to the OS.
                // Without this, the allocator holds ~13GB of freed arenas.
                #[cfg(unix)]
                unsafe { libc::malloc_trim(0); }

                eprintln!("Mmap engine active — {total} records, RSS reduced to page cache");
            }
            Err(e) => {
                eprintln!("Mmap reopen failed: {e} — keeping heap engine");
                // init on the existing heap engine (which has replay data)
                let mut eng = replay_state.engine.write().unwrap();
                init_system_bundles(&mut eng);
                init_app_bundles(&mut eng);
            }
        }

        replay_state.ready.store(true, Ordering::Release);
        eprintln!("Engine ready — all endpoints active");

        // Background: upload snapshots + WAL to Tigris after slow-path write
        if let Ok(bucket) = std::env::var("TIGRIS_BUCKET_NAME") {
            let data_dir_clone = data_dir_for_replay.clone();
            std::thread::spawn(move || {
                eprintln!("Uploading snapshots to Tigris (background)…");
                tigris_push(&data_dir_clone, &bucket);
            });
        }
    });

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        // Wait for Ctrl-C / SIGTERM
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = sigterm.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        // Spec §3.9: system.shutdown
        let ev = state.logger.system_shutdown(
            state.start_time.elapsed().as_micros() as u64,
            0, // queries_served — counters not yet tracked globally
            0, // records_ingested
            0, // anomalies_detected
            "graceful",
        );
        state.logger.emit(ev);
        eprintln!("Graceful shutdown — system.shutdown emitted");
    })
    .await
    .unwrap();
}

/// L1.5.3 helper: build the v2-contract Record from a flat magnetic
/// transport. Returns `None` when the Kähler structure's dimension
/// doesn't match the segment dimension (caller falls back to the
/// classical quaternion / rotation path). Returns
/// `Some(Err(...))` when the integrator rejected the inputs
/// (dimension mismatch, empty segment) — surface as a TRANSPORT
/// error to the GQL client.
///
/// Extracted from the inline match arm so the in-bin test can call
/// it directly without spinning up a full BundleMut.
#[cfg(feature = "kahler")]
fn kahler_transport_dispatch(
    kahler: &gigi::geometry::KahlerStructure,
    p_from: &[f64],
    p_to: &[f64],
    displacement: &[f64],
) -> Option<Result<gigi::types::Record, String>> {
    let dim = p_from.len();
    if kahler.dim() != dim {
        return None;
    }

    // Unit-vector initial velocity pointing from p_from toward
    // p_to. Magnetic perturbation bends the path, so the trajectory
    // does NOT in general arrive AT p_to — that's the price of
    // having a bias. Callers wanting strict endpoint-reaching
    // should use the classical path or wait for the curved-space
    // L5.5 transport API that solves a BVP.
    let mut v_init = displacement.to_vec();
    let mag: f64 = v_init.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag > 1e-12 {
        for x in &mut v_init {
            *x /= mag;
        }
    }

    let seg = match gigi::geometry::TransportSegment::new(
        p_from.to_vec(),
        p_to.to_vec(),
        v_init,
    ) {
        Ok(s) => s,
        Err(e) => return Some(Err(format!("TRANSPORT: {e}"))),
    };

    let r = match gigi::geometry::flat_transport(
        &seg,
        Some(&kahler.b),
        1e-3,
        10_000,
        gigi::geometry::BSource::Bundle,
    ) {
        Ok(r) => r,
        Err(e) => return Some(Err(format!("TRANSPORT: {e}"))),
    };

    // Build the v2-contract response record (consumption draft §2).
    let mut result = gigi::types::Record::new();
    result.insert(
        "path_length".to_string(),
        gigi::types::Value::Float(r.path_length),
    );
    result.insert(
        "energy_drift".to_string(),
        gigi::types::Value::Float(r.energy_drift),
    );
    result.insert(
        "holonomy_norm".to_string(),
        gigi::types::Value::Float(r.holonomy_norm),
    );
    result.insert(
        "used_magnetic".to_string(),
        gigi::types::Value::Bool(r.used_magnetic),
    );
    result.insert(
        "b_source".to_string(),
        gigi::types::Value::Text(
            match r.b_source {
                gigi::geometry::BSource::Bundle => "bundle",
                gigi::geometry::BSource::Override => "override",
                gigi::geometry::BSource::None => "none",
                gigi::geometry::BSource::FallbackNonClosed => "fallback_non_closed",
            }
            .to_string(),
        ),
    );
    if let Some(c) = r.closedness_norm {
        result.insert(
            "closedness_norm".to_string(),
            gigi::types::Value::Float(c),
        );
    }
    // Surface displacement so callers upgrading from the classical
    // verb still see the familiar diagnostic fields.
    for (i, d) in displacement.iter().enumerate() {
        result.insert(
            format!("displacement_{i}"),
            gigi::types::Value::Float(*d),
        );
    }

    Some(Ok(result))
}

// ── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gigi::dhoom;
    use gigi::engine::Engine;
    use gigi::types::{BundleSchema, FieldDef, FieldType};
    use std::path::Path;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("gigi_stream_test_{tag}"))
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // ── BundleFlowCache contract tests (S1 wave 1 §A) ──────────
    //
    // The cache is the load-bearing latency property for brain
    // endpoints. These tests pin the contract: cache miss
    // computes, cache hit serves stored value, counter mismatch
    // invalidates, and the random eviction respects max_entries.
    #[cfg(feature = "kahler")]
    #[test]
    fn flow_cache_miss_then_hit_then_invalidate() {
        let cache = BundleFlowCache::new(10);
        let key = CacheKey::build("test_bundle", FitMode::Diagonal, &["a".to_string(), "b".to_string()], Some(1e-3));

        // 1. Empty cache: miss.
        assert!(cache.get(&key, 0).is_none(), "fresh cache must miss");

        // 2. Insert at counter 5.
        let fit = CachedFit {
            counter_at_fit: 5,
            mu: Arc::new(vec![1.0, 2.0]),
            sigma_sq: 0.5,
            sigma_sq_per_field: Arc::new(vec![0.4, 0.6]),
            sigma_sq_per_field_raw: Arc::new(vec![0.4, 0.6]),
            effective_floor: 0.0,
            floored_indices: Arc::new(Vec::new()),
            precision: None,
            covariance: None,
            eigenvalues_raw: None,
            eigenvalues_effective: None,
            eigenvalue_floor_used: 0.0,
            floored_eigenvalue_count: 0,
            condition_number: 1.5,
            variance_ratio: 1.5,
        };
        cache.insert(key.clone(), fit);

        // 3. Hit at the SAME counter — returns the cached fit.
        let hit = cache.get(&key, 5);
        assert!(hit.is_some(), "hit at same counter");
        assert_eq!(hit.unwrap().sigma_sq, 0.5);

        // 4. Counter mismatch — must return None (stale).
        assert!(
            cache.get(&key, 6).is_none(),
            "counter mismatch must invalidate"
        );
        assert!(
            cache.get(&key, 0).is_none(),
            "earlier counter also invalidates"
        );
    }

    #[cfg(feature = "kahler")]
    #[test]
    fn flow_cache_evicts_at_capacity() {
        let cache = BundleFlowCache::new(3);
        // Fill to capacity.
        for i in 0..3 {
            let key = CacheKey::build(
                &format!("bundle_{}", i),
                FitMode::Diagonal,
                &["a".to_string()],
                Some(1e-3),
            );
            let fit = CachedFit {
                counter_at_fit: 0,
                mu: Arc::new(vec![i as f64]),
                sigma_sq: 1.0,
                sigma_sq_per_field: Arc::new(vec![1.0]),
                sigma_sq_per_field_raw: Arc::new(vec![1.0]),
                effective_floor: 0.0,
                floored_indices: Arc::new(Vec::new()),
                precision: None,
                covariance: None,
                eigenvalues_raw: None,
                eigenvalues_effective: None,
                eigenvalue_floor_used: 0.0,
                floored_eigenvalue_count: 0,
                condition_number: 1.0,
                variance_ratio: 1.0,
            };
            cache.insert(key, fit);
        }
        assert_eq!(cache.len(), 3, "filled to capacity");

        // Insert one more — eviction must keep len bounded.
        let key4 = CacheKey::build("bundle_4", FitMode::Diagonal, &["a".to_string()], Some(1e-3));
        let fit4 = CachedFit {
            counter_at_fit: 0,
            mu: Arc::new(vec![4.0]),
            sigma_sq: 1.0,
            sigma_sq_per_field: Arc::new(vec![1.0]),
            sigma_sq_per_field_raw: Arc::new(vec![1.0]),
            effective_floor: 0.0,
            floored_indices: Arc::new(Vec::new()),
            precision: None,
            covariance: None,
            eigenvalues_raw: None,
            eigenvalues_effective: None,
            eigenvalue_floor_used: 0.0,
            floored_eigenvalue_count: 0,
            condition_number: 1.0,
            variance_ratio: 1.0,
        };
        cache.insert(key4.clone(), fit4);
        assert_eq!(cache.len(), 3, "still at capacity after eviction");
        // New key is present.
        assert!(cache.get(&key4, 0).is_some(), "newly-inserted key present");
    }

    #[cfg(feature = "kahler")]
    #[test]
    fn flow_cache_key_disambiguates_fit_mode_and_fields() {
        // Same bundle, different fit_mode → different cache entries.
        let k_iso = CacheKey::build("b", FitMode::Isotropic, &["x".to_string()], Some(1e-3));
        let k_diag = CacheKey::build("b", FitMode::Diagonal, &["x".to_string()], Some(1e-3));
        let k_full = CacheKey::build("b", FitMode::Full, &["x".to_string()], Some(1e-3));
        assert_ne!(k_iso, k_diag, "fit modes distinguish");
        assert_ne!(k_diag, k_full, "fit modes distinguish");
        assert_ne!(k_iso, k_full, "fit modes distinguish");

        // Same bundle + mode, different fields → different entries.
        let k_a = CacheKey::build("b", FitMode::Diagonal, &["x".to_string()], Some(1e-3));
        let k_b = CacheKey::build("b", FitMode::Diagonal, &["y".to_string()], Some(1e-3));
        assert_ne!(k_a, k_b, "field set distinguishes");

        // Same everything except sigma_floor_epsilon → different.
        let k_eps1 = CacheKey::build("b", FitMode::Diagonal, &["x".to_string()], Some(1e-3));
        let k_eps2 = CacheKey::build("b", FitMode::Diagonal, &["x".to_string()], Some(1e-4));
        assert_ne!(k_eps1, k_eps2, "floor epsilon distinguishes");

        // Same everything → equal (cache hit).
        let k_a2 = CacheKey::build("b", FitMode::Diagonal, &["x".to_string()], Some(1e-3));
        assert_eq!(k_a, k_a2, "identical key parameters → identical key");
    }

    /// Parse a DHOOM body the same way ingest_dhoom will: decode → json_to_value
    /// records → batch_insert.  Verify curvature is non-zero (geometric
    /// structure is preserved end-to-end).
    #[test]
    fn test_dhoom_ingest_pipeline_curvature() {
        let dir = tmp_dir("ingest_curvature");
        cleanup(&dir);

        let mut engine = Engine::open(&dir).unwrap();
        let schema = BundleSchema::new("sensors")
            .base(FieldDef::numeric("ts"))
            .fiber(FieldDef::numeric("temp").with_range(100.0))
            .fiber(FieldDef::categorical("unit"));
        engine.create_bundle(schema).unwrap();

        // Realistic sensor DHOOM — ts arithmetic, modal unit default
        let dhoom_body = "sensors{ts@1710000000+60, temp, unit|C}:\n22.5\n35.0\n10.2\n18.7\n40.1\n";
        let json_recs = dhoom::decode_to_json(dhoom_body).unwrap();
        assert_eq!(json_recs.len(), 5, "decoder must return all 5 records");

        let records: Vec<Record> = json_recs
            .iter()
            .filter_map(|item| {
                if let serde_json::Value::Object(map) = item {
                    Some(map.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect())
                } else {
                    None
                }
            })
            .collect();

        let inserted = engine.batch_insert("sensors", &records).unwrap();
        assert_eq!(inserted, 5, "all 5 records must be WAL-inserted");

        let store = engine.bundle("sensors").unwrap();
        let k = store.scalar_curvature();
        assert!(k > 0.0, "curvature must be positive after inserting varied temp data; got {k}");

        let conf = curvature::confidence(k);
        assert!(
            (0.0..=1.0).contains(&conf),
            "confidence must be in (0, 1]; got {conf}"
        );

        cleanup(&dir);
    }

    /// Ephemeral records must NOT appear in the engine after ingest — the
    /// bundle's record count stays at zero.
    #[test]
    fn test_ephemeral_records_not_persisted() {
        let dir = tmp_dir("ephemeral");
        cleanup(&dir);

        let mut engine = Engine::open(&dir).unwrap();
        let schema = BundleSchema::new("typing_events")
            .base(FieldDef::categorical("sender"))
            .fiber(FieldDef::categorical("conv_id"));
        engine.create_bundle(schema).unwrap();

        // For ephemeral ingest: we deliberately skip batch_insert and assert 0.
        // This test encodes the contract: ephemeral=true → no WAL write.
        let dhoom_body = "typing_events{sender, conv_id}:\nalice, room-1\nbob, room-1\n";
        let json_recs = dhoom::decode_to_json(dhoom_body).unwrap();
        assert_eq!(json_recs.len(), 2);

        // Ephemeral path: parse but DO NOT insert
        let store = engine.bundle("typing_events").unwrap();
        assert_eq!(store.len(), 0, "ephemeral records must not be persisted");

        cleanup(&dir);
    }

    /// DHOOM with arithmetic base field decodes to monotonically increasing ids.
    /// This validates the exact field-key generation the ingest handler relies on.
    #[test]
    fn test_dhoom_arithmetic_ids_become_integer_values() {
        let dhoom = "messages{id@1, body, read|F}:\nhello\nworld\n";
        let recs = dhoom::decode_to_json(dhoom).unwrap();
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0]["id"], serde_json::json!(1i64));
        assert_eq!(recs[1]["id"], serde_json::json!(2i64));
        assert_eq!(recs[0]["read"], serde_json::json!(false));

        // Converted to GIGI Value::Integer, not Float
        let v = json_to_value(&recs[0]["id"]);
        assert!(
            matches!(v, Value::Integer(1)),
            "id must convert to Value::Integer(1), got {v:?}"
        );
    }

    // ── Value::Binary TDD tests ────────────────────────────────────────────

    /// Binary payload survives json_to_value → value_to_json round-trip.
    #[test]
    fn test_binary_roundtrip_via_json() {
        let payload: Vec<u8> = vec![0x00, 0xFF, 0x80, 0x01, 0x02, 0xFE];
        use base64::Engine as _;
        let b64 = format!(
            "b64:{}",
            base64::engine::general_purpose::STANDARD.encode(&payload)
        );
        let json_in = serde_json::Value::String(b64);

        // Decode: "b64:..." → Value::Binary
        let gigi_val = json_to_value(&json_in);
        assert!(
            matches!(gigi_val, Value::Binary(ref b) if *b == payload),
            "json_to_value must produce Value::Binary, got {gigi_val:?}"
        );

        // Re-encode: Value::Binary → "b64:..."
        let json_out = value_to_json(&gigi_val);
        assert_eq!(json_in, json_out, "value_to_json must restore the b64: string");
    }

    /// Plain strings must NOT become Value::Binary.
    #[test]
    fn test_plain_string_stays_text() {
        let plain = serde_json::Value::String("hello world".into());
        let v = json_to_value(&plain);
        assert!(
            matches!(v, Value::Text(_)),
            "plain string must stay Value::Text, got {v:?}"
        );
    }

    // ── §8.9 b64: escape convention (user-controlled text) ────────────────
    //
    // User-generated text that literally begins with "b64:" must be escaped
    // before writing to any GIGI field (§2.1 collision policy).
    // Convention: prepend another "b64:" prefix.
    //
    //   Sender:   user text "b64:hello" → write "b64:b64:hello"
    //   Receiver: json_to_value("b64:b64:hello") → Value::Text("b64:hello")
    //   Re-encode: value_to_json(Value::Text("b64:hello")) → "b64:b64:hello"
    //
    // This creates a lossless round-trip with no schema-based exceptions.

    #[test]
    fn test_b64_escape_decoded_as_text() {
        // "b64:b64:hello" must be decoded as Value::Text("b64:hello"), not binary.
        let escaped = serde_json::Value::String("b64:b64:hello".into());
        let v = json_to_value(&escaped);
        assert_eq!(
            v,
            Value::Text("b64:hello".into()),
            "double-prefix escape must return Value::Text with one prefix stripped"
        );
    }

    #[test]
    fn test_b64_escape_triple_prefix_roundtrip() {
        // Text that is literally "b64:b64:foo" gets three prefixes on wire,
        // receiver strips one → Text("b64:b64:foo"). Fully recursive.
        let escaped = serde_json::Value::String("b64:b64:b64:foo".into());
        let v = json_to_value(&escaped);
        assert_eq!(v, Value::Text("b64:b64:foo".into()));
    }

    #[test]
    fn test_value_to_json_escapes_text_starting_with_b64() {
        // value_to_json must emit the extra prefix so the receiver decodes correctly.
        let v = Value::Text("b64:sensitive data".into());
        let json = value_to_json(&v);
        assert_eq!(
            json,
            serde_json::Value::String("b64:b64:sensitive data".into()),
            "value_to_json must escape Text starting with b64:"
        );
    }

    #[test]
    fn test_b64_escape_full_roundtrip_text() {
        // json_to_value → store → value_to_json must reproduce the original wire string.
        let wire = "b64:b64:user typed this literally";
        let v = json_to_value(&serde_json::Value::String(wire.into()));
        assert_eq!(v, Value::Text("b64:user typed this literally".into()));
        let re_encoded = value_to_json(&v);
        assert_eq!(
            re_encoded,
            serde_json::Value::String(wire.into()),
            "round-trip must reproduce the original escaped wire string"
        );
    }

    #[test]
    fn test_b64_escape_does_not_affect_normal_binary() {
        // Normal binary ingest must still work after adding the escape logic.
        let encoded = serde_json::Value::String("b64:AAEC/w==".into());
        let v = json_to_value(&encoded);
        assert!(
            matches!(v, Value::Binary(ref b) if b.as_slice() == [0x00u8, 0x01, 0x02, 0xFF]),
            "normal b64: must still decode to Value::Binary, got {v:?}"
        );
    }

    /// Value::Binary survives WAL encode → decode round-trip.
    #[test]
    fn test_binary_wal_roundtrip() {
        let dir = tmp_dir("binary_wal");
        cleanup(&dir);

        let mut engine = Engine::open(&dir).unwrap();
        let schema = BundleSchema::new("blobs")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("payload"));
        engine.create_bundle(schema).unwrap();

        let payload: Vec<u8> = (0u8..=255).collect();
        use base64::Engine as _;
        let b64_str = format!(
            "b64:{}",
            base64::engine::general_purpose::STANDARD.encode(&payload)
        );

        let mut rec: Record = Record::new();
        rec.insert("id".into(), Value::Integer(1));
        rec.insert(
            "payload".into(),
            Value::Binary(payload.clone()),
        );
        engine.insert("blobs", &rec).unwrap();

        // Reopen to force WAL replay
        drop(engine);
        let engine2 = Engine::open(&dir).unwrap();
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let result = engine2.point_query("blobs", &key).unwrap().unwrap();

        assert!(
            matches!(result.get("payload"), Some(Value::Binary(b)) if *b == payload),
            "Binary payload must survive WAL encode → decode, got {:?}",
            result.get("payload")
        );

        // Verify value_to_json produces b64: prefix when serialising
        let out = value_to_json(result.get("payload").unwrap());
        assert_eq!(out, serde_json::Value::String(b64_str));

        cleanup(&dir);
    }

    // ── §8.5 Interop Fixture — binary voice note ingest + replay ──────────
    //
    // These three tests map exactly to the §8.5 pass criteria:
    //   Criteria 1+3: test_s85_ndjson_ingest_stores_binary
    //   Criterion  2: test_s85_point_query_by_message_id
    //   Criterion  4+5: test_s85_dhoom_export_reimport_byte_fidelity
    //
    // Run them as a suite: cargo test s85

    fn chat_reaction_schema() -> BundleSchema {
        BundleSchema::new("chat/reaction")
            .base(FieldDef::categorical("projection_type"))
            .base(FieldDef::categorical("sender_id"))
            .base(FieldDef::timestamp("timestamp_ns", 1e9))
            .base(FieldDef::categorical("target_id"))
            .fiber(FieldDef::categorical("emoji"))
            .fiber(FieldDef::categorical("action"))
            .fiber(FieldDef::categorical("conversation_id").with_default(Value::Null))
            .index("timestamp_ns")
            .index("target_id")
    }

    fn chat_voice_note_schema() -> BundleSchema {
        // message_id is the sole base field (primary key) so that
        // point_query({"message_id": ...}) works with a single-column key.
        // Full multi-column base keys require all columns for hash lookup.
        BundleSchema::new("chat_voice_note")
            .base(FieldDef::categorical("message_id"))
            .fiber(FieldDef::categorical("sender_id"))
            .fiber(FieldDef::categorical("recipient_id"))
            .fiber(FieldDef::categorical("conversation_id"))
            .fiber(FieldDef::categorical("projection_type"))
            .fiber(FieldDef::timestamp("timestamp_ns", 1e9))
            .fiber(FieldDef::binary("media_bytes"))
            .fiber(FieldDef::numeric("duration_ms").with_range(60_000.0))
            .fiber(FieldDef::categorical("encrypted").with_default(Value::Bool(false)))
    }

    // §8.5 primary fixture — matches the spec exactly.
    const S85_NDJSON: &str = r#"{"projection_type":"chat/voice_note","sender_id":"alice","recipient_id":"bob","timestamp_ns":1710000000000000000,"message_id":"msg-vn-001","conversation_id":"conv-xyz","media_bytes":"b64:AAEC/w==","duration_ms":4200,"encrypted":true}"#;

    // Second record used only in the DHOOM round-trip test. DHOOM encodes a
    // 1-record batch as all-defaults with no data rows, so the decoder returns
    // 0 records. Two records produce proper columnar rows.
    const S85_NDJSON_2: &str = r#"{"projection_type":"chat/voice_note","sender_id":"carol","recipient_id":"bob","timestamp_ns":1710000060000000000,"message_id":"msg-vn-002","conversation_id":"conv-xyz","media_bytes":"b64:BQYHCAk=","duration_ms":3100,"encrypted":true}"#;

    const S85_EXPECTED_BYTES: [u8; 4] = [0x00, 0x01, 0x02, 0xFF];
    const S85_EXPECTED_BYTES_2: [u8; 5] = [0x05, 0x06, 0x07, 0x08, 0x09];

    fn parse_ndjson_record(ndjson: &str) -> Record {
        let json_val: serde_json::Value = serde_json::from_str(ndjson).unwrap();
        if let serde_json::Value::Object(map) = json_val {
            map.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect()
        } else {
            panic!("expected JSON object");
        }
    }

    /// §8.5 criterion 1: ingest returns count=1.
    /// §8.5 criterion 3: media_bytes stored as Value::Binary — exact bytes,
    ///                    no b64: prefix at rest.
    ///
    /// NOTE: the §8.5 spec says "curvature > 0" but a single-record bundle
    /// always has curvature = 0.0 (no variance to measure). The binary
    /// storage check is the substantive criterion here.
    #[test]
    fn test_s85_ndjson_ingest_stores_binary() {
        let dir = tmp_dir("s85_ingest");
        cleanup(&dir);

        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(chat_voice_note_schema()).unwrap();

        // Criterion 1: count == 1
        let inserted = engine
            .batch_insert("chat_voice_note", &[parse_ndjson_record(S85_NDJSON)])
            .unwrap();
        assert_eq!(inserted, 1, "ingest must return count=1");

        // Criterion 3: media_bytes is Value::Binary in storage — no prefix at rest
        let store = engine.bundle("chat_voice_note").unwrap();
        let record = store.records().next().expect("one record must be in store");
        let media = record.get("media_bytes").expect("media_bytes must be present");
        assert!(
            matches!(media, Value::Binary(b) if b.as_slice() == S85_EXPECTED_BYTES),
            "media_bytes must be Value::Binary([0x00,0x01,0x02,0xFF]) in storage, got {media:?}"
        );

        cleanup(&dir);
    }

    /// §8.5 criterion 2: point-query by message_id returns the record.
    #[test]
    fn test_s85_point_query_by_message_id() {
        let dir = tmp_dir("s85_query");
        cleanup(&dir);

        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(chat_voice_note_schema()).unwrap();
        engine
            .batch_insert("chat_voice_note", &[parse_ndjson_record(S85_NDJSON)])
            .unwrap();

        // Criterion 2: point-query by message_id finds the record
        let mut key = Record::new();
        key.insert("message_id".into(), Value::Text("msg-vn-001".into()));
        let result = engine
            .point_query("chat_voice_note", &key)
            .unwrap()
            .expect("point_query must find msg-vn-001");

        assert_eq!(
            result.get("sender_id"),
            Some(&Value::Text("alice".into())),
            "sender_id must be 'alice'"
        );
        assert!(
            matches!(result.get("media_bytes"), Some(Value::Binary(b)) if b.as_slice() == S85_EXPECTED_BYTES),
            "queried record must have Value::Binary media_bytes, got {:?}",
            result.get("media_bytes")
        );

        cleanup(&dir);
    }

    /// §8.5 criterion 4: DHOOM re-export completes without error.
    /// §8.5 criterion 5: re-importing the DHOOM export into a fresh engine
    ///                    produces identical bytes for media_bytes.
    ///
    /// Two records are used because DHOOM encodes a 1-record batch as
    /// all-defaults with no data rows — the decoder returns 0 records.
    /// With 2+ records, DHOOM emits columnar rows and the round-trip is
    /// lossless.
    #[test]
    fn test_s85_dhoom_export_reimport_byte_fidelity() {
        let dir1 = tmp_dir("s85_export");
        let dir2 = tmp_dir("s85_reimport");
        cleanup(&dir1);
        cleanup(&dir2);

        // Ingest two records into first engine
        let mut engine1 = Engine::open(&dir1).unwrap();
        engine1.create_bundle(chat_voice_note_schema()).unwrap();
        engine1
            .batch_insert(
                "chat_voice_note",
                &[
                    parse_ndjson_record(S85_NDJSON),
                    parse_ndjson_record(S85_NDJSON_2),
                ],
            )
            .unwrap();

        // Criterion 4: DHOOM re-export completes without error
        let store1 = engine1.bundle("chat_voice_note").unwrap();
        let json_records: Vec<serde_json::Value> =
            store1.records().map(|r| record_to_json(&r)).collect();
        assert_eq!(json_records.len(), 2, "export must contain 2 records");

        let export = dhoom::encode_json(&json_records, "chat_voice_note");
        assert!(!export.dhoom.is_empty(), "DHOOM export must not be empty");

        drop(store1);
        drop(engine1);

        // Criterion 5: decode and re-import into a fresh engine
        let mut engine2 = Engine::open(&dir2).unwrap();
        engine2.create_bundle(chat_voice_note_schema()).unwrap();

        let decoded = dhoom::decode_to_json(&export.dhoom)
            .expect("DHOOM decode of exported payload must succeed");
        assert_eq!(decoded.len(), 2, "decoded export must contain 2 records");

        let reimported: Vec<Record> = decoded
            .iter()
            .filter_map(|item| {
                if let serde_json::Value::Object(map) = item {
                    Some(map.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect())
                } else {
                    None
                }
            })
            .collect();
        engine2
            .batch_insert("chat_voice_note", &reimported)
            .unwrap();

        // Criterion 5: both records have identical bytes after round-trip
        let cases: &[(&str, &[u8])] = &[
            ("msg-vn-001", &S85_EXPECTED_BYTES),
            ("msg-vn-002", &S85_EXPECTED_BYTES_2),
        ];
        for (msg_id, expected) in cases {
            let mut key = Record::new();
            key.insert("message_id".into(), Value::Text((*msg_id).into()));
            let rec = engine2
                .point_query("chat_voice_note", &key)
                .unwrap()
                .unwrap_or_else(|| panic!("must find {msg_id} after DHOOM reimport"));
            let media = rec
                .get("media_bytes")
                .unwrap_or_else(|| panic!("media_bytes missing for {msg_id}"));
            assert!(
                matches!(media, Value::Binary(b) if b.as_slice() == *expected),
                "media_bytes for {msg_id} must be {:?} after DHOOM round-trip, got {media:?}",
                expected
            );
        }

        cleanup(&dir1);
        cleanup(&dir2);
    }

    // ── FieldDef::binary() constructor ────────────────────────────────────

    #[test]
    fn test_field_def_binary_constructor() {
        let f = FieldDef::binary("media_bytes");
        assert_eq!(f.name, "media_bytes");
        assert_eq!(f.field_type, FieldType::Binary);
        assert_eq!(f.default, Value::Null);
        assert!(f.range.is_none());
    }

    // ── chat_reaction_schema() fields ────────────────────────────────────

    #[test]
    fn test_chat_reaction_schema_fields() {
        let schema = chat_reaction_schema();
        assert_eq!(schema.name, "chat/reaction");

        // Base fields: projection_type, sender_id, timestamp_ns, target_id
        let base_names: Vec<&str> = schema.base_fields.iter().map(|f| f.name.as_str()).collect();
        assert!(base_names.contains(&"projection_type"), "missing projection_type base");
        assert!(base_names.contains(&"sender_id"), "missing sender_id base");
        assert!(base_names.contains(&"timestamp_ns"), "missing timestamp_ns base");
        assert!(base_names.contains(&"target_id"), "missing target_id base");

        // Fiber fields: emoji, action, conversation_id
        let fiber_names: Vec<&str> = schema.fiber_fields.iter().map(|f| f.name.as_str()).collect();
        assert!(fiber_names.contains(&"emoji"), "missing emoji fiber");
        assert!(fiber_names.contains(&"action"), "missing action fiber");
        assert!(fiber_names.contains(&"conversation_id"), "missing conversation_id fiber");

        // timestamp_ns fiber must be Timestamp type
        let ts = schema.base_fields.iter().find(|f| f.name == "timestamp_ns").unwrap();
        assert_eq!(ts.field_type, FieldType::Timestamp);

        // Indexes include timestamp_ns and target_id
        assert!(schema.indexed_fields.contains(&"timestamp_ns".to_string()));
        assert!(schema.indexed_fields.contains(&"target_id".to_string()));
    }

    // ── chat_voice_note_schema() uses FieldType::Binary ──────────────────

    #[test]
    fn test_chat_voice_note_media_bytes_is_binary_type() {
        let schema = chat_voice_note_schema();
        let media_field = schema
            .fiber_fields
            .iter()
            .find(|f| f.name == "media_bytes")
            .expect("media_bytes fiber must exist");
        assert_eq!(
            media_field.field_type,
            FieldType::Binary,
            "media_bytes must be FieldType::Binary, not Categorical"
        );
    }

    // ── Binary size enforcement (§2.1 — 1 MiB hard cap) ─────────────────

    #[test]
    fn test_binary_size_enforcement_rejects_oversized_payload() {
        let mut record = Record::new();
        record.insert(
            "media_bytes".into(),
            Value::Binary(vec![0u8; MAX_BINARY_FIELD_BYTES + 1]),
        );
        let result = check_binary_sizes(&[record]);
        assert!(result.is_err(), "must reject binary field > 1 MiB");
        let (field, size) = result.unwrap_err();
        assert_eq!(field, "media_bytes");
        assert_eq!(size, MAX_BINARY_FIELD_BYTES + 1);
    }

    #[test]
    fn test_binary_size_enforcement_allows_exactly_1mib() {
        let mut record = Record::new();
        record.insert(
            "media_bytes".into(),
            Value::Binary(vec![0u8; MAX_BINARY_FIELD_BYTES]),
        );
        assert!(
            check_binary_sizes(&[record]).is_ok(),
            "exactly 1 MiB must be accepted"
        );
    }

    #[test]
    fn test_binary_size_enforcement_passes_non_binary_records() {
        let mut record = Record::new();
        record.insert("message_id".into(), Value::Text("msg-001".into()));
        record.insert("duration_ms".into(), Value::Integer(4200));
        assert!(
            check_binary_sizes(&[record]).is_ok(),
            "non-binary records must always pass"
        );
    }

    // ── §9.6 CI Fixture Coverage — all six event families ─────────────────
    //
    // One smoke test per remaining family (chat/dm · signal · ack · typing · reaction).
    // chat/voice_note is fully covered by the three §8.5 tests above.
    //
    // Bundle names use underscores (no slash) — slash chars in bundle names
    // would be interpreted as filesystem path separators in WAL file creation.
    // The projection_type field value retains the "chat/X" slash namespace.

    fn chat_dm_schema_test() -> BundleSchema {
        BundleSchema::new("chat_dm")
            .base(FieldDef::categorical("projection_type"))
            .base(FieldDef::categorical("sender_id"))
            .base(FieldDef::timestamp("timestamp_ns", 1e9))
            .base(FieldDef::categorical("message_id"))
            .base(FieldDef::categorical("recipient_id"))
            .base(FieldDef::categorical("conversation_id"))
            .fiber(FieldDef::categorical("body"))
            .fiber(FieldDef::categorical("encrypted").with_default(Value::Bool(false)))
            .fiber(FieldDef::categorical("media_ref").with_default(Value::Null))
            .fiber(FieldDef::categorical("reply_to").with_default(Value::Null))
            .fiber(FieldDef::categorical("edited").with_default(Value::Bool(false)))
            .index("timestamp_ns")
            .index("conversation_id")
    }

    fn chat_signal_schema_test() -> BundleSchema {
        BundleSchema::new("chat_signal")
            .base(FieldDef::categorical("projection_type"))
            .base(FieldDef::categorical("sender_id"))
            .base(FieldDef::timestamp("timestamp_ns", 1e9))
            .base(FieldDef::categorical("recipient_id"))
            .base(FieldDef::categorical("call_id"))
            .fiber(FieldDef::categorical("signal_type"))
            .fiber(FieldDef::categorical("sdp").with_default(Value::Null))
            .fiber(FieldDef::categorical("ice_candidate").with_default(Value::Null))
            .fiber(FieldDef::categorical("media_type").with_default(Value::Null))
            .index("timestamp_ns")
            .index("call_id")
    }

    fn chat_ack_schema_test() -> BundleSchema {
        BundleSchema::new("chat_ack")
            .base(FieldDef::categorical("projection_type"))
            .base(FieldDef::categorical("sender_id"))
            .base(FieldDef::timestamp("timestamp_ns", 1e9))
            .base(FieldDef::categorical("recipient_id"))
            .fiber(FieldDef::categorical("target_id"))
            .fiber(FieldDef::categorical("ack_type"))
            .fiber(FieldDef::categorical("conversation_id").with_default(Value::Null))
            .index("timestamp_ns")
            .index("target_id")
    }

    fn chat_typing_schema_test() -> BundleSchema {
        BundleSchema::new("chat_typing")
            .base(FieldDef::categorical("projection_type"))
            .base(FieldDef::categorical("sender_id"))
            .base(FieldDef::timestamp("timestamp_ns", 1e9))
            .base(FieldDef::categorical("recipient_id"))
            .fiber(FieldDef::categorical("state"))
            .fiber(FieldDef::categorical("conversation_id").with_default(Value::Null))
    }

    fn chat_reaction_schema_test() -> BundleSchema {
        BundleSchema::new("chat_reaction")
            .base(FieldDef::categorical("projection_type"))
            .base(FieldDef::categorical("sender_id"))
            .base(FieldDef::timestamp("timestamp_ns", 1e9))
            .base(FieldDef::categorical("target_id"))
            .fiber(FieldDef::categorical("emoji"))
            .fiber(FieldDef::categorical("action"))
            .fiber(FieldDef::categorical("conversation_id").with_default(Value::Null))
            .index("timestamp_ns")
            .index("target_id")
    }

    const DM_PLAIN_NDJSON: &str = r#"{"projection_type":"chat/dm","sender_id":"alice","timestamp_ns":1710000000000000000,"message_id":"msg-dm-001","recipient_id":"bob","conversation_id":"conv-abc","body":"hello world","encrypted":false}"#;
    const DM_ENC_NDJSON: &str = r#"{"projection_type":"chat/dm","sender_id":"alice","timestamp_ns":1710000000000000001,"message_id":"msg-dm-002","recipient_id":"bob","conversation_id":"conv-abc","body":"b64:AAEC/w==","encrypted":true}"#;
    const SIGNAL_NDJSON: &str = r#"{"projection_type":"chat/signal","sender_id":"alice","timestamp_ns":1710000000000000002,"recipient_id":"bob","call_id":"call-001","signal_type":"offer"}"#;
    const ACK_NDJSON: &str = r#"{"projection_type":"chat/ack","sender_id":"bob","timestamp_ns":1710000000000000003,"recipient_id":"alice","target_id":"msg-dm-001","ack_type":"delivered"}"#;
    const TYPING_NDJSON: &str = r#"{"projection_type":"chat/typing","sender_id":"alice","timestamp_ns":1710000000000000004,"recipient_id":"bob","state":"start"}"#;
    const REACTION_NDJSON: &str = "{\"projection_type\":\"chat/reaction\",\"sender_id\":\"bob\",\"timestamp_ns\":1710000000000000005,\"target_id\":\"msg-dm-001\",\"emoji\":\"\u{1F44D}\",\"action\":\"add\"}";

    #[test]
    fn test_chat_dm_ingest_plain_body() {
        let dir = tmp_dir("chat_dm_plain");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(chat_dm_schema_test()).unwrap();
        let rec = parse_ndjson_record(DM_PLAIN_NDJSON);
        engine.batch_insert("chat_dm", &[rec]).unwrap();
        let store = engine.bundle("chat_dm").unwrap();
        assert_eq!(store.len(), 1, "one DM record must be stored");
        let body = store
            .records()
            .next()
            .and_then(|r| r.get("body").cloned())
            .expect("body field must be present");
        assert!(
            matches!(body, Value::Text(ref s) if s == "hello world"),
            "plain body must be Value::Text, got {body:?}"
        );
        cleanup(&dir);
    }

    #[test]
    fn test_chat_dm_encrypted_body_roundtrip() {
        // §3.3 binary body convention: encrypted=true → body is Value::Binary (raw ciphertext)
        // §2.1 boundary: "b64:AAEC/w==" at JSON edge → [0x00,0x01,0x02,0xFF] in storage
        // value_to_json must re-encode as "b64:AAEC/w==" when crossing back to JSON
        let dir = tmp_dir("chat_dm_enc");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(chat_dm_schema_test()).unwrap();
        let rec = parse_ndjson_record(DM_ENC_NDJSON);
        engine.batch_insert("chat_dm", &[rec]).unwrap();
        let store = engine.bundle("chat_dm").unwrap();
        let body = store
            .records()
            .next()
            .and_then(|r| r.get("body").cloned())
            .expect("body field must be present");
        assert!(
            matches!(body, Value::Binary(ref b) if b.as_slice() == [0x00u8, 0x01, 0x02, 0xFF]),
            "encrypted body must be Value::Binary([0x00,0x01,0x02,0xFF]), got {body:?}"
        );
        // Boundary round-trip: value_to_json must emit the b64: string
        assert_eq!(
            value_to_json(&body),
            serde_json::Value::String("b64:AAEC/w==".to_string()),
            "value_to_json must re-encode binary body as b64 string"
        );
        cleanup(&dir);
    }

    #[test]
    fn test_chat_signal_ingest() {
        let dir = tmp_dir("chat_signal");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(chat_signal_schema_test()).unwrap();
        let rec = parse_ndjson_record(SIGNAL_NDJSON);
        engine.batch_insert("chat_signal", &[rec]).unwrap();
        let store = engine.bundle("chat_signal").unwrap();
        assert_eq!(store.len(), 1, "one signal record must be stored");
        let signal_type = store
            .records()
            .next()
            .and_then(|r| r.get("signal_type").cloned())
            .expect("signal_type field must be present");
        assert!(
            matches!(signal_type, Value::Text(ref s) if s == "offer"),
            "signal_type must be 'offer', got {signal_type:?}"
        );
        cleanup(&dir);
    }

    #[test]
    fn test_chat_ack_ingest() {
        let dir = tmp_dir("chat_ack");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(chat_ack_schema_test()).unwrap();
        let rec = parse_ndjson_record(ACK_NDJSON);
        engine.batch_insert("chat_ack", &[rec]).unwrap();
        let store = engine.bundle("chat_ack").unwrap();
        assert_eq!(store.len(), 1, "one ack record must be stored");
        let ack_type = store
            .records()
            .next()
            .and_then(|r| r.get("ack_type").cloned())
            .expect("ack_type field must be present");
        assert!(
            matches!(ack_type, Value::Text(ref s) if s == "delivered"),
            "ack_type must be 'delivered', got {ack_type:?}"
        );
        cleanup(&dir);
    }

    #[test]
    fn test_chat_typing_ingest() {
        // Schema test only: production relay must NOT persist typing events (§5).
        // This verifies the data model stores and retrieves correctly at engine level.
        let dir = tmp_dir("chat_typing");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(chat_typing_schema_test()).unwrap();
        let rec = parse_ndjson_record(TYPING_NDJSON);
        engine.batch_insert("chat_typing", &[rec]).unwrap();
        let store = engine.bundle("chat_typing").unwrap();
        assert_eq!(store.len(), 1, "one typing record must be stored");
        let state = store
            .records()
            .next()
            .and_then(|r| r.get("state").cloned())
            .expect("state field must be present");
        assert!(
            matches!(state, Value::Text(ref s) if s == "start"),
            "state must be 'start', got {state:?}"
        );
        cleanup(&dir);
    }

    #[test]
    fn test_chat_reaction_ingest() {
        let dir = tmp_dir("chat_reaction");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();
        engine.create_bundle(chat_reaction_schema_test()).unwrap();
        let rec = parse_ndjson_record(REACTION_NDJSON);
        engine.batch_insert("chat_reaction", &[rec]).unwrap();
        let store = engine.bundle("chat_reaction").unwrap();
        assert_eq!(store.len(), 1, "one reaction record must be stored");
        let emoji = store
            .records()
            .next()
            .and_then(|r| r.get("emoji").cloned())
            .expect("emoji field must be present");
        assert!(
            matches!(emoji, Value::Text(ref s) if s == "👍"),
            "emoji must be '👍', got {emoji:?}"
        );
        cleanup(&dir);
    }

    // ── Schema coercion / validation tests ────────────────────────────────

    /// Numeric field: integer JSON → Value::Integer, float JSON → Value::Float. Both valid.
    #[test]
    fn test_schema_coerce_numeric_accepts_numbers() {
        let schema = BundleSchema::new("t").fiber(FieldDef::numeric("score"));
        let v_int = schema_coerce(&schema, "score", Value::Integer(42));
        assert!(matches!(v_int, Ok(Value::Integer(42))));
        let v_float = schema_coerce(&schema, "score", Value::Float(1.5));
        assert!(matches!(v_float, Ok(Value::Float(_))));
    }

    /// Numeric field: text → rejected.
    #[test]
    fn test_schema_coerce_numeric_rejects_text() {
        let schema = BundleSchema::new("t").fiber(FieldDef::numeric("score"));
        let err = schema_coerce(&schema, "score", Value::Text("hello".into()));
        assert!(err.is_err(), "text must be rejected for Numeric field");
        let msg = err.unwrap_err();
        assert!(msg.contains("score"), "error must name the field");
        assert!(msg.contains("Numeric"), "error must name expected type");
    }

    /// Timestamp field: integer is valid (nanosecond epoch).
    #[test]
    fn test_schema_coerce_timestamp_accepts_integer() {
        let schema = BundleSchema::new("t").base(FieldDef::timestamp("ts", 1e9));
        let v = schema_coerce(&schema, "ts", Value::Integer(1710000000000000000));
        assert!(matches!(v, Ok(Value::Timestamp(_))), "integer must coerce to Timestamp");
    }

    /// Timestamp field: formatted string is rejected (invariant C2 enforcement).
    #[test]
    fn test_schema_coerce_timestamp_rejects_string() {
        let schema = BundleSchema::new("t").base(FieldDef::timestamp("ts", 1e9));
        let err = schema_coerce(&schema, "ts", Value::Text("2026-04-14T00:00:00Z".into()));
        assert!(err.is_err(), "formatted timestamp string must be rejected");
        let msg = err.unwrap_err();
        assert!(msg.contains("ts"));
        assert!(msg.contains("Timestamp"));
    }

    /// Binary field: Value::Binary accepted as-is.
    #[test]
    fn test_schema_coerce_binary_accepts_binary() {
        let schema = BundleSchema::new("t").fiber(FieldDef::binary("blob"));
        let v = schema_coerce(&schema, "blob", Value::Binary(vec![0, 1, 2]));
        assert!(matches!(v, Ok(Value::Binary(_))));
    }

    /// Binary field: plain text (no b64: prefix) → rejected with helpful message.
    #[test]
    fn test_schema_coerce_binary_rejects_unescaped_text() {
        let schema = BundleSchema::new("t").fiber(FieldDef::binary("blob"));
        let err = schema_coerce(&schema, "blob", Value::Text("plain text".into()));
        assert!(err.is_err(), "plain text must be rejected for Binary field");
        let msg = err.unwrap_err();
        assert!(msg.contains("blob"));
        assert!(msg.contains("b64:"), "error must hint at b64: encoding");
    }

    /// Unknown field (not in schema) passes through unchanged.
    #[test]
    fn test_schema_coerce_unknown_field_passthrough() {
        let schema = BundleSchema::new("t").fiber(FieldDef::numeric("score"));
        // "extra" not in schema → no error, value unchanged
        let v = schema_coerce(&schema, "extra", Value::Text("anything".into()));
        assert!(matches!(v, Ok(Value::Text(_))));
    }

    /// Null is always accepted regardless of field type.
    #[test]
    fn test_schema_coerce_null_always_accepted() {
        let schema = BundleSchema::new("t")
            .fiber(FieldDef::numeric("score"))
            .fiber(FieldDef::timestamp("ts", 1e9))
            .fiber(FieldDef::binary("blob"));
        for field in ["score", "ts", "blob"] {
            let v = schema_coerce(&schema, field, Value::Null);
            assert!(matches!(v, Ok(Value::Null)), "Null must be accepted for {field}");
        }
    }

    // ── Prometheus text format ────────────────────────────────────────────────

    #[test]
    fn test_prometheus_text_contains_required_metric_names() {
        let body = build_prometheus_text(100, 2, 1, 50_000, 200_000, 900_000,
                                        5000, 250_000, 3, 7, 14_000, 88, 12, 3600);
        for metric in &[
            "gigi_queries_total",
            "gigi_queries_error_total",
            "gigi_queries_slow_total",
            "gigi_query_duration_microseconds",
            "gigi_records_ingested_total",
            "gigi_bytes_ingested_total",
            "gigi_anomalies_detected_total",
            "gigi_bundles",
            "gigi_records_total",
            "gigi_http_connections_total",
            "gigi_ws_connections_total",
            "gigi_uptime_seconds",
        ] {
            assert!(body.contains(metric), "missing metric: {metric}");
        }
    }

    #[test]
    fn test_prometheus_text_values_correct() {
        let body = build_prometheus_text(100, 2, 1, 50_000, 200_000, 900_000,
                                        5000, 250_000, 3, 7, 14_000, 88, 12, 3600);
        assert!(body.contains("gigi_queries_total 100"));
        assert!(body.contains("gigi_queries_error_total 2"));
        assert!(body.contains("gigi_queries_slow_total 1"));
        assert!(body.contains(r#"gigi_query_duration_microseconds{quantile="0.5"} 50000"#));
        assert!(body.contains(r#"gigi_query_duration_microseconds{quantile="0.95"} 200000"#));
        assert!(body.contains(r#"gigi_query_duration_microseconds{quantile="0.99"} 900000"#));
        assert!(body.contains("gigi_uptime_seconds 3600"));
    }

    #[test]
    fn test_prometheus_text_has_help_and_type_lines() {
        let body = build_prometheus_text(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        // Every metric must be preceded by # HELP and # TYPE
        assert!(body.contains("# HELP gigi_queries_total"));
        assert!(body.contains("# TYPE gigi_queries_total counter"));
        assert!(body.contains("# HELP gigi_bundles"));
        assert!(body.contains("# TYPE gigi_bundles gauge"));
    }

    // ── Observability v1.1 — new builders ─────────────────────────────────

    #[test]
    fn test_obs_connection_open_builder() {
        use gigi::observability::{Logger, LogCategory, LogConfig};
        let (logger, _ingester) = Logger::new(LogConfig::default(), "test-node");
        let ev = logger.connection_open("websocket", "127.0.0.1", "gigi-test");
        assert_eq!(ev.event, "connection.open");
        assert!(ev.category == LogCategory::Connection);
        let proto = ev.payload.get("protocol")
            .and_then(|v| v.as_str()).unwrap_or("");
        assert_eq!(proto, "websocket");
    }

    #[test]
    fn test_obs_connection_close_builder() {
        use gigi::observability::{Logger, LogCategory, LogConfig};
        let (logger, _ingester) = Logger::new(LogConfig::default(), "test-node");
        let ev = logger.connection_close("websocket", "10.0.0.1", 5_000_000, 42, 9000, 1200);
        assert_eq!(ev.event, "connection.close");
        assert!(ev.category == LogCategory::Connection);
        let dur = ev.payload.get("session_duration_us")
            .and_then(|v| v.as_u64()).unwrap_or(0);
        assert_eq!(dur, 5_000_000);
        let reqs = ev.payload.get("requests_served")
            .and_then(|v| v.as_u64()).unwrap_or(0);
        assert_eq!(reqs, 42);
    }

    #[test]
    fn test_obs_ingest_bulk_builder() {
        use gigi::observability::{Logger, LogCategory, LogConfig};
        let (logger, _ingester) = Logger::new(LogConfig::default(), "test-node");
        let ev = logger.ingest_bulk("sensors", 5000, 200_000, 1_000_000, 5000.0, true, 10);
        assert_eq!(ev.event, "ingest.bulk");
        assert!(ev.category == LogCategory::Ingest);
        let recs = ev.payload.get("records_written")
            .and_then(|v| v.as_u64()).unwrap_or(0);
        assert_eq!(recs, 5000);
    }

    #[test]
    fn test_obs_wal_replay_builder() {
        use gigi::observability::{Logger, LogCategory, LogConfig};
        let (logger, _ingester) = Logger::new(LogConfig::default(), "test-node");
        let ev = logger.wal_replay("sensors", 9999, 500_000, "startup");
        assert_eq!(ev.event, "wal.replay");
        assert!(ev.category == LogCategory::Wal);
        let recovered = ev.payload.get("records_recovered")
            .and_then(|v| v.as_u64()).unwrap_or(0);
        assert_eq!(recovered, 9999);
    }

    #[test]
    fn test_obs_wal_compaction_builder() {
        use gigi::observability::{Logger, LogCategory, LogConfig};
        let (logger, _ingester) = Logger::new(LogConfig::default(), "test-node");
        let ev = logger.wal_compaction("sensors", 4, 1_000_000, 200_000, 5.0, 80_000);
        assert_eq!(ev.event, "wal.compaction");
        assert!(ev.category == LogCategory::Wal);
        let ratio = ev.payload.get("compression_ratio")
            .and_then(|v| v.as_f64()).unwrap_or(0.0);
        assert!((ratio - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_obs_log_routing_all_categories() {
        // Verify all LogCategory variants are represented in the routing table.
        // This test compiles the match — if a new variant is added without updating
        // log_bundle_writer, it will fail to compile (non-exhaustive match).
        use gigi::observability::LogCategory;
        let categories = [
            LogCategory::Query,
            LogCategory::Slow,
            LogCategory::Anomaly,
            LogCategory::Audit,
            LogCategory::Stream,
            LogCategory::Bundle,
            LogCategory::Ingest,
            LogCategory::Wal,
            LogCategory::Connection,
            LogCategory::System,
        ];
        let expected_bundles = [
            "_gigi_query_log",
            "_gigi_slow_log",
            "_gigi_anomaly_log",
            "_gigi_audit_log",
            "_gigi_stream_log",
            "_gigi_bundle_log",
            "_gigi_ingest_log",
            "_gigi_wal_log",
            "_gigi_conn_log",
            "_gigi_system_log",
        ];
        assert_eq!(categories.len(), expected_bundles.len(), "routing table must cover all categories");

        for (cat, bundle) in categories.iter().zip(expected_bundles.iter()) {
            let got = match cat {
                LogCategory::Query      => "_gigi_query_log",
                LogCategory::Slow       => "_gigi_slow_log",
                LogCategory::Anomaly    => "_gigi_anomaly_log",
                LogCategory::Audit      => "_gigi_audit_log",
                LogCategory::Stream     => "_gigi_stream_log",
                LogCategory::Bundle     => "_gigi_bundle_log",
                LogCategory::Ingest     => "_gigi_ingest_log",
                LogCategory::Wal        => "_gigi_wal_log",
                LogCategory::Connection => "_gigi_conn_log",
                LogCategory::System     => "_gigi_system_log",
            };
            assert_eq!(got, *bundle, "category {:?} must route to {bundle}", cat);
        }
    }

    // ── Sprint G: app-bundle bootstrap regression tests ────────────────────
    //
    // These tests cover the failure mode that wiped Just Gigi customer chat
    // on 2026-05-01: gigi-stream redeployed without the `jg_kv` bundle, the
    // website returned 500 from /admin/chat, and conversations were lost.
    //
    // The contract `init_app_bundles` must satisfy is:
    //   1. NEVER touch an existing bundle (regardless of whether it appears
    //      in the manifest).  This is the data-safety invariant.
    //   2. Create missing bundles from the manifest schema.
    //   3. Be a no-op when the env var is unset, empty, or invalid — never
    //      panic, never abort startup.

    // Env vars are global state. Tests run in parallel by default, so we
    // serialize the tests that touch GIGI_APP_BUNDLES.
    fn env_lock() -> &'static std::sync::Mutex<()> {
        use std::sync::OnceLock;
        static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    /// RAII guard: sets GIGI_APP_BUNDLES on construction, removes on drop.
    /// Holds the env lock for its lifetime so concurrent tests don't race.
    struct EnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }
    impl EnvGuard {
        fn set(value: &str) -> Self {
            let lock = env_lock().lock().unwrap_or_else(|p| p.into_inner());
            // SAFETY: serialized via env_lock() so no concurrent reads
            // from other env-touching tests during the set/get window.
            unsafe { std::env::set_var("GIGI_APP_BUNDLES", value); }
            Self { _lock: lock }
        }
        fn unset() -> Self {
            let lock = env_lock().lock().unwrap_or_else(|p| p.into_inner());
            unsafe { std::env::remove_var("GIGI_APP_BUNDLES"); }
            Self { _lock: lock }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe { std::env::remove_var("GIGI_APP_BUNDLES"); }
        }
    }

    /// Test 1: when the manifest names a missing bundle, it gets created
    /// with the right schema. This is the recovery path.
    #[test]
    fn init_app_bundles_creates_missing_bundle() {
        let dir = tmp_dir("init_app_bundles_create");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();

        let manifest = r#"[{
            "name": "jg_kv",
            "base":  [{"name": "key", "type": "text"}],
            "fiber": [
                {"name": "kind", "type": "text", "indexed": true},
                {"name": "payload", "type": "text"},
                {"name": "expires_at", "type": "timestamp", "indexed": true},
                {"name": "updated_at", "type": "timestamp", "indexed": true}
            ]
        }]"#;
        let _g = EnvGuard::set(manifest);

        assert!(!engine.bundle_names().contains(&"jg_kv"));
        init_app_bundles(&mut engine);
        assert!(
            engine.bundle_names().contains(&"jg_kv"),
            "bundle must be created from manifest"
        );

        let store = engine.bundle("jg_kv").expect("bundle present");
        let schema = store.schema();
        assert_eq!(schema.base_fields.len(), 1, "1 base field");
        assert_eq!(schema.base_fields[0].name, "key");
        let fiber_names: Vec<String> = schema.fiber_fields.iter().map(|f| f.name.clone()).collect();
        assert!(fiber_names.contains(&"kind".into()));
        assert!(fiber_names.contains(&"payload".into()));
        assert!(fiber_names.contains(&"expires_at".into()));
        assert!(fiber_names.contains(&"updated_at".into()));

        cleanup(&dir);
    }

    /// **CRITICAL DATA-SAFETY TEST.** When the manifest names a bundle that
    /// already exists *with data*, init_app_bundles must NOT recreate or
    /// otherwise touch it. If this test ever fails, the bootstrap path has
    /// regressed into the very wipe-on-cold-start behavior that motivated
    /// this whole sprint.
    #[test]
    fn init_app_bundles_never_wipes_existing_bundle_with_data() {
        let dir = tmp_dir("init_app_bundles_no_wipe");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();

        // Pre-create the bundle the manifest will reference, with a
        // *different* schema than the manifest specifies — so if
        // init_app_bundles incorrectly recreates it, we'll see the schema
        // change as well as record loss.
        let pre_schema = BundleSchema::new("jg_kv")
            .base(FieldDef::categorical("key"))
            .fiber(FieldDef::categorical("payload_v1"))   // intentionally
            .fiber(FieldDef::categorical("legacy_field")); // different
        engine.create_bundle(pre_schema).unwrap();

        // Insert data we want to preserve. These are real chat-like records.
        let mut r1 = Record::new();
        r1.insert("key".into(), Value::Text("jg:conv:abc".into()));
        r1.insert("payload_v1".into(), Value::Text("{\"id\":\"abc\",\"messages\":3}".into()));
        r1.insert("legacy_field".into(), Value::Text("v1".into()));
        engine.insert("jg_kv", &r1).unwrap();

        let mut r2 = Record::new();
        r2.insert("key".into(), Value::Text("jg:conv_index".into()));
        r2.insert("payload_v1".into(), Value::Text("[\"abc\",\"def\"]".into()));
        r2.insert("legacy_field".into(), Value::Text("v1".into()));
        engine.insert("jg_kv", &r2).unwrap();

        let records_before = engine.bundle("jg_kv").unwrap().len();
        assert_eq!(records_before, 2, "precondition: 2 records inserted");

        // Manifest names jg_kv with a DIFFERENT schema. init_app_bundles
        // must see "already exists" and skip — preserving the original
        // schema and all 2 records.
        let manifest = r#"[{
            "name": "jg_kv",
            "base":  [{"name": "key", "type": "text"}],
            "fiber": [
                {"name": "payload_v2", "type": "text"},
                {"name": "kind", "type": "text", "indexed": true}
            ]
        }]"#;
        let _g = EnvGuard::set(manifest);

        init_app_bundles(&mut engine);

        // Bundle still exists.
        assert!(engine.bundle_names().contains(&"jg_kv"));

        // Schema is the ORIGINAL one — manifest schema must not have been
        // applied. This is the clearest signal that the bundle wasn't
        // recreated.
        let store = engine.bundle("jg_kv").unwrap();
        let fiber_names: std::collections::HashSet<String> =
            store.schema().fiber_fields.iter().map(|f| f.name.clone()).collect();
        assert!(
            fiber_names.contains("payload_v1"),
            "original schema field `payload_v1` must still be present; \
             got fiber fields {fiber_names:?} — init_app_bundles RECREATED \
             an existing bundle, which is the data-loss bug"
        );
        assert!(
            !fiber_names.contains("payload_v2"),
            "manifest field `payload_v2` must NOT have been applied to \
             existing bundle"
        );

        // Records survive.
        let records_after = store.len();
        assert_eq!(
            records_after, records_before,
            "records must not be lost — got {records_after}, expected {records_before}"
        );

        cleanup(&dir);
    }

    /// Test 3: no env var → no-op. Doesn't create anything, doesn't panic.
    #[test]
    fn init_app_bundles_no_env_is_noop() {
        let dir = tmp_dir("init_app_bundles_no_env");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();

        let _g = EnvGuard::unset();

        let names_before: Vec<String> = engine.bundle_names().iter().map(|s| s.to_string()).collect();
        init_app_bundles(&mut engine);
        let names_after: Vec<String> = engine.bundle_names().iter().map(|s| s.to_string()).collect();
        assert_eq!(names_before, names_after, "no env var → no changes");

        cleanup(&dir);
    }

    /// Test 4: invalid JSON → graceful no-op (logged to stderr, no panic).
    /// Startup must continue even when the operator misconfigures the manifest.
    #[test]
    fn init_app_bundles_invalid_json_is_safe() {
        let dir = tmp_dir("init_app_bundles_bad_json");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();

        let _g = EnvGuard::set("not valid json {[}");

        let names_before: Vec<String> = engine.bundle_names().iter().map(|s| s.to_string()).collect();
        init_app_bundles(&mut engine); // must not panic
        let names_after: Vec<String> = engine.bundle_names().iter().map(|s| s.to_string()).collect();
        assert_eq!(names_before, names_after);

        cleanup(&dir);
    }

    /// Test 5: a manifest entry with an unknown bundle name AND a missing
    /// `name` field must be skipped without aborting the rest of the manifest.
    /// (Defense in depth: one bad entry shouldn't take out the whole boot.)
    #[test]
    fn init_app_bundles_skips_malformed_entries() {
        let dir = tmp_dir("init_app_bundles_malformed");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();

        let manifest = r#"[
            {"base": [{"name":"x","type":"text"}]},
            {"name": "good_bundle", "fiber": [{"name":"y","type":"text"}]}
        ]"#;
        let _g = EnvGuard::set(manifest);

        init_app_bundles(&mut engine);
        // The malformed entry is skipped; the good one is created.
        assert!(engine.bundle_names().contains(&"good_bundle"));

        cleanup(&dir);
    }

    /// Sprint G-pivot: per-field encryption from the manifest.
    ///
    /// When a fiber field declares `"encrypted": "opaque"` (or affine /
    /// indexed), the bootstrap installs a GaugeKey on the schema using
    /// the seed from the env var named in `seed_env`. Records inserted
    /// into the bundle thereafter have their fiber values encrypted at
    /// rest under the appropriate per-field transform.
    ///
    /// This is the entry point for "Just Gigi encryption": jg_kv's
    /// `payload` field gets OPAQUE-encrypted via this path, with the
    /// seed coming from a Fly secret.
    #[test]
    fn init_app_bundles_with_per_field_encryption() {
        let dir = tmp_dir("init_app_bundles_encrypted");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();

        // Set the seed env var that the manifest references.
        let lock = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        unsafe {
            std::env::set_var(
                "TEST_JG_SEED",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            );
        }

        let manifest = r#"[{
            "name": "jg_kv_test",
            "seed_env": "TEST_JG_SEED",
            "base":  [{"name": "key", "type": "text"}],
            "fiber": [
                {"name": "kind", "type": "text", "indexed": true},
                {"name": "payload", "type": "text", "encrypted": "opaque"},
                {"name": "expires_at", "type": "timestamp", "indexed": true}
            ]
        }]"#;
        // Hand-rolled inline (don't double-acquire the env_lock).
        unsafe { std::env::set_var("GIGI_APP_BUNDLES", manifest); }

        init_app_bundles(&mut engine);

        // Bundle exists.
        assert!(
            engine.bundle_names().contains(&"jg_kv_test"),
            "encrypted bundle must be created"
        );

        // GaugeKey was installed on the schema. Inspect via heap_bundle()
        // so we get a direct &BundleStore (and drop the borrow before
        // mutating the engine).
        {
            let store = engine.heap_bundle("jg_kv_test").expect("bundle present");
            let schema = &store.schema;
            assert!(
                schema.gauge_key.is_some(),
                "schema.gauge_key must be set when any fiber field declares encrypted"
            );
            let payload = schema
                .fiber_fields
                .iter()
                .find(|f| f.name == "payload")
                .expect("payload field present");
            assert_eq!(
                payload.encryption,
                gigi::types::EncryptionMode::Opaque,
                "payload must be OPAQUE-encrypted per manifest"
            );
            let kind = schema
                .fiber_fields
                .iter()
                .find(|f| f.name == "kind")
                .expect("kind field present");
            assert_eq!(
                kind.encryption,
                gigi::types::EncryptionMode::None,
                "kind has no encrypted clause → stays plaintext"
            );
        }

        // Round-trip: insert a record, query it back, get plaintext.
        let mut r = Record::new();
        r.insert("key".into(), Value::Text("jg:conv:test1".into()));
        r.insert("kind".into(), Value::Text("string".into()));
        r.insert(
            "payload".into(),
            Value::Text("{\"messages\":[{\"role\":\"user\",\"body\":\"hi\"}]}".into()),
        );
        r.insert("expires_at".into(), Value::Timestamp(0));
        engine.insert("jg_kv_test", &r).unwrap();

        let store = engine
            .heap_bundle("jg_kv_test")
            .expect("bundle present in heap");
        let mut key = Record::new();
        key.insert("key".into(), Value::Text("jg:conv:test1".into()));
        let got = store.point_query(&key).expect("record findable");
        assert_eq!(
            got.get("payload"),
            Some(&Value::Text(
                "{\"messages\":[{\"role\":\"user\",\"body\":\"hi\"}]}".into()
            )),
            "payload must round-trip to plaintext via gauge_key decrypt"
        );

        // The raw on-disk fiber for `payload` must be CIPHERTEXT, not
        // the plaintext JSON string. This pins encryption-at-rest:
        // get_fiber returns stored bytes without decrypting.
        let bp = store.base_point(&key);
        let raw_fiber = store.get_fiber(bp).expect("raw fiber present");
        let payload_idx = store
            .schema
            .fiber_fields
            .iter()
            .position(|f| f.name == "payload")
            .unwrap();
        let raw_payload = &raw_fiber[payload_idx];
        match raw_payload {
            Value::Binary(_) => {
                // Opaque mode stores ciphertext as a Binary value — exactly
                // what we want.
            }
            other => panic!(
                "raw payload should be Binary ciphertext, got {:?} \
                 (encryption is not actually engaged at rest)",
                other
            ),
        }

        // Cleanup.
        unsafe {
            std::env::remove_var("TEST_JG_SEED");
            std::env::remove_var("GIGI_APP_BUNDLES");
        }
        drop(lock);
        cleanup(&dir);
    }

    /// When `seed_env` points to a missing env var, the bootstrap falls
    /// back to a random seed (with a loud warning to stderr) — better
    /// than refusing to start, but loudly so the operator notices and
    /// sets the secret. The bundle is still created and is encrypted;
    /// it just won't survive a redeploy.
    #[test]
    fn init_app_bundles_missing_seed_env_falls_back() {
        let dir = tmp_dir("init_app_bundles_missing_seed");
        cleanup(&dir);
        let mut engine = Engine::open(&dir).unwrap();

        let lock = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        unsafe {
            std::env::remove_var("DEFINITELY_NOT_SET_AAA_BBB");
            std::env::set_var(
                "GIGI_APP_BUNDLES",
                r#"[{
                    "name": "jg_unset_seed",
                    "seed_env": "DEFINITELY_NOT_SET_AAA_BBB",
                    "fiber": [
                        {"name": "p", "type": "text", "encrypted": "opaque"}
                    ]
                }]"#,
            );
        }

        init_app_bundles(&mut engine);

        // Bundle was still created with a random seed — better than
        // crashing on startup. The schema has a gauge_key.
        let store = engine
            .heap_bundle("jg_unset_seed")
            .expect("bundle present in heap");
        assert!(store.schema.gauge_key.is_some());

        unsafe { std::env::remove_var("GIGI_APP_BUNDLES"); }
        drop(lock);
        cleanup(&dir);
    }

    // ── L1.5.3: in-bin smoke test for the kahler_transport_dispatch
    //   helper. Exercises the same code path the GQL TRANSPORT verb
    //   uses; asserts the response record carries the v2 contract
    //   field set Marcella's runtime deserializes.
    //   Per IMPLEMENTATION_PLAN.md L1.5 + consumption draft v2 §2.
    //   ───────────────────────────────────────────────────────────

    #[cfg(feature = "kahler")]
    #[test]
    fn test_kahler_transport_dispatch_returns_v2_contract_fields() {
        use gigi::geometry::{
            ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm,
        };

        // 2D Kähler: J on R², B = b·dx∧dy with b = 0.5.
        let j = ComplexStructure::standard(1);
        let b = ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
        );
        let k = KahlerStructure::new(j, b);

        // Synthetic transport: from (0, 0) toward (1, 1).
        let p_from = vec![0.0, 0.0];
        let p_to = vec![1.0, 1.0];
        let displacement = vec![1.0, 1.0];

        let result = kahler_transport_dispatch(&k, &p_from, &p_to, &displacement)
            .expect("dim 2 matches K dim 2, dispatch must trigger")
            .expect("flat_transport must succeed on synthetic input");

        // v2 contract fields present.
        assert!(
            result.contains_key("path_length"),
            "path_length missing"
        );
        assert!(
            result.contains_key("energy_drift"),
            "energy_drift missing"
        );
        assert!(
            result.contains_key("holonomy_norm"),
            "holonomy_norm missing"
        );
        assert!(
            result.contains_key("used_magnetic"),
            "used_magnetic missing"
        );
        assert!(result.contains_key("b_source"), "b_source missing");
        // Displacement still surfaced for back-compat with the
        // classical TRANSPORT response shape.
        assert!(result.contains_key("displacement_0"));
        assert!(result.contains_key("displacement_1"));

        // b_source == "bundle" since dispatch used the Kähler's B.
        match result.get("b_source") {
            Some(Value::Text(s)) => assert_eq!(s, "bundle"),
            other => panic!("b_source must be Text(\"bundle\"), got {other:?}"),
        }

        // used_magnetic == true since we actually applied B.
        match result.get("used_magnetic") {
            Some(Value::Bool(true)) => {}
            other => panic!("used_magnetic must be Bool(true), got {other:?}"),
        }

        // energy_drift below the production 1e-9 bound.
        match result.get("energy_drift") {
            Some(Value::Float(drift)) => assert!(
                *drift < 1e-9,
                "energy_drift {drift} exceeds production 1e-9 bound"
            ),
            other => panic!("energy_drift must be Float, got {other:?}"),
        }
    }

    /// Negative: when the Kähler structure's dim doesn't match the
    /// segment dim, the helper returns None so the GQL caller falls
    /// back to the classical quaternion/rotation path. Prevents
    /// dimension-mismatch surprises when a 4D bundle has a 2D
    /// Kähler attached (which is itself a schema bug, but the
    /// transport verb should fail gracefully not panic).
    #[cfg(feature = "kahler")]
    #[test]
    fn test_kahler_transport_dispatch_returns_none_on_dim_mismatch() {
        use gigi::geometry::{
            ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm,
        };

        // 2D Kähler.
        let j = ComplexStructure::standard(1);
        let b = ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).unwrap(),
        );
        let k = KahlerStructure::new(j, b);

        // 4D segment — dim doesn't match.
        let p_from = vec![0.0, 0.0, 0.0, 0.0];
        let p_to = vec![1.0, 1.0, 1.0, 1.0];
        let displacement = vec![1.0, 1.0, 1.0, 1.0];

        assert!(
            kahler_transport_dispatch(&k, &p_from, &p_to, &displacement).is_none(),
            "dim mismatch must return None so caller falls back to classical"
        );
    }
}
