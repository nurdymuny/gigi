//! GIGI Edge — Local-first geometric database.
//!
//! Runs locally with the same REST API as GIGI Stream.
//! Stores to disk via WAL. Syncs to a remote GIGI Stream when connected.
//!
//! Usage:
//!   gigi-edge                           # Start local server on port 3143
//!   gigi-edge --data ./my-data          # Custom data directory
//!   gigi-edge --remote http://host:3142 # Configure sync target
//!   gigi-edge sync                      # One-shot sync to remote

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tower_http::cors::CorsLayer;

use gigi::aggregation;
use gigi::curvature;
use gigi::edge::EdgeEngine;
use gigi::join;
use gigi::spectral;
use gigi::types::{BundleSchema, FieldDef, FieldType, Value};

// ── CLI ──

#[derive(Parser)]
#[command(
    name = "gigi-edge",
    about = "GIGI Edge — Local-first geometric database"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Data directory for local storage
    #[arg(long, default_value = ".gigi-data")]
    data: String,

    /// Remote GIGI Stream server URL
    #[arg(long)]
    remote: Option<String>,

    /// API key for remote server
    #[arg(long)]
    api_key: Option<String>,

    /// Port to listen on
    #[arg(short, long, default_value = "3143")]
    port: u16,
}

#[derive(Subcommand)]
enum Commands {
    /// Sync local data with remote GIGI Stream
    Sync,
    /// Show local database status
    Status,
    /// Compact the local WAL
    Compact,
}

// ── Shared State ──

struct EdgeState {
    engine: RwLock<EdgeEngine>,
}

// ── API Types ──

#[derive(Deserialize)]
struct CreateBundleRequest {
    name: String,
    schema: SchemaSpec,
}

#[derive(Deserialize)]
struct SchemaSpec {
    fields: HashMap<String, String>,
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
struct AggregateRequest {
    group_by: String,
    field: String,
}

#[derive(Deserialize)]
struct JoinRequest {
    right_bundle: String,
    left_field: String,
    right_field: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    engine: &'static str,
    version: &'static str,
    mode: &'static str,
    bundles: usize,
    total_records: usize,
    pending_sync: usize,
    last_sync: u64,
}

// ── Helpers ──

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
        Value::Binary(b) => {
            use base64::Engine as _;
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
        }
        Value::Vector(v) => {
            serde_json::Value::Array(v.iter().map(|x| serde_json::json!(x)).collect())
        }
    }
}

fn record_to_json(record: &HashMap<String, Value>) -> serde_json::Value {
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

// ── REST Handlers ──

async fn health(State(state): State<Arc<EdgeState>>) -> Json<HealthResponse> {
    let engine = state.engine.read().unwrap();
    Json(HealthResponse {
        status: "ok",
        engine: "gigi-edge",
        version: "0.1.0",
        mode: "local-first",
        bundles: engine.bundle_names().len(),
        total_records: engine.total_records(),
        pending_sync: engine.pending_ops(),
        last_sync: engine.last_sync_time(),
    })
}

async fn create_bundle(
    State(state): State<Arc<EdgeState>>,
    Json(req): Json<CreateBundleRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let mut schema = BundleSchema::new(&req.name);

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

    for idx_field in &req.schema.indexed {
        schema = schema.index(idx_field);
    }
    for key in &req.schema.keys {
        schema = schema.index(key);
    }

    let mut engine = state.engine.write().unwrap();
    match engine.create_bundle(schema) {
        Ok(()) => Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({
                "status": "created",
                "bundle": req.name,
                "mode": "local",
                "pending_sync": engine.pending_ops(),
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn drop_bundle(
    State(_state): State<Arc<EdgeState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Engine doesn't support drop yet — return not implemented
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": format!("Drop '{}' not yet implemented", name)})),
    )
}

async fn insert_records(
    State(state): State<Arc<EdgeState>>,
    Path(name): Path<String>,
    Json(req): Json<InsertRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut engine = state.engine.write().unwrap();

    let count = req.records.len();
    for json_record in &req.records {
        if let Some(obj) = json_record.as_object() {
            let mut record: HashMap<String, Value> = HashMap::new();
            for (k, v) in obj {
                record.insert(k.clone(), json_to_value(v));
            }
            if let Err(e) = engine.insert(&name, &record) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e.to_string()})),
                ));
            }
        }
    }

    // Compute curvature
    let (k, conf) = engine.curvature(&name).unwrap_or((0.0, 1.0));

    Ok(Json(serde_json::json!({
        "status": "inserted",
        "count": count,
        "total": engine.bundle(&name).map(|b| b.len()).unwrap_or(0),
        "curvature": (k * 1000.0).round() / 1000.0,
        "confidence": (conf * 100.0).round() / 100.0,
        "pending_sync": engine.pending_ops(),
        "mode": "local",
    })))
}

