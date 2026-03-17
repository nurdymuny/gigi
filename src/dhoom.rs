//! DHOOM Wire Protocol — Encoder/Decoder
//!
//! Fiber bundle serialization format:
//!   - `@` arithmetic fields (predictable sequences elided)
//!   - `|` default/modal fields (zero section σ₀)
//!   - `:` deviation marking (non-default values)
//!   - Trailing elision (omit trailing default fields)
//!
//! Format:
//!   collection{field1, field2@start+step, field3|default, ...}:
//!   val1, val2, val3
//!   val1, val2        (trailing elision: omitted fields = defaults)
//!   val1, val2, :deviated_value  (colon marks deviation from default)

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

/// A detected arithmetic progression: value_n = start + n * step
#[derive(Debug, Clone)]
pub struct ArithmeticField {
    pub name: String,
    pub start: f64,
    pub step: f64,
}

/// A detected default (modal) value for a field
#[derive(Debug, Clone)]
pub struct DefaultField {
    pub name: String,
    pub value: String,
    pub match_count: usize,
    pub match_pct: f64,
}

/// Field classification for DHOOM encoding
#[derive(Debug, Clone)]
pub enum FieldKind {
    /// Regular variable field — always transmitted
    Variable(String),
    /// Arithmetic progression — values derived from index
    Arithmetic(ArithmeticField),
    /// Default (modal) — only deviations transmitted
    Default(DefaultField),
}

impl FieldKind {
    pub fn name(&self) -> &str {
        match self {
            FieldKind::Variable(n) => n,
            FieldKind::Arithmetic(a) => &a.name,
            FieldKind::Default(d) => &d.name,
        }
    }
}

/// DHOOM schema — result of analyzing a dataset
#[derive(Debug, Clone)]
pub struct DhoomSchema {
    pub collection: String,
    /// Fields ordered: variable first, then defaults (for trailing elision)
    pub fields: Vec<FieldKind>,
}

/// Encode result with compression statistics
#[derive(Debug)]
pub struct EncodeResult {
    pub dhoom: String,
    pub json_chars: usize,
    pub dhoom_chars: usize,
    pub compression_pct: f64,
    pub fields_omitted: usize,
    pub total_field_slots: usize,
}

/// Detect arithmetic progression in a sequence of f64 values.
/// Returns Some((start, step)) if all values follow v[i] = start + i*step
/// within tolerance.
pub fn detect_arithmetic(values: &[f64]) -> Option<(f64, f64)> {
    if values.len() < 3 {
        return None;
    }
    let start = values[0];
    let step = values[1] - values[0];
    if step == 0.0 {
        return None; // constant, not arithmetic — will be a default
    }
    let tol = step.abs() * 1e-9 + 1e-12;
    for (i, &v) in values.iter().enumerate() {
        let expected = start + (i as f64) * step;
        if (v - expected).abs() > tol {
            return None;
        }
    }
    Some((start, step))
}

/// Detect the modal (most common) value in a string column.
/// Returns Some((value, count, pct)) if the mode covers >= threshold of records.
pub fn detect_default(values: &[String], threshold: f64) -> Option<(String, usize, f64)> {
    if values.is_empty() {
        return None;
    }
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for v in values {
        *freq.entry(v.as_str()).or_insert(0) += 1;
    }
    let (mode, count) = freq.into_iter().max_by_key(|&(_, c)| c)?;
    let pct = count as f64 / values.len() as f64;
    if pct >= threshold {
        Some((mode.to_string(), count, pct * 100.0))
    } else {
        None
    }
}

/// Value representation for DHOOM encoding
#[derive(Debug, Clone, PartialEq)]
pub enum DhoomValue {
    Number(f64),
    Text(String),
    Bool(bool),
    Null,
}

