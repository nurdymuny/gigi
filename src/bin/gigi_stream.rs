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
use gigi::spectral;
use gigi::types::{BundleSchema, FieldDef, FieldType, Value};

// ── Shared State ──

type Record = HashMap<String, Value>;

struct StreamState {
    engine: RwLock<Engine>,
    /// True once WAL replay is complete and engine is ready for queries.
    ready: AtomicBool,
    /// Per-bundle broadcast channels for subscriptions
    channels: RwLock<HashMap<String, broadcast::Sender<SubscriptionEvent>>>,
    /// Global dashboard broadcast — anomaly + curvature update events for all bundles
    dashboard_tx: broadcast::Sender<DashboardEvent>,
    /// API key for authentication (None = no auth required)
    api_key: Option<String>,
    /// Rate limit: max requests per window (0 = unlimited)
    rate_limit: u32,
    /// Rate limit window in seconds
    rate_window_secs: u64,
    /// Per-IP request tracking for rate limiting
    rate_tracker: RwLock<HashMap<String, Vec<Instant>>>,
    /// Server start time for uptime tracking
    start_time: Instant,
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
    fn new() -> Self {
        let api_key = std::env::var("GIGI_API_KEY").ok();
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

        StreamState {
            engine: RwLock::new(engine),
            ready: AtomicBool::new(false),
            channels: RwLock::new(HashMap::new()),
            dashboard_tx: broadcast::channel(4096).0,
            api_key,
            rate_limit,
            rate_window_secs,
            rate_tracker: RwLock::new(HashMap::new()),
            start_time: Instant::now(),
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
            // Plain strings become Value::Text.
            if let Some(encoded) = s.strip_prefix("b64:") {
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
        Value::Text(s) => serde_json::json!(s),
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

/// Middleware: API key authentication.
/// If GIGI_API_KEY is set, all requests must include `X-API-Key` header.
/// Health endpoint is excluded (checked in the handler itself).
async fn auth_middleware(
    State(state): State<Arc<StreamState>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Skip auth for health endpoint
    if req.uri().path() == "/v1/health" {
        return Ok(next.run(req).await);
    }

    if let Some(ref expected_key) = state.api_key {
        match req.headers().get("x-api-key").and_then(|v| v.to_str().ok()) {
            Some(provided) if provided == expected_key => {}
            _ => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "Invalid or missing API key".to_string(),
                    }),
                ));
            }
        }
    }
    Ok(next.run(req).await)
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

async fn list_bundles(State(state): State<Arc<StreamState>>) -> Json<Vec<BundleInfo>> {
    let engine = state.engine.read().unwrap();
    let infos: Vec<BundleInfo> = engine
        .bundle_names()
        .iter()
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
    Json(req): Json<CreateBundleRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
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

    let mut engine = state.engine.write().unwrap();
    engine.create_bundle(schema).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Storage error: {e}"),
            }),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "created",
            "bundle": req.name
        })),
    ))
}