async fn point_query(
    State(state): State<Arc<EdgeState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let engine = state.engine.read().unwrap();

    let mut key: HashMap<String, Value> = HashMap::new();
    for (k, v) in &params {
        if let Ok(n) = v.parse::<i64>() {
            key.insert(k.clone(), Value::Integer(n));
        } else if let Ok(f) = v.parse::<f64>() {
            key.insert(k.clone(), Value::Float(f));
        } else {
            key.insert(k.clone(), Value::Text(v.clone()));
        }
    }

    match engine.get(&name, &key) {
        Ok(Some(record)) => {
            let (k_val, conf) = engine.curvature(&name).unwrap_or((0.0, 1.0));
            let capacity = if k_val > 0.0 {
                1.0 / k_val
            } else {
                f64::INFINITY
            };

            Ok(Json(serde_json::json!({
                "data": record_to_json(&record),
                "meta": {
                    "confidence": (conf * 100.0).round() / 100.0,
                    "curvature": (k_val * 1000.0).round() / 1000.0,
                    "capacity": (capacity * 100.0).round() / 100.0,
                    "mode": "local",
                }
            })))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Record not found"})),
        )),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn range_query(
    State(state): State<Arc<EdgeState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let engine = state.engine.read().unwrap();

    let (field, value_str) = match params.iter().next() {
        Some((k, v)) => (k.clone(), v.clone()),
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No query parameters"})),
            ));
        }
    };

    let value = if let Ok(n) = value_str.parse::<i64>() {
        Value::Integer(n)
    } else if let Ok(f) = value_str.parse::<f64>() {
        Value::Float(f)
    } else {
        Value::Text(value_str)
    };

    match engine.range(&name, &field, &[value]) {
        Ok(records) => {
            let results: Vec<serde_json::Value> = records.iter().map(record_to_json).collect();
            let (k_val, conf) = engine.curvature(&name).unwrap_or((0.0, 1.0));

            Ok(Json(serde_json::json!({
                "data": results,
                "count": results.len(),
                "meta": {
                    "confidence": (conf * 100.0).round() / 100.0,
                    "curvature": (k_val * 1000.0).round() / 1000.0,
                    "mode": "local",
                }
            })))
        }
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn curvature_report(
    State(state): State<Arc<EdgeState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let engine = state.engine.read().unwrap();
    let store = match engine.bundle(&name) {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Bundle '{}' not found", name)})),
            ));
        }
    };

    let k = store.scalar_curvature();
    let conf = curvature::confidence(k);
    let capacity = if k > 0.0 { 1.0 / k } else { f64::INFINITY };

    let mut per_field = Vec::new();
    for (field_name, stats) in store.field_stats() {
        let range = stats.range();
        let var = stats.variance();
        let fk = if range > 0.0 {
            var / (range * range)
        } else {
            0.0
        };
        per_field.push(serde_json::json!({
            "field": field_name,
            "variance": (var * 1000.0).round() / 1000.0,
            "range": (range * 100.0).round() / 100.0,
            "k": (fk * 1000.0).round() / 1000.0,
        }));
    }

    Ok(Json(serde_json::json!({
        "K": (k * 1000.0).round() / 1000.0,
        "curvature": (k * 1000.0).round() / 1000.0,
        "confidence": (conf * 100.0).round() / 100.0,
        "capacity": (capacity * 100.0).round() / 100.0,
        "per_field": per_field,
        "mode": "local",
    })))
}

