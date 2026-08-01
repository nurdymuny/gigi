//! Wire converters shared between the gigi-stream binary and lib route
//! families (stream-extraction phase 2+; see EXTRACTION_MAP.md
//! "Cross-family shared modules": the wire-converter module hoists in
//! stages ahead of family 9). `value_to_json` is hoisted first (family 3:
//! `patterns::http::hunt_row_to_json` consumes it); the rest of the
//! converter set (`json_to_value`, `record_to_json`, `schema_coerce`, …)
//! follows when family 9 (core bundle CRUD) is extracted. Moved text is
//! verbatim from `src/bin/gigi_stream.rs` — the only edits are
//! `gigi::` → `crate::` paths and `pub` visibility.

use crate::types::Value;

pub fn value_to_json(v: &Value) -> serde_json::Value {
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
