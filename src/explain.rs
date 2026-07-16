//! EXPLAIN SECTION … AT — shared executor for the per-record κ
//! decomposition, used by BOTH the embedded engine executor
//! (`parser::execute`) and the server read path
//! (`execute_gql_on_store_read` in `gigi_stream`). Before this module
//! the two arms carried identical hand-copied logic; the Marcella
//! EXPLAIN-family asks (2026-07-16) are implemented once, here.
//!
//! Error contract (ask 5a): a point read that matches nothing returns
//! `Err` carrying [`NOT_FOUND_PREFIX`] and naming the key and bundle.
//! The GQL HTTP layer strips the sentinel and answers 404 — mirroring
//! the REST section-fetch handler's
//! `Record '<id>' not found in bundle '<name>'` 404 — instead of the
//! blanket executor-Err→500 mapping that used to swallow it.

use crate::bundle::{compute_record_k, explain_record_k, FieldStats};
use crate::mmap_bundle::BundleRef;
use crate::parser::ExecResult;
use crate::types::{FieldDef, FieldType, Record, Value};
use std::collections::HashMap;

/// Typed miss sentinel. Executor error strings beginning with this
/// prefix mean "the thing you addressed does not exist" — the server
/// maps them to HTTP 404 (prefix stripped) instead of 500. Embedded
/// callers see the prefix verbatim; it is part of the error, not
/// decoration.
pub const NOT_FOUND_PREFIX: &str = "NOT_FOUND: ";

/// The `VECTOR (…)` clause on EXPLAIN SECTION — assembles named scalar
/// numeric fiber fields into one virtual vector per record (the
/// marcella_source_embeddings_bge_v2 case, where the 384-dim embedding
/// is stored as 384 separate scalar fibers v0..v383).
///
/// `label` is the stable row label as written in the clause
/// (`vector(v0..v383)` / `vector(v0,v1)`); `fields` is the expanded
/// component list in clause order.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplainVectorSpec {
    pub label: String,
    pub fields: Vec<String>,
}

/// Render a point-query key for error messages: `id='ghost'` /
/// `id=42`, composite pairs joined by `, ` in field-name order
/// (Record is a HashMap — sorting keeps the message deterministic).
fn render_key(key: &Record) -> String {
    let mut pairs: Vec<(&String, &Value)> = key.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", render_value(v)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_value(v: &Value) -> String {
    match v {
        Value::Text(s) => format!("'{s}'"),
        other => format!("{other}"),
    }
}

/// The typed miss for EXPLAIN SECTION … AT. Keeps the historical
/// "no section" phrasing (tests/explain_kappa.rs greps for it) while
/// naming the key and the bundle, per the Marcella error-contract ask.
fn no_section_error(bundle: &str, key: &Record) -> String {
    format!(
        "{NOT_FOUND_PREFIX}EXPLAIN: no section at {} in bundle '{bundle}'",
        render_key(key)
    )
}

fn no_numeric_notice_text() -> &'static str {
    "no numeric fiber fields with enough history (need \
     ≥2 records per field) to decompose κ yet"
}

/// Execute `EXPLAIN SECTION <bundle> AT <key> [VECTOR (…)]
/// [PROJECT (…)]` against a resolved store. Returns the per-field κ
/// decomposition rows from [`explain_record_k`] — the exact loop
/// `compute_record_k` runs — plus the ADDITIVE vector rows
/// (kind='vector').
///
/// Misses are typed: `Err(NOT_FOUND: …)` naming key and bundle.
pub fn execute_explain_section(
    store: &BundleRef<'_>,
    bundle: &str,
    key: &Record,
    project: Option<&[String]>,
    vector: Option<&ExplainVectorSpec>,
) -> Result<ExecResult, String> {
    let Some(mut rec) = store.point_query(key) else {
        return Err(no_section_error(bundle, key));
    };
    if let Some(fields) = project {
        rec.retain(|k, _| fields.iter().any(|f| f == k));
    }
    let mut ex = Explainer::new(store, vector)?;
    let rows = ex.explain_one(&rec)?;
    if rows.is_empty() {
        return Ok(ExecResult::Notice(no_numeric_notice_text().to_string()));
    }
    Ok(ExecResult::Rows(rows))
}

