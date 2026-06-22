//! Virtual bundles — GIGI hosting itself.
//!
//! Personal-list #3 (2026-06-22). Exposes the live engine bundle
//! registry as a queryable bundle named `__bundles__`. The rows are
//! materialized on every `COVER __bundles__` call, so the answer is
//! always honest about current engine state — no caching, no WAL, no
//! on-disk footprint.
//!
//! This is the pyramid-paper structure-as-own-datum motivation lifted
//! into the engine: the registry is data the engine can be queried
//! about with the same vocabulary that queries every other bundle.
//! Theorem 7 of `theory/davis_geometric/pyramid_paper_v8.pdf` argues
//! that a network adjustment which includes the network's own
//! structure as a datum is the only construction that meets Old
//! Kingdom precision; the moral lifts cleanly: a database engine that
//! treats its own registry as a queryable bundle is more honest about
//! what it currently knows than one that hides the registry behind a
//! bespoke `SHOW BUNDLES` verb.
//!
//! ## Schema (synthetic, never persisted)
//!
//! ```text
//!   BASE  (name TEXT)
//!   FIBER (type TEXT, n_records INT, created_ts TIMESTAMP)
//! ```
//!
//! - `name` — bundle key as it appears in `engine.bundle_names()`.
//! - `type` — `"heap"` for `BundleStore` bundles, `"overlay"` for
//!   `OverlayBundle` (mmap-backed), `"virtual"` for the
//!   `__bundles__` self-row.
//! - `n_records` — `BundleRef::len()` (heap: `store.len()`; overlay:
//!   `base.len() + overlay_len()`).
//! - `created_ts` — best-effort creation timestamp:
//!     * heap → data-dir mtime if available, else 0 (epoch),
//!     * overlay → snapshot-file mtime if available, else 0,
//!     * `__bundles__` self-row → query time (seconds since epoch).
//!
//! ## Reserved names
//!
//! The set `reserved_names()` is consulted by every write-shape
//! `Statement` executor arm in `parser.rs`. `CREATE BUNDLE __bundles__`,
//! `INSERT INTO __bundles__`, `COLLAPSE __bundles__`, and the rest are
//! rejected with a clear error before any state mutation, so a
//! rejected write is byte-identical to no write at all. Future virtual
//! bundles (`__lattices__`, `__gauges__`, `__sessions__`) plug into the
//! same guard by extending the set.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bundle::QueryCondition;
use crate::engine::Engine;
use crate::types::{BundleSchema, FieldDef, Record, Value};

/// Canonical name of the virtual bundle that mirrors the engine
/// registry.
pub const BUNDLES_NAME: &str = "__bundles__";

/// True if `name` is a reserved virtual-bundle name. Writes against
/// any reserved name are rejected at the executor entry.
pub fn is_virtual(name: &str) -> bool {
    reserved_names().iter().any(|r| *r == name)
}

/// The set of reserved virtual-bundle names. Initially the singleton
/// `{ __bundles__ }`; future virtual bundles extend this list and
/// every existing write guard picks them up automatically.
pub fn reserved_names() -> &'static [&'static str] {
    &[BUNDLES_NAME]
}

/// Centralized reject helper used by every write-shape executor arm.
/// `verb` is a human-readable verb name (e.g. `"INSERT"`,
/// `"COLLAPSE"`) included in the error so the caller can tell which
/// statement was rejected. Returns `Ok(())` for non-reserved names.
pub fn reject_virtual_write(name: &str, verb: &str) -> Result<(), String> {
    if is_virtual(name) {
        Err(format!(
            "{verb} on '{name}' rejected: '{name}' is a virtual bundle (read-only); \
             use COVER {name} to query the live registry",
        ))
    } else {
        Ok(())
    }
}

/// Synthetic schema for `__bundles__`. Built fresh on every call so no
/// static state is reachable from the engine; `BundleSchema` is cheap
/// to construct (a few small `Vec`s and `String`s).
pub fn bundles_schema() -> BundleSchema {
    BundleSchema::new(BUNDLES_NAME)
        .base(FieldDef::categorical("name"))
        .fiber(FieldDef::categorical("type"))
        .fiber(FieldDef::numeric("n_records"))
        .fiber(FieldDef::timestamp("created_ts", 1.0))
}

/// Materialize the current engine registry as a `Vec<Record>` with
/// the `__bundles__` schema. The `__bundles__` self-row is appended
/// last so `COVER __bundles__` is honest about every bundle visible
/// to the engine including itself.
pub fn materialize_bundles_rows(engine: &Engine) -> Vec<Record> {
    let mut rows: Vec<Record> = Vec::new();
    let mut names: Vec<String> = engine
        .bundle_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    names.sort(); // deterministic order before any RANK BY
    for name in names {
        let (kind, n) = if engine.heap_bundle(&name).is_some() {
            let n = engine
                .bundle(&name)
                .map(|b| b.len() as i64)
                .unwrap_or(0);
            ("heap", n)
        } else if engine.mmap_bundle(&name).is_some() {
            let n = engine
                .bundle(&name)
                .map(|b| b.len() as i64)
                .unwrap_or(0);
            ("overlay", n)
        } else {
            ("unknown", 0)
        };
        let ts = engine.bundle_created_ts(&name).unwrap_or(0);
        rows.push(record_for(&name, kind, n, ts));
    }
    // Self-row — __bundles__ describes itself.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    rows.push(record_for(BUNDLES_NAME, "virtual", 0, now));
    rows
}

