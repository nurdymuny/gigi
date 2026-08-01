//! Patterns / hunt HTTP-surface free logic (stream-extraction phase 2,
//! family 3; see EXTRACTION_MAP.md). Request/response structs and the
//! JSON↔GQL translation bodies for the `/v1/patterns` + `/v1/bundles/
//! {bundle}/hunt` endpoints, moved verbatim out of
//! `src/bin/gigi_stream.rs` — the handlers there stay as thin wrappers
//! that acquire the engine write lock and call into this module. The
//! only cross-family touch is `crate::wire::value_to_json`.

use crate::stream_shared::ErrorResponse;
use crate::wire::value_to_json;
use axum::{http::StatusCode, Json};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PatternListEntry {
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct DefinePatternRequest {
    pub name: String,
    /// Predicate body — the part after `AS`. Example: `"field_a = 1 AND field_b > 5"`.
    pub predicate: String,
    /// Optional WEIGHT arithmetic body. Example: `"field_a * 3 + field_b * 2"`.
    #[serde(default)]
    pub weight: Option<String>,
    /// Optional USING field list.
    #[serde(default)]
    pub using: Vec<String>,
    /// If true, equivalent to `DEFINE OR REPLACE PATTERN`.
    #[serde(default)]
    pub replace: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct HuntRequest {
    pub pattern: String,
    #[serde(default)]
    pub excluding: Vec<String>,
    #[serde(default)]
    pub top: Option<usize>,
    #[serde(default)]
    pub project: Vec<String>,
    // ─── v0.2 additions (additive; old clients send none, get v0.1 shape) ─
    /// Patterns v0.2 — when set ≥ 1, HUNT returns the verdict envelope
    /// (sat/unsat/near_miss) instead of the bare row array. When 0 or
    /// absent, the v0.1 array shape is preserved for backwards compat.
    #[serde(default)]
    pub near_miss_budget: Option<usize>,
    /// Patterns v0.2 — attach `_explain` (WEIGHT decomposition tree) to
    /// each sat row. Forces the envelope response.
    #[serde(default)]
    pub explain: bool,
    /// Patterns v0.2 — attach `_repair_menu` to each near-miss row.
    /// Forces the envelope response.
    #[serde(default)]
    pub include_repair_menu: bool,
    /// Patterns v0.2 — per-field relaxation costs (default 1.0/field).
    /// Only consulted when `include_repair_menu` is true.
    #[serde(default)]
    pub relaxation_costs: std::collections::HashMap<String, f64>,
}

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
pub fn list_patterns(
    engine: &mut crate::Engine,
) -> Result<Json<Vec<PatternListEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let stmt = crate::parser::parse("SHOW PATTERNS").map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("internal parse: {e}"),
            }),
        )
    })?;
    let result = crate::parser::execute(engine, &stmt).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("execute: {e}"),
            }),
        )
    })?;
    match result {
        crate::parser::ExecResult::Rows(rows) => {
            let entries: Vec<PatternListEntry> = rows
                .into_iter()
                .filter_map(|row| match row.get("name") {
                    Some(crate::types::Value::Text(n)) => Some(PatternListEntry { name: n.clone() }),
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
pub fn define_pattern(
    engine: &mut crate::Engine,
    req: DefinePatternRequest,
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
    let stmt = crate::parser::parse(&sql).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("parse: {e}"),
            }),
        )
    })?;
    crate::parser::execute(engine, &stmt).map_err(|e| {
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
pub fn drop_pattern(
    engine: &mut crate::Engine,
    name: String,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let sql = format!("DROP PATTERN {name}");
    let stmt = crate::parser::parse(&sql).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("parse: {e}"),
            }),
        )
    })?;
    crate::parser::execute(engine, &stmt).map_err(|e| {
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
pub fn hunt(
    engine: &mut crate::Engine,
    bundle: String,
    req: HuntRequest,
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
        let args = crate::parser::HuntV2Args {
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
        let env = crate::parser::hunt_v2_orchestrate(engine, &args).map_err(|e| {
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
    let stmt = crate::parser::parse(&sql).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("parse: {e}"),
            }),
        )
    })?;
    let result = crate::parser::execute(engine, &stmt).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("hunt: {e}"),
            }),
        )
    })?;
    match result {
        crate::parser::ExecResult::Rows(rows) => {
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
pub fn envelope_to_json(env: crate::parser::HuntV2Envelope) -> serde_json::Value {
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
pub fn hunt_row_to_json(row: crate::types::Record) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    let mut score: Option<crate::types::Value> = None;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    /// SCJ Round 10 §5(a): `_score` must always be the LAST key in the
    /// serialized HUNT row JSON so TUI consumers can render score columns
    /// without inspecting the schema. Verified at the serialization layer
    /// since JSON object key order is the contract bit on the wire.
    #[test]
    fn hunt_row_to_json_pins_score_last_when_present() {
        let mut row = crate::types::Record::new();
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
    #[test]
    fn hunt_row_to_json_does_not_inject_score_when_absent() {
        let mut row = crate::types::Record::new();
        row.insert("alpha".to_string(), Value::Integer(1));
        row.insert("beta".to_string(), Value::Integer(2));
        let json = hunt_row_to_json(row);
        let serialized = serde_json::to_string(&json).expect("serialize");
        assert!(
            !serialized.contains("_score"),
            "no _score in input → no _score in output: {serialized}"
        );
    }
}
