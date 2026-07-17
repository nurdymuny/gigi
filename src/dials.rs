//! Marcella dial surface — wave 2 (GEODESIC_LOOM_PLAN.md asks #3 + #4,
//! signed Hallie, 2026-07-16).
//!
//! This module owns the report-building for the cognitive-geometry dial
//! endpoints `GET /v1/bundles/{name}/horizon` and `/capacity` (the
//! HTTP handlers in `gigi_stream.rs` are thin glue over these fns), the
//! new opt-in scoping parameters, and the `windowed_coherence` one-shot.
//!
//! ## Ask 4 — locus + vector-only statistics
//!
//! The census pathology: on `marcella_source_embeddings_bge_v2` the
//! whole-bundle Welford radius (the `l_c` fallback estimator and the
//! `D` proxy in the Davis-Conjecture λ-budget) is blown to 4.7e6 by a
//! single huge-variance ts-like scalar (`ingested_at`), so `s_max`
//! collapses to 1.2e-5 and λ saturates at 0.999999 on every prompt.
//! K itself stays healthy (range²-normalized).
//!
//! New OPT-IN query params on both dials:
//!
//! - `fields=v0..v383` · `fields=a,b,c` · `fields=<vector_field>` —
//!   statistics over ONLY the named scalar family (wave-1 `..` range
//!   sugar, inclusive both ends) or over one `Value::Vector` fiber
//!   field, per-component.
//! - `locus=<field>=<value>` `[&k=<n>]` — statistics over the k-nearest
//!   records to the locus record (cosine chord distance in the scoped
//!   vector space; `k` defaults to [`DEFAULT_LOCUS_K`] = 64, ties break
//!   by record iteration order).
//!
//! Composition: `fields` alone = whole bundle, vector-scoped; `locus`
//! alone = neighborhood over all numeric scalar fibers (the same field
//! population the whole-bundle formulas see); both = neighborhood +
//! vector-scoped (the loom's real need).
//!
//! **Precedence: `estimator=fixed` > `locus`/`fields` > default.** The
//! fixed estimator is Marcella's production escape hatch; when it is
//! supplied the scoping params are ignored entirely and the response is
//! byte-identical to `estimator=fixed` alone.
//!
//! ## Same formulas, scoped inputs — the math is not forked
//!
//! Scoped statistics are materialized as a transient in-memory
//! [`BundleStore`] whose fiber fields are exactly the scoped
//! per-component columns and whose records are exactly the scoped
//! population. Per-component accumulation therefore runs through
//! [`BundleStore::insert`]'s own Welford update — the identical code
//! path the whole-bundle statistics were built by — and every dial
//! quantity is then computed by the same public fns the whole-bundle
//! path uses:
//!
//! - K: [`curvature::scalar_curvature`] (mean of var/range² over the
//!   scoped columns),
//! - l_c: [`curvature::horizon_with`] with the default estimator
//!   config (SpectralGap primary, Welford-radius fallback) evaluated
//!   ON the scoped store — sqrt of mean per-component variance, the
//!   euclidean/per-component dispersion convention,
//! - s_max: `horizon_with`'s own `τ/(K·l_c)`,
//! - λ₁: [`spectral::spectral_gap`] of the scoped store (a fiber-
//!   statistics scope carries no index graph, so this is 0.0 and the
//!   Welford fallback engages — the same path every real embedding
//!   bundle takes),
//! - λ-budget: [`curvature::lambda_budget_for_bundle`] of the scoped
//!   store (`1 − τ/(K·D²)`, D = scoped Welford radius).
//!
//! Absent params → the exact pre-change code path (fence-tested
//! byte-for-byte in `tests/locus_dials.rs`).
//!
//! ## Ask 3 — WINDOWED_COHERENCE one-shot
//!
//! `POST /v1/bundles/{name}/windowed_coherence` composes, server-side,
//! what Marcella's laminar gate previously did with per-segment
//! `TRANSPORT_ROTATION` (GQL) + `POST /local_holonomy` round-trips of
//! dim×dim frames: see [`windowed_coherence_report`].

use std::collections::HashMap;

use crate::bundle::BundleStore;
use crate::curvature::{self, LengthScaleEstimator};
use crate::mmap_bundle::BundleRef;
use crate::spectral;
use crate::types::{BundleSchema, FieldDef, FieldType, Record, Value};

/// Default locus-neighborhood size when `locus=` is supplied without
/// `k=`. 64 matches SAMPLE_TRANSPORT-scale neighborhood ergonomics and
/// is large enough for stable Welford statistics, small enough to stay
/// local on Marcella's ~2k-record bundles.
pub const DEFAULT_LOCUS_K: usize = 64;

/// Typed dial errors. The HTTP layer maps `BadRequest` → 400 and
/// `NotFound` → 404, both as the flat `{"error": …}` envelope; message
/// text names the guilty parameter / field / key value (wave-1 loud-
/// typo contract).
#[derive(Debug, Clone, PartialEq)]
pub enum DialError {
    /// Malformed or invalid parameter (400-class).
    BadRequest(String),
    /// A named record / key was not found (404-class).
    NotFound(String),
}

impl std::fmt::Display for DialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialError::BadRequest(m) | DialError::NotFound(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for DialError {}

// ── report structs (wire shapes) ─────────────────────────────────────
//
// Field ORDER is the wire contract: the absent-params serialization
// must stay byte-identical to the pre-change handler structs, so the
// original fields come first, in the original order, and the two new
// fields are appended with `skip_serializing_if` (absent when None).

/// Response for `GET /v1/bundles/{name}/capacity`.
/// Davis capacity C = τ/K (Theorem 8.1 — Cognitive Geometry
/// Correspondence).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CapacityReport {
    /// Davis capacity C = τ/K. How many distinct interpretations the
    /// system can maintain simultaneously at this curvature level.
    pub capacity: f64,
    /// Scalar curvature K (whole-bundle, or scoped when `scope` is
    /// present).
    pub k: f64,
    /// Tolerance budget τ used to compute C.
    pub tau: f64,
    /// Confidence ∈ (0,1]: 1/(1+K).
    pub confidence: f64,
    /// Qualitative regime: "flat" (K≈0), "low" (C>10), "moderate",
    /// "high" (C<1, overloaded), or "critical" (C≈0, K→∞).
    pub regime: &'static str,
    /// Human-readable interpretation for builders.
    pub interpretation: String,
    /// Davis-Conjecture λ-budget recomputed from the SCOPED statistics
    /// (1 − τ/(K·D²), D = scoped Welford radius). Present only when a
    /// scoping param was supplied — the desaturation receipt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lambda_budget: Option<f64>,
    /// Echo of the scope that produced these statistics. Present only
    /// when a scoping param was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ScopeEcho>,
}