impl DhoomValue {
    pub fn to_string_repr(&self) -> String {
        match self {
            DhoomValue::Number(n) => {
                if *n == (*n as i64) as f64 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            DhoomValue::Text(s) => s.clone(),
            DhoomValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            DhoomValue::Null => String::new(),
        }
    }

    pub fn matches_default(&self, default: &str) -> bool {
        self.to_string_repr() == default
    }
}

/// A row of DHOOM values
pub type DhoomRow = Vec<DhoomValue>;

/// Analyze a dataset and produce a DhoomSchema with field classifications.
///
/// `records` is a slice of maps (field_name → string representation).
/// `field_order` is the original column order.
/// `collection` is the bundle/collection name.
pub fn analyze_schema(
    records: &[HashMap<String, String>],
    field_order: &[String],
    collection: &str,
) -> DhoomSchema {
    if records.is_empty() || field_order.is_empty() {
        return DhoomSchema {
            collection: collection.to_string(),
            fields: field_order.iter().map(|f| FieldKind::Variable(f.clone())).collect(),
        };
    }

    let mut variable_fields = Vec::new();
    let mut default_fields = Vec::new();

    for field_name in field_order {
        // Collect values in order
        let str_values: Vec<String> = records.iter()
            .map(|r| r.get(field_name).cloned().unwrap_or_default())
            .collect();

        // Try arithmetic detection (only for numeric columns)
        let numeric_values: Vec<f64> = str_values.iter()
            .filter_map(|v| v.parse::<f64>().ok())
            .collect();

        if numeric_values.len() == records.len() {
            if let Some((start, step)) = detect_arithmetic(&numeric_values) {
                // Arithmetic fields go with variable (they're structural, not elided)
                variable_fields.push(FieldKind::Arithmetic(ArithmeticField {
                    name: field_name.clone(),
                    start,
                    step,
                }));
                continue;
            }
        }

        // Try default detection (threshold: 50% for a field to be "default")
        if let Some((mode, count, pct)) = detect_default(&str_values, 0.5) {
            default_fields.push(FieldKind::Default(DefaultField {
                name: field_name.clone(),
                value: mode,
                match_count: count,
                match_pct: pct,
            }));
        } else {
            variable_fields.push(FieldKind::Variable(field_name.clone()));
        }
    }

    // Order: variable + arithmetic first, then defaults (for trailing elision)
    let mut fields = variable_fields;
    fields.extend(default_fields);

    DhoomSchema {
        collection: collection.to_string(),
        fields,
    }
}

/// Build the DHOOM header line from a schema.
pub fn encode_header(schema: &DhoomSchema) -> String {
    let mut parts = Vec::new();
    for field in &schema.fields {
        match field {
            FieldKind::Variable(name) => parts.push(name.clone()),
            FieldKind::Arithmetic(a) => {
                let start = if a.start == (a.start as i64) as f64 {
                    format!("{}", a.start as i64)
                } else {
                    format!("{}", a.start)
                };
                let step = if a.step == (a.step as i64) as f64 {
                    format!("{}", a.step as i64)
                } else {
                    format!("{}", a.step)
                };
                parts.push(format!("{}@{}+{}", a.name, start, step));
            }
            FieldKind::Default(d) => {
                parts.push(format!("{}|{}", d.name, d.value));
            }
        }
    }
    format!("{}{{{}}}:", schema.collection, parts.join(", "))
}

/// Encode a set of records into DHOOM format.
///
/// `records` is a slice of maps (field_name → DhoomValue).
/// Returns the full DHOOM string and compression stats.
pub fn encode(
    records: &[HashMap<String, DhoomValue>],
    schema: &DhoomSchema,
) -> EncodeResult {
    let mut out = String::new();
    let header = encode_header(schema);
    out.push_str(&header);
    out.push('\n');

    let mut fields_omitted = 0usize;
    let total_field_slots = records.len() * schema.fields.len();

    for (_row_idx, record) in records.iter().enumerate() {
        let mut row_values: Vec<String> = Vec::new();
        let mut row_is_default: Vec<bool> = Vec::new();

        for field in &schema.fields {
            match field {
                FieldKind::Arithmetic(_) => {
                    // Arithmetic fields are derived — don't transmit
                    fields_omitted += 1;
                    row_values.push(String::new());
                    row_is_default.push(true);
                }
                FieldKind::Default(d) => {
                    let val = record.get(&d.name);
                    let is_default = val.map_or(true, |v| v.matches_default(&d.value));
                    if is_default {
                        fields_omitted += 1;
                        row_values.push(String::new());
                        row_is_default.push(true);
                    } else {
                        let repr = val.map_or(String::new(), |v| v.to_string_repr());
                        row_values.push(format!(":{}", repr));
                        row_is_default.push(false);
                    }
                }
                FieldKind::Variable(name) => {
                    let repr = record.get(name)
                        .map_or(String::new(), |v| v.to_string_repr());
                    row_values.push(repr);
                    row_is_default.push(false);
                }
            }
        }

        // Trailing elision: find last non-default field
        let last_non_default = row_is_default.iter().rposition(|&d| !d)
            .unwrap_or(0);

        // Count trailing defaults being elided
        for i in (last_non_default + 1)..row_is_default.len() {
            if !row_is_default[i] {
                // already counted above
            }
            // trailing defaults already counted in fields_omitted
        }

        // Build row: only up to last non-default
        let row_parts: Vec<&str> = row_values[..=last_non_default].iter()
            .enumerate()
            .filter_map(|(i, v)| {
                match &schema.fields[i] {
                    FieldKind::Arithmetic(_) => None, // skip arithmetic
                    _ => Some(v.as_str()),
                }
            })
            .collect();

        let _ = writeln!(out, "{}", row_parts.join(", "));
    }

    // Compute JSON equivalent for comparison
    let json_chars = estimate_json_size(records, schema);
    let dhoom_chars = out.len();
    let compression_pct = if json_chars > 0 {
        100.0 * (1.0 - dhoom_chars as f64 / json_chars as f64)
    } else {
        0.0
    };

    EncodeResult {
        dhoom: out,
        json_chars,
        dhoom_chars,
        compression_pct,
        fields_omitted,
        total_field_slots,
    }
}

/// Estimate equivalent JSON size for a set of records.
fn estimate_json_size(
    records: &[HashMap<String, DhoomValue>],
    schema: &DhoomSchema,
) -> usize {
    let mut size = 2; // []
    for (i, record) in records.iter().enumerate() {
        if i > 0 { size += 1; } // comma
        size += 1; // {
        let mut first = true;
        for field in &schema.fields {
            let name = field.name();
            if !first { size += 1; } // comma
            first = false;
            size += 1 + name.len() + 2; // "name":
            if let Some(val) = record.get(name) {
                let repr = val.to_string_repr();
                match val {
                    DhoomValue::Text(_) => size += repr.len() + 2, // "value"
                    _ => size += repr.len(),
                }
            } else {
                size += 4; // null
            }
        }
        size += 1; // }
    }
    size
}

/// Parsed DHOOM document
#[derive(Debug)]
pub struct ParsedDhoom {
    pub collection: String,
    pub records: Vec<HashMap<String, DhoomValue>>,
    pub schema: DhoomSchema,
}

/// Parsed header field descriptor
#[derive(Debug, Clone)]
enum HeaderField {
    Plain(String),
    Arithmetic { name: String, start: f64, step: f64 },
    Default { name: String, value: String },
}

/// Parse a DHOOM header line like `collection{f1, f2@0+1, f3|default}:`
fn parse_header(line: &str) -> Result<(String, Vec<HeaderField>), String> {
    let line = line.trim();
    let brace_start = line.find('{')
        .ok_or_else(|| "Missing '{' in header".to_string())?;
    let brace_end = line.rfind('}')
        .ok_or_else(|| "Missing '}' in header".to_string())?;

    let collection = line[..brace_start].to_string();
    let after_brace = line[brace_end + 1..].trim();
    if !after_brace.starts_with(':') {
        return Err("Header must end with '}:'".to_string());
    }

    let fields_str = &line[brace_start + 1..brace_end];
    let mut fields = Vec::new();

    for part in fields_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(at_pos) = part.find('@') {
            // Arithmetic: name@start+step
            let name = part[..at_pos].trim().to_string();
            let rest = &part[at_pos + 1..];
            let plus_pos = rest.find('+')
                .ok_or_else(|| format!("Arithmetic field '{}' missing '+' in step", name))?;
            let start: f64 = rest[..plus_pos].trim().parse()
                .map_err(|_| format!("Invalid start in arithmetic field '{}'", name))?;
            let step: f64 = rest[plus_pos + 1..].trim().parse()
                .map_err(|_| format!("Invalid step in arithmetic field '{}'", name))?;
            fields.push(HeaderField::Arithmetic { name, start, step });
        } else if let Some(pipe_pos) = part.find('|') {
            // Default: name|value
            let name = part[..pipe_pos].trim().to_string();
            let value = part[pipe_pos + 1..].trim().to_string();
            fields.push(HeaderField::Default { name, value });
        } else {
            // Plain variable field
            fields.push(HeaderField::Plain(part.to_string()));
        }
    }

