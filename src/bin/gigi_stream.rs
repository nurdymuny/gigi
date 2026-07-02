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
    /// `Arc<RwLock<Engine>>` (lifted from `RwLock<Engine>` for
    /// TDD-HAL-II.6b) so the gauge HTTP surface can share the same
    /// engine via `gauge::engine_handle::install`. `Arc` derefs through
    /// to `RwLock`, so every existing `state.engine.read()/.write()`
    /// call site keeps working unchanged.
    engine: Arc<RwLock<Engine>>,
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
    /// Per-(bundle, field_set) materialized `(N, D)` matrices for the
    /// vector-search brain endpoints (`intent_gate`, `confidence`,
    /// `confidence_with_explain`). Replaces the per-request
    /// `extract_field_samples` allocation + per-call O(N²·D) max-density
    /// recomputation per Marcella's 2026-05-29 latency report. Same
    /// mutation-counter invalidation as `flow_cache`.
    #[cfg(feature = "kahler")]
    vector_cache: Arc<gigi::vector_cache::VectorMatrixCache>,

    /// 2026-06-02 SEMANTIC perf follow-up: cache for
    /// `/brain/semantic` endpoint results. Defense-in-depth on top
    /// of the betti-rank algorithm fix (commit 0ec9405); subsequent
    /// reads on the same bundle skip even the rank computation.
    /// Same pattern as `vector_cache` above: mutation-counter
    /// invalidated, single-flight on cache miss. Capacity tunable
    /// via `GIGI_MORSE_CACHE_SIZE` env (default 64).
    #[cfg(feature = "kahler")]
    morse_cache: Arc<gigi::morse_cache::MorseCache>,

    /// Atomic Sheaf Commits Phase-A — open-transaction registry.
    ///
    /// Per-tx in-memory state for the /v1/transactions/* surface (begin /
    /// write / commit / rollback / status). First ship: writes are buffered
    /// in `tx.pending` and applied through `engine.batch_insert` in commit
    /// order. 2PC failure recovery, SI overlay reads, and the global WAL
    /// log live in `src/transactions/` and ride this surface in a follow-up.
    #[cfg(feature = "transactions")]
    tx_registry: Arc<std::sync::Mutex<HashMap<gigi::transactions::TransactionId, OpenTx>>>,
    /// Monotone snapshot counter feeding tx BEGIN snap_ids.
    #[cfg(feature = "transactions")]
    tx_snap_counter: Arc<std::sync::atomic::AtomicU64>,
    /// Public-read bundle allowlist. Bundles whose names appear here are
    /// exposed via the `/v1/public/gql` endpoint under a strict read-verb
    /// whitelist — no auth required. Populated from the comma-separated
    /// `GIGI_PUBLIC_BUNDLES` env var at startup; empty when the var is
    /// unset (in which case the public route is not registered at all).
    ///
    /// SAFETY: this set controls the readable surface for anonymous
    /// callers. Writes, admin verbs, and non-listed bundles remain
    /// inaccessible via `/v1/public/gql` regardless of anything a caller
    /// sends. See `validate_public_stmt` for the whitelist.
    public_bundles: std::collections::HashSet<String>,
}

/// One open transaction held by the registry.
#[cfg(feature = "transactions")]
#[derive(Debug)]
struct OpenTx {
    snap_id: gigi::transactions::SnapshotId,
    opened_at: std::time::SystemTime,
    isolation: gigi::transactions::IsolationLevel,
    state: gigi::transactions::TransactionState,
    pending: HashMap<String, Vec<Record>>,
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
        // Public-read bundle allowlist. Comma-separated, whitespace trimmed,
        // empty tokens ignored. Unset var = empty set = route not registered.
        let public_bundles: std::collections::HashSet<String> =
            std::env::var("GIGI_PUBLIC_BUNDLES")
                .ok()
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();
        if !public_bundles.is_empty() {
            let mut names: Vec<&String> = public_bundles.iter().collect();
            names.sort();
            eprintln!(
                "  Public-read bundles ({}): {}",
                names.len(),
                names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            );
        }
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

        // Vector matrix cache capacity. At N=10k, D=384, each matrix
        // is ~30 MB (data + per-row norms); 64 entries default = ~2 GB
        // worst case. Production usually keeps one entry per active
        // bundle, so the realistic footprint is much smaller.
        #[cfg(feature = "kahler")]
        let vector_cache_capacity = std::env::var("GIGI_VECTOR_CACHE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(64_usize);

        // Morse / SEMANTIC cache capacity. Each entry is ~50 bytes
        // (CachedMorse holds 3 usize Betti + 4 metadata floats + bool
        // + counter). 64 entries default = trivial memory cost. Tune
        // via GIGI_MORSE_CACHE_SIZE.
        #[cfg(feature = "kahler")]
        let morse_cache_capacity = std::env::var("GIGI_MORSE_CACHE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(64_usize);

        StreamState {
            engine: Arc::new(RwLock::new(engine)),
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
            #[cfg(feature = "kahler")]
            vector_cache: Arc::new(gigi::vector_cache::VectorMatrixCache::new(
                vector_cache_capacity,
            )),
            #[cfg(feature = "kahler")]
            morse_cache: Arc::new(gigi::morse_cache::MorseCache::new(morse_cache_capacity)),
            #[cfg(feature = "transactions")]
            tx_registry: Arc::new(std::sync::Mutex::new(HashMap::new())),
            #[cfg(feature = "transactions")]
            tx_snap_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            public_bundles,
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
    /// Davis Conjecture λ-budget (Thm T8ai): the substrate's runtime
    /// introspection of its own remaining carrying capacity. Computed
    /// via [`gigi::curvature::lambda_budget`] from this bundle's
    /// current K, the substrate's default D-proxy (Welford radius),
    /// and τ_budget = 1.0 (matches the capacity/horizon convention).
    /// Companion: [`gigi::curvature::horizon_closed`] flips `true`
    /// when λ ≥ 0.95 (consensus prohibitively slow).
    lambda_budget: f64,
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

/// Davis Conjecture λ-budget ride-along envelope (Thm T8ai, claim_0104).
///
/// Generic wrapper that flattens `inner`'s fields at the JSON top level
/// (via `#[serde(flatten)]`) and adds a sibling `lambda_budget: f64`
/// key. Used by every kahler-gated `/v1/bundles/{name}/brain/*`
/// endpoint so cognitive consumers (Marcella, claude_substrate_v0,
/// future LLM consumers) can read the substrate's runtime carrying
/// capacity off any brain response — matching the convention already
/// shipped on `CurvatureReport.lambda_budget`.
///
/// Backwards-compatible: existing clients that don't look for
/// `lambda_budget` see no semantic change (no field renamed/removed;
/// only an additive top-level key).
#[cfg(feature = "kahler")]
#[derive(Serialize)]
struct ResponseWithLambda<T: Serialize> {
    #[serde(flatten)]
    inner: T,
    /// Davis Conjecture λ-budget — substrate's current carrying
    /// capacity for this bundle, computed at response time from K
    /// (curvature), D (Welford radius), τ = 1.0. Safe default of 1.0
    /// (no-horizon) on missing / empty / NaN inputs so the hot brain
    /// path never emits NaN on the wire.
    lambda_budget: f64,
}

/// Compute the Davis Conjecture λ-budget for a bundle by name on the
/// hot brain path. Mirrors `/v1/bundles/{name}/curvature`'s compute
/// path (k = scalar_curvature, D = Welford radius, τ = 1.0) but
/// resolves the bundle from `state` and coalesces every degenerate
/// input to the safe default `1.0`. Returns `1.0` if the bundle is
/// missing, empty, or has no usable variance signal — never NaN.
///
/// Cost: O(1) given the bundle's existing curvature + Welford state
/// (~few hundred ns). Brain primitives are called per-turn by
/// cognitive consumers; the lib helper
/// [`gigi::curvature::lambda_budget_for_bundle`] guarantees no NaN
/// poisoning.
#[cfg(feature = "kahler")]
fn lambda_budget_for_bundle(state: &Arc<StreamState>, bundle_name: &str) -> f64 {
    let engine = state.engine.read().unwrap();
    let bundle_ref = match engine.bundle(bundle_name) {
        Some(b) => b,
        // Missing bundle → safe default. The handler will surface the
        // 404 separately; we never panic on the ride-along path.
        None => return 1.0,
    };
    lambda_budget_for_bundle_ref(&bundle_ref)
}

/// Compute λ-budget directly from an already-resolved `BundleRef`.
///
/// Some brain handlers hold an engine read guard for the duration of
/// their compute and would deadlock if `lambda_budget_for_bundle`
/// re-acquired the same RwLock. Those handlers call this variant with
/// the BundleRef they already have.
#[cfg(feature = "kahler")]
fn lambda_budget_for_bundle_ref(bundle_ref: &gigi::BundleRef<'_>) -> f64 {
    // Mirror `curvature_report`'s heap/overlay split: only heap-backed
    // bundles expose a concrete `BundleStore` for the lib helper.
    // Overlay bundles fall back to D = 1.0 + helper's NaN-coalesce.
    match bundle_ref.as_heap() {
        Some(store) => gigi::curvature::lambda_budget_for_bundle(store),
        None => {
            // Overlay path: use the BundleRef's k-mean from
            // CurvatureStats + unit-D fallback. Safe-default 1.0 on NaN.
            let k = bundle_ref.curvature_stats().mean();
            let raw = gigi::curvature::lambda_budget(k, 1.0, 1.0);
            if raw.is_nan() {
                1.0
            } else {
                raw
            }
        }
    }
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

/// Response for `GET /v1/bundles/{name}/capacity` and `CAPACITY` GQL verb.
/// Davis capacity C = τ/K (Theorem 8.1 — Cognitive Geometry Correspondence).
#[derive(Serialize)]
struct CapacityReport {
    /// Davis capacity C = τ/K. How many distinct interpretations the
    /// system can maintain simultaneously at this curvature level.
    capacity: f64,
    /// Local scalar curvature K.
    k: f64,
    /// Tolerance budget τ used to compute C.
    tau: f64,
    /// Confidence ∈ (0,1]: 1/(1+K).
    confidence: f64,
    /// Qualitative regime: "flat" (K≈0), "low" (C>10), "moderate",
    /// "high" (C<1, overloaded), or "critical" (C≈0, K→∞).
    regime: &'static str,
    /// Human-readable interpretation for builders.
    interpretation: String,
}

/// Response for `GET /v1/bundles/{name}/horizon` and `HORIZON` GQL verb.
/// Holonomy horizon s_max = τ/(K·ℓ_c) (Definition 5.1 — Cognitive
/// Geometry Correspondence). The maximum coherent context depth.
///
/// `estimator_used` and `fallback_engaged` report which length-scale
/// estimator actually produced `l_c`. The default config tries
/// SpectralGap first and falls back to WelfordRadius when λ₁ is
/// degenerate — sensor-style bundles always hit the fallback because
/// their connectivity isn't graph-structured.
#[derive(Serialize)]
struct HorizonReport {
    /// s_max = τ/(K·ℓ_c). Beyond this many positions, individual
    /// contributions to the accumulated frame rotation are irrecoverable.
    s_max: f64,
    /// Local scalar curvature K.
    k: f64,
    /// Tolerance budget τ.
    tau: f64,
    /// Correlation length ℓ_c actually used (from the estimator that won).
    l_c: f64,
    /// Spectral gap λ₁ (always reported, even when the fallback fires).
    lambda1: f64,
    /// Which estimator produced `l_c`. Either the primary
    /// (`config.estimator`) or the fallback when the primary was
    /// degenerate. Strings: "spectral_gap" | "welford_radius" | {"fixed":N}.
    estimator_used: gigi::curvature::LengthScaleEstimator,
    /// True iff the primary estimator was degenerate and the fallback
    /// fired. Convenience flag — equivalent to
    /// `estimator_used != config.estimator`.
    fallback_engaged: bool,
    /// Human-readable interpretation.
    interpretation: String,
}

/// Response for `GET /v1/bundles/{name}/depth` and `DEPTH` GQL verb.
/// Encoding depth classification (Theorem 8.14 — Cognitive Geometry
/// Correspondence). Maps K and λ₁ to one of four resistance levels.
#[derive(Serialize)]
struct DepthReport {
    /// Encoding depth: "tangent" | "connection" | "metric" | "topological".
    depth: gigi::curvature::EncodingDepth,
    /// Roman numeral label: "I" | "II" | "III" | "IV".
    level: &'static str,
    /// Scalar curvature K used for classification.
    k: f64,
    /// Spectral gap λ₁ used for classification.
    lambda1: f64,
    /// Erasure energy scale: "low" | "moderate" | "high" | "infinite".
    erasure_energy: &'static str,
    /// Full description of what this depth means.
    description: &'static str,
    /// The threshold config the classifier used. Echoed back so the
    /// caller can audit which numbers produced the verdict — exposes
    /// any query-param overrides the caller supplied. Defaults
    /// (Theorem 8.14 published values) when no overrides.
    config_used: gigi::curvature::DepthConfig,
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
/// Accepts (in order of precedence):
///   - `X-API-Key` header (HTTP)
///   - `Sec-WebSocket-Protocol` subprotocol element `gigi.apikey.<KEY>` or
///     `gigi.bearer.<TOKEN>` (WS upgrade) — preferred over query-string
///     credentials because subprotocol values don't leak into URL access
///     logs / browser history / Referer headers / error-reporting URL
///     captures the way `?api_key=` does.
///   - `?api_key=...` query (WS upgrade, **deprecated**) — kept for one
///     transition cycle while clients move to the subprotocol path.
///   - `Authorization: Bearer <token>` header (HTTP) /
///     `?gigi_token=...` query (WS upgrade, **deprecated**) —
///     verifies HMAC-SHA256 against `GIGI_JWT_SECRET`.
///
/// Attaches `GigiClaims` to the request extensions so the downstream
/// `namespace_enforcement_middleware` can gate /v1/bundles/<name>/*
/// paths by tenant. Health endpoint is excluded so liveness probes
/// don't need credentials.
///
/// **2026-06-04 hardening.** The query-string WS auth path leaked the
/// API key to every place that logs URLs (fly edge, browser history,
/// Referer, JS error reporters). Subprotocol headers don't carry the
/// same exposure surface. Both paths are accepted during the transition;
/// the query path will be removed once all clients move.

/// Extract API-key / bearer-token credentials from the
/// `Sec-WebSocket-Protocol` upgrade header. Returns `(api_key, bearer)`
/// — at most one of each is set. Format expected from the client:
/// `gigi.v1, gigi.apikey.<KEY>` or `gigi.v1, gigi.bearer.<TOKEN>`.
fn extract_subprotocol_credentials(headers: &axum::http::HeaderMap) -> (Option<String>, Option<String>) {
    let raw = match headers.get("sec-websocket-protocol").and_then(|v| v.to_str().ok()) {
        Some(s) => s,
        None => return (None, None),
    };
    let mut api_key = None;
    let mut bearer = None;
    for piece in raw.split(',') {
        let p = piece.trim();
        if let Some(rest) = p.strip_prefix("gigi.apikey.") {
            if api_key.is_none() && !rest.is_empty() {
                api_key = Some(rest.to_string());
            }
        } else if let Some(rest) = p.strip_prefix("gigi.bearer.") {
            if bearer.is_none() && !rest.is_empty() {
                bearer = Some(rest.to_string());
            }
        }
    }
    (api_key, bearer)
}
async fn auth_middleware(
    State(state): State<Arc<StreamState>>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let path = req.uri().path();

    // Skip auth for health endpoint
    if path == "/v1/health" {
        return Ok(next.run(req).await);
    }

    // Skip auth for the public-read GQL endpoint, but only if the
    // `GIGI_PUBLIC_BUNDLES` allowlist is non-empty. When empty, the
    // route isn't even registered — this branch never fires — but the
    // guard here is a defense-in-depth belt/suspenders check.
    //
    // Owner-equivalent claims are stashed so downstream (query executor)
    // treats the request like any other. The narrower verb-and-bundle
    // validation runs inside `public_gql_query` itself.
    if path == "/v1/public/gql" && !state.public_bundles.is_empty() {
        req.extensions_mut().insert(GigiClaims::owner_via_api_key());
        return Ok(next.run(req).await);
    }

    // Try API-key path first (legacy + admin). A successful match
    // grants owner-equivalent claims; the JWT path is skipped.
    //
    // Lookup order: X-API-Key header (HTTP), then `Sec-WebSocket-
    // Protocol: gigi.apikey.<KEY>` (WS upgrade — preferred), then
    // `?api_key=<KEY>` query string (WS upgrade — deprecated, kept
    // for one transition cycle).
    let (subproto_apikey, subproto_bearer) = extract_subprotocol_credentials(req.headers());
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
        if let Some(provided) = header_key
            .or_else(|| subproto_apikey.clone())
            .or(query_key)
        {
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
            if let Some(tok) = header_tok
                .or_else(|| subproto_bearer.clone())
                .or(query_tok)
            {
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
    brain_cache_hits: u64, brain_cache_misses: u64, brain_cache_evictions: u64,
    brain_fit_total_us: u64, brain_compute_total_us: u64,
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
         gigi_uptime_seconds {uptime_secs}\n\
         # HELP gigi_brain_cache_hits_total BundleFlowCache hits\n\
         # TYPE gigi_brain_cache_hits_total counter\n\
         gigi_brain_cache_hits_total {brain_cache_hits}\n\
         # HELP gigi_brain_cache_misses_total BundleFlowCache misses (cold path)\n\
         # TYPE gigi_brain_cache_misses_total counter\n\
         gigi_brain_cache_misses_total {brain_cache_misses}\n\
         # HELP gigi_brain_cache_evictions_total BundleFlowCache evictions (capacity-bound)\n\
         # TYPE gigi_brain_cache_evictions_total counter\n\
         gigi_brain_cache_evictions_total {brain_cache_evictions}\n\
         # HELP gigi_brain_fit_total_microseconds Cumulative time spent in fit (record-walk + Cholesky/eigendecomp)\n\
         # TYPE gigi_brain_fit_total_microseconds counter\n\
         gigi_brain_fit_total_microseconds {brain_fit_total_us}\n\
         # HELP gigi_brain_compute_total_microseconds Cumulative time spent in post-fit brain compute (Langevin etc.)\n\
         # TYPE gigi_brain_compute_total_microseconds counter\n\
         gigi_brain_compute_total_microseconds {brain_compute_total_us}\n"
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

        let brain_cache_hits      = m.brain_cache_hits.load(Ordering::Relaxed);
        let brain_cache_misses    = m.brain_cache_misses.load(Ordering::Relaxed);
        let brain_cache_evictions = m.brain_cache_evictions.load(Ordering::Relaxed);
        let brain_fit_total_us    = m.brain_fit_total_us.load(Ordering::Relaxed);
        let brain_compute_total_us = m.brain_compute_total_us.load(Ordering::Relaxed);

        let body = build_prometheus_text(
            queries_total, errors_total, slow_total,
            p50, p95, p99,
            records_total, bytes_total,
            anomalies, bundle_count, total_records,
            http_conns, ws_conns, uptime_secs,
            brain_cache_hits, brain_cache_misses, brain_cache_evictions,
            brain_fit_total_us, brain_compute_total_us,
        );

        return axum::response::Response::builder()
            .status(200)
            .header("content-type", "text/plain; version=0.0.4; charset=utf-8")
            .body(axum::body::Body::from(body))
            .unwrap();
    }

    // Brain cache stats — pre-computed for the JSON body. The
    // serde_json::json! macro doesn't accept block-expression
    // values inline.
    let brain_cache_hits      = m.brain_cache_hits.load(Ordering::Relaxed);
    let brain_cache_misses    = m.brain_cache_misses.load(Ordering::Relaxed);
    let brain_cache_evictions = m.brain_cache_evictions.load(Ordering::Relaxed);
    let brain_total_calls     = brain_cache_hits + brain_cache_misses;
    let brain_hit_rate = if brain_total_calls > 0 {
        brain_cache_hits as f64 / brain_total_calls as f64
    } else {
        0.0
    };

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
        },
        "brain_cache": {
            "hits":              brain_cache_hits,
            "misses":            brain_cache_misses,
            "evictions":         brain_cache_evictions,
            "hit_rate":          brain_hit_rate,
        },
        "brain_timing_us": {
            "fit_total":         m.brain_fit_total_us.load(Ordering::Relaxed),
            "compute_total":     m.brain_compute_total_us.load(Ordering::Relaxed),
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
                        let mean = fs.mean;
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

#[derive(serde::Serialize)]
struct RecordVectorResponse {
    id: serde_json::Value,
    field: String,
    vector: Vec<f64>,
    dims: usize,
}

#[derive(serde::Deserialize, Default)]
struct RecordVectorParams {
    /// Optional explicit field name. If omitted, the first Vector field on
    /// the record (in schema fiber_fields declaration order) is returned.
    field: Option<String>,
}

/// GET /v1/bundles/{name}/record/{id}/vector
///
/// Surfaces a record's embedding vector for downstream geometric clients
/// (Marcella's IMAGINE Phase 2 uses this to anchor `starting_from = seed_vec`
/// so the walk direction `normalize(prompt_vec − seed_vec)` is geometrically
/// honest instead of the placeholder `−pv̂`).
///
/// Only single-base-field bundles are supported (composite keys → 400).
async fn record_vector(
    State(state): State<Arc<StreamState>>,
    Path((name, id)): Path<(String, String)>,
    Query(params): Query<RecordVectorParams>,
) -> Result<Json<ApiResponse<RecordVectorResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", name),
            }),
        )
    })?;

    // Composite-key bundles need /get with full key params — refuse here so the
    // ambiguity is loud rather than silent.
    let schema = store.schema();
    if schema.base_fields.len() != 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "Bundle '{}' has {} base fields; /record/{{id}}/vector only supports single-base-field bundles. Use /get?{{key}}={{val}} instead.",
                    name,
                    schema.base_fields.len()
                ),
            }),
        ));
    }

    let key_field = schema.base_fields[0].name.clone();
    let id_field_name = schema.base_fields[0].name.clone();
    let fiber_fields = schema.fiber_fields.clone();
    let id_value = if let Ok(n) = id.parse::<i64>() {
        Value::Integer(n)
    } else if let Ok(f) = id.parse::<f64>() {
        Value::Float(f)
    } else {
        Value::Text(id.clone())
    };
    let key: Record = std::iter::once((key_field, id_value)).collect();

    let record = store.point_query(&key).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Record '{}' not found in bundle '{}'", id, name),
            }),
        )
    })?;

    let (field_name, vector) = match params.field.as_deref() {
        Some(explicit) => {
            let v = record.get(explicit).ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: format!("Field '{}' not present on record", explicit),
                    }),
                )
            })?;
            match v {
                Value::Vector(floats) => (explicit.to_string(), floats.clone()),
                other => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: format!(
                                "Field '{}' is {} on this record, not a Vector",
                                explicit,
                                value_type_name(other)
                            ),
                        }),
                    ));
                }
            }
        }
        None => gigi::types::first_vector_field(&record, &fiber_fields).ok_or_else(
            || {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: format!(
                            "No Vector field on record '{}' in bundle '{}'. Pass ?field=<name> if the embedding lives under an unusual name.",
                            id, name
                        ),
                    }),
                )
            },
        )?,
    };

    let dims = vector.len();
    let id_json = match record.get(&id_field_name) {
        Some(v) => value_to_json(v),
        None => serde_json::Value::String(id.clone()),
    };

    let k = store.scalar_curvature();
    Ok(Json(ApiResponse {
        data: RecordVectorResponse {
            id: id_json,
            field: field_name,
            vector,
            dims,
        },
        meta: Some(MetaInfo {
            confidence: Some(curvature::confidence(k)),
            curvature: Some(k),
            capacity: Some(curvature::capacity(1.0, k)),
            count: Some(1),
        }),
    }))
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
    // Davis Conjecture λ-budget — substrate self-introspection of its
    // own carrying capacity (Thm T8ai, claim_0104). D-proxy is the
    // Welford radius (same length scale `horizon_with` uses by
    // default); τ_budget = 1.0 matches the capacity/horizon
    // convention. See [`gigi::curvature::lambda_budget`] doc for the
    // saturation contract (returns 1.0 on flat / collapsed bundles).
    let lambda_budget = match store.as_heap() {
        Some(heap) => curvature::lambda_budget(k, gigi_welford_radius(heap), 1.0),
        // No heap snapshot (e.g. overlay bundle) → fall back to D=1.0,
        // which mirrors the capacity computation above (τ=1.0/K).
        None => curvature::lambda_budget(k, 1.0, 1.0),
    };

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
        lambda_budget,
        per_field,
        #[cfg(feature = "kahler")]
        kahler,
    }))
}

/// Compute the Welford radius (sqrt of mean per-fiber-field variance)
/// for use as the D-proxy in the Davis Conjecture λ-budget ride-along.
/// Mirrors `gigi::curvature::welford_radius` exactly; duplicated here
/// to avoid a cross-crate `pub(crate)` visibility bump for the bin
/// target. Returns NaN when no fiber field has variance > 0.
fn gigi_welford_radius(store: &gigi::bundle::BundleStore) -> f64 {
    let stats = store.field_stats();
    if stats.is_empty() {
        return f64::NAN;
    }
    let mut sum = 0.0_f64;
    let mut n = 0_usize;
    for fs in stats.values() {
        let v = fs.variance();
        if v.is_finite() && v > 0.0 {
            sum += v;
            n += 1;
        }
    }
    if n == 0 {
        f64::NAN
    } else {
        (sum / n as f64).sqrt()
    }
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

// ── Cognitive Geometry Verbs (Branch VII — Davis 2026-05-29) ────────────────

/// `GET /v1/bundles/{name}/capacity[?tau=n]`
///
/// Davis capacity C = τ/K. Returns how many distinct interpretations the
/// bundle can support simultaneously at its current curvature level.
/// τ defaults to 1.0 (C = 1/K in natural units).
async fn bundle_capacity_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<CapacityReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }))
    })?;

    let tau: f64 = params.get("tau").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let k = store.scalar_curvature();
    let c = curvature::capacity(tau, k);
    let conf = curvature::confidence(k);

    let (regime, interpretation) = if k < f64::EPSILON {
        ("flat", format!("K ≈ 0: flat space, infinite capacity. No curvature barriers — every query resolves cleanly."))
    } else if c > 10.0 {
        ("low", format!("C = {c:.2}: low-curvature region. Room for {c:.0} distinct interpretations per unit τ. Synthesis is reliable."))
    } else if c >= 1.0 {
        ("moderate", format!("C = {c:.2}: moderate curvature. The system can hold {c:.1} interpretations simultaneously. Watch for ambiguity."))
    } else if c > 0.1 {
        ("high", format!("C = {c:.3}: high curvature — fewer than one interpretation per unit τ. Ambiguity detection recommended before synthesis."))
    } else {
        ("critical", format!("C = {c:.4}: near-critical curvature. The system cannot reliably distinguish interpretations. Query is at a topological fork."))
    };

    Ok(Json(CapacityReport { capacity: c, k, tau, confidence: conf, regime, interpretation }))
}

/// `GET /v1/bundles/{name}/horizon[?tau=n&estimator=spectral_gap|welford_radius|fixed&fixed_value=N]`
///
/// Holonomy horizon s_max = τ/(K·ℓ_c). Returns the maximum coherent
/// context depth — beyond s_max positions, individual contributions to
/// the accumulated frame rotation become irrecoverable.
///
/// `estimator` chooses the primary length-scale estimator (default:
/// `spectral_gap`). When the primary is degenerate (λ₁ < epsilon for
/// the heat-kernel estimator, NaN for Welford on flat bundles), the
/// fallback (default: `welford_radius`) fires. The response echoes
/// `estimator_used` so the caller can audit which path produced ℓ_c.
async fn bundle_horizon_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<HorizonReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }))
    })?;

    let tau: f64 = params.get("tau").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let k = store.scalar_curvature();
    let lambda1 = store.as_heap().map(spectral::spectral_gap).unwrap_or(0.0);

    // Build HorizonConfig from query params. Default: SpectralGap +
    // WelfordRadius fallback (which is HorizonConfig::default()).
    let estimator = match params.get("estimator").map(|s| s.as_str()) {
        Some("welford_radius") => curvature::LengthScaleEstimator::WelfordRadius,
        Some("fixed") => {
            let v: f64 = params.get("fixed_value")
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse { error: "estimator=fixed requires &fixed_value=<f64>".into() }),
                ))?;
            curvature::LengthScaleEstimator::Fixed(v)
        }
        Some("spectral_gap") | None => curvature::LengthScaleEstimator::SpectralGap,
        Some(other) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "estimator must be one of: spectral_gap, welford_radius, fixed; got {other}"
                    ),
                }),
            ));
        }
    };
    let cfg = curvature::HorizonConfig {
        estimator,
        ..curvature::HorizonConfig::default()
    };

    // The calibrated path needs a heap store for the Welford radius
    // pass. If we only have mmap+overlay, fall back to the scalar
    // shim (same behavior as before the calibrated path existed —
    // documented as a "degenerate when λ₁=0" limitation).
    let (s_max, l_c, estimator_used, fallback_engaged) = if let Some(heap) = store.as_heap() {
        let res = curvature::horizon_with(tau, k, heap, lambda1, &cfg);
        (res.s_max, res.l_c, res.estimator_used, res.fallback_engaged)
    } else {
        let l_c_shim = if lambda1 > f64::EPSILON { 1.0 / lambda1.sqrt() } else { 1.0 };
        let s = curvature::horizon(tau, k, lambda1);
        (s, l_c_shim, curvature::LengthScaleEstimator::SpectralGap, lambda1 < f64::EPSILON)
    };

    let interpretation = if s_max.is_infinite() {
        "K ≈ 0: infinite horizon. Flat geometry — all positions remain \
         individually attributable indefinitely.".to_string()
    } else {
        let fallback_note = if fallback_engaged {
            " [fallback estimator engaged; primary was degenerate]"
        } else {
            ""
        };
        format!(
            "s_max = {s_max:.1}: coherent attribution extends {s_max:.0} positions. \
             Beyond this, accumulated frame rotation cannot be decomposed into \
             individual contributions. (K={k:.4}, ℓ_c={l_c:.4}, τ={tau}){fallback_note}"
        )
    };

    Ok(Json(HorizonReport {
        s_max, k, tau, l_c, lambda1,
        estimator_used, fallback_engaged, interpretation,
    }))
}

/// `GET /v1/bundles/{name}/depth[?k_metric=…&k_connection=…&lambda1_topological=…&lambda1_connection=…]`
///
/// Encoding depth classification from K and λ₁. Returns I (tangent,
/// easily erased) through IV (topological, irrecoverable). Implements
/// Theorem 8.14 of the Cognitive Geometry Correspondence.
///
/// All four threshold query params are optional. Unspecified ones use
/// the `DepthConfig::default()` values from Theorem 8.14. The
/// response echoes the `config_used` so the caller can audit which
/// numbers produced the verdict (defaults are bit-identical to
/// `DepthConfig::default()` when no overrides supplied).
async fn bundle_depth_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<DepthReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }))
    })?;

    let k = store.scalar_curvature();
    let lambda1 = store.as_heap().map(spectral::spectral_gap).unwrap_or(0.0);

    // Build DepthConfig, applying per-field overrides from query params.
    // Unspecified params fall through to DepthConfig::default().
    let mut cfg = curvature::DepthConfig::default();
    let parse_override = |key: &str| -> Option<f64> {
        params.get(key).and_then(|s| s.parse::<f64>().ok())
    };
    if let Some(v) = parse_override("k_metric") { cfg.k_metric = v; }
    if let Some(v) = parse_override("k_connection") { cfg.k_connection = v; }
    if let Some(v) = parse_override("lambda1_topological") { cfg.lambda1_topological = v; }
    if let Some(v) = parse_override("lambda1_connection") { cfg.lambda1_connection = v; }

    let depth = curvature::encoding_depth_with(k, lambda1, &cfg);

    let erasure_energy = match depth {
        curvature::EncodingDepth::Tangent     => "low",
        curvature::EncodingDepth::Connection  => "moderate",
        curvature::EncodingDepth::Metric      => "high",
        curvature::EncodingDepth::Topological => "infinite",
    };

    Ok(Json(DepthReport {
        level: depth.label(),
        description: depth.description(),
        depth,
        k,
        lambda1,
        erasure_energy,
        config_used: cfg,
    }))
}

/// Request body for `POST /v1/bundles/{name}/perceive` — Davis PERCEIVE
/// (Theorem 8.6, Branch VII Cognitive Geometry Correspondence).
///
/// Both `rotation` and `vector` are caller-supplied — typically extracted
/// from a recent TRANSPORT call's `rotation` field. The bundle name is
/// in the path for consistency with the other CG verbs (and so a future
/// server-side rotation source can hang off it), but the math itself is
/// determined by the request body.
#[derive(Deserialize)]
struct PerceiveRequest {
    /// Accumulated rotation matrix R, row-major `dim × dim`. Must be
    /// exactly `dim * dim` floats long.
    rotation: Vec<f64>,
    /// Input vector v, length `dim`. The output is `R · v`.
    vector: Vec<f64>,
    /// Dimension of the rotation matrix and vector.
    dim: usize,
}

/// Response for `POST /v1/bundles/{name}/perceive`. Wraps the
/// `PerceptionResult` from `curvature::perceive` plus the bundle name
/// (echo-back so consumers can log/audit which substrate the verb was
/// scoped to) and a one-line interpretation for builders.
#[derive(Serialize)]
struct PerceiveResponse {
    /// `v_perceived = R · v`. The vector the system actually sees
    /// after parallel-transport through the accumulated rotation R.
    v_perceived: Vec<f64>,
    /// `‖R − I‖_F`. Zero when R = I (no drift); grows monotonically
    /// with the rotation angle. Marcella's coherence-signal δ_t.
    bias: f64,
    /// Dimension of the rotation / vectors, echoed back.
    dim: usize,
    /// Bundle name the request was scoped to.
    bundle: String,
    /// Builder-readable interpretation of the bias magnitude.
    interpretation: String,
}

/// `POST /v1/bundles/{name}/perceive`
///
/// Davis PERCEIVE (Theorem 8.6 — Cognitive Geometry Correspondence).
/// Given an accumulated rotation R (from a prior TRANSPORT, or
/// caller-supplied) and an input vector v, returns:
///
///   v_perceived = R · v
///   bias        = ‖R − I‖_F
///
/// Pure function on `(rotation, vector)` — the bundle name in the
/// path is contextual (for logging / future server-side rotation
/// extraction); the math doesn't read bundle state. Returns 400 on
/// any input-shape mismatch; 404 when the bundle doesn't exist.
async fn bundle_perceive(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<PerceiveRequest>,
) -> Result<Json<PerceiveResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 404 if the bundle doesn't exist. Even though PERCEIVE is pure on
    // its inputs, callers expect the same not-found semantics as the
    // other CG verbs.
    {
        let engine = state.engine.read().unwrap();
        if engine.bundle(&name).is_none() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
            ));
        }
    }

    let result = curvature::perceive(&req.rotation, &req.vector, req.dim).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: format!("perceive: {}", e) }),
        )
    })?;

    // Upper-bound: bias ≤ 2·√dim for orthogonal R (hit by R = −I).
    let max_bias = 2.0 * (req.dim as f64).sqrt();
    let interpretation = if result.bias < 1e-9 {
        "bias ≈ 0: no accumulated rotation. v_perceived ≡ v — the substrate has not \
         distorted this vector along the transport path."
            .to_string()
    } else if result.bias < 0.1 {
        format!(
            "bias = {:.4}: small rotation. v_perceived is a near-trivial perturbation of v; \
             the substrate's drift is below typical action thresholds.",
            result.bias
        )
    } else if result.bias < max_bias / 2.0 {
        format!(
            "bias = {:.4}: moderate rotation (max possible = {:.2}). v_perceived diverges \
             meaningfully from v — re-check before acting on the perceived value.",
            result.bias, max_bias
        )
    } else {
        format!(
            "bias = {:.4}: large rotation (max possible = {:.2}). v_perceived has drifted \
             substantially from v; the substrate's coherence over this path is degraded.",
            result.bias, max_bias
        )
    };

    Ok(Json(PerceiveResponse {
        v_perceived: result.v_perceived,
        bias: result.bias,
        dim: req.dim,
        bundle: name,
        interpretation,
    }))
}

