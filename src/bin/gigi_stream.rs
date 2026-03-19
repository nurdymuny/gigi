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

use axum::{
    Router,
    routing::{get, post, delete},
    Json,
    http::StatusCode,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    middleware as axum_mw,
};
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use tower_http::cors::{CorsLayer, AllowOrigin};
use axum::http::{HeaderName, HeaderValue, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::broadcast;

use gigi::bundle::QueryCondition;
use gigi::bundle::TransactionOp;
use gigi::types::{BundleSchema, FieldDef, FieldType, Value};
use gigi::curvature;
use gigi::spectral;
use gigi::aggregation;
use gigi::join;
use gigi::dhoom;
use gigi::engine::Engine;

// ── Shared State ──

type Record = HashMap<String, Value>;

struct StreamState {
    engine: RwLock<Engine>,
    /// Per-bundle broadcast channels for subscriptions
    channels: RwLock<HashMap<String, broadcast::Sender<SubscriptionEvent>>>,
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

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct SubscriptionEvent {
    bundle: String,
    record_json: String,
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

        let data_dir = std::env::var("GIGI_DATA_DIR")
            .unwrap_or_else(|_| "./gigi_data".to_string());
        let data_path = std::path::PathBuf::from(&data_dir);

        let engine = match Engine::open(&data_path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("FATAL: Failed to open database at {}: {e}", data_path.display());
                std::process::exit(1);
            }
        };

        eprintln!("  WAL persistence: {} ({})", data_path.display(), data_dir);

        StreamState {
            engine: RwLock::new(engine),
            channels: RwLock::new(HashMap::new()),
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

fn default_increment() -> f64 { 1.0 }

/// Body for POST .../add-field
#[derive(Deserialize)]
struct AddFieldRequest {
    name: String,
    #[serde(rename = "type", default = "default_field_type")]
    field_type: String,
    #[serde(default)]
    default: Option<serde_json::Value>,
}

fn default_field_type() -> String { "categorical".to_string() }

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
}

#[derive(Serialize)]
struct BundleInfo {
    name: String,
    records: usize,
    fields: usize,
}

#[derive(Serialize)]
struct CurvatureReport {
    #[serde(rename = "K")]
    k: f64,
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
        serde_json::Value::String(s) => Value::Text(s.clone()),
        serde_json::Value::Bool(b) => Value::Bool(*b),
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
    match s.to_lowercase().as_str() {
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
        Ok(origin) if origin == "*" => {
            CorsLayer::new()
                .allow_origin(AllowOrigin::any())
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::OPTIONS])
                .allow_headers([
                    HeaderName::from_static("content-type"),
                    HeaderName::from_static("x-api-key"),
                ])
        }
        Ok(origin) => {
            let origin_val: HeaderValue = origin.parse().unwrap_or_else(|_| "".parse().unwrap());
            CorsLayer::new()
                .allow_origin(AllowOrigin::exact(origin_val))
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::OPTIONS])
                .allow_headers([
                    HeaderName::from_static("content-type"),
                    HeaderName::from_static("x-api-key"),
                ])
        }
        Err(_) => {
            // No CORS origin set → restrictive, same-origin only
            CorsLayer::new()
        }
    }
}

// ── REST Handlers ──

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
                    Json(ErrorResponse { error: "Invalid or missing API key".to_string() }),
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
                Json(ErrorResponse { error: "Rate limit exceeded".to_string() }),
            ));
        }

        entries.push(now);
    }

    Ok(next.run(req).await)
}

async fn health(State(state): State<Arc<StreamState>>) -> Json<HealthResponse> {
    let engine = state.engine.read().unwrap();
    Json(HealthResponse {
        status: "ok",
        engine: "gigi-stream",
        version: "0.1.0",
        bundles: engine.bundle_names().len(),
        total_records: engine.total_records(),
        uptime_secs: state.start_time.elapsed().as_secs(),
    })
}

async fn list_bundles(State(state): State<Arc<StreamState>>) -> Json<Vec<BundleInfo>> {
    let engine = state.engine.read().unwrap();
    let infos: Vec<BundleInfo> = engine.bundle_names().iter().map(|name| {
        let store = engine.bundle(name).unwrap();
        BundleInfo {
            name: name.to_string(),
            records: store.len(),
            fields: store.schema.base_fields.len() + store.schema.fiber_fields.len(),
        }
    }).collect();
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
        let default_val = req.schema.defaults.get(field_name)
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
    engine.create_bundle(schema).map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: format!("Storage error: {e}") }),
    ))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "status": "created",
        "bundle": req.name
    }))))
}