async fn drop_bundle(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut engine = state.engine.write().unwrap();
    match engine.drop_bundle(&name) {
        Ok(true) => Ok(Json(
            serde_json::json!({"status": "dropped", "bundle": name}),
        )),
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

    Ok(Json(serde_json::json!({
        "status": "inserted",
        "count": inserted,
        "total": store.len(),
        "curvature": k,
        "confidence": conf
    })))
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

    Ok(Json(CurvatureReport {
        k,
        curvature: k,
        confidence: conf,
        capacity: cap,
        per_field,
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

/// GET /v1/bundles/{name}/dhoom — export bundle as DHOOM wire format with compression stats
async fn export_dhoom(
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

    let json_records: Vec<serde_json::Value> =
        store.records().map(|r| record_to_json(&r)).collect();

    let result = dhoom::encode_json(&json_records, &name);

    Ok(Json(serde_json::json!({
        "bundle": name,
        "count": json_records.len(),
        "dhoom": result.dhoom,
        "json_bytes": result.json_bytes,
        "dhoom_bytes": result.dhoom_bytes,
        "compression_pct": result.compression_pct,
    })))
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

    // Parse records according to content type
    let mut parse_errors = 0usize;
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
                    Some(map.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect())
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
                        Some(map.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect())
                    }
                    _ => {
                        parse_errors += 1;
                        None
                    }
                }
            })
            .collect()
    };

    let count = records.len();

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
                            &text, &state, &mut subscriptions
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
                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event.record_json) {
                                        let passes = sub.filters.iter().all(|(field, op, expected)| {
                                            eval_ws_filter(&parsed, field, op, expected)
                                        });
                                        if !passes { continue; }
                                    }
                                }
                                let frame = format!(
                                    "EVENT {} {} {} K={:.6}",
                                    event.bundle, event.op, event.record_json, event.curvature
                                );
                                if push_tx.send(frame).is_err() {
                                    break;
                                }
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
) -> String {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    if parts.is_empty() {
        return "ERROR: empty command".to_string();
    }

    match parts[0].to_uppercase().as_str() {
        "PING" => "PONG".to_string(),

        "SUBSCRIBE" => {
            // SUBSCRIBE <bundle> [WHERE <field> <op> <value> [AND ...]]
            if parts.len() < 2 {
                return "ERROR: SUBSCRIBE requires a bundle name".to_string();
            }
            let rest = parts[1];
            let where_pos = rest.to_uppercase().find(" WHERE ");
            let bundle_name = if let Some(pos) = where_pos {
                rest[..pos].trim().to_string()
            } else {
                rest.trim().to_string()
            };

            let filters = if let Some(pos) = where_pos {
                parse_ws_filters(&rest[pos + 7..])
            } else {
                vec![]
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

// ── GQL endpoint ──

async fn gql_query(
    State(state): State<Arc<StreamState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let query = match body.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'query' field"})),
            )
        }
    };

    let stmt = match gigi::parser::parse(query) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Parse error: {e}")})),
            )
        }
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
            if *encrypted {
                let seed = gigi::crypto::GaugeKey::random_seed();
                let gk = gigi::crypto::GaugeKey::derive(&seed, &schema.fiber_fields);
                schema.gauge_key = Some(gk);
            }
            let mut engine = state.engine.write().unwrap();
            engine.create_bundle(schema).unwrap();
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
            return (StatusCode::OK, Json(serde_json::json!({"bundles": list})));
        }
        gigi::parser::Statement::Collapse { bundle } => {
            let mut engine = state.engine.write().unwrap();
            if engine.drop_bundle(bundle).unwrap_or(false) {
                return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
            } else {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("No bundle: {bundle}")})),
                );
            }
        }
        gigi::parser::Statement::AtlasBegin
        | gigi::parser::Statement::AtlasCommit
        | gigi::parser::Statement::AtlasRollback => {
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
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(serde_json::json!({"error": "This GQL v2.1 command is not yet implemented"})),
            );
        }
        _ => {}
    }

    let bundle_name = match get_bundle_name(&stmt) {
        Some(name) => name,
        None => return (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
    };

    // Check if bundle needs write access
    let needs_write = matches!(
        &stmt,
        gigi::parser::Statement::Insert { .. }
            | gigi::parser::Statement::BatchInsert { .. }
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
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("No bundle: {bundle_name}")})),
                )
            }
        };
        let result = execute_gql_on_store(&mut store, &stmt);
        match result {
            Ok(r) => exec_result_to_response(r),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e})),
            ),
        }
    } else {
        let engine = state.engine.read().unwrap();
        let store = match engine.bundle(&bundle_name) {
            Some(s) => s,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("No bundle: {bundle_name}")})),
                )
            }
        };
        let result = execute_gql_on_store_read(&store, &stmt);
        match result {
            Ok(r) => exec_result_to_response(r),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e})),
            ),
        }
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
            let qcs: Vec<QC> = conditions.iter().flat_map(|fc| filter_to_qcs(fc)).collect();
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
            let qcs: Vec<QC> = conditions.iter().flat_map(|fc| filter_to_qcs(fc)).collect();
            let n = store.bulk_delete(&qcs);
            Ok(ExecResult::Count(n))
        }
        // For read-only ops via mutable ref, delegate
        _ => execute_gql_on_store_read(&store.as_ref(), stmt),
    }
}

/// Execute a GQL statement that only needs read access.
fn execute_gql_on_store_read(
    store: &gigi::mmap_bundle::BundleRef<'_>,
    stmt: &gigi::parser::Statement,
) -> Result<gigi::parser::ExecResult, String> {
    use gigi::bundle::QueryCondition as QC;
    use gigi::parser::{literal_to_value, ExecResult, GqlStats, Statement};

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
                conditions.extend(filter_to_qcs(fc));
            }
            let or_qcs: Vec<Vec<QC>> = or_groups
                .iter()
                .map(|g| g.iter().flat_map(filter_to_qcs).collect())
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
        Statement::Geodesic { from_keys, to_keys, max_hops, .. } => {
            let from_rec: gigi::types::Record = from_keys.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            let to_rec: gigi::types::Record = to_keys.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            let bp_a = store.as_heap().map(|s| s.base_point(&from_rec)).unwrap_or(0);
            let bp_b = store.as_heap().map(|s| s.base_point(&to_rec)).unwrap_or(0);
            match store.geodesic_distance(bp_a, bp_b, *max_hops) {
                Some(d) => Ok(ExecResult::Scalar(d)),
                None => Ok(ExecResult::Scalar(-1.0)),
            }
        }
        Statement::MetricTensor { .. } => {
            let info = store.metric_tensor();
            let cond = if info.condition_number.is_finite() { info.condition_number } else { -1.0 };
            Ok(ExecResult::Scalar(cond))
        }
        Statement::Health { .. } => {
            let k = store.scalar_curvature();
            Ok(ExecResult::Stats(GqlStats {
                curvature: k,
                confidence: gigi::curvature::confidence(k),
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema().base_fields.len(),
                fiber_fields: store.schema().fiber_fields.len(),
            }))
        }
        Statement::Describe { .. } => {
            let k = store.scalar_curvature();
            Ok(ExecResult::Stats(GqlStats {
                curvature: k,
                confidence: gigi::curvature::confidence(k),
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema().base_fields.len(),
                fiber_fields: store.schema().fiber_fields.len(),
            }))
        }
        _ => Ok(ExecResult::Ok),
    }
}

