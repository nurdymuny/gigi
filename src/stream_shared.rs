//! Shared HTTP-layer helpers hoisted from `src/bin/gigi_stream.rs`
//! (stream-extraction phase 2, EXTRACTION_MAP.md "Cross-family shared
//! modules"). Consumers: the brain-primitive handlers and GQL verb arms
//! (still in the binary) and the post-Kähler PK-1..4 REST logic
//! (`geometry::pk_http` / `discrete::pk_http`). Moved text is verbatim
//! from the binary — the only edits are `gigi::` → `crate::` paths and
//! `pub` visibility.

// Only the cfg(kahler) error helpers touch axum types; keep the import
// under the same gate so the no-feature build stays warning-free.
#[cfg(feature = "kahler")]
use axum::{http::StatusCode, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[cfg(feature = "kahler")]
pub fn extract_field_samples(
    store: &crate::BundleStore,
    fields: &[String],
) -> Result<(Vec<Vec<f64>>, Vec<usize>), String> {
    if fields.is_empty() {
        return Err("at least one fiber field required".into());
    }
    // Records are slices indexed by fiber-field position. Resolve
    // each requested name to its index in the schema. We give a
    // detailed error message if the field is in base_fields rather
    // than fiber_fields (per Marcella's 2026-05-25 probe report —
    // her `token_id` is a base_field and the original "not in
    // schema" message was confusing).
    // Each requested field contributes 1 sample column (scalar) or
    // `dims` columns (a `vector(dims)` fiber, expanded per-component —
    // the same expansion the dial surface's `ScopedField::Vector` arm
    // does). Before 2026-08-02 a `Value::Vector` fell to the skip arm
    // below and a fully-vector bundle answered every brain endpoint
    // over zero samples (Art's vec_probe report).
    let mut field_idx = Vec::with_capacity(fields.len());
    let mut field_cols = Vec::with_capacity(fields.len());
    for f in fields {
        let i = store
            .schema
            .fiber_fields
            .iter()
            .position(|fd| fd.name == *f)
            .ok_or_else(|| {
                let in_base = store
                    .schema
                    .base_fields
                    .iter()
                    .any(|fd| fd.name == *f);
                let available_fiber: Vec<&str> = store
                    .schema
                    .fiber_fields
                    .iter()
                    .map(|fd| fd.name.as_str())
                    .collect();
                if in_base {
                    format!(
                        "field '{}' is a base_field (query key), not a fiber_field. \
                         Brain endpoints only operate on fiber dimensions. \
                         Available fiber_fields: {:?}",
                        f, available_fiber
                    )
                } else {
                    format!(
                        "field '{}' not found in schema. \
                         Available fiber_fields: {:?}",
                        f, available_fiber
                    )
                }
            })?;
        field_idx.push(i);
        field_cols.push(
            match store.schema.fiber_fields[i].field_type {
                crate::types::FieldType::Vector { dims } => dims,
                _ => 1,
            },
        );
    }
    let width: usize = field_cols.iter().sum();
    // Skip-and-log (engine hardening, Hallie's ask #7): a single record with a
    // non-numeric or missing fiber value must NOT fail the whole brain endpoint
    // (intent_gate / confidence / attend / explain). One poisoned row today took
    // down live Marcella's confidence gate — fail-open on every query. Drop the
    // offending record, count it, and continue on the valid rows, reporting once.
    // `kept` carries each surviving row's original section index so callers that
    // map results back to records (attend) stay correct; with no corruption it is
    // simply `0..n`, identical to the old behaviour.
    let mut samples = Vec::new();
    let mut kept: Vec<usize> = Vec::new();
    let mut skipped = 0usize;
    let mut skip_field: Option<String> = None;
    for (orig_idx, (_bp, record)) in store.sections().enumerate() {
        let mut row = Vec::with_capacity(width);
        let mut bad = false;
        for (slot, &i) in field_idx.iter().enumerate() {
            match record.get(i) {
                Some(crate::types::Value::Float(x)) if field_cols[slot] == 1 => row.push(*x),
                Some(crate::types::Value::Integer(j)) if field_cols[slot] == 1 => {
                    row.push(*j as f64)
                }
                // vector(dims) fiber: expand per-component. A value whose
                // length disagrees with the schema dims is corruption —
                // skip the record (never zero-pad statistics).
                Some(crate::types::Value::Vector(v)) if v.len() == field_cols[slot] => {
                    row.extend_from_slice(v)
                }
                _ => {
                    if skip_field.is_none() {
                        skip_field = Some(fields[slot].clone());
                    }
                    bad = true;
                    break;
                }
            }
        }
        if bad {
            skipped += 1;
            continue;
        }
        samples.push(row);
        kept.push(orig_idx);
    }
    // Total-blindness refusal (Art's acceptance criterion, 2026-08-02): if
    // EVERY record was skipped, the caller's representation is wholly
    // unsupported or wholly corrupt — returning Ok(empty) would let a brain
    // endpoint answer confidently over zero samples with HTTP 200, which is
    // the exact failure mode the refusal architecture exists to prevent.
    // An empty bundle (skipped == 0) stays Ok: zero records is an honest
    // zero, not blindness. Partial corruption keeps the fail-open contract.
    if samples.is_empty() && skipped > 0 {
        return Err(format!(
            "all {} record(s) were skipped — values in field '{}' do not match its \
             schema type. Refusing to answer over an empty sample set; check the \
             stored representation (scalar fields need Float/Integer, vector(d) \
             fields need a d-component Vector).",
            skipped,
            skip_field.as_deref().unwrap_or("?"),
        ));
    }
    if skipped > 0 {
        eprintln!(
            "[extract_field_samples] skip-and-log: dropped {} malformed record(s) \
             (non-numeric/missing value, first offending field '{}'); continuing on {} \
             valid record(s). Repair the bundle — one bad row no longer fails the brain.",
            skipped,
            skip_field.as_deref().unwrap_or("?"),
            samples.len()
        );
    }
    Ok((samples, kept))
}

// ─── helpers for the 5 brain endpoints ─────────────────────

#[cfg(feature = "kahler")]
pub fn not_found(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

#[cfg(feature = "kahler")]
pub fn bad_request(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

/// **#107 fix.** Adapter that lets brain endpoints handle both
/// heap-resident and mmap+overlay bundles. Heap path is zero-cost
/// (returns the existing &BundleStore reference). Overlay path
/// materializes the merged view into a temporary heap store
/// (O(N) walk, ~10ms for 10k records) and returns a reference
/// into the caller's stack-allocated `Option<BundleStore>`.
///
/// Usage pattern (3 lines per endpoint):
///
/// ```ignore
/// let store_ref = engine.bundle(&name).ok_or_else(|| not_found(...))?;
/// let mut _promoted: Option<gigi::BundleStore> = None;
/// let heap = heap_or_promote(&store_ref, &mut _promoted);
/// // ... `heap: &BundleStore` works identically for both variants ...
/// ```
///
/// This is a deliberately surgical fix: it preserves the existing
/// helper signatures (`extract_field_samples`, `fit_*_gaussian`,
/// `flow_from_bundle_cached`, etc.) that all take `&BundleStore`,
/// instead of refactoring them to be polymorphic. The one-time
/// materialize cost is dominated by the existing per-call fit work.
#[cfg(feature = "kahler")]
pub fn heap_or_promote<'a>(
    store: &'a crate::BundleRef<'a>,
    promoted: &'a mut Option<crate::BundleStore>,
) -> &'a crate::BundleStore {
    match store {
        crate::BundleRef::Heap(h) => *h,
        crate::BundleRef::Overlay(o) => {
            *promoted = Some(o.to_temp_heap_store());
            promoted
                .as_ref()
                .expect("promoted was just set in this branch")
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    /// **Skip-and-log (Hallie's ask #7).** One poisoned record — a non-numeric
    /// value in a numeric fiber field, exactly what landed in
    /// `marcella_source_embeddings_bge_v2` and took down `intent_gate` (and with
    /// it live Marcella's confidence gate) — must NOT fail the whole brain
    /// endpoint. `extract_field_samples` drops the bad row, keeps the rest, and
    /// returns each survivor's original section index so `attend` still maps
    /// results back to the right records.
    ///
    /// Gated on `kahler` because `extract_field_samples` itself is — without
    /// the gate the whole no-feature `--bin gigi-stream` test build fails to
    /// compile (found while closing the h1-vs-κ perf-doc flag, 2026-07-30).
    #[cfg(feature = "kahler")]
    #[test]
    fn extract_field_samples_skips_poisoned_record() {
        use crate::types::{BundleSchema, FieldDef, Record, Value};
        let schema = BundleSchema::new("poisoned_bge")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("v0").with_range(5.0))
            .fiber(FieldDef::numeric("v1").with_range(5.0));
        let mut store = crate::BundleStore::new(schema);
        for i in 0..5 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("v0".into(), Value::Float(i as f64));
            r.insert("v1".into(), Value::Float(i as f64 + 0.5));
            // one record gets a non-numeric v0 — the exact corruption shape.
            if i == 2 {
                r.insert("v0".into(), Value::Text("corrupt".into()));
            }
            store.insert(&r);
        }
        let fields = vec!["v0".to_string(), "v1".to_string()];
        let (samples, kept) = extract_field_samples(&store, &fields)
            .expect("one bad row must not fail the endpoint");

        // exactly the four clean records survive; the poisoned v0 (=2.0) is gone.
        assert_eq!(samples.len(), 4, "four clean records survive");
        assert_eq!(kept.len(), samples.len(), "one kept index per surviving row");
        let mut v0s: Vec<f64> = samples.iter().map(|r| r[0]).collect();
        assert!(!v0s.contains(&2.0), "poisoned record's row is absent");
        v0s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(v0s, vec![0.0, 1.0, 3.0, 4.0]);

        // `kept` maps each surviving row back to its true record (attend-correct).
        let secs: Vec<_> = store.sections().collect();
        for (j, &orig) in kept.iter().enumerate() {
            match secs[orig].1.get(0) {
                Some(Value::Float(x)) => {
                    assert_eq!(*x, samples[j][0], "kept[{j}] maps row to its record")
                }
                other => panic!("kept index {orig} points at non-numeric {other:?}"),
            }
        }
    }

    /// **Vector fibers are brain-visible (Art's vec_probe, 2026-08-02).**
    /// `INGEST … FORMAT JSONL` stores embeddings as `Value::Vector` and the
    /// reference calls that first-class — so `extract_field_samples` must
    /// expand a `vector(dims)` fiber into `dims` sample columns, exactly as
    /// the dial surface's `ScopedField::Vector` arm already does. Before this
    /// test, every Vector record fell to the skip-and-log `_` arm and the
    /// brain answered over zero samples with HTTP 200.
    #[cfg(feature = "kahler")]
    #[test]
    fn extract_field_samples_expands_vector_fibers() {
        use crate::types::{BundleSchema, FieldDef, FieldType, Record, Value};
        let mut emb = FieldDef::numeric("emb");
        emb.field_type = FieldType::Vector { dims: 4 };
        let schema = BundleSchema::new("vec_probe")
            .base(FieldDef::categorical("doc_id"))
            .fiber(emb)
            .fiber(FieldDef::numeric("score"));
        let mut store = crate::BundleStore::new(schema);
        for (id, v, s) in [
            ("a", vec![1.0, 0.0, 0.0, 0.0], 0.9),
            ("b", vec![0.0, 1.0, 0.0, 0.0], 0.7),
        ] {
            let mut r = Record::new();
            r.insert("doc_id".into(), Value::Text(id.into()));
            r.insert("emb".into(), Value::Vector(v));
            r.insert("score".into(), Value::Float(s));
            store.insert(&r);
        }

        // vector-only: 2 samples, width dims=4
        let (samples, kept) =
            extract_field_samples(&store, &["emb".to_string()]).expect("vector fiber is supported");
        assert_eq!(samples.len(), 2, "both records visible to the brain");
        assert!(samples.iter().all(|r| r.len() == 4), "one column per component");
        assert_eq!(kept.len(), 2);
        let mut firsts: Vec<f64> = samples.iter().map(|r| r[0]).collect();
        firsts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(firsts, vec![0.0, 1.0], "real components, not padding");

        // mixed scalar + vector: width = 1 + 4, scalar first per request order
        let (samples, _) =
            extract_field_samples(&store, &["score".to_string(), "emb".to_string()])
                .expect("mixed scalar+vector row");
        assert!(samples.iter().all(|r| r.len() == 5), "1 scalar + 4 vector columns");
        assert!(samples.iter().any(|r| (r[0] - 0.9).abs() < 1e-12));
    }

    /// **Wrong-length vectors are poisoned rows, not padded ones.** A
    /// `vector(4)` fiber carrying a 3-component value is corruption; the
    /// skip-and-log contract drops that record and keeps the clean rest —
    /// it must never silently zero-pad statistics.
    #[cfg(feature = "kahler")]
    #[test]
    fn extract_field_samples_skips_wrong_length_vector() {
        use crate::types::{BundleSchema, FieldDef, FieldType, Record, Value};
        let mut emb = FieldDef::numeric("emb");
        emb.field_type = FieldType::Vector { dims: 4 };
        let schema = BundleSchema::new("vec_len")
            .base(FieldDef::categorical("doc_id"))
            .fiber(emb);
        let mut store = crate::BundleStore::new(schema);
        for (id, v) in [
            ("a", vec![1.0, 0.0, 0.0, 0.0]),
            ("short", vec![1.0, 0.0]),
            ("c", vec![0.0, 0.0, 1.0, 0.0]),
        ] {
            let mut r = Record::new();
            r.insert("doc_id".into(), Value::Text(id.into()));
            r.insert("emb".into(), Value::Vector(v));
            store.insert(&r);
        }
        let (samples, kept) =
            extract_field_samples(&store, &["emb".to_string()]).expect("clean rows survive");
        assert_eq!(samples.len(), 2, "the short vector's record is dropped");
        assert_eq!(kept.len(), 2);
        assert!(samples.iter().all(|r| r.len() == 4));
    }

    /// **Total blindness is a refusal, not a 200 (Art's acceptance criterion).**
    /// When every record is skipped — the caller's representation is wholly
    /// unsupported or wholly corrupt — returning `Ok(empty)` lets a brain
    /// endpoint answer confidently over nothing. That is the exact failure
    /// mode the refusal architecture exists to prevent, so the extractor
    /// must return `Err` naming the offending field. An EMPTY bundle stays
    /// `Ok(empty)` — zero records is an honest zero, not blindness.
    #[cfg(feature = "kahler")]
    #[test]
    fn extract_field_samples_refuses_when_all_records_skipped() {
        use crate::types::{BundleSchema, FieldDef, Record, Value};
        let schema = BundleSchema::new("all_bad")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x"));
        let mut store = crate::BundleStore::new(schema);
        for i in 0..3 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("x".into(), Value::Text("not a number".into()));
            store.insert(&r);
        }
        let err = extract_field_samples(&store, &["x".to_string()])
            .expect_err("all-skipped must refuse, not answer over zero samples");
        assert!(err.contains("all 3"), "error names the count: {err}");
        assert!(err.contains("'x'"), "error names the field: {err}");

        // empty bundle: honest zero, not an error
        let empty = crate::BundleStore::new(
            BundleSchema::new("empty")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x")),
        );
        let (samples, kept) =
            extract_field_samples(&empty, &["x".to_string()]).expect("empty bundle is Ok");
        assert!(samples.is_empty() && kept.is_empty());
    }
}