/// Response for `GET /v1/bundles/{name}/horizon`.
/// Holonomy horizon s_max = τ/(K·ℓ_c) (Definition 5.1 — Cognitive
/// Geometry Correspondence).
#[derive(Debug, Clone, serde::Serialize)]
pub struct HorizonReport {
    /// s_max = τ/(K·ℓ_c). Beyond this many positions, individual
    /// contributions to the accumulated frame rotation are
    /// irrecoverable.
    pub s_max: f64,
    /// Scalar curvature K (whole-bundle, or scoped when `scope` is
    /// present).
    pub k: f64,
    /// Tolerance budget τ.
    pub tau: f64,
    /// Correlation length ℓ_c actually used (from the estimator that
    /// won).
    pub l_c: f64,
    /// Spectral gap λ₁ (of the scoped population when `scope` is
    /// present; a fiber-statistics scope has no index graph so scoped
    /// λ₁ is 0.0 and the Welford fallback engages).
    pub lambda1: f64,
    /// Which estimator produced `l_c`.
    pub estimator_used: LengthScaleEstimator,
    /// True iff the primary estimator was degenerate and the fallback
    /// fired.
    pub fallback_engaged: bool,
    /// Human-readable interpretation.
    pub interpretation: String,
    /// Davis-Conjecture λ-budget recomputed from the SCOPED statistics.
    /// Present only when a scoping param was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lambda_budget: Option<f64>,
    /// Echo of the scope that produced these statistics. Present only
    /// when a scoping param was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ScopeEcho>,
}

/// Names the statistics population a scoped dial response was computed
/// over (D2 probe contract: "the response naming the scope").
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScopeEcho {
    /// The raw `fields=` spec as supplied (range sugar unexpanded),
    /// absent for locus-alone scoping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<String>,
    /// The locus neighborhood, absent for fields-alone scoping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locus: Option<LocusEcho>,
    /// Records in the scoped population (neighborhood size, or the
    /// whole bundle for fields-alone).
    pub n_records: usize,
    /// Per-component statistics columns in scope (a dims-d Vector
    /// field contributes d).
    pub n_fields: usize,
}

/// Locus echo inside [`ScopeEcho`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct LocusEcho {
    /// The locus field name.
    pub field: String,
    /// The locus key value as supplied.
    pub value: String,
    /// The requested neighborhood size (default [`DEFAULT_LOCUS_K`]);
    /// `n_records` on the parent echo is the realized size.
    pub k: usize,
}

// ── scope parsing ────────────────────────────────────────────────────

/// One scoped statistics column source: a scalar numeric/timestamp
/// fiber field, or a whole `Value::Vector` fiber field exploded
/// per-component.
#[derive(Debug, Clone)]
enum ScopedField {
    Scalar(String),
    Vector { name: String, dims: usize },
}

impl ScopedField {
    /// Number of per-component statistics columns this source expands
    /// to.
    fn n_columns(&self) -> usize {
        match self {
            ScopedField::Scalar(_) => 1,
            ScopedField::Vector { dims, .. } => *dims,
        }
    }
}

/// The parsed opt-in scoping parameters.
#[derive(Debug, Clone)]
struct DialScope {
    /// Raw `fields=` spec (for the echo) + parsed sources. None →
    /// locus-alone mode (all numeric scalar fibers).
    fields: Option<(String, Vec<ScopedField>)>,
    /// `locus=<field>=<value>`.
    locus: Option<(String, String)>,
    /// Neighborhood size for locus scoping.
    k: usize,
}

impl DialScope {
    /// Parse the opt-in params. `Ok(None)` when none of
    /// `fields`/`locus`/`k` are present — the caller MUST then take the
    /// exact pre-change path (the defaults fence).
    fn from_params(
        params: &HashMap<String, String>,
        schema: &BundleSchema,
    ) -> Result<Option<DialScope>, DialError> {
        let fields_raw = params.get("fields");
        let locus_raw = params.get("locus");
        let k_raw = params.get("k");
        if fields_raw.is_none() && locus_raw.is_none() && k_raw.is_none() {
            return Ok(None);
        }

        let fields = match fields_raw {
            Some(spec) => Some((spec.clone(), parse_fields_spec(spec, schema, "fields")?)),
            None => None,
        };

        let locus = match locus_raw {
            Some(raw) => {
                let (field, value) = raw.split_once('=').ok_or_else(|| {
                    DialError::BadRequest(format!(
                        "locus must be <field>=<value>; got '{raw}'"
                    ))
                })?;
                let field = field.trim();
                if field.is_empty() {
                    return Err(DialError::BadRequest(format!(
                        "locus must be <field>=<value>; got '{raw}'"
                    )));
                }
                let known = schema
                    .base_fields
                    .iter()
                    .chain(schema.fiber_fields.iter())
                    .any(|fd| fd.name == field);
                if !known {
                    return Err(DialError::BadRequest(format!(
                        "locus: '{field}' is not a field of this bundle"
                    )));
                }
                Some((field.to_string(), value.to_string()))
            }
            None => None,
        };

        let k = match k_raw {
            Some(raw) => {
                if locus.is_none() {
                    return Err(DialError::BadRequest(
                        "k requires locus (k is the locus neighborhood size)".to_string(),
                    ));
                }
                let k: usize = raw.parse().map_err(|_| {
                    DialError::BadRequest(format!("k must be a positive integer; got '{raw}'"))
                })?;
                if k == 0 {
                    return Err(DialError::BadRequest(
                        "k must be a positive integer; got '0'".to_string(),
                    ));
                }
                k
            }
            None => DEFAULT_LOCUS_K,
        };

        Ok(Some(DialScope { fields, locus, k }))
    }
}