/// Execute the batch form `EXPLAIN SECTION <bundle> AT <field> IN
/// (v1, …, vn) [VECTOR (…)] [PROJECT (…)]` (Marcella EXPLAIN-family
/// ask 2).
///
/// Contract:
///   - grouped rows, INPUT order (the caller's list is the contract);
///     each group is one record's full EXPLAIN output with the key
///     value stamped as a discriminator column (`<field>` → value) on
///     EVERY row of the group;
///   - a missing key emits ONE typed miss entry (kind='miss', names
///     key value + bundle) — the batch never fails wholesale and
///     never silently skips;
///   - a found record with nothing to decompose emits ONE kind='empty'
///     note row (groups are never invisible);
///   - one store resolution / one engine read-lock for the whole
///     batch (the caller holds the lock across this call); vector
///     contexts (mu_v, R_cos — bundle-level) are computed once and
///     shared across groups via the Explainer cache.
pub fn execute_explain_batch(
    store: &BundleRef<'_>,
    bundle: &str,
    field: &str,
    values: &[Value],
    project: Option<&[String]>,
    vector: Option<&ExplainVectorSpec>,
) -> Result<ExecResult, String> {
    let mut ex = Explainer::new(store, vector)?;
    let mut out: Vec<Record> = Vec::new();
    for val in values {
        let mut key = Record::new();
        key.insert(field.to_string(), val.clone());
        let Some(mut rec) = store.point_query(&key) else {
            let mut row = Record::new();
            row.insert(field.to_string(), val.clone());
            row.insert("kind".into(), Value::Text("miss".into()));
            row.insert(
                "miss".into(),
                Value::Text(format!(
                    "no section at {field}={} in bundle '{bundle}'",
                    render_value(val)
                )),
            );
            out.push(row);
            continue;
        };
        if let Some(fields) = project {
            rec.retain(|k, _| fields.iter().any(|f| f == k));
        }
        let mut rows = ex.explain_one(&rec)?;
        if rows.is_empty() {
            let mut row = Record::new();
            row.insert(field.to_string(), val.clone());
            row.insert("kind".into(), Value::Text("empty".into()));
            row.insert(
                "note".into(),
                Value::Text(no_numeric_notice_text().to_string()),
            );
            out.push(row);
            continue;
        }
        for r in rows.iter_mut() {
            r.insert(field.to_string(), val.clone());
        }
        out.extend(rows);
    }
    Ok(ExecResult::Rows(out))
}

/// Shared per-statement explainer: resolves the stats source once,
/// validates the VECTOR clause once, and caches the bundle-level
/// vector contexts so a batch computes mu_v / R_cos once per target,
/// not once per key.
struct Explainer<'a> {
    store: &'a BundleRef<'a>,
    stats: std::borrow::Cow<'a, HashMap<String, FieldStats>>,
    fiber_fields: &'a [FieldDef],
    spec: Option<&'a ExplainVectorSpec>,
    /// Bundle-level vector contexts, keyed `<label-or-field>#<dim>` —
    /// mu and R_cos are properties of the bundle, computed once per
    /// statement. `None` caches "no context" (e.g. fewer than 2
    /// defined cosines) so the scans don't rerun either.
    ctx_cache: HashMap<String, Option<VectorContext>>,
}

