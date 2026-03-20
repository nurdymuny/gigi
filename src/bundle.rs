//! BundleStore — the core storage engine (§7).
//!
//! Implements Definitions 1.1–1.4, Theorems 1.2–1.3.
//! Includes flat-base detection: when K=0 (arithmetic keys),
//! storage degenerates to Vec (memcpy insert, pointer-arithmetic lookup).

use std::collections::HashMap;

use regex::Regex;
use roaring::RoaringBitmap;

use crate::hash::HashConfig;
use crate::types::{BasePoint, BundleSchema, FieldDef, FieldType, Record, Value};

// ── Query Conditions ─────────────────────────────────────────

/// A query condition for filtered_query.
#[derive(Debug, Clone)]
pub enum QueryCondition {
    Eq(String, Value),
    Neq(String, Value),
    Gt(String, Value),
    Gte(String, Value),
    Lt(String, Value),
    Lte(String, Value),
    Contains(String, String),
    StartsWith(String, String),
    /// Match any value in a set: `field IN [v1, v2, v3]`
    In(String, Vec<Value>),
    /// Exclude values in a set: `field NOT IN [v1, v2]`
    NotIn(String, Vec<Value>),
    /// Field is null or missing
    IsNull(String),
    /// Field is present and not null
    IsNotNull(String),
    /// Suffix match: `field ENDS_WITH ".gov"`
    EndsWith(String, String),
    /// Regex match: `field MATCHES "^CAS-\\d+"`
    Regex(String, String),
    /// Inclusive range: `low <= field <= high`
    Between(String, Value, Value),
}

impl QueryCondition {
    /// Check whether a record matches this condition.
    pub fn matches(&self, record: &Record) -> bool {
        match self {
            QueryCondition::Eq(field, value) => {
                record.get(field).map_or(false, |v| v == value)
            }
            QueryCondition::Neq(field, value) => {
                record.get(field).map_or(true, |v| v != value)
            }
            QueryCondition::Gt(field, value) => {
                record.get(field).map_or(false, |v| v > value)
            }
            QueryCondition::Gte(field, value) => {
                record.get(field).map_or(false, |v| v >= value)
            }
            QueryCondition::Lt(field, value) => {
                record.get(field).map_or(false, |v| v < value)
            }
            QueryCondition::Lte(field, value) => {
                record.get(field).map_or(false, |v| v <= value)
            }
            QueryCondition::Contains(field, substr) => {
                record.get(field).map_or(false, |v| {
                    if let Value::Text(s) = v {
                        s.to_lowercase().contains(&substr.to_lowercase())
                    } else {
                        false
                    }
                })
            }
            QueryCondition::StartsWith(field, prefix) => {
                record.get(field).map_or(false, |v| {
                    if let Value::Text(s) = v {
                        s.to_lowercase().starts_with(&prefix.to_lowercase())
                    } else {
                        false
                    }
                })
            }
            QueryCondition::In(field, values) => {
                record.get(field).map_or(false, |v| values.contains(v))
            }
            QueryCondition::NotIn(field, values) => {
                record.get(field).map_or(true, |v| !values.contains(v))
            }
            QueryCondition::IsNull(field) => {
                match record.get(field) {
                    None | Some(Value::Null) => true,
                    _ => false,
                }
            }
            QueryCondition::IsNotNull(field) => {
                match record.get(field) {
                    None | Some(Value::Null) => false,
                    _ => true,
                }
            }
            QueryCondition::EndsWith(field, suffix) => {
                record.get(field).map_or(false, |v| {
                    if let Value::Text(s) = v {
                        s.to_lowercase().ends_with(&suffix.to_lowercase())
                    } else {
                        false
                    }
                })
            }
            QueryCondition::Regex(field, pattern) => {
                record.get(field).map_or(false, |v| {
                    if let Value::Text(s) = v {
                        // Use thread-local cache to avoid recompiling regex per record
                        thread_local! {
                            static REGEX_CACHE: std::cell::RefCell<HashMap<String, Option<Regex>>> =
                                std::cell::RefCell::new(HashMap::new());
                        }
                        REGEX_CACHE.with(|cache| {
                            let mut cache = cache.borrow_mut();
                            let compiled = cache.entry(pattern.clone())
                                .or_insert_with(|| Regex::new(pattern).ok());
                            compiled.as_ref().map_or(false, |re| re.is_match(s))
                        })
                    } else {
                        false
                    }
                })
            }
            QueryCondition::Between(field, low, high) => {
                record.get(field).map_or(false, |v| v >= low && v <= high)
            }
        }
    }
}

/// Helper: check if record matches AND conditions plus optional OR groups.
fn matches_filter(record: &Record, conditions: &[QueryCondition], or_conditions: Option<&[Vec<QueryCondition>]>) -> bool {
    if !conditions.iter().all(|c| c.matches(record)) {
        return false;
    }
    matches_or_filter(record, or_conditions)
}

/// Helper: check if record matches optional OR groups.
fn matches_or_filter(record: &Record, or_conditions: Option<&[Vec<QueryCondition>]>) -> bool {
    match or_conditions {
        Some(groups) if !groups.is_empty() => {
            groups.iter().any(|group| group.iter().all(|c| c.matches(record)))
        }
        _ => true,
    }
}

// ── Vector Similarity Metric ──────────────────────────────────

/// Metric for kNN vector search.
///
/// All metrics are unified to "higher score = better match" so callers
/// always sort descending and slice top_k.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorMetric {
    /// Cosine similarity ∈ [-1, 1]. Best for normalized embeddings.
    /// Geometric meaning: angle between fiber vectors (inner product on S^{d-1}).
    Cosine,
    /// Negative L2 (Euclidean) distance. Best for un-normalized embeddings.
    /// Stored as -||a-b||₂ so higher = closer.
    Euclidean,
    /// Raw dot product. Best when vector magnitude encodes relevance.
    Dot,
}

impl VectorMetric {
    /// Compute the score between query vector `q` and candidate vector `v`.
    /// Higher score always means better match.
    pub fn score(self, q: &[f64], v: &[f64]) -> f64 {
        debug_assert_eq!(q.len(), v.len());
        match self {
            VectorMetric::Cosine => {
                let dot: f64 = q.iter().zip(v).map(|(a, b)| a * b).sum();
                let norm_q: f64 = q.iter().map(|x| x * x).sum::<f64>().sqrt();
                let norm_v: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
                if norm_q < 1e-12 || norm_v < 1e-12 { 0.0 } else { dot / (norm_q * norm_v) }
            }
            VectorMetric::Euclidean => {
                let dist: f64 = q.iter().zip(v).map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
                -dist  // negate so higher = closer
            }
            VectorMetric::Dot => {
                q.iter().zip(v).map(|(a, b)| a * b).sum()
            }
        }
    }
}

// ── BaseStorage: coordinate-chart-aware storage ──────────────

/// Storage strategy determined by base-space curvature.
///
/// K > 0  → Hashed (HashMap — general case)
/// K = 0  → Sequential (Vec — flat base, memcpy insert, array[offset] lookup)
/// K ≈ 0  → Hybrid (Vec + overflow HashMap)
#[derive(Debug)]
enum BaseStorage {
    /// Curved base space. General O(1) hash-based storage.
    Hashed {
        sections: HashMap<BasePoint, Vec<Value>>,
        base_values: HashMap<BasePoint, Vec<Value>>,
    },
    /// Flat base space (K=0). Array storage — insert = push, lookup = offset.
    Sequential {
        sections: Vec<Vec<Value>>,
        base_values: Vec<Vec<Value>>,
        start: i64,
        step: i64,
        key_field: String,
    },
    /// Nearly flat (K≈0). Array for sequential runs, hash for outliers.
    Hybrid {
        sections: Vec<Vec<Value>>,
        base_values: Vec<Vec<Value>>,
        overflow_sections: HashMap<BasePoint, Vec<Value>>,
        overflow_base: HashMap<BasePoint, Vec<Value>>,
        start: i64,
        step: i64,
        key_field: String,
    },
}

impl BaseStorage {
    fn new_hashed() -> Self {
        BaseStorage::Hashed {
            sections: HashMap::new(),
            base_values: HashMap::new(),
        }
    }

    fn insert_hashed(&mut self, bp: BasePoint, fiber: Vec<Value>, base: Vec<Value>) {
        match self {
            BaseStorage::Hashed { sections, base_values } => {
                sections.insert(bp, fiber);
                base_values.insert(bp, base);
            }
            BaseStorage::Sequential { sections, base_values, start, step, .. } => {
                // Sequential: push to end. Caller must ensure key matches expected.
                let expected = *start + (*step * sections.len() as i64);
                // If this is the first record, we might need to set start
                if sections.is_empty() {
                    // start already set by detect_geometry
                }
                let _ = expected; // validated by caller
                sections.push(fiber);
                base_values.push(base);
            }
            BaseStorage::Hybrid {
                sections, base_values,
                overflow_sections, overflow_base,
                start, step, ..
            } => {
                // Try sequential slot first; caller provides key_value via separate method
                // Default: push to overflow (the typed insert_hybrid handles key_value)
                let _ = (start, step, sections, base_values);
                overflow_sections.insert(bp, fiber);
                overflow_base.insert(bp, base);
            }
        }
    }

    /// Insert into sequential or hybrid storage with the raw key value.
    fn insert_with_key(&mut self, bp: BasePoint, key_value: i64, fiber: Vec<Value>, base: Vec<Value>) {
        match self {
            BaseStorage::Hashed { sections, base_values } => {
                sections.insert(bp, fiber);
                base_values.insert(bp, base);
            }
            BaseStorage::Sequential { sections, base_values, start, step, .. } => {
                let expected = *start + (*step * sections.len() as i64);
                if key_value == expected {
                    sections.push(fiber);
                    base_values.push(base);
                } else {
                    // Out of order — shouldn't happen in Sequential mode.
                    // Promote will handle this upstream.
                    sections.push(fiber);
                    base_values.push(base);
                }
            }
            BaseStorage::Hybrid {
                sections, base_values,
                overflow_sections, overflow_base,
                start, step, ..
            } => {
                let expected = *start + (*step * sections.len() as i64);
                if key_value == expected {
                    sections.push(fiber);   // hot path: memcpy
                    base_values.push(base);
                } else {
                    overflow_sections.insert(bp, fiber);
                    overflow_base.insert(bp, base);
                }
            }
        }
    }

    #[allow(dead_code)]
    fn get_fiber(&self, bp: BasePoint) -> Option<&[Value]> {
        match self {
            BaseStorage::Hashed { sections, .. } => sections.get(&bp).map(|v| v.as_slice()),
            BaseStorage::Sequential { sections, start, step, .. } => {
                // Reverse: bp was computed from the key, but we need the index.
                // For sequential mode, we use the bp_to_idx map maintained externally.
                // Fallback: linear scan (shouldn't be hit — caller uses get_fiber_by_idx).
                let _ = (start, step);
                // Sequential mode doesn't map by bp — see get_fiber_by_key
                // This path is hit by external code using raw bp; do linear search
                None.or_else(|| {
                    // Not reachable in normal operation for Sequential
                    let _ = sections;
                    None
                })
            }
            BaseStorage::Hybrid {
                sections, overflow_sections, start, step, ..
            } => {
                let _ = (start, step, sections);
                overflow_sections.get(&bp).map(|v| v.as_slice())
            }
        }
    }

    #[allow(dead_code)]
    fn get_section_and_base(&self, bp: BasePoint) -> Option<(&[Value], &[Value])> {
        match self {
            BaseStorage::Hashed { sections, base_values } => {
                let s = sections.get(&bp)?;
                let b = base_values.get(&bp)?;
                Some((s.as_slice(), b.as_slice()))
            }
            BaseStorage::Sequential { .. } | BaseStorage::Hybrid { .. } => {
                // For sequential/hybrid, use get_by_key_value instead
                None
            }
        }
    }