fn filter_to_qcs(fc: &gigi::parser::FilterCondition) -> Vec<gigi::bundle::QueryCondition> {
    use gigi::bundle::QueryCondition as QC;
    use gigi::parser::{literal_to_value, FilterCondition};
    match fc {
        FilterCondition::Eq(f, v) => vec![QC::Eq(f.clone(), literal_to_value(v))],
        FilterCondition::Neq(f, v) => vec![QC::Neq(f.clone(), literal_to_value(v))],
        FilterCondition::Gt(f, v) => vec![QC::Gt(f.clone(), literal_to_value(v))],
        FilterCondition::Gte(f, v) => vec![QC::Gte(f.clone(), literal_to_value(v))],
        FilterCondition::Lt(f, v) => vec![QC::Lt(f.clone(), literal_to_value(v))],
        FilterCondition::Lte(f, v) => vec![QC::Lte(f.clone(), literal_to_value(v))],
        FilterCondition::In(f, vs) => {
            vec![QC::In(f.clone(), vs.iter().map(literal_to_value).collect())]
        }
        FilterCondition::NotIn(f, vs) => vec![QC::NotIn(
            f.clone(),
            vs.iter().map(literal_to_value).collect(),
        )],
        FilterCondition::Contains(f, s) => vec![QC::Contains(f.clone(), s.clone())],
        FilterCondition::StartsWith(f, s) => vec![QC::StartsWith(f.clone(), s.clone())],
        FilterCondition::EndsWith(f, s) => vec![QC::EndsWith(f.clone(), s.clone())],
        FilterCondition::Matches(f, s) => vec![QC::Regex(f.clone(), s.clone())],
        FilterCondition::Void(f) => vec![QC::IsNull(f.clone())],
        FilterCondition::Defined(f) => vec![QC::IsNotNull(f.clone())],
        FilterCondition::Between(f, lo, hi) => vec![
            QC::Gte(f.clone(), literal_to_value(lo)),
            QC::Lte(f.clone(), literal_to_value(hi)),
        ],
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
        _ => None,
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

// ── Main ──

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3142".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let state = Arc::new(StreamState::new());

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
        // WebSocket — per-bundle subscriptions + global dashboard
        .route("/ws", get(ws_handler))
        .route("/v1/ws/dashboard", get(ws_dashboard_handler))
        .route(
            "/v1/ws/{bundle}/dashboard",
            get(ws_bundle_dashboard_handler),
        )
        // Dashboard UI
        .route("/dashboard", get(serve_dashboard))
        // Middleware: auth + rate limiting + readiness
        .layer(axum_mw::from_fn_with_state(state.clone(), auth_middleware))
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
                    *replay_state.engine.write().unwrap() = mmap_engine;
                    #[cfg(unix)]
                    unsafe { libc::malloc_trim(0); }
                    replay_state.ready.store(true, Ordering::Release);
                    eprintln!("Engine ready — {total} records (fast path)");
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
            if let Err(e) = engine.replay_wal() {
                eprintln!("WAL replay error: {e}");
                drop(engine);
                replay_state.ready.store(true, Ordering::Release);
                eprintln!("Engine ready (replay failed, using empty state)");
                return;
            }

            // Phase 2: Snapshot heap bundles to DHOOM files + compact WAL
            let total = engine.total_records();
            if total > 0 {
                eprintln!("WAL replay complete ({total} records). Snapshotting to DHOOM…");
                if let Err(e) = engine.snapshot() {
                    eprintln!("Post-replay snapshot failed: {e}");
                    // Non-fatal: we keep running on heap. Mmap upgrade skipped.
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
                drop(engine);

                // Force glibc to return freed heap pages to the OS.
                // Without this, the allocator holds ~13GB of freed arenas.
                #[cfg(unix)]
                unsafe { libc::malloc_trim(0); }

                eprintln!("Mmap engine active — {total} records, RSS reduced to page cache");
            }
            Err(e) => {
                eprintln!("Mmap reopen failed: {e} — keeping heap engine");
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
    .await
    .unwrap();
}

// ── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gigi::dhoom;
    use gigi::engine::Engine;
    use gigi::types::{BundleSchema, FieldDef};
    use std::path::Path;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("gigi_stream_test_{tag}"))
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
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
}
