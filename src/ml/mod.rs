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