/// Parse + validate a fields/fiber spec: comma-separated fiber names
/// with wave-1 `lo..hi` range sugar, OR exactly one `Value::Vector`
/// fiber field name. Validation mirrors wave-1's VECTOR clause: every
/// named field must be a numeric/timestamp scalar fiber field, typos
/// are loud. `label` names the parameter in error messages ("fields"
/// on the GET dials, "fiber" on windowed_coherence).
fn parse_fields_spec(
    spec: &str,
    schema: &BundleSchema,
    label: &str,
) -> Result<Vec<ScopedField>, DialError> {
    let mut names: Vec<String> = Vec::new();
    for token in spec.split(',') {
        let token = token.trim();
        if token.is_empty() {
            return Err(DialError::BadRequest(format!(
                "{label}: empty entry in '{spec}' — expected comma-separated fiber \
                 fields with optional lo..hi range sugar"
            )));
        }
        if let Some((lo, hi)) = token.split_once("..") {
            let expanded = crate::parser::expand_field_range(lo, hi)
                .map_err(DialError::BadRequest)?;
            names.extend(expanded);
        } else {
            names.push(token.to_string());
        }
    }
    if names.is_empty() {
        return Err(DialError::BadRequest(format!(
            "{label}: at least one fiber field is required"
        )));
    }

    // A single named Value::Vector fiber field scopes per-component.
    if names.len() == 1 {
        if let Some(fd) = schema.fiber_fields.iter().find(|fd| fd.name == names[0]) {
            if let FieldType::Vector { dims } = fd.field_type {
                return Ok(vec![ScopedField::Vector {
                    name: names[0].clone(),
                    dims,
                }]);
            }
        }
    }

    names
        .into_iter()
        .map(|f| {
            let Some(fd) = schema.fiber_fields.iter().find(|fd| fd.name == f) else {
                return Err(DialError::BadRequest(format!(
                    "{label}: '{f}' is not a fiber field of this bundle"
                )));
            };
            match fd.field_type {
                FieldType::Numeric | FieldType::Timestamp => Ok(ScopedField::Scalar(f)),
                FieldType::Vector { .. } => Err(DialError::BadRequest(format!(
                    "{label}: '{f}' is a Vector fiber — a Vector field scopes alone \
                     ({label}={f})"
                ))),
                ref other => Err(DialError::BadRequest(format!(
                    "{label}: field '{f}' is not numeric (type {other:?}) — the scope \
                     assembles scalar numeric fibers"
                ))),
            }
        })
        .collect()
}

/// Locus-alone mode: all numeric/timestamp SCALAR fiber fields, schema
/// order — the same field population the whole-bundle Welford
/// statistics are built from (Vector fields carry no FieldStats on the
/// main path, so they are not part of the scoped population either).
fn all_numeric_scalar_fibers(schema: &BundleSchema) -> Vec<ScopedField> {
    schema
        .fiber_fields
        .iter()
        .filter(|fd| matches!(fd.field_type, FieldType::Numeric | FieldType::Timestamp))
        .map(|fd| ScopedField::Scalar(fd.name.clone()))
        .collect()
}

// ── scoped projection + distance ─────────────────────────────────────

/// Project a record onto the scoped vector space (missing / non-numeric
/// components → 0.0; Vector fields contribute `dims` components, padded
/// with 0.0 when the stored vector is short). Mirrors
/// `geometry::sample_transport::extract_fiber`'s conventions; kept here
/// unconditionally because `geometry` is kahler-gated while the dials
/// are not (a kahler-gated unit test pins the parity).
fn scope_projection(rec: &Record, scoped: &[ScopedField]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for sf in scoped {
        match sf {
            ScopedField::Scalar(name) => {
                out.push(rec.get(name.as_str()).and_then(|v| v.as_f64()).unwrap_or(0.0));
            }
            ScopedField::Vector { name, dims } => match rec.get(name.as_str()) {
                Some(Value::Vector(v)) => {
                    for i in 0..*dims {
                        out.push(v.get(i).copied().unwrap_or(0.0));
                    }
                }
                _ => out.extend(std::iter::repeat(0.0).take(*dims)),
            },
        }
    }
    out
}

/// Squared chord distance between two fiber projections on the unit
/// sphere (half-angle formula): `d² = (1 − cos θ)/2`; degenerate
/// (near-zero-norm) vectors → 1.0 (maximum distance). Same formula as
/// `geometry::sample_transport::fiber_d_sq` (kahler-gated there;
/// parity-pinned by a unit test below when that feature is on).
fn chord_d_sq(p_src: &[f64], p: &[f64]) -> f64 {
    let n = p_src.len().min(p.len());
    let dot: f64 = p_src[..n].iter().zip(p[..n].iter()).map(|(a, b)| a * b).sum();
    let norm_s = p_src[..n].iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_p = p[..n].iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_s < 1e-12 || norm_p < 1e-12 {
        return 1.0;
    }
    let cos_t = (dot / (norm_s * norm_p)).clamp(-1.0, 1.0);
    (1.0 - cos_t) / 2.0
}

/// Does this record match the locus key? Text keys compare as text;
/// numeric-family values (Integer / Float / Timestamp) compare through
/// `as_f64` against the parsed param.
fn locus_matches(rec: &Record, field: &str, raw: &str) -> bool {
    let Some(v) = rec.get(field) else {
        return false;
    };
    match v {
        Value::Text(t) => t == raw,
        other => match (other.as_f64(), raw.parse::<f64>()) {
            (Some(a), Ok(b)) => a == b,
            _ => false,
        },
    }
}