impl<'a> Explainer<'a> {
    fn new(
        store: &'a BundleRef<'a>,
        spec: Option<&'a ExplainVectorSpec>,
    ) -> Result<Self, String> {
        let fiber_fields: &'a [FieldDef] = &store.schema().fiber_fields;
        // Typos are loud even when every key misses: validate the
        // clause against the schema up front.
        if let Some(spec) = spec {
            validate_vector_spec(spec, fiber_fields)?;
        }
        // Field statistics (ask 5b): heap bundles carry them
        // precomputed (borrowed, zero-copy). mmap-backed bundles
        // (OverlayBundle) compute them ON DEMAND — one O(N) scan over
        // the mmap base on first access, Welford-merged with the
        // overlay's live stats; cached in memory inside the overlay,
        // NOTHING persisted. O(N) per first EXPLAIN is accepted:
        // EXPLAIN is a diagnostic verb. This used to be an
        // `as_heap()`-or-decline gate; the polymorphic accessor was
        // already there.
        let stats = match store.as_heap() {
            Some(heap) => std::borrow::Cow::Borrowed(&heap.field_stats),
            None => std::borrow::Cow::Owned(store.field_stats()),
        };
        Ok(Self {
            store,
            stats,
            fiber_fields,
            spec,
            ctx_cache: HashMap::new(),
        })
    }

    /// One record's full EXPLAIN output: scalar rows (explain_record_k,
    /// unchanged) + additive vector rows, record_kappa stamped on all.
    fn explain_one(&mut self, rec: &Record) -> Result<Vec<Record>, String> {
        let mut rows = explain_record_k(&self.stats, rec, self.fiber_fields);
        let record_kappa = record_kappa_of(&rows, &self.stats, rec, self.fiber_fields);
        let vrows = self.vector_rows(rec, record_kappa)?;
        rows.extend(vrows);
        Ok(rows)
    }

    /// The ADDITIVE vector rows for one explained record:
    ///   (a) one row per fiber field carrying `Value::Vector` on the
    ///       target record (automatic — no clause needed);
    ///   (b) one row for the `VECTOR (…)` scalar-family clause,
    ///       labeled with the clause as written.
    fn vector_rows(&mut self, target: &Record, record_kappa: f64) -> Result<Vec<Record>, String> {
        let mut out = Vec::new();

        // (a) true Value::Vector fiber fields, schema order.
        for fd in self.fiber_fields {
            let Some(Value::Vector(tv)) = target.get(&fd.name) else {
                continue;
            };
            let dim = tv.len();
            let name = fd.name.clone();
            let extract = move |rec: &Record| match rec.get(&name) {
                Some(Value::Vector(v)) if v.len() == dim => Some(v.clone()),
                _ => None,
            };
            let cache_key = format!("{}#{dim}", fd.name);
            if let Some(ctx) = self.context_for(cache_key, dim, &extract) {
                if let Some(row) = kappa_v_row(&fd.name, tv, ctx, record_kappa) {
                    out.push(row);
                }
            }
        }

        // (b) explicit scalar-family clause (validated in `new`).
        if let Some(spec) = self.spec {
            let dim = spec.fields.len();
            let fields = spec.fields.clone();
            let extract = move |rec: &Record| assemble_family(rec, &fields);
            let cache_key = format!("{}#{dim}", spec.label);
            if let Some(tv) = assemble_family(target, &spec.fields) {
                if let Some(ctx) = self.context_for(cache_key, dim, &extract) {
                    if let Some(row) = kappa_v_row(&spec.label, &tv, ctx, record_kappa) {
                        out.push(row);
                    }
                }
            }
        }

        Ok(out)
    }

    fn context_for(
        &mut self,
        cache_key: String,
        dim: usize,
        extract: &dyn Fn(&Record) -> Option<Vec<f64>>,
    ) -> Option<&VectorContext> {
        let store = self.store;
        self.ctx_cache
            .entry(cache_key)
            .or_insert_with(|| build_vector_context(store, dim, extract))
            .as_ref()
    }
}

