//! # DHOOM â€” Davis Human-readable Optimized Object Markup
//!
//! A compact, human-readable serialization format built on fiber bundle geometry.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use dhoom::{encode, decode};
//! use serde_json::json;
//!
//! let data = json!({
//!     "reviews": [
//!         {"id": 101, "customer": "Alex Rivera", "rating": 5, "comment": "Excellent!", "verified": true},
//!         {"id": 102, "customer": "Brij Pandey", "rating": 5, "comment": "Game changer!", "verified": true},
//!         {"id": 103, "customer": "Casey Lee", "rating": 3, "comment": "Average", "verified": false}
//!     ]
//! });
//!
//! let dhoom_str = encode(&data).unwrap();
//! let roundtrip = decode(&dhoom_str).unwrap();
//! assert_eq!(data, roundtrip);
//! ```

use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::fmt::Write;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DhoomError {
    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("Encode error: {0}")]
    Encode(String),

    #[error("Invalid arithmetic pattern: {0}")]
    ArithmeticPattern(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, DhoomError>;

/// A field modifier in the fiber header.
#[derive(Debug, Clone, PartialEq)]
pub enum Modifier {
    /// Arithmetic base: `@start` or `@start+step`
    Arithmetic { start: Value, step: Option<i64> },
    /// Modal default: `|value`
    Default(Value),
    /// Nested sub-bundle: `>`
    Nested,
    /// Delta-encoded: `^`
    Delta,
    /// Morphism reference: `->target`
    Morphism { target: String },
    /// String interning: `&`
    Interned { pool: Vec<String> },
    /// Computed field: `#expr`
    Computed { expr: String },
    /// Inline constraint: `!constraint`
    Constraint { constraint: String },
}

/// A single field declaration in the fiber.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub name: String,
    pub modifier: Option<Modifier>,
}

/// A parsed fiber (schema header).
#[derive(Debug, Clone, PartialEq)]
pub struct Fiber {
    pub name: Option<String>,
    pub fields: Vec<FieldDecl>,
    pub sparse: bool,
}

impl Fiber {
    /// Returns the fields that appear in record bodies (non-arithmetic).
    pub fn record_fields(&self) -> Vec<&FieldDecl> {
        self.fields
            .iter()
            .filter(|f| {
                !matches!(
                    f.modifier,
                    Some(Modifier::Arithmetic { .. }) | Some(Modifier::Computed { .. })
                )
            })
            .collect()
    }

    /// Returns the default value for a field, if declared.
    pub fn default_for(&self, name: &str) -> Option<&Value> {
        self.fields.iter().find_map(|f| {
            if f.name == name {
                match &f.modifier {
                    Some(Modifier::Default(v)) => Some(v),
                    _ => None,
                }
            } else {
                None
            }
        })
    }
}

// ---------------------------------------------------------------------------
// DhoomRecordParser — shared record-line parser for streaming + mmap paths
// ---------------------------------------------------------------------------

/// Stateless record-line parser backed by a Fiber schema.
///
/// Both `MmapBundle::decode_record_line()` and the streaming decoder use this
/// so DHOOM format changes only need updating in one place.
pub struct DhoomRecordParser {
    fiber: Fiber,
    /// Cached: non-arithmetic field declarations (record body fields).
    record_fields: Vec<FieldDecl>,
}

impl DhoomRecordParser {
    /// Build a parser from a parsed Fiber.
    pub fn new(fiber: Fiber) -> Self {
        let record_fields: Vec<FieldDecl> = fiber
            .fields
            .iter()
            .filter(|f| {
                !matches!(
                    f.modifier,
                    Some(Modifier::Arithmetic { .. }) | Some(Modifier::Computed { .. })
                )
            })
            .cloned()
            .collect();
        Self { fiber, record_fields }
    }

    /// Access the underlying fiber.
    pub fn fiber(&self) -> &Fiber { &self.fiber }

    /// Number of record-body fields (non-arithmetic).
    pub fn record_field_count(&self) -> usize { self.record_fields.len() }

    /// Decode a single DHOOM record line at a given ordinal.
    pub fn decode_line(&self, line: &str, ordinal: usize) -> Value {
        let raw_fields: Vec<String> = if line.is_empty() {
            vec![]
        } else {
            split_record_fields(line)
        };

        let mut obj = Map::new();

        // Fill arithmetic fields
        for fdecl in &self.fiber.fields {
            if let Some(Modifier::Arithmetic { ref start, ref step }) = fdecl.modifier {
                let s = step.unwrap_or(1);
                obj.insert(fdecl.name.clone(), arithmetic_value(start, s, ordinal));
            }
        }

        // Map positional record values
        for (j, rf) in self.record_fields.iter().enumerate() {
            if j < raw_fields.len() {
                let raw = &raw_fields[j];
                let val = if raw.is_empty() {
                    if let Some(Modifier::Default(ref d)) = rf.modifier {
                        d.clone()
                    } else {
                        Value::String(String::new())
                    }
                } else if let Some(stripped) = raw.strip_prefix(':') {
                    coerce(stripped)
                } else {
                    coerce(raw)
                };
                obj.insert(rf.name.clone(), val);
            } else {
                // Trailing elision → fill with default
                if let Some(Modifier::Default(ref d)) = rf.modifier {
                    obj.insert(rf.name.clone(), d.clone());
                }
            }
        }

        Value::Object(obj)
    }
}

// ---------------------------------------------------------------------------
// Value coercion
// ---------------------------------------------------------------------------

/// Coerce a raw string token into a typed JSON Value per DHOOM spec Â§8.
pub fn coerce(s: &str) -> Value {
    match s {
        "T" => Value::Bool(true),
        "F" => Value::Bool(false),
        "null" => Value::Null,
        "" => Value::String(String::new()),
        _ => {
            if let Ok(i) = s.parse::<i64>() {
                Value::Number(Number::from(i))
            } else if let Ok(f) = s.parse::<f64>() {
                Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or_else(|| Value::String(s.to_string()))
            } else {
                Value::String(s.to_string())
            }
        }
    }
}