// ── scoped statistics population ─────────────────────────────────────

/// Materialize the scoped statistics population as a transient
/// in-memory [`BundleStore`]: fiber fields = the scoped per-component
/// columns, records = the scoped population. Per-component values run
/// through [`BundleStore::insert`]'s own Welford update — the identical
/// accumulation the whole-bundle statistics were built by — so
/// `scalar_curvature` / `horizon_with` / `lambda_budget_for_bundle` /
/// `spectral_gap` evaluated on this store ARE the whole-bundle formulas
/// applied to the scoped inputs.
///
/// Missing / non-numeric components are simply absent on that record
/// (skipped by the per-field Welford update, exactly as on the main
/// path).
fn build_scoped_store(
    records: &[&Record],
    scoped: &[ScopedField],
    source_name: &str,
) -> BundleStore {
    let mut schema = BundleSchema::new(&format!("{source_name}::scoped"))
        .base(FieldDef::numeric("__row"));
    let mut columns: Vec<(String, ColumnSource)> = Vec::new();
    for sf in scoped {
        match sf {
            ScopedField::Scalar(name) => {
                columns.push((name.clone(), ColumnSource::Scalar(name.clone())));
            }
            ScopedField::Vector { name, dims } => {
                for i in 0..*dims {
                    columns.push((
                        format!("{name}[{i}]"),
                        ColumnSource::VectorComponent(name.clone(), i),
                    ));
                }
            }
        }
    }
    for (col, _) in &columns {
        schema = schema.fiber(FieldDef::numeric(col));
    }
    let mut mini = BundleStore::new(schema);
    for (row, rec) in records.iter().enumerate() {
        let mut r = Record::new();
        r.insert("__row".to_string(), Value::Integer(row as i64));
        for (col, src) in &columns {
            let v = match src {
                ColumnSource::Scalar(name) => {
                    rec.get(name.as_str()).and_then(|v| v.as_f64())
                }
                ColumnSource::VectorComponent(name, i) => match rec.get(name.as_str()) {
                    Some(Value::Vector(v)) => v.get(*i).copied(),
                    _ => None,
                },
            };
            if let Some(v) = v {
                r.insert(col.clone(), Value::Float(v));
            }
        }
        mini.insert(&r);
    }
    mini
}

enum ColumnSource {
    Scalar(String),
    VectorComponent(String, usize),
}

/// Resolve the scoped population + build the scoped store + echo.
fn scoped_population(
    store: &BundleRef,
    scope: &DialScope,
) -> Result<(BundleStore, ScopeEcho), DialError> {
    let schema = store.schema();
    let scoped_fields: Vec<ScopedField> = match &scope.fields {
        Some((_, parsed)) => parsed.clone(),
        None => all_numeric_scalar_fibers(schema),
    };
    let n_fields: usize = scoped_fields.iter().map(|sf| sf.n_columns()).sum();

    let records: Vec<Record> = store.records().collect();

    let population: Vec<&Record> = match &scope.locus {
        Some((field, value)) => {
            let locus_rec = records
                .iter()
                .find(|r| locus_matches(r, field, value))
                .ok_or_else(|| {
                    DialError::NotFound(format!(
                        "locus: no record at {field}='{value}' in '{}'",
                        schema.name
                    ))
                })?;
            let locus_p = scope_projection(locus_rec, &scoped_fields);
            // Deterministic k-NN: ascending squared chord distance in
            // the scoped space, ties broken by record iteration order.
            // (In-process deterministic; on heap Hashed-storage bundles
            // records() iterates HashMap keys, process-seeded, so
            // EXACT-distance ties may resolve differently across
            // restarts — measure-zero on real embedding data; mmap
            // iteration order is stable.)
            let mut scored: Vec<(f64, usize)> = records
                .iter()
                .enumerate()
                .map(|(i, r)| (chord_d_sq(&locus_p, &scope_projection(r, &scoped_fields)), i))
                .collect();
            scored.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.cmp(&b.1))
            });
            scored.truncate(scope.k);
            scored.iter().map(|&(_, i)| &records[i]).collect()
        }
        None => records.iter().collect(),
    };

    let mini = build_scoped_store(&population, &scoped_fields, &schema.name);
    let echo = ScopeEcho {
        fields: scope.fields.as_ref().map(|(raw, _)| raw.clone()),
        locus: scope.locus.as_ref().map(|(field, value)| LocusEcho {
            field: field.clone(),
            value: value.clone(),
            k: scope.k,
        }),
        n_records: population.len(),
        n_fields,
    };
    Ok((mini, echo))
}

// ── estimator param (shared with the pre-change path) ────────────────

/// Parse the `estimator` / `fixed_value` params. Error strings are the
/// pre-change handler's exact bytes (they are part of the wire fence).
fn parse_estimator(
    params: &HashMap<String, String>,
) -> Result<LengthScaleEstimator, DialError> {
    match params.get("estimator").map(|s| s.as_str()) {
        Some("welford_radius") => Ok(curvature::LengthScaleEstimator::WelfordRadius),
        Some("fixed") => {
            let v: f64 = params
                .get("fixed_value")
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| {
                    DialError::BadRequest(
                        "estimator=fixed requires &fixed_value=<f64>".to_string(),
                    )
                })?;
            Ok(curvature::LengthScaleEstimator::Fixed(v))
        }
        Some("spectral_gap") | None => Ok(curvature::LengthScaleEstimator::SpectralGap),
        Some(other) => Err(DialError::BadRequest(format!(
            "estimator must be one of: spectral_gap, welford_radius, fixed; got {other}"
        ))),
    }
}