/// The record's κ (compute_record_k, the LOCKED total path over scalar
/// numeric fibers). Read off the scalar rows when they exist —
/// explain_record_k already stamped it — otherwise recomputed the same
/// way explain_record_k does, so vector rows on scalar-less bundles
/// still carry the honest total (0.0 when no scalar fiber
/// participates).
fn record_kappa_of(
    scalar_rows: &[Record],
    stats: &HashMap<String, FieldStats>,
    rec: &Record,
    fiber_fields: &[FieldDef],
) -> f64 {
    if let Some(k) = scalar_rows
        .first()
        .and_then(|r| r.get("record_kappa"))
        .and_then(|v| v.as_f64())
    {
        return k;
    }
    let fiber_vals: Vec<Value> = fiber_fields
        .iter()
        .map(|fd| rec.get(&fd.name).cloned().unwrap_or(Value::Null))
        .collect();
    compute_record_k(stats, &fiber_vals, fiber_fields)
}

// ── vector κ rows (Marcella ask 1) ──────────────────────────────────
//
// A NEW, separately-defined quantity — kappa_v does NOT participate in
// record_kappa (which stays compute_record_k over scalar numeric
// fibers; that function is LOCKED). Vector rows are additive, tagged
// kind='vector', and consumers computing the mean(kappa)==record_kappa
// cross-check must filter them out.
//
//     kappa_v = |1 − cos(v, mu_v)| / R_cos
//
// mu_v   — per-component mean vector of the field across the bundle,
//          computed ON DEMAND from the record scan in this call (NOT
//          from insert-time FieldStats: Value::Vector never enters
//          FieldStats, and the scan keeps mu and R_cos consistent on
//          one population — which is what makes the
//          "record == mean ⇒ kappa_v = 0 exactly" anchor exact).
// cos    — dot(v,mu) / sqrt(dot(v,v)·dot(mu,mu)). The cosine
//          self-normalizes both operands; NO separate unit-
//          normalization is applied. Marcella's BGE-v2 embeddings
//          happen to be unit-norm; correctness does not depend on it,
//          and kappa_v is direction-only by construction (scaling a
//          record's vector changes kappa_v only through mu's shift).
//          The sqrt(dot·dot) formulation makes cos(mu,mu) == 1.0
//          exactly in correctly-rounded f64 (sqrt(x·x) == x), giving
//          the zero anchor exactly.
// R_cos  — the bundle's effective cosine range: max − min of (1−cos)
//          observed across the bundle in this same EXPLAIN call,
//          floored to f64::EPSILON against divide-by-zero (mirrors
//          bundle::effective_range's floor).
//
// Cost: O(N) per vector target per EXPLAIN call (two scans: mu, then
// range). EXPLAIN is a diagnostic verb; nothing is persisted.