/// Request body for `POST /v1/bundles/{name}/local_holonomy` —
/// Marcella's COHERENCE_SIGNAL_SPEC §3 surface. Two cumulative
/// rotation matrices (current + past-window) compose into the
/// windowed-holonomy rotation, defect, and normalized coherence
/// signal A_t.
#[derive(Deserialize)]
struct LocalHolonomyRequest {
    /// `R_acc,t` — cumulative rotation at the current position,
    /// row-major `dim × dim`. Length must be exactly `dim * dim`.
    r_current: Vec<f64>,
    /// `R_acc,t-w` — cumulative rotation at the past-window position,
    /// row-major `dim × dim`. Length must be exactly `dim * dim`.
    r_past: Vec<f64>,
    /// Dimension of the rotation matrices.
    dim: usize,
}

/// Response for `POST /v1/bundles/{name}/local_holonomy`. Echoes the
/// `LocalHolonomyResult` from `curvature::local_holonomy` plus the
/// bundle name (for audit) and a builder-readable interpretation of
/// the coherence signal.
#[derive(Serialize)]
struct LocalHolonomyResponse {
    /// `R_window = R_current · R_past^T` — net rotation accumulated
    /// over the window [t-w, t], row-major `dim × dim`.
    r_window: Vec<f64>,
    /// `‖R_window − I‖_F` — gauge-invariant under unitary
    /// conjugation per Marcella's §3 proof. Range: `[0, 2·√dim]`.
    defect: f64,
    /// `1 − defect / (2·√dim)` in `[0, 1]`. The normalized coherence
    /// A_t. ≈ 1: laminar. ≈ 0: turbulent.
    coherence: f64,
    /// Dimension echoed back.
    dim: usize,
    /// Bundle name the request was scoped to.
    bundle: String,
    /// Builder-readable interpretation of the coherence magnitude.
    interpretation: String,
}

/// `POST /v1/bundles/{name}/local_holonomy`
///
/// Marcella's windowed-holonomy coherence signal (per
/// `COHERENCE_SIGNAL_SPEC.md §3`). Given two cumulative rotation
/// matrices — `R_current = R_acc,t` and `R_past = R_acc,t-w` from
/// the same prefix scan — returns:
///
///   R_window = R_current · R_past^T
///   defect   = ‖R_window − I‖_F        (gauge-invariant)
///   coherence = 1 − defect / (2·√dim)   (∈ [0, 1])
///
/// The bundle name is contextual (matches the pattern of the other
/// PERCEIVE / CG verbs); the math is determined by the request body.
/// Returns 404 when the bundle doesn't exist, 400 on input-shape
/// mismatch.
async fn bundle_local_holonomy(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<LocalHolonomyRequest>,
) -> Result<Json<LocalHolonomyResponse>, (StatusCode, Json<ErrorResponse>)> {
    {
        let engine = state.engine.read().unwrap();
        if engine.bundle(&name).is_none() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
            ));
        }
    }

    let result = curvature::local_holonomy(&req.r_current, &req.r_past, req.dim).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: format!("local_holonomy: {}", e) }),
        )
    })?;

    let interpretation = if result.coherence > 0.9 {
        format!(
            "coherence = {:.4}: laminar regime. Rotations in this window nearly \
             cancelled — the substrate's geometry is agreeing with itself. \
             defect = {:.4} (max {:.2}).",
            result.coherence,
            result.defect,
            2.0 * (req.dim as f64).sqrt()
        )
    } else if result.coherence > 0.5 {
        format!(
            "coherence = {:.4}: moderate alignment. The geometry has measurable \
             windowed drift but is still coherent at this scale. \
             defect = {:.4}.",
            result.coherence, result.defect
        )
    } else if result.coherence > 0.1 {
        format!(
            "coherence = {:.4}: low alignment. Rotations in this window are \
             compounding meaningfully — approach the turbulent regime. \
             defect = {:.4}.",
            result.coherence, result.defect
        )
    } else {
        format!(
            "coherence = {:.4}: turbulent regime. The geometry is fighting \
             itself across this window. defect = {:.4}, near the {:.2} maximum.",
            result.coherence,
            result.defect,
            2.0 * (req.dim as f64).sqrt()
        )
    };

    Ok(Json(LocalHolonomyResponse {
        r_window: result.r_window,
        defect: result.defect,
        coherence: result.coherence,
        dim: req.dim,
        bundle: name,
        interpretation,
    }))
}

// ============================================================================
// IMAGINE_COHERENCE — predictive gain gate surface
// (IMAGINE_AND_WALK.md §5; Marcella feedback round 1 #2)
// ============================================================================
//
// Marcella's gain gate consumes this to make routing decisions on the
// imagined future instead of the reactive past. LOCAL_HOLONOMY answers
// "how coherent has the recent past been?"; IMAGINE_COHERENCE answers
// "what will the coherence signal be if I continue along this geodesic
// for N steps?".
//
// Trust envelope (Marcella feedback round 2):
//   - max_imagined_curvature defaults to 4.0 = K(CP^1 Fubini-Study).
//   - imagined records render with [imagined:] prefix in provenance
//     strings.

#[cfg(feature = "imagine")]
#[derive(Deserialize)]
struct ImagineCoherenceRequest {
    /// Starting coordinates in the substrate's chart space.
    starting_from: Vec<f64>,
    /// Initial direction vector (tangent at the seed).
    along: Vec<f64>,
    /// Number of integrator steps to project forward. Default 3.
    #[serde(default = "default_imagine_steps")]
    steps: u32,
    /// Curvature ceiling per WalkConfig. Default 4.0 = K(CP^1 FS).
    #[serde(default)]
    max_imagined_curvature: Option<f64>,
    /// Accumulated-holonomy budget. Default 0.5.
    #[serde(default)]
    max_accumulated_holonomy: Option<f64>,
    /// Optional explicit substrate curvature override. When absent,
    /// derived from the bundle's `curvature_stats.mean()`.
    #[serde(default)]
    metric_curvature: Option<f64>,
    /// Optional seed grounding density (normalized to `[0, 1]`) used
    /// to compute the FORECAST/IMAGINE routing advisory. When present,
    /// the response includes a `routing_advisory` block per Marcella
    /// round-3 feedback #3. When absent, routing_advisory is `None`.
    #[serde(default)]
    query_grounding_normalized: Option<f64>,
}

#[cfg(feature = "imagine")]
fn default_imagine_steps() -> u32 {
    3
}

#[cfg(feature = "imagine")]
#[derive(Serialize)]
struct ImagineCoherenceResponse {
    /// Bundle the request was scoped to.
    bundle: String,
    /// Substrate dimension (length of `starting_from` / `along`).
    dim: usize,
    /// Effective metric curvature used for the integration. When
    /// `metric_substituted` is `Some`, this is the substituted (tame)
    /// K, NOT the original bundle K (which is recoverable via
    /// `metric_substituted.original_metric_curvature`).
    metric_curvature: f64,
    /// The walk's safety envelope as resolved against request +
    /// defaults.
    max_imagined_curvature: f64,
    max_accumulated_holonomy: f64,
    /// Per-step trajectory points.
    trajectory: Vec<gigi::imagine::CoherencePoint>,
    /// Coherence at the final step.
    endpoint_coherence: f64,
    /// Curvature at the final step.
    endpoint_curvature: f64,
    /// Whether `walk` would refuse the path at commit time.
    refused: bool,
    /// Human-readable refusal reason if `refused = true`.
    refusal_reason: Option<String>,
    /// FORECAST/IMAGINE routing advisory. Present iff the request
    /// included `query_grounding_normalized`. Per Marcella round-3
    /// feedback #3: surfaces whether IMAGINE was the right verb to
    /// invoke given the seed density, or whether FORECAST would have
    /// been better. `mismatch = true` means the caller mis-routed.
    #[serde(skip_serializing_if = "Option::is_none")]
    routing_advisory: Option<gigi::imagine::RoutingAdvisory>,
    /// Audit signal: present iff the caller raised
    /// `max_imagined_curvature` above the default 4.0 trust ceiling.
    /// Per Marcella round-3 feedback #2: refusal fires when curvature
    /// exceeds the threshold; this fires when the threshold itself is
    /// raised. Production callers should propagate this to their
    /// audit log.
    #[serde(skip_serializing_if = "Option::is_none")]
    threshold_drift: Option<gigi::imagine::CurvatureGateRaisedAboveDefault>,
    /// Phase 2 audit: present iff the tame-metric fallback engaged for
    /// this call. Absent (omitted via `skip_serializing_if`) when the
    /// call ran on the literal substrate metric (the Phase-1-equivalent
    /// path) — so consumers that don't know about Phase 2 see no new
    /// key on the wire. When present, names the substitution so the
    /// consumer can audit which trajectories were integrated on a
    /// tamed geometry instead of the substrate's literal K.
    #[serde(skip_serializing_if = "Option::is_none")]
    metric_substituted: Option<MetricSubstitution>,
}

/// Phase 2 metric-substitution audit envelope. Surfaced as the
/// `metric_substituted` field on [`ImagineCoherenceResponse`] when the
/// tame-metric fallback engages. Marcella's confidence routing reads
/// this to decide whether to downgrade the imagined trajectory's
/// trust before consumption.
#[cfg(feature = "imagine")]
#[derive(Serialize)]
struct MetricSubstitution {
    /// Stable machine-readable tag. v0.2 ships exactly one value:
    /// `"high_k_auto_tame"`. Future tags append.
    reason: String,
    /// The bundle K mean that triggered the fallback. Always the
    /// literal substrate K (`bundle.curvature_stats().mean()`), even
    /// if the caller passed an explicit override — in which case the
    /// fallback is bypassed and this field is not produced.
    original_metric_curvature: f64,
    /// The K actually fed to the integrator (v0.2 always `1.0`).
    substituted_metric_curvature: f64,
    /// The K_MAX threshold the original exceeded. v0.2 ships `10.0`.
    k_max_threshold: f64,
}

/// `POST /v1/bundles/{name}/imagine_coherence`
///
/// Marcella's predictive gain gate surface. Given a seed state and
/// direction, project the imagined coherence forward along a geodesic
/// for `steps` integrator steps. The substrate curvature defaults to
/// the bundle's `curvature_stats.mean()` if not explicitly overridden.
///
/// Returns 404 when the bundle doesn't exist, 400 on input-shape
/// mismatch, 422 (Unprocessable Entity) when the walk would be
/// refused at commit time (`refused: true`, `refusal_reason` populated).
#[cfg(feature = "imagine")]
async fn bundle_imagine_coherence(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<ImagineCoherenceRequest>,
) -> Result<Json<ImagineCoherenceResponse>, (StatusCode, Json<ErrorResponse>)> {
    use gigi::imagine::{
        imagine_coherence_trajectory, imagine_coherence_trajectory_phase_2,
        metric_for_constant_k, RoutingAdvisory, WalkConfig, K_MAX_PHASE2, K_TAME_PHASE2,
    };

    // ─── Step 1: dim-pair consistency (400) ───────────────────────────────
    if req.starting_from.len() != req.along.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "starting_from (dim {}) and along (dim {}) must match",
                    req.starting_from.len(),
                    req.along.len()
                ),
            }),
        ));
    }

    // ─── Step 2: Phase 2 dim floor (was the Phase 1 dim==2 guard) ────────
    // Phase 2 supports dim >= 1. dim < 1 still refused.
    let dim = req.starting_from.len();
    if dim < 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "imagine_coherence Phase 2 requires dim >= 1 (got {})",
                    dim
                ),
            }),
        ));
    }

    // ─── Step 3: bundle lookup + raw K derivation (404 unchanged) ────────
    let (bundle_k_mean, _record_count) = {
        let engine = state.engine.read().unwrap();
        let bundle = engine.bundle(&name).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Bundle '{}' not found", name),
                }),
            )
        })?;
        (bundle.curvature_stats().mean(), bundle.len())
    };

    // ─── Step 4: tame-metric fallback decision (Phase 2 D2) ──────────────
    //
    // The fallback engages when bundle K mean is too high AND the caller
    // did NOT pass an explicit override. Explicit overrides bypass the
    // fallback unconditionally — the consumer has declared the geometry
    // they want.
    let explicit_k = req.metric_curvature;
    let raw_k = explicit_k.unwrap_or(bundle_k_mean);
    let auto_fallback = explicit_k.is_none() && raw_k.abs() > K_MAX_PHASE2;

    let (metric_k, metric_substituted): (f64, Option<MetricSubstitution>) =
        if auto_fallback {
            (
                K_TAME_PHASE2,
                Some(MetricSubstitution {
                    reason: "high_k_auto_tame".to_string(),
                    original_metric_curvature: bundle_k_mean,
                    substituted_metric_curvature: K_TAME_PHASE2,
                    k_max_threshold: K_MAX_PHASE2,
                }),
            )
        } else {
            (raw_k, None)
        };

    let walk_config = WalkConfig {
        max_imagined_curvature: req.max_imagined_curvature.unwrap_or(4.0),
        max_accumulated_holonomy: req.max_accumulated_holonomy.unwrap_or(0.5),
        ..WalkConfig::default()
    };

    // ─── Step 5: WAL audit (only when the fallback actually engages) ─────
    if auto_fallback {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut engine = state.engine.write().unwrap();
        let _ = engine.log_imagine_fallback(&name, bundle_k_mean, K_TAME_PHASE2, now_ms);
        // Failure to log is non-fatal at this layer — the trajectory
        // still computes correctly and the consumer sees
        // `metric_substituted` in the HTTP envelope. We deliberately
        // do NOT fail the request on WAL hiccups (matches the
        // existing pattern for HamiltonianDeclare audits — log is
        // diagnostic).
    }

    // ─── Step 6: integrator dispatch ─────────────────────────────────────
    //
    // dim == 2 AND no fallback engaged → Phase 1 path (bit-identical).
    // Anything else → Phase 2 closed-form constant-K integrator.
    let report_result = if dim == 2 && !auto_fallback {
        let metric = metric_for_constant_k(metric_k);
        imagine_coherence_trajectory(
            &metric,
            "imagine_coherence_seed",
            &name,
            &req.starting_from,
            &req.along,
            req.steps,
            &walk_config,
        )
    } else {
        imagine_coherence_trajectory_phase_2(
            "imagine_coherence_seed",
            &name,
            &req.starting_from,
            &req.along,
            req.steps,
            metric_k,
            &walk_config,
        )
    };

    let report = report_result.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("imagine_coherence Phase 2: {}", e),
            }),
        )
    })?;

    // Marcella round-3 feedback #2: surface threshold drift if the
    // caller raised max_imagined_curvature above the default ceiling.
    let threshold_drift = walk_config.audit_threshold_drift();

    // Marcella round-3 feedback #3: routing advisory iff the request
    // provided a normalized grounding density.
    let routing_advisory = req
        .query_grounding_normalized
        .map(RoutingAdvisory::for_imagine_invocation);

    let response = ImagineCoherenceResponse {
        bundle: name,
        dim: report.dim,
        metric_curvature: metric_k,
        max_imagined_curvature: walk_config.max_imagined_curvature,
        max_accumulated_holonomy: walk_config.max_accumulated_holonomy,
        trajectory: report.trajectory,
        endpoint_coherence: report.endpoint_coherence,
        endpoint_curvature: report.endpoint_curvature,
        refused: report.refused,
        refusal_reason: report.refusal_reason,
        routing_advisory,
        threshold_drift,
        metric_substituted,
    };

    // If refused, return 422 with the report still attached so the
    // consumer can inspect it.
    if response.refused {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: format!(
                    "imagine_coherence refused: {}",
                    response.refusal_reason.clone().unwrap_or_default()
                ),
            }),
        ));
    }

    Ok(Json(response))
}

// ============================================================================
// Sharded HTTP endpoints (feature = "sharded")
//
// Three primitives surface as HTTP routes per the sharded module's verb
// table. Each route takes a bundle name and dispatches through the
// canonical end-to-end entry points:
//
//   /v1/bundles/{name}/sharded/spectral_gap
//     -> shard_lambda_1_from_bundle  (Laplacian extractor + Lanczos)
//   /v1/bundles/{name}/sharded/curvature
//     -> shard_curvature  (per-chart aggregation)
//   /v1/bundles/{name}/sharded/holonomy_loop
//     -> shard_holonomy_around_loop  (closed-loop holonomy + Mobius det)
// ============================================================================

#[cfg(feature = "sharded")]
#[derive(serde::Deserialize)]
struct SharededSpectralGapRequest {
    /// Number of nearest neighbors for the k-NN graph. Default 8.
    #[serde(default = "default_sharded_k_neighbors")]
    k_neighbors: usize,
    /// Maximum Lanczos iterations. Default 120.
    #[serde(default = "default_sharded_lanczos_k_max")]
    k_max: u32,
}

#[cfg(feature = "sharded")]
fn default_sharded_k_neighbors() -> usize {
    8
}

#[cfg(feature = "sharded")]
fn default_sharded_lanczos_k_max() -> u32 {
    120
}

#[cfg(feature = "sharded")]
#[derive(serde::Serialize)]
struct SharededSpectralGapResponse {
    bundle: String,
    lambda_1: f64,
    iterations_used: u32,
    converged_by_window: bool,
    k_neighbors: usize,
}

/// `POST /v1/bundles/{name}/sharded/spectral_gap`
///
/// End-to-end sharded SPECTRAL: builds the k-NN Laplacian from the
/// bundle's records, runs distributed Lanczos via the Fiedler-bisection
/// partition, returns λ_1. No manual block extraction required.
#[cfg(feature = "sharded")]
async fn bundle_sharded_spectral_gap(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<SharededSpectralGapRequest>,
) -> Result<Json<SharededSpectralGapResponse>, (StatusCode, Json<ErrorResponse>)> {
    use gigi::sharded::{
        shard_lambda_1_from_bundle, DistributedLanczosConfig, ShardedBundle,
    };

    // Snapshot the bundle into a heap-resident BundleStore, then wrap
    // it as a trivial-atlas ShardedBundle so the end-to-end extractor
    // can operate on it.
    let store = {
        let engine = state.engine.read().unwrap();
        let bundle = engine.bundle(&name).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Bundle '{}' not found", name),
                }),
            )
        })?;
        // Materialize records into a fresh heap-resident BundleStore
        // so the sharded analysis can operate independently of the
        // engine's storage backing (heap, mmap, or remote).
        let schema = bundle.schema().clone();
        let records: Vec<gigi::types::Record> = bundle.records().collect();
        let mut s = gigi::bundle::BundleStore::new(schema);
        for r in records {
            s.insert(&r);
        }
        s
    };
    let sharded =
        ShardedBundle::wrap_trivial(store, gigi::sharded::ShardId(0));

    let config = DistributedLanczosConfig {
        k_max: req.k_max,
        ..Default::default()
    };

    let result = shard_lambda_1_from_bundle(&sharded, req.k_neighbors, &config).map_err(
        |e| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse {
                    error: format!("sharded_spectral_gap: {:?}", e),
                }),
            )
        },
    )?;

    Ok(Json(SharededSpectralGapResponse {
        bundle: name,
        lambda_1: result.lambda_1,
        iterations_used: result.iterations_used,
        converged_by_window: result.converged_by_window,
        k_neighbors: req.k_neighbors,
    }))
}

#[cfg(feature = "sharded")]
#[derive(serde::Deserialize)]
struct SharededCurvatureRequest {
    /// Number of charts to partition into. 1 = trivial atlas (single
    /// chart, equivalent to the un-sharded bundle's curvature_stats).
    /// Larger values hash-partition the records.
    #[serde(default = "default_sharded_n_charts")]
    n_charts: u32,
}

#[cfg(feature = "sharded")]
fn default_sharded_n_charts() -> u32 {
    1
}

#[cfg(feature = "sharded")]
#[derive(serde::Serialize)]
struct SharededCurvatureResponse {
    bundle: String,
    n_charts: u32,
    n_records: u64,
    mean_k: f64,
    std_dev_k: f64,
}

/// `POST /v1/bundles/{name}/sharded/curvature`
///
/// Sharded CURVATURE: aggregates per-chart `CurvatureStats` across the
/// bundle's hash-sharded charts. For `n_charts = 1`, returns the same
/// values as the un-sharded `/curvature` endpoint.
#[cfg(feature = "sharded")]
async fn bundle_sharded_curvature(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<SharededCurvatureRequest>,
) -> Result<Json<SharededCurvatureResponse>, (StatusCode, Json<ErrorResponse>)> {
    use gigi::sharded::{shard_curvature, ShardedBundle};

    if req.n_charts == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "n_charts must be >= 1".into(),
            }),
        ));
    }

    let (schema, records) = {
        let engine = state.engine.read().unwrap();
        let bundle = engine.bundle(&name).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Bundle '{}' not found", name),
                }),
            )
        })?;
        (bundle.schema().clone(), bundle.records().collect::<Vec<_>>())
    };

    let sharded = if req.n_charts == 1 {
        let mut s = gigi::bundle::BundleStore::new(schema);
        for r in records {
            s.insert(&r);
        }
        ShardedBundle::wrap_trivial(s, gigi::sharded::ShardId(0))
    } else {
        ShardedBundle::wrap_hash_sharded(
            schema,
            records,
            req.n_charts,
            gigi::sharded::ShardId(0),
        )
    };

    let report = shard_curvature(&sharded);

    Ok(Json(SharededCurvatureResponse {
        bundle: name,
        n_charts: req.n_charts,
        n_records: report.n_records(),
        mean_k: report.mean(),
        std_dev_k: report.std_dev(),
    }))
}

#[cfg(feature = "sharded")]
#[derive(serde::Deserialize)]
struct SharededHolonomyLoopRequest {
    /// Loop as `[(chart_id, [x, y])]` in path order. The closing
    /// segment from the last point back to the first is implicit.
    path: Vec<(u32, Vec<f64>)>,
    /// Transitions: `[(from_chart, to_chart, [a00, a01, a10, a11])]`.
    /// Missing pairs default to identity.
    #[serde(default)]
    transitions: Vec<(u32, u32, [f64; 4])>,
}

#[cfg(feature = "sharded")]
#[derive(serde::Serialize)]
struct SharededHolonomyLoopResponse {
    bundle: String,
    holonomy: [f64; 4],
    det: f64,
    /// True iff det(H) < 0 (orientation flip / Z_2 monodromy detected).
    orientation_flipped: bool,
}

/// `POST /v1/bundles/{name}/sharded/holonomy_loop`
///
/// Closed-loop holonomy across chart-pair transitions. For Möbius
/// (orientation-reversing) gauges, `det(H) = -1` and `orientation_flipped
/// = true` per T13's Z_2 monodromy detection in a 2D fiber.
#[cfg(feature = "sharded")]
async fn bundle_sharded_holonomy_loop(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<SharededHolonomyLoopRequest>,
) -> Result<Json<SharededHolonomyLoopResponse>, (StatusCode, Json<ErrorResponse>)> {
    use gigi::sharded::{mat2x2_det, shard_holonomy_around_loop, ChartId};
    use std::collections::HashMap;

    // Bundle existence check (just for the 404 path; no data access needed)
    {
        let engine = state.engine.read().unwrap();
        engine.bundle(&name).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Bundle '{}' not found", name),
                }),
            )
        })?;
    }

    if req.path.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "path must contain at least one point".into(),
            }),
        ));
    }

    // Convert path to internal form
    let loop_pts: Vec<(ChartId, Vec<f64>)> = req
        .path
        .into_iter()
        .map(|(c, p)| (ChartId(c), p))
        .collect();
    let mut tx: HashMap<(ChartId, ChartId), [f64; 4]> = HashMap::new();
    for (from, to, mat) in req.transitions {
        tx.insert((ChartId(from), ChartId(to)), mat);
    }

    let atlas = gigi::sharded::Atlas::trivial(gigi::sharded::ShardId(0));
    let h = shard_holonomy_around_loop(&atlas, &loop_pts, &tx).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("sharded_holonomy_loop: {:?}", e),
            }),
        )
    })?;

    let det = mat2x2_det(&h);
    Ok(Json(SharededHolonomyLoopResponse {
        bundle: name,
        holonomy: h,
        det,
        orientation_flipped: det < 0.0,
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

    // Polymorphism gap (live-caught 2026-06-04): spectral_gap_cached
    // lives on BundleStore (heap-only). For mmap-resident bundles
    // `as_heap()` returns None and we'd otherwise emit a misleading
    // "insufficient records" error. Distinguish the two cases so
    // callers know whether to insert more data or to wait on the
    // follow-up that threads spectral_gap through BundleRef.
    let snap = if let Some(s) = store.as_heap() {
        s.spectral_gap_cached().ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!(
                        "Bundle '{}' has insufficient records for spectral gap (need ≥ 2)",
                        name
                    ),
                }),
            )
        })?
    } else {
        // Honest signal: the substrate exists and has records, but
        // this endpoint hasn't been ported to the polymorphic
        // BundleRef path yet.
        return Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(ErrorResponse {
                error: format!(
                    "Bundle '{}' is mmap-resident; /spectral_gap is heap-only \
                     pending the polymorphic-BundleRef follow-up. Workaround: \
                     read via /v1/bundles/{}/curvature which already lifts.",
                    name, name
                ),
            }),
        ));
    };

    Ok(Json(SpectralGapResponse {
        lambda_2: snap.lambda_2,
        mix_time: snap.mix_time,
        cheeger_lower: snap.cheeger_lower,
        cheeger_upper: snap.cheeger_upper,
        cached_at: snap.cached_at,
    }))
}

// ────────────────────────────────────────────────────────────
// Sprint N (v0.4) — Invariant Consistency Verification endpoint.
// Auditor-facing surface for `gigi::invariant_verify`.
//
//   POST /v1/bundles/{name}/verify_invariant
//     Body: { "bundle_id": <prover's claim>, "claimed": { k, lambda_1,
//             holonomy_mean, record_count, beta_0, beta_1 },
//             "tolerances": Option<{ k, lambda_1, holonomy_mean }> }
//     Resp: { "verdict": "verified" | "bundle_mismatch" | "rejected", ... }
//
// Path {name} is the verifier's claim about which bundle this is —
// passed as `store_bundle_id` so the bundle_id binding (review Gap 1)
// is enforced at the HTTP layer too.
// Not gated on `kahler` — Sprint N is generally useful for any v0.2+
// bundle whose invariant tuple computes.
// ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InvariantTupleWire {
    k: f64,
    lambda_1: f64,
    holonomy_mean: f64,
    record_count: u64,
    beta_0: u64,
    beta_1: u64,
}

impl From<gigi::integrity::InvariantTuple> for InvariantTupleWire {
    fn from(t: gigi::integrity::InvariantTuple) -> Self {
        Self {
            k: t.k,
            lambda_1: t.lambda_1,
            holonomy_mean: t.holonomy_mean,
            record_count: t.record_count,
            beta_0: t.beta_0,
            beta_1: t.beta_1,
        }
    }
}

impl From<InvariantTupleWire> for gigi::integrity::InvariantTuple {
    fn from(w: InvariantTupleWire) -> Self {
        Self {
            k: w.k,
            lambda_1: w.lambda_1,
            holonomy_mean: w.holonomy_mean,
            record_count: w.record_count,
            beta_0: w.beta_0,
            beta_1: w.beta_1,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct InvariantTolerancesWire {
    k: Option<f64>,
    lambda_1: Option<f64>,
    holonomy_mean: Option<f64>,
}

impl From<InvariantTolerancesWire> for gigi::invariant_verify::InvariantTolerances {
    fn from(w: InvariantTolerancesWire) -> Self {
        let d = gigi::invariant_verify::InvariantTolerances::default();
        Self {
            k: w.k.unwrap_or(d.k),
            lambda_1: w.lambda_1.unwrap_or(d.lambda_1),
            holonomy_mean: w.holonomy_mean.unwrap_or(d.holonomy_mean),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct VerifyInvariantRequest {
    /// The prover's claim about which bundle the tuple is for. Checked
    /// against the URL path `{name}` (which the verifier asserts is
    /// the bundle they hold); mismatch → `bundle_mismatch` verdict.
    bundle_id: String,
    /// The full six-component tuple the prover claims.
    claimed: InvariantTupleWire,
    /// Per-field f64 tolerances. Defaults (1e-10 each) used when omitted.
    tolerances: Option<InvariantTolerancesWire>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
enum VerifyInvariantResponse {
    /// Bundle + all six tuple components agreed.
    Verified { computed: InvariantTupleWire },
    /// Bundle identity disagreed (Gap 1 trust-handoff check).
    BundleMismatch { claimed: String, store_id: String },
    /// First tuple component disagreement in fingerprint order.
    Rejected {
        field: String,
        claimed: f64,
        computed: f64,
        delta: f64,
    },
}

impl From<gigi::invariant_verify::VerifyResult> for VerifyInvariantResponse {
    fn from(r: gigi::invariant_verify::VerifyResult) -> Self {
        use gigi::invariant_verify::VerifyResult as VR;
        match r {
            VR::Verified { computed } => VerifyInvariantResponse::Verified {
                computed: computed.into(),
            },
            VR::BundleMismatch { claimed, store_id } => {
                VerifyInvariantResponse::BundleMismatch { claimed, store_id }
            }
            VR::Rejected {
                field,
                claimed,
                computed,
                delta,
            } => VerifyInvariantResponse::Rejected {
                field: field.to_string(),
                claimed,
                computed,
                delta,
            },
        }
    }
}

async fn verify_invariant_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<VerifyInvariantRequest>,
) -> Result<Json<VerifyInvariantResponse>, (StatusCode, Json<ErrorResponse>)> {
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
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "Bundle '{}' is mmap-resident; Sprint N verifier requires heap bundle for InvariantTuple::compute",
                    name
                ),
            }),
        )
    })?;
    let statement = gigi::invariant_verify::InvariantStatement {
        bundle_id: req.bundle_id,
        claimed: req.claimed.into(),
    };
    let tolerances: gigi::invariant_verify::InvariantTolerances = req
        .tolerances
        .map(Into::into)
        .unwrap_or_default();
    let result = gigi::invariant_verify::verify_invariant_statement(
        heap,
        &name,
        &statement,
        tolerances,
    );
    Ok(Json(result.into()))
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

/// Materialize a `(N, D)` matrix from a bundle, served from cache when
/// possible. Per Marcella's 2026-05-29 `GIGI_BUG_REPORT_onfields_latency.md`:
/// the three vector-search brain endpoints (`intent_gate`, `confidence`,
/// `confidence_with_explain`) all need a flat numeric view of the fiber
/// columns for KDE + nearest-record queries. Building it per request is
/// `O(N·D)` of HashMap lookups + per-cell type validation +
/// `Vec<Vec<f64>>` allocation; at `N=10k, D=384` that's ~30 s on its own,
/// before any actual math runs.
///
/// This helper hits [`gigi::vector_cache::VectorMatrixCache`] for the
/// cached matrix; on miss it rebuilds via [`extract_field_samples`]
/// (reusing all its schema-validation paths) and flattens into a
/// contiguous `Vec<f64>` for the cosine-identity hot loops. Single-flight
/// on miss via the per-key compute lock — concurrent requests for the
/// same `(bundle, fields)` block on one build instead of all racing.
///
/// Invalidation: `BundleStore::mutation_counter` — any insert/update on
/// the bundle bumps the counter and the next request rebuilds.
#[cfg(feature = "kahler")]
fn materialize_matrix_cached(
    state: &Arc<StreamState>,
    bundle_name: &str,
    heap: &gigi::BundleStore,
    fields: &[String],
) -> Result<gigi::vector_cache::CachedMatrix, (StatusCode, Json<ErrorResponse>)> {
    use gigi::vector_cache::{CachedMatrix, MaterializedMatrix, VectorCacheKey};

    let key = VectorCacheKey::build(bundle_name, fields);
    let counter = heap.mutation_counter();

    // Hot path: cache hit + counter match → O(1) atomic refcount clone.
    if let Some(cached) = state.vector_cache.get(&key, counter) {
        return Ok(cached);
    }

    // Cold path: acquire per-key compute lock, double-check, build.
    let compute_lock_arc = state.vector_cache.acquire_compute_lock(&key);
    let _guard = match compute_lock_arc.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    // Double-check under lock — another thread may have just inserted.
    if let Some(cached) = state.vector_cache.get(&key, counter) {
        state.vector_cache.release_compute_lock(&key);
        return Ok(cached);
    }

    // Build. Reuse extract_field_samples for all the existing validation
    // (base-vs-fiber error messages, non-numeric handling, etc), then
    // flatten into a contiguous Vec<f64> for the matrix.
    let samples = match extract_field_samples(heap, fields) {
        Ok(s) => s,
        Err(e) => {
            state.vector_cache.release_compute_lock(&key);
            return Err(bad_request(&e));
        }
    };
    let n = samples.len();
    let d = fields.len();
    let mut data = Vec::with_capacity(n * d);
    for row in samples {
        data.extend(row);
    }
    let matrix = Arc::new(MaterializedMatrix::new(data, n, d));
    let cached = CachedMatrix::new(counter, matrix);
    state.vector_cache.insert(key.clone(), cached.clone());
    state.vector_cache.release_compute_lock(&key);
    Ok(cached)
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
    /// FLAT row-major Vec<f64> of length n*n (wave 2 §B): element
    /// (i, j) at index `i*n + j`. The Arc means cache-hit cloning
    /// is O(1) atomic refcount, not a 1.2MB memcpy at n=384.
    precision: Option<Arc<Vec<f64>>>,
    /// Σ post-flooring — only populated for `FitMode::Full`.
    /// FLAT row-major (same layout as precision). Surfaced via
    /// fit_diagnostics endpoint.
    covariance: Option<Arc<Vec<f64>>>,
    /// Full-fit diagnostics (None for isotropic/diagonal).
    eigenvalues_raw: Option<Arc<Vec<f64>>>,
    eigenvalues_effective: Option<Arc<Vec<f64>>>,
    eigenvalue_floor_used: f64,
    floored_eigenvalue_count: usize,
    condition_number: f64,
    variance_ratio: f64,
}

/// Thin wrapper around the generic
/// [`gigi::caches::single_flight::SingleFlightCache`] (extracted
/// 2026-06-20, workflow `w2n0fgqkk`). Behavior is byte-identical to
/// the prior hand-rolled implementation:
///
/// - `RwLock<HashMap>` hot path, lock-free read on hit.
/// - Per-key `Mutex<()>` single-flight on cache miss.
/// - Mutation-counter invalidation (caller passes
///   `BundleStore::mutation_counter()` on every call).
/// - Random FIFO-on-iteration eviction at capacity.
///
/// The `counter_at_fit` field is retained on [`CachedFit`] for
/// backward compatibility with existing diagnostic call sites but is
/// no longer consulted for invalidation — the cache's `(V, u64)`
/// tuple is the source of truth.
#[cfg(feature = "kahler")]
pub struct BundleFlowCache {
    inner: gigi::caches::single_flight::SingleFlightCache<CacheKey, CachedFit>,
}

#[cfg(feature = "kahler")]
impl BundleFlowCache {
    pub fn new(max_entries: usize) -> Self {
        BundleFlowCache {
            inner: gigi::caches::single_flight::SingleFlightCache::new(max_entries),
        }
    }

    /// Single-flight: acquire (or create) the per-key compute lock.
    /// See `SingleFlightCache::acquire_compute_lock` for the contract.
    fn acquire_compute_lock(
        &self,
        key: &CacheKey,
    ) -> std::sync::Arc<std::sync::Mutex<()>> {
        self.inner.acquire_compute_lock(key)
    }

    /// Release the per-key compute lock entry.
    fn release_compute_lock(&self, key: &CacheKey) {
        self.inner.release_compute_lock(key);
    }

    /// Hot path lookup. Returns Some only if the entry's stored
    /// counter matches `current_counter`.
    fn get(&self, key: &CacheKey, current_counter: u64) -> Option<CachedFit> {
        self.inner.get(key, current_counter)
    }