/// The pre-change horizon interpretation string, verbatim.
fn horizon_interpretation(
    s_max: f64,
    k: f64,
    l_c: f64,
    tau: f64,
    fallback_engaged: bool,
) -> String {
    if s_max.is_infinite() {
        "K ≈ 0: infinite horizon. Flat geometry — all positions remain \
         individually attributable indefinitely."
            .to_string()
    } else {
        let fallback_note = if fallback_engaged {
            " [fallback estimator engaged; primary was degenerate]"
        } else {
            ""
        };
        format!(
            "s_max = {s_max:.1}: coherent attribution extends {s_max:.0} positions. \
             Beyond this, accumulated frame rotation cannot be decomposed into \
             individual contributions. (K={k:.4}, ℓ_c={l_c:.4}, τ={tau}){fallback_note}"
        )
    }
}

/// The pre-change capacity regime + interpretation, verbatim.
fn capacity_regime(k: f64, c: f64) -> (&'static str, String) {
    if k < f64::EPSILON {
        ("flat", format!("K ≈ 0: flat space, infinite capacity. No curvature barriers — every query resolves cleanly."))
    } else if c > 10.0 {
        ("low", format!("C = {c:.2}: low-curvature region. Room for {c:.0} distinct interpretations per unit τ. Synthesis is reliable."))
    } else if c >= 1.0 {
        ("moderate", format!("C = {c:.2}: moderate curvature. The system can hold {c:.1} interpretations simultaneously. Watch for ambiguity."))
    } else if c > 0.1 {
        ("high", format!("C = {c:.3}: high curvature — fewer than one interpretation per unit τ. Ambiguity detection recommended before synthesis."))
    } else {
        ("critical", format!("C = {c:.4}: near-critical curvature. The system cannot reliably distinguish interpretations. Query is at a topological fork."))
    }
}

// ── the dial report builders (HTTP handlers are thin glue) ───────────

/// Build the HORIZON report. Absent scoping params → the exact
/// pre-change computation (byte-fenced); `estimator=fixed` wins over
/// `locus`/`fields` (escape-hatch precedence); otherwise the scoped
/// path recomputes K / l_c / s_max / λ₁ / λ-budget from the scoped
/// statistics through the same public formula fns.
pub fn horizon_report(
    store: &BundleRef,
    params: &HashMap<String, String>,
) -> Result<HorizonReport, DialError> {
    let tau: f64 = params.get("tau").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let estimator = parse_estimator(params)?;

    // Precedence: fixed > locus/fields > default. The escape hatch
    // ignores the scoping params entirely (response byte-identical to
    // fixed-alone).
    let scope = if matches!(estimator, LengthScaleEstimator::Fixed(_)) {
        None
    } else {
        DialScope::from_params(params, store.schema())?
    };

    if let Some(scope) = scope {
        // ── scoped path: same formulas, scoped inputs ───────────────
        let (mini, echo) = scoped_population(store, &scope)?;
        let k = curvature::scalar_curvature(&mini);
        let lambda1 = spectral::spectral_gap(&mini);
        let cfg = curvature::HorizonConfig {
            estimator,
            ..curvature::HorizonConfig::default()
        };
        let res = curvature::horizon_with(tau, k, &mini, lambda1, &cfg);
        let lambda_budget = curvature::lambda_budget_for_bundle(&mini);
        let interpretation =
            horizon_interpretation(res.s_max, k, res.l_c, tau, res.fallback_engaged);
        return Ok(HorizonReport {
            s_max: res.s_max,
            k,
            tau,
            l_c: res.l_c,
            lambda1,
            estimator_used: res.estimator_used,
            fallback_engaged: res.fallback_engaged,
            interpretation,
            lambda_budget: Some(lambda_budget),
            scope: Some(echo),
        });
    }

    // ── default path — pre-change behavior, byte-fenced ─────────────
    let k = store.scalar_curvature();
    let lambda1 = store.as_heap().map(spectral::spectral_gap).unwrap_or(0.0);
    let cfg = curvature::HorizonConfig {
        estimator,
        ..curvature::HorizonConfig::default()
    };

    // The calibrated path needs a heap store for the Welford radius
    // pass. If we only have mmap+overlay, fall back to the scalar
    // shim (same behavior as before the calibrated path existed —
    // documented as a "degenerate when λ₁=0" limitation).
    let (s_max, l_c, estimator_used, fallback_engaged) = if let Some(heap) = store.as_heap() {
        let res = curvature::horizon_with(tau, k, heap, lambda1, &cfg);
        (res.s_max, res.l_c, res.estimator_used, res.fallback_engaged)
    } else {
        let l_c_shim = if lambda1 > f64::EPSILON { 1.0 / lambda1.sqrt() } else { 1.0 };
        let s = curvature::horizon(tau, k, lambda1);
        (s, l_c_shim, curvature::LengthScaleEstimator::SpectralGap, lambda1 < f64::EPSILON)
    };

    let interpretation = horizon_interpretation(s_max, k, l_c, tau, fallback_engaged);
    Ok(HorizonReport {
        s_max,
        k,
        tau,
        l_c,
        lambda1,
        estimator_used,
        fallback_engaged,
        interpretation,
        lambda_budget: None,
        scope: None,
    })
}

/// Build the CAPACITY report. Absent scoping params → the exact
/// pre-change computation (byte-fenced); otherwise C / confidence /
/// regime / λ-budget recompute from the scoped K through the same
/// public formula fns. (Capacity has no estimator param, so there is
/// no fixed-precedence interaction here.)
pub fn capacity_report(
    store: &BundleRef,
    params: &HashMap<String, String>,
) -> Result<CapacityReport, DialError> {
    let tau: f64 = params.get("tau").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let scope = DialScope::from_params(params, store.schema())?;

    if let Some(scope) = scope {
        let (mini, echo) = scoped_population(store, &scope)?;
        let k = curvature::scalar_curvature(&mini);
        let c = curvature::capacity(tau, k);
        let conf = curvature::confidence(k);
        let lambda_budget = curvature::lambda_budget_for_bundle(&mini);
        let (regime, interpretation) = capacity_regime(k, c);
        return Ok(CapacityReport {
            capacity: c,
            k,
            tau,
            confidence: conf,
            regime,
            interpretation,
            lambda_budget: Some(lambda_budget),
            scope: Some(echo),
        });
    }

    let k = store.scalar_curvature();
    let c = curvature::capacity(tau, k);
    let conf = curvature::confidence(k);
    let (regime, interpretation) = capacity_regime(k, c);
    Ok(CapacityReport {
        capacity: c,
        k,
        tau,
        confidence: conf,
        regime,
        interpretation,
        lambda_budget: None,
        scope: None,
    })
}