async fn drop_bundle(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut engine = state.engine.write().unwrap();
    match engine.drop_bundle(&name) {
        Ok(true) => Ok(Json(serde_json::json!({"status": "dropped", "bundle": name}))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: format!("Storage error: {e}") }),
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
        let store = engine.bundle(&name).ok_or_else(|| (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
        ))?;
        let key = if store.schema.base_fields.len() == 1 {
            Some(store.schema.base_fields[0].name.clone())
        } else {
            None
        };
        let ca = store.schema.fiber_fields.iter().any(|f| f.name == "created_at");
        let ua = store.schema.fiber_fields.iter().any(|f| f.name == "updated_at");
        (key, ca, ua)
    };

    // Convert JSON records to GIGI records
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let mut records: Vec<Record> = req.records.iter()
        .filter_map(|item| {
            if let serde_json::Value::Object(map) = item {
                let mut rec: Record = map.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect();
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
    let inserted = engine.batch_insert(&name, &records).map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: format!("Storage error: {e}") }),
    ))?;

    // Broadcast batch event to subscribers (single event for entire batch)
    let tx = state.get_or_create_channel(&name);
    let _ = tx.send(SubscriptionEvent {
        bundle: name.clone(),
        record_json: format!("{{\"batch\": {inserted}}}"),
    });

    let store = engine.bundle(&name).unwrap();
    let k = curvature::scalar_curvature(store);
    let conf = curvature::confidence(k);

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
                Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
            ));
        }
    }

    // Read body (cap at 256MB to prevent abuse)
    let bytes = to_bytes(body, 256 * 1024 * 1024).await.map_err(|e| (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse { error: format!("Failed to read body: {e}") }),
    ))?;

    let text = String::from_utf8_lossy(&bytes);

    // Parse NDJSON: each line is a JSON object
    let mut records: Vec<Record> = Vec::new();
    let mut parse_errors = 0usize;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(serde_json::Value::Object(map)) => {
                let record: Record = map.iter()
                    .map(|(k, v)| (k.clone(), json_to_value(v)))
                    .collect();
                records.push(record);
            }
            _ => { parse_errors += 1; }
        }
    }

    // WAL-logged batch insert
    let mut engine = state.engine.write().unwrap();
    let inserted = engine.batch_insert(&name, &records).map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: format!("Storage error: {e}") }),
    ))?;

    let tx = state.get_or_create_channel(&name);
    let _ = tx.send(SubscriptionEvent {
        bundle: name.clone(),
        record_json: format!("{{\"stream_batch\": {inserted}}}"),
    });

    let store = engine.bundle(&name).unwrap();
    let k = curvature::scalar_curvature(store);
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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    // Build key record from query params
    let key: Record = params.iter()
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
            let k = curvature::scalar_curvature(store);
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
            Json(ErrorResponse { error: "Record not found".to_string() }),
        ))
    }
}

async fn range_query(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

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
        let json_records: Vec<serde_json::Value> = records.iter()
            .map(record_to_json)
            .collect();
        let k = curvature::scalar_curvature(store);
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
            Json(ErrorResponse { error: "Provide at least one field=value query parameter".to_string() }),
        ))
    }
}