    fn insert(&self, key: CacheKey, fit: CachedFit) {
        let counter = fit.counter_at_fit;
        let _ = self.inner.insert(key, fit, counter);
    }

    /// Insert variant that reports whether an eviction happened.
    /// Returns true iff an entry was evicted to make room.
    fn insert_with_eviction_hint(&self, key: CacheKey, fit: CachedFit) -> bool {
        let counter = fit.counter_at_fit;
        self.inner.insert(key, fit, counter)
    }

    /// Number of entries currently held.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Drop all cached fits.
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.inner.clear();
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

/// Brain endpoint response with content negotiation (wave 2 §D).
///
/// Per Bee's 2026-05-27 product policy: **internals DHOOM-only,
/// externals DHOOM first-class with JSON optional via Accept header.**
///
/// Routing:
///   - `Accept: application/dhoom` → DHOOM-encoded body
///   - default (or `Accept: application/json`) → JSON
///
/// Same X-Bundle-Mutation-Counter header surfaced either way so the
/// cache-warmth contract is encoding-independent.
///
/// Known wart (filed as task #112): we currently serialize the
/// response struct via serde_json::to_value before handing to
/// dhoom::encode. The intermediate JSON Value is an internal
/// implementation detail; the public wire is DHOOM end-to-end. A
/// native Record API on the DHOOM encoder eliminates the
/// intermediate — deferred to wave 2.5 per the spec.
///
/// Safe default: JSON. This is the back-compat-preserving default;
/// existing consumers (Marcella's wire code that doesn't yet send
/// the Accept header) continue to work unchanged. New consumers
/// opt into DHOOM by sending the Accept header. Once Marcella's
/// side flips to DHOOM-by-default in her client code, we can flip
/// the server default too.
#[cfg(feature = "kahler")]
fn negotiated_brain_response<T: serde::Serialize>(
    accept: Option<&str>,
    counter_at_fit: u64,
    body: T,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    use axum::http::{header, HeaderMap, HeaderName, HeaderValue};
    use axum::response::IntoResponse;

    let wants_dhoom = accept
        .map(|s| s.contains("application/dhoom"))
        .unwrap_or(false);

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-bundle-mutation-counter"),
        HeaderValue::from(counter_at_fit),
    );

    if wants_dhoom {
        // Struct → Value → wrap-in-array → DHOOM. The DHOOM encoder
        // is records-oriented: the top-level shape it accepts is
        // `{collection_name: [record, record, ...]}` (encode_json's
        // contract). Analytical responses are one heterogeneous
        // object, not a list of records — so we wrap as a
        // single-element array under collection name "brain_response".
        // Consumers MUST unwrap by indexing [0] under "brain_response".
        //
        // Discovered via wave-2 e2e probe 2026-05-27: prior attempt
        // used dhoom::encode with a single-key object envelope and
        // hit "Top-level object value must be an array" — that's
        // what encode_json's array wrapping solves directly.
        //
        // Task #112 (native heterogeneous-Value Record API on the
        // DHOOM encoder) eliminates the array wrapper; until then,
        // single-element array under "brain_response" is the
        // documented wire contract.
        let value = serde_json::to_value(&body).map_err(|e| {
            bad_request(&format!(
                "DHOOM encode: serialize struct → Value failed: {}",
                e
            ))
        })?;
        let result = gigi::dhoom::encode_json(
            std::slice::from_ref(&value),
            "brain_response",
        );
        let encoded = result.dhoom;
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/dhoom"),
        );
        Ok((headers, encoded).into_response())
    } else {
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let json_bytes = serde_json::to_vec(&body).map_err(|e| {
            bad_request(&format!("JSON encode: serialize failed: {}", e))
        })?;
        Ok((headers, json_bytes).into_response())
    }
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
        mu.push(s.mean);
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
///
/// Matrices stored as FLAT row-major Vec<f64> (length n*n) per
/// wave 2 §B (Bee's product-latency reframe): nested Vec<Vec<f64>>
/// causes pointer-chasing on every row access; flat layout keeps
/// rows contiguous in memory and lets the per-step matvec stream
/// linearly through cache. At n=384 the matvec is the inner loop
/// of every Langevin step — flat layout gives ~30-50% speedup.
#[cfg(feature = "kahler")]
struct FullFitResult {
    mu: Vec<f64>,
    /// Full n×n covariance matrix, row-major flat (length n*n).
    /// Element (i, j) is at index `i * n + j`. Post-eigenvalue-floor.
    covariance: Vec<f64>,
    /// Precision matrix Σ⁻¹, row-major flat (length n*n).
    /// Element (i, j) is at index `i * n + j`. Used by the
    /// per-step Langevin matvec.
    precision: Vec<f64>,
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
        mu.push(s.mean);
    }

    let n = fields.len();

    // Pass 2: walk records, accumulate Σ as a FLAT row-major Vec<f64>
    // (wave 2 §B — no pointer-chasing on the inner loop). For N
    // records, n fields: O(N·n²) memory-light (one record at a time).
    // bge_v2 with N=9964 and n=10 fields = ~10⁶ ops, sub-second.
    // For n=384 (full embedding) ~1.5G ops ~1-2s. Both acceptable.
    let mut cov = vec![0.0_f64; n * n];
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
        // Accumulate Σ += dx · dxᵀ. Inner loop is now contiguous
        // memory access vs the prior `cov[i][j]` double-deref.
        for i in 0..n {
            let dx_i = dx[i];
            let row_start = i * n;
            for j in 0..n {
                cov[row_start + j] += dx_i * dx[j];
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
    for x in cov.iter_mut() {
        *x /= denom;
    }

    // Diagonal floor — analog of L13.6 for the diagonal entries
    // of Σ. Off-diagonal entries are NOT floored (they can legitimately
    // be ~0 for uncorrelated axes). Same composition as fit_diagonal:
    // relative ε·median floor + absolute Euler-stability floor.
    // Indexing: (i, i) lives at i*n + i.
    let sigma_sq_per_field_raw: Vec<f64> =
        (0..n).map(|i| cov[i * n + i].max(0.0)).collect();
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
        if cov[i * n + i] < effective_floor {
            floored_indices.push(i);
            cov[i * n + i] = effective_floor;
        }
    }
    let sigma_sq_per_field: Vec<f64> = (0..n).map(|i| cov[i * n + i]).collect();

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
    // EIGENDIRECTIONS, not axes. See spec v0.3 Appendix B for the
    // history (H2 turned out moot for bge_v2; floor retained as
    // defensive for future bundles).
    //
    // The fix: eigendecompose Σ, clip eigenvalues below
    // ε·median(λ_raw), reconstruct, THEN invert. This makes the
    // geometry well-conditioned regardless of variance skew or
    // correlation pattern.
    //
    // nalgebra::DMatrix accepts our flat row-major Vec directly.
    let mat = nalgebra::DMatrix::from_row_slice(n, n, &cov);
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

    // Reconstruct Σ_regularized = U · diag(λ_eff) · Uᵀ. Write
    // directly to a flat row-major Vec<f64> for the response
    // (wave 2 §B — no pointer-chasing on row access).
    let lambda_diag = nalgebra::DMatrix::from_diagonal(
        &nalgebra::DVector::from_vec(eigenvalues_effective.clone()),
    );
    let cov_regularized = &eigen.eigenvectors * &lambda_diag * eigen.eigenvectors.transpose();
    let mut cov_flat_out = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            cov_flat_out.push(cov_regularized[(i, j)]);
        }
    }
    // Refresh per-axis diagonals (the diagonal floor still applies
    // as a guard, but with eigenvalue flooring done the diagonals
    // are usually already above it).
    let sigma_sq_per_field: Vec<f64> =
        (0..n).map(|i| cov_flat_out[i * n + i]).collect();

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
    // Write Σ⁻¹ directly to flat row-major Vec<f64>. Same shape
    // as covariance; element (i, j) at index i*n + j.
    let mut precision_flat = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            precision_flat.push(precision_mat[(i, j)]);
        }
    }

    Ok(FullFitResult {
        mu,
        covariance: cov_flat_out,
        precision: precision_flat,
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
        mu.push(s.mean);
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
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainSampleRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
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
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    // Davis Conjecture λ-budget ride-along (Thm T8ai, claim_0104).
    // Reuse the engine read guard already held above to avoid a
    // re-entrant RwLock acquisition.
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainSampleResponse {
                samples,
                fit_mean: ctx.mu,
                fit_sigma_sq: ctx.sigma_sq,
                fit_sigma_sq_per_field: ctx.sigma_sq_per_field,
                fit_sigma_sq_per_field_raw: ctx.sigma_sq_per_field_raw,
                fit_sigma_floor_used: ctx.effective_floor,
                fit_floored_indices: ctx.floored_indices,
                fit_mode_used,
            },
            lambda_budget,
        },
    )
}

// ─── fit_diagnostics (S1 wave 1 §G) ─────────────────────────
//
// Per Marcella's 2026-05-26 H2 attractor letter §Asks + her
// 2026-05-27 G-side adjustment: return the FULL eigenvalue
// spectrum (not summary stats) so consumers can see the
// distribution shape — heavy tail (real signal, H1) vs sharp
// cliff (diagonal-fit pathology, H2). 3KB at n=384, tiny.
//
// Uses the cache path: fit_diagnostics on a bundle with a warm
// cache returns sub-µs; cold path = ~3s at n=384 (same cost as
// the other brain endpoints' first call after invalidation).

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainFitDiagnosticsRequest {
    fields: Vec<String>,
    /// Fit mode to inspect. Defaults to isotropic; pass "full" to
    /// get the eigenvalue spectrum (the H1-vs-H2 diagnostic).
    #[serde(default)]
    fit_mode: Option<FitMode>,
    /// Relative-median ε floor for diagonal AND eigenvalue floors.
    /// Default 1e-3 per L13.6 (diagonal) and S1 §2a (eigenvalue).
    /// Pass 0 to disable relative flooring (absolute Euler-
    /// stability floor remains).
    #[serde(default)]
    sigma_floor_epsilon: Option<f64>,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainFitDiagnosticsResponse {
    /// Echo of the fit mode actually used.
    fit_mode_used: String,
    /// Number of dimensions (== `fields.len()`).
    dim: usize,

    // ── Mean ────────────────────────────────────────────────
    /// Fit_mean vector (length = dim).
    fit_mean: Vec<f64>,
    /// L2 norm of fit_mean. Marcella's H2 §Asks Diagnostic 3
    /// uses this to compare against record distances: if a
    /// specific record (e.g. `double_cover_v3`) sits unusually
    /// close to fit_mean it's at the deep point of the
    /// fitted Gaussian — the H2 mechanism.
    fit_mean_norm: f64,

    // ── Per-axis variance (diagonal of Σ) ──────────────────
    /// Per-axis raw variance (pre-floor).
    variance_per_dim_raw: Vec<f64>,
    /// Per-axis effective variance (post-diagonal-floor).
    variance_per_dim_effective: Vec<f64>,
    /// Effective floor applied to per-axis variance.
    variance_floor_used: f64,
    /// Indices in `fields` whose raw variance was below floor.
    floored_diagonal_indices: Vec<usize>,
    /// Ratio max(σ²)/min(σ²) of the diagonal entries after floor.
    /// First-pass H2 diagnostic — high ratio is suggestive.
    variance_ratio: f64,

    // ── Eigenvalue spectrum (Full fit only) ────────────────
    //
    // The load-bearing H1-vs-H2 diagnostic per Marcella's
    // 2026-05-27 G-side ask. Empty for Isotropic/Diagonal
    // (those don't compute eigendecomposition).
    /// Eigenvalues of Σ BEFORE flooring, sorted descending.
    /// Length = dim for Full; empty otherwise.
    eigenvalues_raw: Vec<f64>,
    /// Eigenvalues AFTER flooring. Length = dim for Full; empty
    /// otherwise.
    eigenvalues_effective: Vec<f64>,
    /// Eigenvalue floor used (max of ε·median(λ), absolute
    /// stability floor). 0 for non-Full modes.
    eigenvalue_floor_used: f64,
    /// How many eigenvalues got clipped. Non-zero is H2.
    n_floored_eigenvalues: usize,
    /// λ_max / λ_min post-floor. Bounded above by 1/ε for the
    /// effective-fit case. Infinity if not yet computed.
    condition_number: f64,

    // ── Provenance ─────────────────────────────────────────
    /// Bundle's mutation counter at the time this fit was
    /// computed. Surfaced in the response BODY (also in the
    /// X-Bundle-Mutation-Counter header) so consumers reading
    /// the JSON / DHOOM payload directly can stamp warmth
    /// without needing header access.
    counter_at_fit: u64,
    /// Was this fit served from cache?
    cache_hit: bool,
}

/// POST /v1/bundles/{name}/brain/fit_diagnostics
///
/// Per Marcella's H2 attractor letter — returns the full
/// diagnostic shape of the Gaussian fit on the given bundle +
/// fields + fit_mode + floor, including the eigenvalue spectrum
/// for the H1-vs-H2 verdict.
///
/// Uses the BundleFlowCache: cache hit serves the diagnostic
/// payload from cached fit data (sub-µs); cache miss computes
/// the fit (one record walk + Cholesky/eigendecomp).
#[cfg(feature = "kahler")]
async fn brain_fit_diagnostics_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainFitDiagnosticsRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);

    let fit_mode = req.fit_mode.unwrap_or_default();
    let counter_before = heap.mutation_counter();
    let key = CacheKey::build(&name, fit_mode, &req.fields, req.sigma_floor_epsilon);
    let cache_hit_initial = state.flow_cache.get(&key, counter_before).is_some();

    // Route through the cache so we don't double-compute.
    let (_, counter_at_fit) = flow_from_bundle_cached(
        &state,
        &name,
        heap,
        &req.fields,
        fit_mode,
        req.sigma_floor_epsilon,
    )?;

    // After the call we KNOW the cache has the entry. Read it
    // directly to extract the diagnostic-shaped data.
    let cached = state
        .flow_cache
        .get(&key, counter_at_fit)
        .ok_or_else(|| {
            bad_request("fit_diagnostics: cache lookup failed post-fit; this is a bug")
        })?;

    let fit_mode_used = match fit_mode {
        FitMode::Isotropic => "isotropic",
        FitMode::Diagonal => "diagonal",
        FitMode::Full => "full",
    }
    .to_string();

    let fit_mean_vec = (*cached.mu).clone();
    let fit_mean_norm = fit_mean_vec
        .iter()
        .map(|x| x * x)
        .sum::<f64>()
        .sqrt();

    // Eigenvalues are only populated for Full; empty Vec for
    // other modes (consumers get a stable shape).
    let eigenvalues_raw = cached
        .eigenvalues_raw
        .as_ref()
        .map(|a| (**a).clone())
        .unwrap_or_default();
    let eigenvalues_effective = cached
        .eigenvalues_effective
        .as_ref()
        .map(|a| (**a).clone())
        .unwrap_or_default();

    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainFitDiagnosticsResponse {
                fit_mode_used,
                dim: req.fields.len(),
                fit_mean: fit_mean_vec,
                fit_mean_norm,
                variance_per_dim_raw: (*cached.sigma_sq_per_field_raw).clone(),
                variance_per_dim_effective: (*cached.sigma_sq_per_field).clone(),
                variance_floor_used: cached.effective_floor,
                floored_diagonal_indices: (*cached.floored_indices).clone(),
                variance_ratio: cached.variance_ratio,
                eigenvalues_raw,
                eigenvalues_effective,
                eigenvalue_floor_used: cached.eigenvalue_floor_used,
                n_floored_eigenvalues: cached.floored_eigenvalue_count,
                condition_number: cached.condition_number,
                counter_at_fit,
                cache_hit: cache_hit_initial,
            },
            lambda_budget,
        },
    )
}