// ═════════════════════════════════════════════════════════════════════
// Ask 3 — WINDOWED_COHERENCE one-shot
// ═════════════════════════════════════════════════════════════════════

/// Default laminar threshold on the coherence signal: Marcella's
/// `COHERENCE_CONFIDENT = 0.91` (fiber_lm/voice_math/
/// coherence_forecast.py — the constant her loom's accept gate uses;
/// 0.85 is her accept-with-hedge tier, and the server's older
/// local_holonomy interpretation prose uses 0.9). Thresholds apply to
/// COHERENCE (= 1 − defect/(2√dim), dim-independent), not to the raw
/// defect; override per request via the `threshold` body field
/// (valid range (0, 1]).
///
/// **Dim-dependent floor (review follow-up):** a single segment
/// (window=2) is one plane rotation, capping the defect at 2√2, so a
/// w=2 window can be non-laminar at threshold t only when
/// dim ≤ 2/(1−t)² (≈247 at t=0.91). At dim=384 the w=2 coherence
/// floor is 1 − 2√2/(2√384) ≈ 0.9278 — unconditionally laminar at the
/// default no matter how violent the segment turn. w−1 segments cap
/// the defect at 2√(2(w−1)) (w=3 floor at dim=384 ≈ 0.898). For
/// per-segment discrimination at high dim: use w ≥ 3, raise the
/// threshold, or gate on the raw `holonomy_defect` in the response.
pub const DEFAULT_LAMINAR_THRESHOLD: f64 = 0.91;

/// The TRANSPORT_ROTATION Rodrigues construction, moved VERBATIM from
/// the GQL executor (`Statement::TransportRotation` in gigi_stream.rs)
/// so the verb and the windowed_coherence one-shot execute the same fn
/// body (parity anchor A3-4):
///
///   R = I + (cos θ − 1)(e1 e1ᵀ + e2 e2ᵀ) + sin θ (e2 e1ᵀ − e1 e2ᵀ)
///   e1 = u/‖u‖, e2 = (v − ⟨v,e1⟩e1)/‖…‖
///
/// Returns the flat row-major n×n matrix, n = min(len(u), len(v)).
/// Identity when ‖u‖ < 1e-12, |θ| < 1e-12, or e2 degenerates (u and v
/// collinear — including u == v exactly, which is why identity paths
/// carry defect 0.0 exactly).
pub fn transport_rotation_matrix(u: &[f64], v: &[f64], angle: f64) -> Vec<f64> {
    let n = u.len().min(v.len());
    let nu: f64 = u.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mut matrix = vec![0.0f64; n * n];
    // Identity by default
    for i in 0..n {
        matrix[i * n + i] = 1.0;
    }

    if nu >= 1e-12 && angle.abs() >= 1e-12 {
        let e1: Vec<f64> = u.iter().map(|x| x / nu).collect();
        let dot_v_e1: f64 = v.iter().zip(&e1).map(|(a, b)| a * b).sum();
        let e2_unnorm: Vec<f64> = v
            .iter()
            .zip(&e1)
            .map(|(vi, ei)| vi - dot_v_e1 * ei)
            .collect();
        let ne: f64 = e2_unnorm.iter().map(|x| x * x).sum::<f64>().sqrt();
        if ne >= 1e-12 {
            let e2: Vec<f64> = e2_unnorm.iter().map(|x| x / ne).collect();
            let cos_t = angle.cos();
            let sin_t = angle.sin();
            let coef_p = cos_t - 1.0;
            // R = I + coef_p · (e1 e1^T + e2 e2^T) + sin_t · (e2 e1^T − e1 e2^T)
            for i in 0..n {
                for j in 0..n {
                    let p_ij = e1[i] * e1[j] + e2[i] * e2[j];
                    let a_ij = e2[i] * e1[j] - e1[i] * e2[j];
                    matrix[i * n + j] += coef_p * p_ij + sin_t * a_ij;
                }
            }
        }
    }
    matrix
}

/// The data-derived per-segment transport angle:
/// θ = arccos(clamp(cos_sim(u, v), −1, 1)) — the minimal rotation
/// carrying u to v in their spanned plane. This is the pinned
/// DEVIATION from the GQL TRANSPORT_ROTATION verb, whose angle is
/// CALLER-supplied: the one-shot has no caller angle, so it reads the
/// angle off the segment data itself. Degenerate (near-zero-norm)
/// endpoints → 0.0 (identity transport — the verb's own guard
/// behavior).
///
/// **Exactly-antipodal endpoints (v = −u)** derive θ = π but transport
/// as identity anyway: v is collinear with e1 so the Rodrigues e2
/// degenerates — defect 0, coherence 1.0, DISCONTINUOUS vs
/// near-antipodal (defect → 2√2). Inherited verb convention, pinned by
/// test; measure-zero on real float embeddings. Gate on the segment
/// cosine if exact-reversal detection matters.
fn derived_transport_angle(u: &[f64], v: &[f64]) -> f64 {
    let n = u.len().min(v.len());
    let dot: f64 = u[..n].iter().zip(v[..n].iter()).map(|(a, b)| a * b).sum();
    let nu: f64 = u[..n].iter().map(|x| x * x).sum::<f64>().sqrt();
    let nv: f64 = v[..n].iter().map(|x| x * x).sum::<f64>().sqrt();
    if nu < 1e-12 || nv < 1e-12 {
        return 0.0;
    }
    (dot / (nu * nv)).clamp(-1.0, 1.0).acos()
}