async fn spectral_report(
    State(state): State<Arc<EdgeState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let engine = state.engine.read().unwrap();
    let store = match engine.bundle(&name) {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Bundle '{}' not found", name)})),
            ));
        }
    };

    let lambda1 = store.as_heap().map(spectral::spectral_gap).unwrap_or(0.0);
    let diameter = store.as_heap().map(spectral::graph_diameter).unwrap_or(0);
    let cap = store.as_heap().map(spectral::spectral_capacity).unwrap_or(0.0);

    Ok(Json(serde_json::json!({
        "lambda1": (lambda1 * 10000.0).round() / 10000.0,
        "diameter": diameter,
        "spectral_capacity": (cap * 100.0).round() / 100.0,
        "mode": "local",
    })))
}

async fn consistency_check(
    State(state): State<Arc<EdgeState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let engine = state.engine.read().unwrap();
    let store = match engine.bundle(&name) {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Bundle '{}' not found", name)})),
            ));
        }
    };

    // Čech cohomology requires bitmap access — only available on heap bundles
    let heap = match store.as_heap() {
        Some(s) => s,
        None => {
            return Ok(Json(serde_json::json!({
                "h1": 0,
                "cocycles": [],
                "mode": "mmap (bitmap access unavailable)",
            })));
        }
    };

    // Check Čech cohomology: for each pair of overlapping open sets,
    // verify sections agree on intersection.
    let mut h1 = 0;
    let mut cocycles = Vec::new();

    for field_name in &heap.schema.indexed_fields {
        let values = heap.indexed_values(field_name);
        for i in 0..values.len() {
            for j in (i + 1)..values.len() {
                let bm_i = heap.field_bitmap(field_name, &values[i]);
                let bm_j = heap.field_bitmap(field_name, &values[j]);
                if let (Some(bi), Some(bj)) = (bm_i, bm_j) {
                    let overlap = bi & bj;
                    if !overlap.is_empty() {
                        // Check section agreement on overlap
                        let recs_i = heap.range_query(field_name, &[values[i].clone()]);
                        let recs_j = heap.range_query(field_name, &[values[j].clone()]);
                        // If same base point has different fiber values → cocycle
                        let map_i: HashMap<_, _> = recs_i
                            .iter()
                            .map(|r| {
                                let key: Vec<_> = heap
                                    .schema
                                    .base_fields
                                    .iter()
                                    .map(|f| r.get(&f.name).cloned().unwrap_or(Value::Null))
                                    .collect();
                                (key, r)
                            })
                            .collect();
                        for r_j in &recs_j {
                            let key: Vec<_> = heap
                                .schema
                                .base_fields
                                .iter()
                                .map(|f| r_j.get(&f.name).cloned().unwrap_or(Value::Null))
                                .collect();
                            if let Some(r_i) = map_i.get(&key) {
                                for ff in &heap.schema.fiber_fields {
                                    let vi = r_i.get(&ff.name);
                                    let vj = r_j.get(&ff.name);
                                    if vi != vj {
                                        h1 += 1;
                                        cocycles.push(serde_json::json!({
                                            "field": ff.name,
                                            "values": [
                                                format!("{:?}", values[i]),
                                                format!("{:?}", values[j]),
                                            ],
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "h1": h1,
        "cocycles": cocycles,
        "mode": "local",
    })))
}

async fn aggregate(
    State(state): State<Arc<EdgeState>>,
    Path(name): Path<String>,
    Json(req): Json<AggregateRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let engine = state.engine.read().unwrap();
    let store = match engine.bundle(&name) {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Bundle '{}' not found", name)})),
            ));
        }
    };

    let groups = match store.as_heap() {
        Some(s) => aggregation::group_by(s, &req.group_by, &req.field),
        None => std::collections::HashMap::new(),
    };
    let mut result = serde_json::Map::new();

    for (group_val, agg) in &groups {
        result.insert(
            format!("{}", group_val),
            serde_json::json!({
                "count": agg.count,
                "sum": (agg.sum * 100.0).round() / 100.0,
                "avg": (agg.avg() * 100.0).round() / 100.0,
                "min": agg.min,
                "max": agg.max,
            }),
        );
    }

    Ok(Json(serde_json::json!({"groups": result})))
}

async fn pullback_join(
    State(state): State<Arc<EdgeState>>,
    Path(name): Path<String>,
    Json(req): Json<JoinRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let engine = state.engine.read().unwrap();
    let left = match engine.bundle(&name) {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Bundle '{}' not found", name)})),
            ));
        }
    };
    let right = match engine.bundle(&req.right_bundle) {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(
                    serde_json::json!({"error": format!("Bundle '{}' not found", req.right_bundle)}),
                ),
            ));
        }
    };

    let joined = match (left.as_heap(), right.as_heap()) {
        (Some(l), Some(r)) => join::pullback_join(l, r, &req.left_field, &req.right_field),
        _ => Vec::new(),
    };
    let results: Vec<serde_json::Value> = joined
        .iter()
        .filter_map(|(l, r)| {
            r.as_ref().map(|right_rec| {
                serde_json::json!({
                    "left": record_to_json(l),
                    "right": record_to_json(right_rec),
                })
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "data": results,
        "count": results.len(),
    })))
}