    fn get_by_index(&self, idx: usize) -> Option<(&[Value], &[Value])> {
        match self {
            BaseStorage::Hashed { .. } => None,
            BaseStorage::Sequential { sections, base_values, .. } => {
                let s = sections.get(idx)?;
                let b = base_values.get(idx)?;
                Some((s.as_slice(), b.as_slice()))
            }
            BaseStorage::Hybrid { sections, base_values, .. } => {
                let s = sections.get(idx)?;
                let b = base_values.get(idx)?;
                Some((s.as_slice(), b.as_slice()))
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            BaseStorage::Hashed { sections, .. } => sections.len(),
            BaseStorage::Sequential { sections, .. } => sections.len(),
            BaseStorage::Hybrid { sections, overflow_sections, .. } => {
                sections.len() + overflow_sections.len()
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[allow(dead_code)]
    fn is_sequential(&self) -> bool {
        matches!(self, BaseStorage::Sequential { .. })
    }

    fn is_hybrid(&self) -> bool {
        matches!(self, BaseStorage::Hybrid { .. })
    }

    fn overflow_ratio(&self) -> f64 {
        match self {
            BaseStorage::Hybrid { sections, overflow_sections, .. } => {
                if sections.is_empty() { return 0.0; }
                overflow_sections.len() as f64 / sections.len() as f64
            }
            _ => 0.0,
        }
    }

    /// Promote to Hashed storage. Returns the new Hashed variant populated
    /// with all records. Caller must also rebuild bp mappings.
    fn promote_to_hashed(self, bp_list: &[(BasePoint, usize)], _overflow_bps: &HashMap<BasePoint, ()>) -> BaseStorage {
        match self {
            BaseStorage::Sequential { sections, base_values, .. } => {
                let mut h_sec = HashMap::with_capacity(sections.len());
                let mut h_base = HashMap::with_capacity(base_values.len());
                for &(bp, idx) in bp_list {
                    if idx < sections.len() {
                        h_sec.insert(bp, sections[idx].clone());
                        h_base.insert(bp, base_values[idx].clone());
                    }
                }
                BaseStorage::Hashed { sections: h_sec, base_values: h_base }
            }
            BaseStorage::Hybrid {
                sections, base_values,
                overflow_sections, overflow_base, ..
            } => {
                let mut h_sec = HashMap::with_capacity(sections.len() + overflow_sections.len());
                let mut h_base = HashMap::with_capacity(base_values.len() + overflow_base.len());
                for &(bp, idx) in bp_list {
                    if idx < sections.len() {
                        h_sec.insert(bp, sections[idx].clone());
                        h_base.insert(bp, base_values[idx].clone());
                    }
                }
                for (bp, fiber) in overflow_sections {
                    h_sec.insert(bp, fiber);
                }
                for (bp, base) in overflow_base {
                    h_base.insert(bp, base);
                }
                BaseStorage::Hashed { sections: h_sec, base_values: h_base }
            }
            h @ BaseStorage::Hashed { .. } => h,
        }
    }
}

// ── Base geometry detection ──────────────────────────────────

/// Result of analyzing the first N records for base-space flatness.
#[derive(Debug, Clone)]
pub enum BaseGeometry {
    /// K > 0: curved base, use HashMap.
    Curved,
    /// K = 0: flat base (arithmetic keys), use Vec.
    Flat { start: i64, step: i64, key_field: String },
    /// K ≈ 0: mostly flat, use Vec + overflow HashMap.
    NearlyFlat { start: i64, step: i64, key_field: String },
}

/// Detect the base geometry from initial records.
pub fn detect_base_geometry(schema: &BundleSchema, records: &[Record]) -> BaseGeometry {
    // Need at least 2 records to detect arithmetic
    if records.len() < 2 || schema.base_fields.is_empty() {
        return BaseGeometry::Curved;
    }

    // Only works for single-field base (composite keys → Hashed)
    if schema.base_fields.len() != 1 {
        return BaseGeometry::Curved;
    }

    let key_field = &schema.base_fields[0].name;

    // Extract integer key values
    let keys: Vec<i64> = records.iter()
        .filter_map(|r| r.get(key_field)?.as_i64())
        .collect();

    if keys.len() < 2 {
        return BaseGeometry::Curved;
    }

    let start = keys[0];
    let step = keys[1] - keys[0];

    if step == 0 {
        return BaseGeometry::Curved;
    }

    let arithmetic_count = keys.windows(2)
        .filter(|w| w[1] - w[0] == step)
        .count();
    let total = keys.len() - 1;

    if arithmetic_count == total {
        BaseGeometry::Flat { start, step, key_field: key_field.clone() }
    } else if (arithmetic_count as f64 / total as f64) > 0.95 {
        BaseGeometry::NearlyFlat { start, step, key_field: key_field.clone() }
    } else {
        BaseGeometry::Curved
    }
}

/// The Bundle Store (§7.1).
///
/// Storage strategy adapts to base-space curvature:
/// K=0 (flat) → Sequential Vec (memcpy insert, array[offset] lookup)
/// K>0 (curved) → Hashed HashMap (general O(1))
/// K≈0 (nearly flat) → Hybrid Vec + overflow HashMap
#[derive(Debug)]
pub struct BundleStore {
    pub schema: BundleSchema,
    hash_config: HashConfig,
    /// Geometry-aware storage for sections + base values.
    storage: BaseStorage,
    /// Field topology: open set membership for sheaf queries (Def 2.1).
    /// Maps field_name → field_value → set of base points.
    field_index: HashMap<String, HashMap<Value, RoaringBitmap>>,
    /// Running statistics for curvature computation.
    field_stats: HashMap<String, FieldStats>,
    /// Reverse map from truncated u32 bitmap key → full u64 base point.
    bp_reverse: HashMap<u32, BasePoint>,
    /// For Sequential/Hybrid: map from sequential index → BasePoint.
    seq_bp_list: Vec<BasePoint>,
    /// For Sequential/Hybrid: map from BasePoint → sequential index.
    bp_to_idx: HashMap<BasePoint, usize>,
    /// Track key insertion order for auto-detection (first 32 unique keys).
    detect_keys: Vec<i64>,
    /// Whether detection has already fired.
    detected: bool,
    /// Auto-increment counter for auto-generated IDs.
    auto_id_counter: u64,
    /// Incremental curvature moments: K(p) for every inserted record.
    /// Updated in O(1) per insert — anomaly scores come for free.
    pub curvature_stats: CurvatureStats,
}

/// Per-field running statistics for curvature.
#[derive(Debug, Clone, Default)]
pub struct FieldStats {
    pub count: u64,
    pub sum: f64,
    pub sum_sq: f64,
    pub min: f64,
    pub max: f64,
}

impl FieldStats {
    pub fn update(&mut self, val: f64) {
        self.count += 1;
        self.sum += val;
        self.sum_sq += val * val;
        if self.count == 1 {
            self.min = val;
            self.max = val;
        } else {
            self.min = self.min.min(val);
            self.max = self.max.max(val);
        }
    }

    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        let mean = self.sum / self.count as f64;
        self.sum_sq / self.count as f64 - mean * mean
    }

    pub fn range(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.max - self.min
    }

    /// Merge another FieldStats into this one (for batch insert).
    fn merge(&mut self, other: &FieldStats) {
        if other.count == 0 { return; }
        if self.count == 0 {
            *self = other.clone();
            return;
        }
        self.count += other.count;
        self.sum += other.sum;
        self.sum_sq += other.sum_sq;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }
}

/// Incremental curvature moments over per-record K(p) scores.
///
/// Tracks the running mean μ_K and standard deviation σ_K so anomaly
/// thresholds can be evaluated in O(1) without scanning all records.
///
/// K(p) for a record is computed at insert time as the mean normalised
/// deviation of each numeric field from the running field mean:
///
///   K_record(p) = (1/n) Σᵢ |vᵢ − μᵢ| / max(range_i, ε)
///
/// This is the scalar curvature (Definition 3.4) per record — cheaper
/// than the 3-component form but geometrically equivalent for anomaly
/// scoring purposes.
#[derive(Debug, Clone, Default)]
pub struct CurvatureStats {
    /// Σ K(p) over all inserted records.
    pub k_sum: f64,
    /// Σ K(p)² over all inserted records (for variance).
    pub k_sum_sq: f64,
    /// Number of records whose K was measured.
    pub k_count: u64,
}

impl CurvatureStats {
    /// Update with a new record's curvature value.
    pub fn update(&mut self, k: f64) {
        self.k_sum += k;
        self.k_sum_sq += k * k;
        self.k_count += 1;
    }

    /// Running mean μ_K = Σ K / N.
    pub fn mean(&self) -> f64 {
        if self.k_count == 0 { return 0.0; }
        self.k_sum / self.k_count as f64
    }

    /// Running population standard deviation σ_K.
    pub fn std_dev(&self) -> f64 {
        if self.k_count < 2 { return 0.0; }
        let mean = self.mean();
        let var = self.k_sum_sq / self.k_count as f64 - mean * mean;
        var.max(0.0).sqrt()
    }

    /// Adaptive anomaly threshold: μ_K + n_sigma · σ_K.
    pub fn threshold(&self, n_sigma: f64) -> f64 {
        self.mean() + n_sigma * self.std_dev()
    }

    /// z-score of a given K value: (K − μ_K) / σ_K.
    pub fn z_score(&self, k: f64) -> f64 {
        let sd = self.std_dev();
        if sd < f64::EPSILON { return 0.0; }
        (k - self.mean()) / sd
    }

    /// True if k exceeds the n_sigma threshold.
    pub fn is_anomaly(&self, k: f64, n_sigma: f64) -> bool {
        k > self.threshold(n_sigma)
    }
}

/// Compute per-record scalar curvature K_record(p) in O(1) at insert time.
///
/// Uses the field_stats that existed **before** this record's contribution
/// so we measure how surprising the record is relative to prior data.
/// Returns 0.0 when fewer than 2 records have been seen (no baseline yet).
pub fn compute_record_k(
    field_stats: &HashMap<String, FieldStats>,
    fiber_vals: &[Value],
    fiber_fields: &[crate::types::FieldDef],
) -> f64 {
    let mut total = 0.0f64;
    let mut n = 0usize;
    for (i, field_def) in fiber_fields.iter().enumerate() {
        let Some(v) = fiber_vals.get(i).and_then(|v| v.as_f64()) else { continue };
        let Some(fs) = field_stats.get(&field_def.name) else { continue };
        if fs.count < 2 { continue; }
        let mean = fs.sum / fs.count as f64;
        let range = fs.range().max(f64::EPSILON);
        total += (v - mean).abs() / range;
        n += 1;
    }
    if n == 0 { 0.0 } else { total / n as f64 }
}

impl BundleStore {
    /// Create a new empty bundle. Always starts in Hashed mode;
    /// auto-detects flat base geometry after 32 inserts and switches if K=0.
    pub fn new(schema: BundleSchema) -> Self {
        let hash_config = HashConfig::from_schema(&schema);
        let mut field_index = HashMap::new();
        for name in &schema.indexed_fields {
            field_index.insert(name.clone(), HashMap::new());
        }
        Self {
            schema,
            hash_config,
            storage: BaseStorage::new_hashed(),
            field_index,
            field_stats: HashMap::new(),
            bp_reverse: HashMap::new(),
            seq_bp_list: Vec::new(),
            bp_to_idx: HashMap::new(),
            detect_keys: Vec::new(),
            detected: false,
            auto_id_counter: 0,
            curvature_stats: CurvatureStats::default(),
        }
    }

    /// Create a bundle with a specific geometry (skip auto-detection).
    pub fn with_geometry(schema: BundleSchema, geometry: BaseGeometry) -> Self {
        let hash_config = HashConfig::from_schema(&schema);
        let mut field_index = HashMap::new();
        for name in &schema.indexed_fields {
            field_index.insert(name.clone(), HashMap::new());
        }
        let storage = match &geometry {
            BaseGeometry::Curved => BaseStorage::new_hashed(),
            BaseGeometry::Flat { start, step, key_field } => BaseStorage::Sequential {
                sections: Vec::new(),
                base_values: Vec::new(),
                start: *start,
                step: *step,
                key_field: key_field.clone(),
            },
            BaseGeometry::NearlyFlat { start, step, key_field } => BaseStorage::Hybrid {
                sections: Vec::new(),
                base_values: Vec::new(),
                overflow_sections: HashMap::new(),
                overflow_base: HashMap::new(),
                start: *start,
                step: *step,
                key_field: key_field.clone(),
            },
        };
        Self {
            schema,
            hash_config,
            storage,
            field_index,
            field_stats: HashMap::new(),
            bp_reverse: HashMap::new(),
            seq_bp_list: Vec::new(),
            bp_to_idx: HashMap::new(),
            detect_keys: Vec::new(),
            detected: true, // geometry already set
            auto_id_counter: 0,
            curvature_stats: CurvatureStats::default(),
        }
    }

    /// Get the current storage mode as a string.
    pub fn storage_mode(&self) -> &'static str {
        match &self.storage {
            BaseStorage::Hashed { .. } => "hashed",
            BaseStorage::Sequential { .. } => "sequential",
            BaseStorage::Hybrid { .. } => "hybrid",
        }
    }

    /// Insert = define section value at base point (Thm 1.3).
    ///
    /// O(1) amortized. Overwrites on same key (Def 1.2 — unique section per base point).
    /// For Sequential/Hybrid mode, insert is array.push() — same as Druid's memcpy.
    pub fn insert(&mut self, record: &Record) {
        let bp = self.hash_config.hash(record, &self.schema);

        // Extract base field values
        let base_vals: Vec<Value> = self
            .schema
            .base_fields
            .iter()
            .map(|f| record.get(&f.name).cloned().unwrap_or(Value::Null))
            .collect();

        // Extract fiber field values
        let fiber_vals_raw: Vec<Value> = self
            .schema
            .fiber_fields
            .iter()
            .map(|f| record.get(&f.name).cloned().unwrap_or(f.default.clone()))
            .collect();

        // Apply geometric encryption (gauge transform) if enabled
        let fiber_vals = if let Some(ref gk) = self.schema.gauge_key {
            gk.encrypt_fiber(&fiber_vals_raw)
        } else {
            fiber_vals_raw
        };

        // Update field index (bitmap per indexed-field value)
        let bp32 = bp as u32;
        self.bp_reverse.insert(bp32, bp);
        for idx_field in &self.schema.indexed_fields {
            if let Some(val) = record.get(idx_field) {
                self.field_index
                    .entry(idx_field.clone())
                    .or_default()
                    .entry(val.clone())
                    .or_default()
                    .insert(bp32);
            }
        }

        // Compute per-record K BEFORE updating field_stats so we measure
        // how surprising this record is relative to the existing distribution.
        // This is K_scalar(p) = mean normalised |v - μ| / range  (Def 3.4).
        let k_record = compute_record_k(&self.field_stats, &fiber_vals, &self.schema.fiber_fields);
        self.curvature_stats.update(k_record);

        // Update field stats for curvature tracking
        for (i, field_def) in self.schema.fiber_fields.iter().enumerate() {
            if let Some(v) = fiber_vals[i].as_f64() {
                self.field_stats
                    .entry(field_def.name.clone())
                    .or_default()
                    .update(v);
            }
        }

        // Track key values for auto-detection (first 32 unique inserts)
        if !self.detected && self.schema.base_fields.len() == 1 {
            if let Some(val) = record.get(&self.schema.base_fields[0].name) {
                if let Some(kv) = val.as_i64() {
                    self.detect_keys.push(kv);
                }
            }
        }

        // Insert into storage
        self.insert_into_storage(bp, record, fiber_vals, base_vals);

        // Auto-detection: after 32 inserts, check if base is flat
        if !self.detected && self.detect_keys.len() >= 32 {
            self.detected = true;
            self.try_switch_storage();
        }

        // Auto-promotion: if Hybrid overflow exceeds 5%, promote to Hashed
        if self.storage.is_hybrid() && self.storage.overflow_ratio() > 0.05 {
            self.promote_storage();
        }
    }

    /// Batch insert — amortized overhead across N records.
    ///
    /// For single-integer-base schemas with no indexed fields, takes a turbo
    /// fast path that:
    ///   - Skips per-record hashing (uses hash_int_fast or direct push)
    ///   - Uses Vec-indexed stats (no per-record HashMap::entry)
    ///   - Pre-reserves all storage capacity
    ///   - Rebuilds auxiliary maps once at the end
    ///
    /// For other schemas, falls back to the general path with per-record
    /// insert_into_storage (still defers detection + promotion).
    ///
    /// Returns the number of records actually inserted.
    pub fn batch_insert(&mut self, records: &[Record]) -> usize {
        if records.is_empty() { return 0; }

        // Check if the turbo fast path is available
        let single_int_base = self.schema.base_fields.len() == 1
            && matches!(self.schema.base_fields[0].field_type, FieldType::Numeric);
        let no_indexed = self.schema.indexed_fields.is_empty();

        if single_int_base && no_indexed {
            return self.batch_insert_fast(records);
        }

        // ── General path (indexed fields or composite keys) ─────────
        let mut count = 0usize;

        for record in records {
            let bp = self.hash_config.hash(record, &self.schema);

            let base_vals: Vec<Value> = self.schema.base_fields.iter()
                .map(|f| record.get(&f.name).cloned().unwrap_or(Value::Null))
                .collect();
            let fiber_vals: Vec<Value> = self.schema.fiber_fields.iter()
                .map(|f| record.get(&f.name).cloned().unwrap_or(f.default.clone()))
                .collect();

            let bp32 = bp as u32;
            self.bp_reverse.insert(bp32, bp);
            for idx_field in &self.schema.indexed_fields {
                if let Some(val) = record.get(idx_field) {
                    self.field_index
                        .entry(idx_field.clone())
                        .or_default()
                        .entry(val.clone())
                        .or_default()
                        .insert(bp32);
                }
            }

            for (i, field_def) in self.schema.fiber_fields.iter().enumerate() {
                if let Some(v) = fiber_vals[i].as_f64() {
                    self.field_stats
                        .entry(field_def.name.clone())
                        .or_default()
                        .update(v);
                }
            }

            if !self.detected && self.schema.base_fields.len() == 1 {
                if let Some(val) = record.get(&self.schema.base_fields[0].name) {
                    if let Some(kv) = val.as_i64() {
                        self.detect_keys.push(kv);
                    }
                }
            }

            self.insert_into_storage(bp, record, fiber_vals, base_vals);
            count += 1;
        }

        if !self.detected && self.detect_keys.len() >= 32 {
            self.detected = true;
            self.try_switch_storage();
        }

        if self.storage.is_hybrid() && self.storage.overflow_ratio() > 0.05 {
            self.promote_storage();
        }

        count
    }

    /// Turbo fast path for batch insert — single integer base, no indexed fields.
    ///
    /// Skips per-record hashing for Sequential mode, uses Vec-indexed stats,
    /// and rebuilds auxiliary maps in a single pass at the end.
    fn batch_insert_fast(&mut self, records: &[Record]) -> usize {
        let key_field = self.schema.base_fields[0].name.clone();
        let n_fiber = self.schema.fiber_fields.len();
        let fiber_names: Vec<String> = self.schema.fiber_fields.iter()
            .map(|f| f.name.clone()).collect();
        let fiber_defaults: Vec<Value> = self.schema.fiber_fields.iter()
            .map(|f| f.default.clone()).collect();
        let gauge_key = self.schema.gauge_key.clone();
        let is_seq = matches!(self.storage, BaseStorage::Sequential { .. });
        let track_detect = !self.detected;

        // Pre-reserve storage capacity
        let n = records.len();
        match &mut self.storage {
            BaseStorage::Sequential { sections, base_values, .. } => {
                sections.reserve(n);
                base_values.reserve(n);
            }
            BaseStorage::Hashed { sections, base_values } => {
                sections.reserve(n);
                base_values.reserve(n);
            }
            _ => {}
        }

        // Vec-indexed stats — avoids per-record HashMap::entry + string clone
        let mut local_stats: Vec<FieldStats> = vec![FieldStats::default(); n_fiber];
        let mut count = 0usize;

        if is_seq {
            // ── SEQUENTIAL TURBO: no hashing, direct Vec push ──
            for record in records {
                let key_val = match record.get(&key_field).and_then(|v| v.as_i64()) {
                    Some(k) => k,
                    None => continue,
                };

                let mut fiber_vals = Vec::with_capacity(n_fiber);
                for (i, name) in fiber_names.iter().enumerate() {
                    let val = record.get(name).cloned()
                        .unwrap_or_else(|| fiber_defaults[i].clone());
                    if let Some(v) = val.as_f64() {
                        local_stats[i].update(v);
                    }
                    fiber_vals.push(val);
                }

                // Apply geometric encryption if enabled
                let fiber_vals = if let Some(ref gk) = gauge_key {
                    gk.encrypt_fiber(&fiber_vals)
                } else {
                    fiber_vals
                };

                match &mut self.storage {
                    BaseStorage::Sequential { sections, base_values, .. } => {
                        sections.push(fiber_vals);
                        base_values.push(vec![Value::Integer(key_val)]);
                    }
                    _ => {}
                }
                count += 1;
            }

            // Rebuild maps (bp_reverse, bp_to_idx, seq_bp_list) for sequential turbo path
            // so that update/delete/range_query work correctly after batch insert.
            if let BaseStorage::Sequential { sections, start, step, .. } = &self.storage {
                self.seq_bp_list.clear();
                self.bp_to_idx.clear();
                for i in 0..sections.len() {
                    let key_val = *start + (*step * i as i64);
                    let bp = self.hash_config.hash_int_fast(key_val);
                    let bp32 = bp as u32;
                    self.seq_bp_list.push(bp);
                    self.bp_to_idx.insert(bp, i);
                    self.bp_reverse.insert(bp32, bp);
                }
            }
        } else {
            // ── HASHED TURBO: hash_int_fast, skip per-record bp_reverse ──
            for record in records {
                let key_val = match record.get(&key_field).and_then(|v| v.as_i64()) {
                    Some(k) => k,
                    None => continue,
                };
                let bp = self.hash_config.hash_int_fast(key_val);

                let mut fiber_vals = Vec::with_capacity(n_fiber);
                for (i, name) in fiber_names.iter().enumerate() {
                    let val = record.get(name).cloned()
                        .unwrap_or_else(|| fiber_defaults[i].clone());
                    if let Some(v) = val.as_f64() {
                        local_stats[i].update(v);
                    }
                    fiber_vals.push(val);
                }

                // Apply geometric encryption if enabled
                let fiber_vals = if let Some(ref gk) = gauge_key {
                    gk.encrypt_fiber(&fiber_vals)
                } else {
                    fiber_vals
                };

                let base_vals = vec![Value::Integer(key_val)];

                match &mut self.storage {
                    BaseStorage::Hashed { sections, base_values } => {
                        sections.insert(bp, fiber_vals);
                        base_values.insert(bp, base_vals);
                    }
                    _ => {}
                }

                if track_detect {
                    self.detect_keys.push(key_val);
                }

                count += 1;
            }

            // bp_reverse not rebuilt here — no indexed fields means
            // range_query returns empty. try_switch_storage (if triggered)
            // builds its own maps.
        }

        // Merge local stats into global
        for (i, name) in fiber_names.iter().enumerate() {
            self.field_stats.entry(name.clone()).or_default().merge(&local_stats[i]);
        }

        // Deferred auto-detection
        if !self.detected && self.detect_keys.len() >= 32 {
            self.detected = true;
            self.try_switch_storage();
        }

        // Deferred hybrid promotion
        if self.storage.is_hybrid() && self.storage.overflow_ratio() > 0.05 {
            self.promote_storage();
        }

        count
    }

    /// Update a record: apply partial field patches to an existing record.
    /// Returns true if the record existed and was updated, false otherwise.
    pub fn update(&mut self, key: &Record, patches: &Record) -> bool {
        let bp = self.hash_config.hash(key, &self.schema);

        // Read existing record
        let existing = match self.reconstruct(bp) {
            Some(r) => r,
            None => return false,
        };

        // Merge patches into existing record
        let mut merged = existing;
        for (field, value) in patches {
            merged.insert(field.clone(), value.clone());
        }

        // Remove old field_index entries
        let bp32 = bp as u32;
        for idx_field in &self.schema.indexed_fields {
            if let Some(old_val) = key.get(idx_field).or_else(|| merged.get(idx_field)) {
                if let Some(field_map) = self.field_index.get_mut(idx_field) {
                    if let Some(bitmap) = field_map.get_mut(old_val) {
                        bitmap.remove(bp32);
                    }
                }
            }
        }

        // Extract new fiber values
        let fiber_vals: Vec<Value> = self.schema.fiber_fields.iter()
            .map(|f| merged.get(&f.name).cloned().unwrap_or(f.default.clone()))
            .collect();
        let base_vals: Vec<Value> = self.schema.base_fields.iter()
            .map(|f| merged.get(&f.name).cloned().unwrap_or(Value::Null))
            .collect();

        // Re-add field_index entries with new values
        for idx_field in &self.schema.indexed_fields {
            if let Some(val) = merged.get(idx_field) {
                self.field_index
                    .entry(idx_field.clone())
                    .or_default()
                    .entry(val.clone())
                    .or_default()
                    .insert(bp32);
            }
        }

        // Update field stats for new fiber values
        for (i, field_def) in self.schema.fiber_fields.iter().enumerate() {
            if let Some(v) = fiber_vals[i].as_f64() {
                self.field_stats
                    .entry(field_def.name.clone())
                    .or_default()
                    .update(v);
            }
        }

        // Write back into storage
        self.overwrite_storage(bp, fiber_vals, base_vals);
        true
    }

    /// Bulk update: find all records matching conditions, apply patches to each.
    /// Returns number of records updated.
    pub fn bulk_update(&mut self, conditions: &[QueryCondition], patches: &Record) -> usize {
        // Collect matching keys first (can't iterate and mutate simultaneously)
        let matching_keys: Vec<Record> = self.records()
            .filter(|record| conditions.iter().all(|c| c.matches(record)))
            .map(|record| {
                let mut key = Record::new();
                for f in &self.schema.base_fields {
                    if let Some(v) = record.get(&f.name) {
                        key.insert(f.name.clone(), v.clone());
                    }
                }
                key
            })
            .collect();

        let mut count = 0;
        for key in &matching_keys {
            if self.update(key, patches) {
                count += 1;
            }
        }
        count
    }

    /// Delete a record by key. Returns true if the record existed and was removed.
    pub fn delete(&mut self, key: &Record) -> bool {
        let bp = self.hash_config.hash(key, &self.schema);
        let bp32 = bp as u32;

        // Remove field_index entries
        if let Some(existing) = self.reconstruct(bp) {
            for idx_field in &self.schema.indexed_fields {
                if let Some(val) = existing.get(idx_field) {
                    if let Some(field_map) = self.field_index.get_mut(idx_field) {
                        if let Some(bitmap) = field_map.get_mut(val) {
                            bitmap.remove(bp32);
                        }
                    }
                }
            }
        } else {
            return false;
        }

        // Remove from storage
        self.remove_from_storage(bp);
        self.bp_reverse.remove(&bp32);
        true
    }

    /// Overwrite fiber+base at an existing base point.
    fn overwrite_storage(&mut self, bp: BasePoint, fiber_vals: Vec<Value>, base_vals: Vec<Value>) {
        match &mut self.storage {
            BaseStorage::Hashed { sections, base_values } => {
                sections.insert(bp, fiber_vals);
                base_values.insert(bp, base_vals);
            }
            BaseStorage::Sequential { sections, base_values, .. }
            | BaseStorage::Hybrid { sections, base_values, .. } => {
                if let Some(&idx) = self.bp_to_idx.get(&bp) {
                    if idx < sections.len() {
                        sections[idx] = fiber_vals;
                        base_values[idx] = base_vals;
                    }
                } else if let BaseStorage::Hybrid { overflow_sections, overflow_base, .. } = &mut self.storage {
                    overflow_sections.insert(bp, fiber_vals);
                    overflow_base.insert(bp, base_vals);
                }
            }
        }
    }

    /// Remove a record from storage by base point.
    fn remove_from_storage(&mut self, bp: BasePoint) {
        match &mut self.storage {
            BaseStorage::Hashed { sections, base_values } => {
                sections.remove(&bp);
                base_values.remove(&bp);
            }
            BaseStorage::Sequential { sections, base_values, .. } => {
                // For Sequential, we can't easily remove without shifting.
                // Mark as tombstone (Null values) — reconstruct will skip.
                if let Some(&idx) = self.bp_to_idx.get(&bp) {
                    if idx < sections.len() {
                        // Replace with empty vecs (tombstone)
                        sections[idx] = vec![Value::Null; sections[idx].len()];
                        base_values[idx] = vec![Value::Null; base_values[idx].len()];
                        // Mark as deleted by removing from bp_to_idx
                        self.bp_to_idx.remove(&bp);
                    }
                }
            }
            BaseStorage::Hybrid {
                sections, base_values,
                overflow_sections, overflow_base, ..
            } => {
                if let Some(&idx) = self.bp_to_idx.get(&bp) {
                    if idx < sections.len() {
                        sections[idx] = vec![Value::Null; sections[idx].len()];
                        base_values[idx] = vec![Value::Null; base_values[idx].len()];
                        self.bp_to_idx.remove(&bp);
                    }
                } else {
                    overflow_sections.remove(&bp);
                    overflow_base.remove(&bp);
                }
            }
        }
    }

    /// Insert fiber+base values into the current storage variant.
    fn insert_into_storage(&mut self, bp: BasePoint, record: &Record, fiber_vals: Vec<Value>, base_vals: Vec<Value>) {
        match &self.storage {
            BaseStorage::Hashed { .. } => {
                self.storage.insert_hashed(bp, fiber_vals, base_vals);
            }
            BaseStorage::Sequential { key_field, .. } | BaseStorage::Hybrid { key_field, .. } => {
                let kf = key_field.clone();
                if let Some(val) = record.get(&kf) {
                    if let Some(key_val) = val.as_i64() {
                        // Check for overwrite (same bp already exists)
                        if let Some(&existing_idx) = self.bp_to_idx.get(&bp) {
                            // Overwrite in-place — faster than HashMap update
                            match &mut self.storage {
                                BaseStorage::Sequential { sections, base_values, .. } => {
                                    sections[existing_idx] = fiber_vals;
                                    base_values[existing_idx] = base_vals;
                                }
                                BaseStorage::Hybrid { sections, base_values, .. } => {
                                    if existing_idx < sections.len() {
                                        sections[existing_idx] = fiber_vals;
                                        base_values[existing_idx] = base_vals;
                                    }
                                }
                                _ => {}
                            }
                            return;
                        }
                        let idx = match &self.storage {
                            BaseStorage::Sequential { sections, .. } => sections.len(),
                            BaseStorage::Hybrid { sections, .. } => sections.len(),
                            _ => 0,
                        };
                        self.seq_bp_list.push(bp);
                        self.bp_to_idx.insert(bp, idx);
                        self.storage.insert_with_key(bp, key_val, fiber_vals, base_vals);
                        return;
                    }
                }
                // Fallback: key not extractable
                self.storage.insert_hashed(bp, fiber_vals, base_vals);
            }
        }
    }

    /// Try to switch from Hashed to Sequential/Hybrid after auto-detection.
    /// Drains the HashMap and rebuilds as a sorted Vec.
    fn try_switch_storage(&mut self) {
        if self.detect_keys.len() < 2 {
            return;
        }
        if self.schema.base_fields.len() != 1 {
            return;
        }

        let start = self.detect_keys[0];
        let step = self.detect_keys[1] - self.detect_keys[0];
        if step == 0 {
            return;
        }

        let total = self.detect_keys.len() - 1;
        let arithmetic_count = self.detect_keys.windows(2)
            .filter(|w| w[1] - w[0] == step)
            .count();

        let ratio = arithmetic_count as f64 / total as f64;

        if ratio < 0.95 {
            return; // K > 0, stay Hashed
        }

        let key_field = self.schema.base_fields[0].name.clone();

        // Drain current Hashed storage, sort by key, rebuild as Sequential/Hybrid
        let (old_sections, old_base) = match std::mem::replace(&mut self.storage, BaseStorage::new_hashed()) {
            BaseStorage::Hashed { sections, base_values } => (sections, base_values),
            _ => return, // shouldn't happen
        };

        // Collect all (key_value, bp, fiber, base)
        let mut entries: Vec<(i64, BasePoint, Vec<Value>, Vec<Value>)> = Vec::with_capacity(old_sections.len());
        for (bp, fiber) in &old_sections {
            if let Some(base) = old_base.get(bp) {
                // Extract key value from base values
                let key_val = base.first().and_then(|v| v.as_i64()).unwrap_or(0);
                entries.push((key_val, *bp, fiber.clone(), base.clone()));
            }
        }
        entries.sort_by_key(|e| e.0);

        // Build Sequential or Hybrid
        self.seq_bp_list.clear();
        self.bp_to_idx.clear();

        if ratio == 1.0 {
            let mut sec_vec = Vec::with_capacity(entries.len());
            let mut base_vec = Vec::with_capacity(entries.len());
            for (i, (_kv, bp, fiber, base)) in entries.into_iter().enumerate() {
                self.seq_bp_list.push(bp);
                self.bp_to_idx.insert(bp, i);
                sec_vec.push(fiber);
                base_vec.push(base);
            }
            self.storage = BaseStorage::Sequential {
                sections: sec_vec,
                base_values: base_vec,
                start,
                step,
                key_field,
            };
        } else {
            // Nearly flat: put arithmetic entries in Vec, rest in overflow
            let mut sec_vec = Vec::with_capacity(entries.len());
            let mut base_vec = Vec::with_capacity(entries.len());
            let mut overflow_sec = HashMap::new();
            let mut overflow_base = HashMap::new();
            let mut expected = start;
            for (_kv, bp, fiber, base) in entries {
                if _kv == expected {
                    let idx = sec_vec.len();
                    self.seq_bp_list.push(bp);
                    self.bp_to_idx.insert(bp, idx);
                    sec_vec.push(fiber);
                    base_vec.push(base);
                    expected += step;
                } else {
                    overflow_sec.insert(bp, fiber);
                    overflow_base.insert(bp, base);
                }
            }
            self.storage = BaseStorage::Hybrid {
                sections: sec_vec,
                base_values: base_vec,
                overflow_sections: overflow_sec,
                overflow_base: overflow_base,
                start,
                step,
                key_field,
            };
        }
    }

    /// Promote from Sequential/Hybrid to Hashed (curvature increased).
    fn promote_storage(&mut self) {
        let bp_list: Vec<(BasePoint, usize)> = self.seq_bp_list.iter().copied().enumerate().map(|(i, bp)| (bp, i)).collect();
        let overflow_bps = HashMap::new();
        let old = std::mem::replace(&mut self.storage, BaseStorage::new_hashed());
        self.storage = old.promote_to_hashed(&bp_list, &overflow_bps);
        self.seq_bp_list.clear();
        self.bp_to_idx.clear();
    }

    /// Point query — O(1) section evaluation σ(p) (Thm 1.2).
    /// Sequential mode: arithmetic index (k-start)/step — no hash needed. ~2ns.
    pub fn point_query(&self, key: &Record) -> Option<Record> {
        // Fast path for Sequential: arithmetic indexing, no hashing needed
        if let BaseStorage::Sequential { start, step, key_field, .. } = &self.storage {
            if let Some(key_val) = key.get(key_field).and_then(|v| v.as_i64()) {
                let diff = key_val - start;
                if diff < 0 || *step == 0 || diff % step != 0 { return None; }
                let idx = (diff / step) as usize;
                if let Some((fiber, base)) = self.storage.get_by_index(idx) {
                    let mut record = Record::new();
                    for (i, f) in self.schema.base_fields.iter().enumerate() {
                        record.insert(f.name.clone(), base[i].clone());
                    }
                    // Decrypt fiber values if geometric encryption is enabled
                    if let Some(ref gk) = self.schema.gauge_key {
                        let decrypted = gk.decrypt_fiber(fiber);
                        for (i, f) in self.schema.fiber_fields.iter().enumerate() {
                            record.insert(f.name.clone(), decrypted[i].clone());
                        }
                    } else {
                        for (i, f) in self.schema.fiber_fields.iter().enumerate() {
                            record.insert(f.name.clone(), fiber[i].clone());
                        }
                    }
                    return Some(record);
                }
                return None;
            }
        }
        // General path
        let bp = self.hash_config.hash(key, &self.schema);
        self.reconstruct(bp)
    }

    /// Range query — sheaf evaluation F(U) (Thm 2.4).
    ///
    /// O(|values| + |result|), independent of N.
    pub fn range_query(&self, field: &str, values: &[Value]) -> Vec<Record> {
        let mut bits = RoaringBitmap::new();
        if let Some(field_map) = self.field_index.get(field) {
            for val in values {
                if let Some(val_bits) = field_map.get(val) {
                    bits |= val_bits;
                }
            }
        }
        bits.iter()
            .filter_map(|bp32| {
                let bp = self.bp_reverse.get(&bp32).copied().unwrap_or(bp32 as u64);
                self.reconstruct(bp)
            })
            .collect()
    }

    /// Filtered query with multi-condition AND, comparison operators, text search,
    /// sort, limit, and offset. Scans all records and applies conditions.
    ///
    /// Conditions:
    ///   - `Eq(field, value)` — exact match
    ///   - `Neq(field, value)` — not equal
    ///   - `Gt(field, value)` — greater than
    ///   - `Gte(field, value)` — greater than or equal
    ///   - `Lt(field, value)` — less than
    ///   - `Lte(field, value)` — less than or equal
    ///   - `Contains(field, substring)` — text contains (case-insensitive)
    ///   - `StartsWith(field, prefix)` — text starts with
    pub fn filtered_query(
        &self,
        conditions: &[QueryCondition],
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<Record> {
        self.filtered_query_ex(conditions, None, sort_by, sort_desc, limit, offset)
    }

    /// Extended filtered query with OR condition support.
    /// Uses bitmap indexes to accelerate Eq/In conditions on indexed fields.
    pub fn filtered_query_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<Record> {
        // Try to accelerate via bitmap indexes: extract Eq/In conditions on indexed fields
        let indexed_fields: std::collections::HashSet<&str> = self.schema.indexed_fields.iter().map(|s| s.as_str()).collect();
        let mut index_bits: Option<RoaringBitmap> = None;
        let mut remaining_conditions: Vec<&QueryCondition> = Vec::new();

        for cond in conditions {
            match cond {
                QueryCondition::Eq(field, value) if indexed_fields.contains(field.as_str()) => {
                    let mut bits = RoaringBitmap::new();
                    if let Some(field_map) = self.field_index.get(field.as_str()) {
                        if let Some(val_bits) = field_map.get(value) {
                            bits |= val_bits;
                        }
                    }
                    index_bits = Some(match index_bits {
                        Some(existing) => existing & bits,
                        None => bits,
                    });
                }
                QueryCondition::In(field, values) if indexed_fields.contains(field.as_str()) => {
                    let mut bits = RoaringBitmap::new();
                    if let Some(field_map) = self.field_index.get(field.as_str()) {
                        for val in values {
                            if let Some(val_bits) = field_map.get(val) {
                                bits |= val_bits;
                            }
                        }
                    }
                    index_bits = Some(match index_bits {
                        Some(existing) => existing & bits,
                        None => bits,
                    });
                }
                _ => {
                    remaining_conditions.push(cond);
                }
            }
        }

        // If we narrowed via index, reconstruct only candidate records
        let mut results: Vec<Record> = if let Some(bits) = index_bits {
            bits.iter()
                .filter_map(|bp32| {
                    let bp = self.bp_reverse.get(&bp32).copied().unwrap_or(bp32 as u64);
                    self.reconstruct(bp)
                })
                .filter(|record| {
                    remaining_conditions.iter().all(|c| c.matches(record))
                        && matches_or_filter(record, or_conditions)
                })
                .collect()
        } else {
            self.records()
                .filter(|record| {
                    matches_filter(record, conditions, or_conditions)
                })
                .collect()
        };

        // Sort
        if let Some(field) = sort_by {
            let field = field.to_string();
            results.sort_by(|a, b| {
                let va = a.get(&field).unwrap_or(&Value::Null);
                let vb = b.get(&field).unwrap_or(&Value::Null);
                if sort_desc { vb.cmp(va) } else { va.cmp(vb) }
            });
        }

        // Offset + Limit
        let start = offset.unwrap_or(0);
        if start > 0 {
            results = results.into_iter().skip(start).collect();
        }
        if let Some(lim) = limit {
            results.truncate(lim);
        }

        results
    }

    /// Reconstruct a full record from base point.
    fn reconstruct(&self, bp: BasePoint) -> Option<Record> {
        let (fiber, base) = match &self.storage {
            BaseStorage::Hashed { sections, base_values } => {
                let f = sections.get(&bp)?;
                let b = base_values.get(&bp)?;
                (f.as_slice(), b.as_slice())
            }
            BaseStorage::Sequential { .. } | BaseStorage::Hybrid { .. } => {
                if let Some(&idx) = self.bp_to_idx.get(&bp) {
                    self.storage.get_by_index(idx)?
                } else {
                    match &self.storage {
                        BaseStorage::Hybrid { overflow_sections, overflow_base, .. } => {
                            let f = overflow_sections.get(&bp)?;
                            let b = overflow_base.get(&bp)?;
                            (f.as_slice(), b.as_slice())
                        }
                        _ => return None,
                    }
                }
            }
        };

        let mut record = Record::new();
        for (i, field_def) in self.schema.base_fields.iter().enumerate() {
            record.insert(field_def.name.clone(), base[i].clone());
        }
        // Decrypt fiber values if geometric encryption is enabled
        if let Some(ref gk) = self.schema.gauge_key {
            let decrypted = gk.decrypt_fiber(fiber);
            for (i, field_def) in self.schema.fiber_fields.iter().enumerate() {
                record.insert(field_def.name.clone(), decrypted[i].clone());
            }
        } else {
            for (i, field_def) in self.schema.fiber_fields.iter().enumerate() {
                record.insert(field_def.name.clone(), fiber[i].clone());
            }
        }
        Some(record)
    }

    /// Get raw fiber values at a base point.
    pub fn get_fiber(&self, bp: BasePoint) -> Option<&[Value]> {
        match &self.storage {
            BaseStorage::Hashed { sections, .. } => sections.get(&bp).map(|v| v.as_slice()),
            BaseStorage::Sequential { .. } | BaseStorage::Hybrid { .. } => {
                if let Some(&idx) = self.bp_to_idx.get(&bp) {
                    self.storage.get_by_index(idx).map(|(f, _)| f)
                } else {
                    match &self.storage {
                        BaseStorage::Hybrid { overflow_sections, .. } => {
                            overflow_sections.get(&bp).map(|v| v.as_slice())
                        }
                        _ => None,
                    }
                }
            }
        }
    }

    /// Get the base point for a key record.
    pub fn base_point(&self, key: &Record) -> BasePoint {
        self.hash_config.hash(key, &self.schema)
    }

    /// Number of stored sections.
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Iterate over all (base_point, fiber_values).
    pub fn sections(&self) -> Box<dyn Iterator<Item = (BasePoint, &[Value])> + '_> {
        match &self.storage {
            BaseStorage::Hashed { sections, .. } => {
                Box::new(sections.iter().map(|(&bp, v)| (bp, v.as_slice())))
            }
            BaseStorage::Sequential { sections, start, step, .. } => {
                if self.seq_bp_list.len() == sections.len() {
                    // Maps are populated — use them directly
                    Box::new(
                        self.seq_bp_list.iter().copied()
                            .zip(sections.iter())
                            .map(|(bp, v)| (bp, v.as_slice()))
                    )
                } else {
                    // Maps stale (after fast batch) — compute bp on-the-fly
                    let start = *start;
                    let step = *step;
                    let bps: Vec<BasePoint> = (0..sections.len())
                        .map(|i| self.hash_config.hash_int_fast(start + step * i as i64))
                        .collect();
                    Box::new(bps.into_iter().zip(sections.iter()).map(|(bp, v)| (bp, v.as_slice())))
                }
            }
            BaseStorage::Hybrid { sections, overflow_sections, .. } => {
                let seq_iter = self.seq_bp_list.iter().copied()
                    .zip(sections.iter())
                    .map(|(bp, v)| (bp, v.as_slice()));
                let overflow_iter = overflow_sections.iter().map(|(&bp, v)| (bp, v.as_slice()));
                Box::new(seq_iter.chain(overflow_iter))
            }
        }
    }

    /// Iterate over all reconstructed records.
    pub fn records(&self) -> Box<dyn Iterator<Item = Record> + '_> {
        match &self.storage {
            BaseStorage::Hashed { sections, .. } => {
                Box::new(sections.keys().filter_map(move |&bp| self.reconstruct(bp)))
            }
            BaseStorage::Sequential { sections, base_values, .. } => {
                if self.seq_bp_list.len() == sections.len() {
                    let bps: Vec<BasePoint> = self.seq_bp_list.clone();
                    Box::new(bps.into_iter().filter_map(move |bp| self.reconstruct(bp)))
                } else {
                    // Maps stale — reconstruct directly from storage arrays
                    let base_names: Vec<String> = self.schema.base_fields.iter()
                        .map(|f| f.name.clone()).collect();
                    let fiber_names: Vec<String> = self.schema.fiber_fields.iter()
                        .map(|f| f.name.clone()).collect();
                    let gauge_key = self.schema.gauge_key.clone();
                    let secs = sections.as_slice();
                    let bases = base_values.as_slice();
                    let n = secs.len();
                    Box::new((0..n).filter_map(move |i| {
                        let fiber = secs.get(i)?;
                        let base = bases.get(i)?;
                        let mut record = Record::new();
                        for (j, name) in base_names.iter().enumerate() {
                            if j < base.len() {
                                record.insert(name.clone(), base[j].clone());
                            }
                        }
                        // Decrypt fiber values if geometric encryption is enabled
                        if let Some(ref gk) = gauge_key {
                            let decrypted = gk.decrypt_fiber(fiber);
                            for (j, name) in fiber_names.iter().enumerate() {
                                if j < decrypted.len() {
                                    record.insert(name.clone(), decrypted[j].clone());
                                }
                            }
                        } else {
                            for (j, name) in fiber_names.iter().enumerate() {
                                if j < fiber.len() {
                                    record.insert(name.clone(), fiber[j].clone());
                                }
                            }
                        }
                        Some(record)
                    }))
                }
            }
            BaseStorage::Hybrid { overflow_sections, .. } => {
                let mut bps: Vec<BasePoint> = self.seq_bp_list.clone();
                bps.extend(overflow_sections.keys());
                Box::new(bps.into_iter().filter_map(move |bp| self.reconstruct(bp)))
            }
        }
    }

    /// Get field stats for curvature computation.
    pub fn field_stats(&self) -> &HashMap<String, FieldStats> {
        &self.field_stats
    }

    /// Compute K_record(p) for an already-stored record identified by base point.
    ///
    /// Uses current bundle field_stats (the full distribution) so the score
    /// reflects how anomalous the record is relative to the *entire* bundle.
    pub fn record_k_for(&self, bp: BasePoint) -> f64 {
        let Some(fiber) = self.get_fiber(bp) else { return 0.0 };
        let fiber_vals: Vec<Value> = fiber.to_vec();
        compute_record_k(&self.field_stats, &fiber_vals, &self.schema.fiber_fields)
    }

    /// Scan entire bundle and return records that are geometrically anomalous.
    ///
    /// An anomaly is a record whose K(p) exceeds μ_K + n_sigma · σ_K.
    /// Results are sorted by z-score descending (most anomalous first).
    ///
    /// μ_K and σ_K are computed fresh from all candidate records in-query so
    /// that the reference distribution is always consistent with the current
    /// field_stats (avoids inserting anomalies that then dilute their own score).
    ///
    /// Optionally restrict to records matching `pre_filter` conditions (sheaf
    /// restriction to an open set before curvature scoring).
    ///
    /// Returns `(record, k_record, z_score, contributing_fields)` tuples.
    pub fn compute_anomalies(
        &self,
        n_sigma: f64,
        pre_filter: Option<&[QueryCondition]>,
        limit: usize,
    ) -> Vec<AnomalyRecord> {
        // Collect candidate base points (optionally pre-filtered)
        let base_points: Vec<BasePoint> = if let Some(conditions) = pre_filter {
            if conditions.is_empty() {
                self.all_base_points()
            } else {
                let records = self.filtered_query(conditions, None, false, None, None);
                records.into_iter().map(|rec| {
                    self.hash_config.hash(&rec, &self.schema)
                }).collect()
            }
        } else {
            self.all_base_points()
        };

        if base_points.is_empty() { return Vec::new(); }

        // First pass: compute K for every candidate record so μ_K and σ_K are
        // derived from the same field_stats that record_k_for uses.  This ensures
        // the reference distribution is consistent with the data at query time.
        let scores: Vec<(BasePoint, f64)> = base_points.iter()
            .map(|&bp| (bp, self.record_k_for(bp)))
            .collect();

        let n = scores.len() as f64;
        let mu = scores.iter().map(|(_, k)| k).sum::<f64>() / n;
        let var = scores.iter().map(|(_, k)| (k - mu).powi(2)).sum::<f64>() / n;
        let sigma = var.sqrt();
        let threshold = mu + n_sigma * sigma;

        // Second pass: filter to anomalies and annotate
        let mut anomalies: Vec<AnomalyRecord> = scores
            .into_iter()
            .filter(|(_, k)| *k > threshold)
            .filter_map(|(bp, k)| {
                let z = if sigma < f64::EPSILON { 0.0 } else { (k - mu) / sigma };
                let record = self.reconstruct(bp)?;
                let contributing = self.contributing_fields(bp, &record);
                let dev_norm = self.deviation_norm(bp);
                let dev_dist = self.deviation_distance(bp);
                let neighbourhood = self.geometric_neighbors(bp).len();
                Some(AnomalyRecord {
                    record,
                    local_curvature: k,
                    z_score: z,
                    confidence: 1.0 / (1.0 + k),
                    deviation_norm: dev_norm,
                    deviation_distance: dev_dist,
                    contributing_fields: contributing,
                    neighbourhood_size: neighbourhood,
                })
            })
            .collect();

        anomalies.sort_by(|a, b| b.z_score.partial_cmp(&a.z_score).unwrap_or(std::cmp::Ordering::Equal));
        anomalies.truncate(limit);
        anomalies
    }

    /// Determine which fields are contributing most to a record's anomaly score.
    ///
    /// A field contributes if its normalised deviation > mean normalised deviation
    /// across all fields for this record.
    fn contributing_fields(&self, _bp: BasePoint, record: &Record) -> Vec<String> {
        let mut field_devs: Vec<(String, f64)> = Vec::new();
        for field_def in &self.schema.fiber_fields {
            let Some(fs) = self.field_stats.get(&field_def.name) else { continue };
            if fs.count < 2 { continue; }
            let Some(v) = record.get(&field_def.name).and_then(|v| v.as_f64()) else { continue };
            let mean = fs.sum / fs.count as f64;
            let range = fs.range().max(f64::EPSILON);
            field_devs.push((field_def.name.clone(), (v - mean).abs() / range));
        }
        if field_devs.is_empty() { return Vec::new(); }
        let mean_dev = field_devs.iter().map(|(_, d)| d).sum::<f64>() / field_devs.len() as f64;
        field_devs.into_iter()
            .filter(|(_, d)| *d > mean_dev)
            .map(|(name, _)| name)
            .collect()
    }

    /// Fiber metric distance from the zero section (Def 1.7).
    pub fn deviation_distance(&self, bp: BasePoint) -> f64 {
        let Some(fiber) = self.get_fiber(bp) else { return 0.0 };
        let zero = self.schema.zero_section();
        let n_fields = self.schema.fiber_fields.len();
        if n_fields == 0 { return 0.0; }
        let mut sq_sum = 0.0f64;
        for (i, field_def) in self.schema.fiber_fields.iter().enumerate() {
            let fv = fiber.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let zv = zero.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let range = self.field_stats.get(&field_def.name)
                .map(|fs| fs.range().max(f64::EPSILON))
                .unwrap_or(1.0);
            sq_sum += ((fv - zv) / range).powi(2);
        }
        sq_sum.sqrt()
    }

    /// Collect all base points in the bundle.
    fn all_base_points(&self) -> Vec<BasePoint> {
        match &self.storage {
            BaseStorage::Hashed { sections, .. } => sections.keys().copied().collect(),
            BaseStorage::Sequential { .. } => self.seq_bp_list.clone(),
            BaseStorage::Hybrid { overflow_sections, .. } => {
                let mut bps = self.seq_bp_list.clone();
                bps.extend(overflow_sections.keys());
                bps
            }
        }
    }


    pub fn deviation_norm(&self, bp: BasePoint) -> usize {
        let fiber = match self.get_fiber(bp) {
            Some(f) => f,
            None => return 0,
        };
        let zero = self.schema.zero_section();
        fiber
            .iter()
            .zip(zero.iter())
            .filter(|(v, d)| v != d)
            .count()
    }

    /// Get base points sharing a field value (neighborhood).
    pub fn neighborhood(&self, field: &str, value: &Value) -> Vec<BasePoint> {
        self.field_index
            .get(field)
            .and_then(|m| m.get(value))
            .map(|bits| {
                bits.iter()
                    .map(|bp32| self.bp_reverse.get(&bp32).copied().unwrap_or(bp32 as u64))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all geometric neighbors of a base point across all indexed fields.
    pub fn geometric_neighbors(&self, bp: BasePoint) -> Vec<BasePoint> {
        let bp32 = bp as u32;
        let mut neighbors = std::collections::HashSet::new();

        for (_field_name, field_map) in &self.field_index {
            for (_val, bitmap) in field_map {
                if bitmap.contains(bp32) {
                    for nbp32 in bitmap.iter() {
                        let nbp = self.bp_reverse.get(&nbp32).copied().unwrap_or(nbp32 as u64);
                        if nbp != bp {
                            neighbors.insert(nbp);
                        }
                    }
                }
            }
        }

        neighbors.into_iter().collect()
    }

    /// Get the RoaringBitmap for a field value.
    pub fn field_bitmap(&self, field: &str, value: &Value) -> Option<&RoaringBitmap> {
        self.field_index.get(field)?.get(value)
    }

    /// Get all distinct indexed values for a field.
    pub fn indexed_values(&self, field: &str) -> Vec<Value> {
        self.field_index
            .get(field)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Expose the raw field index maps for efficient iteration.
    pub fn field_index_maps(&self) -> &HashMap<String, HashMap<Value, RoaringBitmap>> {
        &self.field_index
    }

    /// Resolve a u32 bitmap key back to a BasePoint.
    pub fn resolve_bp(&self, bp32: u32) -> BasePoint {
        self.bp_reverse.get(&bp32).copied().unwrap_or(bp32 as u64)
    }

    /// Expire records with a `_ttl` field whose timestamp has passed.
    /// `now_epoch_ms` is the current time in epoch milliseconds.
    /// Returns the number of records removed.
    pub fn expire_ttl(&mut self, now_epoch_ms: i64) -> usize {
        // Collect base points of expired records
        let expired: Vec<BasePoint> = self.records()
            .filter_map(|record| {
                match record.get("_ttl") {
                    Some(Value::Timestamp(t)) if *t <= now_epoch_ms => {
                        let key: Record = self.schema.base_fields.iter()
                            .map(|f| (f.name.clone(), record.get(&f.name).cloned().unwrap_or(Value::Null)))
                            .collect();
                        Some(self.hash_config.hash(&key, &self.schema))
                    }
                    Some(Value::Integer(t)) if *t <= now_epoch_ms => {
                        let key: Record = self.schema.base_fields.iter()
                            .map(|f| (f.name.clone(), record.get(&f.name).cloned().unwrap_or(Value::Null)))
                            .collect();
                        Some(self.hash_config.hash(&key, &self.schema))
                    }
                    _ => None,
                }
            })
            .collect();

        let count = expired.len();
        for bp in expired {
            let bp32 = bp as u32;
            // Remove field index entries
            if let Some(record) = self.reconstruct(bp) {
                for idx_field in &self.schema.indexed_fields {
                    if let Some(val) = record.get(idx_field) {
                        if let Some(field_map) = self.field_index.get_mut(idx_field) {
                            if let Some(bitmap) = field_map.get_mut(val) {
                                bitmap.remove(bp32);
                            }
                        }
                    }
                }
            }
            self.remove_from_storage(bp);
            self.bp_reverse.remove(&bp32);
        }
        count
    }

    /// Upsert — insert if not exists, update if exists. Returns `(inserted: bool)`.
    /// `true` = new record inserted, `false` = existing record updated.
    pub fn upsert(&mut self, record: &Record) -> bool {
        let key: Record = self.schema.base_fields.iter()
            .map(|f| (f.name.clone(), record.get(&f.name).cloned().unwrap_or(Value::Null)))
            .collect();
        let bp = self.hash_config.hash(&key, &self.schema);
        if self.reconstruct(bp).is_some() {
            // Exists — update with all non-key fields as patches
            let patches: Record = record.iter()
                .filter(|(k, _)| !self.schema.base_fields.iter().any(|f| &f.name == *k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            self.update(&key, &patches);
            false
        } else {
            self.insert(record);
            true
        }
    }

    /// Bulk delete — remove all records matching conditions.
    /// Returns number of records deleted.
    pub fn bulk_delete(&mut self, conditions: &[QueryCondition]) -> usize {
        let matching_keys: Vec<Record> = self.records()
            .filter(|record| conditions.iter().all(|c| c.matches(record)))
            .map(|record| {
                self.schema.base_fields.iter()
                    .map(|f| (f.name.clone(), record.get(&f.name).cloned().unwrap_or(Value::Null)))
                    .collect()
            })
            .collect();

        let mut count = 0;
        for key in &matching_keys {
            if self.delete(key) {
                count += 1;
            }
        }
        count
    }

    /// Truncate — remove all records but keep schema. Returns records removed.
    pub fn truncate(&mut self) -> usize {
        let count = self.len();
        self.storage = BaseStorage::Hashed {
            sections: HashMap::new(),
            base_values: HashMap::new(),
        };
        self.field_index.clear();
        self.field_stats.clear();
        self.curvature_stats = CurvatureStats::default();
        self.bp_reverse.clear();
        self.bp_to_idx.clear();
        self.seq_bp_list.clear();
        count
    }

    /// Count records matching conditions without returning them.
    pub fn count_where(&self, conditions: &[QueryCondition]) -> usize {
        self.count_where_ex(conditions, None)
    }

    /// Extended count with OR condition support.
    pub fn count_where_ex(&self, conditions: &[QueryCondition], or_conditions: Option<&[Vec<QueryCondition>]>) -> usize {
        self.records()
            .filter(|record| matches_filter(record, conditions, or_conditions))
            .count()
    }

    /// Check if any record matches the conditions.
    pub fn exists(&self, conditions: &[QueryCondition]) -> bool {
        self.exists_ex(conditions, None)
    }

    /// Extended exists with OR condition support.
    pub fn exists_ex(&self, conditions: &[QueryCondition], or_conditions: Option<&[Vec<QueryCondition>]>) -> bool {
        self.records()
            .any(|record| matches_filter(&record, conditions, or_conditions))
    }

    /// Return distinct values for a field across all records.
    pub fn distinct(&self, field: &str) -> Vec<Value> {
        // Fast path: if the field is indexed, read from field_index
        if let Some(field_map) = self.field_index.get(field) {
            return field_map.keys()
                .filter(|v| !matches!(v, Value::Null))
                .cloned()
                .collect();
        }
        // Slow path: full scan
        let mut seen = Vec::new();
        for record in self.records() {
            if let Some(v) = record.get(field) {
                if !matches!(v, Value::Null) && !seen.contains(v) {
                    seen.push(v.clone());
                }
            }
        }
        seen
    }

    /// Filtered query with field projection — returns only specified fields.
    pub fn filtered_query_projected(
        &self,
        conditions: &[QueryCondition],
        sort_by: Option<&str>,
        sort_desc: bool,
        limit: Option<usize>,
        offset: Option<usize>,
        fields: Option<&[&str]>,
    ) -> (Vec<Record>, usize) {
        let sort_fields = sort_by.map(|f| vec![(f, sort_desc)]);
        self.filtered_query_projected_ex(conditions, None, sort_fields.as_deref(), limit, offset, fields)
    }

    /// Extended filtered query with OR conditions and multi-field sort.
    pub fn filtered_query_projected_ex(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_fields: Option<&[(&str, bool)]>,
        limit: Option<usize>,
        offset: Option<usize>,
        fields: Option<&[&str]>,
    ) -> (Vec<Record>, usize) {
        let all_matching: Vec<Record> = self.records()
            .filter(|record| matches_filter(record, conditions, or_conditions))
            .collect();
        let total = all_matching.len();

        let mut results = all_matching;

        // Multi-field sort
        if let Some(sort_fields) = sort_fields {
            results.sort_by(|a, b| {
                for &(field, desc) in sort_fields {
                    let va = a.get(field).unwrap_or(&Value::Null);
                    let vb = b.get(field).unwrap_or(&Value::Null);
                    let cmp = if desc { vb.cmp(va) } else { va.cmp(vb) };
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                }
                std::cmp::Ordering::Equal
            });
        }

        // Offset + Limit
        let start = offset.unwrap_or(0);
        if start > 0 {
            results = results.into_iter().skip(start).collect();
        }
        if let Some(lim) = limit {
            results.truncate(lim);
        }

        // Project fields
        if let Some(fields) = fields {
            results = results.into_iter().map(|record| {
                let mut proj = Record::new();
                for &f in fields {
                    if let Some(v) = record.get(f) {
                        proj.insert(f.to_string(), v.clone());
                    }
                }
                proj
            }).collect();
        }

        (results, total)
    }

    // ── Vector Similarity Search ───────────────────────────────

    /// Exact k-Nearest Neighbors search in a vector field.
    ///
    /// Geometric interpretation: the `field` is a fiber of a vector bundle
    /// V = B × ℝᵈ. The query vector q ∈ ℝᵈ defines a point in the fiber,
    /// and kNN finds the d nearest sections in the product metric on V.
    ///
    /// Complexity: O(N · d) — brute-force exact search.
    /// For N < 100K this is perfectly fast. For larger datasets, an HNSW
    /// index can be layered on top without changing this interface.
    ///
    /// Args:
    ///   field:     Name of the vector fiber field.
    ///   query:     Query vector (must match stored dimensions).
    ///   top_k:     Number of results to return.
    ///   metric:    Distance/similarity metric (see `VectorMetric`).
    ///   pre_filter: Optional AND conditions applied before scoring.
    ///
    /// Returns: Vec of (score, record), sorted by score (descending for
    /// similarity metrics, ascending for distance metrics).
    pub fn vector_search(
        &self,
        field: &str,
        query: &[f64],
        top_k: usize,
        metric: VectorMetric,
        pre_filter: &[QueryCondition],
    ) -> Vec<(f64, Record)> {
        if top_k == 0 || query.is_empty() {
            return Vec::new();
        }

        // Brute-force scan
        let mut candidates: Vec<(f64, Record)> = self
            .records()
            .filter(|rec| pre_filter.iter().all(|c| c.matches(rec)))
            .filter_map(|rec| {
                let vec = match rec.get(field)? {
                    Value::Vector(v) => v,
                    _ => return None,
                };
                if vec.len() != query.len() {
                    return None;
                }
                let score = metric.score(query, vec);
                Some((score, rec))
            })
            .collect();

        // Sort: higher score = better for similarity, lower = better for distance.
        // We unify: always higher-first so callers can slice top_k uniformly.
        // Distance metrics return negative distances → higher is closer.
        candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
        candidates.truncate(top_k);
        candidates
    }

    // ── Sprint 2: New Methods ──────────────────────────────────

    /// Get next auto-increment ID for auto-generated keys.
    pub fn next_auto_id(&mut self) -> i64 {
        self.auto_id_counter += 1;
        self.auto_id_counter as i64
    }

    /// Atomic increment/decrement — `SET field = field + amount` without race.
    /// Returns true if the record existed and was updated.
    pub fn increment(&mut self, key: &Record, field: &str, amount: f64) -> bool {
        let bp = self.hash_config.hash(key, &self.schema);
        let existing = match self.reconstruct(bp) {
            Some(r) => r,
            None => return false,
        };

        let mut patches = Record::new();
        match existing.get(field) {
            Some(Value::Integer(i)) if amount == (amount as i64) as f64 => {
                patches.insert(field.to_string(), Value::Integer(i + amount as i64));
            }
            Some(Value::Float(f_val)) => {
                patches.insert(field.to_string(), Value::Float(f_val + amount));
            }
            Some(Value::Integer(i)) => {
                patches.insert(field.to_string(), Value::Float(*i as f64 + amount));
            }
            _ => {
                patches.insert(field.to_string(), Value::Float(amount));
            }
        }

        self.update(key, &patches)
    }

    /// Add a fiber field to the schema and extend all existing records with the default value.
    pub fn add_field(&mut self, field_def: FieldDef) {
        self.schema.fiber_fields.push(field_def.clone());

        // Extend all existing fiber vectors with the new default
        match &mut self.storage {
            BaseStorage::Hashed { sections, .. } => {
                for fiber in sections.values_mut() {
                    fiber.push(field_def.default.clone());
                }
            }
            BaseStorage::Sequential { sections, .. } => {
                for fiber in sections.iter_mut() {
                    fiber.push(field_def.default.clone());
                }
            }
            BaseStorage::Hybrid { sections, overflow_sections, .. } => {
                for fiber in sections.iter_mut() {
                    fiber.push(field_def.default.clone());
                }
                for fiber in overflow_sections.values_mut() {
                    fiber.push(field_def.default.clone());
                }
            }
        }
    }

    /// Remove a fiber field from the schema and all existing records.
    /// Returns `true` if the field was found and removed, `false` if it does not exist.
    /// Base fields (keys) cannot be dropped — only fiber fields.
    pub fn drop_field(&mut self, field_name: &str) -> bool {
        let pos = self.schema.fiber_fields.iter().position(|f| f.name == field_name);
        let pos = match pos {
            Some(p) => p,
            None => return false,
        };

        // Remove from schema and indexes
        self.schema.fiber_fields.remove(pos);
        self.schema.indexed_fields.retain(|f| f != field_name);
        self.field_index.remove(field_name);

        // Remove the value at position `pos` from every fiber vector
        match &mut self.storage {
            BaseStorage::Hashed { sections, .. } => {
                for fiber in sections.values_mut() {
                    if pos < fiber.len() { fiber.remove(pos); }
                }
            }
            BaseStorage::Sequential { sections, .. } => {
                for fiber in sections.iter_mut() {
                    if pos < fiber.len() { fiber.remove(pos); }
                }
            }
            BaseStorage::Hybrid { sections, overflow_sections, .. } => {
                for fiber in sections.iter_mut() {
                    if pos < fiber.len() { fiber.remove(pos); }
                }
                for fiber in overflow_sections.values_mut() {
                    if pos < fiber.len() { fiber.remove(pos); }
                }
            }
        }
        true
    }

    /// Add an index on a field and build it from existing records.
    pub fn add_index(&mut self, field_name: &str) {
        if self.schema.indexed_fields.contains(&field_name.to_string()) {
            return;
        }
        self.schema.indexed_fields.push(field_name.to_string());

        // Build index from existing records
        let mut new_index: HashMap<Value, RoaringBitmap> = HashMap::new();
        for record in self.records() {
            if let Some(val) = record.get(field_name) {
                if matches!(val, Value::Null) { continue; }
                let key: Record = self.schema.base_fields.iter()
                    .map(|f| (f.name.clone(), record.get(&f.name).cloned().unwrap_or(Value::Null)))
                    .collect();
                let bp = self.hash_config.hash(&key, &self.schema);
                let bp32 = bp as u32;
                new_index.entry(val.clone()).or_default().insert(bp32);
            }
        }

        self.field_index.insert(field_name.to_string(), new_index);
    }

    // ── Sprint 3: Engine Methods ──────────────────────────────

    /// Update with optimistic concurrency — only succeeds if record's _version
    /// matches expected_version. Bumps _version on success.
    /// Returns Ok(new_version) on success, Err("version_conflict"|"not_found").
    pub fn update_versioned(
        &mut self,
        key: &Record,
        patches: &Record,
        expected_version: i64,
    ) -> Result<i64, &'static str> {
        // Ensure _version field exists in schema
        if !self.schema.fiber_fields.iter().any(|f| f.name == "_version") {
            self.add_field(FieldDef::numeric("_version").with_default(Value::Integer(0)));
        }

        let bp = self.hash_config.hash(key, &self.schema);
        let existing = match self.reconstruct(bp) {
            Some(r) => r,
            None => return Err("not_found"),
        };

        let current_version = match existing.get("_version") {
            Some(Value::Integer(v)) => *v,
            _ => 0,
        };

        if current_version != expected_version {
            return Err("version_conflict");
        }

        let new_version = current_version + 1;
        let mut full_patches = patches.clone();
        full_patches.insert("_version".to_string(), Value::Integer(new_version));
        self.update(key, &full_patches);
        Ok(new_version)
    }

    /// Update with RETURNING — same as update but returns the patched record.
    pub fn update_returning(&mut self, key: &Record, patches: &Record) -> Option<Record> {
        if !self.update(key, patches) {
            return None;
        }
        let bp = self.hash_config.hash(key, &self.schema);
        self.reconstruct(bp)
    }

    /// Delete with RETURNING — returns the record that was deleted.
    pub fn delete_returning(&mut self, key: &Record) -> Option<Record> {
        let bp = self.hash_config.hash(key, &self.schema);
        let record = self.reconstruct(bp)?;
        if self.delete(key) {
            Some(record)
        } else {
            None
        }
    }

    /// Bundle stats — field cardinalities, storage info, index sizes.
    pub fn stats(&self) -> BundleStats {
        let mut index_sizes: Vec<(String, usize)> = Vec::new();
        for (field, field_map) in &self.field_index {
            let total_bits: usize = field_map.values().map(|bm| bm.len() as usize).sum();
            index_sizes.push((field.clone(), total_bits));
        }

        let field_cardinalities: Vec<(String, usize)> = self.schema.fiber_fields.iter()
            .map(|f| {
                let card = self.distinct(&f.name).len();
                (f.name.clone(), card)
            })
            .collect();

        BundleStats {
            name: self.schema.name.clone(),
            record_count: self.len(),
            base_fields: self.schema.base_fields.len(),
            fiber_fields: self.schema.fiber_fields.len(),
            indexed_fields: self.schema.indexed_fields.clone(),
            storage_mode: self.storage_mode().to_string(),
            index_sizes,
            field_cardinalities,
        }
    }

    /// Explain a query — describe what the engine will do without running it.
    pub fn explain(
        &self,
        conditions: &[QueryCondition],
        or_conditions: Option<&[Vec<QueryCondition>]>,
        sort_fields: Option<&[(&str, bool)]>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> QueryPlan {
        let total_records = self.len();

        // Check which conditions can use an index
        let mut index_scans: Vec<String> = Vec::new();
        let mut full_scan_conditions: Vec<String> = Vec::new();

        for cond in conditions {
            let field_name = match cond {
                QueryCondition::Eq(f, _) => f,
                QueryCondition::In(f, _) => f,
                _ => {
                    full_scan_conditions.push(format!("{:?}", cond));
                    continue;
                }
            };
            if self.field_index.contains_key(field_name) {
                index_scans.push(field_name.clone());
            } else {
                full_scan_conditions.push(format!("{:?}", cond));
            }
        }

        let scan_type = if index_scans.is_empty() {
            "full_scan".to_string()
        } else if full_scan_conditions.is_empty() && or_conditions.map_or(true, |g| g.is_empty()) {
            "index_scan".to_string()
        } else {
            "index_scan + filter".to_string()
        };

        let has_sort = sort_fields.is_some();
        let has_limit = limit.is_some();
        let has_offset = offset.is_some();
        let or_group_count = or_conditions.map_or(0, |g| g.len());

        QueryPlan {
            scan_type,
            total_records,
            index_scans,
            full_scan_conditions,
            or_group_count,
            has_sort,
            has_limit,
            has_offset,
            storage_mode: self.storage_mode().to_string(),
        }
    }

    /// Execute a batch of operations atomically (all-or-nothing).
    /// Returns (successes, results_per_op). If any op fails, all are rolled back.
    pub fn execute_transaction(&mut self, ops: &[TransactionOp]) -> Result<Vec<TransactionResult>, String> {
        // Snapshot: collect all records for rollback
        let snapshot: Vec<(Record, Record)> = self.records().map(|rec| {
            let key: Record = self.schema.base_fields.iter()
                .map(|f| (f.name.clone(), rec.get(&f.name).cloned().unwrap_or(Value::Null)))
                .collect();
            (key, rec)
        }).collect();
        let _snapshot_len = self.len();

        let mut results = Vec::with_capacity(ops.len());

        for (i, op) in ops.iter().enumerate() {
            let result = match op {
                TransactionOp::Insert(record) => {
                    self.insert(record);
                    TransactionResult::Ok
                }
                TransactionOp::Update { key, patches } => {
                    if self.update(key, patches) {
                        TransactionResult::Ok
                    } else {
                        TransactionResult::Error(format!("op[{}]: record not found", i))
                    }
                }
                TransactionOp::Delete(key) => {
                    if self.delete(key) {
                        TransactionResult::Ok
                    } else {
                        TransactionResult::Error(format!("op[{}]: record not found", i))
                    }
                }
                TransactionOp::Increment { key, field, amount } => {
                    if self.increment(key, field, *amount) {
                        TransactionResult::Ok
                    } else {
                        TransactionResult::Error(format!("op[{}]: record not found", i))
                    }
                }
            };

            if let TransactionResult::Error(ref msg) = result {
                // Rollback: restore snapshot
                self.truncate();
                // Re-initialize storage for snapshot
                for (_key, rec) in &snapshot {
                    self.insert(rec);
                }
                return Err(msg.clone());
            }
            results.push(result);
        }

        Ok(results)
    }
}

/// Bundle statistics for the stats endpoint.
#[derive(Debug, Clone)]
pub struct BundleStats {
    pub name: String,
    pub record_count: usize,
    pub base_fields: usize,
    pub fiber_fields: usize,
    pub indexed_fields: Vec<String>,
    pub storage_mode: String,
    pub index_sizes: Vec<(String, usize)>,
    pub field_cardinalities: Vec<(String, usize)>,
}

/// One anomalous record with its geometric scores.
///
/// Returned by `BundleStore::compute_anomalies()`.
#[derive(Debug, Clone)]
pub struct AnomalyRecord {
    /// The full reconstructed record (base + fiber fields).
    pub record: Record,
    /// K_scalar(p) computed relative to current bundle field_stats.
    pub local_curvature: f64,
    /// z-score = (K - μ_K) / σ_K.
    pub z_score: f64,
    /// confidence(p) = 1 / (1 + K).
    pub confidence: f64,
    /// Number of fiber fields that deviate from schema defaults.
    pub deviation_norm: usize,
    /// Fiber metric distance from the zero section (√Σ normalised²).
    pub deviation_distance: f64,
    /// Fields whose normalised deviation is above the record's mean deviation.
    pub contributing_fields: Vec<String>,
    /// Number of geometric neighbours (records sharing an indexed field value).
    pub neighbourhood_size: usize,
}



/// Query execution plan for the EXPLAIN endpoint.
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub scan_type: String,
    pub total_records: usize,
    pub index_scans: Vec<String>,
    pub full_scan_conditions: Vec<String>,
    pub or_group_count: usize,
    pub has_sort: bool,
    pub has_limit: bool,
    pub has_offset: bool,
    pub storage_mode: String,
}

/// A single transactional operation.
#[derive(Debug, Clone)]
pub enum TransactionOp {
    Insert(Record),
    Update { key: Record, patches: Record },
    Delete(Record),
    Increment { key: Record, field: String, amount: f64 },
}

/// Result of a single transaction operation.
#[derive(Debug, Clone)]
pub enum TransactionResult {
    Ok,
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FieldDef;

    fn make_store() -> BundleStore {
        let schema = BundleSchema::new("users")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"))
            .fiber(FieldDef::numeric("salary").with_range(100000.0))
            .fiber(FieldDef::categorical("dept"))
            .index("dept");
        BundleStore::new(schema)
    }

    fn rec(id: i64, name: &str, salary: f64, dept: &str) -> Record {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(id));
        r.insert("name".into(), Value::Text(name.into()));
        r.insert("salary".into(), Value::Float(salary));
        r.insert("dept".into(), Value::Text(dept.into()));
        r
    }

    /// TDD-1.1: Section insert/retrieve.
    #[test]
    fn tdd_1_1_insert_retrieve() {
        let mut store = make_store();
        let r = rec(1, "Alice", 75000.0, "Eng");
        store.insert(&r);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let result = store.point_query(&key).unwrap();
        assert_eq!(result.get("name"), Some(&Value::Text("Alice".into())));
        assert_eq!(result.get("salary"), Some(&Value::Float(75000.0)));
        assert_eq!(result.get("dept"), Some(&Value::Text("Eng".into())));
    }

    /// TDD-1.10: Insert then query returns exact record.
    #[test]
    fn tdd_1_10_insert_then_query() {
        let mut store = make_store();
        let r = rec(42, "Bob", 90000.0, "Sales");
        store.insert(&r);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(42));
        let result = store.point_query(&key).unwrap();
        assert_eq!(result, r);
    }

    /// TDD-1.11: Miss query returns None.
    #[test]
    fn tdd_1_11_miss_query() {
        let store = make_store();
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(999));
        assert!(store.point_query(&key).is_none());
    }

    /// GAP-B.1: Same key insert overwrites.
    #[test]
    fn gap_b1_overwrite() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 50000.0, "Eng"));
        store.insert(&rec(1, "Alice_v2", 99000.0, "Sales"));

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let result = store.point_query(&key).unwrap();
        assert_eq!(result.get("name"), Some(&Value::Text("Alice_v2".into())));
        assert_eq!(result.get("salary"), Some(&Value::Float(99000.0)));
    }

