//! GIGI Convert HTTP API Server
//!
//! POST /v1/convert   — JSON → DHOOM
//! POST /v1/decode    — DHOOM → JSON
//! POST /v1/profile   — JSON → geometric profile
//! GET  /v1/health    — health check

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ConvertRequest {
    input: Vec<serde_json::Value>,
    #[serde(default = "default_name")]
    name: String,
    #[serde(default)]
    options: ConvertOptions,
}

#[derive(Deserialize, Default)]
struct ConvertOptions {
    #[serde(default)]
    profile: bool,
    #[serde(default = "default_format")]
    #[allow(dead_code)]
    format: String,
}

fn default_name() -> String {
    "data".to_string()
}
fn default_format() -> String {
    "dhoom".to_string()
}

#[derive(Serialize)]
struct ConvertResponse {
    dhoom: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<ProfileResponse>,
}

#[derive(Serialize)]
struct ProfileResponse {
    records: usize,
    fields: usize,
    arithmetic_fields: usize,
    default_fields: usize,
    compression_pct: f64,
    token_savings_pct: f64,
    json_chars: usize,
    dhoom_chars: usize,
    curvature: std::collections::HashMap<String, CurvatureEntry>,
}

#[derive(Serialize)]
struct CurvatureEntry {
    k: f64,
    confidence: f64,
}

#[derive(Serialize)]
struct DecodeResponse {
    records: Vec<serde_json::Value>,
    collection: String,
    count: usize,
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
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        engine: "gigi-convert",
        version: "0.1.0",
    })
}

async fn convert(
    Json(req): Json<ConvertRequest>,
) -> Result<Json<ConvertResponse>, (StatusCode, Json<ErrorResponse>)> {
    if req.input.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Input array is empty".to_string(),
            }),
        ));
    }

    let result = gigi::convert::encode_json(&req.input, &req.name);

    let profile = if req.options.profile {
        let p = gigi::convert::profile(&req.input, &req.name);
        let mut curv_map = std::collections::HashMap::new();
        for (field, k, conf) in &p.curvature {
            curv_map.insert(
                field.clone(),
                CurvatureEntry {
                    k: *k,
                    confidence: *conf,
                },
            );
        }
        Some(ProfileResponse {
            records: p.records,
            fields: p.fields,
            arithmetic_fields: p.arithmetic_fields.len(),
            default_fields: p.default_fields.len(),
            compression_pct: p.compression_pct,
            token_savings_pct: p.token_savings_pct,
            json_chars: p.json_chars,
            dhoom_chars: p.dhoom_chars,
            curvature: curv_map,
        })
    } else {
        None
    };

    Ok(Json(ConvertResponse {
        dhoom: result.dhoom,
        profile,
    }))
}

async fn decode(body: String) -> Result<Json<DecodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Empty DHOOM input".to_string(),
            }),
        ));
    }

    match gigi::dhoom::decode_legacy(&body) {
        Ok(parsed) => {
            let json_records = gigi::dhoom::dhoom_to_json_array(&parsed);
            let count = json_records.len();
            Ok(Json(DecodeResponse {
                records: json_records,
                collection: parsed.collection,
                count,
            }))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("DHOOM parse error: {}", e),
            }),
        )),
    }
}

async fn profile_endpoint(
    Json(req): Json<ConvertRequest>,
) -> Result<Json<ProfileResponse>, (StatusCode, Json<ErrorResponse>)> {
    if req.input.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Input array is empty".to_string(),
            }),
        ));
    }

    let p = gigi::convert::profile(&req.input, &req.name);
    let mut curv_map = std::collections::HashMap::new();
    for (field, k, conf) in &p.curvature {
        curv_map.insert(
            field.clone(),
            CurvatureEntry {
                k: *k,
                confidence: *conf,
            },
        );
    }
    Ok(Json(ProfileResponse {
        records: p.records,
        fields: p.fields,
        arithmetic_fields: p.arithmetic_fields.len(),
        default_fields: p.default_fields.len(),
        compression_pct: p.compression_pct,
        token_savings_pct: p.token_savings_pct,
        json_chars: p.json_chars,
        dhoom_chars: p.dhoom_chars,
        curvature: curv_map,
    }))
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3141".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let app = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/convert", post(convert))
        .route("/v1/decode", post(decode))
        .route("/v1/profile", post(profile_endpoint));

    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║       GIGI Convert API Server            ║");
    eprintln!("║       http://{}                ║", addr);
    eprintln!("╠══════════════════════════════════════════╣");
    eprintln!("║  POST /v1/convert  — JSON → DHOOM        ║");
    eprintln!("║  POST /v1/decode   — DHOOM → JSON        ║");
    eprintln!("║  POST /v1/profile  — Geometric profile    ║");
    eprintln!("║  GET  /v1/health   — Health check         ║");
    eprintln!("╚══════════════════════════════════════════╝");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