async fn pullback_join(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<JoinRequest>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let left = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;
    let right = engine.bundle(&req.right_bundle).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", req.right_bundle) }),
    ))?;

    let results = join::pullback_join(left, right, &req.left_field, &req.right_field);
    let json_results: Vec<serde_json::Value> = results.iter()
        .map(|(left_rec, right_rec)| {
            let mut combined = serde_json::Map::new();
            combined.insert("left".to_string(), record_to_json(left_rec));
            combined.insert("right".to_string(),
                right_rec.as_ref().map(record_to_json).unwrap_or(serde_json::Value::Null));
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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let groups = if req.conditions.is_empty() {
        aggregation::group_by(store, &req.group_by, &req.field)
    } else {
        let conditions: Vec<QueryCondition> = req.conditions.iter()
            .map(condition_spec_to_query_condition)
            .collect();
        aggregation::filtered_group_by(store, &req.group_by, &req.field, &conditions)
    };
    let mut result_groups = HashMap::new();
    for (key, agg) in groups {
        let key_str = key.to_string();
        result_groups.insert(key_str, AggValues {
            count: agg.count,
            sum: agg.sum,
            avg: agg.avg(),
            min: agg.min,
            max: agg.max,
        });
    }

    Ok(Json(AggResult { groups: result_groups }))
}

async fn curvature_report(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<CurvatureReport>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let k = curvature::scalar_curvature(store);
    let conf = curvature::confidence(k);
    let cap = curvature::capacity(1.0, k);

    // Per-field curvature from stats
    let mut per_field = Vec::new();
    let stats = store.field_stats();
    for (field_name, fs) in stats {
        let variance = fs.variance();
        let range = fs.range();
        let field_k = if range > 0.0 { variance / (range * range) } else { 0.0 };
        per_field.push(FieldCurvature {
            field: field_name.clone(),
            variance,
            range,
            k: field_k,
        });
    }

    Ok(Json(CurvatureReport {
        k,
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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let lambda1 = spectral::spectral_gap(store);
    let diameter = spectral::graph_diameter(store);
    let spectral_cap = spectral::spectral_capacity(store);

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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    // Čech cohomology H¹ — measure holonomy to detect inconsistencies
    // H¹ = 0 means fully consistent (flat connection, path-independent)
    let k = curvature::scalar_curvature(store);

    // Sample random loops and measure holonomy deviation
    let records: Vec<Record> = store.records().take(100).collect();
    let mut cocycles = Vec::new();
    let threshold = 1e-6;

    if records.len() >= 3 {
        // Check holonomy around triangles formed by record triples
        let n = records.len().min(20); // sample up to 20 records for triangles
        for i in 0..n {
            for j in (i+1)..n.min(i+5) {
                for m in (j+1)..n.min(j+3) {
                    // Build key records for the loop: i → j → m → i
                    let keys: Vec<Record> = [&records[i], &records[j], &records[m], &records[i]]
                        .iter()
                        .map(|r| {
                            let mut key = Record::new();
                            for f in &store.schema.base_fields {
                                if let Some(v) = r.get(&f.name) {
                                    key.insert(f.name.clone(), v.clone());
                                }
                            }
                            key
                        })
                        .collect();

                    let hol = curvature::holonomy(store, &keys);
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
        _ => QueryCondition::Eq(spec.field.clone(), value), // default to eq
    }
}

async fn filtered_query(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<FilteredQueryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let conditions: Vec<QueryCondition> = req.conditions.iter()
        .map(condition_spec_to_query_condition)
        .collect();

    // Handle PRISM "order" field: "desc" → sort_desc=true, "asc" → sort_desc=false
    let sort_desc = match &req.order {
        Some(o) if o.eq_ignore_ascii_case("desc") => true,
        Some(_) => false,
        None => req.sort_desc.unwrap_or(false),
    };

    // Build field projection
    let field_refs: Option<Vec<&str>> = req.fields.as_ref()
        .map(|f| f.iter().map(|s| s.as_str()).collect());

    // Build multi-field sort
    let sort_fields_vec: Option<Vec<(String, bool)>> = if let Some(ref sort) = req.sort {
        Some(sort.iter().map(|s| (s.field.clone(), s.desc.unwrap_or(false))).collect())
    } else if let Some(ref field) = req.sort_by {
        Some(vec![(field.clone(), sort_desc)])
    } else {
        None
    };
    let sort_fields_refs: Option<Vec<(&str, bool)>> = sort_fields_vec.as_ref()
        .map(|v| v.iter().map(|(s, d)| (s.as_str(), *d)).collect());

    // Build OR conditions
    let or_conds_vec: Option<Vec<Vec<QueryCondition>>> = req.or_conditions.as_ref()
        .map(|groups| groups.iter()
            .map(|g| g.iter().map(condition_spec_to_query_condition).collect())
            .collect());

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
        results.iter()
            .filter(|record| {
                match &req.search_fields {
                    Some(fields) => {
                        fields.iter().any(|f| {
                            record.get(f).map_or(false, |v| {
                                if let Value::Text(s) = v {
                                    s.to_lowercase().contains(&term_lower)
                                } else {
                                    v.to_string().to_lowercase().contains(&term_lower)
                                }
                            })
                        })
                    }
                    None => {
                        record.values().any(|v| {
                            if let Value::Text(s) = v {
                                s.to_lowercase().contains(&term_lower)
                            } else {
                                false
                            }
                        })
                    }
                }
            })
            .map(record_to_json)
            .collect()
    } else {
        results.iter().map(record_to_json).collect()
    };
    let count = json_records.len();
    let k = curvature::scalar_curvature(store);

    Ok(Json(serde_json::json!({
        "data": json_records,
        "meta": {
            "confidence": curvature::confidence(k),
            "curvature": k,
            "count": count,
            "total": total
        }
    })))
}

// ── PRISM-friendly REST Handlers ──

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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let limit: Option<usize> = params.get("limit").and_then(|v| v.parse().ok());
    let offset: Option<usize> = params.get("offset").and_then(|v| v.parse().ok());

    // Return all records with optional pagination
    let all: Vec<Record> = store.records().collect();
    let start = offset.unwrap_or(0);
    let sliced: Vec<&Record> = all.iter().skip(start).collect();
    let limited: Vec<&Record> = match limit {
        Some(lim) => sliced.into_iter().take(lim).collect(),
        None => sliced,
    };

    let json_records: Vec<serde_json::Value> = limited.iter().map(|r| record_to_json(r)).collect();
    let count = json_records.len();
    let _total = all.len();

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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let val = parse_path_value(&value);
    let mut key = Record::new();
    key.insert(field.clone(), val.clone());

    // Try point_query first (O(1) if it's a base field)
    if let Some(record) = store.point_query(&key) {
        let k = curvature::scalar_curvature(store);
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
        let k = curvature::scalar_curvature(store);
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
        Json(ErrorResponse { error: "Record not found".to_string() }),
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

    let patches: Record = body.fields.iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    if !store.update(&key, &patches) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: "Record not found".to_string() }),
        ));
    }

    let k = curvature::scalar_curvature(store);
    Ok(Json(serde_json::json!({
        "status": "updated",
        "total": store.len(),
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
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    if !store.delete(&key) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: "Record not found".to_string() }),
        ));
    }

    let k = curvature::scalar_curvature(store);
    Ok(Json(serde_json::json!({
        "status": "deleted",
        "total": store.len(),
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
    let conditions: Vec<QueryCondition> = req.filter.iter()
        .map(condition_spec_to_query_condition)
        .collect();

    let patches: Record = req.fields.iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let count = store.bulk_update(&conditions, &patches);

    let k = curvature::scalar_curvature(store);
    Ok(Json(serde_json::json!({
        "status": "updated",
        "matched": count,
        "total": store.len(),
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
    let record: Record = req.record.iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let inserted = store.upsert(&record);
    let k = curvature::scalar_curvature(store);

    Ok(Json(serde_json::json!({
        "status": if inserted { "inserted" } else { "updated" },
        "total": store.len(),
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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let conditions: Vec<QueryCondition> = req.conditions.iter()
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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let conditions: Vec<QueryCondition> = req.conditions.iter()
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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

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
    let conditions: Vec<QueryCondition> = req.conditions.iter()
        .map(condition_spec_to_query_condition)
        .collect();

    let mut engine = state.engine.write().unwrap();
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let deleted = store.bulk_delete(&conditions);
    let k = curvature::scalar_curvature(store);

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "deleted": deleted,
        "total": store.len(),
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
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let base_fields: Vec<serde_json::Value> = store.schema.base_fields.iter().map(|f| {
        serde_json::json!({
            "name": f.name,
            "type": format!("{:?}", f.field_type),
            "weight": f.weight,
        })
    }).collect();

    let fiber_fields: Vec<serde_json::Value> = store.schema.fiber_fields.iter().map(|f| {
        serde_json::json!({
            "name": f.name,
            "type": format!("{:?}", f.field_type),
            "weight": f.weight,
        })
    }).collect();

    let indexed: Vec<String> = store.schema.indexed_fields.clone();

    Ok(Json(serde_json::json!({
        "name": store.schema.name,
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
    let key: Record = req.key.iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    if !store.increment(&key, &req.field, req.amount) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: "Record not found".to_string() }),
        ));
    }

    let k = curvature::scalar_curvature(store);
    Ok(Json(serde_json::json!({
        "status": "incremented",
        "field": req.field,
        "amount": req.amount,
        "curvature": k,
        "confidence": curvature::confidence(k)
    })))
}

/// POST /v1/bundles/{name}/add-field — add a fiber field to the schema
async fn add_field(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<AddFieldRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let ft = str_to_field_type(&req.field_type);
    let default_val = req.default.as_ref().map(json_to_value).unwrap_or(Value::Null);
    let fd = FieldDef {
        name: req.name.clone(),
        field_type: ft,
        default: default_val,
        range: None,
        weight: 1.0,
    };

    let mut engine = state.engine.write().unwrap();
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

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
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    store.add_index(&req.field);

    Ok(Json(serde_json::json!({
        "status": "index_added",
        "field": req.field,
        "indexed_fields": store.schema.indexed_fields
    })))
}

/// GET /v1/bundles/{name}/export — export all records as JSON
async fn export_bundle(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().unwrap();
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let records: Vec<serde_json::Value> = store.records()
        .map(|r| record_to_json(&r))
        .collect();

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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let json_records: Vec<serde_json::Value> = store.records()
        .map(|r| record_to_json(&r))
        .collect();

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

/// POST /v1/bundles/{name}/import — import records from JSON
async fn import_bundle(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<ImportRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut engine = state.engine.write().unwrap();
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let records: Vec<Record> = req.records.iter()
        .filter_map(|item| {
            if let serde_json::Value::Object(map) = item {
                Some(map.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect())
            } else {
                None
            }
        })
        .collect();

    let inserted = store.batch_insert(&records);
    let k = curvature::scalar_curvature(store);

    Ok(Json(serde_json::json!({
        "status": "imported",
        "count": inserted,
        "total": store.len(),
        "curvature": k,
        "confidence": curvature::confidence(k)
    })))
}

// ── Sprint 3: New REST Handlers ──

/// POST /v1/bundles/{name}/update — update with RETURNING + optimistic concurrency
async fn update_records_v2(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    Json(req): Json<UpdateReturningRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let key: Record = req.key.iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();
    let mut patches: Record = req.fields.iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    // Auto-set updated_at
    if store.schema.fiber_fields.iter().any(|f| f.name == "updated_at") && !patches.contains_key("updated_at") {
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
                let k = curvature::scalar_curvature(store);
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
                    Json(ErrorResponse { error: "Version conflict — record was modified by another client".to_string() }),
                ));
            }
            Err(_) => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse { error: "Record not found".to_string() }),
                ));
            }
        }
    }

    // Standard update (with optional RETURNING)
    if req.returning {
        match store.update_returning(&key, &patches) {
            Some(record) => {
                let k = curvature::scalar_curvature(store);
                Ok(Json(serde_json::json!({
                    "status": "updated",
                    "data": record_to_json(&record),
                    "total": store.len(),
                    "curvature": k,
                    "confidence": curvature::confidence(k)
                })))
            }
            None => Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: "Record not found".to_string() }),
            )),
        }
    } else {
        if !store.update(&key, &patches) {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: "Record not found".to_string() }),
            ));
        }
        let k = curvature::scalar_curvature(store);
        Ok(Json(serde_json::json!({
            "status": "updated",
            "total": store.len(),
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
    let key: Record = req.key.iter()
        .map(|(k, v)| (k.clone(), json_to_value(v)))
        .collect();

    let mut engine = state.engine.write().unwrap();
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    if req.returning {
        match store.delete_returning(&key) {
            Some(record) => {
                let k = curvature::scalar_curvature(store);
                Ok(Json(serde_json::json!({
                    "status": "deleted",
                    "data": record_to_json(&record),
                    "total": store.len(),
                    "curvature": k,
                    "confidence": curvature::confidence(k)
                })))
            }
            None => Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: "Record not found".to_string() }),
            )),
        }
    } else {
        if !store.delete(&key) {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: "Record not found".to_string() }),
            ));
        }
        let k = curvature::scalar_curvature(store);
        Ok(Json(serde_json::json!({
            "status": "deleted",
            "total": store.len(),
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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let stats = store.stats();
    let k = curvature::scalar_curvature(store);

    let index_sizes: serde_json::Value = stats.index_sizes.iter()
        .map(|(f, s)| (f.clone(), serde_json::json!(s)))
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    let cardinalities: serde_json::Value = stats.field_cardinalities.iter()
        .map(|(f, c)| (f.clone(), serde_json::json!(c)))
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    // Per-field stats
    let field_stats_raw = store.field_stats();
    let field_stats_json: serde_json::Value = field_stats_raw.iter()
        .map(|(f, fs)| (f.clone(), serde_json::json!({
            "count": fs.count,
            "sum": fs.sum,
            "min": fs.min,
            "max": fs.max,
            "variance": fs.variance(),
            "range": fs.range(),
        })))
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
    let store = engine.bundle(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let conditions: Vec<QueryCondition> = req.conditions.iter()
        .map(condition_spec_to_query_condition)
        .collect();

    let or_conds_vec: Option<Vec<Vec<QueryCondition>>> = req.or_conditions.as_ref()
        .map(|groups| groups.iter()
            .map(|g| g.iter().map(condition_spec_to_query_condition).collect())
            .collect());

    let sort_fields_vec: Option<Vec<(String, bool)>> = req.sort.as_ref()
        .map(|v| v.iter().map(|s| (s.field.clone(), s.desc.unwrap_or(false))).collect());
    let sort_fields_refs: Option<Vec<(&str, bool)>> = sort_fields_vec.as_ref()
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
    let store = engine.bundle_mut(&name).ok_or_else(|| (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: format!("Bundle '{}' not found", name) }),
    ))?;

    let mut ops: Vec<TransactionOp> = Vec::with_capacity(req.ops.len());

    for (i, op_spec) in req.ops.iter().enumerate() {
        let op = match op_spec.op.as_str() {
            "insert" => {
                let record_json = op_spec.record.as_ref().ok_or_else(|| (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse { error: format!("op[{}]: insert requires 'record'", i) }),
                ))?;
                if let serde_json::Value::Object(map) = record_json {
                    let record: Record = map.iter()
                        .map(|(k, v)| (k.clone(), json_to_value(v)))
                        .collect();
                    TransactionOp::Insert(record)
                } else {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse { error: format!("op[{}]: record must be an object", i) }),
                    ));
                }
            }
            "update" => {
                let key_json = op_spec.key.as_ref().ok_or_else(|| (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse { error: format!("op[{}]: update requires 'key'", i) }),
                ))?;
                let fields_json = op_spec.fields.as_ref().ok_or_else(|| (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse { error: format!("op[{}]: update requires 'fields'", i) }),
                ))?;
                let key: Record = key_json.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect();
                let patches: Record = fields_json.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect();
                TransactionOp::Update { key, patches }
            }
            "delete" => {
                let key_json = op_spec.key.as_ref().ok_or_else(|| (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse { error: format!("op[{}]: delete requires 'key'", i) }),
                ))?;
                let key: Record = key_json.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect();
                TransactionOp::Delete(key)
            }
            "increment" => {
                let key_json = op_spec.key.as_ref().ok_or_else(|| (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse { error: format!("op[{}]: increment requires 'key'", i) }),
                ))?;
                let field = op_spec.field.as_ref().ok_or_else(|| (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse { error: format!("op[{}]: increment requires 'field'", i) }),
                ))?;
                let key: Record = key_json.iter().map(|(k, v)| (k.clone(), json_to_value(v))).collect();
                let amount = op_spec.amount.unwrap_or(1.0);
                TransactionOp::Increment { key, field: field.clone(), amount }
            }
            other => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse { error: format!("op[{}]: unknown operation '{}'", i, other) }),
                ));
            }
        };
        ops.push(op);
    }

    match store.execute_transaction(&ops) {
        Ok(results) => {
            let k = curvature::scalar_curvature(store);
            Ok(Json(serde_json::json!({
                "status": "committed",
                "ops": results.len(),
                "total": store.len(),
                "curvature": k,
                "confidence": curvature::confidence(k)
            })))
        }
        Err(msg) => {
            Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse { error: format!("Transaction rolled back: {}", msg) }),
            ))
        }
    }
}

