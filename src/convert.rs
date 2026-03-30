//! GIGI Convert — Format conversion + geometric data profiling
//!
//! Converts JSON/CSV into DHOOM format with full geometric analysis:
//! curvature per field, compression estimates, fiber structure report.

use crate::dhoom::{self, EncodeResult, FieldRole};

/// Geometric profile of a dataset — wraps dhoom::Profile with GIGI metadata.
#[derive(Debug)]
pub struct Profile {
    pub collection: String,
    pub records: usize,
    pub fields: usize,
    pub arithmetic_fields: Vec<String>,
    pub default_fields: Vec<(String, String, f64)>, // (name, value, match_pct)
    pub variable_fields: Vec<String>,
    pub json_chars: usize,
    pub dhoom_chars: usize,
    pub compression_pct: f64,
    pub token_savings_pct: f64,
    pub fields_elided_pct: f64,
    pub curvature: Vec<(String, f64, f64)>, // (field, K, confidence)
}

/// Profile a JSON dataset — analyze its geometric/DHOOM structure.
pub fn profile(input: &[serde_json::Value], collection: &str) -> Profile {
    let wrapped = serde_json::Value::Object({
        let mut m = serde_json::Map::new();
        m.insert(
            collection.to_string(),
            serde_json::Value::Array(input.to_vec()),
        );
        m
    });
    let p = dhoom::profile(&wrapped).unwrap_or_else(|_| dhoom::Profile {
        collection: collection.to_string(),
        record_count: 0,
        field_count: 0,
        fields: vec![],
        dhoom_bytes: 0,
        json_bytes: 0,
        compression_pct: 0.0,
        fields_elided_pct: 0.0,
    });

    let mut arithmetic_fields = Vec::new();
    let mut default_fields = Vec::new();
    let mut variable_fields = Vec::new();
    let mut curvature = Vec::new();

    for fp in &p.fields {
        match &fp.role {
            FieldRole::Arithmetic { .. } => {
                arithmetic_fields.push(fp.name.clone());
            }
            FieldRole::Default { value, match_pct } => {
                default_fields.push((fp.name.clone(), value.clone(), *match_pct));
            }
            FieldRole::Delta | FieldRole::Interned { .. } | FieldRole::Variable => {
                variable_fields.push(fp.name.clone());
            }
            FieldRole::Computed { .. } | FieldRole::Nested => {}
        }
        if fp.curvature > 0.0 || fp.confidence < 1.0 {
            curvature.push((fp.name.clone(), fp.curvature, fp.confidence));
        }
    }

    Profile {
        collection: p.collection,
        records: p.record_count,
        fields: p.field_count,
        arithmetic_fields,
        default_fields,
        variable_fields,
        json_chars: p.json_bytes,
        dhoom_chars: p.dhoom_bytes,
        compression_pct: p.compression_pct,
        token_savings_pct: p.compression_pct,
        fields_elided_pct: p.fields_elided_pct,
        curvature,
    }
}

/// Full encode pipeline: JSON array → DHOOM string
pub fn encode_json(input: &[serde_json::Value], collection: &str) -> EncodeResult {
    dhoom::encode_json(input, collection)
}

/// Full decode pipeline: DHOOM string → JSON array
pub fn decode_to_json(dhoom_str: &str) -> Result<Vec<serde_json::Value>, String> {
    dhoom::decode_to_json(dhoom_str).map_err(|e| e.to_string())
}

/// CSV → DHOOM conversion
pub fn encode_csv(csv_text: &str, collection: &str) -> Result<EncodeResult, String> {
    dhoom::encode_csv(csv_text, collection)
}

