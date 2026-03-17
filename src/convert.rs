//! GIGI Convert — Format conversion + geometric data profiling
//!
//! Converts JSON/CSV into DHOOM format with full geometric analysis:
//! curvature per field, compression estimates, fiber structure report.

use crate::dhoom::{
    self, DhoomSchema, DhoomValue, EncodeResult, FieldKind,
    json_array_to_dhoom, analyze_schema,
};
use std::collections::HashMap;

/// Geometric profile of a dataset
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
    pub fields_omitted: usize,
    pub total_field_slots: usize,
    pub field_omission_pct: f64,
    pub curvature: Vec<(String, f64, f64)>, // (field, K, confidence)
    pub schema: DhoomSchema,
}

/// Per-field curvature computed from value distribution
fn compute_field_curvature(values: &[f64]) -> (f64, f64) {
    if values.len() < 2 {
        return (0.0, 1.0);
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range <= 0.0 {
        return (0.0, 1.0);
    }
    let k = variance / (range * range);
    let confidence = 1.0 / (1.0 + k);
    (k, confidence)
}

/// Profile a JSON dataset without encoding — analyze its geometric structure.
pub fn profile(
    input: &[serde_json::Value],
    collection: &str,
) -> Profile {
    let (records, field_order) = json_array_to_dhoom(input);

    // Build string representations for schema analysis
    let str_records: Vec<HashMap<String, String>> = records.iter()
        .map(|r| r.iter().map(|(k, v)| (k.clone(), v.to_string_repr())).collect())
        .collect();

    let schema = analyze_schema(&str_records, &field_order, collection);

    // Encode to get compression stats
    let encode_result = dhoom::encode(&records, &schema);

    // Classify fields
    let mut arithmetic_fields = Vec::new();
    let mut default_fields = Vec::new();
    let mut variable_fields = Vec::new();

    for field in &schema.fields {
        match field {
            FieldKind::Arithmetic(a) => arithmetic_fields.push(a.name.clone()),
            FieldKind::Default(d) => {
                default_fields.push((d.name.clone(), d.value.clone(), d.match_pct));
            }
            FieldKind::Variable(name) => variable_fields.push(name.clone()),
        }
    }

    // Compute curvature per numeric field
    let mut curvature = Vec::new();
    for field_name in &field_order {
        let numeric_values: Vec<f64> = records.iter()
            .filter_map(|r| r.get(field_name))
            .filter_map(|v| match v {
                DhoomValue::Number(n) => Some(*n),
                _ => None,
            })
            .collect();

        if numeric_values.len() >= 2 {
            let (k, conf) = compute_field_curvature(&numeric_values);
            curvature.push((field_name.clone(), k, conf));
        }
    }

    let field_omission_pct = if encode_result.total_field_slots > 0 {
        100.0 * encode_result.fields_omitted as f64 / encode_result.total_field_slots as f64
    } else {
        0.0
    };

    Profile {
        collection: collection.to_string(),
        records: records.len(),
        fields: field_order.len(),
        arithmetic_fields,
        default_fields,
        variable_fields,
        json_chars: encode_result.json_chars,
        dhoom_chars: encode_result.dhoom_chars,
        compression_pct: encode_result.compression_pct,
        token_savings_pct: encode_result.compression_pct, // chars ≈ tokens for estimation
        fields_omitted: encode_result.fields_omitted,
        total_field_slots: encode_result.total_field_slots,
        field_omission_pct,
        curvature,
        schema,
    }
}

/// Full encode pipeline: JSON → DHOOM
pub fn encode_json(
    input: &[serde_json::Value],
    collection: &str,
) -> EncodeResult {
    dhoom::encode_json(input, collection)
}

/// Full decode pipeline: DHOOM → JSON
pub fn decode_to_json(dhoom: &str) -> Result<Vec<serde_json::Value>, String> {
    dhoom::decode_to_json(dhoom)
}

/// CSV → DHOOM conversion
pub fn encode_csv(
    csv_text: &str,
    collection: &str,
) -> Result<EncodeResult, String> {
    let (records, field_order) = dhoom::csv_to_records(csv_text)?;

    let str_records: Vec<HashMap<String, String>> = records.iter()
        .map(|r| r.iter().map(|(k, v)| (k.clone(), v.to_string_repr())).collect())
        .collect();

    let schema = analyze_schema(&str_records, &field_order, collection);
    Ok(dhoom::encode(&records, &schema))
}

/// Format a profile as a human-readable report (like the spec shows)
pub fn format_profile(p: &Profile) -> String {
    let mut out = String::new();
    out.push_str("\nGIGI Convert — Geometric Data Profile\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");
    out.push_str(&format!("  Collection:    {} ({} records, {} fields)\n\n",
        p.collection, p.records, p.fields));

    out.push_str("  Field Analysis:\n");
    for field in &p.schema.fields {
        match field {
            FieldKind::Arithmetic(a) => {
                let start = if a.start == (a.start as i64) as f64 {
                    format!("{}", a.start as i64)
                } else {
                    format!("{:.1}", a.start)
                };
                let step = if a.step == (a.step as i64) as f64 {
                    format!("+{}", a.step as i64)
                } else {
                    format!("+{:.1}", a.step)
                };
                out.push_str(&format!("    {:<14} ARITHMETIC  @{}{:<12} ({} values derived)\n",
                    a.name, start, step, p.records));
            }
            FieldKind::Default(d) => {
                out.push_str(&format!("    {:<14} DEFAULT     {:14} ({} matches, {:.1}%)\n",
                    d.name, format!("\"{}\"", d.value), d.match_count, d.match_pct));
            }
            FieldKind::Variable(name) => {
                out.push_str(&format!("    {:<14} VARIABLE\n", name));
            }
        }
    }

    out.push_str("\n  Fiber Structure:\n");
    out.push_str(&format!("    Base space:     {} arithmetic fields\n",
        p.arithmetic_fields.len()));
    out.push_str(&format!("    Fiber:          {} value fields\n",
        p.fields - p.arithmetic_fields.len()));
    if !p.default_fields.is_empty() {
        let defaults: Vec<String> = p.default_fields.iter()
            .map(|(n, v, _)| format!("{}=\"{}\"", n, v))
            .collect();
        out.push_str(&format!("    Zero section:   {}\n", defaults.join(", ")));
    }

    out.push_str(&format!("\n  Compression:\n"));
    let json_tokens = (p.json_chars as f64 / 3.5).ceil() as usize;
    let dhoom_tokens = (p.dhoom_chars as f64 / 3.5).ceil() as usize;
    out.push_str(&format!("    JSON (minified):  {:>10} chars  (~{:>6} tokens)\n",
        p.json_chars, json_tokens));
    out.push_str(&format!("    DHOOM:            {:>10} chars  (~{:>6} tokens)\n",
        p.dhoom_chars, dhoom_tokens));
    out.push_str(&format!("    Savings:          {:.1}% smaller    ({:.1}% fewer tokens)\n",
        p.compression_pct, p.token_savings_pct));
    out.push_str(&format!("    Fields omitted:   {} of {} ({:.1}%)\n",
        p.fields_omitted, p.total_field_slots, p.field_omission_pct));

    if !p.curvature.is_empty() {
        out.push_str(&format!("\n  Curvature:\n"));
        for (field, k, conf) in &p.curvature {
            out.push_str(&format!("    K({:<12})   {:.6}  (confidence: {:.4})\n",
                format!("{})", field), k, conf));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_sensor_data() {
        let input: Vec<serde_json::Value> = (0..100).map(|i| {
            serde_json::json!({
                "sensor_id": format!("S-{:03}", i),
                "timestamp": 1710000000 + i * 60,
                "temperature": 20.0 + (i % 10) as f64 * 0.5,
                "humidity": 50.0 + (i % 20) as f64 * 0.3,
                "unit": "celsius",
                "status": if i % 15 == 0 { "alert" } else { "normal" },
            })
        }).collect();

        let p = profile(&input, "sensor_data");
        assert_eq!(p.records, 100);
        assert_eq!(p.fields, 6);
        assert!(p.compression_pct > 0.0);
        // timestamp should be detected as arithmetic
        assert!(p.arithmetic_fields.contains(&"timestamp".to_string()),
            "Expected timestamp as arithmetic. Got: {:?}", p.arithmetic_fields);
        // unit should be 100% default
        assert!(p.default_fields.iter().any(|(n, v, _)| n == "unit" && v == "celsius"),
            "Expected unit=celsius as default. Got: {:?}", p.default_fields);
    }

    #[test]
    fn test_csv_encode() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA\nCarol,35,NYC\nDave,28,NYC\nEve,32,NYC\n";
        let result = encode_csv(csv, "people").unwrap();
        assert!(result.dhoom.contains("people{"));
        assert!(result.dhoom_chars < result.json_chars);
    }

    #[test]
    fn test_full_roundtrip_json() {
        let input = serde_json::json!([
            {"x": 1, "y": "hello", "z": true},
            {"x": 2, "y": "world", "z": false},
            {"x": 3, "y": "hello", "z": true},
        ]);
        let arr = input.as_array().unwrap();
        let encoded = encode_json(arr, "test");
        let decoded = decode_to_json(&encoded.dhoom).unwrap();
        assert_eq!(decoded.len(), 3);
    }
}