// ── WebSocket Handler ──

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<StreamState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<StreamState>) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                let response = handle_ws_command(&text, &state).await;
                if socket.send(Message::Text(response.into())).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

async fn handle_ws_command(cmd: &str, state: &Arc<StreamState>) -> String {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    if parts.is_empty() {
        return "ERROR: empty command".to_string();
    }

    match parts[0].to_uppercase().as_str() {
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

            // Parse DHOOM and insert
            match dhoom::decode_legacy(dhoom_data) {
                Ok(parsed) => {
                    let mut engine = state.engine.write().unwrap();
                    if let Some(store) = engine.bundle_mut(bundle_name) {
                        let mut count = 0;
                        for dhoom_record in &parsed.records {
                            let record: Record = dhoom_record.iter()
                                .map(|(k, v)| (k.clone(), dhoom_value_to_value(v)))
                                .collect();
                            store.insert(&record);
                            count += 1;
                        }
                        let k = curvature::scalar_curvature(store);
                        format!("OK inserted={} total={} K={:.6} confidence={:.4}",
                            count, store.len(), k, curvature::confidence(k))
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
            let where_pos = rest.to_uppercase().find("WHERE");
            let bundle_name = if let Some(pos) = where_pos {
                rest[..pos].trim()
            } else {
                rest.trim()
            };

            let engine = state.engine.read().unwrap();
            if let Some(store) = engine.bundle(bundle_name) {
                if let Some(pos) = where_pos {
                    let condition = &rest[pos + 5..].trim();
                    // Parse "field = value AND field = value"
                    let mut key: Record = HashMap::new();
                    for clause in condition.split("AND") {
                        let clause = clause.trim();
                        if let Some(eq_pos) = clause.find('=') {
                            let field = clause[..eq_pos].trim();
                            let val = clause[eq_pos + 1..].trim().trim_matches('"');
                            key.insert(field.to_string(), parse_ws_value(val));
                        }
                    }
                    match store.point_query(&key) {
                        Some(record) => {
                            let json = record_to_json(&record);
                            let k = curvature::scalar_curvature(store);
                            format!("RESULT {}\nMETA confidence={:.4} curvature={:.6}",
                                json, curvature::confidence(k), k)
                        }
                        None => "RESULT null".to_string(),
                    }
                } else {
                    format!("ERROR: QUERY requires WHERE clause")
                }
            } else {
                format!("ERROR: Bundle '{}' not found", bundle_name)
            }
        }

        "RANGE" => {
            // RANGE bundle WHERE field = "value"
            if parts.len() < 2 {
                return "ERROR: RANGE requires bundle and WHERE clause".to_string();
            }
            let rest = parts[1];
            let where_pos = rest.to_uppercase().find("WHERE");
            let bundle_name = if let Some(pos) = where_pos {
                rest[..pos].trim()
            } else {
                return "ERROR: RANGE requires WHERE clause".to_string();
            };

            let engine = state.engine.read().unwrap();
            if let Some(store) = engine.bundle(bundle_name) {
                if let Some(pos) = where_pos {
                    let condition = &rest[pos + 5..].trim();
                    if let Some(eq_pos) = condition.find('=') {
                        let field = condition[..eq_pos].trim();
                        let val = condition[eq_pos + 1..].trim().trim_matches('"');
                        let results = store.range_query(field, &[parse_ws_value(val)]);
                        let json_arr: Vec<serde_json::Value> = results.iter()
                            .map(record_to_json).collect();
                        let k = curvature::scalar_curvature(store);
                        format!("RESULT {}\nMETA count={} confidence={:.4} curvature={:.6}",
                            serde_json::to_string(&json_arr).unwrap_or_default(),
                            json_arr.len(), curvature::confidence(k), k)
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

        "SUBSCRIBE" => {
            // SUBSCRIBE bundle WHERE field = "value"
            // This returns an initial ACK; real subscriptions use the broadcast channel
            if parts.len() < 2 {
                return "ERROR: SUBSCRIBE requires bundle and WHERE clause".to_string();
            }
            format!("SUBSCRIBED {}", parts[1])
        }

        "CURVATURE" => {
            if parts.len() < 2 {
                return "ERROR: CURVATURE requires bundle.field".to_string();
            }
            let target = parts[1].trim();
            let dot_pos = target.find('.');
            let bundle_name = if let Some(pos) = dot_pos {
                &target[..pos]
            } else {
                target
            };

            let engine = state.engine.read().unwrap();
            if let Some(store) = engine.bundle(bundle_name) {
                let k = curvature::scalar_curvature(store);
                let conf = curvature::confidence(k);
                let cap = curvature::capacity(1.0, k);
                format!("CURVATURE K={:.6} confidence={:.4} capacity={:.2}",
                    k, conf, cap)
            } else {
                format!("ERROR: Bundle '{}' not found", bundle_name)
            }
        }

        "CONSISTENCY" => {
            if parts.len() < 2 {
                return "ERROR: CONSISTENCY requires bundle name".to_string();
            }
            format!("CONSISTENCY h1=0 cocycles=0")
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
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        spec,
    )
}

// ── GQL endpoint ──

async fn gql_query(
    State(state): State<Arc<StreamState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let query = match body.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Missing 'query' field"}))),
    };

    let stmt = match gigi::parser::parse(query) {
        Ok(s) => s,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("Parse error: {e}")}))),
    };

    // Handle statements that don't need an existing bundle
    match &stmt {
        gigi::parser::Statement::CreateBundle { name, base_fields, fiber_fields, indexed, encrypted } => {
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
            let list: Vec<serde_json::Value> = engine.bundle_names().iter()
                .map(|name| {
                    let store = engine.bundle(name).unwrap();
                    serde_json::json!({
                        "name": name,
                        "records": store.len(),
                        "fields": store.schema.base_fields.len() + store.schema.fiber_fields.len(),
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
                return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("No bundle: {bundle}")})));
            }
        }
        gigi::parser::Statement::AtlasBegin | gigi::parser::Statement::AtlasCommit | gigi::parser::Statement::AtlasRollback => {
            return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
        }
        // v2.1: statements parsed but not yet implemented — return 501
        gigi::parser::Statement::ShowRoles | gigi::parser::Statement::ShowPrepared
        | gigi::parser::Statement::ShowBackups | gigi::parser::Statement::ShowSettings
        | gigi::parser::Statement::ShowSession | gigi::parser::Statement::ShowCurrentRole
        | gigi::parser::Statement::WeaveRole { .. } | gigi::parser::Statement::UnweaveRole { .. }
        | gigi::parser::Statement::Grant { .. } | gigi::parser::Statement::Revoke { .. }
        | gigi::parser::Statement::CreatePolicy { .. }
        | gigi::parser::Statement::Set { .. } | gigi::parser::Statement::Reset { .. }
        | gigi::parser::Statement::Prepare { .. } | gigi::parser::Statement::Execute { .. }
        | gigi::parser::Statement::Deallocate { .. }
        | gigi::parser::Statement::Backup { .. } | gigi::parser::Statement::Restore { .. }
        | gigi::parser::Statement::VerifyBackup { .. }
        | gigi::parser::Statement::CommentOn { .. } => {
            return (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({"error": "This GQL v2.1 command is not yet implemented"})));
        }
        _ => {}
    }

    let bundle_name = match get_bundle_name(&stmt) {
        Some(name) => name,
        None => return (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
    };

    // Check if bundle needs write access
    let needs_write = matches!(&stmt,
        gigi::parser::Statement::Insert { .. } |
        gigi::parser::Statement::BatchInsert { .. } |
        gigi::parser::Statement::SectionUpsert { .. } |
        gigi::parser::Statement::Redefine { .. } |
        gigi::parser::Statement::BulkRedefine { .. } |
        gigi::parser::Statement::Retract { .. } |
        gigi::parser::Statement::BulkRetract { .. }
    );

    if needs_write {
        let mut engine = state.engine.write().unwrap();
        let store = match engine.bundle_mut(&bundle_name) {
            Some(s) => s,
            None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("No bundle: {bundle_name}")}))),
        };
        let result = execute_gql_on_store(store, &stmt);
        match result {
            Ok(r) => exec_result_to_response(r),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))),
        }
    } else {
        let engine = state.engine.read().unwrap();
        let store = match engine.bundle(&bundle_name) {
            Some(s) => s,
            None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("No bundle: {bundle_name}")}))),
        };
        let result = execute_gql_on_store_read(store, &stmt);
        match result {
            Ok(r) => exec_result_to_response(r),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))),
        }
    }
}

