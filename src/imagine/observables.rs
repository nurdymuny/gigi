//! Canonical observable dispatch for `INTEGRATE OBSERVABLE <name>
//! ALONG <path_ident>` per Hallie's WISH ASK 4 (2026-06-22).
//!
//! The verb does a trapezoidal line integral
//!
//!     ∫_γ O(γ) ds ≈ Σ_i 0.5 · Δs_i · (O(γ_i) + O(γ_{i+1}))
//!
//! and this module wires `<name> → O(record) -> f64` for the small
//! set of canonical names the v1 verb knows about. Bundle-specific
//! observables go through the `WishBundle::evaluate_observable`
//! extension point on the bundle trait (deferred to the Halcyon
//! buckyball follow-up — until then, unknown names return a clear
//! error with the full known-canonical list).
//!
//! ## Tier 1 — canonical names
//!
//!   * `arc_length_unit` — constant 1.0; integrating it returns total
//!     arc length. Hallie's first sanity test ("constant observable
//!     integrates to total arc length").
//!   * `local_k` — Gaussian curvature at `γ_i`, read from
//!     `ImaginedRecord::local_k`. Useful for curvature-energy probes.
//!   * `accumulated_holonomy` — running holonomy defect at `γ_i`. Lets
//!     callers integrate ∫ ω along γ without re-running parallel
//!     transport.
//!   * `path_length_so_far` — when the path provenance carries a
//!     `Geodesic { path_length, .. }`, return that; else fall back to
//!     `arc_length_unit` (documented sentinel value 1.0).
//!
//! ## Tier 2 — `WishBundle::evaluate_observable`
//!
//! Future ride-along: when `WishBundle` registers an `evaluate_observable`
//! dispatcher for a named bundle, that handler is preferred over the
//! tier-1 table. v1 ships the tier-1 surface only; tier-2 wiring lands
//! with the Halcyon follow-up.

use crate::imagine::provenance::{ImaginedProvenance, ImaginedRecord};

/// Errors from observable resolution.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ObservableError {
    #[error(
        "observable '{name}' is not registered. Known canonical names: \
         {known:?}. Bundle-specific observables register through \
         `WishBundle::evaluate_observable` (not wired in this v1)."
    )]
    Unknown { name: String, known: Vec<String> },
}

/// Names the tier-1 dispatch table recognizes. Kept in a single list
/// so the error message + the `is_canonical` helper stay in sync.
pub const CANONICAL_NAMES: &[&str] = &[
    "arc_length_unit",
    "local_k",
    "accumulated_holonomy",
    "path_length_so_far",
];

/// Resolve `name` against the tier-1 canonical dispatch table.
/// Returns `O(record)` as `f64` or `ObservableError::Unknown` if the
/// name is unknown (the caller surfaces this through the GQL error
/// channel).
pub fn evaluate_canonical(name: &str, record: &ImaginedRecord) -> Result<f64, ObservableError> {
    match name {
        "arc_length_unit" => Ok(1.0),
        "local_k" => Ok(record.local_k),
        "accumulated_holonomy" => Ok(record.accumulated_holonomy),
        "path_length_so_far" => match &record.provenance {
            ImaginedProvenance::Geodesic { path_length, .. } => Ok(*path_length),
            _ => Ok(1.0),
        },
        other => Err(ObservableError::Unknown {
            name: other.to_string(),
            known: CANONICAL_NAMES.iter().map(|s| s.to_string()).collect(),
        }),
    }
}

/// True iff `name` is in the tier-1 canonical table.
pub fn is_canonical(name: &str) -> bool {
    CANONICAL_NAMES.contains(&name)
}

/// Parameter-space Euclidean Δs between two adjacent records on γ.
/// First-ship default per the design note ("fall back to the
/// connection's natural arc-length — parameter-space Euclidean
/// distance Δs_i = ||γ_{i+1} − γ_i||_2 with a clear doc-comment
/// noting this is the parameter-space convention"). When a registered
/// `WishBundle::induced_metric` is present, the future tier-2 surface
/// upgrades this with `sqrt(g_{ab} Δγ^a Δγ^b)`.
pub fn euclidean_segment_length(a: &ImaginedRecord, b: &ImaginedRecord) -> f64 {
    debug_assert_eq!(
        a.coords.len(),
        b.coords.len(),
        "ImaginedRecord coords must share dim on a single path"
    );
    let mut sum = 0.0_f64;
    for (ax, bx) in a.coords.iter().zip(b.coords.iter()) {
        let d = bx - ax;
        sum += d * d;
    }
    sum.sqrt()
}

/// Trapezoidal line integral of `name` along γ. `records` must hold
/// at least two entries — empty / single-element paths are caller-
/// validated (see executor: `EmptyPath` is the surfaced error). The
/// observable is evaluated at every record, then adjacent pairs are
/// trapezoidally summed.
pub fn trapezoidal_integrate(
    name: &str,
    records: &[ImaginedRecord],
) -> Result<f64, ObservableError> {
    if records.len() < 2 {
        // Single-record path — degenerate but well-defined; the
        // line integral is zero. Empty paths are caught by the
        // executor before this function is called.
        return Ok(0.0);
    }
    // Pre-evaluate at every record (matches the
    // `Σ_i 0.5 · Δs_i · (O(γ_i) + O(γ_{i+1}))` shape — every
    // interior record is touched twice, the endpoints once).
    let mut values: Vec<f64> = Vec::with_capacity(records.len());
    for r in records {
        values.push(evaluate_canonical(name, r)?);
    }
    let mut acc = 0.0_f64;
    for i in 0..records.len() - 1 {
        let ds = euclidean_segment_length(&records[i], &records[i + 1]);
        acc += 0.5 * ds * (values[i] + values[i + 1]);
    }
    Ok(acc)
}