async fn sync_handler(
    State(state): State<Arc<EdgeState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut engine = state.engine.write().unwrap();
    match engine.sync() {
        Ok(report) => Ok(Json(serde_json::json!({
            "status": "synced",
            "pushed": report.pushed,
            "pulled": report.pulled,
            "h1": report.h1,
            "conflicts": report.conflicts.len(),
            "timestamp": report.timestamp,
        }))),
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": e.to_string(),
                "hint": "Is the remote GIGI Stream server running?",
            })),
        )),
    }
}

async fn status_handler(State(state): State<Arc<EdgeState>>) -> Json<serde_json::Value> {
    let engine = state.engine.read().unwrap();

    let bundles: Vec<serde_json::Value> = engine
        .bundle_names()
        .iter()
        .map(|name| {
            let records = engine.bundle(name).map(|b| b.len()).unwrap_or(0);
            let (k, conf) = engine.curvature(name).unwrap_or((0.0, 1.0));
            serde_json::json!({
                "name": name,
                "records": records,
                "curvature": (k * 1000.0).round() / 1000.0,
                "confidence": (conf * 100.0).round() / 100.0,
            })
        })
        .collect();

    Json(serde_json::json!({
        "mode": "local-first",
        "data_dir": engine.data_dir().to_string_lossy(),
        "bundles": bundles,
        "total_records": engine.total_records(),
        "pending_sync": engine.pending_ops(),
        "last_sync": engine.last_sync_time(),
    }))
}