/// Row-major dim×dim matrix product `a · b`.
fn matmul(a: &[f64], b: &[f64], dim: usize) -> Vec<f64> {
    let mut out = vec![0.0_f64; dim * dim];
    for i in 0..dim {
        for j in 0..dim {
            let mut acc = 0.0_f64;
            for k in 0..dim {
                acc += a[i * dim + k] * b[k * dim + j];
            }
            out[i * dim + j] = acc;
        }
    }
    out
}

/// The whole-bundle Davis-Conjecture λ-budget ride-along, matching the
/// convention every lambda_budget-carrying response uses (mirrors the
/// binary's `lambda_budget_for_bundle_ref`): heap bundles go through
/// [`curvature::lambda_budget_for_bundle`]; overlay bundles fall back
/// to the CurvatureStats mean-K with D = 1.0, NaN-coalesced to the
/// saturated default 1.0.
fn lambda_budget_envelope(store: &BundleRef) -> f64 {
    match store.as_heap() {
        Some(heap) => curvature::lambda_budget_for_bundle(heap),
        None => {
            let k = store.curvature_stats().mean();
            let raw = curvature::lambda_budget(k, 1.0, 1.0);
            if raw.is_nan() {
                1.0
            } else {
                raw
            }
        }
    }
}

/// Request body for `POST /v1/bundles/{name}/windowed_coherence`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WindowedCoherenceRequest {
    /// Ordered record key values (strings for text keys, numbers for
    /// numeric keys) — the path to transport along.
    pub path: Vec<serde_json::Value>,
    /// The base field the path keys address.
    pub key_field: String,
    /// Window size in RECORDS (a window composes window−1 segment
    /// rotations). Valid range: [2, len(path)].
    pub window: usize,
    /// Fiber scope — same grammar as ask 4's `fields=`: scalar family
    /// entries with `lo..hi` range sugar, or exactly one Value::Vector
    /// fiber field name.
    pub fiber: Vec<String>,
    /// Laminar threshold on coherence, in (0, 1]. Default
    /// [`DEFAULT_LAMINAR_THRESHOLD`] (0.91, Marcella's
    /// COHERENCE_CONFIDENT accept gate).
    #[serde(default)]
    pub threshold: Option<f64>,
}

/// One sliding window's verdict.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoherenceWindow {
    /// Index into `path` where this window starts.
    pub start_index: usize,
    /// The window's key values, echoed (`path[start..start+window]`).
    pub keys: Vec<serde_json::Value>,
    /// `‖R_window − I‖_F` where R_window composes the window's
    /// window−1 segment transport rotations. Range [0, 2√dim].
    pub holonomy_defect: f64,
    /// `1 − holonomy_defect/(2√dim)`, clamped to [0, 1] — the
    /// normalized coherence signal A_t the threshold applies to.
    pub coherence: f64,
    /// `coherence >= threshold_used`.
    pub laminar: bool,
}

/// Response for `POST /v1/bundles/{name}/windowed_coherence`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WindowedCoherenceReport {
    /// One row per sliding window (stride 1).
    pub windows: Vec<CoherenceWindow>,
    /// `len(path) − window + 1`.
    pub n_windows: usize,
    /// True iff every window is laminar.
    pub laminar_all: bool,
    /// The threshold the verdicts were computed against.
    pub threshold_used: f64,
    /// Dimension of the scoped fiber space (rotation matrices are
    /// dim×dim server-side — the round-trip the one-shot removes).
    pub dim: usize,
    /// Window size echoed.
    pub window: usize,
    /// Bundle name echoed.
    pub bundle: String,
    /// Standard whole-bundle Davis-Conjecture λ-budget ride-along.
    pub lambda_budget: f64,
}