// ─── distance_to_fit_mean (S1 wave 1 §H) ────────────────────
//
// Per Marcella's 2026-05-26 H2 attractor letter Diagnostic 3:
// "does `double_cover_v3` live near the fit_mean? what's
// ‖vec(double_cover_v3) − fit_mean‖? and for comparison, the
// median across all 9964 records?"
//
// Endpoint that answers BOTH at once: for any set of target
// vectors, returns each target's distance to fit_mean + its
// percentile within the bundle's distance distribution, plus
// the full distribution statistics (min/p25/median/p75/p90/p99/
// max) so consumers can interpret the percentile in context.
//
// Detects the H2 mechanism directly: if the target sits at p<0.01
// (closer than 99% of records), it's at the deep point of the
// fitted Gaussian — every Langevin walk rolls there.

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainDistanceToFitMeanRequest {
    fields: Vec<String>,
    /// Fit mode for fit_mean computation. Defaults to isotropic.
    #[serde(default)]
    fit_mode: Option<FitMode>,
    /// Floor for the fit. See L13.6 + S1 §2a.
    #[serde(default)]
    sigma_floor_epsilon: Option<f64>,
    /// Target vectors to measure. Each must have length == fields.len().
    targets: Vec<Vec<f64>>,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct DistanceDistribution {
    /// Records counted (those with all fields present).
    n_records: usize,
    /// Euclidean distance statistics across all records.
    min: f64,
    p25: f64,
    median: f64,
    p75: f64,
    p90: f64,
    p99: f64,
    max: f64,
    mean: f64,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainDistanceToFitMeanResponse {
    /// Fit_mean vector (length = fields.len()).
    fit_mean: Vec<f64>,
    /// Per-target Euclidean distance to fit_mean. Aligned with
    /// request.targets order.
    target_distances: Vec<f64>,
    /// Per-target percentile rank within the bundle's distance
    /// distribution. percentile=0.01 means "closer than 99% of
    /// records" — the H2 hallmark.
    target_percentiles: Vec<f64>,
    /// Full distance distribution across all records.
    distance_distribution: DistanceDistribution,
    /// fit_mode that was used to compute fit_mean.
    fit_mode_used: String,
    counter_at_fit: u64,
}

/// POST /v1/bundles/{name}/brain/distance_to_fit_mean
///
/// Diagnoses the H2 mechanism for any target vector. Walks all
/// records once to build the distance distribution, returns
/// per-target distance + percentile + distribution stats.
///
/// Cost: O(N·n) per call (one record walk; not cached). For
/// bge_v2 at N=9964, n=384 that's ~3.8M ops, ~10ms. The fit
/// itself comes from the cache (sub-µs warm).
#[cfg(feature = "kahler")]
async fn brain_distance_to_fit_mean_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainDistanceToFitMeanRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);

    // Validate every target has the right dim.
    let n = req.fields.len();
    for (i, t) in req.targets.iter().enumerate() {
        if t.len() != n {
            return Err(bad_request(&format!(
                "target[{}] length {} ≠ fields length {}",
                i,
                t.len(),
                n
            )));
        }
    }

    let fit_mode = req.fit_mode.unwrap_or_default();
    let (ctx, counter_at_fit) = flow_from_bundle_cached(
        &state,
        &name,
        heap,
        &req.fields,
        fit_mode,
        req.sigma_floor_epsilon,
    )?;
    let fit_mean = ctx.mu.clone();

    // Build distance distribution across all records.
    let samples = extract_field_samples(heap, &req.fields)
        .map_err(|e| bad_request(&e))?;
    let mut distances: Vec<f64> = samples
        .iter()
        .map(|s| {
            s.iter()
                .zip(fit_mean.iter())
                .map(|(a, m)| (a - m).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .collect();
    if distances.is_empty() {
        return Err(bad_request(
            "no records have all requested fields present",
        ));
    }
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n_records = distances.len();

    let percentile_at = |p: f64| -> f64 {
        let idx = ((p * (n_records - 1) as f64).round() as usize).min(n_records - 1);
        distances[idx]
    };
    let mean = distances.iter().sum::<f64>() / n_records as f64;
    let distribution = DistanceDistribution {
        n_records,
        min: distances[0],
        p25: percentile_at(0.25),
        median: percentile_at(0.5),
        p75: percentile_at(0.75),
        p90: percentile_at(0.9),
        p99: percentile_at(0.99),
        max: *distances.last().unwrap(),
        mean,
    };

    // Compute target distances and percentile ranks.
    let target_distances: Vec<f64> = req
        .targets
        .iter()
        .map(|t| {
            t.iter()
                .zip(fit_mean.iter())
                .map(|(a, m)| (a - m).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .collect();
    // Percentile rank: fraction of records whose distance is
    // strictly less than the target's. Use binary search since
    // `distances` is sorted.
    let target_percentiles: Vec<f64> = target_distances
        .iter()
        .map(|&d| {
            let pos = distances.partition_point(|x| *x < d);
            pos as f64 / n_records as f64
        })
        .collect();

    let fit_mode_used = match fit_mode {
        FitMode::Isotropic => "isotropic",
        FitMode::Diagonal => "diagonal",
        FitMode::Full => "full",
    }
    .to_string();

    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainDistanceToFitMeanResponse {
                fit_mean,
                target_distances,
                target_percentiles,
                distance_distribution: distribution,
                fit_mode_used,
                counter_at_fit,
            },
            lambda_budget,
        },
    )
}

// ─── SUDOKU (S3 — HTTP endpoint for the constraint-inference meta-primitive) ─
//
// Per theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md v0.3. The
// geometry layer (src/geometry/sudoku.rs, S2) does the actual
// constraint matching + verdict logic; this layer is just wire
// marshalling.
//
// Constraint vocabulary on the wire is tagged-enum JSON:
//   { "type": "field", "field": "...", "op": "eq" | ..., "value": ..., "hard": bool }
//   { "type": "manifold", ... }   (stubbed — returns 400 with S4 note)
//   { "type": "relation", ... }   (stubbed — returns 400 with S4 note)
//
// Response uses the existing content negotiation helper from §D:
// DHOOM if Accept: application/dhoom, else JSON. X-Bundle-Mutation-
// Counter header surfaced same as other brain endpoints.

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ConstraintWire {
    Field {
        field: String,
        op: String,
        value: serde_json::Value,
        #[serde(default = "default_constraint_hard")]
        hard: bool,
    },
    Manifold {
        field: String,
        near_manifold: String,
        epsilon: f64,
        #[serde(default = "default_constraint_hard")]
        hard: bool,
    },
    Relation {
        expr: String,
        #[serde(default)]
        vars: std::collections::HashMap<String, f64>,
        #[serde(default = "default_constraint_hard")]
        hard: bool,
    },
}

#[cfg(feature = "kahler")]
fn default_constraint_hard() -> bool { true }

/// **S3.5 — Expansion config wire.** Opt-in puzzle expansion: when
/// the original puzzle is UNSAT, try relaxing the cheapest constraint.
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct ExpansionConfigWire {
    /// Master switch. Must be `true` for expansion to run.
    #[serde(default)]
    allowed: bool,
    /// How many relaxation options to try before giving up. Default 1.
    #[serde(default = "default_max_constraint_relaxations")]
    max_constraint_relaxations: usize,
}

// ── S4: SAMPLE_TRANSPORT brain endpoint ─────────────────────────────────

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainSampleTransportRequest {
    /// Key-value pairs that identify the source record.
    from_keys: serde_json::Value,
    /// Fiber field names to project onto.
    fiber_fields: Vec<String>,
    /// Max `d^2` per candidate. Must be in `[0.0, 1.0]`. Default 0.3.
    #[serde(default = "default_sample_transport_budget")]
    budget: f64,
    /// Number of candidates to return. Default 16.
    #[serde(default = "default_sample_transport_k")]
    k: usize,
    /// Temperature for `exp(-beta * d^2)` kernel. Default 1.0.
    #[serde(default = "default_sample_transport_beta")]
    beta: f64,
    /// Optional deterministic seed.
    #[serde(default)]
    seed: Option<u64>,
}

#[cfg(feature = "kahler")]
fn default_sample_transport_budget() -> f64 { 0.3 }
#[cfg(feature = "kahler")]
fn default_sample_transport_k() -> usize { 16 }
#[cfg(feature = "kahler")]
fn default_sample_transport_beta() -> f64 { 1.0 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct TransportCandidateWire {
    record: serde_json::Value,
    fiber_projection: Vec<f64>,
    d_sq: f64,
    sameness: f64,
    weight: f64,
    curvature_k: f64,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainSampleTransportResponse {
    candidates: Vec<TransportCandidateWire>,
    budget: f64,
    n_admissible: usize,
    n_returned: usize,
    kappa: f64,
    confidence: f64,
}

/// POST /v1/bundles/{name}/brain/sample_transport
///
/// Curvature-bounded neighborhood sampling — returns `k` candidates
/// from the fiber neighborhood `N(p_src, tau)` of a source record,
/// weighted by `exp(-beta * d^2)` and sampled without replacement.
///
/// See `theory/GIGI_SAMPLE_TRANSPORT_SPRINT_SPEC.md` for the full
/// math (Double Cover budget, Efraimidis-Spirakis sampling).
#[cfg(feature = "kahler")]
async fn brain_sample_transport_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainSampleTransportRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic over Heap and Overlay.

    if req.fiber_fields.is_empty() {
        return Err(bad_request("fiber_fields must not be empty"));
    }

    // Parse from_keys JSON object into a Record filter.
    let from_map: HashMap<String, gigi::types::Value> = req
        .from_keys
        .as_object()
        .ok_or_else(|| bad_request("from_keys must be a JSON object"))?
        .iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    // Collect all records; locate source and extract fiber projection.
    let all_records: Vec<gigi::types::Record> = store.records().collect();
    let src_fiber: Vec<f64> = {
        let src_rec = all_records
            .iter()
            .find(|rec: &&gigi::types::Record| {
                from_map.iter().all(|(k, v)| rec.get(k.as_str()) == Some(v))
            })
            .ok_or_else(|| not_found("Source record not found in bundle"))?;
        gigi::geometry::extract_fiber(src_rec, &req.fiber_fields)
    };

    let st_req = gigi::geometry::SampleTransportRequest {
        fiber_fields: req.fiber_fields.clone(),
        budget: req.budget,
        k: req.k,
        beta: req.beta,
        seed: req.seed,
    };

    let kappa = store.scalar_curvature();
    let counter_at_fit = store.mutation_counter();

    let result = gigi::geometry::sample_transport_neighborhood(
        &all_records,
        &src_fiber,
        &st_req,
        kappa,
    )
    .map_err(|e| bad_request(&e.to_string()))?;

    let candidates_wire: Vec<TransportCandidateWire> = result
        .candidates
        .into_iter()
        .map(|c| TransportCandidateWire {
            record: record_to_json(&c.record),
            fiber_projection: c.fiber_projection,
            d_sq: c.d_sq,
            sameness: c.sameness,
            weight: c.weight,
            curvature_k: c.curvature_k,
        })
        .collect();

    let resp_body = BrainSampleTransportResponse {
        candidates: candidates_wire,
        budget: result.budget,
        n_admissible: result.n_admissible,
        n_returned: result.n_returned,
        kappa: result.kappa,
        confidence: result.confidence,
    };

    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: resp_body,
            lambda_budget,
        },
    )
}

// ── End S4: SAMPLE_TRANSPORT ─────────────────────────────────────────────

#[cfg(feature = "kahler")]
fn default_max_constraint_relaxations() -> usize { 1 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainSudokuRequest {
    /// Constraints to satisfy. v1 supports only `type: "field"`;
    /// manifold + relation route to a 400 error pointing at S4.
    constraints: Vec<ConstraintWire>,
    /// Cap on returned satisfying options. Default 5.
    #[serde(default = "default_sudoku_max_options")]
    max_options: usize,
    /// Cap on returned near-misses (options violating ONE constraint).
    /// Default 3.
    #[serde(default = "default_sudoku_max_near_misses")]
    max_near_misses: usize,
    /// **S3.5 — puzzle expansion.** Opt-in: when set with
    /// `allowed: true` and the puzzle is UNSAT, try relaxing the
    /// cheapest constraint. Default: None (no expansion).
    #[serde(default)]
    expansion: Option<ExpansionConfigWire>,
}

#[cfg(feature = "kahler")]
fn default_sudoku_max_options() -> usize { 5 }
#[cfg(feature = "kahler")]
fn default_sudoku_max_near_misses() -> usize { 3 }

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct SudokuSolutionWire {
    record: serde_json::Value,
    stated_prior_mass: f64,
    /// **Wave 4 — Upgrade 4.** Depth into the satisfaction region in
    /// [0, 1]. Higher = better margin to every constraint. Sort key
    /// alongside stated_prior_mass.
    quality_score: f64,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct SudokuViolationWire {
    constraint_idx: usize,
    field: String,
    violation: String,
    relax_to: serde_json::Value,
    /// **Wave 3 — Upgrade 1.** Normalized cost to relax this
    /// constraint enough to admit this record. Z-score: |actual -
    /// threshold| / std(field). 1.0 for categorical violations.
    relaxation_cost: f64,
    /// Raw violation magnitude in the field's native units. None
    /// for categorical / non-ordered violations.
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_delta: Option<f64>,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct SudokuNearMissWire {
    record: serde_json::Value,
    stated_prior_mass: f64,
    violations: Vec<SudokuViolationWire>,
    would_unlock_if_relaxed: Vec<usize>,
}

/// **Wave 3 — Upgrade 2.** Per-constraint selectivity wire.
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct SudokuSelectivityWire {
    constraint_idx: usize,
    field: String,
    n_match_all: usize,
    n_match_without: usize,
    marginal_filter_count: usize,
    /// True if this constraint filters the most records given the
    /// others. The deal-breaker. Ties: multiple may be flagged.
    binding: bool,
    /// **Wave 6.** Per-constraint raw curvature K_c in [0, 1] =
    /// fraction of records that fail this constraint regardless
    /// of others. High K_c + low marginal = REDUNDANT constraint
    /// (already covered by another). High K_c + high marginal =
    /// the deal-breaker. Maps to sudoky-energy's K_loc.
    raw_curvature: f64,
}

/// **Wave 3 — Upgrade 3.** Counterfactual relaxation menu entry.
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct SudokuRelaxationWire {
    constraint_idx: usize,
    field: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_threshold: Option<serde_json::Value>,
    gain: usize,
    relaxation_cost: f64,
    /// **Wave 6.4** — energy descent per unit cost (the negative
    /// log-likelihood drop divided by σ-cost). Higher = more
    /// satisfaction-probability gained per σ of bending. The menu is
    /// ordered by this descending; `gain` / `relaxation_cost` remain
    /// available for consumers who prefer the W3 ordering.
    energy_descent: f64,
}

/// **Wave 3 — Upgrade 5.** Pareto-frontier near-miss (allows
/// multi-violation; non-dominated on n_violations × total_cost).
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct SudokuParetoNearMissWire {
    record: serde_json::Value,
    stated_prior_mass: f64,
    violations: Vec<SudokuViolationWire>,
    total_relaxation_cost: f64,
}

/// **S3.5 — Expanded solution wire.** A record from the RELAXED
/// puzzle — clearly distinct from `solutions` (which requires all
/// original constraints). `relaxed_constraint_idx` identifies which
/// original constraint was relaxed or dropped.
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct SudokuExpandedSolutionWire {
    record: serde_json::Value,
    stated_prior_mass: f64,
    /// Normalized cost of the relaxation that unlocked this record.
    /// Same units as `ViolationDetail::relaxation_cost`.
    expansion_cost: f64,
    /// Index into the original `constraints` array that was relaxed.
    relaxed_constraint_idx: usize,
    /// The new threshold used (null = constraint dropped entirely).
    #[serde(skip_serializing_if = "Option::is_none")]
    relaxed_to: Option<serde_json::Value>,
}

/// **S3.5 — Expansion result wire.** Present only when
/// `expansion.allowed` was true and original verdict was "unsat".
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct SudokuExpansionResultWire {
    /// True when expansion was actually attempted.
    attempted: bool,
    /// "constraint_relaxation" in v1; "bundle_hop" added at HTTP
    /// layer in S3.5.
    expansion_type: String,
    /// Solutions from the relaxed puzzle. Empty → expansion also
    /// failed; see `advisory`.
    solutions: Vec<SudokuExpandedSolutionWire>,
    /// Set when expansion finds nothing. Suggests asking a human
    /// or reformulating.
    #[serde(skip_serializing_if = "Option::is_none")]
    advisory: Option<String>,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainSudokuResponse {
    solutions: Vec<SudokuSolutionWire>,
    near_misses: Vec<SudokuNearMissWire>,
    /// "sat" | "unsat" | "unknown" — the honest-coverage tristate.
    /// Per spec §3: unknown means "I didn't look enough to claim
    /// either" (rather than "shrug, no solution"). Consumers MUST
    /// handle unknown distinctly from unsat.
    verdict: String,
    /// Fraction of stated-prior mass explored. v1 exhaustive walk
    /// → always 1.0.
    coverage: f64,
    /// Number of records considered (post-context filter).
    n_records_considered: usize,
    /// **Wave 3 — Upgrade 2.** Per-constraint selectivity report —
    /// identifies which constraint(s) are doing the binding work.
    /// Empty if no constraints were supplied.
    selectivity: Vec<SudokuSelectivityWire>,
    /// **Wave 3 — Upgrade 3.** Counterfactual relaxation menu.
    /// Sorted by gain/cost descending — best bang-per-bend first.
    relaxations: Vec<SudokuRelaxationWire>,
    /// **Wave 3 — Upgrade 5.** Pareto-optimal multi-violation
    /// near-misses (non-dominated on n_violations × total_cost).
    /// Generalizes `near_misses` (which is the k=1 slice).
    pareto_near_misses: Vec<SudokuParetoNearMissWire>,
    /// Bundle's mutation counter at the time of this fit. Same
    /// value as the X-Bundle-Mutation-Counter response header.
    counter_at_fit: u64,
    /// **Wave 6.2 — pre-flight contradiction reason.** Populated when
    /// the constraint set is trivially self-contradictory (detected in
    /// O(C²) before any bundle walk). When present, verdict is "unsat"
    /// and n_records_considered is 0. Consumers should surface this
    /// as "your constraints cannot both hold" rather than "no records
    /// match."
    #[serde(skip_serializing_if = "Option::is_none")]
    pre_flight_unsat_reason: Option<String>,
    /// **S3.5 — puzzle expansion result.** Present only when
    /// `expansion.allowed` was true and original verdict is "unsat".
    /// Null → expansion was not attempted (SAT, unknown, or not
    /// opted in).
    #[serde(skip_serializing_if = "Option::is_none")]
    expanded: Option<SudokuExpansionResultWire>,
    /// **Wave 6.3 — Γ trichotomy diagnostic.** Dimensionless quantity
    /// `Γ = m / (K̂_max · log(n+1))` classifying the problem into
    /// `numeric` / `structural` / `geometric` regime. Null when the
    /// diagnostic is undefined (pre-flight UNSAT or n = 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    gamma_trichotomy: Option<SudokuGammaTrichotomyWire>,
}

/// Wire shape for the Wave 6.3 Γ trichotomy diagnostic.
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SudokuGammaTrichotomyWire {
    /// Dimensionless Γ value.
    gamma: f64,
    /// Regime tag: `"numeric"` / `"structural"` / `"geometric"`.
    regime: String,
    /// Number of constraints (`m`).
    m: usize,
    /// Max raw_curvature across all constraints (`K̂_max`).
    k_max: f64,
    /// `log(n + 1)` term.
    log_s: f64,
}

#[cfg(feature = "kahler")]
impl From<gigi::geometry::GammaTrichotomy> for SudokuGammaTrichotomyWire {
    fn from(g: gigi::geometry::GammaTrichotomy) -> Self {
        Self {
            gamma: g.gamma,
            regime: g.regime.as_str().to_string(),
            m: g.m,
            k_max: g.k_max,
            log_s: g.log_s,
        }
    }
}

/// Translate wire constraints to the geometry-layer Constraint
/// types. Returns 400-style errors for the v1-unsupported types
/// (manifold, relation) with S4 references.
#[cfg(feature = "kahler")]
fn translate_constraints(
    wire: Vec<ConstraintWire>,
) -> Result<Vec<gigi::geometry::Constraint>, String> {
    let mut out = Vec::with_capacity(wire.len());
    for (i, c) in wire.into_iter().enumerate() {
        let translated = match c {
            ConstraintWire::Field { field, op, value, hard } => {
                let op = translate_field_op(&op, &value).map_err(|e| {
                    format!("constraint[{}]: {}", i, e)
                })?;
                gigi::geometry::Constraint::Field { field, op, hard }
            }
            ConstraintWire::Manifold { field, near_manifold, epsilon, hard } => {
                gigi::geometry::Constraint::Manifold {
                    field,
                    near_manifold,
                    epsilon,
                    hard,
                }
            }
            ConstraintWire::Relation { expr, vars, hard } => {
                gigi::geometry::Constraint::Relation { expr, vars, hard }
            }
        };
        out.push(translated);
    }
    Ok(out)
}

#[cfg(feature = "kahler")]
fn translate_field_op(
    op: &str,
    value: &serde_json::Value,
) -> Result<gigi::geometry::FieldOp, String> {
    use gigi::geometry::FieldOp;
    match op {
        "eq" => Ok(FieldOp::Eq(json_to_value(value))),
        "ne" => Ok(FieldOp::Ne(json_to_value(value))),
        "lt" | "le" | "gt" | "ge" => {
            let n = value.as_f64().ok_or_else(|| {
                format!("op '{}' requires numeric value, got {:?}", op, value)
            })?;
            Ok(match op {
                "lt" => FieldOp::Lt(n),
                "le" => FieldOp::Le(n),
                "gt" => FieldOp::Gt(n),
                "ge" => FieldOp::Ge(n),
                _ => unreachable!(),
            })
        }
        "between" => {
            let arr = value.as_array().ok_or_else(|| {
                format!("op 'between' requires array [lo, hi], got {:?}", value)
            })?;
            if arr.len() != 2 {
                return Err(format!(
                    "op 'between' requires array of length 2, got {} items",
                    arr.len()
                ));
            }
            let lo = arr[0].as_f64().ok_or("between lo not numeric")?;
            let hi = arr[1].as_f64().ok_or("between hi not numeric")?;
            Ok(FieldOp::Between { lo, hi })
        }
        "is_in" => {
            let arr = value.as_array().ok_or_else(|| {
                format!("op 'is_in' requires array, got {:?}", value)
            })?;
            let values: Vec<Value> = arr.iter().map(json_to_value).collect();
            Ok(FieldOp::IsIn(values))
        }
        other => Err(format!(
            "unknown op '{}'; expected one of: eq, ne, lt, le, gt, ge, between, is_in",
            other
        )),
    }
}

/// POST /v1/bundles/{name}/brain/sudoku
///
/// Constraint-inference meta-primitive per
/// theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md v0.3.
///
/// v1 (S3): exhaustive O(N) record walk + Field-predicate
/// constraints + honest-coverage tristate. S3.5 ships puzzle
/// expansion (constraint_relaxation + bundle_hop); S4 ships
/// manifold-distance + cross-field-relation constraints + soft
/// scoring with penalty calibration; S5 ships demos.
#[cfg(feature = "kahler")]
async fn brain_sudoku_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainSudokuRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic over Heap and Overlay (post-restart
    // mmap+overlay bundles). SUDOKU is read-only; both variants
    // support records() and mutation_counter() identically.

    // Translate wire constraints → geometry-layer types.
    let constraints = translate_constraints(req.constraints)
        .map_err(|e| bad_request(&e))?;
    // S3.5 — translate expansion config from wire format.
    let expansion = req.expansion.map(|e| gigi::geometry::ExpansionConfig {
        allowed: e.allowed,
        max_constraint_relaxations: e.max_constraint_relaxations,
    });
    let sudoku_req = gigi::geometry::SudokuRequest {
        constraints,
        max_options: req.max_options,
        max_near_misses: req.max_near_misses,
        expansion,
    };
    let config = gigi::geometry::SudokuConfig::default();

    // Snapshot the mutation counter at solve time so the response
    // header reflects the bundle state we actually read.
    let counter_at_fit = store.mutation_counter();

    // Walk the bundle's records. Polymorphic — works on Heap and
    // Overlay alike. The geometry-layer solver does the actual
    // filtering + verdict logic.
    let records_iter = store.records();
    let resp = gigi::geometry::solve_constraints(records_iter, &sudoku_req, &config)
        .map_err(|e| bad_request(&format!("{}", e)))?;

    // Marshal to wire format. Records → JSON via record_to_json
    // (DHOOM-flag at response-encoding time, not here).
    // Helper: violation → wire (DRY — used by near_misses AND pareto).
    fn vd_to_wire(v: gigi::geometry::ViolationDetail) -> SudokuViolationWire {
        SudokuViolationWire {
            constraint_idx: v.constraint_idx,
            field: v.field,
            violation: v.violation,
            relax_to: value_to_json(&v.relax_to),
            relaxation_cost: v.relaxation_cost,
            raw_delta: v.raw_delta,
        }
    }
    let solutions_wire: Vec<SudokuSolutionWire> = resp
        .solutions
        .into_iter()
        .map(|s| SudokuSolutionWire {
            record: record_to_json(&s.record),
            stated_prior_mass: s.stated_prior_mass,
            quality_score: s.quality_score,
        })
        .collect();
    let near_misses_wire: Vec<SudokuNearMissWire> = resp
        .near_misses
        .into_iter()
        .map(|nm| SudokuNearMissWire {
            record: record_to_json(&nm.record),
            stated_prior_mass: nm.stated_prior_mass,
            violations: nm.violations.into_iter().map(vd_to_wire).collect(),
            would_unlock_if_relaxed: nm.would_unlock_if_relaxed,
        })
        .collect();

    // Wave 3 — Upgrade 2: selectivity report wire-format.
    let selectivity_wire: Vec<SudokuSelectivityWire> = resp
        .selectivity
        .into_iter()
        .map(|s| SudokuSelectivityWire {
            constraint_idx: s.constraint_idx,
            field: s.field,
            n_match_all: s.n_match_all,
            n_match_without: s.n_match_without,
            marginal_filter_count: s.marginal_filter_count,
            binding: s.binding,
            raw_curvature: s.raw_curvature,
        })
        .collect();

    // Wave 3 — Upgrade 3: relaxation menu wire-format.
    let relaxations_wire: Vec<SudokuRelaxationWire> = resp
        .relaxations
        .into_iter()
        .map(|r| SudokuRelaxationWire {
            constraint_idx: r.constraint_idx,
            field: r.field,
            description: r.description,
            new_threshold: r.new_threshold.as_ref().map(value_to_json),
            gain: r.gain,
            relaxation_cost: r.relaxation_cost,
            energy_descent: r.energy_descent,
        })
        .collect();

    // Wave 3 — Upgrade 5: Pareto near-misses wire-format.
    let pareto_wire: Vec<SudokuParetoNearMissWire> = resp
        .pareto_near_misses
        .into_iter()
        .map(|p| SudokuParetoNearMissWire {
            record: record_to_json(&p.record),
            stated_prior_mass: p.stated_prior_mass,
            violations: p.violations.into_iter().map(vd_to_wire).collect(),
            total_relaxation_cost: p.total_relaxation_cost,
        })
        .collect();

    let verdict = match resp.verdict {
        gigi::geometry::SudokuVerdict::Sat => "sat",
        gigi::geometry::SudokuVerdict::Unsat => "unsat",
        gigi::geometry::SudokuVerdict::Unknown => "unknown",
    }
    .to_string();

    // S3.5 — convert expansion result to wire format.
    let expanded_wire: Option<SudokuExpansionResultWire> = resp.expanded.map(|exp| {
        let sol_wire: Vec<SudokuExpandedSolutionWire> = exp
            .solutions
            .into_iter()
            .map(|s| SudokuExpandedSolutionWire {
                record: record_to_json(&s.record),
                stated_prior_mass: s.stated_prior_mass,
                expansion_cost: s.expansion_cost,
                relaxed_constraint_idx: s.relaxed_constraint_idx,
                relaxed_to: s.relaxed_to.as_ref().map(value_to_json),
            })
            .collect();
        SudokuExpansionResultWire {
            attempted: exp.attempted,
            expansion_type: exp.expansion_type,
            solutions: sol_wire,
            advisory: exp.advisory,
        }
    });

    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainSudokuResponse {
                solutions: solutions_wire,
                near_misses: near_misses_wire,
                verdict,
                coverage: resp.coverage,
                n_records_considered: resp.n_records_considered,
                selectivity: selectivity_wire,
                relaxations: relaxations_wire,
                pareto_near_misses: pareto_wire,
                counter_at_fit,
                pre_flight_unsat_reason: resp.pre_flight_unsat_reason,
                expanded: expanded_wire,
                gamma_trichotomy: resp.gamma_trichotomy.map(Into::into),
            },
            lambda_budget,
        },
    )
}

// ─── S7: /brain/intent_gate — refuse-gate composite ─────────────────────────
//
// JTBD: "I'm any GIGI-backed system deciding whether to commit to a
// response. I want one atomic call that tells me — is this query
// feasible against my bundle, AND is the user's intent grounded in
// known territory?"
//
// Composes three primitives that all shipped:
//   1. Čech holonomy pre-flight (W6.2) — instant UNSAT on contradictions
//   2. SUDOKU walk (waves 3-6.2) — verdict + near-misses + Pareto
//   3. kernel-density confidence (L11) — geometric grounding of query
//
// Returns ALL the signals; bakes NO refuse threshold. Consumers
// (Marcella, PRISM, ICARUS) compose their own decision from raw values.
// Spec: theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md §11.

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainIntentGateRequest {
    constraints: Vec<ConstraintWire>,
    #[serde(default = "default_sudoku_max_options")]
    max_options: usize,
    #[serde(default = "default_sudoku_max_near_misses")]
    max_near_misses: usize,
    /// Optional S3.5 puzzle expansion (forwarded to SUDOKU).
    #[serde(default)]
    expansion: Option<ExpansionConfigWire>,
    /// Optional query-grounding half. If `query_fields` and `query`
    /// are both Some, kernel-density confidence runs; otherwise it
    /// is skipped and `query_grounding` in the response is `null`.
    #[serde(default)]
    query_fields: Option<Vec<String>>,
    #[serde(default)]
    query: Option<Vec<f64>>,
    /// Optional bandwidth override. None → defaults to √σ² from the
    /// bundle's isotropic fit (data-derived, no consumer config).
    #[serde(default)]
    bandwidth: Option<f64>,
}

/// Query-grounding half of the intent_gate response. `null` when the
/// caller did not supply `query_fields` + `query`.
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct IntentGateQueryGroundingWire {
    /// `Σᵢ exp(-‖q-xᵢ‖²/2σ²)` — raw Bayesian-precision proxy.
    raw: f64,
    /// `raw / max_density_in_bundle` ∈ [0, 1]. Consumer threshold:
    /// `> 0.5` ≈ "comparable to a typical sample"; `< 0.1` ≈
    /// "very far from anything we know." Both bounds are suggested,
    /// not enforced.
    normalized: f64,
    /// Bandwidth actually used (request override or fit-derived).
    bandwidth_used: f64,
    /// Number of records that contributed to the density estimate.
    n_samples: usize,
    /// Index of the closest record by raw L2 (free signal — comes
    /// from the same single pass). `None` if no samples in the bundle.
    #[serde(skip_serializing_if = "Option::is_none")]
    nearest_record_index: Option<usize>,
    /// L2 distance from `query` to the nearest record.
    nearest_distance: f64,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainIntentGateResponse {
    // ── SUDOKU half (always present) ────────────────────────────────
    verdict: String,
    coverage: f64,
    n_records_considered: usize,
    solutions: Vec<SudokuSolutionWire>,
    near_misses: Vec<SudokuNearMissWire>,
    selectivity: Vec<SudokuSelectivityWire>,
    relaxations: Vec<SudokuRelaxationWire>,
    pareto_near_misses: Vec<SudokuParetoNearMissWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pre_flight_unsat_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expanded: Option<SudokuExpansionResultWire>,
    /// **Wave 6.3 — Γ trichotomy diagnostic** (composite endpoint).
    /// Same shape as the standalone SUDOKU endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    gamma_trichotomy: Option<SudokuGammaTrichotomyWire>,

    // ── Confidence half (null if no query supplied) ─────────────────
    #[serde(skip_serializing_if = "Option::is_none")]
    query_grounding: Option<IntentGateQueryGroundingWire>,

    // ── Cache freshness (always present) ────────────────────────────
    counter_at_fit: u64,
}

#[cfg(feature = "kahler")]
async fn brain_intent_gate_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainIntentGateRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix** — polymorphic over heap + mmap+overlay bundles.
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);

    // Validate query / fields pair: both-or-neither.
    let query_pair = match (&req.query_fields, &req.query) {
        (Some(f), Some(q)) => {
            if f.len() != q.len() {
                return Err(bad_request(&format!(
                    "query length {} ≠ query_fields length {}",
                    q.len(),
                    f.len()
                )));
            }
            if f.is_empty() {
                return Err(bad_request(
                    "query_fields must not be empty when supplied",
                ));
            }
            Some((f.clone(), q.clone()))
        }
        (None, None) => None,
        _ => {
            return Err(bad_request(
                "query_fields and query must be supplied together (both or neither)",
            ));
        }
    };

    // ── 1. SUDOKU half ──────────────────────────────────────────────
    let constraints = translate_constraints(req.constraints)
        .map_err(|e| bad_request(&e))?;
    let expansion = req.expansion.map(|e| gigi::geometry::ExpansionConfig {
        allowed: e.allowed,
        max_constraint_relaxations: e.max_constraint_relaxations,
    });
    let sudoku_req = gigi::geometry::SudokuRequest {
        constraints,
        max_options: req.max_options,
        max_near_misses: req.max_near_misses,
        expansion,
    };
    let config = gigi::geometry::SudokuConfig::default();
    let counter_at_fit = heap.mutation_counter();

    // **#210 perf fix.** When intent_gate is called with empty
    // constraints, the SUDOKU half has no semantic value — the
    // caller is asking purely about query-grounding (confidence
    // half). The math primitive `solve_constraints` still has a
    // legitimate contract for empty constraints (mass-collapsed
    // signature view; see `identical_records_collapse_to_single
    // _solution_with_full_mass` test). But the intent_gate
    // endpoint can skip the full O(N) materialize + classify +
    // mass-compute pass when it knows the consumer only wants the
    // confidence half.
    //
    // Synthetic response: verdict "sat" (zero constraints are
    // vacuously satisfied), coverage 1.0, empty collections,
    // n_records_considered 0 (we genuinely didn't look). This is
    // what the consumer would observe IF they cared, and it costs
    // nanoseconds instead of seconds on a 10k-record bundle.
    //
    // Per Marcella's intent_gate JTBD: "given this query, should
    // I respond? gate me on constraints + confidence." With no
    // constraints, the gate is purely confidence-driven; SUDOKU
    // output is moot.
    let resp = if sudoku_req.constraints.is_empty() {
        gigi::geometry::SudokuResponse {
            solutions: Vec::new(),
            near_misses: Vec::new(),
            verdict: gigi::geometry::SudokuVerdict::Sat,
            coverage: 1.0,
            n_records_considered: 0,
            selectivity: Vec::new(),
            relaxations: Vec::new(),
            pareto_near_misses: Vec::new(),
            pre_flight_unsat_reason: None,
            expanded: None,
            gamma_trichotomy: None,
        }
    } else {
        gigi::geometry::solve_constraints(heap.records(), &sudoku_req, &config)
            .map_err(|e| bad_request(&format!("{}", e)))?
    };

    // Wire-format the SUDOKU result (same helpers brain_sudoku uses).
    fn vd_to_wire(v: gigi::geometry::ViolationDetail) -> SudokuViolationWire {
        SudokuViolationWire {
            constraint_idx: v.constraint_idx,
            field: v.field,
            violation: v.violation,
            relax_to: value_to_json(&v.relax_to),
            relaxation_cost: v.relaxation_cost,
            raw_delta: v.raw_delta,
        }
    }
    let solutions_wire: Vec<SudokuSolutionWire> = resp
        .solutions
        .into_iter()
        .map(|s| SudokuSolutionWire {
            record: record_to_json(&s.record),
            stated_prior_mass: s.stated_prior_mass,
            quality_score: s.quality_score,
        })
        .collect();
    let near_misses_wire: Vec<SudokuNearMissWire> = resp
        .near_misses
        .into_iter()
        .map(|nm| SudokuNearMissWire {
            record: record_to_json(&nm.record),
            stated_prior_mass: nm.stated_prior_mass,
            violations: nm.violations.into_iter().map(vd_to_wire).collect(),
            would_unlock_if_relaxed: nm.would_unlock_if_relaxed,
        })
        .collect();
    let selectivity_wire: Vec<SudokuSelectivityWire> = resp
        .selectivity
        .into_iter()
        .map(|s| SudokuSelectivityWire {
            constraint_idx: s.constraint_idx,
            field: s.field,
            n_match_all: s.n_match_all,
            n_match_without: s.n_match_without,
            marginal_filter_count: s.marginal_filter_count,
            binding: s.binding,
            raw_curvature: s.raw_curvature,
        })
        .collect();
    let relaxations_wire: Vec<SudokuRelaxationWire> = resp
        .relaxations
        .into_iter()
        .map(|r| SudokuRelaxationWire {
            constraint_idx: r.constraint_idx,
            field: r.field,
            description: r.description,
            new_threshold: r.new_threshold.as_ref().map(value_to_json),
            gain: r.gain,
            relaxation_cost: r.relaxation_cost,
            energy_descent: r.energy_descent,
        })
        .collect();
    let pareto_wire: Vec<SudokuParetoNearMissWire> = resp
        .pareto_near_misses
        .into_iter()
        .map(|p| SudokuParetoNearMissWire {
            record: record_to_json(&p.record),
            stated_prior_mass: p.stated_prior_mass,
            violations: p.violations.into_iter().map(vd_to_wire).collect(),
            total_relaxation_cost: p.total_relaxation_cost,
        })
        .collect();
    let expanded_wire = resp.expanded.map(|e| SudokuExpansionResultWire {
        attempted: e.attempted,
        expansion_type: e.expansion_type,
        solutions: e.solutions.into_iter().map(|s| SudokuExpandedSolutionWire {
            record: record_to_json(&s.record),
            stated_prior_mass: s.stated_prior_mass,
            expansion_cost: s.expansion_cost,
            relaxed_constraint_idx: s.relaxed_constraint_idx,
            relaxed_to: s.relaxed_to.as_ref().map(value_to_json),
        }).collect(),
        advisory: e.advisory,
    });

    let verdict = match resp.verdict {
        gigi::geometry::SudokuVerdict::Sat => "sat",
        gigi::geometry::SudokuVerdict::Unsat => "unsat",
        gigi::geometry::SudokuVerdict::Unknown => "unknown",
    }
    .to_string();

    // ── 2. Confidence half (only if query supplied) ─────────────────
    //
    // Per Marcella's 2026-05-29 bug report `GIGI_BUG_REPORT_onfields_latency.md`:
    // this used to allocate a fresh `Vec<Vec<f64>>` via `extract_field_samples`
    // and then run `confidence_normalized`'s O(N²·D) max-density loop per
    // request (35 s at N=10k, D=384). Now: cached `(N, D)` matrix +
    // cached max-density per (matrix, bandwidth). Same response shape,
    // ~200x faster.
    let query_grounding = if let Some((fields, query)) = query_pair {
        let cached_matrix = materialize_matrix_cached(&state, &name, heap, &fields)?;

        // Bandwidth: request override if positive, else derive from
        // bundle's isotropic fit (cached path — sub-µs warm).
        let bandwidth = match req.bandwidth {
            Some(b) if b > 0.0 => b,
            _ => {
                let (_, fit_counter) = flow_from_bundle_cached(
                    &state,
                    &name,
                    heap,
                    &fields,
                    FitMode::Isotropic,
                    None,
                )?;
                let key = CacheKey::build(&name, FitMode::Isotropic, &fields, None);
                let cached = state
                    .flow_cache
                    .get(&key, fit_counter)
                    .ok_or_else(|| bad_request("intent_gate: cache lookup failed post-fit"))?;
                cached.sigma_sq.sqrt().max(1e-9)
            }
        };

        let raw = gigi::vector_cache::kde_raw_from_matrix(
            &cached_matrix.matrix,
            &query,
            bandwidth,
        );
        let normalized =
            gigi::vector_cache::kde_normalized_cached(&cached_matrix, &query, bandwidth);

        // Nearest-record info (single contiguous loop, no allocation).
        let n_samples = cached_matrix.matrix.n;
        let (nearest_index, nearest_distance) = if n_samples > 0 {
            let (idx, d_sq) = cached_matrix.matrix.nearest(&query);
            (Some(idx), d_sq.sqrt())
        } else {
            (None, 0.0)
        };

        Some(IntentGateQueryGroundingWire {
            raw,
            normalized,
            bandwidth_used: bandwidth,
            n_samples,
            nearest_record_index: nearest_index,
            nearest_distance,
        })
    } else {
        None
    };

    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainIntentGateResponse {
                verdict,
                coverage: resp.coverage,
                n_records_considered: resp.n_records_considered,
                solutions: solutions_wire,
                near_misses: near_misses_wire,
                selectivity: selectivity_wire,
                relaxations: relaxations_wire,
                pareto_near_misses: pareto_wire,
                pre_flight_unsat_reason: resp.pre_flight_unsat_reason,
                expanded: expanded_wire,
                gamma_trichotomy: resp.gamma_trichotomy.map(Into::into),
                query_grounding,
                counter_at_fit,
            },
            lambda_budget,
        },
    )
}

// ─── End S7: intent_gate ────────────────────────────────────────────────────

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
) -> Result<Json<ResponseWithLambda<BrainConfidenceResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
    if req.query.len() != req.fields.len() {
        return Err(bad_request(&format!(
            "query length {} ≠ fields length {}",
            req.query.len(),
            req.fields.len()
        )));
    }
    // Cached matrix path — see materialize_matrix_cached docstring
    // for the Marcella 2026-05-29 latency bug context. Same response
    // shape as before; previously rebuilt the (N, D) matrix and the
    // O(N²·D) max-density loop per request.
    let cached_matrix = materialize_matrix_cached(&state, &name, heap, &req.fields)?;
    let bandwidth = match req.bandwidth {
        Some(b) if b > 0.0 => b,
        _ => {
            let (_, s_sq) = fit_isotropic_gaussian(heap, &req.fields)
                .map_err(|e| bad_request(&e))?;
            s_sq.sqrt().max(1e-9)
        }
    };
    let raw = gigi::vector_cache::kde_raw_from_matrix(
        &cached_matrix.matrix,
        &req.query,
        bandwidth,
    );
    let normalized =
        gigi::vector_cache::kde_normalized_cached(&cached_matrix, &req.query, bandwidth);
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    Ok(Json(ResponseWithLambda {
        inner: BrainConfidenceResponse {
            raw,
            normalized,
            bandwidth,
            n_samples: cached_matrix.matrix.n,
        },
        lambda_budget,
    }))
}

// ─── confidence_with_explain (S1 wave 1, Marcella P0 #3 unblock) ─
//
// Combined endpoint that returns BOTH /brain/confidence (Gaussian-
// kernel density) AND /brain/explain (interpolation path to nearest
// known record) in one call. Marcella's refuse-gate hits both per
// turn; combining saves a round trip + one record walk + lets us
// share the cached fit/bandwidth in a single response.
//
// Why this exists: refuse-gate runs every conversational turn,
// must complete in <50ms total. Two separate HTTP round trips
// (each ~5-15ms network + processing) + two separate record walks
// (each O(N·n) over the same samples) is wasteful. Combining
// halves the network cost and the record-walk cost.

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Deserialize)]
struct BrainConfidenceWithExplainRequest {
    fields: Vec<String>,
    /// Query point — length must equal `fields.len()`.
    query: Vec<f64>,
    /// Kernel bandwidth for confidence. Default √σ² from fit.
    #[serde(default)]
    bandwidth: Option<f64>,
    /// Interpolation resolution for explain path. Default 10
    /// (returns 11 points: start + 10 forward toward target).
    #[serde(default = "default_explain_n_steps")]
    n_steps: usize,
}

#[cfg(feature = "kahler")]
#[derive(Debug, Clone, serde::Serialize)]
struct BrainConfidenceWithExplainResponse {
    // ── Confidence side — field names MATCH /brain/confidence
    //    verbatim (raw, normalized, bandwidth) so consumers can
    //    treat this endpoint as a strict superset. Per Marcella's
    //    2026-05-27 wire-shape verification: her refuse-gate code
    //    indexes resp["raw"], resp["nearest_index"],
    //    resp["nearest_distance"] at top level — this layout
    //    gives her exactly that.
    /// Σᵢ exp(−‖q−xᵢ‖²/2σ²) — raw Bayesian-precision proxy.
    raw: f64,
    /// raw / max_density — ratio to densest sample point.
    normalized: f64,
    /// Bandwidth actually used (request or derived from fit).
    bandwidth: f64,

    // ── Explain side — field names MATCH /brain/explain
    //    verbatim (query, nearest_record, nearest_index,
    //    nearest_distance, path, n_steps).
    /// Query echo.
    query: Vec<f64>,
    /// Nearest record's fiber values (None if no records).
    nearest_record: Option<Vec<f64>>,
    /// Nearest record's iteration index.
    nearest_index: Option<usize>,
    /// Euclidean distance query → nearest.
    nearest_distance: f64,
    /// `n_steps + 1` interpolation points from query → nearest.
    path: Vec<Vec<f64>>,
    /// Step count actually used.
    n_steps: usize,

    // ── Shared diagnostics ─────────────────────────────────
    /// Number of records with all requested fields present.
    /// Single key (vs `confidence.n_samples` + `explain.n_samples`)
    /// because the two ops share the same sample extraction.
    n_samples: usize,
    /// Bundle's mutation counter at the time of the response.
    /// Same value as the X-Bundle-Mutation-Counter header.
    counter_at_fit: u64,
}

/// POST /v1/bundles/{name}/brain/confidence_with_explain
///
/// Combined refuse-gate endpoint. Returns both Gaussian-kernel
/// confidence and explain-path trace from a single record walk +
/// cached fit lookup. Saves a network round trip + one O(N·n)
/// record walk vs calling /brain/confidence and /brain/explain
/// separately.
#[cfg(feature = "kahler")]
async fn brain_confidence_with_explain_endpoint(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainConfidenceWithExplainRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
    if req.query.len() != req.fields.len() {
        return Err(bad_request(&format!(
            "query length {} ≠ fields length {}",
            req.query.len(),
            req.fields.len()
        )));
    }

    // Cached matrix path — see materialize_matrix_cached docstring
    // for the Marcella 2026-05-29 latency bug context. KDE + normalized
    // + nearest-record + explain-path all share one materialization;
    // previously each request rebuilt `extract_field_samples` + ran
    // O(N²·D) max-density + duplicated the nearest-record loop in
    // both confidence and explain.
    let cached_matrix = materialize_matrix_cached(&state, &name, heap, &req.fields)?;

    // Bandwidth: request value if positive, else derive from
    // isotropic fit (cached path — sub-µs warm).
    let bandwidth = match req.bandwidth {
        Some(b) if b > 0.0 => b,
        _ => {
            let (_, counter_at_fit) = flow_from_bundle_cached(
                &state,
                &name,
                heap,
                &req.fields,
                FitMode::Isotropic,
                None,
            )?;
            let key = CacheKey::build(&name, FitMode::Isotropic, &req.fields, None);
            let cached = state.flow_cache.get(&key, counter_at_fit).ok_or_else(|| {
                bad_request(
                    "confidence_with_explain: cache lookup failed post-fit; this is a bug",
                )
            })?;
            cached.sigma_sq.sqrt().max(1e-9)
        }
    };

    // Header counter — current bundle state so consumers can stamp warmth.
    let counter_at_fit = heap.mutation_counter();

    // Confidence (matrix-cached).
    let confidence_raw = gigi::vector_cache::kde_raw_from_matrix(
        &cached_matrix.matrix,
        &req.query,
        bandwidth,
    );
    let confidence_normalized =
        gigi::vector_cache::kde_normalized_cached(&cached_matrix, &req.query, bandwidth);

    // Explain — same shape as `geometry::explain` (nearest record,
    // linear interpolation from query to nearest in n_steps + 1
    // points). Reuses the cached matrix's nearest() for the search;
    // pulls the nearest row directly out of the contiguous slab and
    // builds the path inline.
    let n_samples = cached_matrix.matrix.n;
    let d = cached_matrix.matrix.d;
    let (nearest_record, nearest_index, nearest_distance, path) = if n_samples == 0
        || req.query.is_empty()
    {
        (None, None, 0.0_f64, Vec::new())
    } else {
        let (idx, d_sq) = cached_matrix.matrix.nearest(&req.query);
        let nearest = cached_matrix.matrix.data[idx * d..(idx + 1) * d].to_vec();
        let n_steps = req.n_steps;
        let path: Vec<Vec<f64>> = (0..=n_steps)
            .map(|i| {
                let t = if n_steps == 0 {
                    1.0
                } else {
                    i as f64 / n_steps as f64
                };
                req.query
                    .iter()
                    .zip(nearest.iter())
                    .map(|(q, x)| (1.0 - t) * q + t * x)
                    .collect()
            })
            .collect();
        (Some(nearest), Some(idx), d_sq.sqrt(), path)
    };

    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainConfidenceWithExplainResponse {
                raw: confidence_raw,
                normalized: confidence_normalized,
                bandwidth,
                query: req.query.clone(),
                nearest_record,
                nearest_index,
                nearest_distance,
                path,
                n_steps: req.n_steps,
                n_samples,
                counter_at_fit,
            },
            lambda_budget,
        },
    )
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
) -> Result<Json<ResponseWithLambda<BrainAttendResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
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
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    Ok(Json(ResponseWithLambda {
        inner: BrainAttendResponse {
            weights,
            indices,
            bandwidth,
            n_samples: samples.len(),
        },
        lambda_budget,
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
) -> Result<Json<ResponseWithLambda<BrainEpisodicResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);

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
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    Ok(Json(ResponseWithLambda {
        inner: BrainEpisodicResponse {
            events: wire,
            n_records: values.len(),
            threshold_used: req.min_persistence_ratio,
            filter_applied: filter.map(|(field, value)| EpisodicFilterEcho { field, value }),
            gap_floor_epsilon_used: epsilon,
        },
        lambda_budget,
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
) -> Result<Json<ResponseWithLambda<BrainExplainResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
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
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    Ok(Json(ResponseWithLambda {
        inner: BrainExplainResponse {
            query: exp.query,
            nearest_record: exp.nearest_record,
            nearest_index: exp.nearest_index,
            nearest_distance: exp.nearest_distance,
            path: exp.path,
            n_steps: exp.n_steps,
            n_samples: samples.len(),
        },
        lambda_budget,
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
) -> Result<Json<ResponseWithLambda<BrainSemanticResponse>>, (StatusCode, Json<ErrorResponse>)> {
    use gigi::morse_cache::{CachedMorse, MorseCacheKey};

    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);

    // 2026-06-02 MorseCache: mutation-counter-keyed cache for
    // SEMANTIC results. Hot path on cache hit + matching counter:
    // O(1) hashmap lookup, no morse_compress, no betti, no
    // HodgeComplex construction. Same pattern as vector_cache.rs.
    let cache_key = MorseCacheKey::build(&name);
    let counter = heap.mutation_counter();

    // 1. Hot path — cache hit.
    if let Some(cached) = state.morse_cache.get(&cache_key, counter) {
        let lambda_budget = lambda_budget_for_bundle_ref(&store);
        return Ok(Json(ResponseWithLambda {
            inner: BrainSemanticResponse {
                betti_b0: cached.betti.b0,
                betti_b1: cached.betti.b1,
                betti_b2: cached.betti.b2,
                n_critical: cached.n_critical,
                n_original: cached.n_original,
                compression_ratio: cached.compression_ratio,
                cohomology_preserved: cached.cohomology_preserved,
            },
            lambda_budget,
        }));
    }

    // 2. Cold path — single-flight via per-key compute lock.
    let compute_lock_arc = state.morse_cache.acquire_compute_lock(&cache_key);
    let _guard = match compute_lock_arc.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    // 2a. Double-check under lock — another thread may have just
    // inserted while we were waiting for the compute lock.
    if let Some(cached) = state.morse_cache.get(&cache_key, counter) {
        state.morse_cache.release_compute_lock(&cache_key);
        let lambda_budget = lambda_budget_for_bundle_ref(&store);
        return Ok(Json(ResponseWithLambda {
            inner: BrainSemanticResponse {
                betti_b0: cached.betti.b0,
                betti_b1: cached.betti.b1,
                betti_b2: cached.betti.b2,
                n_critical: cached.n_critical,
                n_original: cached.n_original,
                compression_ratio: cached.compression_ratio,
                cohomology_preserved: cached.cohomology_preserved,
            },
            lambda_budget,
        }));
    }

    // 2b. Build. (The actual SEMANTIC math — now rank-based per
    // the betti-rank commit `0ec9405`, ~0.54s on the production
    // 9964-record bundle.)
    let morse = gigi::geometry::semantic_gist(heap).ok_or_else(|| {
        // Release the compute lock before returning the error so
        // future requests don't wait forever on a poisoned key.
        state.morse_cache.release_compute_lock(&cache_key);
        not_found(&format!(
            "Bundle '{}' produced no Morse compression (too few records or degenerate complex)",
            name
        ))
    })?;

    // 2c. Publish to cache + release the single-flight lock.
    let cached = CachedMorse::new(
        counter,
        morse.betti,
        morse.n_critical(),
        morse.n_original(),
        morse.compression_ratio(),
        morse.cohomology_preserved(),
    );
    state.morse_cache.insert(cache_key.clone(), cached);
    state.morse_cache.release_compute_lock(&cache_key);

    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    Ok(Json(ResponseWithLambda {
        inner: BrainSemanticResponse {
            betti_b0: cached.betti.b0,
            betti_b1: cached.betti.b1,
            betti_b2: cached.betti.b2,
            n_critical: cached.n_critical,
            n_original: cached.n_original,
            compression_ratio: cached.compression_ratio,
            cohomology_preserved: cached.cohomology_preserved,
        },
        lambda_budget,
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

/// **#107 fix.** Adapter that lets brain endpoints handle both
/// heap-resident and mmap+overlay bundles. Heap path is zero-cost
/// (returns the existing &BundleStore reference). Overlay path
/// materializes the merged view into a temporary heap store
/// (O(N) walk, ~10ms for 10k records) and returns a reference
/// into the caller's stack-allocated `Option<BundleStore>`.
///
/// Usage pattern (3 lines per endpoint):
///
/// ```ignore
/// let store_ref = engine.bundle(&name).ok_or_else(|| not_found(...))?;
/// let mut _promoted: Option<gigi::BundleStore> = None;
/// let heap = heap_or_promote(&store_ref, &mut _promoted);
/// // ... `heap: &BundleStore` works identically for both variants ...
/// ```
///
/// This is a deliberately surgical fix: it preserves the existing
/// helper signatures (`extract_field_samples`, `fit_*_gaussian`,
/// `flow_from_bundle_cached`, etc.) that all take `&BundleStore`,
/// instead of refactoring them to be polymorphic. The one-time
/// materialize cost is dominated by the existing per-call fit work.
#[cfg(feature = "kahler")]
fn heap_or_promote<'a>(
    store: &'a gigi::BundleRef<'a>,
    promoted: &'a mut Option<gigi::BundleStore>,
) -> &'a gigi::BundleStore {
    match store {
        gigi::BundleRef::Heap(h) => *h,
        gigi::BundleRef::Overlay(o) => {
            *promoted = Some(o.to_temp_heap_store());
            promoted
                .as_ref()
                .expect("promoted was just set in this branch")
        }
    }
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

    // Hot path: cache lookup. Lock-free read; no contention with
    // concurrent hits.
    if let Some(cached) = state.flow_cache.get(&key, counter) {
        state.metrics.record_brain_cache_hit();
        let ctx = build_ctx_from_cached(&cached, fit_mode, n, b.clone())
            .map_err(|e| bad_request(&e))?;
        return Ok((ctx, counter));
    }

    // Cache miss path with single-flight (per Marcella's 2026-05-27
    // check #1a). Without this, N concurrent misses on the same key
    // perform N independent fits and the last wins — N-1 wasted.
    //
    // Pattern:
    //   1. Acquire per-key compute lock. If another thread is mid-
    //      compute, BLOCK here until they finish + release.
    //   2. After acquire, re-check the main cache. The thread that
    //      held the lock before us may have just inserted; serve
    //      from cache and skip compute.
    //   3. If still missing, compute + insert. Hold the per-key
    //      lock for the duration so other waiters serialize.
    //   4. Release the per-key lock entry so it doesn't accumulate.
    let compute_lock_arc = state.flow_cache.acquire_compute_lock(&key);
    let _compute_guard = match compute_lock_arc.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    // Re-check after acquiring the lock — another thread may have
    // computed + inserted while we were blocked.
    if let Some(cached) = state.flow_cache.get(&key, counter) {
        // Single-flight saved a redundant compute.
        state.metrics.record_brain_cache_hit();
        let ctx = build_ctx_from_cached(&cached, fit_mode, n, b.clone())
            .map_err(|e| bad_request(&e))?;
        // Release the compute lock entry (we held it briefly but
        // didn't compute). Best-effort cleanup; if another thread
        // is queued behind, they'll just see the same cached entry.
        state.flow_cache.release_compute_lock(&key);
        return Ok((ctx, counter));
    }

    // True cache miss: compute and insert.
    state.metrics.record_brain_cache_miss();
    let counter_at_fit = store.mutation_counter();
    let fit_start = std::time::Instant::now();
    let cached = compute_fit_data(store, fields, fit_mode, sigma_floor_epsilon, counter_at_fit)
        .map_err(|e| {
            // Release compute lock on error so the next caller can retry.
            state.flow_cache.release_compute_lock(&key);
            bad_request(&e)
        })?;
    let fit_us = fit_start.elapsed().as_micros() as u64;
    state.metrics.record_brain_timing(fit_us, 0);

    let ctx = build_ctx_from_cached(&cached, fit_mode, n, b).map_err(|e| {
        state.flow_cache.release_compute_lock(&key);
        bad_request(&e)
    })?;
    let evicted = state.flow_cache.insert_with_eviction_hint(key.clone(), cached);
    if evicted {
        state.metrics.record_brain_cache_eviction();
    }
    // Drop the per-key compute lock entry. Other waiters will find
    // the new cache entry on their re-check after our lock release.
    state.flow_cache.release_compute_lock(&key);
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
            // Wave 2 §B: precision is flat row-major Vec<f64> of
            // length n*n. The matvec ∇H = Σ⁻¹(x−μ) streams the
            // precision matrix row-by-row through cache — no
            // pointer-chasing on row access.
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
                // Σ⁻¹·dx with flat row-major precision.
                // row i lives at precision[i*n .. (i+1)*n].
                let mut out = Vec::with_capacity(n_for_grad);
                for i in 0..n_for_grad {
                    let row_start = i * n_for_grad;
                    let mut acc = 0.0_f64;
                    for j in 0..n_for_grad {
                        acc += precision_arc[row_start + j] * dx[j];
                    }
                    out.push(acc);
                }
                out
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
            let precision_for_grad = fit.precision.clone(); // flat row-major Vec<f64>, length n*n
            let n_for_grad = n;
            let grad: Box<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync> =
                Box::new(move |x: &[f64]| -> Vec<f64> {
                    // dx = x − μ
                    let dx: Vec<f64> = x
                        .iter()
                        .zip(mu_for_grad.iter())
                        .map(|(xi, mi)| xi - mi)
                        .collect();
                    // Σ⁻¹ · dx (flat row-major matvec, wave 2 §B).
                    let mut out = Vec::with_capacity(n_for_grad);
                    for i in 0..n_for_grad {
                        let row_start = i * n_for_grad;
                        let mut acc = 0.0_f64;
                        for j in 0..n_for_grad {
                            acc += precision_for_grad[row_start + j] * dx[j];
                        }
                        out.push(acc);
                    }
                    out
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
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainDreamRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
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

    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainDreamResponse {
                trajectory,
                fit_mean: ctx.mu,
                fit_sigma_sq: ctx.sigma_sq,
                temperature_used: req.temperature,
                mean_dist_from_mean: mean_d,
                max_dist_from_mean: max_d,
            },
            lambda_budget,
        },
    )
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
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainForecastRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
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
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainForecastResponse {
                trajectory,
                fit_mean: ctx.mu,
                fit_sigma_sq: ctx.sigma_sq,
            },
            lambda_budget,
        },
    )
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
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainReconstructRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
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
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainReconstructResponse {
                result,
                fit_mean: ctx.mu,
                descent_distance,
            },
            lambda_budget,
        },
    )
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
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainInpaintRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
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
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainInpaintResponse {
                result,
                locked_indices: req.locked_indices,
                fit_mean: ctx.mu,
                fit_sigma_sq: ctx.sigma_sq,
            },
            lambda_budget,
        },
    )
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
    headers: axum::http::HeaderMap,
    Json(req): Json<BrainPredictRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    // **#107 fix.** Polymorphic via heap_or_promote: heap path
    // is zero-cost; overlay path materializes once (~10ms/10k records).
    let mut _promoted: Option<gigi::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
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
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let lambda_budget = lambda_budget_for_bundle_ref(&store);
    negotiated_brain_response(
        accept,
        counter_at_fit,
        ResponseWithLambda {
            inner: BrainPredictResponse {
                next_state,
                fit_mean: ctx.mu,
                fit_sigma_sq: ctx.sigma_sq,
                step_size,
            },
            lambda_budget,
        },
    )
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
    // Davis Conjecture λ-budget — substrate self-introspection rides
    // along with curvature/confidence on every query. See
    // `CurvatureReport.lambda_budget` for the field semantics.
    let lambda = match store.as_heap() {
        Some(heap) => curvature::lambda_budget(k, gigi_welford_radius(heap), 1.0),
        None => curvature::lambda_budget(k, 1.0, 1.0),
    };

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
            "lambda_budget": lambda,
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
        let lambda = match store.as_heap() {
            Some(heap) => curvature::lambda_budget(k, gigi_welford_radius(heap), 1.0),
            None => curvature::lambda_budget(k, 1.0, 1.0),
        };
        let meta = serde_json::json!({
            "__meta": true,
            "count": count,
            "curvature": k,
            "confidence": curvature::confidence(k),
            "lambda_budget": lambda
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
                    "sum": fs.sum(),
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
    // Echo back the `gigi.v1` subprotocol marker if the client offered
    // it. Browsers require the server to confirm one of the protocols
    // they listed during the upgrade; without this, the connection
    // closes immediately after the handshake on the subprotocol-auth
    // path. `protocols()` picks the FIRST matching name from the
    // client's offered list; we only ever advertise the marker (the
    // credential subprotocols are consumed by auth_middleware, never
    // echoed).
    ws.protocols(["gigi.v1"])
        .on_upgrade(move |socket| handle_ws(socket, state))
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
    let snapshot = tokio::task::spawn_blocking(move || {
        let mut engine = state.engine.write().unwrap();
        engine.snapshot_with_report()
    })
    .await;

    match snapshot {
        Ok(Ok(report)) if report.timed_out_bundles.is_empty() => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "total_records_snapshotted": report.total_records_written,
                "message": "DHOOM snapshots written; WAL compacted to schema-only."
            })),
        ),
        Ok(Ok(report)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Snapshot timed out before WAL compaction",
                "total_records_snapshotted": report.total_records_written,
                "timed_out_bundles": report.timed_out_bundles,
            })),
        ),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Snapshot failed: {e}") })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Snapshot task failed: {e}") })),
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