// ── Main ──

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let data_dir = std::path::PathBuf::from(&cli.data);

    // Open local engine
    let mut engine = EdgeEngine::open(&data_dir).expect("Failed to open local database");

    // Configure remote if provided
    if let Some(ref remote) = cli.remote {
        engine.set_remote(remote, cli.api_key.as_deref());
    }

    // Handle subcommands
    match cli.command {
        Some(Commands::Sync) => {
            if cli.remote.is_none() {
                eprintln!("Error: --remote URL required for sync");
                std::process::exit(1);
            }
            println!("Syncing to {}...", cli.remote.as_deref().unwrap());
            match engine.sync() {
                Ok(report) => {
                    println!("  Pushed: {} operations", report.pushed);
                    println!("  Pulled: {} records", report.pulled);
                    println!(
                        "  H1: {} ({})",
                        report.h1,
                        if report.h1 == 0 {
                            "clean merge"
                        } else {
                            "CONFLICTS"
                        }
                    );
                    if !report.conflicts.is_empty() {
                        println!("  Conflicts:");
                        for c in &report.conflicts {
                            println!(
                                "    - {}.{}: local={}, remote={}",
                                c.bundle, c.field, c.local_value, c.remote_value
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Sync failed: {}", e);
                    std::process::exit(1);
                }
            }
            return;
        }

        Some(Commands::Status) => {
            println!("GIGI Edge — Local Database Status");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("  Data dir:     {}", data_dir.display());
            println!("  Bundles:      {}", engine.bundle_names().len());
            println!("  Total records: {}", engine.total_records());
            println!("  Pending sync: {} ops", engine.pending_ops());
            println!(
                "  Last sync:    {}",
                if engine.last_sync_time() == 0 {
                    "never".to_string()
                } else {
                    format!("{} ms", engine.last_sync_time())
                }
            );

            for name in engine.bundle_names() {
                if let Some(store) = engine.bundle(name) {
                    let (k, conf) = engine.curvature(name).unwrap_or((0.0, 1.0));
                    println!("\n  Bundle: {}", name);
                    println!("    Records:    {}", store.len());
                    println!("    Curvature:  K={:.4}, confidence={:.4}", k, conf);
                    println!(
                        "    Fields:     {}",
                        store.schema().all_field_names().join(", ")
                    );
                }
            }
            return;
        }

        Some(Commands::Compact) => {
            println!("Compacting WAL...");
            match engine.compact() {
                Ok(()) => println!("WAL compacted successfully."),
                Err(e) => {
                    eprintln!("Compaction failed: {}", e);
                    std::process::exit(1);
                }
            }
            return;
        }

        None => {
            // Start server mode
        }
    }

    // ── Server Mode ──

    let state = Arc::new(EdgeState {
        engine: RwLock::new(engine),
    });

    let app = Router::new()
        // Health & status
        .route("/v1/health", get(health))
        .route("/v1/status", get(status_handler))
        .route("/v1/sync", post(sync_handler))
        // Bundle CRUD
        .route("/v1/bundles", post(create_bundle))
        .route("/v1/bundles/{name}", delete(drop_bundle))
        // Data operations (same API as GIGI Stream)
        .route("/v1/bundles/{name}/insert", post(insert_records))
        .route("/v1/bundles/{name}/get", get(point_query))
        .route("/v1/bundles/{name}/range", get(range_query))
        .route("/v1/bundles/{name}/join", post(pullback_join))
        .route("/v1/bundles/{name}/aggregate", post(aggregate))
        // Geometric analysis
        .route("/v1/bundles/{name}/curvature", get(curvature_report))
        .route("/v1/bundles/{name}/spectral", get(spectral_report))
        .route("/v1/bundles/{name}/consistency", get(consistency_check))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cli.port);

    println!(
        r#"
╔═══════════════════════════════════════════════════╗
║           GIGI Edge — Local-First Engine          ║
╠═══════════════════════════════════════════════════╣
║                                                   ║
║  REST API:  http://localhost:{:<5}                ║
║  Data dir:  {:<38}║
║  Mode:      local-first (offline-capable)         ║
║  Remote:    {:<38}║
║                                                   ║
║  Endpoints:                                       ║
║    POST   /v1/bundles            Create bundle    ║
║    POST   /v1/bundles/:n/insert  Insert records   ║
║    GET    /v1/bundles/:n/get     Point query O(1)  ║
║    GET    /v1/bundles/:n/range   Range query       ║
║    POST   /v1/bundles/:n/join   Pullback join     ║
║    POST   /v1/bundles/:n/aggregate  Fiber integral║
║    GET    /v1/bundles/:n/curvature  K report      ║
║    GET    /v1/bundles/:n/spectral   Spectral gap  ║
║    GET    /v1/bundles/:n/consistency Čech H¹      ║
║    POST   /v1/sync               Sync to remote   ║
║    GET    /v1/status             Local status      ║
║    GET    /v1/health             Health check      ║
║                                                   ║
╚═══════════════════════════════════════════════════╝
"#,
        cli.port,
        data_dir.display(),
        cli.remote.as_deref().unwrap_or("(not configured)")
    );

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