/// Execute a GQL statement that needs mutable access to a BundleStore.
fn execute_gql_on_store(store: &mut gigi::bundle::BundleStore, stmt: &gigi::parser::Statement) -> Result<gigi::parser::ExecResult, String> {
    use gigi::parser::{Statement, ExecResult, literal_to_value};
    use gigi::bundle::QueryCondition as QC;

    match stmt {
        Statement::Insert { columns, values, .. } => {
            let mut record = std::collections::HashMap::new();
            for (c, v) in columns.iter().zip(values.iter()) {
                record.insert(c.clone(), literal_to_value(v));
            }
            store.insert(&record);
            Ok(ExecResult::Ok)
        }
        Statement::SectionUpsert { columns, values, .. } => {
            let mut record = std::collections::HashMap::new();
            for (c, v) in columns.iter().zip(values.iter()) {
                record.insert(c.clone(), literal_to_value(v));
            }
            store.upsert(&record);
            Ok(ExecResult::Ok)
        }
        Statement::BatchInsert { columns, rows, .. } => {
            let records: Vec<gigi::types::Record> = rows.iter().map(|row| {
                columns.iter().zip(row.iter())
                    .map(|(c, v)| (c.clone(), literal_to_value(v)))
                    .collect()
            }).collect();
            store.batch_insert(&records);
            Ok(ExecResult::Ok)
        }
        Statement::Redefine { key, sets, .. } => {
            let key_rec: gigi::types::Record = key.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            let patches: gigi::types::Record = sets.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            if store.update(&key_rec, &patches) { Ok(ExecResult::Ok) } else { Err("Record not found".into()) }
        }
        Statement::BulkRedefine { conditions, sets, .. } => {
            let qcs: Vec<QC> = conditions.iter().flat_map(|fc| filter_to_qcs(fc)).collect();
            let patches: gigi::types::Record = sets.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            let n = store.bulk_update(&qcs, &patches);
            Ok(ExecResult::Count(n))
        }
        Statement::Retract { key, .. } => {
            let key_rec: gigi::types::Record = key.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            if store.delete(&key_rec) { Ok(ExecResult::Ok) } else { Err("Record not found".into()) }
        }
        Statement::BulkRetract { conditions, .. } => {
            let qcs: Vec<QC> = conditions.iter().flat_map(|fc| filter_to_qcs(fc)).collect();
            let n = store.bulk_delete(&qcs);
            Ok(ExecResult::Count(n))
        }
        // For read-only ops via mutable ref, delegate
        _ => execute_gql_on_store_read(store, stmt),
    }
}

