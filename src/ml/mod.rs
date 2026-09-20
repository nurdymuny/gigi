//! GIGI's geometric ML suite — the compute core behind the
//! `/v1/bundles/{name}/{scan, scan/fit, cluster, infer, reduce, prescribe,
//! solve, circulation, factorize, changepoints}` REST endpoints.
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1, see EXTRACTION_MAP.md). The axum handlers and the `/v1/ml`
//! discovery catalog remain in the binary; each handler is a thin wrapper
//! that binds the bundle + JSON wire format around these functions.

pub mod cadence;
pub mod changepoints;
pub mod circulation;
pub mod cluster;
pub mod factorize;
pub mod infer;
pub mod precedence;
pub mod prescribe;
pub mod reduce;
pub mod scan;
pub mod solve;
pub mod texture;

#[cfg(test)]
pub mod test_support;

/// Numeric value of an `order` field entry, accepting numeric **text**.
///
/// A field whose values are numeric strings — a trade id, a row key exported as
/// text — must sort NUMERICALLY. Lexicographic order puts `"10"` before `"9"`,
/// which silently scrambles the record order that TEXTURE and PRECEDENCE read.
///
/// Measured before this existed: one trending series read `exponent 0.5307 /
/// RANDOM_WALK` ordered by a numeric field and `0.0882 / ROUGH` ordered by its
/// own text id — same data, same verb, a flipped verdict and nothing in the
/// response saying why. Zero-padded ids happened to be correct, which is worse:
/// the defect appears only on some customers' data.
pub fn order_value(v: &crate::types::Value) -> Option<f64> {
    v.as_f64().or_else(|| match v {
        crate::types::Value::Text(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Sort `records` by `order`, numerically when every value is numeric (or
/// numeric text), lexicographically otherwise.
///
/// Returns `true` when the sort was lexicographic, so the caller can DISCLOSE
/// it. Lexicographic is right for ISO-8601 timestamps and wrong for unpadded
/// numbers, and the caller cannot tell which they have — so the verb says.
/// Resolve which field gives an order-sensitive verb its order.
///
/// Precedence, and it is deliberate: an explicit per-call override wins,
/// because reading a bundle for reconciliation and reading it for replay are
/// different questions. The bundle's declared order field is the default, so a
/// caller cannot forget it. With neither, record order is only allowed when the
/// bundle actually iterates in insertion order.
///
/// `Ok(None)` means "record order is meaningful here, use it".
///
/// The storage check used to compare against "hashed" and "hybrid" by name,
/// which silently admitted memory-mapped bundles -- they report
/// "mmap+overlay" and iterate overlay rows before snapshot rows, which is not
/// insertion order by construction and is what a restarted engine serves. It
/// now admits only the one mode that is insertion-ordered.
pub fn resolve_order(
    bundle: &str,
    schema: &crate::types::BundleSchema,
    storage_mode: &str,
    call_override: Option<&str>,
) -> Result<Option<String>, (axum::http::StatusCode, String)> {
    use axum::http::StatusCode;
    let declared = |f: &str| {
        schema
            .base_fields
            .iter()
            .chain(schema.fiber_fields.iter())
            .any(|d| d.name == f)
    };
    if let Some(o) = call_override {
        if !declared(o) {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("order field '{o}' not found"),
            ));
        }
        return Ok(Some(o.to_string()));
    }
    if let Some(o) = &schema.order_field {
        return Ok(Some(o.clone()));
    }
    if storage_mode == "sequential" {
        return Ok(None);
    }
    Err((
        StatusCode::UNPROCESSABLE_ENTITY,
        format!(
            "bundle '{bundle}' is {storage_mode}-stored, so its records do not iterate in insertion order -- and this verb reads record order. Declare an ordering field on the bundle, or name one on the call."
        ),
    ))
}

/// Refuse when the ordering field does not actually order the rows.
///
/// Two rows sharing an order value leave their relative order to whatever the
/// sort happened to do with them, which today is the storage order because the
/// sort is stable. That is an accident, not a contract: swapping in an unstable
/// sort would change answers with nothing to catch it. A verb that reads order
/// should not answer from one that does not exist.
pub fn refuse_on_ties(
    records: &[crate::types::Record],
    order: &str,
    bundle: &str,
) -> Result<(), (axum::http::StatusCode, String)> {
    use axum::http::StatusCode;
    let key = |r: &crate::types::Record| {
        r.get(order)
            .map(|v| format!("{v}"))
            .unwrap_or_else(|| "<missing>".into())
    };
    for pair in records.windows(2) {
        if key(&pair[0]) == key(&pair[1]) {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                format!(
                    "bundle '{bundle}': order field '{order}' has at least two rows sharing the value {}, so it does not order them. Name a field that is unique across the rows you are measuring, or add a tie-break to the data.",
                    key(&pair[0])
                ),
            ));
        }
    }
    Ok(())
}

pub fn sort_by_order(records: &mut [crate::types::Record], order: &str) -> bool {
    let all_numeric = records
        .iter()
        .all(|r| r.get(order).and_then(order_value).is_some());
    if all_numeric {
        records.sort_by(|a, b| {
            let x = a.get(order).and_then(order_value).unwrap_or(f64::NAN);
            let y = b.get(order).and_then(order_value).unwrap_or(f64::NAN);
            x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        records.sort_by(|a, b| {
            a.get(order).map(|v| format!("{}", v))
                .cmp(&b.get(order).map(|v| format!("{}", v)))
        });
    }
    !all_numeric
}

pub use cadence::*;
pub use changepoints::*;
pub use circulation::*;
pub use cluster::*;
pub use factorize::*;
pub use infer::*;
pub use precedence::*;
pub use prescribe::*;
pub use reduce::*;
pub use scan::*;
pub use texture::*;
pub use solve::*;