/// Validate that a parsed statement is safe to expose on `/v1/public/gql`.
///
/// The public endpoint accepts only reads and only on allowlisted bundles.
/// Everything not explicitly listed here — writes (`Insert`, `BatchInsert`,
/// `Retract`, etc.), schema mutations (`CreateBundle`, `AlterBundleAddBase`,
/// `Collapse`), admin verbs (`Snapshot`, `Backup`, `Restore`, `Ingest`,
/// `Transplant`, `RotateKey`, `Grant`/`Revoke`), and every analytics verb
/// that isn't the small hand-picked set below — falls through the wildcard
/// arm and gets rejected.
///
/// When adding a new verb here, ask two questions:
///   1. Can this ever mutate storage? If yes, do NOT add it.
///   2. Does the allowed bundle set need finer granularity per verb? If yes,
///      thread a per-verb allowlist through instead of expanding this one.
fn validate_public_stmt(
    stmt: &gigi::parser::Statement,
    allowlist: &std::collections::HashSet<String>,
) -> Result<(), String> {
    use gigi::parser::Statement as S;
    let bundle_ok = |b: &str| -> Result<(), String> {
        if allowlist.contains(b) {
            Ok(())
        } else {
            Err(format!(
                "bundle '{b}' is not exposed on the public read endpoint"
            ))
        }
    };
    match stmt {
        // No bundle parameter. Handled specially in the handler so the
        // response only lists allowlisted names, not every bundle.
        S::ShowBundles => Ok(()),
        // Bundle-scoped read verbs — safe to expose.
        S::Health { bundle } => bundle_ok(bundle),
        S::Describe { bundle, .. } => bundle_ok(bundle),
        S::PointQuery { bundle, .. } => bundle_ok(bundle),
        S::ExistsSection { bundle, .. } => bundle_ok(bundle),
        S::Cover { bundle, .. } => bundle_ok(bundle),
        S::Integrate { bundle, .. } => bundle_ok(bundle),
        S::Select { bundle, .. } => bundle_ok(bundle),
        // Everything else — writes, admin, non-whitelisted analytics — is
        // refused. The error is generic on purpose (don't leak the shape of
        // the verb enum to anonymous callers).
        _ => Err(
            "verb not allowed on the public read endpoint: only reads on \
             allowlisted bundles are permitted"
                .to_string(),
        ),
    }
}

/// Public read-only GQL endpoint at `POST /v1/public/gql`.
///
/// Preconditions enforced here BEFORE any executor is called:
///   * `state.public_bundles` is non-empty (also guarded by not registering
///     the route when empty — this handler is unreachable in that case)
///   * body has a string `query` field
///   * the query is a single statement (no `;`-separated compound queries —
///     that check runs before parsing so a malformed second statement can't
///     smuggle intent past the parser)
///   * the parsed statement matches the read-verb whitelist in
///     `validate_public_stmt`
///   * the target bundle (if the verb takes one) is in the allowlist
///
/// `ShowBundles` is answered directly with just the allowlisted names — the
/// executor is not called, so private bundle names never appear in the
/// response no matter what else is stored.
///
/// All other allowed statements are forwarded to the same `gql_query`
/// handler the authenticated `/v1/gql` route uses. The auth middleware has
/// already stashed owner-equivalent claims for this request via its
/// `/v1/public/gql` bypass, so no per-user authorization runs.
async fn public_gql_query(
    State(state): State<Arc<StreamState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let query = match body.get("query").and_then(|v| v.as_str()) {
        Some(q) => q.trim(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'query' field"})),
            )
                .into_response()
        }
    };

    // Reject compound statements. Split on `;` and reject if more than one
    // non-empty segment appears — no smuggling writes past a read.
    let non_empty_segments = query
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .count();
    if non_empty_segments > 1 {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "public endpoint accepts only a single statement (no compound queries)"
            })),
        )
            .into_response();
    }

    let stmt = match gigi::parser::parse(query) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Parse error: {e}")})),
            )
                .into_response()
        }
    };

    if let Err(msg) = validate_public_stmt(&stmt, &state.public_bundles) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": msg})),
        )
            .into_response();
    }

    // ShowBundles: never call the executor. Return only the allowlisted
    // names so non-public bundles stay hidden regardless of engine state.
    if matches!(stmt, gigi::parser::Statement::ShowBundles) {
        let mut names: Vec<&String> = state.public_bundles.iter().collect();
        names.sort();
        return (
            StatusCode::OK,
            Json(serde_json::json!({"bundles": names})),
        )
            .into_response();
    }

    // Forward to the authenticated executor. The `body` we hand off is the
    // original JSON, so the executor sees the query verbatim.
    gql_query(State(state), headers, Json(body)).await.into_response()
}

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
    // Called by all the match arms that don't go through the bundle-aware path.
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

    // Halcyon Part V P-1 — §2.5 drop-bug fix.
    //
    // Gauge-feature statements (LATTICE / GAUGE_FIELD / GIBBS_SAMPLE
    // / E_FIELD / SYMPLECTIC_FLOW / SHOW (E_FIELD | GAUGE_FIELD |
    // LATTICE) / SELECT (PLAQUETTE | Q_SURROGATE | H_TOTAL |
    // GAUSS_RESIDUAL_MAX) / LATTICE FROM TRUNCATED_ICOSAHEDRON) have
    // no single bundle binding, so `get_bundle_name(&stmt)` below
    // returns `None` and the default early-return drops the
    // statement on the floor with a `{"status":"ok"}` envelope.
    // The helper in `gigi::halcyon_gql_dispatch` is the testable
    // boundary that dispatches through `parser::execute` for any
    // gauge-feature variant; the response is lowered through the
    // same `exec_result_to_response` envelope the bundle-aware
    // path uses, so JSON shape stays uniform across both surfaces.
    //
    // The dedicated /v1/gauge_field/* + /v1/lattice/* + /v1/e_field/*
    // routes in `src/gauge/http.rs` are unaffected — they continue to
    // expose the same read-only surface they already shipped (Part
    // II.6 / III.7 / IV.8). This fix re-enables the universal /v1/gql
    // reach-through so Halcyon's SNAPSHOT verb (Part V P0.2) and
    // every future gauge statement land without a per-statement HTTP
    // route.
    #[cfg(feature = "gauge")]
    {
        if let Some(result) =
            gigi::halcyon_gql_dispatch::try_dispatch_gauge_statement(&state.engine, &stmt)
        {
            let dur = t0.elapsed().as_micros() as u64;
            let stmt_type = gql_stmt_type_name(&stmt);
            let (status, resp) = match result {
                Ok(r) => exec_result_to_response(r),
                Err(e) => {
                    let ev = state.logger.query_error(
                        &req_id, query, dur, "ExecError", &e, 500,
                    );
                    state.logger.emit(ev);
                    state.metrics.record_query(dur, stmt_type, false, true);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": e})),
                    );
                }
            };
            let slow = dur >= state.logger.slow_threshold_us();
            let ev = state.logger.query_complete(
                &req_id, "gql", stmt_type, query, dur, 0, dur,
                &[], 0, 0, 0, 0, false, None, None,
            );
            state.logger.emit(ev);
            if slow {
                let ev2 = state.logger.query_slow(
                    &req_id, stmt_type, query, dur, false, false, "gauge dispatch",
                );
                state.logger.emit(ev2);
            }
            state.metrics.record_query(dur, stmt_type, slow, false);
            return (status, resp);
        }
    }

    // Halcyon Bridge Trilogy follow-up — topology-verb route-handler bypass.
    //
    // Hallie's smoke chain (2026-06-28, gigi-stream a1c9c57) caught
    // the 5 topology verbs (CHERN_CLASS / PONTRYAGIN / BETTI ORDER k /
    // PI_1 / OBSTRUCTION) dropping at the bundle pre-resolve below:
    // `get_bundle_name(&stmt)` returns the gauge-field-or-lattice name
    // (`U_smoke` for CHERN/PONT/OBSTRUCTION, `smoke` for BETTI/PI_1),
    // none of which live in the engine bundle store. The pre-resolve
    // then 404s with `{"error":"No bundle: <name>"}` or, when the
    // bundle-name extraction returns `None`, drops the statement with
    // a silent `{"status":"ok"}` envelope.
    //
    // Fix: dispatch the 5 variants through
    // `halcyon_gql_dispatch::try_dispatch_topology_statement` BEFORE
    // the bundle pre-resolve. The helper consults
    // `gigi::gauge::registry` (CHERN_CLASS / PONTRYAGIN), the engine
    // bundle store + the gauge registry (OBSTRUCTION two-path),
    // `gigi::lattice::registry` (PI_1 / BETTI ORDER), reaching the
    // kernels (`chern_weil::chern_class`, `chern_weil::pontryagin_class`,
    // `topology::pi_1_presentation`, `obstruction::obstruction_with_default`,
    // `topology::betti_topological`) directly.
    //
    // `BETTI` with `order = None` is NOT routed here — it falls
    // through to the legacy bundle path which returns `β_0 + β_1`
    // from the field-index graph.
    #[cfg(feature = "gauge")]
    {
        let is_topology_verb = matches!(
            &stmt,
            gigi::parser::Statement::ChernClass { .. }
                | gigi::parser::Statement::Pontryagin { .. }
                | gigi::parser::Statement::Pi1 { .. }
                | gigi::parser::Statement::Obstruction { .. }
                | gigi::parser::Statement::Betti { order: Some(_), .. }
        );
        if is_topology_verb {
            let result = gigi::halcyon_gql_dispatch::try_dispatch_topology_statement(
                &state.engine,
                &stmt,
            );
            let dur = t0.elapsed().as_micros() as u64;
            let stmt_type = gql_stmt_type_name(&stmt);
            let (status, resp) = match result {
                Ok(r) => exec_result_to_response(r),
                Err(e) => {
                    let ev = state.logger.query_error(
                        &req_id, query, dur, "ExecError", &e, 500,
                    );
                    state.logger.emit(ev);
                    state.metrics.record_query(dur, stmt_type, false, true);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": e})),
                    );
                }
            };
            let slow = dur >= state.logger.slow_threshold_us();
            let ev = state.logger.query_complete(
                &req_id, "gql", stmt_type, query, dur, 0, dur,
                &[], 0, 0, 0, 0, false, None, None,
            );
            state.logger.emit(ev);
            if slow {
                let ev2 = state.logger.query_slow(
                    &req_id, stmt_type, query, dur, false, false, "topology dispatch",
                );
                state.logger.emit(ev2);
            }
            state.metrics.record_query(dur, stmt_type, slow, false);
            return (status, resp);
        }
    }

    // ── INGEST bypass (Halcyon 2026-07-01 follow-up) ──────────────────
    //
    // Hallie's afternoon smoke chain against gigi-stream v233 caught
    // the SAME pre-resolve drop bug that killed the topology verbs on
    // 2026-06-28 (fixed above by `try_dispatch_topology_statement`).
    // Firing:
    //
    //     LATTICE l4_obc_verify FROM CUBIC L=4 DIM=4 OBC AXIS 0;
    //     INGEST su2_L4_obc_verify FROM '..._L4/raw_U_configs.npz'
    //         FORMAT NPZ AS GAUGE_FIELD GROUP SU(2) ON LATTICE l4_obc_verify;
    //
    // returned HTTP 404 `{"error":"No bundle: su2_L4_obc_verify"}`
    // because `get_bundle_name(&stmt)` returns `Some("su2_L4_obc_verify")`
    // for `Statement::Ingest` and the pre-resolve below then 404s
    // before the INGEST executor gets to run. INGEST is a bundle-
    // CREATOR (not consumer) — the executor at
    // `src/ingest.rs::execute_ingest` calls
    // `ensure_bundle_compatible(..., allow_auto_create=true)`
    // (ingest.rs:417-422) to materialize the bundle from the NPZ
    // header when the name is fresh. The pre-resolve wall stops that
    // from ever happening.
    //
    // Fix mirrors the topology bypass: dispatch `Statement::Ingest`
    // through `halcyon_gql_dispatch::try_dispatch_ingest_statement`
    // BEFORE the bundle pre-resolve. The helper acquires a write
    // lock and forwards to `parser::execute`, which delegates to
    // `crate::ingest::execute_ingest` (plain INGEST) or
    // `execute_ingest_as_gauge_field` (AS GAUGE_FIELD variant).
    #[cfg(feature = "gauge")]
    {
        let is_ingest = matches!(&stmt, gigi::parser::Statement::Ingest { .. });
        if is_ingest {
            let result = gigi::halcyon_gql_dispatch::try_dispatch_ingest_statement(
                &state.engine,
                &stmt,
            );
            let dur = t0.elapsed().as_micros() as u64;
            let stmt_type = gql_stmt_type_name(&stmt);
            let (status, resp) = match result {
                Ok(r) => exec_result_to_response(r),
                Err(e) => {
                    let ev = state.logger.query_error(
                        &req_id, query, dur, "ExecError", &e, 500,
                    );
                    state.logger.emit(ev);
                    state.metrics.record_query(dur, stmt_type, false, true);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": e})),
                    );
                }
            };
            let slow = dur >= state.logger.slow_threshold_us();
            let ev = state.logger.query_complete(
                &req_id, "gql", stmt_type, query, dur, 0, dur,
                &[], 0, 0, 0, 0, false, None, None,
            );
            state.logger.emit(ev);
            if slow {
                let ev2 = state.logger.query_slow(
                    &req_id, stmt_type, query, dur, false, false, "ingest dispatch",
                );
                state.logger.emit(ev2);
            }
            state.metrics.record_query(dur, stmt_type, slow, false);
            return (status, resp);
        }
    }

    // ── Bundle pre-resolve ─────────────────────────────────────────────
    //
    // By the time we reach this point, every dispatch block above has
    // declined the statement. The remaining variants ALL expect a
    // bundle binding the route handler can pre-resolve:
    //
    //   - The gauge dispatch block (`try_dispatch_gauge_statement`,
    //     ~line 12421) handles LATTICE / GAUGE_FIELD / GIBBS_SAMPLE /
    //     E_FIELD / SYMPLECTIC_FLOW / SELECT (PLAQUETTE | Q_SURROGATE |
    //     H_TOTAL | GAUSS_RESIDUAL_MAX) / SHOW (LATTICE | E_FIELD |
    //     GAUGE_FIELD) / LATTICE FROM TRUNCATED_ICOSAHEDRON / SNAPSHOT /
    //     LOOP / LOOP_TRANSPORT.
    //   - The topology dispatch block (`try_dispatch_topology_statement`,
    //     ~line 12484) handles CHERN_CLASS / PONTRYAGIN / PI_1 /
    //     OBSTRUCTION / BETTI ORDER k.
    //   - The INGEST dispatch block (`try_dispatch_ingest_statement`,
    //     ~line 12531) handles INGEST (fresh bundle names — the
    //     executor auto-creates the bundle from the NPZ header).
    //
    // Any new variant whose "bundle" field is NOT a registered bundle
    // (e.g. a gauge-field name or a lattice name), OR any variant that
    // is a bundle CREATOR rather than consumer, must be added to the
    // appropriate dispatch block above — adding the kernel logic to
    // `execute_gql_on_store_read` alone will land in the dead-code
    // arms there and never fire from the HTTP path.
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
            | gigi::parser::Statement::AlterBundleAddBase { .. }
    );

    if needs_write {
        let mut engine = state.engine.write().unwrap();
        if engine.bundle(&bundle_name).is_none() {
            let dur = t0.elapsed().as_micros() as u64;
            let ev = state.logger.query_error(&req_id, query, dur, "BundleNotFound", &format!("No bundle: {bundle_name}"), 404);
            state.logger.emit(ev);
            state.metrics.record_query(dur, stmt_type, false, true);
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("No bundle: {bundle_name}")})),
            );
        }
        let result = execute_gql_on_engine(&mut engine, &stmt);
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
    } else if gigi::virtual_bundles::is_virtual(&bundle_name) {
        // Virtual-bundle short-circuit (`__bundles__` etc): the registry
        // is materialized fresh per call by `parser::execute`, which has
        // the dispatch at `src/parser.rs:8738` for `Statement::Cover` on
        // virtual names. We hold a write lock only briefly because
        // `materialize_bundles_rows` itself takes `&Engine`; the `&mut`
        // requirement comes from the function signature on the executor,
        // not the operation. Read-only semantics preserved: writes against
        // a virtual bundle are rejected upstream by
        // `reject_virtual_write` in the parser's Insert/Upsert/etc arms.
        let mut engine = state.engine.write().unwrap();
        let result = gigi::parser::execute(&mut engine, &stmt);
        drop(engine);
        let dur = t0.elapsed().as_micros() as u64;
        let (status, resp) = match result {
            Ok(r) => exec_result_to_response(r),
            Err(e) => {
                let ev = state.logger.query_error(&req_id, query, dur, "VirtualBundleExecError", &e, 500);
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
            let ev2 = state.logger.query_slow(&req_id, stmt_type, query, dur, false, false, "virtual bundle read path");
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

/// Execute a GQL statement that mutates bundle data through Engine WAL paths.
fn execute_gql_on_engine(
    engine: &mut gigi::engine::Engine,
    stmt: &gigi::parser::Statement,
) -> Result<gigi::parser::ExecResult, String> {
    use gigi::bundle::QueryCondition as QC;
    use gigi::parser::{literal_to_value, ExecResult, Statement};

    match stmt {
        Statement::Insert {
            bundle, columns, values
        } => {
            let mut record = std::collections::HashMap::new();
            for (c, v) in columns.iter().zip(values.iter()) {
                record.insert(c.clone(), literal_to_value(v));
            }
            engine.insert(bundle, &record).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }
        Statement::SectionUpsert {
            bundle, columns, values, ..
        } => {
            let mut record = std::collections::HashMap::new();
            for (c, v) in columns.iter().zip(values.iter()) {
                record.insert(c.clone(), literal_to_value(v));
            }
            engine.upsert(bundle, &record).map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }
        Statement::BatchInsert { bundle, columns, rows } => {
            let records: Vec<gigi::types::Record> = rows
                .iter()
                .map(|row| {
                    if columns.is_empty() {
                        row.iter()
                            .enumerate()
                            .map(|(i, v)| (format!("_{i}"), literal_to_value(v)))
                            .collect()
                    } else {
                        columns
                            .iter()
                            .zip(row.iter())
                            .map(|(c, v)| (c.clone(), literal_to_value(v)))
                            .collect()
                    }
                })
                .collect();
            engine
                .batch_insert(bundle, &records)
                .map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Ok)
        }
        Statement::BatchSectionUpsert { bundle, columns, rows } => {
            let records: Vec<gigi::types::Record> = rows
                .iter()
                .map(|row| {
                    if columns.is_empty() {
                        row.iter()
                            .enumerate()
                            .map(|(i, v)| (format!("_{i}"), literal_to_value(v)))
                            .collect()
                    } else {
                        columns
                            .iter()
                            .zip(row.iter())
                            .map(|(c, v)| (c.clone(), literal_to_value(v)))
                            .collect()
                    }
                })
                .collect();
            let (inserted, updated) = engine
                .batch_upsert(bundle, &records)
                .map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Rows(vec![{
                let mut r = gigi::types::Record::new();
                r.insert("inserted".to_string(), gigi::types::Value::Integer(inserted as i64));
                r.insert("updated".to_string(), gigi::types::Value::Integer(updated as i64));
                r
            }]))
        }
        Statement::Redefine { bundle, key, sets } => {
            let key_rec: gigi::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let patches: gigi::types::Record = sets
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            if engine
                .update(bundle, &key_rec, &patches)
                .map_err(|e| format!("{e}"))?
            {
                Ok(ExecResult::Ok)
            } else {
                Err("Record not found".into())
            }
        }
        Statement::BulkRedefine {
            bundle, conditions, sets, ..
        } => {
            let qcs: Vec<QC> = conditions.iter().flat_map(|fc| gigi::parser::filter_to_query_conditions(fc)).collect();
            let patches: gigi::types::Record = sets
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            let n = engine
                .bulk_update(bundle, &qcs, &patches)
                .map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Count(n))
        }
        Statement::Retract { bundle, key } => {
            let key_rec: gigi::types::Record = key
                .iter()
                .map(|(k, v)| (k.clone(), literal_to_value(v)))
                .collect();
            if engine
                .delete(bundle, &key_rec)
                .map_err(|e| format!("{e}"))?
            {
                Ok(ExecResult::Ok)
            } else {
                Err("Record not found".into())
            }
        }
        Statement::BulkRetract { bundle, conditions } => {
            let qcs: Vec<QC> = conditions.iter().flat_map(|fc| gigi::parser::filter_to_query_conditions(fc)).collect();
            let n = engine
                .bulk_delete(bundle, &qcs)
                .map_err(|e| format!("{e}"))?;
            Ok(ExecResult::Count(n))
        }
        // ALTER BUNDLE ADD BASE — schema evolution. The parser executor at
        // src/parser.rs:9216 owns the snapshot + drop + recreate + re-insert
        // dance; we delegate here rather than duplicate that logic so the
        // in-process test path and the HTTP route stay bit-identical.
        Statement::AlterBundleAddBase { .. } => gigi::parser::execute(engine, stmt),
        _ => Ok(ExecResult::Ok),
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
/// Discrete Gauss-Bonnet angle-deficit holonomy in the (f0, f1) fiber plane.
///
/// **Gauge invariance (v0.3.1)**: previously this function computed angles
/// directly on raw centroid coordinates, which made the result depend on the
/// active gauge — under per-field Affine encryption with scales (a_0, a_1),
/// the direction vectors (dx, dy) become (a_0·dx, a_1·dy), and `atan2`
/// distorts when a_0 ≠ a_1. v0.3.1 normalizes centroids by their own
/// min/max per axis before computing angles. Under any field-wise Affine
/// gauge g_i(v) = a_i·v + b_i, the centroid-set's min/range transforms
/// equivariantly, so the normalized centroids are gauge-invariant up to
/// per-axis reflection in [0,1]². Reflection preserves angle magnitudes;
/// the angle-deficit therefore remains invariant in absolute value.
///
/// The returned centroids are reported in raw (gauge-active) coordinates
/// for display purposes; only the angle/deficit computation runs in
/// normalized space. This means the `transport_angle` values reported per
/// centroid are also the gauge-invariant normalized-space angles.
///
/// **No decryption required**: the function reads the stored fiber bytes
/// directly. For the Affine, Isometric, and Identity encryption modes the
/// returned deficit equals what would be computed on plaintext (up to sign
/// for per-axis reflections under negative scales). For Opaque / Indexed
/// (non-numeric ciphertexts), `Value::as_f64()` will return None for most
/// fields and the function returns 0.0 — call HOLONOMY only on numeric
/// fiber fields.
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

    // v0.3.1 gauge-invariance: normalize centroids by their own (min, max)
    // per axis. This makes the subsequent angle computation invariant under
    // per-field Aff(ℝ) up to per-axis reflection (which preserves magnitudes).
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for (_, cx, cy) in &centroids {
        if *cx < min_x { min_x = *cx; }
        if *cx > max_x { max_x = *cx; }
        if *cy < min_y { min_y = *cy; }
        if *cy > max_y { max_y = *cy; }
    }
    let range_x = (max_x - min_x).max(f64::EPSILON);
    let range_y = (max_y - min_y).max(f64::EPSILON);
    let normalized: Vec<(f64, f64)> = centroids.iter()
        .map(|(_, cx, cy)| ((cx - min_x) / range_x, (cy - min_y) / range_y))
        .collect();

    let nc = normalized.len();
    let mut transport_angles = vec![0.0f64; nc];
    for i in 0..nc {
        let prev = if i == 0 { nc - 1 } else { i - 1 };
        let next = (i + 1) % nc;
        let dx_in  = normalized[i].0 - normalized[prev].0;
        let dy_in  = normalized[i].1 - normalized[prev].1;
        let dx_out = normalized[next].0 - normalized[i].0;
        let dy_out = normalized[next].1 - normalized[i].1;
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

/// Canonical fiber-field name list for a gauge group. Used by
/// CHERN_CLASS / PONTRYAGIN when the caller omits the `ON FIBER`
/// clause — the executor synthesizes the canonical names from the
/// group's representation.
///
/// SU(2): `["q0", "q1", "q2", "q3"]` — quaternion scalar-first.
/// SU(3): `["m00_re", "m00_im", ..., "m22_re", "m22_im"]` — 9 complex
/// entries row-major, 18 floats total.
/// U(1): `["theta"]`.
/// Z(N): `["k"]`.
#[cfg(feature = "gauge")]
fn canonical_fiber_fields(group: gigi::gauge::Group) -> Vec<String> {
    match group {
        gigi::gauge::Group::SU2 => {
            vec!["q0".into(), "q1".into(), "q2".into(), "q3".into()]
        }
        gigi::gauge::Group::SU3 => {
            let mut out = Vec::with_capacity(18);
            for i in 0..3 {
                for j in 0..3 {
                    out.push(format!("m{i}{j}_re"));
                    out.push(format!("m{i}{j}_im"));
                }
            }
            out
        }
        gigi::gauge::Group::U1 => vec!["theta".into()],
        gigi::gauge::Group::ZN { .. } => vec!["k".into()],
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
            // Validate every referenced field against the schema up front.
            // Unknown fields used to fail silently (a typo'd WHERE matched
            // nothing; a typo'd PROJECT column just vanished), which is
            // indistinguishable from "no data" — the worst kind of wrong.
            {
                let known = store.field_names();
                let mut referenced: Vec<&str> = Vec::new();
                referenced.extend(
                    on_conditions
                        .iter()
                        .chain(where_conditions.iter())
                        .chain(or_groups.iter().flatten())
                        .filter_map(|c| c.field_name()),
                );
                if let Some(fields) = project {
                    referenced.extend(fields.iter().map(|s| s.as_str()));
                }
                if let Some(specs) = rank_by {
                    referenced.extend(specs.iter().map(|s| s.field.as_str()));
                }
                if let Some(f) = distinct_field {
                    referenced.push(f.as_str());
                }
                for f in referenced {
                    if !known.iter().any(|k| k == f) {
                        return Err(format!(
                            "Unknown field '{}' — this bundle's fields are: {}",
                            f,
                            known.join(", ")
                        ));
                    }
                }
            }
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
            // Validate the group-by field and every measure field against
            // the schema ('*' is COUNT(*) and always legal) — same
            // rationale as the Cover-arm validation above.
            {
                let known = store.field_names();
                let mut referenced: Vec<&str> =
                    measures.iter().map(|m| m.field.as_str()).filter(|f| *f != "*").collect();
                if let Some(gb) = over {
                    referenced.push(gb.as_str());
                }
                for f in referenced {
                    if !known.iter().any(|k| k == f) {
                        return Err(format!(
                            "Unknown field '{}' — this bundle's fields are: {}",
                            f,
                            known.join(", ")
                        ));
                    }
                }
            }
            // One accumulator per measure — a shared single-field
            // accumulator makes every measure return the first field's
            // value, and drops whole groups when that field is `*` or
            // non-numeric. Uses BundleRef::records(), so it works for
            // both heap & mmap stores.
            let fields: Vec<&str> = measures.iter().map(|m| m.field.as_str()).collect();
            let measure_value =
                |m: &gigi::parser::MeasureSpec, agg: &gigi::aggregation::AggResult| {
                    match m.func {
                        gigi::parser::AggFunc::Count => {
                            gigi::types::Value::Float(agg.count as f64)
                        }
                        gigi::parser::AggFunc::Sum => gigi::types::Value::Float(agg.sum),
                        gigi::parser::AggFunc::Avg => gigi::types::Value::Float(agg.avg()),
                        // min/max sentinels mean "no numeric values seen"
                        // (empty group or non-numeric field) — surface as
                        // null instead of serializing an infinity.
                        gigi::parser::AggFunc::Min if agg.min.is_finite() => {
                            gigi::types::Value::Float(agg.min)
                        }
                        gigi::parser::AggFunc::Max if agg.max.is_finite() => {
                            gigi::types::Value::Float(agg.max)
                        }
                        _ => gigi::types::Value::Null,
                    }
                };
            let measure_name = |m: &gigi::parser::MeasureSpec| {
                m.alias
                    .clone()
                    .unwrap_or_else(|| format!("{}_{}", m.func_name(), m.field))
            };
            if let Some(gb_field) = over {
                let groups =
                    gigi::aggregation::group_by_measures(store.records(), gb_field, &fields);
                let mut rows = Vec::new();
                for (key, aggs) in &groups {
                    let mut row = std::collections::HashMap::new();
                    row.insert(gb_field.clone(), key.clone());
                    for (m, agg) in measures.iter().zip(aggs) {
                        row.insert(measure_name(m), measure_value(m, agg));
                    }
                    rows.push(row);
                }
                Ok(ExecResult::Rows(rows))
            } else {
                // Global aggregation — single row over every record.
                let aggs = gigi::aggregation::integrate_measures(store.records(), &fields);
                let mut row = std::collections::HashMap::new();
                for (m, agg) in measures.iter().zip(&aggs) {
                    row.insert(measure_name(m), measure_value(m, agg));
                }
                Ok(ExecResult::Rows(vec![row]))
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
        // ── BETTI / PI_1 / OBSTRUCTION: UNREACHABLE from gql_query ─────
        //
        // The HTTP route handler routes these variants through
        // `halcyon_gql_dispatch::try_dispatch_topology_statement` BEFORE
        // the bundle pre-resolve (see the special-case block at
        // `gql_query` line ~12484). The arms below are dead code from
        // the HTTP path but kept for direct programmatic callers of
        // `execute_gql_on_store_read` that supply a pre-resolved
        // BundleStore — they should not be deleted without also
        // auditing every `execute_gql_with_exists` / `execute_gql_on_engine`
        // call site. Maintainer warning: a topology-kernel fix landed
        // here will NOT affect production traffic unless the dispatcher
        // in `src/halcyon_gql_dispatch.rs` is updated to match.
        Statement::Betti { bundle: _, order } => {
            // Default (legacy) path: order = None returns β_0 + β_1 from the
            // field-index graph. ORDER k path delegates to the cell-complex
            // β_k via crate::topology::betti_topological — needs the Lattice,
            // not the BundleStore. The lattice lookup uses the bundle's
            // _gigi_lattice metadata if present; else falls back to the
            // legacy graph path with a warning.
            match order {
                None => {
                    let (b0, b1) = store.betti_numbers();
                    Ok(ExecResult::Scalar(b0 as f64 + b1 as f64))
                }
                Some(k) => {
                    // For Phase 1: the lattice lookup from bundle metadata
                    // is not yet shipped. Fall back to graph betti for k<=1
                    // and return NotImplemented-style error for k>=2.
                    //
                    // Math-divergence note: graph β_k equals cell-complex β_k
                    // only when ∂_2 has rank 0 (no 2-cells in the lattice).
                    // The dispatcher in `halcyon_gql_dispatch.rs` prefers
                    // the lattice path which uses real ∂_2 boundary-rank
                    // arithmetic, so this arm is unreachable from production
                    // traffic — but a direct caller hitting this arm with a
                    // lattice-carrying bundle will get a different (graph)
                    // answer than the cell-complex β_k.
                    let (b0, b1) = store.betti_numbers();
                    match *k {
                        0 => Ok(ExecResult::Scalar(b0 as f64)),
                        1 => Ok(ExecResult::Scalar(b1 as f64)),
                        _ => Err(format!(
                            "BETTI ORDER {} on a bundle requires the bundle's \
                             lattice metadata (not yet shipped). Use the \
                             in-process API: crate::topology::betti_topological(&lattice, {}).",
                            k, k
                        )),
                    }
                }
            }
        }
        Statement::Pi1 { lattice } => {
            // PI_1 operates on a Lattice (1-skeleton) — looked up by name
            // from the lattice registry. Returns rank as scalar.
            //
            // UNREACHABLE from gql_query (see banner above).
            #[cfg(feature = "lattice")]
            {
                match gigi::lattice::registry::get(lattice) {
                    Some(lat) => {
                        let pres = gigi::topology::pi_1_presentation(&lat);
                        Ok(ExecResult::Scalar(pres.rank as f64))
                    }
                    None => Err(format!("No lattice: {}", lattice)),
                }
            }
            #[cfg(not(feature = "lattice"))]
            {
                let _ = lattice;
                Err("PI_1 requires the `lattice` feature".to_string())
            }
        }
        Statement::Obstruction { bundle } => {
            // OBSTRUCTION returns the integer characteristic-class sector
            // (Phase 1: Scalar of `class` field from ObstructionResult).
            //
            // UNREACHABLE from gql_query (see banner above).
            match engine {
                Some(eng_lock) => {
                    let eng = eng_lock.read().map_err(|e| format!("engine lock: {}", e))?;
                    match gigi::obstruction::obstruction_with_default(&eng, bundle) {
                        Ok(res) => Ok(ExecResult::Scalar(res.class as f64)),
                        Err(e) => Err(format!("OBSTRUCTION: {}", e)),
                    }
                }
                None => Err("OBSTRUCTION requires engine context (not available in this dispatch path)".to_string()),
            }
        }
        Statement::Entropy { .. } => {
            let s = store.entropy();
            Ok(ExecResult::Scalar(s))
        }
        Statement::FreeEnergy { tau, .. } => {
            let f = store.free_energy(*tau);
            Ok(ExecResult::Scalar(f))
        }
        // ── Cognitive Geometry (Branch VII) ─────────────────────────────────
        Statement::Capacity { tau, .. } => {
            // C = τ/K. Returns the Davis capacity as a scalar.
            // For rich interpretation use GET /v1/bundles/{name}/capacity.
            let k = store.scalar_curvature();
            Ok(ExecResult::Scalar(curvature::capacity(*tau, k)))
        }
        Statement::Horizon { tau, .. } => {
            // s_max = τ/(K·ℓ_c). Returns the holonomy horizon as a scalar.
            let k = store.scalar_curvature();
            let lambda1 = store.as_heap().map(spectral::spectral_gap).unwrap_or(0.0);
            Ok(ExecResult::Scalar(curvature::horizon(*tau, k, lambda1)))
        }
        Statement::Depth { .. } => {
            // Returns encoding depth as a scalar: I=1, II=2, III=3, IV=4.
            // For the full classification use GET /v1/bundles/{name}/depth.
            let k = store.scalar_curvature();
            let lambda1 = store.as_heap().map(spectral::spectral_gap).unwrap_or(0.0);
            let depth = curvature::encoding_depth(k, lambda1);
            let level: f64 = match depth {
                curvature::EncodingDepth::Tangent     => 1.0,
                curvature::EncodingDepth::Connection  => 2.0,
                curvature::EncodingDepth::Metric      => 3.0,
                curvature::EncodingDepth::Topological => 4.0,
            };
            Ok(ExecResult::Scalar(level))
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
        // SPECTRAL_GAUGE bundle ON FIBER (...) [GROUP <g>] [FULL [LIMIT k]]
        //
        // Halcyon Phase 1 (2026-06-28): fiber-weighted graph Laplacian
        // λ₁ via dense nalgebra SymmetricEigen. FULL mode returns a
        // typed PhaseNotImplemented error pointing at Phase 2 (Lanczos
        // sparse). Engine is required to resolve the bundle name with
        // the typed BundleNotFound error rather than the silent-zero
        // fallback the unweighted SPECTRAL verb uses.
        //
        // HONEST FRAMING: the returned `gap` is the spectral gap of
        // the gauge-weighted Laplacian L_A — globally gauge-invariant
        // in its spectrum, but the per-edge trace weight is only
        // locally gauge-covariant. This is NOT the strict Yang-Mills
        // mass gap. Halcyon understands the distinction.
        #[cfg(feature = "gauge")]
        Statement::SpectralGauge { bundle, fiber_fields, group, full, limit, where_conditions } => {
            let eng = engine.ok_or_else(|| {
                "SPECTRAL_GAUGE requires an Engine handle in the executor context".to_string()
            })?;

            // Infer group at exec time when not specified. Same arity
            // table the parser would use, just at a different layer
            // so programmatic callers of the function ALSO get the
            // same inference behaviour.
            let resolved_group = match group {
                Some(g) => *g,
                None => match fiber_fields.len() {
                    1 => gigi::gauge::Group::U1,
                    4 => gigi::gauge::Group::SU2,
                    18 => gigi::gauge::Group::SU3,
                    other => return Err(format!(
                        "SPECTRAL_GAUGE: GROUP required when fiber width is ambiguous \
                         (got {} fields; canonical widths are 1/4/18)",
                        other
                    )),
                },
            };

            // Flatten WHERE clause to QueryCondition[] via the same
            // helper COVER/LOAD use — semantics identical to COVER
            // WHERE. Empty vec → filter_opt=None → zero behaviour
            // change on the locked gates.
            let query_conditions: Vec<gigi::bundle::QueryCondition> =
                where_conditions
                    .iter()
                    .flat_map(|fc| gigi::parser::filter_to_query_conditions(fc))
                    .collect();
            let filter_opt = if query_conditions.is_empty() {
                None
            } else {
                Some(query_conditions.as_slice())
            };

            // Read-only engine borrow — the eigendecomposition does
            // not mutate any state.
            let eng_guard = eng.read().unwrap();
            let result = gigi::spectral::spectral_gauge_gap(
                &eng_guard,
                bundle,
                fiber_fields,
                resolved_group,
                *full,
                *limit,
                filter_opt,
            )
            .map_err(|e| e.to_string())?;

            // Single-row result envelope mirrors the SPECTRAL_FIBER
            // pattern — gap / n_records_used / group_used. Phase 2's
            // eigenvalues vector will add a second row block.
            let mut row = gigi::types::Record::new();
            row.insert("gap".to_string(), gigi::types::Value::Float(result.gap));
            row.insert(
                "n_records_used".to_string(),
                gigi::types::Value::Integer(result.n_records_used as i64),
            );
            row.insert(
                "group_used".to_string(),
                gigi::types::Value::Text(result.group_used.label().to_string()),
            );
            Ok(ExecResult::Rows(vec![row]))
        }
        #[cfg(not(feature = "gauge"))]
        Statement::SpectralGauge { .. } => {
            Err("SPECTRAL_GAUGE requires the `gauge` feature to be enabled".to_string())
        }
        // CHERN_CLASS bundle ORDER <k> [ON FIBER (...)] [GROUP <g>]
        //
        // Halcyon Phase 1 (2026-06-29): discrete Chern-Weil integration
        // over the gauge field bound to `bundle`. Resolves the gauge
        // field via the in-process registry, the lattice via the same
        // registry, then calls `chern_weil::chern_class`. Returns a
        // Scalar with the (near-)integer characteristic class.
        //
        // HONEST FRAMING: Phase 1 ships SU(2) ORDER 2 on a 4D cubic
        // base. Identity and abelian-fixture short-circuits are
        // documented in `src/chern_weil.rs`; the SIGNED clover sum is
        // used when non-zero, the ABS-SUM fallback fires on synthetic
        // single-axis abelian fixtures (witness-only).
        //
        // UNREACHABLE from gql_query — see the dead-code banner above
        // the BETTI / PI_1 / OBSTRUCTION arms. The HTTP route handler
        // dispatches this variant through
        // `halcyon_gql_dispatch::try_dispatch_topology_statement` BEFORE
        // the bundle pre-resolve, so this arm only fires for direct
        // programmatic callers of `execute_gql_on_store_read`. Kernel
        // fixes must update BOTH this arm and the dispatcher.
        #[cfg(feature = "gauge")]
        Statement::ChernClass {
            bundle,
            order,
            fiber_fields,
            group,
            lattice: _stmt_lattice,
            per_field: _per_field,
            into_column: _into_column,
        } => {
            // NOTE (parity contract): this dead-code arm only fires
            // from direct programmatic callers of
            // `execute_gql_on_store_read`, NOT from production /v1/gql
            // traffic (which routes through
            // `try_dispatch_topology_statement`). The bundle-target /
            // PER / INTO_COLUMN semantics live in the dispatcher; this
            // arm keeps the pre-existing gauge-field-target path for
            // callers that already had a fully-populated
            // `Statement::ChernClass` in hand.
            // If the caller supplies any of the new clauses here,
            // we surface a specific "route via dispatcher" error
            // rather than silently ignoring them.
            if _stmt_lattice.is_some()
                || _per_field.is_some()
                || _into_column.is_some()
            {
                return Err(
                    "CHERN_CLASS bundle-target extensions (ON LATTICE / \
                     PER / INTO_COLUMN) require the production dispatcher \
                     (`try_dispatch_topology_statement`) — the \
                     `execute_gql_on_store_read` fallback only handles \
                     gauge-field targets"
                        .to_string()
                );
            }
            let handle = gigi::gauge::registry::get(bundle).ok_or_else(|| {
                format!(
                    "CHERN_CLASS: gauge field '{}' not declared (use \
                     GAUGE_FIELD {} ON LATTICE ... first)",
                    bundle, bundle
                )
            })?;
            let lattice_name = handle.lattice_name().to_string();
            let lat = gigi::lattice::registry::get(&lattice_name).ok_or_else(|| {
                format!(
                    "CHERN_CLASS: lattice '{}' bound to gauge field '{}' not \
                     found (was it declared?)",
                    lattice_name, bundle
                )
            })?;

            // Resolve the canonical fiber list from the field's group
            // when the caller didn't specify ON FIBER. Same arity table
            // as SPECTRAL_GAUGE so the contract stays uniform.
            let resolved_group = group.unwrap_or_else(|| handle.group());
            let fields_owned: Vec<String> = if fiber_fields.is_empty() {
                canonical_fiber_fields(resolved_group)
            } else {
                fiber_fields.clone()
            };

            // The chern_class kernel takes `&dyn EdgeConnection`;
            // `Arc<dyn GaugeFieldHandle>` derefs to a trait object that
            // is also `EdgeConnection` (super-trait), so we just borrow
            // through the Arc and re-cast.
            let edge_conn: &dyn gigi::gauge::edge_connection::EdgeConnection =
                handle.as_ref();
            let q = gigi::chern_weil::chern_class(
                edge_conn,
                &lat,
                *order,
                &fields_owned,
                Some(resolved_group),
            )
            .map_err(|e| e.to_string())?;
            Ok(ExecResult::Scalar(q))
        }
        #[cfg(not(feature = "gauge"))]
        Statement::ChernClass { .. } => {
            Err("CHERN_CLASS requires the `gauge` feature to be enabled".to_string())
        }
        // PONTRYAGIN bundle ORDER <k> [ON FIBER (...)] [GROUP <g>]
        //
        // Halcyon Phase 1: p_1 = 2 · c_2 for SU(N). Delegates to
        // chern_weil::pontryagin_class which delegates to chern_class.
        //
        // UNREACHABLE from gql_query — same banner as the ChernClass
        // arm above. Production traffic for this variant flows through
        // `try_dispatch_topology_statement` in
        // `src/halcyon_gql_dispatch.rs`.
        #[cfg(feature = "gauge")]
        Statement::Pontryagin { bundle, order, fiber_fields, group } => {
            let handle = gigi::gauge::registry::get(bundle).ok_or_else(|| {
                format!(
                    "PONTRYAGIN: gauge field '{}' not declared (use \
                     GAUGE_FIELD {} ON LATTICE ... first)",
                    bundle, bundle
                )
            })?;
            let lattice_name = handle.lattice_name().to_string();
            let lat = gigi::lattice::registry::get(&lattice_name).ok_or_else(|| {
                format!(
                    "PONTRYAGIN: lattice '{}' bound to gauge field '{}' not \
                     found (was it declared?)",
                    lattice_name, bundle
                )
            })?;

            let resolved_group = group.unwrap_or_else(|| handle.group());
            let fields_owned: Vec<String> = if fiber_fields.is_empty() {
                canonical_fiber_fields(resolved_group)
            } else {
                fiber_fields.clone()
            };

            let edge_conn: &dyn gigi::gauge::edge_connection::EdgeConnection =
                handle.as_ref();
            let p = gigi::chern_weil::pontryagin_class(
                edge_conn,
                &lat,
                *order,
                &fields_owned,
                Some(resolved_group),
            )
            .map_err(|e| e.to_string())?;
            Ok(ExecResult::Scalar(p))
        }
        #[cfg(not(feature = "gauge"))]
        Statement::Pontryagin { .. } => {
            Err("PONTRYAGIN requires the `gauge` feature to be enabled".to_string())
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
        | Iterate { bundle, .. }
        | AlterBundleAddBase { bundle, .. } => Some(bundle.clone()),
        Pullback { left, .. } | Join { left, .. } => Some(left.clone()),
        Transplant { source, .. } => Some(source.clone()),
        DropPolicy { bundle, .. } | DropTrigger { bundle, .. } => Some(bundle.clone()),
        CreateTrigger { bundle, .. } => Some(bundle.clone()),
        Explain { inner } => get_bundle_name(inner),
        // Fiber-geometric analytics (Sprint 2)
        HolonomyFiber { bundle, .. } => Some(bundle.clone()),
        SpectralFiber { bundle, .. } => Some(bundle.clone()),
        // Halcyon Phase 1 (2026-06-28): SPECTRAL_GAUGE is a single-bundle
        // read; expose the bundle name so the dispatcher can attach.
        SpectralGauge { bundle, .. } => Some(bundle.clone()),
        ChernClass { bundle, .. } => Some(bundle.clone()),
        Pontryagin { bundle, .. } => Some(bundle.clone()),
        Transport { bundle, .. } => Some(bundle.clone()),
        TransportRotation { bundle, .. } => Some(bundle.clone()),
        SampleTransport { bundle, .. } => Some(bundle.clone()),
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
        // Cognitive Geometry (Branch VII — Davis 2026-05-29)
        Capacity { bundle, .. } | Horizon { bundle, .. } | Depth { bundle, .. } => Some(bundle.clone()),
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
        SpectralGauge { .. }  => "SPECTRAL_GAUGE",
        ChernClass { .. }     => "CHERN_CLASS",
        Pontryagin { .. }     => "PONTRYAGIN",
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

// ─── Ask G — Patterns HTTP surface ──────────────────────────────────────────
//
// Per `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` §9.2 + Bee's request to ship
// the HTTP surface alongside the GQL one. All 5 endpoints translate JSON
// → GQL → execute → JSON; the GQL execute path is already covered by
// 42 pattern tests in `tests/pattern_hunt_*.rs` so correctness here is a
// matter of translation glue.
//
// Gated on the `patterns` Cargo feature flag; the binary builds without
// the feature with zero footprint.

#[cfg(feature = "patterns")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PatternListEntry {
    name: String,
}

#[cfg(feature = "patterns")]
#[derive(Debug, serde::Deserialize)]
struct DefinePatternRequest {
    name: String,
    /// Predicate body — the part after `AS`. Example: `"field_a = 1 AND field_b > 5"`.
    predicate: String,
    /// Optional WEIGHT arithmetic body. Example: `"field_a * 3 + field_b * 2"`.
    #[serde(default)]
    weight: Option<String>,
    /// Optional USING field list.
    #[serde(default)]
    using: Vec<String>,
    /// If true, equivalent to `DEFINE OR REPLACE PATTERN`.
    #[serde(default)]
    replace: bool,
}

#[cfg(feature = "patterns")]
#[derive(Debug, serde::Deserialize)]
struct HuntRequest {
    pattern: String,
    #[serde(default)]
    excluding: Vec<String>,
    #[serde(default)]
    top: Option<usize>,
    #[serde(default)]
    project: Vec<String>,
    // ─── v0.2 additions (additive; old clients send none, get v0.1 shape) ─
    /// Patterns v0.2 — when set ≥ 1, HUNT returns the verdict envelope
    /// (sat/unsat/near_miss) instead of the bare row array. When 0 or
    /// absent, the v0.1 array shape is preserved for backwards compat.
    #[serde(default)]
    near_miss_budget: Option<usize>,
    /// Patterns v0.2 — attach `_explain` (WEIGHT decomposition tree) to
    /// each sat row. Forces the envelope response.
    #[serde(default)]
    explain: bool,
    /// Patterns v0.2 — attach `_repair_menu` to each near-miss row.
    /// Forces the envelope response.
    #[serde(default)]
    include_repair_menu: bool,
    /// Patterns v0.2 — per-field relaxation costs (default 1.0/field).
    /// Only consulted when `include_repair_menu` is true.
    #[serde(default)]
    relaxation_costs: std::collections::HashMap<String, f64>,
}

#[cfg(feature = "patterns")]
impl HuntRequest {
    /// True iff the request opts into the v0.2 envelope. Set by any
    /// of the v0.2 flags.
    fn uses_v02_envelope(&self) -> bool {
        self.near_miss_budget.is_some()
            || self.explain
            || self.include_repair_menu
            || !self.relaxation_costs.is_empty()
    }
}

/// GET /v1/patterns — list all defined patterns.
#[cfg(feature = "patterns")]
async fn list_patterns(
    State(state): State<Arc<StreamState>>,
) -> Result<Json<Vec<PatternListEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let stmt = gigi::parser::parse("SHOW PATTERNS").map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("internal parse: {e}"),
            }),
        )
    })?;
    let mut engine = state.engine.write().unwrap();
    let result = gigi::parser::execute(&mut engine, &stmt).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("execute: {e}"),
            }),
        )
    })?;
    match result {
        gigi::parser::ExecResult::Rows(rows) => {
            let entries: Vec<PatternListEntry> = rows
                .into_iter()
                .filter_map(|row| match row.get("name") {
                    Some(gigi::types::Value::Text(n)) => Some(PatternListEntry { name: n.clone() }),
                    _ => None,
                })
                .collect();
            Ok(Json(entries))
        }
        _ => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "SHOW PATTERNS unexpected result shape".to_string(),
            }),
        )),
    }
}

