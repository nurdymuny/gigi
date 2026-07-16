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

use crate::bundle::{explain_record_k, FieldStats};
use crate::mmap_bundle::BundleRef;
use crate::parser::ExecResult;
use crate::types::{Record, Value};
use std::collections::HashMap;

/// Typed miss sentinel. Executor error strings beginning with this
/// prefix mean "the thing you addressed does not exist" — the server
/// maps them to HTTP 404 (prefix stripped) instead of 500. Embedded
/// callers see the prefix verbatim; it is part of the error, not
/// decoration.
pub const NOT_FOUND_PREFIX: &str = "NOT_FOUND: ";

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

/// Execute `EXPLAIN SECTION <bundle> AT <key> [PROJECT (…)]` against a
/// resolved store. Returns the per-field κ decomposition rows from
/// [`explain_record_k`] — the exact loop `compute_record_k` runs.
///
/// Misses are typed: `Err(NOT_FOUND: …)` naming key and bundle.
pub fn execute_explain_section(
    store: &BundleRef<'_>,
    bundle: &str,
    key: &Record,
    project: Option<&[String]>,
) -> Result<ExecResult, String> {
    let Some(mut rec) = store.point_query(key) else {
        return Err(no_section_error(bundle, key));
    };
    if let Some(fields) = project {
        rec.retain(|k, _| fields.iter().any(|f| f == k));
    }

    let Some(heap) = store.as_heap() else {
        return Ok(ExecResult::Notice(
            "EXPLAIN κ needs heap-resident field statistics; this \
             bundle is mmap-backed — HEALTH gives the aggregate view"
                .to_string(),
        ));
    };
    let stats: &HashMap<String, FieldStats> = &heap.field_stats;
    let fiber_fields = &heap.schema.fiber_fields;

    let rows = explain_record_k(stats, &rec, fiber_fields);
    if rows.is_empty() {
        return Ok(ExecResult::Notice(
            "no numeric fiber fields with enough history (need \
             ≥2 records per field) to decompose κ yet"
                .to_string(),
        ));
    }
    Ok(ExecResult::Rows(rows))
}
