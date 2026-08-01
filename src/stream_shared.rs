//! Shared HTTP-layer helpers hoisted from `src/bin/gigi_stream.rs`
//! (stream-extraction phase 2, EXTRACTION_MAP.md "Cross-family shared
//! modules"). Consumers: the brain-primitive handlers and GQL verb arms
//! (still in the binary) and the post-Kähler PK-1..4 REST logic
//! (`geometry::pk_http` / `discrete::pk_http`). Moved text is verbatim
//! from the binary — the only edits are `gigi::` → `crate::` paths and
//! `pub` visibility.

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
    let mut field_idx = Vec::with_capacity(fields.len());
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
    }
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
        let mut row = Vec::with_capacity(fields.len());
        let mut bad = false;
        for &i in &field_idx {
            let v = match record.get(i) {
                Some(crate::types::Value::Float(x)) => *x,
                Some(crate::types::Value::Integer(j)) => *j as f64,
                _ => {
                    if skip_field.is_none() {
                        skip_field = Some(
                            fields[field_idx.iter().position(|&x| x == i).unwrap_or(0)]
                                .clone(),
                        );
                    }
                    bad = true;
                    break;
                }
            };
            row.push(v);
        }
        if bad {
            skipped += 1;
            continue;
        }
        samples.push(row);
        kept.push(orig_idx);
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
}