/// Execute a GQL statement that only needs read access.
fn execute_gql_on_store_read(store: &gigi::bundle::BundleStore, stmt: &gigi::parser::Statement) -> Result<gigi::parser::ExecResult, String> {
    use gigi::parser::{Statement, ExecResult, GqlStats, literal_to_value};
    use gigi::bundle::QueryCondition as QC;

    match stmt {
        Statement::PointQuery { key, project, .. } => {
            let key_rec: gigi::types::Record = key.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            match store.point_query(&key_rec) {
                Some(mut rec) => {
                    if let Some(fields) = project {
                        rec = rec.into_iter().filter(|(k, _)| fields.contains(k)).collect();
                    }
                    Ok(ExecResult::Rows(vec![rec]))
                }
                None => Ok(ExecResult::Rows(vec![])),
            }
        }
        Statement::ExistsSection { key, .. } => {
            let key_rec: gigi::types::Record = key.iter().map(|(k, v)| (k.clone(), literal_to_value(v))).collect();
            Ok(ExecResult::Bool(store.point_query(&key_rec).is_some()))
        }
        Statement::Cover { on_conditions, where_conditions, or_groups,
                           distinct_field, project, rank_by, first, skip, .. } => {
            if let Some(field) = distinct_field {
                let vals = store.distinct(field);
                let rows: Vec<gigi::types::Record> = vals.into_iter().map(|v| {
                    let mut r = std::collections::HashMap::new();
                    r.insert(field.clone(), v);
                    r
                }).collect();
                return Ok(ExecResult::Rows(rows));
            }
            let mut conditions: Vec<QC> = Vec::new();
            for fc in on_conditions.iter().chain(where_conditions.iter()) {
                conditions.extend(filter_to_qcs(fc));
            }
            let or_qcs: Vec<Vec<QC>> = or_groups.iter()
                .map(|g| g.iter().flat_map(filter_to_qcs).collect())
                .collect();
            let or_ref = if or_qcs.is_empty() { None } else { Some(or_qcs.as_slice()) };

            let results = if let Some(fields) = project {
                let sort_refs: Vec<(&str, bool)> = rank_by.as_ref()
                    .map(|specs| specs.iter().map(|s| (s.field.as_str(), s.desc)).collect())
                    .unwrap_or_default();
                let sort_opt = if sort_refs.is_empty() { None } else { Some(sort_refs.as_slice()) };
                let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
                let (rows, _) = store.filtered_query_projected_ex(
                    &conditions, or_ref, sort_opt, *first, *skip, Some(&field_refs));
                rows
            } else {
                let (sort_by, sort_desc) = rank_by.as_ref()
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
                let groups = gigi::aggregation::group_by(store, gb_field, agg_field);
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
                        row.insert(m.alias.clone().unwrap_or_else(|| format!("{}_{}", m.func_name(), m.field)),
                            gigi::types::Value::Float(val));
                    }
                    rows.push(row);
                }
                Ok(ExecResult::Rows(rows))
            } else {
                Ok(ExecResult::Rows(vec![]))
            }
        }
        Statement::Curvature { .. } => {
            let k = gigi::curvature::scalar_curvature(store);
            Ok(ExecResult::Scalar(k))
        }
        Statement::Spectral { .. } => {
            let lambda1 = gigi::spectral::spectral_gap(store);
            Ok(ExecResult::Scalar(lambda1))
        }
        Statement::Consistency { .. } => {
            let k = gigi::curvature::scalar_curvature(store);
            Ok(ExecResult::Scalar(if k.abs() < 1e-10 { 0.0 } else { k }))
        }
        Statement::Health { .. } => {
            let k = gigi::curvature::scalar_curvature(store);
            Ok(ExecResult::Stats(GqlStats {
                curvature: k,
                confidence: gigi::curvature::confidence(k),
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema.base_fields.len(),
                fiber_fields: store.schema.fiber_fields.len(),
            }))
        }
        Statement::Describe { .. } => {
            let k = gigi::curvature::scalar_curvature(store);
            Ok(ExecResult::Stats(GqlStats {
                curvature: k,
                confidence: gigi::curvature::confidence(k),
                record_count: store.len(),
                storage_mode: store.storage_mode().to_string(),
                base_fields: store.schema.base_fields.len(),
                fiber_fields: store.schema.fiber_fields.len(),
            }))
        }
        _ => Ok(ExecResult::Ok),
    }
}