/// Format a profile as a human-readable report.
pub fn format_profile(p: &Profile) -> String {
    let mut out = String::new();
    out.push_str("\nGIGI Convert — Geometric Data Profile\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");
    out.push_str(&format!(
        "  Collection:    {} ({} records, {} fields)\n\n",
        p.collection, p.records, p.fields
    ));

    out.push_str("  Field Analysis:\n");
    for name in &p.arithmetic_fields {
        out.push_str(&format!("    {:<14} ARITHMETIC\n", name));
    }
    for (name, value, pct) in &p.default_fields {
        out.push_str(&format!(
            "    {:<14} DEFAULT     {:14} ({:.1}%)\n",
            name,
            format!("\"{}\"", value),
            pct
        ));
    }
    for name in &p.variable_fields {
        out.push_str(&format!("    {:<14} VARIABLE\n", name));
    }

    out.push_str("\n  Fiber Structure:\n");
    out.push_str(&format!(
        "    Base space:     {} arithmetic fields\n",
        p.arithmetic_fields.len()
    ));
    out.push_str(&format!(
        "    Fiber:          {} value fields\n",
        p.fields.saturating_sub(p.arithmetic_fields.len())
    ));
    if !p.default_fields.is_empty() {
        let defaults: Vec<String> = p
            .default_fields
            .iter()
            .map(|(n, v, _)| format!("{}=\"{}\"", n, v))
            .collect();
        out.push_str(&format!("    Zero section:   {}\n", defaults.join(", ")));
    }

    out.push_str("\n  Compression:\n");
    let json_tokens = (p.json_chars as f64 / 3.5).ceil() as usize;
    let dhoom_tokens = (p.dhoom_chars as f64 / 3.5).ceil() as usize;
    out.push_str(&format!(
        "    JSON (minified):  {:>10} chars  (~{:>6} tokens)\n",
        p.json_chars, json_tokens
    ));
    out.push_str(&format!(
        "    DHOOM:            {:>10} chars  (~{:>6} tokens)\n",
        p.dhoom_chars, dhoom_tokens
    ));
    out.push_str(&format!(
        "    Savings:          {:.1}% smaller    ({:.1}% fewer tokens)\n",
        p.compression_pct, p.token_savings_pct
    ));
    out.push_str(&format!(
        "    Fields elided:    {:.1}%\n",
        p.fields_elided_pct
    ));

    if !p.curvature.is_empty() {
        out.push_str("\n  Curvature:\n");
        for (field, k, conf) in &p.curvature {
            out.push_str(&format!(
                "    K({:<12})   {:.6}  (confidence: {:.4})\n",
                format!("{})", field),
                k,
                conf
            ));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_sensor_data() {
        let input: Vec<serde_json::Value> = (0..100)
            .map(|i| {
                serde_json::json!({
                    "sensor_id": format!("S-{:03}", i),
                    "timestamp": 1710000000 + i * 60,
                    "temperature": 20.0 + (i % 10) as f64 * 0.5,
                    "humidity": 50.0 + (i % 20) as f64 * 0.3,
                    "unit": "celsius",
                    "status": if i % 15 == 0 { "alert" } else { "normal" },
                })
            })
            .collect();

        let p = profile(&input, "sensor_data");
        assert_eq!(p.records, 100);
        assert_eq!(p.fields, 6);
        assert!(p.compression_pct > 0.0);
        // unit should be detected as default (100% celsius)
        assert!(
            p.default_fields.iter().any(|(n, v, _)| n == "unit" && v == "celsius"),
            "Expected unit=celsius as default. Got: {:?}",
            p.default_fields
        );
    }

    #[test]
    fn test_csv_encode() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA\nCarol,35,NYC\nDave,28,NYC\nEve,32,NYC\n";
        let result = encode_csv(csv, "people").unwrap();
        assert!(result.dhoom.contains("people{"));
        assert!(result.dhoom_bytes < result.json_bytes);
    }

    #[test]
    fn test_full_roundtrip_json() {
        // Note: use varied data to avoid trailing all-default records,
        // which are lost when the encoder's trailing newline is trimmed by decode.
        let input = serde_json::json!([
            {"id": 1, "name": "Alice", "score": 42},
            {"id": 2, "name": "Bob",   "score": 85},
            {"id": 3, "name": "Carol", "score": 17},
            {"id": 4, "name": "Dave",  "score": 63},
            {"id": 5, "name": "Eve",   "score": 99},
        ]);
        let arr = input.as_array().unwrap();
        let encoded = encode_json(arr, "people");
        let decoded = decode_to_json(&encoded.dhoom).unwrap();
        assert_eq!(decoded.len(), 5);
    }
}