fn record_for(name: &str, kind: &str, n_records: i64, created_ts: i64) -> Record {
    let mut r: HashMap<String, Value> = HashMap::new();
    r.insert("name".to_string(), Value::Text(name.to_string()));
    r.insert("type".to_string(), Value::Text(kind.to_string()));
    r.insert("n_records".to_string(), Value::Integer(n_records));
    r.insert("created_ts".to_string(), Value::Timestamp(created_ts));
    r
}

/// Apply the standard COVER clause vocabulary (WHERE / OR / RANK BY /
/// FIRST / SKIP / PROJECT / DISTINCT) against an in-memory row set.
/// Reuses `QueryCondition::matches` and `matches_filter` so behavior
/// is byte-identical to a real-bundle COVER for the same clauses.
pub fn apply_query_clauses(
    rows: Vec<Record>,
    conditions: &[QueryCondition],
    or_groups: Option<&[Vec<QueryCondition>]>,
    sort: Option<&[(&str, bool)]>,
    first: Option<usize>,
    skip: Option<usize>,
    project: Option<&[&str]>,
    distinct: Option<&str>,
) -> Vec<Record> {
    // DISTINCT short-circuits the same way the real Cover executor
    // does: it returns one row per unique value of the named field
    // (any other clauses are intentionally ignored, matching the
    // real bundle's `distinct()` surface).
    if let Some(field) = distinct {
        let mut seen: Vec<Value> = Vec::new();
        for r in &rows {
            if let Some(v) = r.get(field) {
                if !seen.contains(v) {
                    seen.push(v.clone());
                }
            }
        }
        return seen
            .into_iter()
            .map(|v| {
                let mut r: HashMap<String, Value> = HashMap::new();
                r.insert(field.to_string(), v);
                r
            })
            .collect();
    }

    // 1. Filter (AND conditions + optional OR groups).
    let mut filtered: Vec<Record> = rows
        .into_iter()
        .filter(|r| crate::bundle::matches_filter(r, conditions, or_groups))
        .collect();

    // 2. Sort (only first sort key honored — matches the real Cover
    //    executor's `filtered_query_ex` path, which takes a single
    //    `(field, desc)` pair).
    if let Some(specs) = sort {
        if let Some((field, desc)) = specs.first() {
            let field = field.to_string();
            let desc = *desc;
            filtered.sort_by(|a, b| {
                let va = a.get(&field);
                let vb = b.get(&field);
                let ord = match (va, vb) {
                    (Some(a), Some(b)) => a.cmp(b),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                };
                if desc { ord.reverse() } else { ord }
            });
        }
    }

    // 3. SKIP then FIRST.
    if let Some(n) = skip {
        if n < filtered.len() {
            filtered.drain(0..n);
        } else {
            filtered.clear();
        }
    }
    if let Some(n) = first {
        filtered.truncate(n);
    }

    // 4. PROJECT.
    if let Some(fields) = project {
        filtered = filtered
            .into_iter()
            .map(|r| {
                let mut out: HashMap<String, Value> = HashMap::new();
                for f in fields {
                    if let Some(v) = r.get(*f) {
                        out.insert((*f).to_string(), v.clone());
                    }
                }
                out
            })
            .collect();
    }

    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_virtual_recognizes_bundles_name() {
        assert!(is_virtual(BUNDLES_NAME));
        assert!(is_virtual("__bundles__"));
        assert!(!is_virtual("users"));
        assert!(!is_virtual("bundles"));
    }

    #[test]
    fn test_reserved_names_includes_bundles() {
        let names = reserved_names();
        assert!(names.contains(&"__bundles__"));
    }

    #[test]
    fn test_reject_virtual_write_passes_real_names() {
        assert!(reject_virtual_write("users", "INSERT").is_ok());
        assert!(reject_virtual_write("orders", "COLLAPSE").is_ok());
    }

    #[test]
    fn test_reject_virtual_write_rejects_virtual_names() {
        let err = reject_virtual_write("__bundles__", "INSERT")
            .expect_err("must reject");
        assert!(err.contains("__bundles__"));
        assert!(err.contains("virtual"));
        assert!(err.contains("read-only"));
    }

    #[test]
    fn test_bundles_schema_has_expected_shape() {
        let s = bundles_schema();
        assert_eq!(s.name, BUNDLES_NAME);
        assert_eq!(s.base_fields.len(), 1);
        assert_eq!(s.base_fields[0].name, "name");
        assert_eq!(s.fiber_fields.len(), 3);
        let fiber_names: Vec<&str> = s.fiber_fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(fiber_names, vec!["type", "n_records", "created_ts"]);
    }
}