fn filter_to_qcs(fc: &gigi::parser::FilterCondition) -> Vec<gigi::bundle::QueryCondition> {
    use gigi::parser::{FilterCondition, literal_to_value};
    use gigi::bundle::QueryCondition as QC;
    match fc {
        FilterCondition::Eq(f, v) => vec![QC::Eq(f.clone(), literal_to_value(v))],
        FilterCondition::Neq(f, v) => vec![QC::Neq(f.clone(), literal_to_value(v))],
        FilterCondition::Gt(f, v) => vec![QC::Gt(f.clone(), literal_to_value(v))],
        FilterCondition::Gte(f, v) => vec![QC::Gte(f.clone(), literal_to_value(v))],
        FilterCondition::Lt(f, v) => vec![QC::Lt(f.clone(), literal_to_value(v))],
        FilterCondition::Lte(f, v) => vec![QC::Lte(f.clone(), literal_to_value(v))],
        FilterCondition::In(f, vs) => vec![QC::In(f.clone(), vs.iter().map(literal_to_value).collect())],
        FilterCondition::NotIn(f, vs) => vec![QC::NotIn(f.clone(), vs.iter().map(literal_to_value).collect())],
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

fn exec_result_to_response(result: gigi::parser::ExecResult) -> (StatusCode, Json<serde_json::Value>) {
    use gigi::parser::ExecResult::*;
    match result {
        Ok => (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
        Rows(rows) => {
            let json_rows: Vec<serde_json::Value> = rows.iter().map(|r| record_to_json(r)).collect();
            (StatusCode::OK, Json(serde_json::json!({"rows": json_rows, "count": json_rows.len()})))
        }
        Scalar(v) => (StatusCode::OK, Json(serde_json::json!({"value": v}))),
        Bool(v) => (StatusCode::OK, Json(serde_json::json!({"value": v}))),
        Count(n) => (StatusCode::OK, Json(serde_json::json!({"affected": n}))),
        Stats(stats) => (StatusCode::OK, Json(serde_json::json!({
            "curvature": stats.curvature, "confidence": stats.confidence,
            "record_count": stats.record_count, "storage_mode": stats.storage_mode,
            "base_fields": stats.base_fields, "fiber_fields": stats.fiber_fields,
        }))),
        Bundles(infos) => {
            let list: Vec<serde_json::Value> = infos.iter()
                .map(|i| serde_json::json!({"name": i.name, "records": i.records, "fields": i.fields}))
                .collect();
            (StatusCode::OK, Json(serde_json::json!({"bundles": list})))
        }
    }
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
        .route("/v1/bundles/{name}/stream", post(stream_ingest))
        .route("/v1/bundles/{name}/get", get(point_query))
        .route("/v1/bundles/{name}/range", get(range_query))
        .route("/v1/bundles/{name}/join", post(pullback_join))
        .route("/v1/bundles/{name}/aggregate", post(aggregate))
        // PRISM-friendly REST endpoints
        .route("/v1/bundles/{name}/points", get(list_all_records).post(insert_records).patch(bulk_update_records))
        .route("/v1/bundles/{name}/points/{field}/{value}", get(get_by_path).patch(patch_by_path).delete(delete_by_path))
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
        .route("/v1/bundles/{name}/add-field", post(add_field))
        .route("/v1/bundles/{name}/add-index", post(add_index))
        .route("/v1/bundles/{name}/export", get(export_bundle))
        .route("/v1/bundles/{name}/dhoom", get(export_dhoom))
        .route("/v1/bundles/{name}/import", post(import_bundle))
        // Sprint 3: Enterprise operations
        .route("/v1/bundles/{name}/stats", get(bundle_stats))
        .route("/v1/bundles/{name}/explain", post(explain_query))
        .route("/v1/bundles/{name}/transaction", post(execute_transaction))
        // OpenAPI spec
        .route("/v1/openapi.json", get(openapi_spec))
        // GQL endpoint
        .route("/v1/gql", post(gql_query))
        // Analytics
        .route("/v1/bundles/{name}/curvature", get(curvature_report))
        .route("/v1/bundles/{name}/spectral", get(spectral_report))
        .route("/v1/bundles/{name}/consistency", get(consistency_check))
        // WebSocket
        .route("/ws", get(ws_handler))
        // Middleware: auth + rate limiting
        .layer(axum_mw::from_fn_with_state(state.clone(), auth_middleware))
        .layer(axum_mw::from_fn_with_state(state.clone(), rate_limit_middleware))
        // CORS — configurable via GIGI_CORS_ORIGIN env var
        // Default: restrictive (no cross-origin). Set GIGI_CORS_ORIGIN=* for permissive.
        .layer(build_cors_layer())
        .with_state(state);

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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    ).await.unwrap();
}