/// Bundle-level context for one vector target: the mean vector and the
/// observed (1−cos) range.
struct VectorContext {
    mu: Vec<f64>,
    /// dot(mu, mu), cached for the cosine denominator.
    mu_dot: f64,
    r_cos: f64,
    /// Records whose cosine participated in the range (dim-matched,
    /// nonzero norm).
    n: usize,
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// cos(v, mu) = dot(v,mu)/sqrt(dot(v,v)·dot(mu,mu)), clamped to
/// [−1, 1]. None when either norm is zero (no direction — mirrors the
/// σ>0 guard on the scalar z column).
fn cosine(v: &[f64], mu: &[f64], mu_dot: f64) -> Option<f64> {
    let vv = dot(v, v);
    if !(vv > 0.0) || !(mu_dot > 0.0) {
        return None;
    }
    let c = dot(v, mu) / (vv * mu_dot).sqrt();
    Some(c.clamp(-1.0, 1.0))
}

/// Two-pass context build over `store.records()`:
///   pass 1 — mu = per-component mean over every record `extract`
///            accepts (zero-norm vectors included: mu is a mean of
///            vectors, not of directions);
///   pass 2 — min/max of (1−cos) over records with a defined cosine.
/// None when fewer than 2 records have a defined cosine (mirrors the
/// count<2 skip on scalar fields: no baseline yet).
fn build_vector_context(
    store: &BundleRef<'_>,
    dim: usize,
    extract: &dyn Fn(&Record) -> Option<Vec<f64>>,
) -> Option<VectorContext> {
    if dim == 0 {
        return None;
    }
    let mut sums = vec![0.0f64; dim];
    let mut n_present = 0usize;
    for rec in store.records() {
        if let Some(v) = extract(&rec) {
            for (s, x) in sums.iter_mut().zip(v.iter()) {
                *s += x;
            }
            n_present += 1;
        }
    }
    if n_present < 2 {
        return None;
    }
    let mu: Vec<f64> = sums.iter().map(|s| s / n_present as f64).collect();
    let mu_dot = dot(&mu, &mu);
    if !(mu_dot > 0.0) {
        return None;
    }

    let mut min_omc = f64::INFINITY;
    let mut max_omc = f64::NEG_INFINITY;
    let mut n_cos = 0usize;
    for rec in store.records() {
        if let Some(v) = extract(&rec) {
            if let Some(c) = cosine(&v, &mu, mu_dot) {
                let omc = 1.0 - c;
                min_omc = min_omc.min(omc);
                max_omc = max_omc.max(omc);
                n_cos += 1;
            }
        }
    }
    if n_cos < 2 {
        return None;
    }
    let r_cos = (max_omc - min_omc).max(f64::EPSILON);
    Some(VectorContext {
        mu,
        mu_dot,
        r_cos,
        n: n_cos,
    })
}

/// One kappa_v row. None when the target's cosine is undefined
/// (zero-norm vector) — the row is skipped, never fabricated.
fn kappa_v_row(label: &str, target: &[f64], ctx: &VectorContext, record_kappa: f64) -> Option<Record> {
    let c = cosine(target, &ctx.mu, ctx.mu_dot)?;
    let omc = 1.0 - c;
    let kappa = omc.abs() / ctx.r_cos;
    let mut row = Record::new();
    row.insert("field".into(), Value::Text(label.to_string()));
    row.insert("kind".into(), Value::Text("vector".to_string()));
    row.insert("kappa".into(), Value::Float(kappa));
    row.insert("cos".into(), Value::Float(c));
    row.insert("one_minus_cos".into(), Value::Float(omc));
    row.insert("r_cos".into(), Value::Float(ctx.r_cos));
    row.insert("dim".into(), Value::Integer(target.len() as i64));
    row.insert("n".into(), Value::Integer(ctx.n as i64));
    // Same stamp every row carries. kappa_v does NOT feed record_kappa
    // — this is the scalar total riding along for context.
    row.insert("record_kappa".into(), Value::Float(record_kappa));
    Some(row)
}

/// Assemble the named scalar fields of `rec` into one virtual vector,
/// clause order. None if any component is absent or non-numeric on
/// this record (the virtual vector is undefined there — mirrored by
/// the same skip in the bundle passes).
fn assemble_family(rec: &Record, fields: &[String]) -> Option<Vec<f64>> {
    let mut v = Vec::with_capacity(fields.len());
    for f in fields {
        v.push(rec.get(f)?.as_f64()?);
    }
    Some(v)
}

/// Typos are loud: every field named by a `VECTOR (…)` clause must be
/// a scalar numeric fiber field of the schema. (Data-level sparsity is
/// not a typo — records that can't assemble the full vector are simply
/// skipped from the passes and get no row.)
fn validate_vector_spec(
    spec: &ExplainVectorSpec,
    fiber_fields: &[FieldDef],
) -> Result<(), String> {
    for f in &spec.fields {
        let Some(fd) = fiber_fields.iter().find(|fd| &fd.name == f) else {
            return Err(format!("VECTOR: '{f}' is not a fiber field of this bundle"));
        };
        match fd.field_type {
            FieldType::Numeric | FieldType::Timestamp => {}
            ref other => {
                return Err(format!(
                    "VECTOR: field '{f}' is not numeric (type {other:?}) — \
                     the clause assembles scalar numeric fibers"
                ))
            }
        }
    }
    Ok(())
}