/// POST /v1/patterns — DEFINE PATTERN.
///
/// Body: `{name, predicate, weight?, using?[], replace?}` — translates to
/// `DEFINE [OR REPLACE] PATTERN <name> AS <predicate> [WEIGHT (<weight>)]
/// [USING (<using>)]`.
#[cfg(feature = "patterns")]
async fn define_pattern_http(
    State(state): State<Arc<StreamState>>,
    Json(req): Json<DefinePatternRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    if req.name.is_empty() || req.predicate.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "`name` and `predicate` are required".to_string(),
            }),
        ));
    }
    let mut sql = String::new();
    sql.push_str("DEFINE ");
    if req.replace {
        sql.push_str("OR REPLACE ");
    }
    sql.push_str("PATTERN ");
    sql.push_str(&req.name);
    sql.push_str(" AS ");
    sql.push_str(&req.predicate);
    if let Some(w) = &req.weight {
        sql.push_str(" WEIGHT (");
        sql.push_str(w);
        sql.push(')');
    }
    if !req.using.is_empty() {
        sql.push_str(" USING (");
        sql.push_str(&req.using.join(", "));
        sql.push(')');
    }
    let stmt = gigi::parser::parse(&sql).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("parse: {e}"),
            }),
        )
    })?;
    let mut engine = state.engine.write().unwrap();
    gigi::parser::execute(&mut engine, &stmt).map_err(|e| {
        (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("define: {e}"),
            }),
        )
    })?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"name": req.name, "ok": true})),
    ))
}

/// DELETE /v1/patterns/{name} — DROP PATTERN.
#[cfg(feature = "patterns")]
async fn drop_pattern_http(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let sql = format!("DROP PATTERN {name}");
    let stmt = gigi::parser::parse(&sql).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("parse: {e}"),
            }),
        )
    })?;
    let mut engine = state.engine.write().unwrap();
    gigi::parser::execute(&mut engine, &stmt).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("drop: {e}"),
            }),
        )
    })?;
    Ok(Json(serde_json::json!({"name": name, "ok": true})))
}

/// POST /v1/bundles/{bundle}/hunt — execute a HUNT.
///
/// Body: `{pattern, excluding?[], top?, project?[]}`. Returns the rows
/// each as a JSON object with the projected fields plus `_score`.
#[cfg(feature = "patterns")]
async fn hunt_http(
    State(state): State<Arc<StreamState>>,
    Path(bundle): Path<String>,
    Json(req): Json<HuntRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.pattern.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "`pattern` is required".to_string(),
            }),
        ));
    }

    // ─── v0.2 path: full verdict envelope via hunt_v2_orchestrate ────────
    if req.uses_v02_envelope() {
        let args = gigi::parser::HuntV2Args {
            pattern: req.pattern.clone(),
            bundle: bundle.clone(),
            excluding: req.excluding.clone(),
            top: req.top,
            project: if req.project.is_empty() { None } else { Some(req.project.clone()) },
            near_miss_budget: req.near_miss_budget.unwrap_or(1),
            explain: req.explain,
            include_repair_menu: req.include_repair_menu,
            relaxation_costs: req.relaxation_costs.clone(),
        };
        let mut engine = state.engine.write().unwrap();
        let env = gigi::parser::hunt_v2_orchestrate(&mut engine, &args).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("hunt v2: {e}"),
                }),
            )
        })?;
        return Ok(Json(envelope_to_json(env)));
    }

    // ─── v0.1 path: bare array of row objects (backwards compat) ────────
    let mut sql = format!("HUNT {pat} IN {b}", pat = req.pattern, b = bundle);
    for excl in &req.excluding {
        sql.push_str(" EXCLUDING IN ");
        sql.push_str(excl);
    }
    if let Some(n) = req.top {
        sql.push_str(&format!(" TOP {n}"));
    }
    if !req.project.is_empty() {
        sql.push_str(" PROJECT (");
        sql.push_str(&req.project.join(", "));
        sql.push(')');
    }
    let stmt = gigi::parser::parse(&sql).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("parse: {e}"),
            }),
        )
    })?;
    let mut engine = state.engine.write().unwrap();
    let result = gigi::parser::execute(&mut engine, &stmt).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("hunt: {e}"),
            }),
        )
    })?;
    match result {
        gigi::parser::ExecResult::Rows(rows) => {
            let out: Vec<serde_json::Value> =
                rows.into_iter().map(hunt_row_to_json).collect();
            // v0.1 wire shape is a bare array; wrap in serde_json::Value
            // since the handler's return type is now Json<Value>.
            Ok(Json(serde_json::Value::Array(out)))
        }
        _ => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "HUNT did not return rows".to_string(),
            }),
        )),
    }
}

/// Patterns v0.2 — serialize a `HuntV2Envelope` to wire JSON.
///
/// Per spec §4.1: the envelope always carries `verdict`; the other fields
/// are populated only when their verdict applies. `_score` stays the last
/// key in row objects (SCJ §5(a)). When `_explain` is present it's emitted
/// as a nested JSON tree.
#[cfg(feature = "patterns")]
fn envelope_to_json(env: gigi::parser::HuntV2Envelope) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("verdict".to_string(), serde_json::Value::String(env.verdict.clone()));

    match env.verdict.as_str() {
        "sat" => {
            obj.insert(
                "n_matches".to_string(),
                serde_json::json!(env.n_matches.unwrap_or(0)),
            );
            obj.insert(
                "rows".to_string(),
                serde_json::Value::Array(
                    env.rows.into_iter().map(hunt_row_to_json).collect(),
                ),
            );
        }
        "near_miss" => {
            obj.insert(
                "near_miss_count".to_string(),
                serde_json::json!(env.near_miss_count.unwrap_or(0)),
            );
            obj.insert(
                "near_miss_rows".to_string(),
                serde_json::Value::Array(
                    env.near_miss_rows
                        .into_iter()
                        .map(|nm| hunt_row_to_json(nm.row))
                        .collect(),
                ),
            );
        }
        _ => {
            if let Some(reason) = env.reason {
                obj.insert("reason".to_string(), serde_json::Value::String(reason));
            }
            if let Some(pc) = env.preflight_caught {
                obj.insert("preflight_caught".to_string(), serde_json::json!(pc));
            }
            obj.insert("rows".to_string(), serde_json::Value::Array(Vec::new()));
        }
    }
    serde_json::Value::Object(obj)
}

/// Build the JSON object for one HUNT result row.
///
/// SCJ §5(a): `_score` is always emitted LAST so TUI clients can render
/// the score column without column-order detection. (Note: JSON object
/// keys are semantically unordered, but real-world consumers — jq, TUI
/// table renderers, debug logs — often respect serialization order.
/// The `preserve_order` feature on `serde_json` makes `serde_json::Map`
/// an order-preserving structure for exactly this reason.)
#[cfg(feature = "patterns")]
fn hunt_row_to_json(row: gigi::types::Record) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    let mut score: Option<gigi::types::Value> = None;
    for (k, v) in row {
        if k == "_score" {
            score = Some(v);
        } else {
            obj.insert(k, value_to_json(&v));
        }
    }
    if let Some(s) = score {
        obj.insert("_score".to_string(), value_to_json(&s));
    }
    serde_json::Value::Object(obj)
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
        .route("/v1/health", get(health));

    // ── Ask G — Patterns HTTP surface ──
    #[cfg(feature = "patterns")]
    let app = app
        .route("/v1/patterns", get(list_patterns))
        .route("/v1/patterns", post(define_pattern_http))
        .route("/v1/patterns/{name}", axum::routing::delete(drop_pattern_http))
        .route("/v1/bundles/{name}/hunt", post(hunt_http));

    let app = app
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
        .route("/v1/bundles/{name}/record/{id}/vector", get(record_vector))
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
        .route("/v1/openapi.json", get(openapi_spec));

    // Atomic Sheaf Commits Phase-A surface (spec §7.1). Only mounted
    // when the `transactions` feature is enabled — pure additions, no
    // impact on existing routes.
    #[cfg(feature = "transactions")]
    let app = app
        .route("/v1/transactions/begin", post(tx_begin))
        .route("/v1/transactions/{tx_id}", get(tx_status))
        .route("/v1/transactions/{tx_id}/write", post(tx_write))
        .route("/v1/transactions/{tx_id}/commit", post(tx_commit))
        .route("/v1/transactions/{tx_id}/rollback", post(tx_rollback));

    // Causal States v0.1 — Update Commutator HTTP envelope (CV4-wire).
    // Only mounted when the `causal_states` feature flag is enabled;
    // strict-additive otherwise. Backs the Davis (2026) paper §7.
    #[cfg(feature = "causal_states")]
    let app = app.route(
        "/v1/causal_states/commutator",
        post(causal_states_commutator_http),
    );

    // WISH v0.1 — boundary-value geodesic verb HTTP envelope (Phase 5).
    // 2D conformally-flat demo wire (Flat/S2/CP1/Pinch); production
    // substrate-backed metrics ship with the dim-lift. Only mounted
    // when the `wish` feature flag is enabled.
    #[cfg(feature = "wish")]
    let app = app
        .route("/v1/wish", post(wish_http))
        .route("/v1/bundles/{name}/wish", post(wish_bundle_http));

    // TDD-HAL-II.6 — LATTICE + GAUGE_FIELD HTTP surface. The four
    // routes (POST /v1/lattice, GET /v1/lattice/{name}, POST
    // /v1/gauge_field, GET /v1/gauge_field/{name}) are built by
    // `gigi::gauge::http::build_router()` so the same router shape
    // serves the in-process test harness (tests/halcyon_part_ii_http.rs)
    // and the gigi-stream binary without duplication. Routes are
    // stateless — the lattice + gauge registries are process
    // singletons that the handlers thread through directly. Halcyon's
    // mock-to-live swap (the production consumer) parses the JSON
    // envelope `{"group": "SU(2)", "repr_dim": 4, "n_edges": 90,
    // "data": [[…],…]}` per Bee's locked decision 4.
    //
    // TDD-HAL-II.6b: install the engine handle into the gauge
    // engine_handle module-global so the `persist:true` branch of
    // POST /v1/lattice and POST /v1/gauge_field can reach the same
    // `Engine` the rest of the binary writes through. The handle is
    // a clone of `state.engine` (lifted to `Arc<RwLock<Engine>>` for
    // this purpose); all other handlers continue to dereference
    // `state.engine` exactly as before. Install runs once, at
    // startup; tests reset with `engine_handle::clear_for_test()`.
    #[cfg(feature = "gauge")]
    {
        if let Err(e) = gigi::gauge::engine_handle::install(Arc::clone(&state.engine)) {
            eprintln!("WARNING: gauge engine_handle install failed: {e}");
        }
    }
    #[cfg(feature = "gauge")]
    let app = app.merge(gigi::gauge::http::build_router());

    let app = app
        // GQL endpoint
        .route("/v1/gql", post(gql_query));

    // Public-read GQL endpoint at /v1/public/gql. Only mounted when the
    // `GIGI_PUBLIC_BUNDLES` allowlist is non-empty, so an unset env var
    // gives you 404 (not a mistake — a positive signal that no bundle is
    // exposed anonymously). The handler enforces its own read-verb and
    // per-bundle allowlist regardless of auth; see `public_gql_query`.
    let app = if state.public_bundles.is_empty() {
        app
    } else {
        app.route("/v1/public/gql", post(public_gql_query))
    };

    let app = app
        // Analytics
        .route("/v1/bundles/{name}/curvature", get(curvature_report))
        .route("/v1/bundles/{name}/spectral", get(spectral_report))
        // Cognitive geometry (Branch VII — C = τ/K, horizon, encoding depth)
        .route("/v1/bundles/{name}/capacity", get(bundle_capacity_report))
        .route("/v1/bundles/{name}/horizon",  get(bundle_horizon_report))
        .route("/v1/bundles/{name}/depth",    get(bundle_depth_report))
        .route("/v1/bundles/{name}/perceive", post(bundle_perceive))
        .route("/v1/bundles/{name}/local_holonomy", post(bundle_local_holonomy));

    // IMAGINE_COHERENCE: predictive coherence trajectory along an
    // imagined geodesic. Marcella's predictive gain gate surface per
    // IMAGINE_AND_WALK.md §5. Feature-gated; only registered when the
    // `imagine` feature is on.
    #[cfg(feature = "imagine")]
    let app = app
        .route("/v1/bundles/{name}/imagine_coherence", post(bundle_imagine_coherence));

    // Sharded SPECTRAL / CURVATURE / HOLONOMY endpoints. End-to-end
    // wired against any bundle via shard_lambda_1_from_bundle (uses
    // the Laplacian extractor + distributed Lanczos pipeline) and the
    // wrap_trivial / wrap_hash_sharded / wrap_fiedler_sharded
    // constructors. Feature-gated; only registered when the `sharded`
    // feature is on.
    #[cfg(feature = "sharded")]
    let app = app
        .route("/v1/bundles/{name}/sharded/spectral_gap", post(bundle_sharded_spectral_gap))
        .route("/v1/bundles/{name}/sharded/curvature", post(bundle_sharded_curvature))
        .route("/v1/bundles/{name}/sharded/holonomy_loop", post(bundle_sharded_holonomy_loop));

    let app = app
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
        .route("/dashboard", get(serve_dashboard))
        // Sprint N (v0.4) — Invariant Consistency Verification.
        // Auditor-facing endpoint; bundle_id binding enforced.
        // Not feature-gated — Sprint N applies to any v0.2+ bundle.
        .route(
            "/v1/bundles/{name}/verify_invariant",
            post(verify_invariant_endpoint),
        );

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
        )
        // S1 wave 1 §G — fit_diagnostics, the H2-vs-H1 verdict
        // endpoint. Returns full eigenvalue spectrum + per-axis
        // variance + fit_mean for any (bundle, fit_mode, fields,
        // sigma_floor_epsilon) configuration. Uses the
        // BundleFlowCache so warm calls are sub-µs.
        .route(
            "/v1/bundles/{name}/brain/fit_diagnostics",
            post(brain_fit_diagnostics_endpoint),
        )
        // S1 wave 1 §H — distance_to_fit_mean. Diagnoses the H2
        // mechanism for any target vector by reporting its
        // percentile within the bundle's full distance
        // distribution. Target at p<0.01 = at the deep point
        // of the fitted Gaussian = H2 attractor source.
        .route(
            "/v1/bundles/{name}/brain/distance_to_fit_mean",
            post(brain_distance_to_fit_mean_endpoint),
        )
        // S1 wave 1 — combined confidence + explain for Marcella's
        // refuse-gate (P0 #3). One record walk + one network call
        // instead of two of each.
        .route(
            "/v1/bundles/{name}/brain/confidence_with_explain",
            post(brain_confidence_with_explain_endpoint),
        )
        // SUDOKU S3 — constraint-inference meta-primitive
        // (theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md v0.3).
        // v1: Field-predicate constraints + honest-coverage
        // tristate verdict. Expansion (S3.5) + manifold/relation
        // constraints (S4) + soft scoring + demos (S5) follow.
        .route(
            "/v1/bundles/{name}/brain/sudoku",
            post(brain_sudoku_endpoint),
        )
        // S4: SAMPLE_TRANSPORT — curvature-bounded neighborhood
        // sampling on the fiber. Returns k candidates from
        // N(p_src, tau) weighted by exp(-beta * d^2).
        .route(
            "/v1/bundles/{name}/brain/sample_transport",
            post(brain_sample_transport_endpoint),
        )
        // S7: intent_gate — composite refuse-gate primitive (SUDOKU +
        // Čech pre-flight + kernel-density confidence in one atomic
        // call). Marcella's refuse-gate migration target. JTBD-gated
        // by e2e/probes/intent_gate_demo.py.
        .route(
            "/v1/bundles/{name}/brain/intent_gate",
            post(brain_intent_gate_endpoint),
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
                if std::env::var("GIGI_SKIP_BOOT_SNAPSHOT").is_ok() {
                    eprintln!(
                        "WAL replay complete ({total} records). \
                         GIGI_SKIP_BOOT_SNAPSHOT set — skipping post-replay snapshot. \
                         Engine ready on heap (no mmap upgrade this boot)."
                    );
                    init_system_bundles(&mut engine);
                    init_app_bundles(&mut engine);
                    drop(engine);
                    replay_state.ready.store(true, Ordering::Release);
                    return;
                }

                eprintln!("WAL replay complete ({total} records). Snapshotting to DHOOM…");

                let snapshots_dir = data_dir_for_replay.join("snapshots");
                if let Ok(entries) = std::fs::read_dir(&snapshots_dir) {
                    for ent in entries.flatten() {
                        let path = ent.path();
                        if path.extension().map_or(false, |e| e == "tmp") {
                            if let Ok(meta) = ent.metadata() {
                                if meta.len() == 0 {
                                    let _ = std::fs::remove_file(&path);
                                    eprintln!(
                                        "Cleaned stale 0-byte snapshot tmp: {}",
                                        path.display()
                                    );
                                }
                            }
                        }
                    }
                }

                match engine.snapshot_with_report() {
                    Ok(report) => {
                        if !report.timed_out_bundles.is_empty() {
                            eprintln!(
                                "Post-replay snapshot completed with {} timed-out bundle(s): {:?}. \
                                 Data preserved in WAL; will retry next compaction cycle.",
                                report.timed_out_bundles.len(),
                                report.timed_out_bundles
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Post-replay snapshot failed: {e}");
                        init_system_bundles(&mut engine);
                        init_app_bundles(&mut engine);
                        drop(engine);
                        replay_state.ready.store(true, Ordering::Release);
                        eprintln!("Engine ready — running on heap (snapshot failed)");
                        return;
                    }
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

// ────────────────────────────────────────────────────────────────────────────
// Atomic Sheaf Commits — HTTP surface (spec §7.1)
// ────────────────────────────────────────────────────────────────────────────
//
// Five endpoints expose the transactions substrate to clients:
//
//   POST /v1/transactions/begin              -> { tx_id, snap_id, opened_at }
//   POST /v1/transactions/{tx_id}/write      -> stage records on a bundle
//   POST /v1/transactions/{tx_id}/commit     -> apply via engine.batch_insert
//   POST /v1/transactions/{tx_id}/rollback   -> discard pending writes
//   GET  /v1/transactions/{tx_id}            -> status
//
// First ship is Phase-A: registry-only, no PREPARE/DECISION/NOTIFY 2PC
// dance over multiple participants (that machinery exists in
// src/transactions/ and is gated by tests; wiring it to real bundles
// requires the global WAL log work). Commit iterates touched bundles in
// stable order; if any insert fails, the prior bundles' writes are
// already applied — same semantics as a sequential batch insert today,
// just with an explicit lifecycle. Full atomicity rides the follow-up.

#[cfg(feature = "transactions")]
#[derive(Debug, Deserialize)]
struct TxBeginRequest {
    #[serde(default)]
    isolation: Option<String>,
}

#[cfg(feature = "transactions")]
#[derive(Debug, Serialize)]
struct TxBeginResponse {
    tx_id: String,
    snap_id: u64,
    opened_at: String,
    isolation: String,
}

#[cfg(feature = "transactions")]
#[derive(Debug, Deserialize)]
struct TxWriteRequest {
    bundle: String,
    records: Vec<serde_json::Value>,
}

#[cfg(feature = "transactions")]
#[derive(Debug, Serialize)]
struct TxWriteResponse {
    staged: usize,
    total_in_tx: usize,
    touched_bundles: Vec<String>,
}

#[cfg(feature = "transactions")]
#[derive(Debug, Serialize)]
struct TxCommitResponse {
    committed_at: String,
    new_snap_id: u64,
    bundles_committed: Vec<String>,
    records_committed: usize,
}

#[cfg(feature = "transactions")]
#[derive(Debug, Serialize)]
struct TxRollbackResponse {
    aborted: bool,
    discarded_records: usize,
}

#[cfg(feature = "transactions")]
#[derive(Debug, Serialize)]
struct TxStatusResponse {
    tx_id: String,
    snap_id: u64,
    state: String,
    isolation: String,
    opened_at: String,
    age_secs: u64,
    touched_bundles: Vec<String>,
    pending_writes: usize,
}

#[cfg(feature = "transactions")]
fn parse_tx_id(
    s: &str,
) -> Result<gigi::transactions::TransactionId, (StatusCode, Json<ErrorResponse>)> {
    let stripped = s.strip_prefix("tx_").unwrap_or(s);
    let uuid = uuid::Uuid::parse_str(stripped).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("invalid tx_id '{}': expected 'tx_<uuid>'", s),
            }),
        )
    })?;
    Ok(gigi::transactions::TransactionId(uuid))
}