    Ok((collection, fields))
}

/// Decode a DHOOM string back into records.
pub fn decode(dhoom: &str) -> Result<ParsedDhoom, String> {
    let mut lines = dhoom.lines();

    let header_line = lines.next()
        .ok_or_else(|| "Empty DHOOM input".to_string())?;
    let (collection, header_fields) = parse_header(header_line)?;

    // Build schema from header
    let mut schema_fields = Vec::new();
    let mut transmit_fields: Vec<(usize, &str)> = Vec::new(); // (schema_idx, kind)

    for (i, hf) in header_fields.iter().enumerate() {
        match hf {
            HeaderField::Plain(name) => {
                schema_fields.push(FieldKind::Variable(name.clone()));
                transmit_fields.push((i, "variable"));
            }
            HeaderField::Arithmetic { name, start, step } => {
                schema_fields.push(FieldKind::Arithmetic(ArithmeticField {
                    name: name.clone(),
                    start: *start,
                    step: *step,
                }));
                // Not transmitted — derived from row index
            }
            HeaderField::Default { name, value } => {
                schema_fields.push(FieldKind::Default(DefaultField {
                    name: name.clone(),
                    value: value.clone(),
                    match_count: 0,
                    match_pct: 0.0,
                }));
                transmit_fields.push((i, "default"));
            }
        }
    }

    let schema = DhoomSchema {
        collection: collection.clone(),
        fields: schema_fields,
    };

    let mut records = Vec::new();
    let mut row_idx = 0usize;

    for line in lines {
        let line = line.trim();
        // Every line (including empty ones for all-default rows) is a data row

        let mut record: HashMap<String, DhoomValue> = HashMap::new();

        // Parse transmitted values
        let parts: Vec<&str> = if line.is_empty() {
            Vec::new()
        } else {
            line.split(',').map(|s| s.trim()).collect()
        };

        // Map transmitted values to their fields
        let mut part_idx = 0;
        for &(schema_idx, _kind) in &transmit_fields {
            let field = &header_fields[schema_idx];
            let name = match field {
                HeaderField::Plain(n) => n.clone(),
                HeaderField::Default { name, .. } => name.clone(),
                HeaderField::Arithmetic { name, .. } => name.clone(),
            };

            if part_idx < parts.len() {
                let val_str = parts[part_idx];
                if val_str.is_empty() {
                    // Empty = default for default fields
                    if let HeaderField::Default { value, .. } = field {
                        record.insert(name, parse_value(value));
                    }
                } else if val_str.starts_with(':') {
                    // Deviation from default
                    let deviated = &val_str[1..];
                    record.insert(name, parse_value(deviated));
                } else {
                    record.insert(name, parse_value(val_str));
                }
                part_idx += 1;
            } else {
                // Trailing elision — use default
                if let HeaderField::Default { value, .. } = field {
                    record.insert(name, parse_value(value));
                }
            }
        }

        // Fill in arithmetic fields
        for hf in &header_fields {
            if let HeaderField::Arithmetic { name, start, step } = hf {
                let val = start + (row_idx as f64) * step;
                record.insert(name.clone(), DhoomValue::Number(val));
            }
        }

        records.push(record);
        row_idx += 1;
    }

    Ok(ParsedDhoom {
        collection,
        records,
        schema,
    })
}