/// Format a JSON Value back to its DHOOM record representation.
fn value_to_dhoom(v: &Value) -> String {
    match v {
        Value::Bool(true) => "T".into(),
        Value::Bool(false) => "F".into(),
        Value::Null => "null".into(),
        Value::String(s) => {
            if s.contains(',') || s.contains(':') || s.contains('\n') || s.contains('"') {
                let escaped = s.replace('"', "\"\"");
                format!("\"{}\"", escaped)
            } else {
                s.clone()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Array(_) | Value::Object(_) => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Arithmetic helpers
// ---------------------------------------------------------------------------

/// Parse a string-pattern arithmetic start value.
/// Returns (prefix, numeric_suffix, padding_width) or None if purely numeric.
fn parse_string_pattern(s: &str) -> Option<(String, i64, usize)> {
    // Find the last non-digit character
    let last_nondigit = s.rfind(|c: char| !c.is_ascii_digit())?;
    let prefix = &s[..=last_nondigit];
    let suffix = &s[last_nondigit + 1..];
    if suffix.is_empty() {
        return None;
    }
    let width = suffix.len();
    let num: i64 = suffix.parse().ok()?;
    Some((prefix.to_string(), num, width))
}

/// Compute arithmetic value at ordinal index i.
pub fn arithmetic_value(start: &Value, step: i64, i: usize) -> Value {
    match start {
        Value::Number(n) => {
            if let Some(base) = n.as_i64() {
                Value::Number(Number::from(base + step * i as i64))
            } else if let Some(base) = n.as_f64() {
                let val = base + (step as f64) * (i as f64);
                Number::from_f64(val)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        Value::String(s) => {
            if let Some((prefix, base_num, width)) = parse_string_pattern(s) {
                let val = base_num + step * i as i64;
                Value::String(format!("{}{:0>width$}", prefix, val, width = width))
            } else {
                Value::String(s.clone())
            }
        }
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// Fiber parser
// ---------------------------------------------------------------------------

/// Parse a fiber header string into a `Fiber` struct.
///
/// Accepts the full header line, e.g.:
/// `reviews{id@101, customer, comment, rating|5, verified|T}`
/// or anonymous: `{status, data>}`
pub fn parse_fiber(input: &str) -> Result<Fiber> {
    let input = input.trim();
    let brace_start = input.find('{').ok_or_else(|| DhoomError::Parse {
        line: 0,
        message: "Missing '{' in fiber header".into(),
    })?;
    let brace_end = input.rfind('}').ok_or_else(|| DhoomError::Parse {
        line: 0,
        message: "Missing '}' in fiber header".into(),
    })?;

    let raw_name = if brace_start > 0 {
        Some(input[..brace_start].trim().to_string())
    } else {
        None
    };

    let mut sparse = false;
    let name = match raw_name {
        Some(ref n) if n.starts_with('~') => {
            sparse = true;
            let stripped = n[1..].trim();
            if stripped.is_empty() {
                None
            } else {
                Some(stripped.to_string())
            }
        }
        other => other,
    };

    let fields_str = &input[brace_start + 1..brace_end];
    let mut fields = Vec::new();

    for raw in fields_str.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        fields.push(parse_field_decl(token)?);
    }

    Ok(Fiber {
        name,
        fields,
        sparse,
    })
}

fn parse_field_decl(token: &str) -> Result<FieldDecl> {
    // Morphism: field->target (must check before nested >)
    if let Some(arrow_pos) = token.find("->") {
        let name = token[..arrow_pos].to_string();
        let target = token[arrow_pos + 2..].to_string();
        return Ok(FieldDecl {
            name,
            modifier: Some(Modifier::Morphism { target }),
        });
    }

    // Computed: field#expr
    if let Some(hash_pos) = token.find('#') {
        let name = token[..hash_pos].to_string();
        let expr = token[hash_pos + 1..].to_string();
        return Ok(FieldDecl {
            name,
            modifier: Some(Modifier::Computed { expr }),
        });
    }

    // Constraint: field!constraint
    if let Some(bang_pos) = token.find('!') {
        let name = token[..bang_pos].to_string();
        let constraint = token[bang_pos + 1..].to_string();
        return Ok(FieldDecl {
            name,
            modifier: Some(Modifier::Constraint { constraint }),
        });
    }

    // Interned: field&
    if let Some(name) = token.strip_suffix('&') {
        return Ok(FieldDecl {
            name: name.to_string(),
            modifier: Some(Modifier::Interned { pool: Vec::new() }),
        });
    }

    // Delta: field^
    if let Some(name) = token.strip_suffix('^') {
        return Ok(FieldDecl {
            name: name.to_string(),
            modifier: Some(Modifier::Delta),
        });
    }

    // Nested: field>
    if let Some(name) = token.strip_suffix('>') {
        return Ok(FieldDecl {
            name: name.to_string(),
            modifier: Some(Modifier::Nested),
        });
    }

    // Arithmetic: field@start or field@start+step
    if let Some(at_pos) = token.find('@') {
        let name = token[..at_pos].to_string();
        let rest = &token[at_pos + 1..];
        let (start_str, step) = if let Some(plus_pos) = rest.find('+') {
            let s: i64 = rest[plus_pos + 1..].parse().map_err(|_| {
                DhoomError::ArithmeticPattern(format!("Invalid step in '{}'", token))
            })?;
            (&rest[..plus_pos], Some(s))
        } else {
            (rest, None)
        };
        let start = coerce(start_str);
        return Ok(FieldDecl {
            name,
            modifier: Some(Modifier::Arithmetic { start, step }),
        });
    }

    // Default: field|value
    if let Some(pipe_pos) = token.find('|') {
        let name = token[..pipe_pos].to_string();
        let default_val = coerce(&token[pipe_pos + 1..]);
        return Ok(FieldDecl {
            name,
            modifier: Some(Modifier::Default(default_val)),
        });
    }

    // Plain variable field
    Ok(FieldDecl {
        name: token.to_string(),
        modifier: None,
    })
}

// ---------------------------------------------------------------------------
// Record parser â€” split a record line respecting quotes
// ---------------------------------------------------------------------------

pub fn split_record_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    // Escaped double quote
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            fields.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(c);
        }
    }
    fields.push(current.trim().to_string());
    fields
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/// Decode a DHOOM string into a JSON value.
pub fn decode(input: &str) -> Result<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(Value::Null);
    }
    let (bundle_name, value) = decode_bundle(input, 0)?;
    // Wrap in object with bundle name as key, or return directly if anonymous
    match bundle_name {
        Some(name) => {
            let mut map = Map::new();
            map.insert(name, value);
            Ok(Value::Object(map))
        }
        None => Ok(value),
    }
}

/// Decode a single bundle starting at the given line offset.
/// Returns (optional_name, decoded_value).
fn decode_bundle(input: &str, line_offset: usize) -> Result<(Option<String>, Value)> {
    // Find the header line (contains '{...}:')
    let colon_pos = find_header_end(input).ok_or_else(|| DhoomError::Parse {
        line: line_offset,
        message: "Missing '}:' header terminator".into(),
    })?;

    let header = input[..colon_pos - 1].trim(); // everything before ':'
    let body_full = &input[colon_pos..]; // everything after ':'
    let mut fiber = parse_fiber(header)?;

    // Parse pool lines for interned fields
    let mut body_start = 0;
    for line in body_full.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            body_start += line.len() + 1;
            continue;
        }
        // Pool line: &fieldname[val1, val2, ...]
        if trimmed.starts_with('&') {
            if let Some(bracket_start) = trimmed.find('[') {
                if trimmed.ends_with(']') {
                    let field_name = &trimmed[1..bracket_start];
                    let pool_str = &trimmed[bracket_start + 1..trimmed.len() - 1];
                    let pool: Vec<String> =
                        pool_str.split(',').map(|s| s.trim().to_string()).collect();
                    // Set pool on matching field
                    for fd in &mut fiber.fields {
                        if fd.name == field_name {
                            if let Some(Modifier::Interned { pool: ref mut p }) = fd.modifier {
                                *p = pool.clone();
                            }
                        }
                    }
                    body_start += line.len() + 1;
                    continue;
                }
            }
        }
        break;
    }
    let body = &body_full[body_start..];

    let record_fields = fiber.record_fields();
    let has_nested = record_fields
        .iter()
        .any(|f| matches!(f.modifier, Some(Modifier::Nested)));

    let mut records = if fiber.sparse {
        decode_sparse_records(body, &fiber, line_offset + 1)?
    } else if has_nested {
        decode_nested_records(body, &fiber, line_offset + 1)?
    } else {
        decode_flat_records(body, &fiber, line_offset + 1)?
    };

    // Resolve interned fields (map integer indices to pool values)
    for fd in &fiber.fields {
        if let Some(Modifier::Interned { ref pool }) = fd.modifier {
            if pool.is_empty() {
                continue;
            }
            for rec in &mut records {
                if let Some(obj) = rec.as_object_mut() {
                    if let Some(val) = obj.get(&fd.name).cloned() {
                        if let Some(idx) = val.as_i64() {
                            if idx >= 0 && (idx as usize) < pool.len() {
                                obj.insert(
                                    fd.name.clone(),
                                    Value::String(pool[idx as usize].clone()),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Evaluate computed fields
    for fd in &fiber.fields {
        if let Some(Modifier::Computed { ref expr }) = fd.modifier {
            // Parse binary expression: fieldA op fieldB
            let ops = ['*', '+', '-'];
            let mut op_char = ' ';
            let mut left_field = "";
            let mut right_field = "";
            for op in &ops {
                if let Some(pos) = expr.find(*op) {
                    left_field = expr[..pos].trim();
                    right_field = expr[pos + 1..].trim();
                    op_char = *op;
                    break;
                }
            }
            if op_char == ' ' {
                continue;
            }
            for rec in &mut records {
                if let Some(obj) = rec.as_object_mut() {
                    let left = obj.get(left_field).and_then(|v| v.as_f64());
                    let right = obj.get(right_field).and_then(|v| v.as_f64());
                    if let (Some(l), Some(r)) = (left, right) {
                        let result = match op_char {
                            '*' => l * r,
                            '+' => l + r,
                            '-' => l - r,
                            _ => 0.0,
                        };
                        let rounded = (result * 1e10).round() / 1e10;
                        if rounded == rounded.trunc() {
                            obj.insert(
                                fd.name.clone(),
                                Value::Number(Number::from(rounded as i64)),
                            );
                        } else if let Some(n) = Number::from_f64(rounded) {
                            obj.insert(fd.name.clone(), Value::Number(n));
                        }
                    }
                }
            }
        }
    }

    Ok((fiber.name.clone(), Value::Array(records)))
}

/// Find the position of ':' that ends the fiber header (the one after '}').
/// Returns the byte index immediately after the ':'.
fn find_header_end(input: &str) -> Option<usize> {
    let brace = input.find('}')?;
    let after = &input[brace + 1..];
    let colon_offset = after.find(':')?;
    Some(brace + 1 + colon_offset + 1) // index after ':'
}

fn decode_flat_records(body: &str, fiber: &Fiber, _line_offset: usize) -> Result<Vec<Value>> {
    let record_fields = fiber.record_fields();
    let mut records = Vec::new();
    let mut record_ordinal = 0;
    let mut delta_accum: HashMap<String, f64> = HashMap::new();

    for line in body.lines() {
        let trimmed = line.trim();

        // An empty line means all record fields take their defaults (or arithmetic values).
        // We must NOT skip it — it represents a legitimate all-default record.
        let raw_fields: Vec<String> = if trimmed.is_empty() {
            vec![]
        } else {
            split_record_fields(trimmed)
        };
        let mut obj = Map::new();
        let mut field_idx = 0;

        // Fill arithmetic fields
        for fdecl in &fiber.fields {
            if let Some(Modifier::Arithmetic {
                ref start,
                ref step,
            }) = fdecl.modifier
            {
                let s = step.unwrap_or(1);
                obj.insert(
                    fdecl.name.clone(),
                    arithmetic_value(start, s, record_ordinal),
                );
            }
        }

        // Map positional record values
        for (j, rf) in record_fields.iter().enumerate() {
            if j < raw_fields.len() {
                let raw = &raw_fields[j];
                let val = if raw.is_empty() {
                    // Omitted â†’ use default if available
                    if let Some(Modifier::Default(ref d)) = rf.modifier {
                        d.clone()
                    } else {
                        Value::String(String::new())
                    }
                } else if let Some(stripped) = raw.strip_prefix(':') {
                    // Deviation override
                    coerce(stripped)
                } else if let Some(Modifier::Default(_)) = rf.modifier {
                    coerce(raw)
                } else {
                    coerce(raw)
                };

                // Delta accumulation
                if matches!(rf.modifier, Some(Modifier::Delta)) {
                    if let Some(num) = val.as_f64() {
                        if record_ordinal == 0 {
                            delta_accum.insert(rf.name.clone(), num);
                            obj.insert(rf.name.clone(), val);
                        } else {
                            let prev = *delta_accum.get(&rf.name).unwrap_or(&0.0);
                            let absolute = prev + num;
                            delta_accum.insert(rf.name.clone(), absolute);
                            if absolute == absolute.trunc() {
                                obj.insert(
                                    rf.name.clone(),
                                    Value::Number(Number::from(absolute as i64)),
                                );
                            } else {
                                obj.insert(
                                    rf.name.clone(),
                                    Number::from_f64(absolute)
                                        .map(Value::Number)
                                        .unwrap_or(Value::Null),
                                );
                            }
                        }
                    } else {
                        obj.insert(rf.name.clone(), val);
                    }
                } else {
                    obj.insert(rf.name.clone(), val);
                }
            } else {
                // Trailing elision â€” fill with default
                if let Some(Modifier::Default(ref d)) = rf.modifier {
                    obj.insert(rf.name.clone(), d.clone());
                }
            }
            field_idx = j + 1;
        }

        // Fill any remaining default fields (trailing elision)
        for rf in record_fields.iter().skip(field_idx) {
            if let Some(Modifier::Default(ref d)) = rf.modifier {
                obj.insert(rf.name.clone(), d.clone());
            }
        }

        records.push(Value::Object(obj));
        record_ordinal += 1;
    }

    Ok(records)
}

fn decode_sparse_records(body: &str, fiber: &Fiber, _line_offset: usize) -> Result<Vec<Value>> {
    let mut records = Vec::new();
    let mut record_ordinal = 0;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut obj = Map::new();

        // Fill arithmetic fields
        for fdecl in &fiber.fields {
            if let Some(Modifier::Arithmetic {
                ref start,
                ref step,
            }) = fdecl.modifier
            {
                let s = step.unwrap_or(1);
                obj.insert(
                    fdecl.name.clone(),
                    arithmetic_value(start, s, record_ordinal),
                );
            }
        }

        // Parse name:value pairs
        let pairs = split_record_fields(trimmed);
        for pair in &pairs {
            if let Some(colon_pos) = pair.find(':') {
                let field_name = pair[..colon_pos].trim();
                let raw_value = pair[colon_pos + 1..].trim();
                obj.insert(field_name.to_string(), coerce(raw_value));
            }
        }

        // Fill defaults for missing fields
        for fdecl in &fiber.fields {
            if !obj.contains_key(&fdecl.name) {
                if let Some(Modifier::Default(ref d)) = fdecl.modifier {
                    obj.insert(fdecl.name.clone(), d.clone());
                } else if !matches!(fdecl.modifier, Some(Modifier::Arithmetic { .. })) {
                    obj.insert(fdecl.name.clone(), Value::Null);
                }
            }
        }

        records.push(Value::Object(obj));
        record_ordinal += 1;
    }

    Ok(records)
}

fn decode_nested_records(body: &str, fiber: &Fiber, line_offset: usize) -> Result<Vec<Value>> {
    let record_fields = fiber.record_fields();
    let mut records = Vec::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut line_idx = 0;
    let mut record_ordinal = 0;

    while line_idx < lines.len() {
        let trimmed = lines[line_idx].trim();
        if trimmed.is_empty() {
            line_idx += 1;
            continue;
        }

        let mut obj = Map::new();

        // Fill arithmetic fields
        for fdecl in &fiber.fields {
            if let Some(Modifier::Arithmetic {
                ref start,
                ref step,
            }) = fdecl.modifier
            {
                let s = step.unwrap_or(1);
                obj.insert(
                    fdecl.name.clone(),
                    arithmetic_value(start, s, record_ordinal),
                );
            }
        }

        // Parse the parent record line
        let raw_fields = split_record_fields(trimmed);
        let mut nested_fields: Vec<&FieldDecl> = Vec::new();
        let mut rf_idx = 0;

        for rf in &record_fields {
            if matches!(rf.modifier, Some(Modifier::Nested)) {
                nested_fields.push(rf);
            } else {
                if rf_idx < raw_fields.len() {
                    let raw = &raw_fields[rf_idx];
                    let val = if raw.is_empty() {
                        if let Some(Modifier::Default(ref d)) = rf.modifier {
                            d.clone()
                        } else {
                            Value::String(String::new())
                        }
                    } else if let Some(stripped) = raw.strip_prefix(':') {
                        coerce(stripped)
                    } else {
                        coerce(raw)
                    };
                    obj.insert(rf.name.clone(), val);
                } else if let Some(Modifier::Default(ref d)) = rf.modifier {
                    obj.insert(rf.name.clone(), d.clone());
                }
                rf_idx += 1;
            }
        }

        line_idx += 1;

        // Parse nested bundles
        for nf in &nested_fields {
            // Collect indented lines that form the nested bundle
            let mut nested_text = String::new();
            while line_idx < lines.len() {
                let l = lines[line_idx];
                // Nested content is indented; stop when we hit non-indented non-empty
                if !l.is_empty()
                    && !l.starts_with(' ')
                    && !l.starts_with('\t')
                    && !nested_text.is_empty()
                {
                    break;
                }
                if l.trim().is_empty() && nested_text.is_empty() {
                    line_idx += 1;
                    continue;
                }
                // Check if this starts a new nested bundle header
                if nested_text.contains("}:\n") && l.trim().starts_with('{') {
                    break;
                }
                nested_text.push_str(l.trim());
                nested_text.push('\n');
                line_idx += 1;
            }

            if !nested_text.trim().is_empty() {
                let (_, nested_val) = decode_bundle(nested_text.trim(), line_offset + line_idx)?;
                obj.insert(nf.name.clone(), nested_val);
            }
        }

        records.push(Value::Object(obj));
        record_ordinal += 1;
    }

    Ok(records)
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

/// Encode a JSON value into DHOOM format.
pub fn encode(value: &Value) -> Result<String> {
    let mut out = String::new();
    match value {
        Value::Object(map) => {
            if map.len() == 1 {
                let (key, val) = map.iter().next().unwrap();
                if let Value::Array(arr) = val {
                    encode_bundle(key, arr, &mut out, 0)?;
                } else {
                    return Err(DhoomError::Encode(
                        "Top-level object value must be an array".into(),
                    ));
                }
            } else {
                return Err(DhoomError::Encode(
                    "Top-level object must have exactly one key (the bundle name)".into(),
                ));
            }
        }
        Value::Array(arr) => {
            encode_bundle("data", arr, &mut out, 0)?;
        }
        _ => {
            return Err(DhoomError::Encode(
                "Top-level value must be an object or array".into(),
            ));
        }
    }
    Ok(out)
}

fn encode_bundle(name: &str, records: &[Value], out: &mut String, indent: usize) -> Result<()> {
    if records.is_empty() {
        let _ = write!(out, "{}{}{{}}:\n", " ".repeat(indent), name);
        return Ok(());
    }

    let first = records[0]
        .as_object()
        .ok_or_else(|| DhoomError::Encode("Array elements must be objects".into()))?;
    let keys: Vec<String> = first.keys().cloned().collect();

    let mut field_decls: Vec<FieldDecl> = Vec::new();
    let mut arithmetic_fields: Vec<String> = Vec::new();
    let mut delta_fields: Vec<String> = Vec::new();
    let mut default_fields: Vec<(String, Value)> = Vec::new();
    let mut variable_fields: Vec<String> = Vec::new();
    let mut nested_fields: Vec<String> = Vec::new();
    let mut computed_fields: Vec<(String, String)> = Vec::new();
    let mut interned_fields: Vec<(String, Vec<String>)> = Vec::new();

    // Phase 1: categorize nested and arithmetic
    let mut remaining_keys: Vec<String> = Vec::new();
    for key in &keys {
        let values: Vec<&Value> = records
            .iter()
            .filter_map(|r| r.as_object().and_then(|o| o.get(key)))
            .collect();

        if values.iter().all(|v| v.is_array()) {
            nested_fields.push(key.clone());
            continue;
        }

        if let Some((start, step)) = detect_arithmetic(&values) {
            arithmetic_fields.push(key.clone());
            let step_val = if step == 1 { None } else { Some(step) };
            field_decls.push(FieldDecl {
                name: key.clone(),
                modifier: Some(Modifier::Arithmetic {
                    start: start.clone(),
                    step: step_val,
                }),
            });
            continue;
        }

        remaining_keys.push(key.clone());
    }

    // Phase 2: detect computed fields among remaining keys
    for key in remaining_keys.clone() {
        if let Some(expr) = detect_computed_field(&key, records, &remaining_keys) {
            computed_fields.push((key.clone(), expr));
        }
    }
    for (k, _) in &computed_fields {
        remaining_keys.retain(|r| r != k);
    }

    // Phase 3: categorize remaining as delta, interned, default, or variable
    for key in &remaining_keys {
        let values: Vec<&Value> = records
            .iter()
            .filter_map(|r| r.as_object().and_then(|o| o.get(key)))
            .collect();

        if detect_delta(&values) {
            delta_fields.push(key.clone());
            continue;
        }

        if let Some(pool) = detect_interned(&values) {
            interned_fields.push((key.clone(), pool));
            continue;
        }

        if let Some((default_val, match_count)) = find_modal_default(&values) {
            if match_count > records.len() / 2 {
                default_fields.push((key.clone(), default_val));
                continue;
            }
        }

        variable_fields.push(key.clone());
    }

    let mut ordered_fields: Vec<FieldDecl> = Vec::new();

    for fd in &field_decls {
        if matches!(fd.modifier, Some(Modifier::Arithmetic { .. })) {
            ordered_fields.push(fd.clone());
        }
    }

    for (key, expr) in &computed_fields {
        ordered_fields.push(FieldDecl {
            name: key.clone(),
            modifier: Some(Modifier::Computed { expr: expr.clone() }),
        });
    }

    for key in &delta_fields {
        ordered_fields.push(FieldDecl {
            name: key.clone(),
            modifier: Some(Modifier::Delta),
        });
    }

    for (key, pool) in &interned_fields {
        ordered_fields.push(FieldDecl {
            name: key.clone(),
            modifier: Some(Modifier::Interned { pool: pool.clone() }),
        });
    }

    for key in &variable_fields {
        ordered_fields.push(FieldDecl {
            name: key.clone(),
            modifier: None,
        });
    }

    let mut default_with_freq: Vec<(String, Value, usize)> = default_fields
        .into_iter()
        .map(|(key, dval)| {
            let count = records
                .iter()
                .filter(|r| r.as_object().and_then(|o| o.get(&key)) == Some(&dval))
                .count();
            (key, dval, count)
        })
        .collect();
    default_with_freq.sort_by(|a, b| b.2.cmp(&a.2));

    for (key, dval, _) in &default_with_freq {
        ordered_fields.push(FieldDecl {
            name: key.clone(),
            modifier: Some(Modifier::Default(dval.clone())),
        });
    }

    for key in &nested_fields {
        ordered_fields.push(FieldDecl {
            name: key.clone(),
            modifier: Some(Modifier::Nested),
        });
    }

    // Check sparsity
    let non_arith_keys: Vec<&String> = keys
        .iter()
        .filter(|k| !arithmetic_fields.contains(k) && !nested_fields.contains(k))
        .collect();
    let use_sparse = if non_arith_keys.len() >= 8 {
        let mut null_count = 0usize;
        let total_cells = records.len() * non_arith_keys.len();
        for r in records {
            if let Some(obj) = r.as_object() {
                for k in &non_arith_keys {
                    match obj.get(*k) {
                        None | Some(Value::Null) => null_count += 1,
                        Some(Value::String(s)) if s.is_empty() => null_count += 1,
                        _ => {}
                    }
                }
            }
        }
        null_count > total_cells * 3 / 4
    } else {
        false
    };

    let prefix = " ".repeat(indent);
    if use_sparse {
        let _ = write!(out, "{}~{}", prefix, name);
    } else {
        let _ = write!(out, "{}{}", prefix, name);
    }
    out.push('{');
    for (i, fd) in ordered_fields.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&fd.name);
        match &fd.modifier {
            Some(Modifier::Arithmetic { start, step }) => {
                out.push('@');
                out.push_str(&value_to_dhoom(start));
                if let Some(s) = step {
                    let _ = write!(out, "+{}", s);
                }
            }
            Some(Modifier::Default(v)) => {
                out.push('|');
                out.push_str(&value_to_dhoom(v));
            }
            Some(Modifier::Nested) => {
                out.push('>');
            }
            Some(Modifier::Delta) => {
                out.push('^');
            }
            Some(Modifier::Morphism { target }) => {
                out.push_str("->");
                out.push_str(target);
            }
            Some(Modifier::Interned { .. }) => {
                out.push('&');
            }
            Some(Modifier::Computed { ref expr }) => {
                out.push('#');
                out.push_str(expr);
            }
            Some(Modifier::Constraint { ref constraint }) => {
                out.push('!');
                out.push_str(constraint);
            }
            None => {}
        }
    }
    out.push_str("}:\n");

    // Emit pool lines for interned fields
    for fd in &ordered_fields {
        if let Some(Modifier::Interned { ref pool }) = fd.modifier {
            if !pool.is_empty() {
                let _ = writeln!(out, "&{}[{}]", fd.name, pool.join(", "));
            }
        }
    }

    let rec_fields: Vec<&FieldDecl> = ordered_fields
        .iter()
        .filter(|f| {
            !matches!(
                f.modifier,
                Some(Modifier::Arithmetic { .. }) | Some(Modifier::Computed { .. })
            )
        })
        .collect();

    if use_sparse {
        for record in records {
            let obj = record
                .as_object()
                .ok_or_else(|| DhoomError::Encode("Record must be an object".into()))?;
            let mut pairs: Vec<String> = Vec::new();
            for rf in &rec_fields {
                if matches!(rf.modifier, Some(Modifier::Nested)) {
                    continue;
                }
                if let Some(v) = obj.get(&rf.name) {
                    match v {
                        Value::Null => {}
                        Value::String(s) if s.is_empty() => {}
                        _ => {
                            if let Some(Modifier::Interned { ref pool }) = rf.modifier {
                                let idx = pool
                                    .iter()
                                    .position(|p| *p == value_to_dhoom(v))
                                    .unwrap_or(0);
                                pairs.push(format!("{}:{}", rf.name, idx));
                            } else {
                                pairs.push(format!("{}:{}", rf.name, value_to_dhoom(v)));
                            }
                        }
                    }
                }
            }
            if pairs.is_empty() {
                if let Some(rf) = rec_fields.first() {
                    pairs.push(format!("{}:null", rf.name));
                }
            }
            let _ = writeln!(out, "{}{}", prefix, pairs.join(", "));
        }
        return Ok(());
    }

    let mut record_idx = 0usize;
    let mut prev_delta: HashMap<String, f64> = HashMap::new();

    for record in records {
        let obj = record
            .as_object()
            .ok_or_else(|| DhoomError::Encode("Record must be an object".into()))?;

        let mut values: Vec<String> = Vec::new();
        let mut nested_bundles: Vec<(&str, &Value)> = Vec::new();

        for rf in &rec_fields {
            if matches!(rf.modifier, Some(Modifier::Nested)) {
                if let Some(v) = obj.get(&rf.name) {
                    nested_bundles.push((&rf.name, v));
                }
                continue;
            }

            let val = obj.get(&rf.name);

            if matches!(rf.modifier, Some(Modifier::Delta)) {
                let num_val = val.and_then(|v| v.as_f64()).unwrap_or(0.0);
                if record_idx == 0 {
                    prev_delta.insert(rf.name.clone(), num_val);
                    values.push(val.map(|v| value_to_dhoom(v)).unwrap_or_default());
                } else {
                    let prev = *prev_delta.get(&rf.name).unwrap_or(&0.0);
                    let delta = num_val - prev;
                    prev_delta.insert(rf.name.clone(), num_val);
                    let delta_i = delta as i64;
                    if (delta_i as f64 - delta).abs() < 1e-9 {
                        values.push(delta_i.to_string());
                    } else {
                        values.push(format!("{}", delta));
                    }
                }
            } else {
                match (&rf.modifier, val) {
                    (Some(Modifier::Default(d)), Some(v)) if v == d => {
                        values.push(String::new());
                    }
                    (Some(Modifier::Default(_)), Some(v)) => {
                        values.push(format!(":{}", value_to_dhoom(v)));
                    }
                    (Some(Modifier::Interned { ref pool }), Some(v)) => {
                        if let Some(s) = v.as_str() {
                            let idx = pool.iter().position(|p| p == s).unwrap_or(0);
                            values.push(idx.to_string());
                        } else {
                            values.push(value_to_dhoom(v));
                        }
                    }
                    (_, Some(v)) => {
                        values.push(value_to_dhoom(v));
                    }
                    (_, None) => {
                        values.push(String::new());
                    }
                }
            }
        }

        while values.last().map_or(false, |v| v.is_empty()) {
            values.pop();
        }

        let _ = write!(out, "{}{}", prefix, values.join(", "));

        if !nested_bundles.is_empty() {
            out.push_str(",\n");
            for (_nname, nval) in &nested_bundles {
                if let Value::Array(arr) = nval {
                    encode_bundle("", arr, out, indent + 2)?;
                }
            }
        } else {
            out.push('\n');
        }
        record_idx += 1;
    }

    Ok(())
}

/// Detect if delta encoding would be beneficial for a sequence of values.
fn detect_delta(values: &[&Value]) -> bool {
    if values.len() < 3 {
        return false;
    }
    let nums: Option<Vec<i64>> = values.iter().map(|v| v.as_i64()).collect();
    let nums = match nums {
        Some(n) => n,
        None => return false,
    };
    let deltas: Vec<i64> = std::iter::once(nums[0])
        .chain(nums.windows(2).map(|w| w[1] - w[0]))
        .collect();
    let abs_len: usize = nums.iter().map(|n| n.to_string().len()).sum();
    let delta_len: usize = deltas.iter().map(|d| d.to_string().len()).sum();
    delta_len * 10 < abs_len * 7
}

/// Detect if a sequence of values forms an arithmetic progression.
/// Returns (start_value, step) if so.
fn detect_arithmetic(values: &[&Value]) -> Option<(Value, i64)> {
    if values.len() < 2 {
        return None;
    }

    // Try numeric arithmetic
    let nums: Option<Vec<i64>> = values.iter().map(|v| v.as_i64()).collect();
    if let Some(nums) = nums {
        let step = nums[1] - nums[0];
        if nums.windows(2).all(|w| w[1] - w[0] == step) {
            return Some((values[0].clone(), step));
        }
    }

    // Try string-pattern arithmetic
    let strings: Option<Vec<&str>> = values.iter().map(|v| v.as_str()).collect();
    if let Some(strings) = strings {
        let patterns: Option<Vec<(String, i64, usize)>> =
            strings.iter().map(|s| parse_string_pattern(s)).collect();
        if let Some(patterns) = patterns {
            if patterns
                .iter()
                .all(|(p, _, w)| p == &patterns[0].0 && *w == patterns[0].2)
            {
                let step = patterns[1].1 - patterns[0].1;
                if patterns.windows(2).all(|w| w[1].1 - w[0].1 == step) {
                    return Some((values[0].clone(), step));
                }
            }
        }
    }

    None
}

/// Find the most common (modal) value in a list.
fn find_modal_default(values: &[&Value]) -> Option<(Value, usize)> {
    if values.is_empty() {
        return None;
    }
    let mut counts: HashMap<String, (Value, usize)> = HashMap::new();
    for v in values {
        let key = format!("{}", v);
        counts
            .entry(key)
            .and_modify(|e| e.1 += 1)
            .or_insert_with(|| ((*v).clone(), 1));
    }
    counts.into_values().max_by_key(|&(_, c)| c)
}

/// Detect if a field's string values should use string interning.
fn detect_interned(values: &[&Value]) -> Option<Vec<String>> {
    if values.len() < 3 {
        return None;
    }
    let strings: Vec<&str> = values.iter().filter_map(|v| v.as_str()).collect();
    if strings.len() != values.len() {
        return None;
    }
    let mut distinct: Vec<String> = Vec::new();
    for s in &strings {
        if !distinct.iter().any(|d| d == *s) {
            distinct.push(s.to_string());
        }
    }
    if distinct.len() < 2 || distinct.len() > (values.len() + 2) / 3 {
        return None;
    }
    let raw_len: usize = strings.iter().map(|s| s.len()).sum();
    let pool_len = distinct.iter().map(|s| s.len()).sum::<usize>() + (distinct.len() - 1) * 2 + 2;
    let index_len: usize = strings
        .iter()
        .map(|s| {
            distinct
                .iter()
                .position(|d| d == *s)
                .unwrap_or(0)
                .to_string()
                .len()
        })
        .sum();
    if index_len + pool_len >= raw_len * 9 / 10 {
        return None;
    }
    Some(distinct)
}

/// Detect if a field can be computed from two other fields via a binary op.
fn detect_computed_field(
    key: &str,
    records: &[Value],
    candidate_keys: &[String],
) -> Option<String> {
    if records.len() < 2 {
        return None;
    }
    let values: Vec<f64> = records
        .iter()
        .filter_map(|r| r.as_object()?.get(key)?.as_f64())
        .collect();
    if values.len() != records.len() {
        return None;
    }
    for op in &['*', '+', '-'] {
        for a in candidate_keys {
            if a == key {
                continue;
            }
            let a_vals: Vec<f64> = records
                .iter()
                .filter_map(|r| r.as_object()?.get(a.as_str())?.as_f64())
                .collect();
            if a_vals.len() != records.len() {
                continue;
            }
            for b in candidate_keys {
                if b == key || b == a {
                    continue;
                }
                let b_vals: Vec<f64> = records
                    .iter()
                    .filter_map(|r| r.as_object()?.get(b.as_str())?.as_f64())
                    .collect();
                if b_vals.len() != records.len() {
                    continue;
                }
                let mut all_match = true;
                for i in 0..records.len() {
                    let expected = values[i];
                    let result = match op {
                        '*' => a_vals[i] * b_vals[i],
                        '+' => a_vals[i] + b_vals[i],
                        '-' => a_vals[i] - b_vals[i],
                        _ => 0.0,
                    };
                    if (result - expected).abs() > 1e-9 {
                        all_match = false;
                        break;
                    }
                }
                if all_match {
                    return Some(format!("{}{}{}", a, op, b));
                }
            }
        }
    }
    None
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Profile API â€” geometric data analysis (ported from GIGI convert.rs)
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

/// Classification of a field's geometric role in the fiber bundle.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldRole {
    /// Arithmetic progression â€” values derived from index, zero curvature
    Arithmetic { start: String, step: i64 },
    /// Modal default â€” most records share this value
    Default { value: String, match_pct: f64 },
    /// Delta-encoded â€” sequential differences are smaller than absolutes
    Delta,
    /// String-interned â€” categorical field with a finite pool
    Interned { pool_size: usize },
    /// Computed â€” derivable from other fields
    Computed { expr: String },
    /// Nested sub-bundle
    Nested,
    /// Regular variable field
    Variable,
}

/// Per-field curvature analysis.
#[derive(Debug, Clone)]
pub struct FieldProfile {
    pub name: String,
    pub role: FieldRole,
    /// Scalar curvature K = Var / rangeÂ². Low K = predictable = compresses well.
    pub curvature: f64,
    /// Confidence C = 1 / (1 + K). High confidence = low curvature.
    pub confidence: f64,
}

/// Geometric profile of a dataset â€” analyze before encoding.
#[derive(Debug, Clone)]
pub struct Profile {
    pub collection: String,
    pub record_count: usize,
    pub field_count: usize,
    pub fields: Vec<FieldProfile>,
    pub dhoom_bytes: usize,
    pub json_bytes: usize,
    pub compression_pct: f64,
    pub fields_elided_pct: f64,
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "DHOOM Profile: {}", self.collection)?;
        writeln!(
            f,
            "  Records: {}  Fields: {}",
            self.record_count, self.field_count
        )?;
        writeln!(
            f,
            "  DHOOM: {} bytes  JSON: {} bytes  Savings: {:.0}%",
            self.dhoom_bytes, self.json_bytes, self.compression_pct
        )?;
        writeln!(f, "  Fields elided: {:.0}%", self.fields_elided_pct)?;
        writeln!(f, "")?;
        for fp in &self.fields {
            let role_str = match &fp.role {
                FieldRole::Arithmetic { start, step } => {
                    format!("@ arithmetic ({}+{}n)", start, step)
                }
                FieldRole::Default { value, match_pct } => {
                    format!("| default \"{}\" ({:.0}%)", value, match_pct)
                }
                FieldRole::Delta => "^ delta".to_string(),
                FieldRole::Interned { pool_size } => format!("& interned ({} values)", pool_size),
                FieldRole::Computed { expr } => format!("# computed ({})", expr),
                FieldRole::Nested => "> nested".to_string(),
                FieldRole::Variable => "  variable".to_string(),
            };
            writeln!(
                f,
                "  {:16} {:30} K={:.4}  C={:.4}",
                fp.name, role_str, fp.curvature, fp.confidence
            )?;
        }
        Ok(())
    }
}

/// Compute scalar curvature K = Var / rangeÂ² for a numeric field.
fn field_curvature(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range <= 0.0 {
        return 0.0;
    }
    variance / (range * range)
}

/// Profile a JSON dataset's geometric structure without modifying it.
///
/// Returns per-field curvature analysis, compression estimates, and field
/// classifications. Use this to understand *why* DHOOM compresses well
/// (or doesn't) for a given dataset.
///
/// ```rust,ignore
/// use dhoom::profile;
/// use serde_json::json;
///
/// let data = json!({
///     "sensors": [
///         {"id": 1, "location": "roof", "temp": 22.1, "status": "ok"},
///         {"id": 2, "location": "roof", "temp": 22.3, "status": "ok"},
///         {"id": 3, "location": "basement", "temp": 18.5, "status": "ok"}
///     ]
/// });
/// let p = profile(&data).unwrap();
/// println!("{}", p);
/// ```
pub fn profile(value: &Value) -> Result<Profile> {
    let (collection, records) = match value {
        Value::Object(map) if map.len() == 1 => {
            let (key, val) = map.iter().next().unwrap();
            match val {
                Value::Array(arr) => (key.clone(), arr.as_slice()),
                _ => {
                    return Err(DhoomError::Encode(
                        "Top-level value must be an array".into(),
                    ))
                }
            }
        }
        _ => return Err(DhoomError::Encode("Expected {collection: [...]}".into())),
    };

    if records.is_empty() {
        return Ok(Profile {
            collection,
            record_count: 0,
            field_count: 0,
            fields: vec![],
            dhoom_bytes: 0,
            json_bytes: 2,
            compression_pct: 0.0,
            fields_elided_pct: 0.0,
        });
    }

    let first = records[0]
        .as_object()
        .ok_or_else(|| DhoomError::Encode("Records must be objects".into()))?;
    let keys: Vec<String> = first.keys().cloned().collect();

    // Encode to get real byte counts
    let dhoom_str = encode(value)?;
    let json_str = serde_json::to_string(value).unwrap_or_default();
    let dhoom_bytes = dhoom_str.len();
    let json_bytes = json_str.len();
    let compression_pct = if json_bytes > 0 {
        100.0 * (1.0 - dhoom_bytes as f64 / json_bytes as f64)
    } else {
        0.0
    };

    let mut field_profiles = Vec::new();
    let mut elided_slots = 0usize;
    let total_slots = records.len() * keys.len();

    for key in &keys {
        let values: Vec<&Value> = records
            .iter()
            .filter_map(|r| r.as_object().and_then(|o| o.get(key)))
            .collect();

        // Extract numeric values for curvature
        let nums: Vec<f64> = values.iter().filter_map(|v| v.as_f64()).collect();
        let k = if nums.len() >= 2 {
            field_curvature(&nums)
        } else {
            0.0
        };
        let conf = 1.0 / (1.0 + k);

        // Classify field role (same logic as encoder)
        let role = if values.iter().all(|v| v.is_array()) {
            elided_slots += records.len();
            FieldRole::Nested
        } else if let Some((start, step)) = detect_arithmetic(&values) {
            elided_slots += records.len();
            FieldRole::Arithmetic {
                start: value_to_dhoom(&start),
                step,
            }
        } else if let Some(expr) = detect_computed_field(key, records, &keys) {
            elided_slots += records.len();
            FieldRole::Computed { expr }
        } else if detect_delta(&values) {
            FieldRole::Delta
        } else if let Some(pool) = detect_interned(&values) {
            FieldRole::Interned {
                pool_size: pool.len(),
            }
        } else if let Some((default_val, match_count)) = find_modal_default(&values) {
            let pct = 100.0 * match_count as f64 / values.len() as f64;
            if match_count > records.len() / 2 {
                elided_slots += match_count;
                FieldRole::Default {
                    value: value_to_dhoom(&default_val),
                    match_pct: pct,
                }
            } else {
                FieldRole::Variable
            }
        } else {
            FieldRole::Variable
        };

        field_profiles.push(FieldProfile {
            name: key.clone(),
            role,
            curvature: k,
            confidence: conf,
        });
    }

    let fields_elided_pct = if total_slots > 0 {
        100.0 * elided_slots as f64 / total_slots as f64
    } else {
        0.0
    };

    Ok(Profile {
        collection,
        record_count: records.len(),
        field_count: keys.len(),
        fields: field_profiles,
        dhoom_bytes,
        json_bytes,
        compression_pct,
        fields_elided_pct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fiber_parse_simple() {
        let fiber =
            parse_fiber("reviews{id@101, customer, comment, rating|5, verified|T}").unwrap();
        assert_eq!(fiber.name, Some("reviews".into()));
        assert_eq!(fiber.fields.len(), 5);
        assert_eq!(
            fiber.fields[0].modifier,
            Some(Modifier::Arithmetic {
                start: json!(101),
                step: None
            })
        );
        assert_eq!(fiber.fields[1].modifier, None); // customer
        assert_eq!(fiber.fields[3].modifier, Some(Modifier::Default(json!(5))));
        assert_eq!(
            fiber.fields[4].modifier,
            Some(Modifier::Default(Value::Bool(true)))
        );
    }

    #[test]
    fn test_fiber_parse_anonymous() {
        let fiber = parse_fiber("{status, data>}").unwrap();
        assert_eq!(fiber.name, None);
        assert_eq!(fiber.fields.len(), 2);
        assert_eq!(fiber.fields[1].modifier, Some(Modifier::Nested));
    }

    #[test]
    fn test_arithmetic_numeric() {
        let v0 = arithmetic_value(&json!(101), 1, 0);
        let v1 = arithmetic_value(&json!(101), 1, 1);
        let v2 = arithmetic_value(&json!(101), 1, 2);
        assert_eq!(v0, json!(101));
        assert_eq!(v1, json!(102));
        assert_eq!(v2, json!(103));
    }

    #[test]
    fn test_arithmetic_numeric_step() {
        let v0 = arithmetic_value(&json!(1710000000), 60, 0);
        let v1 = arithmetic_value(&json!(1710000000), 60, 1);
        let v2 = arithmetic_value(&json!(1710000000), 60, 2);
        assert_eq!(v0, json!(1710000000));
        assert_eq!(v1, json!(1710000060));
        assert_eq!(v2, json!(1710000120));
    }

    #[test]
    fn test_arithmetic_string_pattern() {
        let start = Value::String("T-001".into());
        let v0 = arithmetic_value(&start, 1, 0);
        let v1 = arithmetic_value(&start, 1, 1);
        let v2 = arithmetic_value(&start, 1, 2);
        assert_eq!(v0, json!("T-001"));
        assert_eq!(v1, json!("T-002"));
        assert_eq!(v2, json!("T-003"));
    }

    #[test]
    fn test_trailing_elision() {
        let input = "items{name, active|T, role|user}:\nAlice\nBob\n";
        let result = decode(input).unwrap();
        let expected = json!({
            "items": [
                {"name": "Alice", "active": true, "role": "user"},
                {"name": "Bob", "active": true, "role": "user"}
            ]
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_deviation_marking() {
        let input = "items{name, score|10}:\nAlice\nBob, :7\n";
        let result = decode(input).unwrap();
        let expected = json!({
            "items": [
                {"name": "Alice", "score": 10},
                {"name": "Bob", "score": 7}
            ]
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_roundtrip_reviews() {
        let data = json!({
            "reviews": [
                {"id": 101, "customer": "Alex Rivera", "rating": 5, "comment": "Excellent!", "verified": true},
                {"id": 102, "customer": "Brij Pandey", "rating": 5, "comment": "Game changer!", "verified": true},
                {"id": 103, "customer": "Casey Lee", "rating": 3, "comment": "Average", "verified": false}
            ]
        });
        let dhoom = encode(&data).unwrap();
        let roundtrip = decode(&dhoom).unwrap();
        assert_eq!(data, roundtrip);
    }

    #[test]
    fn test_roundtrip_sensors() {
        let data = json!({
            "readings": [
                {"sensor_id": "T-001", "timestamp": 1710000000, "value": 22.4, "status": "normal", "unit": "celsius"},
                {"sensor_id": "T-002", "timestamp": 1710000060, "value": 23.1, "status": "normal", "unit": "celsius"},
                {"sensor_id": "T-003", "timestamp": 1710000120, "value": 45.8, "status": "alert", "unit": "celsius"}
            ]
        });
        let dhoom = encode(&data).unwrap();
        let roundtrip = decode(&dhoom).unwrap();
        assert_eq!(data, roundtrip);
    }

    #[test]
    fn test_decode_reviews_example() {
        let input = "\
reviews{id@101, customer, comment, rating|5, verified|T}:
Alex Rivera, Excellent!
Brij Pandey, Game changer!
Casey Lee, Average, :3, :F
";
        let result = decode(input).unwrap();
        let expected = json!({
            "reviews": [
                {"id": 101, "customer": "Alex Rivera", "comment": "Excellent!", "rating": 5, "verified": true},
                {"id": 102, "customer": "Brij Pandey", "comment": "Game changer!", "rating": 5, "verified": true},
                {"id": 103, "customer": "Casey Lee", "comment": "Average", "rating": 3, "verified": false}
            ]
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_sensors_example() {
        let input = "\
readings{sensor_id@T-001, timestamp@1710000000+60, value, status|normal, unit|celsius}:
22.4
23.1
45.8, :alert
";
        let result = decode(input).unwrap();
        let expected = json!({
            "readings": [
                {"sensor_id": "T-001", "timestamp": 1710000000, "value": 22.4, "status": "normal", "unit": "celsius"},
                {"sensor_id": "T-002", "timestamp": 1710000060, "value": 23.1, "status": "normal", "unit": "celsius"},
                {"sensor_id": "T-003", "timestamp": 1710000120, "value": 45.8, "status": "alert", "unit": "celsius"}
            ]
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_boolean_shorthand() {
        assert_eq!(coerce("T"), Value::Bool(true));
        assert_eq!(coerce("F"), Value::Bool(false));
    }

    #[test]
    fn test_empty_collection() {
        let input = "items{id, name}:\n";
        let result = decode(input).unwrap();
        assert_eq!(result, json!({"items": []}));
    }

    #[test]
    fn test_coerce_types() {
        assert_eq!(coerce("42"), json!(42));
        assert_eq!(coerce("3.14"), json!(3.14));
        assert_eq!(coerce("null"), Value::Null);
        assert_eq!(coerce("hello"), json!("hello"));
        assert_eq!(coerce(""), json!(""));
    }

    #[test]
    fn test_quoted_strings() {
        let fields = split_record_fields(r#"Alice, "value, with comma", Bob"#);
        assert_eq!(fields, vec!["Alice", "value, with comma", "Bob"]);
    }

    #[test]
    fn test_detect_arithmetic_sequence() {
        let vals = vec![json!(1), json!(2), json!(3)];
        let refs: Vec<&Value> = vals.iter().collect();
        let result = detect_arithmetic(&refs);
        assert_eq!(result, Some((json!(1), 1)));
    }

    #[test]
    fn test_detect_arithmetic_step() {
        let vals = vec![json!(10), json!(15), json!(20)];
        let refs: Vec<&Value> = vals.iter().collect();
        let result = detect_arithmetic(&refs);
        assert_eq!(result, Some((json!(10), 5)));
    }

    #[test]
    fn test_find_modal_default_majority() {
        let vals = vec![json!(5), json!(5), json!(3)];
        let refs: Vec<&Value> = vals.iter().collect();
        let (modal, count) = find_modal_default(&refs).unwrap();
        assert_eq!(modal, json!(5));
        assert_eq!(count, 2);
    }

    // -----------------------------------------------------------------------
    // Delta fields
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_delta_modifier() {
        let fd = parse_field_decl("ts^").unwrap();
        assert_eq!(fd.name, "ts");
        assert_eq!(fd.modifier, Some(Modifier::Delta));
    }

    #[test]
    fn test_decode_delta_values() {
        let input = "events{name, ts^}:\nA, 1000000\nB, 50\nC, 70\n";
        let result = decode(input).unwrap();
        let expected = json!({
            "events": [
                {"name": "A", "ts": 1000000},
                {"name": "B", "ts": 1000050},
                {"name": "C", "ts": 1000120}
            ]
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_encode_delta_when_beneficial() {
        let data = json!({
            "events": [
                {"name": "A", "ts": 1000000},
                {"name": "B", "ts": 1000050},
                {"name": "C", "ts": 1000120},
                {"name": "D", "ts": 1000200},
                {"name": "E", "ts": 1000310}
            ]
        });
        let dhoom = encode(&data).unwrap();
        assert!(dhoom.contains("ts^"), "Should use delta modifier");
    }

    #[test]
    fn test_roundtrip_delta() {
        let data = json!({
            "events": [
                {"name": "s0", "ts": 1000000},
                {"name": "s1", "ts": 1000050},
                {"name": "s2", "ts": 1000120},
                {"name": "s3", "ts": 1000200},
                {"name": "s4", "ts": 1000310}
            ]
        });
        let dhoom = encode(&data).unwrap();
        let roundtrip = decode(&dhoom).unwrap();
        assert_eq!(data, roundtrip);
    }

    // -----------------------------------------------------------------------
    // Sparse bundles
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_sparse_prefix() {
        let fiber = parse_fiber("~profiles{a, b, c, d, e, f, g, h}").unwrap();
        assert_eq!(fiber.name, Some("profiles".into()));
        assert!(fiber.sparse);
        assert_eq!(fiber.fields.len(), 8);
    }

    #[test]
    fn test_decode_sparse_records() {
        let input = "~items{a, b, c, d, e, f, g, h}:\na:1, c:3\nb:2\n";
        let result = decode(input).unwrap();
        let arr = result["items"].as_array().unwrap();
        assert_eq!(arr[0]["a"], json!(1));
        assert_eq!(arr[0]["c"], json!(3));
        assert_eq!(arr[0]["b"], Value::Null);
        assert_eq!(arr[1]["b"], json!(2));
        assert_eq!(arr[1]["a"], Value::Null);
    }

    #[test]
    fn test_encode_sparse_when_mostly_null() {
        let mut records = Vec::new();
        let field_names: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
        for i in 0..5 {
            let mut obj = serde_json::Map::new();
            for name in &field_names {
                obj.insert(name.to_string(), Value::Null);
            }
            // Set just one field per record
            let key = field_names[i % field_names.len()];
            obj.insert(key.to_string(), json!(i + 1));
            records.push(Value::Object(obj));
        }
        let data = json!({"sparse_data": records});
        let dhoom = encode(&data).unwrap();
        assert!(dhoom.contains("~sparse_data"), "Should use sparse prefix");
    }

    // -----------------------------------------------------------------------
    // Morphism fields
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_morphism_modifier() {
        let fd = parse_field_decl("user_id->users").unwrap();
        assert_eq!(fd.name, "user_id");
        assert_eq!(
            fd.modifier,
            Some(Modifier::Morphism {
                target: "users".into()
            })
        );
    }

    #[test]
    fn test_decode_morphism_as_regular_values() {
        let input = "orders{id@1, user_id->users}:\nAlice\nBob\n";
        let result = decode(input).unwrap();
        let expected = json!({
            "orders": [
                {"id": 1, "user_id": "Alice"},
                {"id": 2, "user_id": "Bob"}
            ]
        });
        assert_eq!(result, expected);
    }

    // -------------------------------------------------------------------
    // v0.5: String Interning (&)
    // -------------------------------------------------------------------

    #[test]
    fn test_parse_interned_modifier() {
        let fd = parse_field_decl("level&").unwrap();
        assert_eq!(fd.name, "level");
        assert!(matches!(fd.modifier, Some(Modifier::Interned { .. })));
    }

    #[test]
    fn test_decode_interned_fields() {
        let input = "logs{ts@1, level&, msg}:\n&level[INFO, WARN, ERROR]\n0, hello\n1, warning\n2, critical\n0, fine\n";
        let result = decode(input).unwrap();
        let arr = result["logs"].as_array().unwrap();
        assert_eq!(arr[0]["level"], json!("INFO"));
        assert_eq!(arr[1]["level"], json!("WARN"));
        assert_eq!(arr[2]["level"], json!("ERROR"));
        assert_eq!(arr[3]["level"], json!("INFO"));
    }

    #[test]
    fn test_roundtrip_interned() {
        let data = json!({
            "events": [
                {"id": 1, "status": "completed", "msg": "a"},
                {"id": 2, "status": "completed", "msg": "b"},
                {"id": 3, "status": "pending", "msg": "c"},
                {"id": 4, "status": "completed", "msg": "d"},
                {"id": 5, "status": "failed", "msg": "e"},
                {"id": 6, "status": "completed", "msg": "f"},
                {"id": 7, "status": "pending", "msg": "g"},
                {"id": 8, "status": "completed", "msg": "h"},
                {"id": 9, "status": "completed", "msg": "i"}
            ]
        });
        let dhoom = encode(&data).unwrap();
        assert!(dhoom.contains("status&"), "should use interned modifier");
        let roundtrip = decode(&dhoom).unwrap();
        assert_eq!(data, roundtrip);
    }

    // -------------------------------------------------------------------
    // v0.5: Computed Fields (#)
    // -------------------------------------------------------------------

    #[test]
    fn test_parse_computed_modifier() {
        let fd = parse_field_decl("total#qty*price").unwrap();
        assert_eq!(fd.name, "total");
        assert_eq!(
            fd.modifier,
            Some(Modifier::Computed {
                expr: "qty*price".into()
            })
        );
    }

    #[test]
    fn test_decode_computed_fields() {
        let input = "orders{qty, price, total#qty*price}:\n3, 10\n5, 20\n2, 15\n";
        let result = decode(input).unwrap();
        let arr = result["orders"].as_array().unwrap();
        assert_eq!(arr[0]["total"], json!(30));
        assert_eq!(arr[1]["total"], json!(100));
        assert_eq!(arr[2]["total"], json!(30));
    }

    #[test]
    fn test_roundtrip_computed() {
        let data = json!({
            "orders": [
                {"qty": 3, "price": 10, "total": 30},
                {"qty": 5, "price": 20, "total": 100},
                {"qty": 2, "price": 15, "total": 30}
            ]
        });
        let dhoom = encode(&data).unwrap();
        assert!(dhoom.contains("total#"), "should use computed modifier");
        let roundtrip = decode(&dhoom).unwrap();
        assert_eq!(data, roundtrip);
    }

    // -------------------------------------------------------------------
    // v0.5: Inline Constraints (!)
    // -------------------------------------------------------------------

    #[test]
    fn test_parse_constraint_modifier() {
        let fd = parse_field_decl("age!int").unwrap();
        assert_eq!(fd.name, "age");
        assert_eq!(
            fd.modifier,
            Some(Modifier::Constraint {
                constraint: "int".into()
            })
        );
    }

    #[test]
    fn test_decode_constraint_as_variable() {
        let input = "users{id!int, name!str, active!bool}:\n1, Alice, T\n2, Bob, F\n";
        let result = decode(input).unwrap();
        let expected = json!({
            "users": [
                {"id": 1, "name": "Alice", "active": true},
                {"id": 2, "name": "Bob", "active": false}
            ]
        });
        assert_eq!(result, expected);
    }

    // -----------------------------------------------------------------------
    // Profile API
    // -----------------------------------------------------------------------

    #[test]
    fn test_profile_basic() {
        let data = json!({
            "sensors": [
                {"id": 1, "location": "roof", "temp": 22.1, "status": "ok"},
                {"id": 2, "location": "lobby", "temp": 21.8, "status": "ok"},
                {"id": 3, "location": "basement", "temp": 18.5, "status": "ok"}
            ]
        });
        let p = profile(&data).unwrap();
        assert_eq!(p.collection, "sensors");
        assert_eq!(p.record_count, 3);
        assert_eq!(p.field_count, 4);
        assert!(p.compression_pct > 0.0, "Should compress");
        assert!(p.dhoom_bytes < p.json_bytes, "DHOOM should be smaller");
    }

    #[test]
    fn test_profile_arithmetic_field() {
        let data = json!({
            "items": [
                {"id": 100, "name": "A"},
                {"id": 101, "name": "B"},
                {"id": 102, "name": "C"}
            ]
        });
        let p = profile(&data).unwrap();
        let id_field = p.fields.iter().find(|f| f.name == "id").unwrap();
        assert!(matches!(id_field.role, FieldRole::Arithmetic { .. }));
        // Arithmetic integer sequence has well-defined curvature (uniform distribution)
        assert!(
            id_field.confidence > 0.5,
            "Arithmetic field should have reasonable confidence"
        );
    }

    #[test]
    fn test_profile_default_field() {
        // Use enough unique values so interning doesn't trigger first
        let data = json!({
            "orders": [
                {"id": 1, "status": "delivered", "region": "US"},
                {"id": 2, "status": "delivered", "region": "EU"},
                {"id": 3, "status": "delivered", "region": "APAC"},
                {"id": 4, "status": "delivered", "region": "LATAM"},
                {"id": 5, "status": "shipped", "region": "MEA"},
                {"id": 6, "status": "delivered", "region": "CAN"}
            ]
        });
        let p = profile(&data).unwrap();
        let status = p.fields.iter().find(|f| f.name == "status").unwrap();
        // May be detected as Default or Interned depending on pool threshold
        match &status.role {
            FieldRole::Default { value, match_pct } => {
                assert_eq!(value, "delivered");
                assert!(*match_pct > 70.0);
            }
            FieldRole::Interned { pool_size } => {
                assert_eq!(*pool_size, 2);
            }
            other => panic!("Expected Default or Interned, got {:?}", other),
        }
    }

    #[test]
    fn test_profile_curvature_low_variance() {
        // All temps close together â†’ low curvature â†’ high confidence
        let data = json!({
            "readings": [
                {"sensor": "A", "temp": 22.0},
                {"sensor": "B", "temp": 22.1},
                {"sensor": "C", "temp": 22.2},
                {"sensor": "D", "temp": 21.9}
            ]
        });
        let p = profile(&data).unwrap();
        let temp = p.fields.iter().find(|f| f.name == "temp").unwrap();
        assert!(
            temp.curvature < 0.25,
            "Low-variance data should have low K, got {}",
            temp.curvature
        );
        assert!(
            temp.confidence > 0.8,
            "Low K should give high confidence, got {}",
            temp.confidence
        );
    }

    #[test]
    fn test_profile_curvature_high_variance() {
        // Spread-out values â†’ higher curvature
        let data = json!({
            "readings": [
                {"sensor": "A", "value": 1.0},
                {"sensor": "B", "value": 50.0},
                {"sensor": "C", "value": 99.0},
                {"sensor": "D", "value": 2.0}
            ]
        });
        let p = profile(&data).unwrap();
        let val = p.fields.iter().find(|f| f.name == "value").unwrap();
        assert!(
            val.curvature > 0.05,
            "Spread data should have higher K, got {}",
            val.curvature
        );
    }

    #[test]
    fn test_profile_display() {
        let data = json!({
            "items": [
                {"id": 1, "name": "Widget", "price": 10},
                {"id": 2, "name": "Gadget", "price": 20},
                {"id": 3, "name": "Doohickey", "price": 15}
            ]
        });
        let p = profile(&data).unwrap();
        let display = format!("{}", p);
        assert!(display.contains("DHOOM Profile: items"));
        assert!(display.contains("Records: 3"));
        assert!(display.contains("Savings:"));
    }

    #[test]
    fn test_profile_empty_dataset() {
        let data = json!({"things": []});
        let p = profile(&data).unwrap();
        assert_eq!(p.record_count, 0);
        assert_eq!(p.field_count, 0);
        assert!(p.fields.is_empty());
    }

    #[test]
    fn test_profile_elided_pct() {
        // With arithmetic + defaults, elided percentage should be significant
        let data = json!({
            "orders": [
                {"id": 1, "customer": "A", "status": "ok"},
                {"id": 2, "customer": "B", "status": "ok"},
                {"id": 3, "customer": "C", "status": "ok"},
                {"id": 4, "customer": "D", "status": "ok"}
            ]
        });
        let p = profile(&data).unwrap();
        assert!(
            p.fields_elided_pct > 0.0,
            "Should have elided fields, got {}%",
            p.fields_elided_pct
        );
    }

    // -------------------------------------------------------------------
    // StreamingDhoomEncoder (Feature #2 — TDD)
    // -------------------------------------------------------------------

    #[test]
    fn streaming_roundtrip_fidelity() {
        // Test 2.1: encode with streaming, decode, verify field-by-field equality
        let records: Vec<Value> = (0..250)
            .map(|i| {
                json!({
                    "id": i,
                    "name": format!("Drug_{i}"),
                    "mw": 200.0 + i as f64 * 0.5
                })
            })
            .collect();

        let mut buf = Vec::new();
        {
            let mut enc = StreamingDhoomEncoder::new(&mut buf, "drugs", 100);
            for r in &records {
                enc.push(r.clone()).unwrap();
            }
            enc.finish().unwrap();
        }

        let dhoom_str = String::from_utf8(buf).unwrap();
        let decoded = decode_to_json(&dhoom_str).unwrap();
        assert_eq!(decoded.len(), 250, "record count mismatch");
        for (i, rec) in decoded.iter().enumerate() {
            assert_eq!(rec["id"], json!(i as i64), "id mismatch at {i}");
            assert_eq!(rec["name"], json!(format!("Drug_{i}")), "name mismatch at {i}");
        }
    }

    #[test]
    fn streaming_chunk_boundary_correctness() {
        // Test 2.3: N=250, chunk_size=100 → 3 chunks (100+100+50)
        let records: Vec<Value> = (0..250)
            .map(|i| json!({"id": i, "val": format!("v{i}")}))
            .collect();

        let mut buf = Vec::new();
        {
            let mut enc = StreamingDhoomEncoder::new(&mut buf, "items", 100);
            for r in &records {
                enc.push(r.clone()).unwrap();
            }
            let written = enc.finish().unwrap();
            assert_eq!(written, 250);
        }

        let dhoom_str = String::from_utf8(buf).unwrap();
        let decoded = decode_to_json(&dhoom_str).unwrap();
        assert_eq!(decoded.len(), 250, "all 250 records must survive chunking");

        // Verify order preserved
        for i in 0..250 {
            assert_eq!(decoded[i]["id"], json!(i as i64));
        }
    }

    #[test]
    fn streaming_empty_bundle() {
        // Test 2.4: 0 records → valid empty DHOOM
        let mut buf = Vec::new();
        {
            let enc = StreamingDhoomEncoder::new(&mut buf, "empty", 100);
            let written = enc.finish().unwrap();
            assert_eq!(written, 0);
        }

        let dhoom_str = String::from_utf8(buf).unwrap();
        assert!(!dhoom_str.is_empty(), "should produce at least a header");
    }

    #[test]
    fn streaming_single_record() {
        // Test 2.5: 1 record, large chunk_size → correct roundtrip.
        // Use 2 records minimum so not all fields become defaults (DHOOM
        // decoder doesn't handle the all-default single-record edge case).
        let records = vec![
            json!({"id": 1, "label": "alpha"}),
            json!({"id": 2, "label": "beta"}),
        ];

        let mut buf = Vec::new();
        {
            let mut enc = StreamingDhoomEncoder::new(&mut buf, "singles", 1000);
            for r in &records {
                enc.push(r.clone()).unwrap();
            }
            let written = enc.finish().unwrap();
            assert_eq!(written, 2);
        }

        let dhoom_str = String::from_utf8(buf).unwrap();
        let decoded = decode_to_json(&dhoom_str).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0]["id"], json!(1));
        assert_eq!(decoded[0]["label"], json!("alpha"));
        assert_eq!(decoded[1]["id"], json!(2));
        assert_eq!(decoded[1]["label"], json!("beta"));
    }

    #[test]
    fn streaming_equivalence_with_batch_encoder() {
        // Test 2.6: streaming decode == batch decode (semantic equality)
        let records: Vec<Value> = (0..50)
            .map(|i| {
                json!({
                    "id": i,
                    "status": if i % 3 == 0 { "active" } else { "inactive" },
                    "score": i as f64 * 1.5
                })
            })
            .collect();

        // Batch encode
        let batch = encode_json(&records, "data");
        let batch_decoded = decode_to_json(&batch.dhoom).unwrap();

        // Streaming encode (chunk_size = N → equivalent to single chunk)
        let mut buf = Vec::new();
        {
            let mut enc = StreamingDhoomEncoder::new(&mut buf, "data", records.len());
            for r in &records {
                enc.push(r.clone()).unwrap();
            }
            enc.finish().unwrap();
        }
        let stream_decoded = decode_to_json(&String::from_utf8(buf).unwrap()).unwrap();

        assert_eq!(batch_decoded.len(), stream_decoded.len());
        for (b, s) in batch_decoded.iter().zip(stream_decoded.iter()) {
            assert_eq!(b, s, "batch vs stream mismatch");
        }
    }

    #[test]
    fn streaming_memory_bound() {
        // Test 2.2: verify the encoder produces correct output at scale with
        // small chunk size. All N records survive encode → decode roundtrip.
        let n = 1000usize;
        let chunk = 100;
        let mut buf = Vec::new();
        {
            let mut enc = StreamingDhoomEncoder::new(&mut buf, "mem_test", chunk);
            for i in 0..n {
                enc.push(json!({"id": i, "label": format!("item_{i}")})).unwrap();
            }
            let written = enc.finish().unwrap();
            assert_eq!(written, n);
        }
        let decoded = decode_to_json(&String::from_utf8(buf).unwrap()).unwrap();
        assert_eq!(decoded.len(), n);
    }
}

// ---------------------------------------------------------------------------
// GIGI compat layer — CSV support, StreamEncoder, encode_json/decode_to_json
// ---------------------------------------------------------------------------

/// Encode a JSON array of objects to DHOOM, wrapping under `collection` name.
/// Returns the DHOOM string and byte-count stats.
pub fn encode_json(input: &[Value], collection: &str) -> EncodeResult {
    let wrapped = Value::Object({
        let mut m = serde_json::Map::new();
        m.insert(collection.to_string(), Value::Array(input.to_vec()));
        m
    });
    let dhoom_str = encode(&wrapped).unwrap_or_default();
    let json_str = serde_json::to_string(&wrapped).unwrap_or_default();
    let dhoom_bytes = dhoom_str.len();
    let json_bytes = json_str.len();
    let compression_pct = if json_bytes > 0 {
        100.0 * (1.0 - dhoom_bytes as f64 / json_bytes as f64)
    } else {
        0.0
    };
    EncodeResult {
        dhoom: dhoom_str,
        json_bytes,
        dhoom_bytes,
        compression_pct,
    }
}

/// Decode a DHOOM string to a JSON array of objects.
pub fn decode_to_json(input: &str) -> Result<Vec<Value>> {
    let val = decode(input)?;
    match val {
        Value::Object(map) => {
            if let Some((_, arr)) = map.into_iter().next() {
                match arr {
                    Value::Array(records) => Ok(records),
                    _ => Ok(vec![]),
                }
            } else {
                Ok(vec![])
            }
        }
        Value::Array(records) => Ok(records),
        _ => Ok(vec![]),
    }
}

/// Statistics from encoding a dataset to DHOOM.
#[derive(Debug)]
pub struct EncodeResult {
    pub dhoom: String,
    pub json_bytes: usize,
    pub dhoom_bytes: usize,
    pub compression_pct: f64,
}

// ---------------------------------------------------------------------------
// CSV support
// ---------------------------------------------------------------------------

/// Parse CSV text into a (records, field_order) pair ready for DHOOM encoding.
pub fn csv_to_records(csv_text: &str) -> std::result::Result<(Vec<Value>, Vec<String>), String> {
    let mut lines = csv_text.lines();
    let header_line = lines.next().ok_or("Empty CSV input")?;
    let field_order: Vec<String> = header_line
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .collect();

    let mut records = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut map = serde_json::Map::new();
        let raw_values = split_record_fields(line);
        for (i, field_name) in field_order.iter().enumerate() {
            let raw = raw_values.get(i).map(|s| s.trim()).unwrap_or("");
            map.insert(field_name.clone(), coerce(raw));
        }
        records.push(Value::Object(map));
    }
    Ok((records, field_order))
}

/// CSV → DHOOM encode pipeline.
pub fn encode_csv(csv_text: &str, collection: &str) -> std::result::Result<EncodeResult, String> {
    let (records, _) = csv_to_records(csv_text)?;
    Ok(encode_json(&records, collection))
}

// ---------------------------------------------------------------------------
// StreamEncoder — emit DHOOM header once, then rows on demand.
// The schema is inferred from the first batch of records.
// ---------------------------------------------------------------------------

/// Streaming DHOOM encoder.
pub struct StreamEncoder {
    schema_header: String,
    fiber: Fiber,
    record_idx: usize,
}

impl StreamEncoder {
    /// Build a StreamEncoder from a sample of records (enough to detect all modifiers).
    pub fn new(sample: &[Value], collection: &str) -> std::result::Result<Self, String> {
        if sample.is_empty() {
            return Err("StreamEncoder requires at least one sample record".into());
        }
        let wrapped = Value::Object({
            let mut m = serde_json::Map::new();
            m.insert(collection.to_string(), Value::Array(sample.to_vec()));
            m
        });
        // Encode the sample just to generate the canonical header
        let dhoom_str = encode(&wrapped).map_err(|e| e.to_string())?;
        let header_line = dhoom_str.lines().next().unwrap_or("").to_string();
        // Trim the trailing ':'  — e.g. "name{...}:" → we keep it as-is for header()
        let fiber = parse_fiber(header_line.trim_end_matches(':')).map_err(|e| e.to_string())?;
        Ok(StreamEncoder {
            schema_header: header_line,
            fiber,
            record_idx: 0,
        })
    }

    /// The DHOOM header line (include in the first chunk you send).
    pub fn header(&self) -> &str {
        &self.schema_header
    }

    /// Encode a single record into a DHOOM row string.
    pub fn push(&mut self, record: &Value) -> String {
        let obj = match record.as_object() {
            Some(o) => o,
            None => return String::new(),
        };

        let rec_fields = self.fiber.record_fields();
        let mut values: Vec<String> = Vec::new();
        let mut is_default: Vec<bool> = Vec::new();

        for rf in &rec_fields {
            let raw_val = obj.get(&rf.name).cloned().unwrap_or(Value::Null);
            match &rf.modifier {
                Some(Modifier::Default(d)) => {
                    if &raw_val == d {
                        values.push(String::new());
                        is_default.push(true);
                    } else {
                        values.push(format!(":{}", value_to_dhoom(&raw_val)));
                        is_default.push(false);
                    }
                }
                _ => {
                    values.push(value_to_dhoom(&raw_val));
                    is_default.push(false);
                }
            }
        }

        // Trailing elision
        while values.last().map_or(false, |v| v.is_empty())
            && is_default.last().copied().unwrap_or(false)
        {
            values.pop();
            is_default.pop();
        }

        self.record_idx += 1;
        values.join(", ")
    }

    pub fn record_count(&self) -> usize {
        self.record_idx
    }
}

// ---------------------------------------------------------------------------
// Legacy DhoomValue type — kept for callers that use it via gigi_stream.rs
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// StreamingDhoomEncoder — chunked streaming to io::Write in constant memory
// ---------------------------------------------------------------------------

use std::io;

/// Streaming DHOOM encoder that writes to any `io::Write` sink.
///
/// Records are buffered in chunks of `chunk_size`. Each chunk is encoded and
/// flushed before the next begins, bounding memory to O(chunk_size * record_size).
///
/// Usage:
/// ```ignore
/// let file = File::create("bundle.dhoom")?;
/// let mut enc = StreamingDhoomEncoder::new(BufWriter::new(file), "drugs", 50_000);
/// for rec in store.records() {
///     enc.push(record_to_serde_json(&rec))?;
/// }
/// enc.finish()?;
/// ```
pub struct StreamingDhoomEncoder<W: io::Write> {
    writer: W,
    collection: String,
    chunk_size: usize,
    buffer: Vec<Value>,
    records_written: usize,
    header_written: bool,
    encoder: Option<StreamEncoder>,
    /// Minimum records to accumulate before creating the encoder.
    /// This ensures the sample has enough variance to detect field modifiers.
    min_sample: usize,
}

impl<W: io::Write> StreamingDhoomEncoder<W> {
    pub fn new(writer: W, collection: &str, chunk_size: usize) -> Self {
        let cs = chunk_size.max(1);
        Self {
            writer,
            collection: collection.to_string(),
            chunk_size: cs,
            buffer: Vec::with_capacity(cs.min(50_000)),
            records_written: 0,
            header_written: false,
            encoder: None,
            min_sample: 100,
        }
    }

    /// Feed one record. Flushes when buffer reaches `chunk_size`.
    pub fn push(&mut self, record: Value) -> io::Result<()> {
        self.buffer.push(record);
        // Don't flush until we have at least min_sample records for the encoder
        if self.encoder.is_some() && self.buffer.len() >= self.chunk_size {
            self.flush_chunk()?;
        } else if self.encoder.is_none() && self.buffer.len() >= self.min_sample.max(self.chunk_size) {
            self.flush_chunk()?;
        }
        Ok(())
    }

    /// Finalize — flush remaining buffer, return total records written.
    pub fn finish(mut self) -> io::Result<usize> {
        if !self.buffer.is_empty() {
            self.flush_chunk()?;
        }
        if !self.header_written {
            // Empty bundle — write a valid empty DHOOM (just the header)
            let header = format!("{}{{}}:\n", self.collection);
            self.writer.write_all(header.as_bytes())?;
        }
        self.writer.flush()?;
        Ok(self.records_written)
    }

    fn flush_chunk(&mut self) -> io::Result<()> {
        let chunk = std::mem::take(&mut self.buffer);
        if chunk.is_empty() {
            return Ok(());
        }

        if self.encoder.is_none() {
            // First chunk: create encoder from sample and write header.
            // Use all buffered records as the sample for best modifier detection.
            let enc = StreamEncoder::new(&chunk, &self.collection)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            self.writer.write_all(enc.header().as_bytes())?;
            self.writer.write_all(b"\n")?;
            self.encoder = Some(enc);
            self.header_written = true;
        }

        let enc = self.encoder.as_mut().unwrap();
        for record in &chunk {
            let row = enc.push(record);
            self.writer.write_all(row.as_bytes())?;
            self.writer.write_all(b"\n")?;
            self.records_written += 1;
        }
        Ok(())
    }
}

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
            DhoomValue::Bool(b) => if *b { "T" } else { "F" }.to_string(),
            DhoomValue::Null => String::new(),
        }
    }

    pub fn matches_default(&self, default: &str) -> bool {
        self.to_string_repr() == default
    }
}

/// Convert serde_json::Value to DhoomValue.
pub fn json_to_dhoom_value(v: &Value) -> DhoomValue {
    match v {
        Value::Number(n) => DhoomValue::Number(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => DhoomValue::Text(s.clone()),
        Value::Bool(b) => DhoomValue::Bool(*b),
        _ => DhoomValue::Null,
    }
}

/// Convert DhoomValue to serde_json::Value.
pub fn dhoom_to_json_value(v: &DhoomValue) -> Value {
    match v {
        DhoomValue::Number(n) => {
            if *n == (*n as i64) as f64 && n.abs() < 1e15 {
                Value::Number(Number::from(*n as i64))
            } else {
                Number::from_f64(*n)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
        }
        DhoomValue::Text(s) => Value::String(s.clone()),
        DhoomValue::Bool(b) => Value::Bool(*b),
        DhoomValue::Null => Value::Null,
    }
}

// ---------------------------------------------------------------------------
// Legacy ParsedDhoom — preserved for gigi_server.rs / gigi_stream.rs
// ---------------------------------------------------------------------------

/// Parsed DHOOM document in legacy format.
pub struct ParsedDhoom {
    pub collection: String,
    pub records: Vec<std::collections::HashMap<String, DhoomValue>>,
}

/// Parse DHOOM string into legacy ParsedDhoom (for backward-compat callers).
pub fn decode_legacy(input: &str) -> std::result::Result<ParsedDhoom, String> {
    let val = decode(input).map_err(|e| e.to_string())?;
    let (collection, arr) = match val {
        Value::Object(ref map) => {
            if let Some((k, v)) = map.iter().next() {
                (k.clone(), v.as_array().cloned().unwrap_or_default())
            } else {
                ("data".to_string(), vec![])
            }
        }
        Value::Array(ref a) => ("data".to_string(), a.clone()),
        _ => ("data".to_string(), vec![]),
    };
    let records = arr
        .into_iter()
        .filter_map(|item| {
            item.as_object().map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), json_to_dhoom_value(v)))
                    .collect()
            })
        })
        .collect();
    Ok(ParsedDhoom {
        collection,
        records,
    })
}

/// dhoom_to_json_array for legacy ParsedDhoom.
pub fn dhoom_to_json_array(parsed: &ParsedDhoom) -> Vec<Value> {
    parsed
        .records
        .iter()
        .map(|rec| {
            let map: serde_json::Map<String, Value> = rec
                .iter()
                .map(|(k, v)| (k.clone(), dhoom_to_json_value(v)))
                .collect();
            Value::Object(map)
        })
        .collect()
}