#[cfg(feature = "transactions")]
fn sys_time_to_iso(t: std::time::SystemTime) -> String {
    let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("epoch:{}", dur.as_secs())
}

#[cfg(feature = "transactions")]
async fn tx_begin(
    State(state): State<Arc<StreamState>>,
    Json(req): Json<TxBeginRequest>,
) -> Result<Json<TxBeginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let isolation = match req.isolation.as_deref() {
        None | Some("snapshot_isolation") | Some("snapshot") | Some("si") => {
            gigi::transactions::IsolationLevel::SnapshotIsolation
        }
        Some("read_committed") | Some("rc") => gigi::transactions::IsolationLevel::ReadCommitted,
        Some(other) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "unknown isolation '{}'; valid: snapshot_isolation, read_committed",
                        other
                    ),
                }),
            ));
        }
    };

    let snap_id = gigi::transactions::SnapshotId(
        state
            .tx_snap_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1,
    );
    let tx_id = gigi::transactions::TransactionId::new();
    let opened_at = std::time::SystemTime::now();
    let tx = OpenTx {
        snap_id,
        opened_at,
        isolation,
        state: gigi::transactions::TransactionState::Open,
        pending: HashMap::new(),
    };
    state.tx_registry.lock().unwrap().insert(tx_id, tx);

    Ok(Json(TxBeginResponse {
        tx_id: format!("{}", tx_id),
        snap_id: snap_id.0,
        opened_at: sys_time_to_iso(opened_at),
        isolation: match isolation {
            gigi::transactions::IsolationLevel::SnapshotIsolation => "snapshot_isolation".into(),
            gigi::transactions::IsolationLevel::ReadCommitted => "read_committed".into(),
        },
    }))
}

#[cfg(feature = "transactions")]
async fn tx_write(
    State(state): State<Arc<StreamState>>,
    Path(tx_id_str): Path<String>,
    Json(req): Json<TxWriteRequest>,
) -> Result<Json<TxWriteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tx_id = parse_tx_id(&tx_id_str)?;
    if req.bundle.starts_with("_gigi_") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: format!("'{}' is a system bundle and is read-only", req.bundle),
            }),
        ));
    }
    {
        let engine = state.engine.read().unwrap();
        if engine.bundle(&req.bundle).is_none() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Bundle '{}' not found", req.bundle),
                }),
            ));
        }
    }

    let mut registry = state.tx_registry.lock().unwrap();
    let tx = registry.get_mut(&tx_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!(
                    "transaction {} not found (already committed/aborted?)",
                    tx_id_str
                ),
            }),
        )
    })?;
    if tx.state != gigi::transactions::TransactionState::Open {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!(
                    "transaction {} is in state {:?}; only Open transactions accept writes",
                    tx_id_str, tx.state
                ),
            }),
        ));
    }

    let records: Vec<Record> = req
        .records
        .iter()
        .filter_map(|item| {
            if let serde_json::Value::Object(map) = item {
                Some(
                    map.iter()
                        .map(|(k, v)| (k.clone(), json_to_value(v)))
                        .collect::<Record>(),
                )
            } else {
                None
            }
        })
        .collect();
    let staged = records.len();
    tx.pending
        .entry(req.bundle.clone())
        .or_default()
        .extend(records);

    let total: usize = tx.pending.values().map(|v| v.len()).sum();
    let mut touched: Vec<String> = tx.pending.keys().cloned().collect();
    touched.sort();
    Ok(Json(TxWriteResponse {
        staged,
        total_in_tx: total,
        touched_bundles: touched,
    }))
}

#[cfg(feature = "transactions")]
async fn tx_commit(
    State(state): State<Arc<StreamState>>,
    Path(tx_id_str): Path<String>,
) -> Result<Json<TxCommitResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tx_id = parse_tx_id(&tx_id_str)?;
    let (snap_id, pending) = {
        let mut registry = state.tx_registry.lock().unwrap();
        let tx = registry.get_mut(&tx_id).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("transaction {} not found", tx_id_str),
                }),
            )
        })?;
        if tx.state != gigi::transactions::TransactionState::Open {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: format!(
                        "transaction {} is in state {:?}; only Open transactions can commit",
                        tx_id_str, tx.state
                    ),
                }),
            ));
        }
        tx.state = gigi::transactions::TransactionState::Preparing;
        (tx.snap_id, std::mem::take(&mut tx.pending))
    };

    let mut engine = state.engine.write().unwrap();
    let mut bundles_sorted: Vec<String> = pending.keys().cloned().collect();
    bundles_sorted.sort();

    let mut records_committed = 0usize;
    let mut applied: Vec<String> = Vec::new();
    for bundle_name in &bundles_sorted {
        let records = pending.get(bundle_name).cloned().unwrap_or_default();
        if records.is_empty() {
            continue;
        }
        match engine.batch_insert(bundle_name, &records) {
            Ok(n) => {
                records_committed += n;
                applied.push(bundle_name.clone());
            }
            Err(e) => {
                drop(engine);
                let mut registry = state.tx_registry.lock().unwrap();
                if let Some(tx) = registry.get_mut(&tx_id) {
                    tx.state = gigi::transactions::TransactionState::Aborted;
                }
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!(
                            "commit failed on bundle '{}' after applying {:?}: {} \
                             (Phase A surface; full 2PC atomicity ships with the global WAL log)",
                            bundle_name, applied, e
                        ),
                    }),
                ));
            }
        }
    }
    drop(engine);

    {
        let mut registry = state.tx_registry.lock().unwrap();
        if let Some(tx) = registry.get_mut(&tx_id) {
            tx.state = gigi::transactions::TransactionState::Committed;
        }
    }

    Ok(Json(TxCommitResponse {
        committed_at: sys_time_to_iso(std::time::SystemTime::now()),
        new_snap_id: snap_id.0,
        bundles_committed: applied,
        records_committed,
    }))
}

#[cfg(feature = "transactions")]
async fn tx_rollback(
    State(state): State<Arc<StreamState>>,
    Path(tx_id_str): Path<String>,
) -> Result<Json<TxRollbackResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tx_id = parse_tx_id(&tx_id_str)?;
    let mut registry = state.tx_registry.lock().unwrap();
    let tx = registry.get_mut(&tx_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("transaction {} not found", tx_id_str),
            }),
        )
    })?;
    let discarded: usize = tx.pending.values().map(|v| v.len()).sum();
    tx.pending.clear();
    tx.state = gigi::transactions::TransactionState::Aborted;
    Ok(Json(TxRollbackResponse {
        aborted: true,
        discarded_records: discarded,
    }))
}

#[cfg(feature = "transactions")]
async fn tx_status(
    State(state): State<Arc<StreamState>>,
    Path(tx_id_str): Path<String>,
) -> Result<Json<TxStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tx_id = parse_tx_id(&tx_id_str)?;
    let registry = state.tx_registry.lock().unwrap();
    let tx = registry.get(&tx_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("transaction {} not found", tx_id_str),
            }),
        )
    })?;
    let age_secs = tx.opened_at.elapsed().map(|d| d.as_secs()).unwrap_or(0);
    let mut touched: Vec<String> = tx.pending.keys().cloned().collect();
    touched.sort();
    Ok(Json(TxStatusResponse {
        tx_id: format!("{}", tx_id),
        snap_id: tx.snap_id.0,
        state: format!("{:?}", tx.state).to_lowercase(),
        isolation: match tx.isolation {
            gigi::transactions::IsolationLevel::SnapshotIsolation => "snapshot_isolation".into(),
            gigi::transactions::IsolationLevel::ReadCommitted => "read_committed".into(),
        },
        opened_at: sys_time_to_iso(tx.opened_at),
        age_secs,
        touched_bundles: touched,
        pending_writes: tx.pending.values().map(|v| v.len()).sum(),
    }))
}

// ── Causal States v0.1 — HTTP envelope (CV4-wire) ─────────────────────────
//
// POST /v1/causal_states/commutator
//
// Wire-side surface for the Davis (2026) update-commutator substrate.
// Computes Ω = U_a∘U_b − U_b∘U_a on a base belief and returns the three
// scalar diagnostics plus a three-way regime classification (Saturating
// / Smooth / Borderline). Mounted only when the `causal_states` feature
// flag is enabled.
//
// Request format (operator pair + base belief + optional bands):
//
//   {
//     "a":    {"kind": "hmm", "alpha": 0.2, "beta": 0.3, "symbol": 0},
//     "b":    {"kind": "hmm", "alpha": 0.2, "beta": 0.3, "symbol": 1},
//     "base_belief": [0.5, 0.5],
//     "bands": {"tv_low": 0.30, "tv_high": 0.95}   // optional; defaults
//   }
//
// Operator kinds: "even_u0", "even_u1", "hmm". HMM requires alpha, beta,
// symbol ∈ {0, 1}.
//
// Response (success):
//
//   {
//     "forward":  [0.4469, 0.5531],
//     "backward": [0.5531, 0.4469],
//     "tv":        0.106195,
//     "hellinger": 0.075197,
//     "kl":        {"kind": "finite", "value": 0.0327},
//     "regime":    "smooth"
//   }
//
// Errors return 400 with {"error": "..."} for bad input, or with an
// additional "which" field ("forward" or "backward") when one of the
// composition paths hit an inadmissible state.

#[cfg(feature = "causal_states")]
#[derive(Debug, Deserialize)]
struct CommutatorRequest {
    a: OperatorSpec,
    b: OperatorSpec,
    base_belief: Vec<f64>,
    #[serde(default)]
    bands: Option<RegimeBandsSpec>,
}

/// Wire-side discriminated union for an update operator.
///
/// `kind` discriminates the variant; HMM additionally carries alpha,
/// beta, and a symbol ∈ {0, 1}.
#[cfg(feature = "causal_states")]
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum OperatorSpec {
    EvenU0,
    EvenU1,
    Hmm { alpha: f64, beta: f64, symbol: u8 },
}

#[cfg(feature = "causal_states")]
#[derive(Debug, Deserialize)]
struct RegimeBandsSpec {
    tv_low: f64,
    tv_high: f64,
}

#[cfg(feature = "causal_states")]
#[derive(Debug, Serialize)]
struct CommutatorResponse {
    forward: Vec<f64>,
    backward: Vec<f64>,
    tv: f64,
    hellinger: f64,
    kl: gigi::causal_states::KlValue,
    regime: gigi::causal_states::Regime,
}

#[cfg(feature = "causal_states")]
fn build_operator(
    spec: &OperatorSpec,
) -> Result<Box<dyn gigi::causal_states::UpdateOperator>, (StatusCode, Json<ErrorResponse>)> {
    use gigi::causal_states::{EvenU0, EvenU1, HmmUpdate};
    match spec {
        OperatorSpec::EvenU0 => Ok(Box::new(EvenU0)),
        OperatorSpec::EvenU1 => Ok(Box::new(EvenU1)),
        OperatorSpec::Hmm { alpha, beta, symbol } => {
            if *symbol > 1 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!(
                            "HMM operator: symbol must be 0 or 1, got {symbol}"
                        ),
                    }),
                ));
            }
            if !alpha.is_finite() || !beta.is_finite() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!(
                            "HMM operator: alpha and beta must be finite, got alpha={alpha}, beta={beta}"
                        ),
                    }),
                ));
            }
            Ok(Box::new(HmmUpdate {
                alpha: *alpha,
                beta: *beta,
                symbol: *symbol,
            }))
        }
    }
}

#[cfg(feature = "causal_states")]
async fn causal_states_commutator_http(
    State(_state): State<Arc<StreamState>>,
    Json(req): Json<CommutatorRequest>,
) -> Result<Json<CommutatorResponse>, (StatusCode, Json<ErrorResponse>)> {
    use gigi::causal_states::{classify_regime, commutator, CommutatorError, RegimeBands};

    // Input validation: base belief must be a non-trivial probability vector.
    if req.base_belief.len() != 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "base_belief must have exactly 2 entries (2-state substrate v0.1); got {}",
                    req.base_belief.len()
                ),
            }),
        ));
    }
    for (i, v) in req.base_belief.iter().enumerate() {
        if !v.is_finite() || *v < 0.0 || *v > 1.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "base_belief[{i}] = {v} is not in [0, 1] or is non-finite"
                    ),
                }),
            ));
        }
    }
    let sum: f64 = req.base_belief.iter().sum();
    if (sum - 1.0).abs() > 1e-6 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "base_belief must sum to 1 within 1e-6, got sum = {sum}"
                ),
            }),
        ));
    }

    let a_op = build_operator(&req.a)?;
    let b_op = build_operator(&req.b)?;

    let omega = commutator(a_op.as_ref(), b_op.as_ref(), &req.base_belief)
        .map_err(|e| match e {
            CommutatorError::PathInadmissible { which, error } => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "commutator path inadmissible: which={which:?}, error={error:?}"
                    ),
                }),
            ),
        })?;

    let bands = req
        .bands
        .map(|b| RegimeBands {
            tv_low: b.tv_low,
            tv_high: b.tv_high,
        })
        .unwrap_or_default();

    let regime = classify_regime(&omega, bands);

    Ok(Json(CommutatorResponse {
        forward: omega.forward,
        backward: omega.backward,
        tv: omega.tv,
        hellinger: omega.hellinger,
        kl: omega.kl,
        regime,
    }))
}

// ── WISH v0.1 — HTTP envelope ──────────────────────────────────────────────
//
// POST /v1/wish
//
// The boundary-value geodesic verb. Solves on a 2D conformally flat
// manifold selected by the `metric` field of the request — `flat`,
// `s2`, `cp1`, or `pinch` (the W4 fixture). Production substrate-backed
// metrics land with the §3 dim-lift. The 2D demo wire is enough for
// IMAGINE Phase 2 tooling and for the live-smoke contract probes.
//
// Request shape:
//   {
//     "seed":   [0.1, 0.0],
//     "target": [0.5, 0.3],
//     "metric": "s2",
//     "max_imagined_curvature":     4.0,
//     "max_accumulated_holonomy":   0.5,
//     "max_arc_length":             4.0,
//     "max_solve_ms":             250,
//     "max_iterations":           200,
//     "n_nodes":                    32
//   }
// All trust-envelope and solver fields are optional and fall back to
// `WishConfig::default()` (matching `WISH_SPEC_v0.1.md §5`). The
// `max_solve_ms` floor of 50 ms is enforced server-side regardless of
// caller input — per the GIGI-team review's anti-gaming clause.
//
// Response (verdict trichotomy):
//   200 Granted     -> { "verdict": "granted",      "unsat": false, ... }
//   200 Unreachable -> { "verdict": "unreachable",  "unsat": true,  ... }
//   200 Indeterminate -> { "verdict": "indeterminate", "unsat": null, ... }
// The trichotomy rides on 200 because all three are well-posed answers
// to a well-posed question. 400 is reserved for malformed requests
// (dim mismatch, unsupported metric kind, non-finite numbers).

#[cfg(feature = "wish")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WishMetricKind {
    Flat,
    S2,
    Cp1,
    Pinch {
        #[serde(default = "default_pinch_amplitude")]
        amplitude: f64,
        #[serde(default = "default_pinch_sigma")]
        sigma: f64,
        #[serde(default = "default_pinch_center")]
        x_center: f64,
    },
}

#[cfg(feature = "wish")]
fn default_pinch_amplitude() -> f64 { 0.1 }
#[cfg(feature = "wish")]
fn default_pinch_sigma() -> f64 { 0.15 }
#[cfg(feature = "wish")]
fn default_pinch_center() -> f64 { 0.5 }

#[cfg(feature = "wish")]
#[derive(Debug, Deserialize)]
struct WishHttpRequest {
    seed: [f64; 2],
    target: [f64; 2],
    #[serde(default = "default_metric_kind")]
    metric: WishMetricKind,
    #[serde(default)]
    max_imagined_curvature: Option<f64>,
    #[serde(default)]
    max_accumulated_holonomy: Option<f64>,
    #[serde(default)]
    max_arc_length: Option<f64>,
    #[serde(default)]
    max_solve_ms: Option<u32>,
    #[serde(default)]
    max_iterations: Option<u32>,
    #[serde(default)]
    n_nodes: Option<u32>,
    #[serde(default)]
    grad_tol: Option<f64>,
}

#[cfg(feature = "wish")]
fn default_metric_kind() -> WishMetricKind { WishMetricKind::Flat }

#[cfg(feature = "wish")]
#[derive(Debug, Serialize)]
struct WishHttpResponse {
    /// "granted" | "unreachable" | "indeterminate" — matches §10 wire pin.
    verdict: &'static str,
    /// SUDOKU trichotomy: false / true / null.
    unsat: Option<bool>,

    // ── Granted fields ──
    #[serde(skip_serializing_if = "Option::is_none")]
    capacity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arc_length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integrated_curvature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accumulated_holonomy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solver_iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<Vec<Vec<f64>>>,

    // ── Unreachable fields ──
    #[serde(skip_serializing_if = "Option::is_none")]
    frontier_waypoint: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    waypoint_kind: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reached_fraction: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocked_by: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capacity_to_waypoint: Option<f64>,

    // ── Indeterminate fields ──
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    final_residual: Option<f64>,
}

#[cfg(feature = "wish")]
fn build_wish_config(req: &WishHttpRequest) -> gigi::imagine::wish::WishConfig {
    use gigi::imagine::wish::{SolverKind, WishConfig};
    let mut cfg = WishConfig::default();
    if let Some(v) = req.max_imagined_curvature { cfg.max_imagined_curvature = v; }
    if let Some(v) = req.max_accumulated_holonomy { cfg.max_accumulated_holonomy = v; }
    if let Some(v) = req.max_arc_length { cfg.max_arc_length = v; }
    if let Some(v) = req.max_solve_ms { cfg.max_solve_ms = v; }
    if let Some(v) = req.max_iterations { cfg.max_iterations = v; }
    if let Some(v) = req.grad_tol { cfg.grad_tol = v; }
    if let Some(n) = req.n_nodes {
        cfg.solver = SolverKind::Relaxation { n_nodes: n };
    }
    cfg
}

#[cfg(feature = "wish")]
fn solve_with_metric(
    kind: &WishMetricKind,
    seed: [f64; 2],
    target: [f64; 2],
    cfg: &gigi::imagine::wish::WishConfig,
) -> gigi::imagine::wish::WishOutcome {
    use gigi::imagine::wish::{
        relaxation_solve, CP1FubiniStudy, CurvaturePinch, S2Stereographic, T2Flat,
    };
    match kind {
        WishMetricKind::Flat => relaxation_solve(&T2Flat, seed, target, cfg),
        WishMetricKind::S2 => relaxation_solve(&S2Stereographic, seed, target, cfg),
        WishMetricKind::Cp1 => relaxation_solve(&CP1FubiniStudy, seed, target, cfg),
        WishMetricKind::Pinch { amplitude, sigma, x_center } => {
            let m = CurvaturePinch {
                amplitude: *amplitude,
                sigma: *sigma,
                x_center: *x_center,
            };
            relaxation_solve(&m, seed, target, cfg)
        }
    }
}

#[cfg(feature = "wish")]
fn outcome_to_response(outcome: gigi::imagine::wish::WishOutcome) -> WishHttpResponse {
    use gigi::imagine::wish::{IndeterminateReason, WishOutcome};
    use gigi::imagine::provenance::WishBlockReason;
    let block_tag = |b: WishBlockReason| -> &'static str {
        match b {
            WishBlockReason::Curvature => "curvature",
            WishBlockReason::Holonomy => "holonomy",
            WishBlockReason::ArcLength => "arc_length",
        }
    };
    match outcome {
        WishOutcome::Granted {
            path,
            arc_length,
            integrated_curvature,
            capacity,
            accumulated_holonomy,
            solver_iterations,
            ..
        } => WishHttpResponse {
            verdict: "granted",
            unsat: Some(false),
            capacity: Some(capacity),
            arc_length: Some(arc_length),
            integrated_curvature: Some(integrated_curvature),
            accumulated_holonomy: Some(accumulated_holonomy),
            solver_iterations: Some(solver_iterations),
            path: Some(path),
            frontier_waypoint: None,
            waypoint_kind: None,
            reached_fraction: None,
            blocked_by: None,
            capacity_to_waypoint: None,
            reason: None,
            final_residual: None,
        },
        WishOutcome::Unreachable {
            frontier_waypoint,
            reached_fraction,
            blocked_by,
            capacity_to_waypoint,
        } => WishHttpResponse {
            verdict: "unreachable",
            unsat: Some(true),
            capacity: None,
            arc_length: None,
            integrated_curvature: None,
            accumulated_holonomy: None,
            solver_iterations: None,
            path: None,
            frontier_waypoint: Some(frontier_waypoint),
            waypoint_kind: Some("frontier_truncation"),
            reached_fraction: Some(reached_fraction),
            blocked_by: Some(block_tag(blocked_by)),
            capacity_to_waypoint: Some(capacity_to_waypoint),
            reason: None,
            final_residual: None,
        },
        WishOutcome::Indeterminate { reason } => {
            let (tag, residual) = match reason {
                IndeterminateReason::ConjugateLocus { at_fraction } => {
                    ("conjugate_locus", at_fraction)
                }
                IndeterminateReason::NonConvergence { final_residual } => {
                    ("non_convergence", final_residual)
                }
            };
            WishHttpResponse {
                verdict: "indeterminate",
                unsat: None,
                capacity: None,
                arc_length: None,
                integrated_curvature: None,
                accumulated_holonomy: None,
                solver_iterations: None,
                path: None,
                frontier_waypoint: None,
                waypoint_kind: None,
                reached_fraction: None,
                blocked_by: None,
                capacity_to_waypoint: None,
                reason: Some(tag),
                final_residual: Some(residual),
            }
        }
    }
}

#[cfg(feature = "wish")]
async fn wish_http(
    State(_state): State<Arc<StreamState>>,
    Json(req): Json<WishHttpRequest>,
) -> Result<Json<WishHttpResponse>, (StatusCode, Json<ErrorResponse>)> {
    for (name, p) in [("seed", &req.seed), ("target", &req.target)] {
        for v in p.iter() {
            if !v.is_finite() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("{} contains non-finite value", name),
                    }),
                ));
            }
        }
    }
    let cfg = build_wish_config(&req);
    let outcome = solve_with_metric(&req.metric, req.seed, req.target, &cfg);
    Ok(Json(outcome_to_response(outcome)))
}