    /// GAP-B.2: Only one section at overwritten base point.
    #[test]
    fn gap_b2_single_section() {
        let mut store = make_store();
        store.insert(&rec(1, "First", 10.0, "A"));
        store.insert(&rec(1, "Second", 20.0, "B"));
        assert_eq!(store.len(), 1);
    }

    /// TDD-1.2: Zero deviation for default record.
    #[test]
    fn tdd_1_2_zero_deviation() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_default(Value::Float(0.0)))
            .fiber(FieldDef::categorical("cat").with_default(Value::Text("X".into())));
        let mut store = BundleStore::new(schema);
        let r = rec_simple(1, 0.0, "X");
        store.insert(&r);

        let bp = store.base_point(&r);
        assert_eq!(store.deviation_norm(bp), 0);
    }

    /// TDD-1.3: Deviation norm = 2 for 2-field deviant.
    #[test]
    fn tdd_1_3_deviation_norm() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_default(Value::Float(0.0)))
            .fiber(FieldDef::categorical("cat").with_default(Value::Text("X".into())));
        let mut store = BundleStore::new(schema);
        let r = rec_simple(1, 999.0, "Y"); // both deviate
        store.insert(&r);

        let bp = store.base_point(&r);
        assert_eq!(store.deviation_norm(bp), 2);
    }

    fn rec_simple(id: i64, val: f64, cat: &str) -> Record {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(id));
        r.insert("val".into(), Value::Float(val));
        r.insert("cat".into(), Value::Text(cat.into()));
        r
    }

    /// TDD-2.1: Restriction — F(narrow) ⊆ F(wide).
    #[test]
    fn tdd_2_1_restriction() {
        let mut store = make_store();
        for i in 0..100 {
            let dept = ["Eng", "Sales", "HR"][i % 3];
            store.insert(&rec(i as i64, &format!("U{i}"), 50000.0, dept));
        }
        let wide = store.range_query("dept", &[
            Value::Text("Eng".into()),
            Value::Text("Sales".into()),
        ]);
        let narrow = store.range_query("dept", &[Value::Text("Eng".into())]);

        // Every record in narrow must appear in wide
        for r in &narrow {
            assert!(wide.contains(r));
        }
    }

    /// TDD-2.4: Gluing — F(A) ∪ F(B) = F(A∪B).
    #[test]
    fn tdd_2_4_gluing() {
        let mut store = make_store();
        for i in 0..50 {
            let dept = ["Eng", "Sales"][i % 2];
            store.insert(&rec(i as i64, &format!("U{i}"), 50000.0, dept));
        }
        let fa = store.range_query("dept", &[Value::Text("Eng".into())]);
        let fb = store.range_query("dept", &[Value::Text("Sales".into())]);
        let fab = store.range_query("dept", &[
            Value::Text("Eng".into()),
            Value::Text("Sales".into()),
        ]);

        let mut union: Vec<Record> = fa.into_iter().chain(fb).collect();
        union.sort_by_key(|r| match r.get("id") {
            Some(Value::Integer(i)) => *i,
            _ => 0,
        });
        let mut fab_sorted = fab;
        fab_sorted.sort_by_key(|r| match r.get("id") {
            Some(Value::Integer(i)) => *i,
            _ => 0,
        });
        assert_eq!(union, fab_sorted);
    }

    // ── Flat-base (K=0) storage tests ──────────────────────

    /// Auto-detect arithmetic keys and switch to Sequential storage.
    #[test]
    fn flat_base_auto_detect_sequential() {
        let schema = BundleSchema::new("timeseries")
            .base(FieldDef::numeric("ts"))
            .fiber(FieldDef::numeric("val").with_range(1000.0))
            .index("ts");
        let mut store = BundleStore::new(schema);

        // Insert 50 sequential timestamps (step=60)
        for i in 0..50 {
            let mut r = Record::new();
            r.insert("ts".into(), Value::Integer(1710000000 + i * 60));
            r.insert("val".into(), Value::Float(22.0 + i as f64 * 0.1));
            store.insert(&r);
        }

        assert_eq!(store.storage_mode(), "sequential");
        assert_eq!(store.len(), 50);
    }

    /// Sequential storage: point queries work via bp_to_idx.
    #[test]
    fn flat_base_point_query() {
        let schema = BundleSchema::new("events")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("score").with_range(100.0));
        let mut store = BundleStore::new(schema);

        for i in 0..100 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("score".into(), Value::Float(i as f64 * 1.5));
            store.insert(&r);
        }

        assert_eq!(store.storage_mode(), "sequential");

        // Point query record 42
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(42));
        let result = store.point_query(&key).unwrap();
        assert_eq!(result.get("score"), Some(&Value::Float(63.0)));

        // Miss query
        let mut miss = Record::new();
        miss.insert("id".into(), Value::Integer(999));
        assert!(store.point_query(&miss).is_none());
    }

    /// Sequential storage handles overwrites in-place.
    #[test]
    fn flat_base_overwrite() {
        let schema = BundleSchema::new("data")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(1000.0));
        let mut store = BundleStore::new(schema);

        for i in 0..50 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("val".into(), Value::Float(i as f64));
            store.insert(&r);
        }

        assert_eq!(store.storage_mode(), "sequential");
        assert_eq!(store.len(), 50);

        // Overwrite record 10
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(10));
        r.insert("val".into(), Value::Float(999.0));
        store.insert(&r);

        // len should still be 50 (overwrite, not append)
        assert_eq!(store.len(), 50);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(10));
        let result = store.point_query(&key).unwrap();
        assert_eq!(result.get("val"), Some(&Value::Float(999.0)));
    }

    /// with_geometry: explicit Sequential mode from the start.
    #[test]
    fn flat_base_explicit_geometry() {
        let schema = BundleSchema::new("sensors")
            .base(FieldDef::numeric("ts"))
            .fiber(FieldDef::numeric("temp").with_range(100.0));
        let mut store = BundleStore::with_geometry(
            schema,
            BaseGeometry::Flat { start: 1000, step: 10, key_field: "ts".into() },
        );

        assert_eq!(store.storage_mode(), "sequential");

        for i in 0..20 {
            let mut r = Record::new();
            r.insert("ts".into(), Value::Integer(1000 + i * 10));
            r.insert("temp".into(), Value::Float(20.0 + i as f64));
            store.insert(&r);
        }

        assert_eq!(store.len(), 20);

        let mut key = Record::new();
        key.insert("ts".into(), Value::Integer(1050));
        let result = store.point_query(&key).unwrap();
        assert_eq!(result.get("temp"), Some(&Value::Float(25.0)));
    }

    /// Non-arithmetic keys stay Hashed.
    #[test]
    fn curved_base_stays_hashed() {
        let mut store = make_store();

        // Random IDs — not arithmetic
        for i in [3, 7, 11, 19, 23, 29, 31, 37, 41, 43, 47, 53,
                  59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103,
                  107, 109, 113, 127, 131, 137, 139, 149, 151] {
            store.insert(&rec(i, &format!("U{i}"), 50000.0, "Eng"));
        }

        assert_eq!(store.storage_mode(), "hashed");
        assert_eq!(store.len(), 32);
    }

    /// Sequential mode: sections() iterator works.
    #[test]
    fn flat_base_sections_iter() {
        let schema = BundleSchema::new("seq")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(100.0));
        let mut store = BundleStore::new(schema);

        for i in 0..40 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("x".into(), Value::Float(i as f64));
            store.insert(&r);
        }

        assert_eq!(store.storage_mode(), "sequential");
        let count = store.sections().count();
        assert_eq!(count, 40);
    }

    /// Sequential mode: records() iterator works.
    #[test]
    fn flat_base_records_iter() {
        let schema = BundleSchema::new("seq2")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(100.0));
        let mut store = BundleStore::new(schema);

        for i in 0..40 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("x".into(), Value::Float(i as f64));
            store.insert(&r);
        }

        assert_eq!(store.storage_mode(), "sequential");
        let records: Vec<_> = store.records().collect();
        assert_eq!(records.len(), 40);
    }

    /// detect_base_geometry function directly.
    #[test]
    fn detect_geometry_arithmetic() {
        let schema = BundleSchema::new("ts")
            .base(FieldDef::numeric("t"));
        let records: Vec<Record> = (0..10).map(|i| {
            let mut r = Record::new();
            r.insert("t".into(), Value::Integer(100 + i * 5));
            r
        }).collect();

        match detect_base_geometry(&schema, &records) {
            BaseGeometry::Flat { start, step, .. } => {
                assert_eq!(start, 100);
                assert_eq!(step, 5);
            }
            other => panic!("Expected Flat, got {:?}", other),
        }
    }

    /// detect_base_geometry with non-arithmetic keys.
    #[test]
    fn detect_geometry_curved() {
        let schema = BundleSchema::new("random")
            .base(FieldDef::numeric("id"));
        let records: Vec<Record> = [3, 7, 15, 22, 30, 41, 55, 70, 88, 99].iter().map(|&i| {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r
        }).collect();

        assert!(matches!(detect_base_geometry(&schema, &records), BaseGeometry::Curved));
    }

    // ── Batch Insert Tests ──

    #[test]
    fn batch_insert_basic() {
        let schema = BundleSchema::new("ts")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0));

        let mut store = BundleStore::new(schema);
        let records: Vec<Record> = (0..100).map(|i| {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("val".into(), Value::Float(i as f64 * 1.5));
            r
        }).collect();

        let count = store.batch_insert(&records);
        assert_eq!(count, 100);
        assert_eq!(store.len(), 100);

        // Verify point queries work
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(50));
        let result = store.point_query(&key).unwrap();
        assert_eq!(result.get("val"), Some(&Value::Float(75.0)));
    }

    #[test]
    fn batch_insert_triggers_auto_detect() {
        let schema = BundleSchema::new("ts")
            .base(FieldDef::numeric("ts"))
            .fiber(FieldDef::numeric("cpu").with_range(100.0));

        let mut store = BundleStore::new(schema);
        assert_eq!(store.storage_mode(), "hashed");

        // Batch of 50 arithmetic records (step=10) → should auto-detect flat
        let records: Vec<Record> = (0..50).map(|i| {
            let mut r = Record::new();
            r.insert("ts".into(), Value::Integer(i * 10));
            r.insert("cpu".into(), Value::Float(50.0));
            r
        }).collect();

        let count = store.batch_insert(&records);
        assert_eq!(count, 50);
        assert_eq!(store.storage_mode(), "sequential");
    }

    #[test]
    fn batch_insert_matches_single_insert() {
        let schema = BundleSchema::new("cmp")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"))
            .fiber(FieldDef::numeric("score").with_range(100.0))
            .index("name");

        let records: Vec<Record> = (0..200).map(|i| {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("user_{}", i % 5)));
            r.insert("score".into(), Value::Float((i % 100) as f64));
            r
        }).collect();

        // Single insert
        let mut store_single = BundleStore::new(schema.clone());
        for r in &records {
            store_single.insert(r);
        }

        // Batch insert
        let mut store_batch = BundleStore::new(schema);
        store_batch.batch_insert(&records);

        // Same count
        assert_eq!(store_single.len(), store_batch.len());

        // Same point query results
        for i in [0, 50, 100, 150, 199] {
            let mut key = Record::new();
            key.insert("id".into(), Value::Integer(i));
            let s = store_single.point_query(&key);
            let b = store_batch.point_query(&key);
            assert_eq!(s.is_some(), b.is_some(), "Mismatch at id={i}");
            if let (Some(sr), Some(br)) = (s, b) {
                assert_eq!(sr.get("score"), br.get("score"), "Score mismatch at id={i}");
            }
        }

        // Same range query results
        let sr = store_single.range_query("name", &[Value::Text("user_0".into())]);
        let br = store_batch.range_query("name", &[Value::Text("user_0".into())]);
        assert_eq!(sr.len(), br.len());
    }

    #[test]
    fn batch_insert_empty() {
        let schema = BundleSchema::new("empty")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(10.0));
        let mut store = BundleStore::new(schema);
        let count = store.batch_insert(&[]);
        assert_eq!(count, 0);
        assert_eq!(store.len(), 0);
    }

    // ── Update Tests ──

    #[test]
    fn update_existing_record() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let mut patches = Record::new();
        patches.insert("salary".into(), Value::Float(95000.0));
        patches.insert("dept".into(), Value::Text("Management".into()));

        assert!(store.update(&key, &patches));
        assert_eq!(store.len(), 1);

        let result = store.point_query(&key).unwrap();
        assert_eq!(result.get("name"), Some(&Value::Text("Alice".into())));
        assert_eq!(result.get("salary"), Some(&Value::Float(95000.0)));
        assert_eq!(result.get("dept"), Some(&Value::Text("Management".into())));
    }

    #[test]
    fn update_nonexistent_returns_false() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(999));
        let mut patches = Record::new();
        patches.insert("salary".into(), Value::Float(1.0));

        assert!(!store.update(&key, &patches));
    }

    // ── Delete Tests ──

    #[test]
    fn delete_existing_record() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));
        store.insert(&rec(2, "Bob", 80000.0, "Sales"));
        assert_eq!(store.len(), 2);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        assert!(store.delete(&key));

        // Record is gone
        assert!(store.point_query(&key).is_none());
        // Other record still there
        let mut key2 = Record::new();
        key2.insert("id".into(), Value::Integer(2));
        assert!(store.point_query(&key2).is_some());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(999));
        assert!(!store.delete(&key));
        assert_eq!(store.len(), 1);
    }

    // ── Filtered Query Tests ──

    #[test]
    fn filtered_query_eq() {
        let mut store = make_store();
        for i in 0..10 {
            let dept = ["Eng", "Sales"][i % 2];
            store.insert(&rec(i as i64, &format!("U{i}"), 50000.0 + i as f64 * 1000.0, dept));
        }

        let conds = vec![QueryCondition::Eq("dept".into(), Value::Text("Eng".into()))];
        let results = store.filtered_query(&conds, None, false, None, None);
        assert_eq!(results.len(), 5);
        for r in &results {
            assert_eq!(r.get("dept"), Some(&Value::Text("Eng".into())));
        }
    }

    #[test]
    fn filtered_query_gt_sorted_limited() {
        let mut store = make_store();
        for i in 0..20 {
            store.insert(&rec(i, &format!("U{i}"), 50000.0 + i as f64 * 1000.0, "Eng"));
        }

        let conds = vec![QueryCondition::Gt("salary".into(), Value::Float(60000.0))];
        let results = store.filtered_query(&conds, Some("salary"), true, Some(5), None);
        assert_eq!(results.len(), 5);
        // Should be sorted descending by salary
        for w in results.windows(2) {
            assert!(w[0].get("salary").unwrap() >= w[1].get("salary").unwrap());
        }
    }

    #[test]
    fn filtered_query_contains() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice Smith", 75000.0, "Eng"));
        store.insert(&rec(2, "Bob Jones", 80000.0, "Sales"));
        store.insert(&rec(3, "Charlie Smith", 60000.0, "HR"));

        let conds = vec![QueryCondition::Contains("name".into(), "smith".into())];
        let results = store.filtered_query(&conds, None, false, None, None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn filtered_query_offset() {
        let mut store = make_store();
        for i in 0..10 {
            store.insert(&rec(i, &format!("U{i}"), 50000.0, "Eng"));
        }

        let results = store.filtered_query(&[], Some("id"), false, Some(3), Some(5));
        assert_eq!(results.len(), 3);
    }

    // ── TTL Tests ──

    #[test]
    fn ttl_expire_removes_records() {
        let schema = BundleSchema::new("sessions")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("user"))
            .fiber(FieldDef::numeric("_ttl").with_range(1e15));
        let mut store = BundleStore::new(schema);

        let mut r1 = Record::new();
        r1.insert("id".into(), Value::Integer(1));
        r1.insert("user".into(), Value::Text("alice".into()));
        r1.insert("_ttl".into(), Value::Timestamp(1000)); // expired
        store.insert(&r1);

        let mut r2 = Record::new();
        r2.insert("id".into(), Value::Integer(2));
        r2.insert("user".into(), Value::Text("bob".into()));
        r2.insert("_ttl".into(), Value::Timestamp(999999999)); // not expired
        store.insert(&r2);

        assert_eq!(store.len(), 2);
        let expired = store.expire_ttl(5000);
        assert_eq!(expired, 1);

        let mut key1 = Record::new();
        key1.insert("id".into(), Value::Integer(1));
        assert!(store.point_query(&key1).is_none());

        let mut key2 = Record::new();
        key2.insert("id".into(), Value::Integer(2));
        assert!(store.point_query(&key2).is_some());
    }

    // ── Timestamp Value Tests ──

    #[test]
    fn timestamp_value_ordering() {
        let t1 = Value::Timestamp(1000);
        let t2 = Value::Timestamp(2000);
        assert!(t1 < t2);
        assert_eq!(t1.as_timestamp(), Some(1000));
        assert_eq!(t1.as_f64(), Some(1000.0));
    }

    // ── Bulk Update Tests ──

    #[test]
    fn bulk_update_matching_records() {
        let schema = BundleSchema::new("notifs")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("module"))
            .fiber(FieldDef::categorical("read"))
            .index("module");
        let mut store = BundleStore::new(schema);

        for i in 1..=5 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("module".into(), Value::Text("system".into()));
            r.insert("read".into(), Value::Bool(false));
            store.insert(&r);
        }
        // Add one non-matching
        let mut r6 = Record::new();
        r6.insert("id".into(), Value::Integer(6));
        r6.insert("module".into(), Value::Text("alerts".into()));
        r6.insert("read".into(), Value::Bool(false));
        store.insert(&r6);

        // Bulk update: mark all system notifications as read
        let conditions = vec![QueryCondition::Eq("module".into(), Value::Text("system".into()))];
        let mut patches = Record::new();
        patches.insert("read".into(), Value::Bool(true));

        let updated = store.bulk_update(&conditions, &patches);
        assert_eq!(updated, 5);

        // Verify system ones are read=true
        let mut key1 = Record::new();
        key1.insert("id".into(), Value::Integer(1));
        let rec1 = store.point_query(&key1).unwrap();
        assert_eq!(rec1.get("read"), Some(&Value::Bool(true)));

        // Verify alerts one is still read=false
        let mut key6 = Record::new();
        key6.insert("id".into(), Value::Integer(6));
        let rec6 = store.point_query(&key6).unwrap();
        assert_eq!(rec6.get("read"), Some(&Value::Bool(false)));
    }

    #[test]
    fn bulk_update_no_matches() {
        let schema = BundleSchema::new("empty")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("status"));
        let mut store = BundleStore::new(schema);

        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("status".into(), Value::Text("open".into()));
        store.insert(&r);

        let conditions = vec![QueryCondition::Eq("status".into(), Value::Text("closed".into()))];
        let mut patches = Record::new();
        patches.insert("status".into(), Value::Text("archived".into()));

        let updated = store.bulk_update(&conditions, &patches);
        assert_eq!(updated, 0);
    }

    // ── Sprint 1: New Query Operators ──

    fn sprint1_store() -> BundleStore {
        let schema = BundleSchema::new("items")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"))
            .fiber(FieldDef::numeric("price"))
            .fiber(FieldDef::categorical("category"))
            .index("category");
        let mut store = BundleStore::new(schema);

        let data = vec![
            (1, "apple",  1.50, "fruit"),
            (2, "banana", 0.75, "fruit"),
            (3, "carrot", 2.00, "vegetable"),
            (4, "donut",  3.50, "pastry"),
        ];

        for (id, name, price, cat) in data {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(id));
            r.insert("name".into(), Value::Text(name.into()));
            r.insert("price".into(), Value::Float(price));
            r.insert("category".into(), Value::Text(cat.into()));
            store.insert(&r);
        }

        // Insert a record with a null category
        let mut r_null = Record::new();
        r_null.insert("id".into(), Value::Integer(5));
        r_null.insert("name".into(), Value::Text("mystery".into()));
        r_null.insert("price".into(), Value::Float(9.99));
        r_null.insert("category".into(), Value::Null);
        store.insert(&r_null);

        store
    }

    #[test]
    fn query_in_operator() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::In(
            "category".into(),
            vec![Value::Text("fruit".into()), Value::Text("pastry".into())],
        )];
        let results = store.filtered_query(&cond, None, false, None, None);
        assert_eq!(results.len(), 3); // apple, banana, donut
    }

    #[test]
    fn query_not_in_operator() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::NotIn(
            "category".into(),
            vec![Value::Text("fruit".into())],
        )];
        let results = store.filtered_query(&cond, None, false, None, None);
        // vegetable, pastry, null-category (NotIn means "value not in set" — null is not "fruit")
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn query_is_null() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::IsNull("category".into())];
        let results = store.filtered_query(&cond, None, false, None, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get("name"), Some(&Value::Text("mystery".into())));
    }

    #[test]
    fn query_is_not_null() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::IsNotNull("category".into())];
        let results = store.filtered_query(&cond, None, false, None, None);
        assert_eq!(results.len(), 4); // all except mystery
    }

    #[test]
    fn query_ends_with() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::EndsWith("name".into(), "ot".into())];
        let results = store.filtered_query(&cond, None, false, None, None);
        assert_eq!(results.len(), 1); // carrot
        assert_eq!(results[0].get("name"), Some(&Value::Text("carrot".into())));
    }

    #[test]
    fn query_regex() {
        let store = sprint1_store();
        // Match names starting with 'a' or 'b'
        let cond = vec![QueryCondition::Regex("name".into(), "^[ab]".into())];
        let results = store.filtered_query(&cond, None, false, None, None);
        assert_eq!(results.len(), 2); // apple, banana
    }

    #[test]
    fn query_regex_invalid_pattern_matches_nothing() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::Regex("name".into(), "[invalid".into())];
        let results = store.filtered_query(&cond, None, false, None, None);
        assert_eq!(results.len(), 0); // invalid regex → no matches
    }

    // ── Sprint 1: Upsert ──

    #[test]
    fn upsert_insert_new() {
        let mut store = sprint1_store();
        assert_eq!(store.len(), 5);
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(99));
        r.insert("name".into(), Value::Text("kiwi".into()));
        r.insert("price".into(), Value::Float(4.00));
        r.insert("category".into(), Value::Text("fruit".into()));

        let inserted = store.upsert(&r);
        assert!(inserted);
        assert_eq!(store.len(), 6);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(99));
        let rec = store.point_query(&key).unwrap();
        assert_eq!(rec.get("name"), Some(&Value::Text("kiwi".into())));
    }

    #[test]
    fn upsert_update_existing() {
        let mut store = sprint1_store();
        assert_eq!(store.len(), 5);

        // Upsert id=1 (apple) with new price
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("name".into(), Value::Text("apple".into()));
        r.insert("price".into(), Value::Float(2.99));
        r.insert("category".into(), Value::Text("fruit".into()));

        let inserted = store.upsert(&r);
        assert!(!inserted); // should be update, not insert
        assert_eq!(store.len(), 5); // count unchanged

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let rec = store.point_query(&key).unwrap();
        assert_eq!(rec.get("price"), Some(&Value::Float(2.99)));
    }

    // ── Sprint 1: Count Where ──

    #[test]
    fn count_where_basic() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::Eq("category".into(), Value::Text("fruit".into()))];
        assert_eq!(store.count_where(&cond), 2);
    }

    #[test]
    fn count_where_no_match() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::Eq("category".into(), Value::Text("candy".into()))];
        assert_eq!(store.count_where(&cond), 0);
    }

    #[test]
    fn count_where_all() {
        let store = sprint1_store();
        assert_eq!(store.count_where(&[]), 5); // empty conditions = match all
    }

    // ── Sprint 1: Exists ──

    #[test]
    fn exists_true() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::Eq("name".into(), Value::Text("donut".into()))];
        assert!(store.exists(&cond));
    }

    #[test]
    fn exists_false() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::Eq("name".into(), Value::Text("pizza".into()))];
        assert!(!store.exists(&cond));
    }

    // ── Sprint 1: Distinct ──

    #[test]
    fn distinct_values() {
        let store = sprint1_store();
        let vals = store.distinct("category");
        // Should have fruit, vegetable, pastry (NOT null)
        assert_eq!(vals.len(), 3);
        assert!(vals.contains(&Value::Text("fruit".into())));
        assert!(vals.contains(&Value::Text("vegetable".into())));
        assert!(vals.contains(&Value::Text("pastry".into())));
    }

    #[test]
    fn distinct_no_field() {
        let store = sprint1_store();
        let vals = store.distinct("nonexistent");
        assert_eq!(vals.len(), 0);
    }

    // ── Sprint 1: Bulk Delete ──

    #[test]
    fn bulk_delete_by_filter() {
        let mut store = sprint1_store();
        assert_eq!(store.len(), 5);

        let cond = vec![QueryCondition::Eq("category".into(), Value::Text("fruit".into()))];
        let deleted = store.bulk_delete(&cond);
        assert_eq!(deleted, 2); // apple, banana
        assert_eq!(store.len(), 3);

        // Verify they're gone
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        assert!(store.point_query(&key).is_none());
    }

    #[test]
    fn bulk_delete_no_match() {
        let mut store = sprint1_store();
        let cond = vec![QueryCondition::Eq("category".into(), Value::Text("candy".into()))];
        let deleted = store.bulk_delete(&cond);
        assert_eq!(deleted, 0);
        assert_eq!(store.len(), 5);
    }

    // ── Sprint 1: Truncate ──

    #[test]
    fn truncate_clears_all() {
        let mut store = sprint1_store();
        assert_eq!(store.len(), 5);
        let removed = store.truncate();
        assert_eq!(removed, 5);
        assert_eq!(store.len(), 0);
        assert_eq!(store.records().count(), 0);
    }

    #[test]
    fn truncate_then_insert() {
        let mut store = sprint1_store();
        store.truncate();

        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(100));
        r.insert("name".into(), Value::Text("new_item".into()));
        r.insert("price".into(), Value::Float(5.0));
        r.insert("category".into(), Value::Text("misc".into()));
        store.insert(&r);

        assert_eq!(store.len(), 1);
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(100));
        assert!(store.point_query(&key).is_some());
    }

    // ── Sprint 1: Field Projection + Total Count ──

    #[test]
    fn filtered_query_projected_fields() {
        let store = sprint1_store();
        let cond = vec![QueryCondition::Eq("category".into(), Value::Text("fruit".into()))];
        let (results, total) = store.filtered_query_projected(&cond, None, false, None, None, Some(&["name", "price"]));

        assert_eq!(total, 2);
        assert_eq!(results.len(), 2);
        // Each result should only have name and price, not id or category
        for r in &results {
            assert!(r.contains_key("name"));
            assert!(r.contains_key("price"));
            assert!(!r.contains_key("id"));
            assert!(!r.contains_key("category"));
        }
    }

    #[test]
    fn filtered_query_projected_total_count_with_pagination() {
        let store = sprint1_store();
        // Get all records but limit to 2
        let (results, total) = store.filtered_query_projected(&[], None, false, Some(2), None, None);
        assert_eq!(results.len(), 2);
        assert_eq!(total, 5); // total matching is 5, but only 2 returned
    }

    #[test]
    fn filtered_query_projected_offset() {
        let store = sprint1_store();
        let (results, total) = store.filtered_query_projected(
            &[], Some("id"), false, Some(2), Some(2), None
        );
        assert_eq!(total, 5);
        assert_eq!(results.len(), 2);
        // Sorted by id asc, offset 2 → ids 3,4
        assert_eq!(results[0].get("id"), Some(&Value::Integer(3)));
        assert_eq!(results[1].get("id"), Some(&Value::Integer(4)));
    }

    // ── Sprint 1: Combined operator tests (math validation) ──

    #[test]
    fn combined_in_and_gt_operators() {
        let store = sprint1_store();
        // fruit OR pastry, AND price > 1.00
        let cond = vec![
            QueryCondition::In("category".into(), vec![Value::Text("fruit".into()), Value::Text("pastry".into())]),
            QueryCondition::Gt("price".into(), Value::Float(1.00)),
        ];
        let results = store.filtered_query(&cond, None, false, None, None);
        // apple (1.50) and donut (3.50) — banana (0.75) excluded by price
        assert_eq!(results.len(), 2);

        // Math check: count matches count_where
        assert_eq!(store.count_where(&cond), 2);
        assert!(store.exists(&cond));
    }

    #[test]
    fn math_count_equals_len_for_empty_filter() {
        let store = sprint1_store();
        // count_where with no conditions == len
        assert_eq!(store.count_where(&[]), store.len());
    }

    #[test]
    fn math_bulk_delete_count_consistency() {
        let mut store = sprint1_store();
        let initial = store.len();
        let cond = vec![QueryCondition::Eq("category".into(), Value::Text("fruit".into()))];
        let count_before = store.count_where(&cond);
        let deleted = store.bulk_delete(&cond);
        assert_eq!(deleted, count_before); // deleted count == pre-counted
        assert_eq!(store.len(), initial - deleted); // len reduced exactly
    }

    #[test]
    fn math_truncate_returns_len() {
        let mut store = sprint1_store();
        let len_before = store.len();
        let removed = store.truncate();
        assert_eq!(removed, len_before);
        assert_eq!(store.len(), 0);
    }

    // ── Sprint 2: OR Conditions ──

    #[test]
    fn or_conditions_basic() {
        let store = sprint1_store();
        // (category = fruit) OR (category = pastry)
        let or_groups = vec![
            vec![QueryCondition::Eq("category".into(), Value::Text("fruit".into()))],
            vec![QueryCondition::Eq("category".into(), Value::Text("pastry".into()))],
        ];
        let results = store.filtered_query_ex(&[], Some(&or_groups), None, false, None, None);
        assert_eq!(results.len(), 3); // apple, banana, donut
    }

    #[test]
    fn or_conditions_with_and() {
        let store = sprint1_store();
        // price > 2.00 AND ((category = fruit) OR (category = pastry))
        let conditions = vec![QueryCondition::Gt("price".into(), Value::Float(2.00))];
        let or_groups = vec![
            vec![QueryCondition::Eq("category".into(), Value::Text("fruit".into()))],
            vec![QueryCondition::Eq("category".into(), Value::Text("pastry".into()))],
        ];
        let results = store.filtered_query_ex(&conditions, Some(&or_groups), None, false, None, None);
        assert_eq!(results.len(), 1); // only donut (3.50)
        assert_eq!(results[0].get("name"), Some(&Value::Text("donut".into())));
    }

    #[test]
    fn or_conditions_count_where() {
        let store = sprint1_store();
        let or_groups = vec![
            vec![QueryCondition::Eq("name".into(), Value::Text("apple".into()))],
            vec![QueryCondition::Eq("name".into(), Value::Text("banana".into()))],
        ];
        let count = store.count_where_ex(&[], Some(&or_groups));
        assert_eq!(count, 2);
    }

    #[test]
    fn or_conditions_exists() {
        let store = sprint1_store();
        let or_groups = vec![
            vec![QueryCondition::Eq("name".into(), Value::Text("pizza".into()))],
            vec![QueryCondition::Eq("name".into(), Value::Text("sushi".into()))],
        ];
        assert!(!store.exists_ex(&[], Some(&or_groups)));

        let or_groups2 = vec![
            vec![QueryCondition::Eq("name".into(), Value::Text("pizza".into()))],
            vec![QueryCondition::Eq("name".into(), Value::Text("apple".into()))],
        ];
        assert!(store.exists_ex(&[], Some(&or_groups2)));
    }

    #[test]
    fn or_conditions_projected() {
        let store = sprint1_store();
        let or_groups = vec![
            vec![QueryCondition::Eq("category".into(), Value::Text("fruit".into()))],
            vec![QueryCondition::Eq("category".into(), Value::Text("vegetable".into()))],
        ];
        let (results, total) = store.filtered_query_projected_ex(
            &[], Some(&or_groups), None, None, None, Some(&["name"]),
        );
        assert_eq!(total, 3); // apple, banana, carrot
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.contains_key("name"));
            assert!(!r.contains_key("price"));
        }
    }

    // ── Sprint 2: Multi-field Sort ──

    #[test]
    fn multi_field_sort() {
        let schema = BundleSchema::new("employees")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("dept"))
            .fiber(FieldDef::numeric("salary"));
        let mut store = BundleStore::new(schema);

        let data = vec![
            (1, "Eng", 80000.0),
            (2, "Sales", 70000.0),
            (3, "Eng", 60000.0),
            (4, "Sales", 90000.0),
            (5, "Eng", 80000.0),
        ];
        for (id, dept, salary) in data {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(id));
            r.insert("dept".into(), Value::Text(dept.into()));
            r.insert("salary".into(), Value::Float(salary));
            store.insert(&r);
        }

        // Sort by dept ASC, then salary DESC
        let sort_fields = vec![("dept", false), ("salary", true)];
        let (results, total) = store.filtered_query_projected_ex(
            &[], None, Some(&sort_fields), None, None, None,
        );
        assert_eq!(total, 5);
        // Eng group should come first (ASC), salary DESC within
        assert_eq!(results[0].get("dept"), Some(&Value::Text("Eng".into())));
        assert_eq!(results[0].get("salary"), Some(&Value::Float(80000.0)));
        // Last should be Sales with lowest salary
        assert_eq!(results[4].get("dept"), Some(&Value::Text("Sales".into())));
        assert_eq!(results[4].get("salary"), Some(&Value::Float(70000.0)));
    }

    // ── Sprint 2: Auto-generated IDs ──

    #[test]
    fn auto_id_counter() {
        let schema = BundleSchema::new("logs")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("msg"));
        let mut store = BundleStore::new(schema);

        let id1 = store.next_auto_id();
        let id2 = store.next_auto_id();
        let id3 = store.next_auto_id();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);

        // Use the auto-generated ID
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(id1));
        r.insert("msg".into(), Value::Text("hello".into()));
        store.insert(&r);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        assert!(store.point_query(&key).is_some());
    }

    // ── Sprint 2: Atomic Increment ──

    #[test]
    fn increment_integer_field() {
        let mut store = sprint1_store();
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1)); // apple, price=1.50

        // Increment price by 1 (integer amount on float field)
        assert!(store.increment(&key, "price", 1.0));
        let rec = store.point_query(&key).unwrap();
        assert_eq!(rec.get("price"), Some(&Value::Float(2.50)));
    }

    #[test]
    fn increment_by_negative() {
        let mut store = sprint1_store();
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1)); // apple, price=1.50

        assert!(store.increment(&key, "price", -0.50));
        let rec = store.point_query(&key).unwrap();
        assert_eq!(rec.get("price"), Some(&Value::Float(1.00)));
    }

    #[test]
    fn increment_missing_record() {
        let mut store = sprint1_store();
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(999));
        assert!(!store.increment(&key, "price", 1.0));
    }

    #[test]
    fn increment_integer_preserves_type() {
        let schema = BundleSchema::new("counters")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("hits"));
        let mut store = BundleStore::new(schema);

        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("hits".into(), Value::Integer(10));
        store.insert(&r);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        assert!(store.increment(&key, "hits", 5.0));
        let rec = store.point_query(&key).unwrap();
        assert_eq!(rec.get("hits"), Some(&Value::Integer(15))); // stays Integer
    }

    // ── Sprint 2: Add Field ──

    #[test]
    fn add_field_extends_records() {
        let mut store = sprint1_store();
        assert_eq!(store.len(), 5);

        // Add a new field with default
        store.add_field(FieldDef::categorical("color").with_default(Value::Text("red".into())));

        // All existing records should now have the new field with default
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let rec = store.point_query(&key).unwrap();
        assert_eq!(rec.get("color"), Some(&Value::Text("red".into())));

        // New inserts can set the new field
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(99));
        r.insert("name".into(), Value::Text("kiwi".into()));
        r.insert("price".into(), Value::Float(2.0));
        r.insert("category".into(), Value::Text("fruit".into()));
        r.insert("color".into(), Value::Text("green".into()));
        store.insert(&r);

        let mut key99 = Record::new();
        key99.insert("id".into(), Value::Integer(99));
        let rec99 = store.point_query(&key99).unwrap();
        assert_eq!(rec99.get("color"), Some(&Value::Text("green".into())));
    }

    // ── Sprint 2: Add Index ──

    #[test]
    fn add_index_builds_from_existing() {
        let schema = BundleSchema::new("products")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("brand"))
            .fiber(FieldDef::numeric("price"));
        let mut store = BundleStore::new(schema);

        // Insert some records without brand index
        for i in 0..10 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("brand".into(), Value::Text(["Nike", "Adidas"][i as usize % 2].into()));
            r.insert("price".into(), Value::Float(50.0 + i as f64 * 10.0));
            store.insert(&r);
        }

        // No index yet
        assert!(store.indexed_values("brand").is_empty());

        // Add index
        store.add_index("brand");

        // Index should now work
        let vals = store.indexed_values("brand");
        assert_eq!(vals.len(), 2); // Nike, Adidas

        // Range query should work now
        let results = store.range_query("brand", &[Value::Text("Nike".into())]);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn add_index_idempotent() {
        let mut store = sprint1_store();
        store.add_index("category");
        let vals1 = store.indexed_values("category");
        store.add_index("category"); // should be no-op
        let vals2 = store.indexed_values("category");
        assert_eq!(vals1.len(), vals2.len());
    }

    // ── Sprint 2: Math Validation ──

    #[test]
    fn math_or_conditions_partition() {
        // OR conditions should produce the union of each group's matches
        let store = sprint1_store();
        let fruit_cond = vec![QueryCondition::Eq("category".into(), Value::Text("fruit".into()))];
        let pastry_cond = vec![QueryCondition::Eq("category".into(), Value::Text("pastry".into()))];
        let veg_cond = vec![QueryCondition::Eq("category".into(), Value::Text("vegetable".into()))];

        let fruit_count = store.count_where(&fruit_cond);
        let pastry_count = store.count_where(&pastry_cond);
        let veg_count = store.count_where(&veg_cond);

        // OR(fruit, pastry) should equal count(fruit) + count(pastry) (no overlap)
        let or_groups = vec![fruit_cond, pastry_cond.clone()];
        let or_count = store.count_where_ex(&[], Some(&or_groups));
        assert_eq!(or_count, fruit_count + pastry_count);

        // OR(fruit, pastry, vegetable) should equal fruit+pastry+veg
        let or_all = vec![
            vec![QueryCondition::Eq("category".into(), Value::Text("fruit".into()))],
            pastry_cond,
            veg_cond,
        ];
        let or_all_count = store.count_where_ex(&[], Some(&or_all));
        assert_eq!(or_all_count, fruit_count + pastry_count + veg_count);
    }

    #[test]
    fn math_increment_accumulates() {
        let schema = BundleSchema::new("counters")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("hits"));
        let mut store = BundleStore::new(schema);

        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("hits".into(), Value::Integer(0));
        store.insert(&r);

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));

        // Increment 100 times by 1
        for _ in 0..100 {
            store.increment(&key, "hits", 1.0);
        }
        let rec = store.point_query(&key).unwrap();
        assert_eq!(rec.get("hits"), Some(&Value::Integer(100)));
    }

    #[test]
    fn math_add_field_preserves_count() {
        let mut store = sprint1_store();
        let count_before = store.len();
        store.add_field(FieldDef::numeric("new_field").with_default(Value::Float(0.0)));
        assert_eq!(store.len(), count_before);
    }

    // ── Sprint 3 Tests ──────────────────────────────────────────

    #[test]
    fn versioned_update_succeeds() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let mut patches = Record::new();
        patches.insert("salary".into(), Value::Float(80000.0));

        let result = store.update_versioned(&key, &patches, 0);
        assert_eq!(result, Ok(1));

        // Check version was bumped
        let fetched = store.point_query(&key).unwrap();
        assert_eq!(fetched.get("_version"), Some(&Value::Integer(1)));
        assert_eq!(fetched.get("salary"), Some(&Value::Float(80000.0)));
    }

    #[test]
    fn versioned_update_conflict() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let mut patches = Record::new();
        patches.insert("salary".into(), Value::Float(80000.0));

        // First bump version to 1
        store.update_versioned(&key, &patches, 0).unwrap();

        // Now try with wrong version (0, but it's 1 now) → conflict
        let result = store.update_versioned(&key, &patches, 0);
        assert_eq!(result, Err("version_conflict"));
    }

    #[test]
    fn update_returning_gives_patched_record() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let mut patches = Record::new();
        patches.insert("name".into(), Value::Text("Alice V2".into()));

        let result = store.update_returning(&key, &patches);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.get("name"), Some(&Value::Text("Alice V2".into())));
        assert_eq!(r.get("salary"), Some(&Value::Float(75000.0)));
    }

    #[test]
    fn delete_returning_gives_removed_record() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));

        let result = store.delete_returning(&key);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.get("name"), Some(&Value::Text("Alice".into())));

        // Should be gone now
        assert!(store.point_query(&key).is_none());
    }

    #[test]
    fn bundle_stats_correct() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));
        store.insert(&rec(2, "Bob", 80000.0, "Sales"));

        let stats = store.stats();
        assert_eq!(stats.record_count, 2);
        assert_eq!(stats.base_fields, 1);
        assert_eq!(stats.fiber_fields, 3);
        assert!(stats.indexed_fields.contains(&"dept".to_string()));
    }

    #[test]
    fn explain_full_scan() {
        let store = make_store();
        let conditions = vec![QueryCondition::Gt("salary".into(), Value::Float(50000.0))];
        let plan = store.explain(&conditions, None, None, None, None);
        assert_eq!(plan.scan_type, "full_scan");
    }

    #[test]
    fn explain_index_scan() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));
        let conditions = vec![QueryCondition::Eq("dept".into(), Value::Text("Eng".into()))];
        let plan = store.explain(&conditions, None, None, None, None);
        assert!(plan.scan_type.contains("index_scan"));
        assert!(plan.index_scans.contains(&"dept".to_string()));
    }

    #[test]
    fn transaction_all_succeed() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let ops = vec![
            TransactionOp::Insert(rec(2, "Bob", 80000.0, "Sales")),
            TransactionOp::Update {
                key: {
                    let mut k = Record::new();
                    k.insert("id".into(), Value::Integer(1));
                    k
                },
                patches: {
                    let mut p = Record::new();
                    p.insert("salary".into(), Value::Float(90000.0));
                    p
                },
            },
        ];

        let result = store.execute_transaction(&ops);
        assert!(result.is_ok());
        assert_eq!(store.len(), 2);
        // Alice's salary updated
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let alice = store.point_query(&key).unwrap();
        assert_eq!(alice.get("salary"), Some(&Value::Float(90000.0)));
    }

    #[test]
    fn transaction_rollback_on_failure() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let ops = vec![
            TransactionOp::Insert(rec(2, "Bob", 80000.0, "Sales")),
            TransactionOp::Delete({
                let mut k = Record::new();
                k.insert("id".into(), Value::Integer(999)); // doesn't exist
                k
            }),
        ];

        let result = store.execute_transaction(&ops);
        assert!(result.is_err());
        // Rolled back: Bob should NOT exist, Alice should still be there
        assert_eq!(store.len(), 1);
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        assert!(store.point_query(&key).is_some());
    }

    #[test]
    fn math_versioned_update_monotone() {
        // Version must strictly increase
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));

        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let mut patches = Record::new();
        patches.insert("salary".into(), Value::Float(80000.0));

        let v1 = store.update_versioned(&key, &patches, 0).unwrap();
        let v2 = store.update_versioned(&key, &patches, v1).unwrap();
        let v3 = store.update_versioned(&key, &patches, v2).unwrap();
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn math_transaction_preserves_count() {
        let mut store = make_store();
        store.insert(&rec(1, "Alice", 75000.0, "Eng"));
        store.insert(&rec(2, "Bob", 80000.0, "Sales"));

        let ops = vec![
            TransactionOp::Insert(rec(3, "Charlie", 90000.0, "Eng")),
            TransactionOp::Delete({
                let mut k = Record::new();
                k.insert("id".into(), Value::Integer(1));
                k
            }),
        ];

        let count_before = store.len();
        let result = store.execute_transaction(&ops);
        assert!(result.is_ok());
        assert_eq!(store.len(), count_before); // +1 -1 = same
    }

    // ── Geometric Encryption Tests (GEO-ENC-1 through GEO-ENC-15) ──

    fn make_encrypted_store() -> BundleStore {
        let seed: [u8; 32] = {
            let mut s = [0u8; 32];
            for i in 0..32 { s[i] = (i as u8).wrapping_mul(7).wrapping_add(13); }
            s
        };
        let mut schema = BundleSchema::new("enc_weather")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("temp").with_range(120.0))
            .fiber(FieldDef::numeric("humidity").with_range(100.0))
            .fiber(FieldDef::numeric("pressure").with_range(200.0));
        let gk = crate::crypto::GaugeKey::derive(&seed, &schema.fiber_fields);
        schema.gauge_key = Some(gk);
        BundleStore::new(schema)
    }

    fn make_plain_store() -> BundleStore {
        let schema = BundleSchema::new("plain_weather")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("temp").with_range(120.0))
            .fiber(FieldDef::numeric("humidity").with_range(100.0))
            .fiber(FieldDef::numeric("pressure").with_range(200.0));
        BundleStore::new(schema)
    }

    fn weather_rec(id: i64, temp: f64, hum: f64, press: f64) -> Record {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(id));
        r.insert("temp".into(), Value::Float(temp));
        r.insert("humidity".into(), Value::Float(hum));
        r.insert("pressure".into(), Value::Float(press));
        r
    }

    fn insert_weather_data(store: &mut BundleStore) {
        let data = vec![
            (1, -31.9, 45.0, 1013.25),
            (2, 22.5, 65.0, 1010.0),
            (3, 35.1, 80.0, 1005.5),
            (4, -5.0, 30.0, 1020.0),
            (5, 15.3, 55.0, 1015.0),
            (6, 40.2, 90.0, 998.0),
            (7, 0.0, 50.0, 1012.0),
            (8, -15.7, 35.0, 1018.5),
            (9, 28.8, 70.0, 1008.0),
            (10, 12.1, 60.0, 1014.0),
        ];
        for (id, t, h, p) in data {
            store.insert(&weather_rec(id, t, h, p));
        }
    }

    /// GEO-ENC-1: Insert N records — all succeed, stored values ≠ plaintext
    #[test]
    fn geo_enc_1_insert_stored_differs() {
        let mut store = make_encrypted_store();
        insert_weather_data(&mut store);
        assert_eq!(store.len(), 10);

        // Raw stored fiber values should differ from plaintext
        if let Some(fiber) = store.get_fiber(store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(1));
            r
        })) {
            // The stored value for temp should NOT be -31.9
            match &fiber[0] {
                Value::Float(f) => assert!((*f - (-31.9)).abs() > 0.01,
                    "Stored value should be encrypted, got {f}"),
                _ => {} // Integer passthrough is fine
            }
        }
    }

    /// GEO-ENC-2: K(encrypted) ≡ K(plaintext)
    #[test]
    fn geo_enc_2_curvature_invariant() {
        let mut enc_store = make_encrypted_store();
        let mut plain_store = make_plain_store();
        insert_weather_data(&mut enc_store);
        insert_weather_data(&mut plain_store);

        let k_enc = crate::curvature::scalar_curvature(&enc_store);
        let k_plain = crate::curvature::scalar_curvature(&plain_store);

        assert!((k_enc - k_plain).abs() < 1e-10,
            "K must be gauge-invariant: encrypted={k_enc}, plain={k_plain}");
    }

    /// GEO-ENC-3: Confidence(encrypted) ≡ Confidence(plaintext)
    #[test]
    fn geo_enc_3_confidence_invariant() {
        let mut enc_store = make_encrypted_store();
        let mut plain_store = make_plain_store();
        insert_weather_data(&mut enc_store);
        insert_weather_data(&mut plain_store);

        let k_enc = crate::curvature::scalar_curvature(&enc_store);
        let k_plain = crate::curvature::scalar_curvature(&plain_store);
        let c_enc = crate::curvature::confidence(k_enc);
        let c_plain = crate::curvature::confidence(k_plain);

        assert!((c_enc - c_plain).abs() < 1e-10,
            "Confidence must be invariant: encrypted={c_enc}, plain={c_plain}");
    }

    /// GEO-ENC-4: Spectral gap λ₁(encrypted) ≡ λ₁(plaintext)
    #[test]
    fn geo_enc_4_spectral_gap_invariant() {
        let mut enc_store = make_encrypted_store();
        let mut plain_store = make_plain_store();
        insert_weather_data(&mut enc_store);
        insert_weather_data(&mut plain_store);

        // Spectral analysis uses only bitmap topology, not fiber values
        // So it should be identical regardless of encryption
        let spectrum_enc = crate::spectral::spectral_gap(&enc_store);
        let spectrum_plain = crate::spectral::spectral_gap(&plain_store);

        assert!((spectrum_enc - spectrum_plain).abs() < 1e-10,
            "Spectral gap must be invariant: encrypted={spectrum_enc}, plain={spectrum_plain}");
    }

    /// GEO-ENC-5: Point query with correct key → exact plaintext values
    #[test]
    fn geo_enc_5_point_query_decrypts() {
        let mut store = make_encrypted_store();
        insert_weather_data(&mut store);

        // Query id=1 should return the original plaintext values
        let result = store.point_query(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(1));
            r
        });
        assert!(result.is_some(), "Point query should find the record");
        let rec = result.unwrap();
        match rec.get("temp") {
            Some(Value::Float(f)) => assert!((*f - (-31.9)).abs() < 1e-6,
                "Decrypted temp should be -31.9, got {f}"),
            other => panic!("Expected Float for temp, got {:?}", other),
        }
    }

    /// GEO-ENC-6: Raw storage values ≠ plaintext (without key, no decryption)
    #[test]
    fn geo_enc_6_raw_storage_encrypted() {
        let mut store = make_encrypted_store();
        insert_weather_data(&mut store);

        // get_fiber returns raw (encrypted) storage
        let bp = store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(3));
            r
        });
        let raw = store.get_fiber(bp).expect("Should have fiber data");
        // temp was 35.1 — raw should be different
        match &raw[0] {
            Value::Float(f) => assert!((*f - 35.1).abs() > 0.01,
                "Raw stored value should be encrypted, not 35.1, got {f}"),
            _ => {}
        }
    }

    /// GEO-ENC-7: Records iterator returns decrypted values (WHERE works)
    #[test]
    fn geo_enc_7_records_decrypted() {
        let mut store = make_encrypted_store();
        insert_weather_data(&mut store);

        // records() should return decrypted values
        let all: Vec<Record> = store.records().collect();
        assert_eq!(all.len(), 10);

        // Find the record with id=2 and check decrypted values
        let rec2 = all.iter().find(|r| r.get("id") == Some(&Value::Integer(2)));
        assert!(rec2.is_some());
        match rec2.unwrap().get("temp") {
            Some(Value::Float(f)) => assert!((*f - 22.5).abs() < 1e-6,
                "Decrypted temp should be 22.5, got {f}"),
            other => panic!("Expected Float for temp, got {:?}", other),
        }
    }

    /// GEO-ENC-9: Metric distance invariant under gauge transform
    #[test]
    fn geo_enc_9_metric_invariant() {
        // FiberMetric normalizes by range, which is gauge-invariant
        let mut enc_store = make_encrypted_store();
        let mut plain_store = make_plain_store();
        insert_weather_data(&mut enc_store);
        insert_weather_data(&mut plain_store);

        // Both stores should return same records after decryption
        let enc_recs: Vec<Record> = enc_store.records().collect();
        let plain_recs: Vec<Record> = plain_store.records().collect();

        // Check that decrypted records match plaintext records
        for plain_rec in &plain_recs {
            let id = plain_rec.get("id").unwrap();
            let enc_rec = enc_recs.iter().find(|r| r.get("id") == Some(id));
            assert!(enc_rec.is_some(), "Missing record with id={:?}", id);
            let enc_rec = enc_rec.unwrap();

            for field in &["temp", "humidity", "pressure"] {
                match (plain_rec.get(*field), enc_rec.get(*field)) {
                    (Some(Value::Float(a)), Some(Value::Float(b))) => {
                        assert!((*a - *b).abs() < 1e-6,
                            "Field {field}: plain={a}, decrypted={b}");
                    }
                    _ => {}
                }
            }
        }
    }

    /// GEO-ENC-10: Batch insert works with encryption
    #[test]
    fn geo_enc_10_batch_insert() {
        let mut store = make_encrypted_store();
        let records: Vec<Record> = (1..=100).map(|i| {
            weather_rec(i, -30.0 + i as f64 * 0.7, 20.0 + i as f64 * 0.5, 990.0 + i as f64 * 0.3)
        }).collect();
        let count = store.batch_insert(&records);
        assert_eq!(count, 100);
        assert_eq!(store.len(), 100);

        // Verify roundtrip: records() should decrypt
        let rec1 = store.point_query(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(1));
            r
        });
        assert!(rec1.is_some());
        match rec1.unwrap().get("temp") {
            Some(Value::Float(f)) => assert!((*f - (-29.3)).abs() < 1e-6,
                "Batch-inserted record should decrypt correctly, got {f}"),
            other => panic!("Expected Float, got {:?}", other),
        }
    }

    /// GEO-ENC-11: K invariant on encrypted batch insert
    #[test]
    fn geo_enc_11_batch_curvature_invariant() {
        let mut enc_store = make_encrypted_store();
        let mut plain_store = make_plain_store();

        let records: Vec<Record> = (1..=100).map(|i| {
            weather_rec(i, -30.0 + i as f64 * 0.7, 20.0 + i as f64 * 0.5, 990.0 + i as f64 * 0.3)
        }).collect();
        enc_store.batch_insert(&records);
        plain_store.batch_insert(&records);

        let k_enc = crate::curvature::scalar_curvature(&enc_store);
        let k_plain = crate::curvature::scalar_curvature(&plain_store);

        assert!((k_enc - k_plain).abs() < 1e-10,
            "K must be gauge-invariant for batch: enc={k_enc}, plain={k_plain}");
    }

    /// GEO-ENC-12: Different seeds produce different encrypted values
    #[test]
    fn geo_enc_12_different_seeds() {
        let seed1: [u8; 32] = {
            let mut s = [0u8; 32];
            for i in 0..32 { s[i] = i as u8; }
            s
        };
        let seed2: [u8; 32] = {
            let mut s = [0u8; 32];
            for i in 0..32 { s[i] = (i as u8).wrapping_add(100); }
            s
        };

        let fields = vec![FieldDef::numeric("temp")];
        let k1 = crate::crypto::GaugeKey::derive(&seed1, &fields);
        let k2 = crate::crypto::GaugeKey::derive(&seed2, &fields);

        assert_ne!(k1.transforms[0].scale, k2.transforms[0].scale);
    }

    /// GEO-ENC-13: Known-plaintext resistance — knowing one field's transform
    /// doesn't reveal another field's transform
    #[test]
    fn geo_enc_13_known_plaintext_resistance() {
        let seed: [u8; 32] = {
            let mut s = [0u8; 32];
            for i in 0..32 { s[i] = (i as u8).wrapping_mul(7).wrapping_add(13); }
            s
        };
        let fields = vec![
            FieldDef::numeric("temp"),
            FieldDef::numeric("humidity"),
            FieldDef::numeric("pressure"),
        ];
        let key = crate::crypto::GaugeKey::derive(&seed, &fields);

        // Each field transform is independently derived
        let t0 = &key.transforms[0];
        let t1 = &key.transforms[1];
        let t2 = &key.transforms[2];

        // All three should be different (field name is mixed into derivation)
        assert_ne!(t0.scale, t1.scale);
        assert_ne!(t1.scale, t2.scale);
        assert_ne!(t0.offset, t1.offset);
    }

    /// GEO-ENC-14: GQL ENCRYPTED syntax creates encrypted bundle
    #[test]
    fn geo_enc_14_gql_encrypted_syntax() {
        let stmt = crate::parser::parse(
            "BUNDLE enc_test BASE (id NUMERIC) FIBER (val NUMERIC RANGE 100) ENCRYPTED"
        ).unwrap();
        match stmt {
            crate::parser::Statement::CreateBundle { encrypted, .. } => {
                assert!(encrypted, "ENCRYPTED keyword should set encrypted=true");
            }
            _ => panic!("Expected CreateBundle"),
        }
    }

    /// GEO-ENC-15: Non-encrypted bundle has no gauge key
    #[test]
    fn geo_enc_15_no_encryption_default() {
        let mut store = make_plain_store();
        assert!(store.schema.gauge_key.is_none());
        insert_weather_data(&mut store);

        // get_fiber should return actual plaintext
        let bp = store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(1));
            r
        });
        let raw = store.get_fiber(bp).expect("Should have fiber data");
        match &raw[0] {
            Value::Float(f) => assert!((*f - (-31.9)).abs() < 1e-10,
                "Plaintext store should store raw values, got {f}"),
            _ => {}
        }
    }

    // ── BETWEEN operator tests ──────────────────────────────────────────────

    #[test]
    fn test_between_numeric_inclusive_bounds() {
        let schema = BundleSchema::new("t")
            .base(FieldDef::numeric("x"));
        let mut store = BundleStore::new(schema);
        for i in 0i64..=10 {
            let mut r = Record::new();
            r.insert("x".into(), Value::Integer(i));
            store.insert(&r);
        }
        let cond = QueryCondition::Between(
            "x".into(), Value::Integer(3), Value::Integer(7),
        );
        let results = store.filtered_query(&[cond], None, false, None, None);
        // Should return 3,4,5,6,7 → 5 records
        assert_eq!(results.len(), 5);
        for r in &results {
            let v = match r.get("x").unwrap() {
                Value::Integer(i) => *i,
                _ => panic!("unexpected type"),
            };
            assert!((3..=7).contains(&v), "x={v} not in [3,7]");
        }
    }

    #[test]
    fn test_between_boundary_values_included() {
        let schema = BundleSchema::new("t")
            .base(FieldDef::numeric("score"));
        let mut store = BundleStore::new(schema);
        for v in [0.0f64, 5.0, 10.0, 15.0] {
            let mut r = Record::new();
            r.insert("score".into(), Value::Float(v));
            store.insert(&r);
        }
        let cond = QueryCondition::Between(
            "score".into(), Value::Float(5.0), Value::Float(10.0),
        );
        let results = store.filtered_query(&[cond], None, false, None, None);
        // 5.0 and 10.0 are both included (inclusive)
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_between_no_matches() {
        let schema = BundleSchema::new("t")
            .base(FieldDef::numeric("n"));
        let mut store = BundleStore::new(schema);
        for i in [1i64, 2, 3] {
            let mut r = Record::new();
            r.insert("n".into(), Value::Integer(i));
            store.insert(&r);
        }
        let cond = QueryCondition::Between(
            "n".into(), Value::Integer(10), Value::Integer(20),
        );
        let results = store.filtered_query(&[cond], None, false, None, None);
        assert!(results.is_empty());
    }

    // ── VectorMetric tests ──────────────────────────────────────────────────

    #[test]
    fn test_cosine_identical_unit_vectors() {
        let v = vec![1.0f64, 0.0, 0.0];
        let score = VectorMetric::Cosine.score(&v, &v);
        assert!((score - 1.0).abs() < 1e-10, "cosine of identical vectors = 1, got {score}");
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let a = vec![1.0f64, 0.0];
        let b = vec![0.0f64, 1.0];
        let score = VectorMetric::Cosine.score(&a, &b);
        assert!(score.abs() < 1e-10, "cosine of orthogonal vectors = 0, got {score}");
    }

    #[test]
    fn test_cosine_opposite_vectors() {
        let a = vec![1.0f64, 0.0];
        let b = vec![-1.0f64, 0.0];
        let score = VectorMetric::Cosine.score(&a, &b);
        assert!((score - (-1.0)).abs() < 1e-10, "cosine of opposite vectors = -1, got {score}");
    }

    #[test]
    fn test_euclidean_same_point() {
        let v = vec![3.0f64, 4.0];
        // Same point → distance 0 → score 0
        let score = VectorMetric::Euclidean.score(&v, &v);
        assert!(score.abs() < 1e-10, "euclidean same point = 0, got {score}");
    }

    #[test]
    fn test_euclidean_known_distance() {
        let a = vec![0.0f64, 0.0];
        let b = vec![3.0f64, 4.0];
        // distance = 5 → score = -5
        let score = VectorMetric::Euclidean.score(&a, &b);
        assert!((score - (-5.0)).abs() < 1e-10, "euclidean score = -5, got {score}");
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0f64, 2.0, 3.0];
        let b = vec![4.0f64, 5.0, 6.0];
        // dot = 1*4+2*5+3*6 = 4+10+18 = 32
        let score = VectorMetric::Dot.score(&a, &b);
        assert!((score - 32.0).abs() < 1e-10, "dot product = 32, got {score}");
    }

    // ── vector_search tests ─────────────────────────────────────────────────

    fn make_vector_store() -> BundleStore {
        use crate::types::FieldType;
        let schema = BundleSchema::new("vecs")
            .base(FieldDef { name: "id".into(), field_type: FieldType::Numeric, default: Value::Null, range: None, weight: 1.0 })
            .fiber(FieldDef { name: "emb".into(), field_type: FieldType::Vector { dims: 2 }, default: Value::Null, range: None, weight: 1.0 })
            .fiber(FieldDef { name: "cat".into(), field_type: FieldType::Categorical, default: Value::Null, range: None, weight: 1.0 });
        let mut store = BundleStore::new(schema);
        // Insert 5 vectors at known positions in 2D
        let vecs: &[(i64, [f64; 2], &str)] = &[
            (1, [1.0, 0.0], "A"),
            (2, [0.0, 1.0], "B"),
            (3, [-1.0, 0.0], "A"),
            (4, [0.0, -1.0], "B"),
            (5, [0.707, 0.707], "A"),
        ];
        for (id, emb, cat) in vecs {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(*id));
            r.insert("emb".into(), Value::Vector(emb.to_vec()));
            r.insert("cat".into(), Value::Text(cat.to_string()));
            store.insert(&r);
        }
        store
    }

    #[test]
    fn test_vector_search_top1_cosine() {
        let store = make_vector_store();
        // Query [1,0] → closest by cosine is itself (id=1)
        let results = store.vector_search("emb", &[1.0, 0.0], 1, VectorMetric::Cosine, &[]);
        assert_eq!(results.len(), 1);
        let (score, rec) = &results[0];
        assert!((score - 1.0).abs() < 1e-6, "top cosine score = 1.0, got {score}");
        assert_eq!(rec.get("id"), Some(&Value::Integer(1)));
    }

    #[test]
    fn test_vector_search_top3_cosine() {
        let store = make_vector_store();
        // Query [1,0] → top-3 by cosine: id=1 (1.0), id=5 (~0.707), id=2 or 4 (0.0)
        let results = store.vector_search("emb", &[1.0, 0.0], 3, VectorMetric::Cosine, &[]);
        assert_eq!(results.len(), 3);
        // Scores are descending
        for i in 0..results.len() - 1 {
            assert!(results[i].0 >= results[i + 1].0, "scores not sorted descending");
        }
        // id=1 must be first
        assert_eq!(results[0].1.get("id"), Some(&Value::Integer(1)));
    }

    #[test]
    fn test_vector_search_top_k_clamped_to_n() {
        let store = make_vector_store();
        // Requesting more than N → returns all N
        let results = store.vector_search("emb", &[1.0, 0.0], 100, VectorMetric::Cosine, &[]);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_vector_search_with_prefilter() {
        let store = make_vector_store();
        // Only look at cat="A" records: ids 1, 3, 5
        let pre_filter = vec![QueryCondition::Eq("cat".into(), Value::Text("A".into()))];
        let results = store.vector_search("emb", &[1.0, 0.0], 10, VectorMetric::Cosine, &pre_filter);
        assert_eq!(results.len(), 3, "pre-filter should restrict to cat=A");
        for (_, rec) in &results {
            assert_eq!(rec.get("cat"), Some(&Value::Text("A".into())));
        }
    }

    #[test]
    fn test_vector_search_dimension_mismatch_skips() {
        let store = make_vector_store();
        // Wrong dims → all vectors skipped → empty results
        let results = store.vector_search("emb", &[1.0, 0.0, 0.0], 10, VectorMetric::Cosine, &[]);
        assert!(results.is_empty(), "dimension mismatch should return empty");
    }

    #[test]
    fn test_vector_search_euclidean_ranking() {
        let store = make_vector_store();
        // Query [1,0] by euclidean → id=1 is closest (distance 0)
        let results = store.vector_search("emb", &[1.0, 0.0], 1, VectorMetric::Euclidean, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.get("id"), Some(&Value::Integer(1)));
    }

    // ── drop_field tests ────────────────────────────────────────────────────

    fn make_drop_store() -> BundleStore {
        let schema = BundleSchema::new("t")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"))
            .fiber(FieldDef::numeric("score").with_range(100.0))
            .fiber(FieldDef::categorical("dept"))
            .index("dept");
        let mut store = BundleStore::new(schema);
        for i in 0i64..5 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("user_{i}")));
            r.insert("score".into(), Value::Float(i as f64 * 10.0));
            r.insert("dept".into(), Value::Text("eng".into()));
            store.insert(&r);
        }
        store
    }

    #[test]
    fn test_drop_field_removes_from_schema() {
        let mut store = make_drop_store();
        assert!(store.schema.fiber_fields.iter().any(|f| f.name == "score"));
        let ok = store.drop_field("score");
        assert!(ok, "drop_field should return true for existing field");
        assert!(!store.schema.fiber_fields.iter().any(|f| f.name == "score"),
            "score should be gone from schema");
    }

    #[test]
    fn test_drop_field_records_no_longer_have_field() {
        let mut store = make_drop_store();
        store.drop_field("score");
        for rec in store.records() {
            assert!(!rec.contains_key("score"), "dropped field should not appear in records");
        }
    }

    #[test]
    fn test_drop_field_returns_false_for_unknown() {
        let mut store = make_drop_store();
        let ok = store.drop_field("nonexistent_field");
        assert!(!ok);
    }

    #[test]
    fn test_drop_field_does_not_affect_other_fields() {
        let mut store = make_drop_store();
        store.drop_field("score");
        for rec in store.records() {
            assert!(rec.contains_key("name"), "name should still be present");
            assert!(rec.contains_key("dept"), "dept should still be present");
        }
    }

    #[test]
    fn test_drop_indexed_field_removes_index() {
        let mut store = make_drop_store();
        assert!(store.field_index.contains_key("dept"));
        store.drop_field("dept");
        assert!(!store.field_index.contains_key("dept"), "index should be removed");
        assert!(!store.schema.indexed_fields.contains(&"dept".to_string()),
            "indexed_fields should be updated");
    }

    #[test]
    fn test_drop_field_store_still_queryable() {
        let mut store = make_drop_store();
        store.drop_field("score");
        // Should still be able to query on remaining fields
        let results = store.filtered_query(
            &[QueryCondition::Eq("dept".into(), Value::Text("eng".into()))],
            None, false, None, None,
        );
        assert_eq!(results.len(), 5);
    }

    // =========================================================================
    // ANOMALY DETECTION — INTEGRATION TESTS (real complex dataset)
    //
    // Dataset: 20 cities × 366 days = 7 320 weather records.
    // Schema: id (key), city (indexed categorical), day (int),
    //         temp_c (°C), humidity_pct, pressure_hpa, wind_kmh,
    //         precip_mm, uv_index (all numeric fiber fields).
    //
    // Injected anomalies:
    //   • Moscow record on day 15  → temp = −55 °C   (extreme cold)
    //   • Dubai record on day 180  → humidity = 1 %  (extreme dry)
    //   • Reykjavik record on day 300 → pressure = 880 hPa (storm)
    //   • Singapore record on day 90 → uv_index = 20  (extreme UV)
    //
    // Expected: all 4 anomalies surface in top anomalies at ≥ 2σ.
    //           Normal records stay below the threshold.
    // =========================================================================

    /// Build the 20-city, 366-day weather bundle.
    fn make_weather_bundle() -> BundleStore {
        // 20 representative cities across climate zones
        // (city, base_temp_c, base_hum_pct, base_press_hpa, base_wind_kmh, base_uv)
        let cities: &[(&str, f64, f64, f64, f64, f64)] = &[
            ("Moscow",       -5.0,  65.0, 1013.0,  18.0,  2.0),
            ("Dubai",        35.0,  55.0, 1010.0,  14.0,  9.0),
            ("Reykjavik",     4.0,  80.0, 1008.0,  25.0,  1.5),
            ("Singapore",    27.0,  85.0, 1009.0,   8.0, 11.0),
            ("London",       11.0,  75.0, 1012.0,  16.0,  3.0),
            ("Chicago",       9.0,  68.0, 1011.0,  22.0,  4.0),
            ("Sydney",       19.0,  70.0, 1014.0,  12.0,  6.0),
            ("Tokyo",        16.0,  72.0, 1012.0,  10.0,  5.0),
            ("Cairo",        24.0,  45.0, 1011.0,  15.0,  8.0),
            ("SaoPaulo",     22.0,  78.0, 1010.0,  11.0,  7.0),
            ("Toronto",       5.0,  66.0, 1012.0,  20.0,  3.5),
            ("Mumbai",       28.0,  82.0, 1008.0,   9.0, 10.0),
            ("Paris",        12.0,  74.0, 1013.0,  14.0,  3.5),
            ("Nairobi",      18.0,  62.0, 1015.0,  13.0,  7.5),
            ("Oslo",          4.0,  70.0, 1009.0,  19.0,  2.0),
            ("BuenosAires",  18.0,  73.0, 1011.0,  15.0,  6.5),
            ("Seoul",        12.0,  65.0, 1013.0,  17.0,  4.5),
            ("Lagos",        28.0,  80.0, 1009.0,  10.0,  9.5),
            ("Melbourne",    15.0,  66.0, 1012.0,  18.0,  5.5),
            ("NewYork",      13.0,  67.0, 1012.0,  19.0,  4.0),
        ];

        // Schema: id (base key), city (indexed categorical fiber),
        // measurement fields only (no day – keeps K focused on observations).
        let schema = BundleSchema::new("weather")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("city"))
            .fiber(FieldDef::numeric("temp_c").with_range(120.0))
            .fiber(FieldDef::numeric("humidity_pct").with_range(100.0))
            .fiber(FieldDef::numeric("pressure_hpa").with_range(200.0))
            .fiber(FieldDef::numeric("wind_kmh").with_range(120.0))
            .fiber(FieldDef::numeric("precip_mm").with_range(60.0))
            .fiber(FieldDef::numeric("uv_index").with_range(20.0))
            .index("city");

        let mut store = BundleStore::new(schema);

        // Deterministic LCG noise: xₙ₊₁ = (a·xₙ + c) mod 2^64
        let lcg = |seed: u64| -> f64 {
            let s = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (((s >> 33) as f64) / (u32::MAX as f64)) - 0.5  // ∈ [−0.5, 0.5]
        };

        let mut id: i64 = 0;
        for (city_idx, (city, base_temp, base_hum, base_press, base_wind, base_uv)) in cities.iter().enumerate() {
            for day in 1i64..=366 {
                let seed = (city_idx as u64 * 400 + day as u64) * 2654435761;
                let noise = |scale: f64, s: u64| lcg(seed.wrapping_add(s)) * scale;
                let season_temp = base_temp + 10.0 * (2.0 * std::f64::consts::PI * day as f64 / 366.0).sin();
                let temp    = (season_temp  + noise(4.0, 1)).clamp(-40.0, 55.0);
                let hum     = (*base_hum    + noise(10.0, 2)).clamp(10.0, 100.0);
                let press   = (*base_press  + noise(5.0,  3)).clamp(950.0, 1040.0);
                let wind    = (*base_wind   + noise(8.0,  4)).clamp(0.0,  100.0);
                let precip  = (noise(5.0, 5).abs() * 8.0).clamp(0.0, 50.0);
                let uv      = (*base_uv     + noise(2.0,  6)).clamp(0.0,  14.0);

                let mut rec = Record::new();
                rec.insert("id".into(),           Value::Integer(id));
                rec.insert("city".into(),          Value::Text(city.to_string()));
                rec.insert("temp_c".into(),        Value::Float(temp));
                rec.insert("humidity_pct".into(),  Value::Float(hum));
                rec.insert("pressure_hpa".into(),  Value::Float(press));
                rec.insert("wind_kmh".into(),      Value::Float(wind));
                rec.insert("precip_mm".into(),     Value::Float(precip));
                rec.insert("uv_index".into(),      Value::Float(uv));
                store.insert(&rec);
                id += 1;
            }
        }

        // ── Inject 4 hard anomalies — overwrite existing records in-place ─────
        //
        // City indices (0-based) × 366 days/city:
        //   Moscow=0, Dubai=1, Reykjavik=2, Singapore=3
        //
        // Anomaly 1: Moscow day 15 (id=14) → extreme cold −65 °C
        let anomaly_ids: &[(i64, &str, f64, f64, f64, f64, f64, f64)] = &[
            // (id, city, temp, hum, press, wind, precip, uv)
            (0*366 + 14, "Moscow",    -65.0, 62.0, 1013.0, 16.0,  3.0,  2.0),
            (1*366 + 179, "Dubai",     36.0,  1.5, 1010.0, 14.0,  0.0,  9.0),
            (2*366 + 299, "Reykjavik",  3.0, 79.0,  862.0, 24.0, 12.0,  1.5),
            (3*366 + 89,  "Singapore", 27.0, 84.0, 1009.0,  8.0,  5.0, 22.0),
        ];

        for &(an_id, city, temp, hum, press, wind, precip, uv) in anomaly_ids {
            let mut r = Record::new();
            r.insert("id".into(),           Value::Integer(an_id));
            r.insert("city".into(),          Value::Text(city.into()));
            r.insert("temp_c".into(),        Value::Float(temp));
            r.insert("humidity_pct".into(),  Value::Float(hum));
            r.insert("pressure_hpa".into(),  Value::Float(press));
            r.insert("wind_kmh".into(),      Value::Float(wind));
            r.insert("precip_mm".into(),     Value::Float(precip));
            r.insert("uv_index".into(),      Value::Float(uv));
            store.insert(&r);
        }

        store
    }

    /// AD-1.1: 7320 normal records → curvature_stats populated, non-zero count.
    #[test]
    fn ad_1_1_curvature_stats_populated_after_bulk_insert() {
        let store = make_weather_bundle();
        let cs = &store.curvature_stats;
        assert!(cs.k_count >= 7320, "k_count = {}", cs.k_count);
        assert!(cs.k_sum > 0.0, "k_sum should be positive");
        assert!(cs.mean() > 0.0, "mean K should be positive");
        assert!(cs.std_dev() > 0.0, "std dev should be positive");
    }

    /// AD-1.2: Injected anomalies have z-score > 2.0.
    #[test]
    fn ad_1_2_injected_anomalies_exceed_2sigma() {
        let store = make_weather_bundle();
        // All 4 anomalies surface in compute_anomalies at 2σ
        let anomalies = store.compute_anomalies(2.0, None, 50);
        let z_scores: Vec<f64> = anomalies.iter().map(|a| a.z_score).collect();
        assert!(!anomalies.is_empty(), "should detect at least 1 anomaly");
        for (a, z) in anomalies.iter().zip(&z_scores) {
            assert!(z > &2.0, "anomaly z={z:.2} for {:?}", a.record.get("city"));
        }
    }

    /// AD-1.3: Confidence is 1/(1+K) — verify formula holds for top anomaly.
    #[test]
    fn ad_1_3_confidence_formula() {
        let store = make_weather_bundle();
        let anomalies = store.compute_anomalies(2.0, None, 10);
        assert!(!anomalies.is_empty());
        for a in &anomalies {
            let expected = 1.0 / (1.0 + a.local_curvature);
            assert!((a.confidence - expected).abs() < 1e-9,
                "conf mismatch: {} vs {}", a.confidence, expected);
        }
    }

    /// AD-1.4: Normal records below 2σ threshold — normal:anomaly ratio high.
    #[test]
    fn ad_1_4_normal_records_below_threshold() {
        let store = make_weather_bundle();
        let total = store.len();
        let anomalies = store.compute_anomalies(2.0, None, total);
        // Injected only 4 hard anomalies into 7320 records → < 1% anomaly rate expected
        let rate = anomalies.len() as f64 / total as f64;
        assert!(rate < 0.05, "anomaly rate = {rate:.3}, expected < 5%");
        assert!(anomalies.len() >= 4, "should catch all 4 injected anomalies, got {}", anomalies.len());
    }

    /// AD-1.5: deviation_distance is non-negative.
    #[test]
    fn ad_1_5_deviation_distance_non_negative() {
        let store = make_weather_bundle();
        let anomalies = store.compute_anomalies(2.0, None, 20);
        for a in &anomalies {
            assert!(a.deviation_distance >= 0.0,
                "deviation_distance = {}", a.deviation_distance);
        }
    }

    /// AD-2.1: Adaptive threshold shifts with n_sigma parameter.
    #[test]
    fn ad_2_1_threshold_shifts_with_sigma() {
        let store = make_weather_bundle();
        let cs = &store.curvature_stats;
        let t2 = cs.threshold(2.0);
        let t3 = cs.threshold(3.0);
        assert!(t2 < t3, "2σ threshold {t2:.4} should be < 3σ threshold {t3:.4}");
    }

    /// AD-2.2: Higher sigma → fewer anomalies returned.
    #[test]
    fn ad_2_2_higher_sigma_fewer_anomalies() {
        let store = make_weather_bundle();
        let total = store.len();
        let a2 = store.compute_anomalies(2.0, None, total);
        let a3 = store.compute_anomalies(3.0, None, total);
        assert!(a2.len() >= a3.len(),
            "2σ count {} should be >= 3σ count {}", a2.len(), a3.len());
    }

    /// AD-2.3: Very high sigma (999) → zero anomalies.
    #[test]
    fn ad_2_3_extreme_sigma_returns_zero() {
        let store = make_weather_bundle();
        let anomalies = store.compute_anomalies(999.0, None, 1000);
        assert_eq!(anomalies.len(), 0,
            "999σ should yield 0 anomalies, got {}", anomalies.len());
    }

    /// AD-2.4: threshold = μ_K + n_sigma × σ_K.
    #[test]
    fn ad_2_4_threshold_formula() {
        let store = make_weather_bundle();
        let cs = &store.curvature_stats;
        let expected = cs.mean() + 2.0 * cs.std_dev();
        let actual = cs.threshold(2.0);
        assert!((expected - actual).abs() < 1e-9,
            "threshold formula: expected {expected}, got {actual}");
    }

    /// AD-3.1: Contributing field for Moscow anomaly is temp_c.
    #[test]
    fn ad_3_1_moscow_cold_anomaly_contributing_field() {
        let store = make_weather_bundle();
        let anomalies = store.compute_anomalies(2.0, None, 100);
        // Find the Moscow −55 °C record
        let moscow_anomaly = anomalies.iter().find(|a| {
            a.record.get("city") == Some(&Value::Text("Moscow".into()))
                && a.record.get("temp_c") == Some(&Value::Float(-65.0))
        });
        assert!(moscow_anomaly.is_some(), "Moscow cold anomaly not found in top anomalies");
        let a = moscow_anomaly.unwrap();
        assert!(a.contributing_fields.contains(&"temp_c".to_string()),
            "contributing_fields should include temp_c, got {:?}", a.contributing_fields);
    }

    /// AD-3.2: Reykjavik storm anomaly contributing field includes pressure_hpa.
    #[test]
    fn ad_3_2_reykjavik_storm_contributing_field() {
        let store = make_weather_bundle();
        let anomalies = store.compute_anomalies(2.0, None, 100);
        let reyk = anomalies.iter().find(|a| {
            a.record.get("city") == Some(&Value::Text("Reykjavik".into()))
                && a.record.get("pressure_hpa") == Some(&Value::Float(862.0))
        });
        assert!(reyk.is_some(), "Reykjavik storm anomaly not found");
        let a = reyk.unwrap();
        assert!(a.contributing_fields.contains(&"pressure_hpa".to_string()),
            "contributing_fields should include pressure_hpa, got {:?}", a.contributing_fields);
    }

    /// AD-4.1: Pre-filter by city isolates anomalies to that city only.
    #[test]
    fn ad_4_1_prefilter_by_city() {
        let store = make_weather_bundle();
        let conditions = vec![QueryCondition::Eq("city".into(), Value::Text("Moscow".into()))];
        let anomalies = store.compute_anomalies(2.0, Some(&conditions), 50);
        for a in &anomalies {
            assert_eq!(
                a.record.get("city"),
                Some(&Value::Text("Moscow".into())),
                "pre-filter should restrict to Moscow only"
            );
        }
    }

    /// AD-4.2: Pre-filter by Dubai → finds Dubai humidity anomaly.
    #[test]
    fn ad_4_2_prefilter_dubai_finds_humidity_anomaly() {
        let store = make_weather_bundle();
        let conditions = vec![QueryCondition::Eq("city".into(), Value::Text("Dubai".into()))];
        let anomalies = store.compute_anomalies(2.0, Some(&conditions), 20);
        let dubai_humid = anomalies.iter().find(|a| {
            a.record.get("humidity_pct") == Some(&Value::Float(1.5))
        });
        assert!(dubai_humid.is_some(), "Dubai humidity anomaly not found in filtered results; anomalies: {anomalies:?}");
    }

    /// AD-5.1: curvature_stats.mean() is in (0, 1] after bulk insert.
    #[test]
    fn ad_5_1_mean_in_unit_interval() {
        let store = make_weather_bundle();
        let mu = store.curvature_stats.mean();
        assert!(mu > 0.0 && mu <= 1.0,
            "global K mean = {mu}, expected ∈ (0, 1]");
    }

    /// AD-5.2: Results are sorted descending by z-score.
    #[test]
    fn ad_5_2_results_sorted_by_z_score() {
        let store = make_weather_bundle();
        let anomalies = store.compute_anomalies(2.0, None, 50);
        for w in anomalies.windows(2) {
            assert!(w[0].z_score >= w[1].z_score,
                "out of order: z[0]={:.3} < z[1]={:.3}", w[0].z_score, w[1].z_score);
        }
    }

    /// AD-5.3: limit parameter is respected.
    #[test]
    fn ad_5_3_limit_respected() {
        let store = make_weather_bundle();
        let anomalies = store.compute_anomalies(0.5, None, 10); // low sigma = many anomalies
        assert!(anomalies.len() <= 10, "limit 10 violated: got {}", anomalies.len());
    }

    /// AD-5.4: Truncated bundle → curvature_stats reset.
    #[test]
    fn ad_5_4_truncate_resets_curvature_stats() {
        let mut store = make_weather_bundle();
        assert!(store.curvature_stats.k_count > 0);
        store.truncate();
        assert_eq!(store.curvature_stats.k_count, 0);
        assert_eq!(store.curvature_stats.k_sum, 0.0);
    }

    /// AD-6.1: record_k_for returns higher K for anomaly vs normal record.
    #[test]
    fn ad_6_1_record_k_for_anomaly_gt_normal() {
        let store = make_weather_bundle();
        // Moscow day 15 anomaly: id = 0*366 + 14 = 14
        let mut key_anomaly = Record::new();
        key_anomaly.insert("id".into(), Value::Integer(0 * 366 + 14));
        let bp_anomaly = store.base_point(&key_anomaly);
        let k_anomaly = store.record_k_for(bp_anomaly);

        // Moscow day 1 (normal): id = 0
        let mut key_normal = Record::new();
        key_normal.insert("id".into(), Value::Integer(0));
        let bp_normal = store.base_point(&key_normal);
        let k_normal = store.record_k_for(bp_normal);

        assert!(k_anomaly > k_normal,
            "K(anomaly)={k_anomaly:.4} should > K(normal)={k_normal:.4}");
    }

    /// AD-6.2: All 4 injected anomalies appear in top-50 results at 2σ.
    #[test]
    fn ad_6_2_all_four_anomalies_detected() {
        let store = make_weather_bundle();
        let anomalies = store.compute_anomalies(2.0, None, 100);

        let has_moscow = anomalies.iter().any(|a|
            a.record.get("city") == Some(&Value::Text("Moscow".into()))
                && a.record.get("temp_c") == Some(&Value::Float(-65.0))
        );
        let has_dubai = anomalies.iter().any(|a|
            a.record.get("city") == Some(&Value::Text("Dubai".into()))
                && a.record.get("humidity_pct") == Some(&Value::Float(1.5))
        );
        let has_reykjavik = anomalies.iter().any(|a|
            a.record.get("city") == Some(&Value::Text("Reykjavik".into()))
                && a.record.get("pressure_hpa") == Some(&Value::Float(862.0))
        );
        let has_singapore = anomalies.iter().any(|a|
            a.record.get("city") == Some(&Value::Text("Singapore".into()))
                && a.record.get("uv_index") == Some(&Value::Float(22.0))
        );

        assert!(has_moscow,    "Moscow extreme cold anomaly not detected");
        assert!(has_dubai,     "Dubai extreme dry anomaly not detected");
        assert!(has_reykjavik, "Reykjavik storm pressure anomaly not detected");
        assert!(has_singapore, "Singapore extreme UV anomaly not detected");
    }

    /// AD-7.1: compute_anomalies on 7320 records completes in < 2000ms (debug) / 250ms (release).
    #[test]
    fn ad_7_1_anomaly_scan_timing() {
        let store = make_weather_bundle();
        let start = std::time::Instant::now();
        let _ = store.compute_anomalies(2.0, None, 100);
        let elapsed = start.elapsed();
        // Generous limit for debug builds; release builds are 10-20× faster.
        let limit_ms = if cfg!(debug_assertions) { 2000 } else { 250 };
        assert!(elapsed.as_millis() < limit_ms,
            "compute_anomalies took {}ms, expected < {}ms", elapsed.as_millis(), limit_ms);
    }

    /// AD-7.2: curvature_stats.mean() / std_dev() are O(1) — no scan needed.
    #[test]
    fn ad_7_2_stats_access_is_o1() {
        let store = make_weather_bundle();
        let start = std::time::Instant::now();
        for _ in 0..10_000 {
            let _ = store.curvature_stats.mean();
            let _ = store.curvature_stats.std_dev();
            let _ = store.curvature_stats.threshold(2.0);
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 5,
            "10k stats calls took {}ms — should be ~0ms (O(1))", elapsed.as_millis());
    }
}
