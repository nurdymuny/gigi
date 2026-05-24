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
    /// Raw binary blob (voice notes, encrypted payloads). Serializes as base64.
    Binary(Vec<u8>),
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
                Value::Binary(_) => 7,
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
            Value::Binary(b) => b.hash(state),
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
            Value::Binary(b) => write!(f, "<binary {} bytes>", b.len()),
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

/// Per-field encryption mode declared at `CREATE BUNDLE` time.
///
/// v0.2 introduces four named modes alongside the legacy `Affine` (v0.1) path.
/// The mode is set on each fiber field — base fields are never gauge-encrypted
/// (the base hash already provides constant-time equality lookup).
///
/// Mutual exclusion rules:
/// - `Affine` and `Probabilistic` both modify numeric values; pick one.
/// - `Isometric` is only valid on grouped numeric fiber declarations.
/// - `Probabilistic` requires a numeric field type.
/// - `Indexed` is conventionally for high-cardinality columns only
///   (deterministic encryption leaks frequency on low-cardinality data;
///   the parser warns at schema time when this is detectable).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EncryptionMode {
    /// No encryption — field stored as plaintext.
    None,
    /// v0.1 affine numeric: ρ_g(v) = a·v + b.
    /// Default for NUMERIC / INTEGER / TIMESTAMP under bundle-level ENCRYPTED.
    Affine,
    /// AEAD-randomized (AES-GCM-SIV). Per-record nonce; IND-CPA; not equality-queryable.
    /// Default for TEXT / CATEGORICAL / BINARY / BOOL under bundle-level ENCRYPTED.
    Opaque,
    /// PRF-deterministic (AES-256-CMAC or keyed SipHash). Equality-queryable.
    /// Frequency-leakage caveat: high-cardinality columns only.
    Indexed,
    /// Affine + Gaussian noise with schema-declared σ. Statistical unlinkability +
    /// queryable equality via Davis Identity. Numeric fields only.
    Probabilistic { sigma: f64 },
    /// Orthogonal O(k) gauge on grouped numeric fiber. Pairwise distance preserving.
    /// Only valid when the field is part of a GROUP declaration.
    Isometric,
}

impl EncryptionMode {
    /// True if this mode actually transforms the stored value.
    pub fn is_encrypted(&self) -> bool {
        !matches!(self, EncryptionMode::None)
    }

    /// Default mode when bundle-level `ENCRYPTED` is declared without a per-field
    /// override, given the field's type. Numeric fields get Affine (the v0.1 path);
    /// text / binary / bool get Opaque (the safe choice). Categorical also
    /// defaults to Opaque even though Indexed is sometimes desirable — the
    /// schema author must opt in to Indexed because of the frequency-leakage
    /// caveat.
    pub fn default_for_type(field_type: &FieldType) -> Self {
        match field_type {
            FieldType::Numeric | FieldType::Timestamp | FieldType::Vector { .. } => EncryptionMode::Affine,
            FieldType::Categorical | FieldType::OrderedCat { .. } => EncryptionMode::Opaque,
            FieldType::Binary => EncryptionMode::Opaque,
        }
    }
}

/// Where the master seed used to derive a bundle's GaugeKey came from.
/// v0.2 adds three sources beyond the v0.1 random default:
///
/// - `Random`  — server-generated 32-byte seed via OS CSPRNG (the v0.1 path).
///               Used when no `WITH ENCRYPTION SEED` clause appears.
/// - `Hex(s)`  — caller supplied 64 hex chars verbatim. Deterministic across
///               deployments using the same seed; useful for reproducible
///               builds and shared-key scenarios.
/// - `Env(n)`  — schema stores the env-var name; engine resolves the actual
///               seed at startup. Lets ops keep keys out of schema dumps and
///               rotate via env-var change + restart.
#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionSeedSource {
    Random,
    Hex(String),
    Env(String),
}

impl Default for EncryptionSeedSource {
    fn default() -> Self {
        EncryptionSeedSource::Random
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
    /// Per-field encryption mode (v0.2). Default `None` (plaintext); set to a
    /// specific mode by the parser when the schema declares one. The
    /// bundle-level `ENCRYPTED` keyword (v0.1) propagates through to per-field
    /// `default_for_type` defaults at parse time.
    pub encryption: EncryptionMode,
    /// Optional group identifier for ISOMETRIC mode. Multiple fiber fields
    /// declared with the same `encryption_group` share a single O(k)
    /// orthogonal matrix at encrypt time, where k is the group size. For
    /// non-Isometric modes this field is ignored. Defaults to `None`; a
    /// solo Isometric field with no group declaration ends up in a
    /// singleton group (k=1) which is the trivial sign-flip case.
    pub encryption_group: Option<String>,
}

impl FieldDef {
    pub fn numeric(name: &str) -> Self {
        Self {
            name: name.to_string(),
            field_type: FieldType::Numeric,
            default: Value::Null,
            range: None,
            weight: 1.0,
            encryption: EncryptionMode::None,
            encryption_group: None,
        }
    }

    pub fn categorical(name: &str) -> Self {
        Self {
            name: name.to_string(),
            field_type: FieldType::Categorical,
            default: Value::Null,
            range: None,
            weight: 1.0,
            encryption: EncryptionMode::None,
            encryption_group: None,
        }
    }