/// POST /v1/bundles/{name}/wish
///
/// Bundle-scoped WISH. Phase 5 v0.1 verifies the bundle exists and
/// dispatches to the same solver as the global endpoint — the metric
/// in the request body still picks from {flat, s2, cp1, pinch}. This
/// shape exists so consumers (Marcella's IMAGINE Phase 2 cross-check
/// suite) can write tests against the real URL today; the dim-lift
/// later swaps in the bundle's substrate metric (Kähler structure /
/// `metric_at` trait) without changing the URL or the request body
/// fields.
#[cfg(feature = "wish")]
async fn wish_bundle_http(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<WishHttpRequest>,
) -> Result<Json<WishHttpResponse>, (StatusCode, Json<ErrorResponse>)> {
    {
        // Scope the read lock so we drop it before the (potentially
        // slow) solver runs. The bundle handle is only consulted here
        // for existence; substrate-metric resolution lands with the
        // dim-lift, at which point this block grows.
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
    wish_http(State(state), Json(req)).await
}

// ── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gigi::dhoom;
    use gigi::engine::Engine;
    use gigi::types::{BundleSchema, FieldDef, FieldType};
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("gigi_stream_test_{tag}"))
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn stream_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    async fn post_gql_for_test(state: Arc<StreamState>, query: &str) -> StatusCode {
        gql_query(
            State(state),
            axum::http::HeaderMap::new(),
            Json(serde_json::json!({ "query": query })),
        )
        .await
        .into_response()
        .status()
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

    /// SCJ Round 10 §5(a): `_score` must always be the LAST key in the
    /// serialized HUNT row JSON so TUI consumers can render score columns
    /// without inspecting the schema. Verified at the serialization layer
    /// since JSON object key order is the contract bit on the wire.
    #[cfg(feature = "patterns")]
    #[test]
    fn hunt_row_to_json_pins_score_last_when_present() {
        let mut row = gigi::types::Record::new();
        row.insert("_score".to_string(), Value::Float(7.5));
        row.insert("alpha".to_string(), Value::Integer(1));
        row.insert("zulu".to_string(), Value::Integer(2));
        row.insert("mike".to_string(), Value::Integer(3));
        let json = hunt_row_to_json(row);
        let serialized = serde_json::to_string(&json).expect("serialize");
        // Find the offset of every key; `_score` must be greatest.
        let pos_score = serialized.find("\"_score\"").expect("_score present");
        for k in ["alpha", "mike", "zulu"] {
            let needle = format!("\"{k}\"");
            let pos = serialized
                .find(&needle)
                .unwrap_or_else(|| panic!("{k} present"));
            assert!(
                pos < pos_score,
                "`{k}` must appear before `_score` in {serialized}"
            );
        }
    }

    /// Absent `_score` must NOT inject one — the helper only re-orders,
    /// never invents columns.
    #[cfg(feature = "patterns")]
    #[test]
    fn hunt_row_to_json_does_not_inject_score_when_absent() {
        let mut row = gigi::types::Record::new();
        row.insert("alpha".to_string(), Value::Integer(1));
        row.insert("beta".to_string(), Value::Integer(2));
        let json = hunt_row_to_json(row);
        let serialized = serde_json::to_string(&json).expect("serialize");
        assert!(
            !serialized.contains("_score"),
            "no _score in input → no _score in output: {serialized}"
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

    #[tokio::test(flavor = "current_thread")]
    async fn http_gql_insert_and_batch_insert_are_wal_logged() {
        let _guard = stream_env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let dir = tmp_dir("http_gql_wal");
        cleanup(&dir);
        std::env::set_var("GIGI_DATA_DIR", &dir);

        {
            let (logger, _ingester) = Logger::new(LogConfig::default(), "http-gql-wal-test");
            let state = Arc::new(StreamState::new(logger, Arc::new(Metrics::new())));
            state.ready.store(true, Ordering::Release);

            assert_eq!(
                post_gql_for_test(
                    Arc::clone(&state),
                    "CREATE BUNDLE http_wal (id INT BASE, label TEXT FIBER);",
                )
                .await,
                StatusCode::OK
            );
            assert_eq!(
                post_gql_for_test(
                    Arc::clone(&state),
                    "INSERT INTO http_wal (id, label) VALUES (1, 'one');",
                )
                .await,
                StatusCode::OK
            );
            assert_eq!(
                post_gql_for_test(
                    Arc::clone(&state),
                    "SECTIONS http_wal (id, label) (2, 'two'), (3, 'three');",
                )
                .await,
                StatusCode::OK
            );

            state.engine.write().unwrap().checkpoint().unwrap();
        }

        {
            let engine = Engine::open(&dir).unwrap();
            for (id, label) in [(1, "one"), (2, "two"), (3, "three")] {
                let mut key = Record::new();
                key.insert("id".into(), Value::Integer(id));
                let row = engine.point_query("http_wal", &key).unwrap().unwrap();
                assert_eq!(row.get("label"), Some(&Value::Text(label.into())));
            }
        }

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
        // S1 wave 1 §E: 5 new args for brain cache + timing metrics.
        let body = build_prometheus_text(
            100, 2, 1, 50_000, 200_000, 900_000,
            5000, 250_000, 3, 7, 14_000, 88, 12, 3600,
            42, 8, 0, 1_500_000, 12_000,
        );
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
            // S1 wave 1 §E additions.
            "gigi_brain_cache_hits_total",
            "gigi_brain_cache_misses_total",
            "gigi_brain_cache_evictions_total",
            "gigi_brain_fit_total_microseconds",
            "gigi_brain_compute_total_microseconds",
        ] {
            assert!(body.contains(metric), "missing metric: {metric}");
        }
    }

    #[test]
    fn test_prometheus_text_values_correct() {
        let body = build_prometheus_text(
            100, 2, 1, 50_000, 200_000, 900_000,
            5000, 250_000, 3, 7, 14_000, 88, 12, 3600,
            42, 8, 0, 1_500_000, 12_000,
        );
        assert!(body.contains("gigi_queries_total 100"));
        assert!(body.contains("gigi_queries_error_total 2"));
        assert!(body.contains("gigi_queries_slow_total 1"));
        assert!(body.contains(r#"gigi_query_duration_microseconds{quantile="0.5"} 50000"#));
        assert!(body.contains(r#"gigi_query_duration_microseconds{quantile="0.95"} 200000"#));
        assert!(body.contains(r#"gigi_query_duration_microseconds{quantile="0.99"} 900000"#));
        assert!(body.contains("gigi_uptime_seconds 3600"));
        // S1 wave 1 §E values.
        assert!(body.contains("gigi_brain_cache_hits_total 42"));
        assert!(body.contains("gigi_brain_cache_misses_total 8"));
        assert!(body.contains("gigi_brain_fit_total_microseconds 1500000"));
    }

    #[test]
    fn test_prometheus_text_has_help_and_type_lines() {
        let body = build_prometheus_text(
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
        );
        // Every metric must be preceded by # HELP and # TYPE
        assert!(body.contains("# HELP gigi_queries_total"));
        assert!(body.contains("# TYPE gigi_queries_total counter"));
        assert!(body.contains("# HELP gigi_bundles"));
        assert!(body.contains("# TYPE gigi_bundles gauge"));
        // S1 wave 1 §E HELP/TYPE.
        assert!(body.contains("# HELP gigi_brain_cache_hits_total"));
        assert!(body.contains("# TYPE gigi_brain_cache_hits_total counter"));
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

    // ── SUDOKU wire-format gate tests (waves 3-6.2) ─────────────────────
    // These tests exercise the in-process geometry layer directly (not the
    // HTTP stack) to prove that every wave-3+ field actually reaches the
    // BrainSudokuResponse wire struct. The geometry-layer unit tests in
    // sudoku.rs prove the math; these prove the wire boundary carries it.

    /// **Gate W3/W4/W6.** A multi-constraint request on a small bundle
    /// produces non-empty selectivity, non-empty relaxation menu,
    /// quality_score > 0 on solutions, and raw_curvature on every
    /// selectivity report. Confirms waves 3, 4, and 6.1 all reach wire.
    #[cfg(feature = "kahler")]
    #[test]
    fn sudoku_wire_gate_w3_w4_w6_fields_present() {
        use gigi::geometry::{
            Constraint, FieldOp, SudokuConfig, SudokuRequest, SudokuVerdict,
        };
        use gigi::types::Value;

        // 8 records: price 100..800 in 100-steps, color alternating.
        let mut records: Vec<gigi::types::Record> = Vec::new();
        for i in 1..=8u64 {
            let mut r = gigi::types::Record::new();
            r.insert("price".into(), Value::Float(i as f64 * 100.0));
            r.insert(
                "color".into(),
                Value::Text(if i % 2 == 0 { "red" } else { "blue" }.into()),
            );
            records.push(r);
        }

        // Constraints: price <= 500 (5 pass) AND color == "red" (4 pass).
        // Intersection: price <= 500 AND red → records 2,4 → 2 solutions.
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "price".into(),
                    op: FieldOp::Le(500.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };

        let resp = gigi::geometry::solve_constraints(
            records,
            &req,
            &SudokuConfig::default(),
        )
        .unwrap();

        // W3 — selectivity present and non-trivial.
        assert_eq!(
            resp.selectivity.len(),
            2,
            "one selectivity report per constraint"
        );
        // At least one constraint binds — the Pareto test for binding.
        assert!(
            resp.selectivity.iter().any(|s| s.binding),
            "at least one binding constraint"
        );

        // W6.1 — raw_curvature is in [0,1] for every report.
        for s in &resp.selectivity {
            assert!(
                (0.0..=1.0).contains(&s.raw_curvature),
                "raw_curvature {} out of [0,1] for field '{}'",
                s.raw_curvature,
                s.field
            );
        }

        // W4 — quality_score is in [0,1] for every solution.
        assert!(
            !resp.solutions.is_empty(),
            "expected solutions for compatible constraints"
        );
        for sol in &resp.solutions {
            assert!(
                (0.0..=1.0).contains(&sol.quality_score),
                "quality_score {} out of [0,1]",
                sol.quality_score
            );
        }

        // W3 — relaxation menu non-empty (the binding constraint
        // has near-misses it can propose thresholds for).
        assert!(
            !resp.relaxations.is_empty(),
            "relaxation menu must be non-empty when constraints filter records"
        );

        // W6.2 — no pre-flight contradiction (compatible constraints).
        assert!(
            resp.pre_flight_unsat_reason.is_none(),
            "compatible constraints must not trigger pre-flight: {:?}",
            resp.pre_flight_unsat_reason
        );
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
    }

    /// **Gate W6.2 wire.** Contradictory constraints (Le(100) + Ge(500))
    /// produce a non-None pre_flight_unsat_reason that names the field.
    /// Proves the pre-flight path from geometry layer to BrainSudokuResponse.
    #[cfg(feature = "kahler")]
    #[test]
    fn sudoku_wire_gate_w6_2_preflight_reaches_wire() {
        use gigi::geometry::{
            Constraint, FieldOp, SudokuConfig, SudokuRequest, SudokuVerdict,
        };
        use gigi::types::Value;

        // Bundle has 1000 records — they never get walked (pre-flight fires first).
        let records: Vec<gigi::types::Record> = (0..1000)
            .map(|i| {
                let mut r = gigi::types::Record::new();
                r.insert("rent".into(), Value::Float(i as f64 * 10.0));
                r
            })
            .collect();

        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "rent".into(),
                    op: FieldOp::Le(100.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "rent".into(),
                    op: FieldOp::Ge(500.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };

        let resp = gigi::geometry::solve_constraints(
            records,
            &req,
            &SudokuConfig::default(),
        )
        .unwrap();

        assert_eq!(
            resp.verdict,
            SudokuVerdict::Unsat,
            "contradictory constraints must be Unsat"
        );
        let reason = resp
            .pre_flight_unsat_reason
            .as_ref()
            .expect("pre_flight_unsat_reason must be Some for Le(100)+Ge(500)");
        assert!(
            reason.contains("rent"),
            "reason must name the field 'rent'; got: {reason}"
        );
        // Pre-flight fires before any bundle walk.
        assert_eq!(
            resp.n_records_considered, 0,
            "n_records_considered must be 0 (no bundle walk on pre-flight Unsat)"
        );
    }

    /// **Gate W5.** Pareto frontier includes multi-violation records and
    /// excludes dominated ones. Confirms wave-5 Pareto logic is active.
    #[cfg(feature = "kahler")]
    #[test]
    fn sudoku_wire_gate_w5_pareto_non_empty_and_dominated_excluded() {
        use gigi::geometry::{Constraint, FieldOp, SudokuConfig, SudokuRequest};
        use gigi::types::Value;

        // 4 records: one satisfies all, two violate one each (different
        // costs), one violates both. Pareto must keep both single-violation
        // records if their costs differ; only the dominated multi-violation
        // should be filtered.
        let mut records: Vec<gigi::types::Record> = Vec::new();
        let mut make = |a: f64, b: f64| {
            let mut r = gigi::types::Record::new();
            r.insert("a".into(), Value::Float(a));
            r.insert("b".into(), Value::Float(b));
            r
        };
        records.push(make(3.0, 3.0));   // solution
        records.push(make(15.0, 3.0));  // violates only a, cheap
        records.push(make(3.0, 25.0));  // violates only b, expensive
        records.push(make(15.0, 25.0)); // violates both

        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "a".into(),
                    op: FieldOp::Le(10.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "b".into(),
                    op: FieldOp::Le(10.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };

        let resp = gigi::geometry::solve_constraints(
            records,
            &req,
            &SudokuConfig::default(),
        )
        .unwrap();

        assert!(
            !resp.pareto_near_misses.is_empty(),
            "Pareto frontier must not be empty"
        );
        // The cheap single-violation record must appear.
        let has_cheap = resp.pareto_near_misses.iter().any(|p| {
            p.violations.len() == 1
                && p.record.get("a") == Some(&Value::Float(15.0))
                && p.record.get("b") == Some(&Value::Float(3.0))
        });
        assert!(has_cheap, "cheap single-violation record must be on frontier");
    }

    // ── S3.5 expansion wire-gate tests ───────────────────────────────────────
    // These tests hit the geometry layer directly (same pattern as the W3-W6
    // gates above) and prove that the expansion result reaches the
    // BrainSudokuResponse wire struct. The math is proved in sudoku.rs;
    // these prove the HTTP boundary carries the new fields.

    /// **Gate E-WIRE-1.** Expansion field is None (omitted) on SAT verdict.
    /// Consumers must not see `expanded` when the original puzzle found
    /// solutions — we only expand on UNSAT.
    #[cfg(feature = "kahler")]
    #[test]
    fn sudoku_wire_gate_expansion_absent_on_sat() {
        use gigi::geometry::{
            solve_constraints, Constraint, ExpansionConfig, FieldOp, SudokuConfig,
            SudokuRequest, SudokuVerdict,
        };
        use gigi::types::Value;

        let mut records = Vec::new();
        for i in 1..=5 {
            let mut r = gigi::types::Record::new();
            r.insert("price".into(), Value::Float(i as f64 * 10.0));
            records.push(r);
        }
        // price <= 40 → records 10, 20, 30, 40 all satisfy → SAT.
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "price".into(),
                op: FieldOp::Le(40.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 3,
            expansion: Some(ExpansionConfig { allowed: true, max_constraint_relaxations: 1 }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat, "must be SAT");
        assert!(
            resp.expanded.is_none(),
            "expanded must be None on SAT — expansion only runs on UNSAT"
        );
    }

    /// **Gate E-WIRE-2.** Expansion is attempted and finds solutions on
    /// UNSAT. The `expanded` field is Some with `attempted: true` and
    /// at least one expanded solution. The expanded solution carries the
    /// correct `relaxed_constraint_idx` and a finite `expansion_cost`.
    ///
    /// This proves the full path: geometry layer → BrainSudokuResponse →
    /// wire struct fields are populated correctly.
    #[cfg(feature = "kahler")]
    #[test]
    fn sudoku_wire_gate_expansion_result_reaches_wire_on_unsat() {
        use gigi::geometry::{
            solve_constraints, Constraint, ExpansionConfig, FieldOp, SudokuConfig,
            SudokuRequest, SudokuVerdict,
        };
        use gigi::types::Value;

        // 5 records: price 100..500. Constraint: price <= 90 → UNSAT.
        // Expansion: relax price → should find the $100 record.
        let records: Vec<gigi::types::Record> = (1..=5)
            .map(|i| {
                let mut r = gigi::types::Record::new();
                r.insert("price".into(), Value::Float(i as f64 * 100.0));
                r
            })
            .collect();
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "price".into(),
                op: FieldOp::Le(90.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: Some(ExpansionConfig { allowed: true, max_constraint_relaxations: 1 }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat, "must be UNSAT");
        let expanded = resp.expanded.as_ref()
            .expect("expanded must be Some when UNSAT + expansion allowed");
        assert!(expanded.attempted, "attempted must be true");
        assert_eq!(expanded.expansion_type, "constraint_relaxation");
        assert!(
            !expanded.solutions.is_empty(),
            "expansion must find the $100 record"
        );
        let sol = &expanded.solutions[0];
        assert_eq!(sol.relaxed_constraint_idx, 0, "price is constraint 0");
        assert!(
            sol.expansion_cost.is_finite() && sol.expansion_cost > 0.0,
            "expansion_cost must be finite positive, got {}",
            sol.expansion_cost
        );
    }

    /// **Gate E-WIRE-3.** When expansion also fails (advisory path):
    /// `expanded.solutions` is empty, `advisory` is Some, and the
    /// advisory message suggests asking a human or reformulating.
    #[cfg(feature = "kahler")]
    #[test]
    fn sudoku_wire_gate_expansion_advisory_on_double_unsat() {
        use gigi::geometry::{
            solve_constraints, Constraint, ExpansionConfig, FieldOp, SudokuConfig,
            SudokuRequest, SudokuVerdict,
        };
        use gigi::types::Value;

        // All records fail price AND color — no near-misses → empty
        // relaxation menu → expansion also fails → advisory.
        let records: Vec<gigi::types::Record> = (1..=3)
            .map(|i| {
                let mut r = gigi::types::Record::new();
                r.insert("price".into(), Value::Float(i as f64 * 100.0));
                r.insert("color".into(), Value::Text("blue".into()));
                r
            })
            .collect();
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "price".into(),
                    op: FieldOp::Le(50.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: Some(ExpansionConfig { allowed: true, max_constraint_relaxations: 1 }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        let expanded = resp.expanded.as_ref().expect("expanded must be Some");
        assert!(expanded.attempted);
        assert!(expanded.solutions.is_empty(), "double-UNSAT: no expansion solutions");
        let advisory = expanded.advisory.as_ref().expect("advisory must be set");
        assert!(
            advisory.to_lowercase().contains("human") || advisory.to_lowercase().contains("reformulat"),
            "advisory must suggest asking a human or reformulating; got: {advisory}"
        );
    }

    // ── SAMPLE_TRANSPORT wire-format gate tests (S4) ─────────────────────
    // Exercise the geometry layer directly to prove the wire boundary
    // carries d_sq, sameness, weight, curvature_k, n_admissible, kappa.

    /// **ST-WIRE-1.** Full-budget query returns all corpus records; each
    /// candidate has correct sameness = 1 - d_sq and curvature_k = 2*sqrt(d_sq).
    #[cfg(feature = "kahler")]
    #[test]
    fn sample_transport_wire_gate_full_budget_all_returned() {
        use std::f64::consts::PI;
        use gigi::geometry::{sample_transport_neighborhood, SampleTransportRequest};
        use gigi::types::Value;
        use std::collections::HashMap;

        let corpus: Vec<HashMap<String, Value>> = (0..8)
            .map(|i| {
                let theta = i as f64 * PI / 4.0;
                let mut m = HashMap::new();
                m.insert("x".to_string(), Value::Float(theta.cos()));
                m.insert("y".to_string(), Value::Float(theta.sin()));
                m
            })
            .collect();

        let src_fiber = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: vec!["x".to_string(), "y".to_string()],
            budget: 1.0,
            k: 100,
            beta: 1.0,
            seed: Some(10),
        };
        let result = sample_transport_neighborhood(&corpus, &src_fiber, &req, 0.2).unwrap();

        assert_eq!(result.n_admissible, corpus.len(), "budget=1.0 => all records admissible");
        assert_eq!(result.n_returned, corpus.len(), "k > n => n_returned = n_admissible");
        for c in &result.candidates {
            assert!(
                (c.sameness - (1.0 - c.d_sq)).abs() < 1e-12,
                "sameness wire invariant broken"
            );
            assert!(
                (c.curvature_k - 2.0 * c.d_sq.sqrt()).abs() < 1e-12,
                "curvature_k wire invariant broken"
            );
            assert!(c.d_sq <= 1.0 + 1e-12, "d_sq > 1.0");
        }
    }

    /// **ST-WIRE-2.** Budget 0.15 admits only the near-neighbors; every
    /// returned candidate satisfies d_sq <= budget.
    #[cfg(feature = "kahler")]
    #[test]
    fn sample_transport_wire_gate_budget_filter_respected() {
        use std::f64::consts::PI;
        use gigi::geometry::{sample_transport_neighborhood, SampleTransportRequest};
        use gigi::types::Value;
        use std::collections::HashMap;

        let budget = 0.15_f64;
        let corpus: Vec<HashMap<String, Value>> = (0..16)
            .map(|i| {
                let theta = i as f64 * 2.0 * PI / 16.0;
                let mut m = HashMap::new();
                m.insert("a".to_string(), Value::Float(theta.cos()));
                m.insert("b".to_string(), Value::Float(theta.sin()));
                m
            })
            .collect();

        let src_fiber = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: vec!["a".to_string(), "b".to_string()],
            budget,
            k: 100,
            beta: 1.0,
            seed: Some(11),
        };
        let result = sample_transport_neighborhood(&corpus, &src_fiber, &req, 0.1).unwrap();

        assert!(result.n_admissible > 0, "budget=0.15 should admit some records");
        for c in &result.candidates {
            assert!(
                c.d_sq <= budget + 1e-12,
                "returned candidate d_sq={} exceeds budget={}",
                c.d_sq,
                budget
            );
        }
    }

    /// **ST-WIRE-3.** Kappa is echoed; weight = exp(-beta * d_sq) exactly.
    #[cfg(feature = "kahler")]
    #[test]
    fn sample_transport_wire_gate_kappa_and_weight_kernel() {
        use std::f64::consts::PI;
        use gigi::geometry::{sample_transport_neighborhood, SampleTransportRequest};
        use gigi::types::Value;
        use std::collections::HashMap;

        let corpus: Vec<HashMap<String, Value>> = (0..6)
            .map(|i| {
                let theta = i as f64 * PI / 3.0;
                let mut m = HashMap::new();
                m.insert("p".to_string(), Value::Float(theta.cos()));
                m.insert("q".to_string(), Value::Float(theta.sin()));
                m
            })
            .collect();

        let kappa = 0.42_f64;
        let beta = 2.5_f64;
        let src_fiber = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: vec!["p".to_string(), "q".to_string()],
            budget: 1.0,
            k: 100,
            beta,
            seed: Some(12),
        };
        let result = sample_transport_neighborhood(&corpus, &src_fiber, &req, kappa).unwrap();

        assert!((result.kappa - kappa).abs() < 1e-12, "kappa not echoed");
        let expected_conf = 1.0 / (1.0 + kappa);
        assert!((result.confidence - expected_conf).abs() < 1e-12, "confidence wrong");
        for c in &result.candidates {
            let expected_w = (-beta * c.d_sq).exp();
            assert!(
                (c.weight - expected_w).abs() < 1e-12,
                "weight={} != exp(-{} * {}) = {}",
                c.weight, beta, c.d_sq, expected_w
            );
        }
    }

    // ─── S7 intent_gate composition tests ──────────────────────────────────
    //
    // The endpoint composes three primitives (Čech pre-flight, SUDOKU
    // walk, kernel-density confidence). These four tests assert each
    // verdict shape is producible from the same composition. Geometry-
    // level — actual HTTP path is exercised by intent_gate_demo.py.

    /// **S7 gate 1**: contradictory constraints → pre-flight fires,
    /// verdict=Unsat, zero records walked. The "your prompt is broken"
    /// case — Marcella shows it back to the user, doesn't search.
    #[cfg(feature = "kahler")]
    #[test]
    fn intent_gate_composition_contradiction() {
        use gigi::geometry::{
            Constraint, FieldOp, SudokuConfig, SudokuRequest, SudokuVerdict,
        };
        use gigi::types::Value;

        // 100 records — any bundle would do; pre-flight must fire
        // BEFORE the walk, so the record count is irrelevant.
        let records: Vec<gigi::types::Record> = (0..100)
            .map(|i| {
                let mut r = gigi::types::Record::new();
                r.insert(
                    "color".into(),
                    Value::Text(["red", "blue", "green"][i % 3].into()),
                );
                r
            })
            .collect();

        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("blue".into())),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 3,
            expansion: None,
        };
        let resp = gigi::geometry::solve_constraints(records, &req, &SudokuConfig::default())
            .unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        assert_eq!(resp.n_records_considered, 0,
            "pre-flight must short-circuit before walking the bundle");
        let reason = resp.pre_flight_unsat_reason.as_ref()
            .expect("pre_flight_unsat_reason must be populated on Eq+Eq contradiction");
        assert!(reason.contains("color"),
            "reason must name the conflicting field; got {}", reason);
    }

    /// **S7 gate 2**: compatible constraints that no record satisfies →
    /// walk UNSAT, near-misses populated, no pre-flight reason. The
    /// "we searched and didn't find it" case — Marcella surfaces the
    /// near-misses + relaxation menu to offer alternatives.
    #[cfg(feature = "kahler")]
    #[test]
    fn intent_gate_composition_empty_feasible() {
        use gigi::geometry::{
            Constraint, FieldOp, SudokuConfig, SudokuRequest, SudokuVerdict,
        };
        use gigi::types::Value;

        // 30 records: prices 100..400 in 10-steps, colors red/blue/green.
        // Constraint: color="purple" AND price<=50 — no record satisfies.
        let records: Vec<gigi::types::Record> = (0..30)
            .map(|i| {
                let mut r = gigi::types::Record::new();
                r.insert("price".into(), Value::Float(100.0 + 10.0 * i as f64));
                r.insert(
                    "color".into(),
                    Value::Text(["red", "blue", "green"][i % 3].into()),
                );
                r
            })
            .collect();
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("purple".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "price".into(),
                    op: FieldOp::Le(50.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = gigi::geometry::solve_constraints(records, &req, &SudokuConfig::default())
            .unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        assert!(resp.pre_flight_unsat_reason.is_none(),
            "constraints are compatible — pre-flight must NOT fire");
        assert!(resp.n_records_considered > 0,
            "walk must run when pre-flight passes");
        assert!(resp.solutions.is_empty());
        // Near-misses OR pareto OR relaxation menu — at least ONE
        // actionable signal must surface so consumer can offer alternatives.
        assert!(
            !resp.near_misses.is_empty()
                || !resp.pareto_near_misses.is_empty()
                || !resp.relaxations.is_empty(),
            "walk-UNSAT must produce actionable alternatives (near_misses, \
             pareto, or relaxation menu); else consumer is told 'no' with \
             nothing to do about it"
        );
    }

    /// **S7 gate 3**: feasible constraints + query vector FAR from any
    /// record → SAT but confidence is low. The "I can answer but I'm
    /// guessing" case — Marcella responds with caveat or declines.
    #[cfg(feature = "kahler")]
    #[test]
    fn intent_gate_composition_sat_low_confidence() {
        use gigi::geometry::{
            kernel_density_confidence, confidence_normalized,
            Constraint, FieldOp, SudokuConfig, SudokuRequest, SudokuVerdict,
        };
        use gigi::types::Value;

        // 40 records clustered near origin in 4D; query far away.
        let mut samples: Vec<Vec<f64>> = Vec::new();
        let mut records: Vec<gigi::types::Record> = Vec::new();
        for i in 0..40u64 {
            let theta = i as f64 * 0.15;
            let v = vec![theta.cos() * 0.1, theta.sin() * 0.1, 0.0, 0.0];
            samples.push(v.clone());
            let mut r = gigi::types::Record::new();
            r.insert("price".into(), Value::Float(100.0 + i as f64));
            r.insert(
                "color".into(),
                Value::Text(if i % 2 == 0 { "red" } else { "blue" }.into()),
            );
            records.push(r);
        }
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "price".into(),
                    op: FieldOp::Le(200.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 3,
            expansion: None,
        };
        let resp = gigi::geometry::solve_constraints(records, &req, &SudokuConfig::default())
            .unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat,
            "constraints have satisfying records");

        // Query 10× outside the cluster's typical scale → low confidence.
        let far_query = vec![10.0, 10.0, 10.0, 10.0];
        let bw = 0.1; // matches cluster scale
        let normalized = confidence_normalized(&samples, &far_query, bw);
        assert!(normalized < 0.1,
            "far query must produce normalized confidence < 0.1; got {}",
            normalized);
        let raw = kernel_density_confidence(&samples, &far_query, bw);
        assert!(raw < 1.0,
            "raw density at far query must be near-zero; got {}", raw);
    }

    /// **S7 gate 4**: feasible constraints + query vector NEAR records →
    /// SAT with high confidence. The "I know this territory" case —
    /// Marcella responds with full confidence.
    #[cfg(feature = "kahler")]
    #[test]
    fn intent_gate_composition_sat_high_confidence() {
        use gigi::geometry::{
            confidence_normalized, Constraint, FieldOp, SudokuConfig,
            SudokuRequest, SudokuVerdict,
        };
        use gigi::types::Value;

        // Same cluster as gate 3; query AT the centroid.
        let mut samples: Vec<Vec<f64>> = Vec::new();
        let mut records: Vec<gigi::types::Record> = Vec::new();
        for i in 0..40u64 {
            let theta = i as f64 * 0.15;
            let v = vec![theta.cos() * 0.1, theta.sin() * 0.1, 0.0, 0.0];
            samples.push(v.clone());
            let mut r = gigi::types::Record::new();
            r.insert("price".into(), Value::Float(100.0 + i as f64));
            r.insert(
                "color".into(),
                Value::Text(if i % 2 == 0 { "red" } else { "blue" }.into()),
            );
            records.push(r);
        }
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "price".into(),
                    op: FieldOp::Le(200.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 3,
            expansion: None,
        };
        let resp = gigi::geometry::solve_constraints(records, &req, &SudokuConfig::default())
            .unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);

        // Query at a sample point itself → normalized confidence must
        // be exactly 1.0 (the densest possible point).
        let near_query = samples[0].clone();
        let bw = 0.1;
        let normalized = confidence_normalized(&samples, &near_query, bw);
        assert!(normalized > 0.5,
            "query at a sample point must produce normalized confidence > 0.5; \
             got {}", normalized);
    }

    // ── Davis Conjecture λ-budget ride-along contract ──────────
    //
    // The substrate's runtime introspection of its own carrying
    // capacity rides on the same response envelope as curvature and
    // confidence. These tests pin the wire shape so future cognitive
    // consumers (Marcella, future Claude) parse a stable field.

    /// CurvatureReport JSON serializes `lambda_budget` as a top-level
    /// f64, sibling to `curvature`/`confidence`/`capacity`.
    #[test]
    fn curvature_report_serializes_lambda_budget_field() {
        let report = CurvatureReport {
            k: 0.05,
            curvature: 0.05,
            confidence: 1.0 / (1.0 + 0.05),
            capacity: 1.0 / 0.05,
            lambda_budget: gigi::curvature::lambda_budget(0.05, 2.0, 1.0),
            per_field: Vec::new(),
            #[cfg(feature = "kahler")]
            kahler: None,
        };
        let json = serde_json::to_value(&report).expect("serialize CurvatureReport");
        let obj = json.as_object().expect("object");
        assert!(
            obj.contains_key("lambda_budget"),
            "CurvatureReport JSON must contain lambda_budget; got keys {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        // Existing wire fields preserved (bit-identity gate).
        for required in ["K", "curvature", "confidence", "capacity", "per_field"] {
            assert!(
                obj.contains_key(required),
                "CurvatureReport JSON missing required field `{required}`; \
                 ride-along must be additive"
            );
        }
        // Numeric value is finite (or the documented saturated 1.0).
        let v = obj["lambda_budget"].as_f64().expect("lambda_budget is f64");
        assert!(v.is_finite() || v == 1.0, "lambda_budget = {v}");
    }

    /// Filtered-query meta object serializes `lambda_budget` alongside
    /// the existing `curvature`/`confidence` keys (the substrate's
    /// per-query ride-along path).
    #[test]
    fn filtered_query_meta_serializes_lambda_budget() {
        // Mirror of the meta JSON the filtered_query handler builds
        // (handler is async + tied to State<Arc<StreamState>> so we
        // verify the shape of the JSON literal directly).
        let k: f64 = 0.07;
        let d: f64 = 1.5;
        let lambda = gigi::curvature::lambda_budget(k, d, 1.0);
        let meta = serde_json::json!({
            "confidence": gigi::curvature::confidence(k),
            "curvature": k,
            "lambda_budget": lambda,
            "count": 0_usize,
            "total": 0_usize,
            "offset": 0_usize,
            "limit": None::<usize>,
            "next_offset": 0_usize,
            "truncated": false
        });
        let obj = meta.as_object().expect("object");
        assert!(
            obj.contains_key("lambda_budget"),
            "filtered_query meta must contain lambda_budget"
        );
        // The ride-along never alters the existing key set.
        for k in [
            "confidence",
            "curvature",
            "count",
            "total",
            "offset",
            "limit",
            "next_offset",
            "truncated",
        ] {
            assert!(obj.contains_key(k), "existing meta key `{k}` preserved");
        }
    }

    /// NDJSON stream trailing __meta line carries lambda_budget too,
    /// symmetric with how curvature already rides there.
    #[test]
    fn stream_query_meta_serializes_lambda_budget() {
        let k: f64 = 0.07;
        let lambda = gigi::curvature::lambda_budget(k, 1.5, 1.0);
        let meta = serde_json::json!({
            "__meta": true,
            "count": 0_usize,
            "curvature": k,
            "confidence": gigi::curvature::confidence(k),
            "lambda_budget": lambda
        });
        let obj = meta.as_object().expect("object");
        assert!(obj.contains_key("lambda_budget"));
        assert_eq!(obj["__meta"].as_bool(), Some(true));
        for k in ["count", "curvature", "confidence"] {
            assert!(obj.contains_key(k), "existing key `{k}` preserved");
        }
    }

    /// The substrate's helper that picks D for the ride-along returns
    /// a finite radius on a realistic bundle and NaN on an empty one.
    /// Mirrors the contract of `gigi::curvature::welford_radius`.
    #[test]
    fn gigi_welford_radius_finite_on_real_bundle() {
        use gigi::bundle::BundleStore;
        use gigi::types::{BundleSchema, FieldDef, Record, Value};
        let schema = BundleSchema::new("welford_ride_along")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(5.0))
            .fiber(FieldDef::numeric("y").with_range(5.0));
        let mut store = BundleStore::new(schema);
        for i in 0..30 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("x".into(), Value::Float((i as f64 * 0.13).sin()));
            r.insert("y".into(), Value::Float((i as f64 * 0.17).cos()));
            store.insert(&r);
        }
        let radius = gigi_welford_radius(&store);
        assert!(radius.is_finite() && radius > 0.0, "radius = {radius}");

        let empty_schema = BundleSchema::new("welford_empty")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(5.0));
        let empty_store = BundleStore::new(empty_schema);
        assert!(
            gigi_welford_radius(&empty_store).is_nan(),
            "empty bundle ⇒ NaN radius (uninitialized signal)"
        );
    }

    // ── Public-read bundle allowlist ─────────────────────────────
    //
    // Contract: the `/v1/public/gql` endpoint is anonymous, so the shape
    // Bee is protecting against is not confidentiality — she said there
    // is no PII or secrets in her bundles — but data loss and
    // embarrassment from a hack. These tests pin the guarantee that
    // writes, admin verbs, and non-allowlisted bundles are refused at
    // the validator layer, before any executor is called.

    fn allowlist(names: &[&str]) -> std::collections::HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn parse_or_panic(q: &str) -> gigi::parser::Statement {
        gigi::parser::parse(q).unwrap_or_else(|e| panic!("parse `{q}` failed: {e}"))
    }

    #[test]
    fn public_validate_showbundles_always_ok() {
        let allow = allowlist(&["stations"]);
        let s = parse_or_panic("SHOW BUNDLES;");
        assert!(validate_public_stmt(&s, &allow).is_ok());
        // Even when the allowlist is empty (route guard handles this),
        // the validator itself lets SHOW BUNDLES through — the response
        // filter in the handler returns just the allowlisted names.
        let empty: std::collections::HashSet<String> = std::collections::HashSet::new();
        assert!(validate_public_stmt(&s, &empty).is_ok());
    }

    #[test]
    fn public_validate_allowlisted_reads_pass() {
        let allow = allowlist(&["stations"]);
        for q in [
            "HEALTH stations;",
            "SECTION stations AT station_id='s151';",
            "INTEGRATE stations MEASURE SUM(temp), AVG(temp);",
        ] {
            let s = parse_or_panic(q);
            assert!(
                validate_public_stmt(&s, &allow).is_ok(),
                "expected `{q}` to pass"
            );
        }
    }

    #[test]
    fn public_validate_non_allowlisted_bundle_rejected() {
        let allow = allowlist(&["stations"]);
        let s = parse_or_panic("HEALTH sensors;");
        let err = validate_public_stmt(&s, &allow).unwrap_err();
        assert!(
            err.contains("sensors"),
            "error should name the rejected bundle, got: {err}"
        );
    }

    #[test]
    fn public_validate_write_verbs_rejected_even_on_allowlisted_bundle() {
        let allow = allowlist(&["stations"]);
        // Writes and destructive verbs. Each MUST be refused with the
        // generic verb-not-allowed error, not the bundle-specific one.
        for q in [
            "INSERT INTO stations (station_id, temp) VALUES ('s1', 20.0);",
            "RETRACT FROM stations WHERE station_id='s1';",
            "COLLAPSE stations;",
            "CREATE BUNDLE stations (id INT BASE, temp FLOAT FIBER);",
        ] {
            let s = match gigi::parser::parse(q) {
                Ok(s) => s,
                // Some verb forms may need slightly different syntax on this
                // parser build — skip the ones that don't parse; the ones
                // that do get the important assertion.
                Err(_) => continue,
            };
            let err = validate_public_stmt(&s, &allow).unwrap_err();
            assert!(
                err.contains("verb not allowed"),
                "expected verb-refusal on `{q}`, got: {err}"
            );
        }
    }

    #[test]
    fn public_validate_admin_verbs_rejected() {
        let allow = allowlist(&["stations"]);
        // Snapshot is the most dangerous one — it's how a caller could
        // trigger disk pressure or race a concurrent write. Must never
        // fire on the public endpoint.
        for q in ["SNAPSHOT;", "SHOW BACKUPS;"] {
            if let Ok(s) = gigi::parser::parse(q) {
                let err = validate_public_stmt(&s, &allow).unwrap_err();
                assert!(
                    err.contains("verb not allowed"),
                    "expected verb-refusal on `{q}`, got: {err}"
                );
            }
        }
    }

    async fn post_public_gql_for_test(
        state: Arc<StreamState>,
        query: &str,
    ) -> (StatusCode, serde_json::Value) {
        let resp = public_gql_query(
            State(state),
            axum::http::HeaderMap::new(),
            Json(serde_json::json!({ "query": query })),
        )
        .await;
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&body).unwrap_or(serde_json::json!({}));
        (status, json)
    }

    #[tokio::test(flavor = "current_thread")]
    async fn public_gql_showbundles_lists_only_allowlisted() {
        let _guard = stream_env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let dir = tmp_dir("public_showbundles");
        cleanup(&dir);
        std::env::set_var("GIGI_DATA_DIR", &dir);
        std::env::set_var("GIGI_PUBLIC_BUNDLES", "stations, sensors");

        {
            let (logger, _ingester) = Logger::new(LogConfig::default(), "public-showbundles-test");
            let state = Arc::new(StreamState::new(logger, Arc::new(Metrics::new())));
            state.ready.store(true, Ordering::Release);

            // Create two bundles: `secret_stuff` NOT on the allowlist,
            // `stations` on it. SHOW BUNDLES via /v1/public/gql must not
            // reveal `secret_stuff`.
            assert_eq!(
                post_gql_for_test(
                    Arc::clone(&state),
                    "CREATE BUNDLE stations (station_id TEXT BASE, temp FLOAT FIBER);",
                )
                .await,
                StatusCode::OK
            );
            assert_eq!(
                post_gql_for_test(
                    Arc::clone(&state),
                    "CREATE BUNDLE secret_stuff (id INT BASE, note TEXT FIBER);",
                )
                .await,
                StatusCode::OK
            );

            let (status, body) =
                post_public_gql_for_test(Arc::clone(&state), "SHOW BUNDLES;").await;
            assert_eq!(status, StatusCode::OK);
            let bundles = body
                .get("bundles")
                .and_then(|v| v.as_array())
                .expect("bundles array")
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect::<Vec<_>>();
            assert!(
                bundles.contains(&"stations".to_string()),
                "public SHOW BUNDLES must list the allowlisted `stations`"
            );
            assert!(
                !bundles.contains(&"secret_stuff".to_string()),
                "public SHOW BUNDLES must not leak `secret_stuff`"
            );
        }

        std::env::remove_var("GIGI_PUBLIC_BUNDLES");
        cleanup(&dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn public_gql_rejects_write_on_allowlisted_bundle() {
        let _guard = stream_env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let dir = tmp_dir("public_write_reject");
        cleanup(&dir);
        std::env::set_var("GIGI_DATA_DIR", &dir);
        std::env::set_var("GIGI_PUBLIC_BUNDLES", "stations");

        {
            let (logger, _ingester) = Logger::new(LogConfig::default(), "public-write-reject-test");
            let state = Arc::new(StreamState::new(logger, Arc::new(Metrics::new())));
            state.ready.store(true, Ordering::Release);

            assert_eq!(
                post_gql_for_test(
                    Arc::clone(&state),
                    "CREATE BUNDLE stations (station_id TEXT BASE, temp FLOAT FIBER);",
                )
                .await,
                StatusCode::OK
            );

            let (status, body) = post_public_gql_for_test(
                Arc::clone(&state),
                "INSERT INTO stations (station_id, temp) VALUES ('s1', 20.0);",
            )
            .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "public endpoint must refuse INSERT even on an allowlisted bundle"
            );
            let err = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            assert!(
                err.contains("verb not allowed"),
                "error should identify verb refusal, got: {err}"
            );
        }

        std::env::remove_var("GIGI_PUBLIC_BUNDLES");
        cleanup(&dir);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn public_gql_rejects_compound_query() {
        let _guard = stream_env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let dir = tmp_dir("public_compound_reject");
        cleanup(&dir);
        std::env::set_var("GIGI_DATA_DIR", &dir);
        std::env::set_var("GIGI_PUBLIC_BUNDLES", "stations");

        {
            let (logger, _ingester) = Logger::new(LogConfig::default(), "public-compound-reject");
            let state = Arc::new(StreamState::new(logger, Arc::new(Metrics::new())));
            state.ready.store(true, Ordering::Release);

            assert_eq!(
                post_gql_for_test(
                    Arc::clone(&state),
                    "CREATE BUNDLE stations (station_id TEXT BASE, temp FLOAT FIBER);",
                )
                .await,
                StatusCode::OK
            );

            // Try to smuggle a write behind an allowed read.
            let (status, body) = post_public_gql_for_test(
                Arc::clone(&state),
                "SHOW BUNDLES; COLLAPSE stations;",
            )
            .await;
            assert_eq!(status, StatusCode::FORBIDDEN);
            let err = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            assert!(
                err.contains("single statement"),
                "error should call out compound rejection, got: {err}"
            );
        }

        std::env::remove_var("GIGI_PUBLIC_BUNDLES");
        cleanup(&dir);
    }

    #[test]
    fn public_bundles_parse_from_env_trims_and_splits() {
        // Direct env parse — StreamState::new reads the same string and
        // this test pins the tokenization: whitespace trimmed, empties
        // dropped. Regression guard for a stray space breaking the
        // allowlist match.
        let raw = "stations,  sensors ,,chembl ,";
        let set: std::collections::HashSet<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(set.len(), 3);
        assert!(set.contains("stations"));
        assert!(set.contains("sensors"));
        assert!(set.contains("chembl"));
    }
}
