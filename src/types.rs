//! Core types — Definitions 1.1–1.4, 1.7 from the spec.

use std::collections::HashMap;
use std::fmt;

/// Field type enumeration (Def 1.7 — fiber metric type table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Numeric,
    Categorical,
    OrderedCat {
        order: Vec<String>,
    },
    Timestamp,
    Binary,
    /// Dense float vector of fixed dimensionality (embedding field).
    /// Geometric meaning: a section into a vector bundle V = B × ℝᵈ.
    Vector {
        dims: usize,
    },
}

/// A dynamically-typed value stored in a fiber.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Timestamp(i64),
    /// Dense float vector — embedding / feature vector.
    Vector(Vec<f64>),
    Null,
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        fn type_order(v: &Value) -> u8 {
            match v {
                Value::Null => 0,
                Value::Bool(_) => 1,
                Value::Integer(_) => 2,
                Value::Float(_) => 3,
                Value::Text(_) => 4,
                Value::Timestamp(_) => 5,
                Value::Vector(_) => 6,
            }
        }
        match (self, other) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Less,
            (_, Value::Null) => Ordering::Greater,
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => a.total_cmp(b),
            (Value::Integer(a), Value::Float(b)) => (*a as f64).total_cmp(b),
            (Value::Float(a), Value::Integer(b)) => a.total_cmp(&(*b as f64)),
            (Value::Text(a), Value::Text(b)) => a.cmp(b),
            (Value::Timestamp(a), Value::Timestamp(b)) => a.cmp(b),
            (Value::Vector(a), Value::Vector(b)) => {
                // Lexicographic on bit patterns (for Ord consistency; semantic
                // similarity is handled by vector_search, not ordering).
                for (x, y) in a.iter().zip(b.iter()) {
                    let cmp = x.total_cmp(y);
                    if cmp != Ordering::Equal {
                        return cmp;
                    }
                }
                a.len().cmp(&b.len())
            }
            _ => type_order(self).cmp(&type_order(other)),
        }
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Integer(v) => v.hash(state),
            Value::Float(v) => v.to_bits().hash(state),
            Value::Text(v) => v.hash(state),
            Value::Bool(v) => v.hash(state),
            Value::Timestamp(v) => v.hash(state),
            Value::Vector(v) => {
                v.len().hash(state);
                for x in v {
                    x.to_bits().hash(state);
                }
            }
            Value::Null => {}
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Text(v) => write!(f, "{v}"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::Timestamp(v) => write!(f, "T{v}"),
            Value::Vector(v) => {
                write!(f, "[")?;
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{x}")?;
                }
                write!(f, "]")
            }
            Value::Null => write!(f, "NULL"),
        }
    }
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Integer(v) => Some(*v as f64),
            Value::Float(v) => Some(*v),
            Value::Timestamp(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(v) => Some(*v),
            Value::Timestamp(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_timestamp(&self) -> Option<i64> {
        match self {
            Value::Timestamp(v) => Some(*v),
            Value::Integer(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(v) => Some(v.as_str()),
            _ => None,
        }
    }
}

/// Field definition in the schema.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub default: Value,
    /// For numeric/timestamp: the range of the field (used in metric normalization).
    pub range: Option<f64>,
    /// Weight in the product metric (default 1.0).
    pub weight: f64,
}

impl FieldDef {
    pub fn numeric(name: &str) -> Self {
        Self {
            name: name.to_string(),
            field_type: FieldType::Numeric,
            default: Value::Null,
            range: None,
            weight: 1.0,
        }
    }

    pub fn categorical(name: &str) -> Self {
        Self {
            name: name.to_string(),
            field_type: FieldType::Categorical,
            default: Value::Null,
            range: None,
            weight: 1.0,
        }
    }

    pub fn timestamp(name: &str, time_scale: f64) -> Self {
        Self {
            name: name.to_string(),
            field_type: FieldType::Timestamp,
            default: Value::Null,
            range: Some(time_scale),
            weight: 1.0,
        }
    }

    pub fn with_range(mut self, range: f64) -> Self {
        self.range = Some(range);
        self
    }

    pub fn with_default(mut self, default: Value) -> Self {
        self.default = default;
        self
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Bundle schema (Def 1.1) — declares base fields and fiber fields.
#[derive(Debug, Clone)]
pub struct BundleSchema {
    pub name: String,
    /// Base fields parameterize B (the key).
    pub base_fields: Vec<FieldDef>,
    /// Fiber fields are the non-key data.
    pub fiber_fields: Vec<FieldDef>,
    /// Which fiber fields are indexed for range queries.
    pub indexed_fields: Vec<String>,
    /// Optional geometric encryption key (gauge transform on fibers).
    pub gauge_key: Option<crate::crypto::GaugeKey>,
    /// Schema-declared adjacency functions for COMPLETE.
    pub adjacencies: Vec<AdjacencyDef>,
    /// H¹ z-score threshold for consistency checks (default 3.0).
    pub h1_threshold: f64,
}

impl BundleSchema {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            base_fields: Vec::new(),
            fiber_fields: Vec::new(),
            indexed_fields: Vec::new(),
            gauge_key: None,
            adjacencies: Vec::new(),
            h1_threshold: 3.0,
        }
    }

    pub fn base(mut self, field: FieldDef) -> Self {
        self.base_fields.push(field);
        self
    }

    pub fn fiber(mut self, field: FieldDef) -> Self {
        self.fiber_fields.push(field);
        self
    }

    pub fn index(mut self, field_name: &str) -> Self {
        self.indexed_fields.push(field_name.to_string());
        self
    }

    pub fn adjacency(mut self, adj: AdjacencyDef) -> Self {
        self.adjacencies.push(adj);
        self
    }

    pub fn with_h1_threshold(mut self, threshold: f64) -> Self {
        self.h1_threshold = threshold;
        self
    }

    /// Get the zero section (Def 1.3) — all defaults.
    pub fn zero_section(&self) -> Vec<Value> {
        self.fiber_fields
            .iter()
            .map(|f| f.default.clone())
            .collect()
    }

    pub fn fiber_field_index(&self, name: &str) -> Option<usize> {
        self.fiber_fields.iter().position(|f| f.name == name)
    }

    pub fn base_field_index(&self, name: &str) -> Option<usize> {
        self.base_fields.iter().position(|f| f.name == name)
    }

    /// All field names (base + fiber) in order.
    pub fn all_field_names(&self) -> Vec<&str> {
        self.base_fields
            .iter()
            .chain(self.fiber_fields.iter())
            .map(|f| f.name.as_str())
            .collect()
    }
}

/// A record: map from field name to value.
pub type Record = HashMap<String, Value>;

/// Base point in the discrete base space B.
pub type BasePoint = u64;

/// Schema-declared adjacency kind for COMPLETE.
#[derive(Debug, Clone, PartialEq)]
pub enum AdjacencyKind {
    /// ON field = field — neighbor if same value in the named field.
    Equality { field: String },
    /// ON field WITHIN radius — neighbor if |field_a - field_b| < radius.
    Metric { field: String, radius: f64 },
    /// ON field ABOVE threshold — neighbor if |field_value| > threshold.
    Threshold { field: String, threshold: f64 },
}

/// A named, weighted adjacency function declared in a bundle schema.
#[derive(Debug, Clone, PartialEq)]
pub struct AdjacencyDef {
    pub name: String,
    pub kind: AdjacencyKind,
    pub weight: f64,
}