/// Parse a string value into DhoomValue
fn parse_value(s: &str) -> DhoomValue {
    let s = s.trim();
    if s.is_empty() {
        return DhoomValue::Null;
    }
    if s == "true" {
        return DhoomValue::Bool(true);
    }
    if s == "false" {
        return DhoomValue::Bool(false);
    }
    if let Ok(n) = s.parse::<f64>() {
        return DhoomValue::Number(n);
    }
    DhoomValue::Text(s.to_string())
}

// ── Conversion helpers: serde_json::Value <-> DhoomValue ──

/// Convert serde_json::Value to DhoomValue
pub fn json_to_dhoom_value(v: &serde_json::Value) -> DhoomValue {
    match v {
        serde_json::Value::Number(n) => {
            DhoomValue::Number(n.as_f64().unwrap_or(0.0))
        }
        serde_json::Value::String(s) => DhoomValue::Text(s.clone()),
        serde_json::Value::Bool(b) => DhoomValue::Bool(*b),
        serde_json::Value::Null => DhoomValue::Null,
        _ => DhoomValue::Text(v.to_string()),
    }
}

/// Convert DhoomValue to serde_json::Value
pub fn dhoom_to_json_value(v: &DhoomValue) -> serde_json::Value {
    match v {
        DhoomValue::Number(n) => {
            if *n == (*n as i64) as f64 && n.abs() < 1e15 {
                serde_json::Value::Number(serde_json::Number::from(*n as i64))
            } else {
                serde_json::Number::from_f64(*n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
        }
        DhoomValue::Text(s) => serde_json::Value::String(s.clone()),
        DhoomValue::Bool(b) => serde_json::Value::Bool(*b),
        DhoomValue::Null => serde_json::Value::Null,
    }
}

/// Convert a JSON array to DHOOM-ready records with field order.
pub fn json_array_to_dhoom(
    input: &[serde_json::Value],
) -> (Vec<HashMap<String, DhoomValue>>, Vec<String>) {
    let mut field_order: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut records = Vec::new();

    // Discover field order from first pass
    for item in input {
        if let serde_json::Value::Object(map) = item {
            for key in map.keys() {
                if seen.insert(key.clone()) {
                    field_order.push(key.clone());
                }
            }
        }
    }

    // Convert records
    for item in input {
        if let serde_json::Value::Object(map) = item {
            let mut record: HashMap<String, DhoomValue> = HashMap::new();
            for (k, v) in map {
                record.insert(k.clone(), json_to_dhoom_value(v));
            }
            records.push(record);
        }
    }

    (records, field_order)
}

/// Convert a parsed DHOOM result back to JSON array.
pub fn dhoom_to_json_array(parsed: &ParsedDhoom) -> Vec<serde_json::Value> {
    parsed.records.iter().map(|record| {
        let mut map = serde_json::Map::new();
        for field in &parsed.schema.fields {
            let name = field.name();
            if let Some(val) = record.get(name) {
                map.insert(name.to_string(), dhoom_to_json_value(val));
            }
        }
        serde_json::Value::Object(map)
    }).collect()
}

/// Full encode pipeline: JSON array → DHOOM string
pub fn encode_json(
    input: &[serde_json::Value],
    collection: &str,
) -> EncodeResult {
    let (records, field_order) = json_array_to_dhoom(input);

    // Build string representations for schema analysis
    let str_records: Vec<HashMap<String, String>> = records.iter()
        .map(|r| {
            r.iter().map(|(k, v)| (k.clone(), v.to_string_repr())).collect()
        })
        .collect();

    let schema = analyze_schema(&str_records, &field_order, collection);
    encode(&records, &schema)
}

/// Full decode pipeline: DHOOM string → JSON array
pub fn decode_to_json(dhoom: &str) -> Result<Vec<serde_json::Value>, String> {
    let parsed = decode(dhoom)?;
    Ok(dhoom_to_json_array(&parsed))
}

// ── CSV support ──

/// Parse CSV text into JSON-like records
pub fn csv_to_records(csv_text: &str) -> Result<(Vec<HashMap<String, DhoomValue>>, Vec<String>), String> {
    let mut lines = csv_text.lines();
    let header_line = lines.next()
        .ok_or_else(|| "Empty CSV input".to_string())?;

    let field_order: Vec<String> = header_line.split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .collect();

    let mut records = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut record: HashMap<String, DhoomValue> = HashMap::new();
        let values: Vec<&str> = line.split(',').collect();
        for (i, field_name) in field_order.iter().enumerate() {
            let val = values.get(i).map(|s| s.trim().trim_matches('"')).unwrap_or("");
            record.insert(field_name.clone(), parse_value(val));
        }
        records.push(record);
    }

    Ok((records, field_order))
}

// ── Streaming encoder ──

/// Streaming DHOOM encoder — emit header once, then rows incrementally.
pub struct StreamEncoder {
    schema: DhoomSchema,
    header_emitted: bool,
    row_count: usize,
}

impl StreamEncoder {
    pub fn new(schema: DhoomSchema) -> Self {
        StreamEncoder {
            schema,
            header_emitted: false,
            row_count: 0,
        }
    }

    /// Emit the header line (only on first call)
    pub fn header(&mut self) -> Option<String> {
        if self.header_emitted {
            return None;
        }
        self.header_emitted = true;
        Some(encode_header(&self.schema))
    }

    /// Encode a single record into a DHOOM row line
    pub fn push(&mut self, record: &HashMap<String, DhoomValue>) -> String {
        let mut row_values: Vec<String> = Vec::new();
        let mut row_is_default: Vec<bool> = Vec::new();

        for field in &self.schema.fields {
            match field {
                FieldKind::Arithmetic(_) => {
                    row_values.push(String::new());
                    row_is_default.push(true);
                }
                FieldKind::Default(d) => {
                    let val = record.get(&d.name);
                    let is_default = val.map_or(true, |v| v.matches_default(&d.value));
                    if is_default {
                        row_values.push(String::new());
                        row_is_default.push(true);
                    } else {
                        let repr = val.map_or(String::new(), |v| v.to_string_repr());
                        row_values.push(format!(":{}", repr));
                        row_is_default.push(false);
                    }
                }
                FieldKind::Variable(name) => {
                    let repr = record.get(name)
                        .map_or(String::new(), |v| v.to_string_repr());
                    row_values.push(repr);
                    row_is_default.push(false);
                }
            }
        }

        // Trailing elision
        let last_non_default = row_is_default.iter().rposition(|&d| !d)
            .unwrap_or(0);

        let row_parts: Vec<&str> = row_values[..=last_non_default].iter()
            .enumerate()
            .filter_map(|(i, v)| {
                match &self.schema.fields[i] {
                    FieldKind::Arithmetic(_) => None,
                    _ => Some(v.as_str()),
                }
            })
            .collect();

        self.row_count += 1;
        row_parts.join(", ")
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic_detection() {
        let vals: Vec<f64> = (0..100).map(|i| 1000.0 + i as f64 * 60.0).collect();
        let result = detect_arithmetic(&vals);
        assert!(result.is_some());
        let (start, step) = result.unwrap();
        assert_eq!(start, 1000.0);
        assert_eq!(step, 60.0);
    }

    #[test]
    fn test_arithmetic_detection_negative() {
        let vals = vec![1.0, 3.0, 2.0, 5.0]; // not arithmetic
        assert!(detect_arithmetic(&vals).is_none());
    }

    #[test]
    fn test_default_detection() {
        let mut vals: Vec<String> = vec!["normal".to_string(); 90];
        vals.extend(vec!["alert".to_string(); 10]);
        let result = detect_default(&vals, 0.5);
        assert!(result.is_some());
        let (mode, count, _) = result.unwrap();
        assert_eq!(mode, "normal");
        assert_eq!(count, 90);
    }

    #[test]
    fn test_default_detection_no_majority() {
        let vals = vec![
            "a".to_string(), "b".to_string(), "c".to_string(),
            "d".to_string(), "e".to_string(),
        ];
        assert!(detect_default(&vals, 0.5).is_none());
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let json_input = serde_json::json!([
            {"id": 1, "name": "Alice", "status": "active", "score": 95.5},
            {"id": 2, "name": "Bob",   "status": "active", "score": 87.3},
            {"id": 3, "name": "Carol", "status": "active", "score": 92.1},
            {"id": 4, "name": "Dave",  "status": "inactive", "score": 78.0},
            {"id": 5, "name": "Eve",   "status": "active", "score": 99.2},
        ]);
        let arr = json_input.as_array().unwrap();

        // Encode
        let result = encode_json(arr, "users");
        assert!(result.dhoom.contains("users{"));
        assert!(result.compression_pct > 0.0);

        // Decode
        let decoded = decode_to_json(&result.dhoom).unwrap();
        assert_eq!(decoded.len(), 5);

        // Verify round-trip: all values preserved
        for (orig, dec) in arr.iter().zip(decoded.iter()) {
            let orig_obj = orig.as_object().unwrap();
            let dec_obj = dec.as_object().unwrap();
            for (key, orig_val) in orig_obj {
                let dec_val = dec_obj.get(key)
                    .unwrap_or_else(|| panic!("Missing field '{}' in decoded record", key));
                // Compare as strings for cross-type compatibility
                if orig_val.is_number() && dec_val.is_number() {
                    let ov = orig_val.as_f64().unwrap();
                    let dv = dec_val.as_f64().unwrap();
                    assert!((ov - dv).abs() < 1e-9,
                        "Field '{}' mismatch: {} vs {}", key, ov, dv);
                } else {
                    assert_eq!(orig_val, dec_val,
                        "Field '{}' mismatch", key);
                }
            }
        }
    }

    #[test]
    fn test_encode_with_defaults() {
        // 8/10 records have status="normal" → should detect as default
        let json_input: Vec<serde_json::Value> = (0..10).map(|i| {
            serde_json::json!({
                "sensor": format!("S-{:03}", i),
                "value": 20.0 + i as f64,
                "status": if i < 8 { "normal" } else { "alert" },
            })
        }).collect();

        let result = encode_json(&json_input, "sensors");
        // Header should show status|normal
        assert!(result.dhoom.contains("status|normal"),
            "Expected default annotation in header. Got:\n{}", result.dhoom);
    }

    #[test]
    fn test_csv_parsing() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA\nCarol,35,NYC\n";
        let (records, fields) = csv_to_records(csv).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(fields, vec!["name", "age", "city"]);
        assert_eq!(records[0].get("name"), Some(&DhoomValue::Text("Alice".to_string())));
        assert_eq!(records[1].get("age"), Some(&DhoomValue::Number(25.0)));
    }

    #[test]
    fn test_streaming_encoder() {
        let schema = DhoomSchema {
            collection: "test".to_string(),
            fields: vec![
                FieldKind::Variable("x".to_string()),
                FieldKind::Variable("y".to_string()),
                FieldKind::Default(DefaultField {
                    name: "status".to_string(),
                    value: "ok".to_string(),
                    match_count: 0,
                    match_pct: 0.0,
                }),
            ],
        };

        let mut encoder = StreamEncoder::new(schema);
        let header = encoder.header().unwrap();
        assert!(header.contains("test{"));
        assert!(header.contains("status|ok"));

        // Second header call returns None
        assert!(encoder.header().is_none());

        let mut rec = HashMap::new();
        rec.insert("x".to_string(), DhoomValue::Number(1.0));
        rec.insert("y".to_string(), DhoomValue::Number(2.0));
        rec.insert("status".to_string(), DhoomValue::Text("ok".to_string()));
        let row = encoder.push(&rec);
        // Status is default → trailing elision
        assert_eq!(row, "1, 2");

        let mut rec2 = HashMap::new();
        rec2.insert("x".to_string(), DhoomValue::Number(3.0));
        rec2.insert("y".to_string(), DhoomValue::Number(4.0));
        rec2.insert("status".to_string(), DhoomValue::Text("error".to_string()));
        let row2 = encoder.push(&rec2);
        // Status deviates → deviation marker
        assert_eq!(row2, "3, 4, :error");

        assert_eq!(encoder.row_count(), 2);
    }

    #[test]
    fn test_arithmetic_encode_decode() {
        // Create records with arithmetic id field
        let json_input: Vec<serde_json::Value> = (0..20).map(|i| {
            serde_json::json!({
                "id": 100 + i * 10,
                "name": format!("item_{}", i),
                "value": (i as f64) * 1.5,
            })
        }).collect();

        let result = encode_json(&json_input, "items");
        // Header should show id@100+10
        assert!(result.dhoom.contains("@100+10"),
            "Expected arithmetic annotation. Got:\n{}", result.dhoom);

        // Decode and verify
        let decoded = decode_to_json(&result.dhoom).unwrap();
        assert_eq!(decoded.len(), 20);
        // Check arithmetic values reconstructed correctly
        for i in 0..20 {
            let expected_id = 100 + i * 10;
            let got = decoded[i].get("id").unwrap().as_i64().unwrap();
            assert_eq!(got, expected_id as i64);
        }
    }

    #[test]
    fn test_large_compression() {
        // Simulate sensor data: high default rate, arithmetic id
        let json_input: Vec<serde_json::Value> = (0..1000).map(|i| {
            serde_json::json!({
                "sensor_id": format!("S-{:04}", i),
                "timestamp": 1710000000 + i * 60,
                "temperature": 20.0 + (i % 10) as f64 * 0.5,
                "unit": "celsius",
                "status": if i % 20 == 0 { "alert" } else { "normal" },
            })
        }).collect();

        let result = encode_json(&json_input, "sensors");
        assert!(result.compression_pct > 30.0,
            "Expected >30% compression, got {:.1}%", result.compression_pct);

        // Round-trip
        let decoded = decode_to_json(&result.dhoom).unwrap();
        assert_eq!(decoded.len(), 1000);
    }
}