    pub fn timestamp(name: &str, time_scale: f64) -> Self {
        Self {
            name: name.to_string(),
            field_type: FieldType::Timestamp,
            default: Value::Null,
            range: Some(time_scale),
            weight: 1.0,
            encryption: EncryptionMode::None,
            encryption_group: None,
        }
    }

    pub fn binary(name: &str) -> Self {
        Self {
            name: name.to_string(),
            field_type: FieldType::Binary,
            default: Value::Null,
            range: None,
            weight: 1.0,
            encryption: EncryptionMode::None,
            encryption_group: None,
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

    /// Set the encryption mode (v0.2). Defaults to `EncryptionMode::None`
    /// (plaintext) until the parser sets a specific mode at schema time.
    pub fn with_encryption(mut self, mode: EncryptionMode) -> Self {
        self.encryption = mode;
        self
    }

    /// v0.2 (Sprint E): assign this field to a named ISOMETRIC group. All fields
    /// sharing a group_id are encrypted jointly with one shared O(k) matrix.
    /// Group has no effect on non-Isometric modes.
    pub fn with_encryption_group(mut self, group: impl Into<String>) -> Self {
        self.encryption_group = Some(group.into());
        self
    }
}

/// A schema-level invariant constraint: field must equal value ± tol on every insert.
/// Used for INVARIANT clauses in CREATE BUNDLE (Ask 1 / unit-norm enforcement for Ask 2).
#[derive(Debug, Clone)]
pub struct InvariantDef {
    /// Fiber field the constraint applies to (e.g. "norm_sq")
    pub expr_field: String,
    /// Expected value (e.g. 1.0)
    pub expected: f64,
    /// Absolute tolerance (e.g. 1e-9)
    pub tol: f64,
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
    /// Schema-declared invariant constraints checked on every insert.
    pub invariants: Vec<InvariantDef>,
    /// Optional Kähler structure (complex structure J + closed 2-form B)
    /// attached to the fiber tangent space. When `Some`, downstream
    /// layers (dual adjacency, Jacobi cost, Hadamard detection,
    /// prequantization) automatically apply their Kähler-aware code
    /// paths. When `None`, the bundle is purely Riemannian and behaves
    /// identically to a pre-upgrade GIGI. Gated by the `kahler`
    /// feature flag so the engine surface stays bit-identical when
    /// the feature is off — see catalog.md §1 + IMPLEMENTATION_PLAN.md
    /// L1.4.
    #[cfg(feature = "kahler")]
    pub kahler: Option<crate::geometry::KahlerStructure>,
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
            invariants: Vec::new(),
            // Default: no Kähler structure attached — bundle is
            // purely Riemannian and behaves exactly as pre-upgrade.
            // Attach via `with_kahler` when the feature is on.
            #[cfg(feature = "kahler")]
            kahler: None,
        }
    }

    /// Attach a Kähler structure (J, B) to this schema. Sets the
    /// `kahler` field; idempotent — calling twice replaces. The
    /// dim-coherence invariant is checked at attach time so a
    /// schema with mismatched J/B dimensions can never be stored.
    #[cfg(feature = "kahler")]
    pub fn with_kahler(mut self, k: crate::geometry::KahlerStructure) -> Self {
        assert!(
            k.dim_coherent(),
            "KahlerStructure dim mismatch: J dim={}, B dim={}",
            k.j.dim(),
            k.b.dim()
        );
        self.kahler = Some(k);
        self
    }

    pub fn with_invariant(mut self, inv: InvariantDef) -> Self {
        self.invariants.push(inv);
        self
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
    /// ON field_a TO field_b VIA fn — non-identity restriction map.
    Transform {
        source_field: String,
        target_field: String,
        transform: TransformFn,
    },
    /// ON MORPH source_bundle.key — cross-bundle join via shared key field.
    Morphism {
        source_bundle: String,
        join_field: String,
        /// Quality discount ∈ (0,1] — how well-aligned the external measurement is.
        quality: f64,
    },
}

/// Built-in transform functions for non-identity restriction maps.
#[derive(Debug, Clone, PartialEq)]
pub enum TransformFn {
    /// f(x) = log10(x)
    Log10,
    /// f(x) = a*x + b
    Scale { a: f64, b: f64 },
    /// f(x) = x * β, β ∈ [lo, hi] — returns the midpoint, uncertainty = (hi-lo)/2
    Biofilm { lo: f64, hi: f64 },
}

impl TransformFn {
    /// Apply the forward transform.
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            TransformFn::Log10 => x.log10(),
            TransformFn::Scale { a, b } => a * x + b,
            TransformFn::Biofilm { lo, hi } => x * (lo + hi) / 2.0,
        }
    }

    /// Apply the inverse transform (for reverse mapping).
    pub fn inverse(&self, y: f64) -> f64 {
        match self {
            TransformFn::Log10 => 10.0_f64.powf(y),
            TransformFn::Scale { a, b } => (y - b) / a,
            TransformFn::Biofilm { lo, hi } => y * 2.0 / (lo + hi),
        }
    }
}

/// A named, weighted adjacency function declared in a bundle schema.
#[derive(Debug, Clone, PartialEq)]
pub struct AdjacencyDef {
    pub name: String,
    pub kind: AdjacencyKind,
    pub weight: f64,
}