/// Does this record match one path key value? Text keys compare as
/// text, numeric-family values through `as_f64`.
fn path_key_matches(rec: &Record, field: &str, key: &serde_json::Value) -> bool {
    let Some(v) = rec.get(field) else {
        return false;
    };
    match (v, key) {
        (Value::Text(t), serde_json::Value::String(s)) => t == s,
        (other, serde_json::Value::Number(n)) => {
            match (other.as_f64(), n.as_f64()) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        _ => false,
    }
}

/// WINDOWED_COHERENCE — one server-side call composing the SAME two
/// surfaces Marcella's laminar gate previously round-tripped per
/// segment (GQL TRANSPORT_ROTATION for the segment rotation, then
/// POST /local_holonomy for the windowed defect):
///
/// 1. project each path record onto the scoped fiber space,
/// 2. per segment i: θᵢ = [`derived_transport_angle`], R_seg(i) =
///    [`transport_rotation_matrix`] (the verb's exact fn body),
/// 3. cumulative frames R_acc[0] = I, R_acc[i+1] = R_seg(i)·R_acc[i],
/// 4. per window starting at s:
///    [`curvature::local_holonomy`](R_acc[s+w−1], R_acc[s]) →
///    R_window = R_acc[s+w−1]·R_acc[s]ᵀ = R_seg(s+w−2)···R_seg(s),
///    holonomy_defect = ‖R_window − I‖_F,
///    coherence = 1 − defect/(2√dim),
/// 5. laminar = coherence ≥ threshold (default 0.91).
///
/// n_windows = len(path) − window + 1, sliding by 1.
///
/// Memory note: the cumulative frames held at once are O(len(path))
/// dim×dim matrices — for Marcella's 384-dim embeddings that is ~1.2MB
/// per path position, the exact frames she previously shipped over
/// HTTP per segment; keep paths to loom scale (tens of records).
pub fn windowed_coherence_report(
    store: &BundleRef,
    req: &WindowedCoherenceRequest,
) -> Result<WindowedCoherenceReport, DialError> {
    let schema = store.schema();
    let bundle = schema.name.clone();

    // ── validation (typed, loud) ────────────────────────────────────
    let threshold = match req.threshold {
        Some(t) => {
            if !(t.is_finite() && t > 0.0 && t <= 1.0) {
                return Err(DialError::BadRequest(format!(
                    "windowed_coherence: threshold must be a finite value in (0, 1]; got {t}"
                )));
            }
            t
        }
        None => DEFAULT_LAMINAR_THRESHOLD,
    };
    if req.window < 2 || req.window > req.path.len() {
        return Err(DialError::BadRequest(format!(
            "windowed_coherence: window must be in [2, len(path)] (got window={}, len(path)={})",
            req.window,
            req.path.len()
        )));
    }
    if !schema.base_fields.iter().any(|fd| fd.name == req.key_field) {
        return Err(DialError::NotFound(format!(
            "windowed_coherence: key_field '{}' is not a base field of bundle '{bundle}'",
            req.key_field
        )));
    }
    for key in &req.path {
        if !(key.is_string() || key.is_number()) {
            return Err(DialError::BadRequest(format!(
                "windowed_coherence: path values must be strings or numbers; got {key}"
            )));
        }
    }
    let scoped = parse_fields_spec(&req.fiber.join(","), schema, "fiber")?;
    let dim: usize = scoped.iter().map(|sf| sf.n_columns()).sum();

    // ── resolve the path records (first match per key value) ───────
    let records: Vec<Record> = store.records().collect();
    let mut vectors: Vec<Vec<f64>> = Vec::with_capacity(req.path.len());
    for key in &req.path {
        let rec = records
            .iter()
            .find(|r| path_key_matches(r, &req.key_field, key))
            .ok_or_else(|| {
                DialError::NotFound(format!(
                    "windowed_coherence: no record at {}={} in '{bundle}'",
                    req.key_field, key
                ))
            })?;
        vectors.push(scope_projection(rec, &scoped));
    }

    // ── cumulative transport frames ─────────────────────────────────
    let mut identity = vec![0.0_f64; dim * dim];
    for i in 0..dim {
        identity[i * dim + i] = 1.0;
    }
    let mut r_acc: Vec<Vec<f64>> = Vec::with_capacity(vectors.len());
    r_acc.push(identity);
    for s in 0..vectors.len() - 1 {
        let theta = derived_transport_angle(&vectors[s], &vectors[s + 1]);
        let r_seg = transport_rotation_matrix(&vectors[s], &vectors[s + 1], theta);
        let next = matmul(&r_seg, &r_acc[s], dim);
        r_acc.push(next);
    }

    // ── sliding windows through local_holonomy (the same fn the HTTP
    //    /local_holonomy surface calls) ────────────────────────────
    let n_windows = req.path.len() - req.window + 1;
    let mut windows = Vec::with_capacity(n_windows);
    let mut laminar_all = true;
    for start in 0..n_windows {
        let res = curvature::local_holonomy(
            &r_acc[start + req.window - 1],
            &r_acc[start],
            dim,
        )
        .map_err(|e| DialError::BadRequest(format!("windowed_coherence: {e}")))?;
        let laminar = res.coherence >= threshold;
        laminar_all &= laminar;
        windows.push(CoherenceWindow {
            start_index: start,
            keys: req.path[start..start + req.window].to_vec(),
            holonomy_defect: res.defect,
            coherence: res.coherence,
            laminar,
        });
    }

    Ok(WindowedCoherenceReport {
        windows,
        n_windows,
        laminar_all,
        threshold_used: threshold,
        dim,
        window: req.window,
        bundle,
        lambda_budget: lambda_budget_envelope(store),
    })
}

// ── unit tests (parity pins that need crate-private visibility) ──────

#[cfg(test)]
mod tests {
    use super::*;

    /// `chord_d_sq` must stay the same formula as the kahler-gated
    /// `geometry::sample_transport::fiber_d_sq` (kept separate only
    /// because `geometry` is feature-gated while the dials are not).
    #[cfg(feature = "kahler")]
    #[test]
    fn chord_d_sq_matches_sample_transport_fiber_d_sq() {
        let cases: [(&[f64], &[f64]); 5] = [
            (&[1.0, 0.0], &[0.0, 1.0]),
            (&[1.0, 0.0], &[1.0, 0.0]),
            (&[0.3, -0.4, 0.5], &[-0.2, 0.9, 0.1]),
            (&[1e-13, 0.0], &[1.0, 0.0]), // degenerate source
            (&[2.0, 1.0], &[-2.0, -1.0]), // antipodal
        ];
        for (a, b) in cases {
            let ours = chord_d_sq(a, b);
            let theirs = crate::geometry::sample_transport::fiber_d_sq(a, b);
            assert!(
                (ours - theirs).abs() < 1e-15,
                "chord_d_sq drifted from fiber_d_sq on {a:?} vs {b:?}: {ours} vs {theirs}"
            );
        }
    }

    /// The scoped store reproduces the main-path Welford statistics
    /// exactly: same values, same insert order → bit-identical
    /// variance/range per column.
    #[test]
    fn scoped_store_reproduces_main_path_welford() {
        let schema = BundleSchema::new("src")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x"))
            .fiber(FieldDef::numeric("y"));
        let mut store = BundleStore::new(schema);
        for i in 0..7 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("x".into(), Value::Float(0.1 * i as f64));
            r.insert("y".into(), Value::Float((i as f64).sin()));
            store.insert(&r);
        }
        let records: Vec<Record> = store.records().collect();
        let refs: Vec<&Record> = records.iter().collect();
        let scoped = vec![
            ScopedField::Scalar("x".to_string()),
            ScopedField::Scalar("y".to_string()),
        ];
        let mini = build_scoped_store(&refs, &scoped, "src");

        // Note: store.records() iteration order is not insertion order
        // in general, but Welford mean/variance/min/max over the same
        // VALUE MULTISET differ only by summation order; both sides here
        // consume the same records() order via the same insert loop, so
        // the K must agree to fp-jitter.
        let k_main = curvature::scalar_curvature(&store);
        let k_mini = curvature::scalar_curvature(&mini);
        assert!(
            (k_main - k_mini).abs() < 1e-12,
            "scoping to ALL fields must reproduce whole-bundle K: {k_main} vs {k_mini}"
        );
    }
}
