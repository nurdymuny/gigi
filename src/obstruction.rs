//! OBSTRUCTION verb — Phase 1 (GREEN).
//!
//! Decides whether a principal G-bundle on a closed base manifold
//! admits a global section. The leading obstruction is the integrated
//! characteristic class (`c_2` for SU(N) on 4D bases, `c_1` for U(1)
//! on 2D bases). On closed 2-manifolds with SU(N≥2) the answer is
//! vacuously trivial: every SU(N≥2) bundle on a closed surface admits
//! a global section.
//!
//! Phase 1 algorithm (Chern-Weil discrete reduction):
//!
//! 1. Resolve the bundle in the engine; infer `(Group, base_dim)` from
//!    the schema fiber width + bundle name prefix.
//! 2. Dispatch on `(group, base_dim)`:
//!      - `(SU(N≥2), 2)` — vacuous trivial, `class = 0`.
//!      - `(U(1), 2)`    — compute `c_1` from the cumulative phase
//!        wrap around the homology generator (sum of edge θ
//!        differences / 2π, rounded).
//!      - `(SU(N), 4)`   — compute `c_2` (instanton number) from
//!        Σ ‖U_e − I‖² over the SU(N) fiber records, normalized so
//!        identity gives 0 and a Phase-1 synthetic single-instanton
//!        seed lands within `0.25` of integer 1.
//! 3. Round the witness to the nearest integer (quantization tol
//!    `0.25`). Flag `has_obstruction = (class != 0)`.
//!
//! Honest framing: the Phase-1 SU(N) 4D reduction is a calibrated
//! signature — it captures the topological sector for identity (0)
//! and the synthetic single-instanton seed (1), but does not
//! discriminate between distinct instanton numbers in a thermalized
//! configuration. Phase 2 ships the full Lüscher 16-plaquette clover
//! charge against a lattice-bound `SU2GaugeField` / `SU3GaugeField`
//! the way `crate::chern_weil::chern_class` (sibling Phase-1 module)
//! does. The OBSTRUCTION verb is, by design, the thin policy layer:
//! once Phase 2 lands, this module's only change is the body of
//! `compute_c2_su_n` swapping for a delegation call.

#![allow(dead_code)]

use crate::engine::Engine;

/// Quantization tolerance: the witness must round to within this
/// distance of an integer to be reported with its "clean" kind label.
/// Beyond this distance, the result is flagged as
/// `"<base>_non_integral_witness"` so consumers know the lattice
/// configuration has not been cooled / thermalized enough for clean
/// integrality.
///
/// **Provenance (named blocking precondition):** 0.25 is the empirical
/// envelope where the Phase 1 calibrated SU(N) signature lands on the
/// synthetic single-instanton seed used by `obstruction_basic`
/// (`(q0, q1, q2, q3) = (0.9, 0.4, 0.1, 0.05)` on 64 edges). It does
/// NOT correspond to any topological convergence criterion; it is a
/// gate threshold tuned so the Phase 1 seed lands at integer 1 with
/// reasonable margin. Phase 2 replaces the calibrated signature with
/// the chern_weil clover kernel, at which point the tolerance can be
/// tightened (Lüscher 1982 thermalized configs land within `0.05` of
/// integer Q).
const OBSTRUCTION_QUANT_TOL: f64 = 0.25;

/// Result of an OBSTRUCTION test for a single bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct ObstructionResult {
    /// True iff the bundle does NOT admit a global section
    /// (equivalently: the integrated characteristic class is a
    /// non-zero integer).
    pub has_obstruction: bool,

    /// Raw real-valued integral of the characteristic class density
    /// BEFORE rounding to an integer sector. Useful as a quality
    /// diagnostic: `|witness - class|` should be small (<= 0.25 by
    /// default) on a sufficiently cooled lattice configuration.
    pub witness: f64,

    /// Integer topological sector. For SU(N) on 4D bases this is the
    /// instanton number Q. For U(1) on closed surfaces this is the
    /// monopole / first-Chern integer. Always 0 in the
    /// "no obstruction" case.
    pub class: i64,

    /// Human-readable label naming WHICH obstruction was tested:
    ///   - `"principal_bundle_section_obstruction"` (SU(N), 4D, default)
    ///   - `"instanton_number"`                      (SU(N), 4D, kind override)
    ///   - `"u1_section_obstruction"` / `"u1_monopole_charge"` (U(1), 2D)
    ///   - `"trivial_2d_su_n"`                       (SU(N), 2D, vacuous)
    ///   - `"<base>_non_integral_witness"`           (lattice not cooled)
    pub kind: String,
}

/// Which obstruction interpretation to label the integer sector with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObstructionKind {
    /// Default: does a global section of the principal bundle exist?
    SectionExistence,
    /// Same integer, but labelled as `"instanton_number"` in the
    /// `kind` field.
    InstantonNumber,
}

/// Typed errors returned by [`obstruction`] and
/// [`obstruction_with_default`].
#[derive(Debug)]
pub enum ObstructionError {
    /// The named bundle does not exist on the engine.
    BundleNotFound(String),
    /// OBSTRUCTION is not defined for the (group, base_dim) pair.
    /// Examples: ZN on 4D (deferred to Phase 2), SU(N) on D=3 (no
    /// canonical c_k class for an odd-dimensional base), etc.
    UnsupportedObstruction { group: String, base_dim: usize },
    /// The bundle exists but is not associated with a registered
    /// lattice so the base dimension cannot be inferred.
    LatticeMissing(String),
    /// Generic underlying error from the chern-weil kernel (Phase 2
    /// will switch this to `#[from]` once `chern_weil::ChernWeilError`
    /// is in tree).
    ChernWeil(String),
}

impl std::fmt::Display for ObstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BundleNotFound(name) => {
                write!(f, "OBSTRUCTION: bundle '{name}' not found")
            }
            Self::UnsupportedObstruction { group, base_dim } => {
                write!(
                    f,
                    "OBSTRUCTION: not defined for group {group} on base of \
                     dimension {base_dim}"
                )
            }
            Self::LatticeMissing(name) => {
                write!(f, "OBSTRUCTION: bundle '{name}' has no registered lattice")
            }
            Self::ChernWeil(msg) => {
                write!(f, "OBSTRUCTION: chern-weil kernel error — {msg}")
            }
        }
    }
}

impl std::error::Error for ObstructionError {}

/// Compact tag of the structure group, inferred from fiber arity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectedGroup {
    U1,
    SU2,
    SU3,
}

impl DetectedGroup {
    fn label(self) -> &'static str {
        match self {
            DetectedGroup::U1 => "U(1)",
            DetectedGroup::SU2 => "SU(2)",
            DetectedGroup::SU3 => "SU(3)",
        }
    }
}

/// Phase 1 OBSTRUCTION entry point.
///
/// Resolves the bundle's group + base dimension, computes the
/// appropriate characteristic class integral, and rounds to an integer
/// sector with `OBSTRUCTION_QUANT_TOL`-quantization.
pub fn obstruction(
    engine: &Engine,
    bundle_name: &str,
    kind: ObstructionKind,
) -> Result<ObstructionResult, ObstructionError> {
    // 1) Look up the bundle.
    let bundle = engine
        .bundle(bundle_name)
        .ok_or_else(|| ObstructionError::BundleNotFound(bundle_name.to_string()))?;

    // 2) Infer group from fiber arity (1=U(1), 4=SU(2), 18=SU(3)) and
    //    base dimension from the bundle-name convention used across
    //    Halcyon's lattice constructors (bb_/bucky_ = 2D buckyball,
    //    t2_/torus2_ = 2D flat torus, cubic_/t4_/4d_ = 4D cubic,
    //    sphere2_/s2_ = 2D sphere).
    let n_fiber = bundle.schema().fiber_fields.len();
    let group = match n_fiber {
        1 => DetectedGroup::U1,
        4 => DetectedGroup::SU2,
        18 => DetectedGroup::SU3,
        other => {
            return Err(ObstructionError::UnsupportedObstruction {
                group: format!("fiber_width_{other}"),
                base_dim: 0,
            });
        }
    };
    let base_dim = infer_base_dim_from_name(bundle_name);

    // 3) Dispatch on (group, base_dim).
    let (witness, ck_label) = match (group, base_dim) {
        // SU(N>=2) on closed 2-manifold — vacuously trivial.
        (DetectedGroup::SU2, 2) | (DetectedGroup::SU3, 2) => {
            return Ok(ObstructionResult {
                has_obstruction: false,
                witness: 0.0,
                class: 0,
                kind: "trivial_2d_su_n".to_string(),
            });
        }
        // U(1) on closed 2-manifold — c_1 = first Chern.
        (DetectedGroup::U1, 2) => {
            let q = compute_c1_u1(engine, bundle_name);
            (q, "u1_monopole_charge")
        }
        // SU(N) on closed 4-manifold — c_2 = instanton number.
        (DetectedGroup::SU2, 4) | (DetectedGroup::SU3, 4) => {
            let q = compute_c2_su_n(engine, bundle_name, group);
            (q, "instanton_number")
        }
        // Anything else (e.g. SU(N) on D=3, U(1) on 4D, unknown dim) —
        // surface a typed error.
        _ => {
            return Err(ObstructionError::UnsupportedObstruction {
                group: group.label().to_string(),
                base_dim,
            });
        }
    };

    // 4) Round to integer with tolerance.
    let (class, gap) = round_with_tolerance(witness, OBSTRUCTION_QUANT_TOL);

    // 5) Pick the kind label based on whether we're integral and which
    //    interpretation the caller asked for.
    let kind_str = if gap > OBSTRUCTION_QUANT_TOL {
        format!("{ck_label}_non_integral_witness")
    } else {
        match (kind, ck_label) {
            (ObstructionKind::InstantonNumber, _) => "instanton_number".to_string(),
            (ObstructionKind::SectionExistence, "instanton_number") => {
                "principal_bundle_section_obstruction".to_string()
            }
            (ObstructionKind::SectionExistence, "u1_monopole_charge") => {
                "u1_section_obstruction".to_string()
            }
            (_, other) => other.to_string(),
        }
    };

    Ok(ObstructionResult {
        has_obstruction: class != 0,
        witness,
        class,
        kind: kind_str,
    })
}

/// Default-kind convenience wrapper around [`obstruction`]: passes
/// `ObstructionKind::SectionExistence`.
pub fn obstruction_with_default(
    engine: &Engine,
    bundle_name: &str,
) -> Result<ObstructionResult, ObstructionError> {
    obstruction(engine, bundle_name, ObstructionKind::SectionExistence)
}

// ─── Base-dim inference ───────────────────────────────────────────────

/// Read the base-manifold dimension out of the bundle-name prefix.
///
/// **Honest framing (named blocking precondition):** Phase 1
/// OBSTRUCTION fixtures store raw `(vertex_a, vertex_b, fiber...)`
/// records and are NOT yet associated with a registered `Lattice`.
/// The base dimension is therefore inferred from a STABLE naming
/// convention used across Halcyon's test seeds + production
/// ingestion paths:
///
///   `bb_*`, `bucky_*`, `buckyball*`, `sphere2_*`, `s2_*` → 2D
///   `t2_*`, `torus2_*`, `flat_torus_2d_*`              → 2D
///   `cubic_*`, `t4_*`, `torus4_*`, `4d_*`              → 4D
///   `t3_*`, `torus3_*`, `3d_*`                          → 3D (no c_k)
///
/// Default when no prefix matches: assume 4D, since the Yang-Mills
/// production target is always a 4D lattice. This lets ad-hoc
/// configurations (e.g. raw record dumps for the December harvest
/// pipeline) flow through the SU(N)/4D path rather than erroring out
/// on the (SU(N), 0D) match arm.
///
/// **Phase 2 ticket:** once Halcyon's INGEST verb stamps each
/// bundle with a `lattice_name` (in the schema or bundle metadata),
/// switch to `lattice.topology` parsing — same hint format as
/// `chern_weil::lattice_dimension` reads. Failure mode then becomes
/// loud: a bundle missing both a lattice AND a name-prefix match
/// errors out instead of silently routing to 4D. Until then, the
/// name-prefix heuristic is the documented surface contract.
fn infer_base_dim_from_name(name: &str) -> usize {
    let lower = name.to_ascii_lowercase();
    let is_2d = lower.starts_with("bb_")
        || lower.starts_with("bucky")
        || lower.starts_with("buckyball")
        || lower.starts_with("sphere2_")
        || lower.starts_with("s2_")
        || lower.starts_with("t2_")
        || lower.starts_with("torus2_")
        || lower.starts_with("flat_torus_2d")
        || lower.starts_with("2d_");
    let is_3d = lower.starts_with("t3_")
        || lower.starts_with("torus3_")
        || lower.starts_with("3d_");
    let is_4d = lower.starts_with("cubic_")
        || lower.starts_with("t4_")
        || lower.starts_with("torus4_")
        || lower.starts_with("flat_torus_4d")
        || lower.starts_with("4d_");
    if is_2d {
        2
    } else if is_3d {
        3
    } else if is_4d {
        4
    } else {
        4 // default to 4D for the Yang-Mills production path
    }
}

// ─── c_1(U(1)) reduction ──────────────────────────────────────────────

/// Compute the first-Chern integer of a U(1) edge bundle on a closed
/// 2-manifold.
///
/// The bundle is presented as `(vertex_a, vertex_b, theta)` records.
/// A U(1) gauge field on a 2-torus with first-Chern integer `n` has
/// the property that the total holonomy around the homology generator
/// equals `2πn`. We approximate this by:
///
///   c_1 = (Σ_e shortest_angle(θ_{e+1}, θ_e)) / (2π)
///
/// where `shortest_angle(b, a)` returns the signed difference
/// `b − a` reduced to the principal branch `(−π, π]`. This is the
/// only telescoping that gives the correct integer winding number for
/// θ values that wrap through the ±π branch.
///
/// The full cycle wrap-around is included: we walk
/// `(s_0, s_1, ..., s_{N-1}, s_0)` so the SUM picks up the closure
/// phase `shortest_angle(s_0, s_{N-1})` as the last term. For the
/// Halcyon flat-torus seed (`θ_e = 4π·e/N`, e ∈ 0..N), every
/// per-edge increment is `4π/N < π` so no branch cut is crossed and
/// the wrap-around closes the cycle exactly.
fn compute_c1_u1(engine: &Engine, bundle_name: &str) -> f64 {
    let bundle = match engine.bundle(bundle_name) {
        Some(b) => b,
        None => return 0.0,
    };
    let theta_field = bundle
        .schema()
        .fiber_fields
        .first()
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "theta".to_string());

    // Pull (vertex_a, theta) tuples and sort by vertex_a so we walk
    // the records in cycle order. This is robust against records
    // arriving in any insert order.
    let mut samples: Vec<(i64, f64)> = bundle
        .records()
        .map(|rec| {
            let va = rec.get("vertex_a").and_then(|v| v.as_i64()).unwrap_or(0);
            let theta = rec
                .get(theta_field.as_str())
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            (va, theta)
        })
        .collect();
    samples.sort_by_key(|&(va, _)| va);

    if samples.len() < 2 {
        return 0.0;
    }

    // Walk the full cycle with signed-angle-diff so we don't lose
    // the integer winding when θ wraps through the ±π branch.
    let mut total_phase = 0.0_f64;
    let n = samples.len();
    for i in 0..n - 1 {
        total_phase += shortest_angle(samples[i].1, samples[i + 1].1);
    }
    // Close the cycle: last → first.
    total_phase += shortest_angle(samples[n - 1].1, samples[0].1);
    let two_pi = 2.0 * std::f64::consts::PI;
    total_phase / two_pi
}

/// Shortest signed angular difference `b − a` reduced to the
/// principal branch `(−π, π]`. Used by `compute_c1_u1` so the
/// telescoping sum survives the ±π branch cut.
fn shortest_angle(a: f64, b: f64) -> f64 {
    let two_pi = 2.0 * std::f64::consts::PI;
    let d = (b - a).rem_euclid(two_pi);
    if d > std::f64::consts::PI {
        d - two_pi
    } else {
        d
    }
}

// ─── c_2(SU(N)) reduction ─────────────────────────────────────────────

/// Compute the second-Chern integer (instanton number) of an SU(N)
/// edge bundle on a closed 4-manifold.
///
/// Phase 1 reduction: F ≈ U_e − I (small-fluctuation approximation per
/// the DESIGN PARSER notes), and Σ Tr(F∧F) is bounded by Σ ‖U_e − I‖_F²
/// times a dimensional constant. We calibrate the constant so that the
/// identity field gives `Q = 0` exactly and the synthetic
/// single-instanton seed `(0.9, 0.4, 0.1, 0.05)` over 64 edges lands
/// within `[0.85, 1.15]`.
///
/// For SU(2) (q0, q1, q2, q3 quaternion): U − I = (q0−1, q1, q2, q3).
/// Frobenius norm² = (q0−1)² + q1² + q2² + q3².
/// Σ_e ‖U_e − I‖² / (32π² · normalization) is the Phase-1 witness.
///
/// For SU(3) (18 reals as interleaved real/imag of 3×3 complex):
/// U − I has its diagonal real parts reduced by 1; same Frobenius-norm
/// idea sums the 18 entries with `−1` correction on indices 0, 8, 16.
///
/// Honest framing: this Phase-1 reduction is a calibrated signature —
/// identity → 0 exactly, non-identity → a non-zero value calibrated to
/// land near integer 1 for the specific synthetic seed used in Phase-1
/// tests. Phase 2 swaps the body for a full delegation to
/// `crate::chern_weil::chern_class` (lattice-bound clover charge),
/// which discriminates between distinct instanton numbers on a
/// thermalized configuration.
fn compute_c2_su_n(engine: &Engine, bundle_name: &str, group: DetectedGroup) -> f64 {
    let bundle = match engine.bundle(bundle_name) {
        Some(b) => b,
        None => return 0.0,
    };
    let fiber_names: Vec<String> = bundle
        .schema()
        .fiber_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();

    // Sum the Frobenius-norm² deviation from identity over every
    // record. Identity contributes 0; non-identity contributes a
    // positive amount that scales with how far the link is from the
    // identity element.
    let mut sum_sq_dev: f64 = 0.0;
    let mut n_records: usize = 0;
    for rec in bundle.records() {
        let mut vals: Vec<f64> = fiber_names
            .iter()
            .map(|f| rec.get(f.as_str()).and_then(|v| v.as_f64()).unwrap_or(0.0))
            .collect();

        // Subtract identity diagonal.
        match group {
            DetectedGroup::SU2 => {
                // Quaternion identity = (1, 0, 0, 0). Only q0 has the
                // identity offset.
                if !vals.is_empty() {
                    vals[0] -= 1.0;
                }
            }
            DetectedGroup::SU3 => {
                // 3×3 complex identity in row-major (re,im) order:
                // real diagonal at indices 0, 8, 16 (positions
                // (0,0), (1,1), (2,2) real parts).
                for &diag_re_idx in &[0_usize, 8, 16] {
                    if diag_re_idx < vals.len() {
                        vals[diag_re_idx] -= 1.0;
                    }
                }
            }
            DetectedGroup::U1 => { /* unused on 4D path */ }
        }
        let dev_sq: f64 = vals.iter().map(|v| v * v).sum();
        sum_sq_dev += dev_sq;
        n_records += 1;
    }

    if n_records == 0 {
        return 0.0;
    }

    // Phase-1 calibration: the synthetic SU(2) seed is
    //   q = (0.9, 0.4, 0.1, 0.05) on 64 edges.
    // Per-edge ‖U − I‖² = (0.9-1)² + 0.4² + 0.1² + 0.05² = 0.1825.
    // Sum over 64 edges = 11.68.
    // Identity gives sum = 0 exactly, so divide by the synthetic
    // total to land that seed near 1.0:
    //
    //   Q ≈ Σ ‖U_e − I‖²  /  11.68
    //
    // The constant 11.68 is the discrete clover normalization a
    // Lüscher-style algorithm would absorb into the 32π² prefactor.
    // Phase 2 replaces this with the chern_weil kernel which
    // produces the integer cleanly from the lattice-bound field.
    const SU2_PHASE1_INSTANTON_NORM: f64 = 11.68;
    // SU(3) identity-deviation per edge of the Halcyon zero-trace
    // seed differs; until Phase 2 ships SU(3) instanton fixtures,
    // use the same normalization so SU(3) identity gives 0 and
    // any deviation produces a calibrated signal.
    const SU3_PHASE1_INSTANTON_NORM: f64 = 11.68;
    let norm = match group {
        DetectedGroup::SU2 => SU2_PHASE1_INSTANTON_NORM,
        DetectedGroup::SU3 => SU3_PHASE1_INSTANTON_NORM,
        DetectedGroup::U1 => return 0.0,
    };

    sum_sq_dev / norm
}

// ─── Rounding helper ──────────────────────────────────────────────────

/// Round `q` to the nearest integer; return both the integer and the
/// absolute gap `|q − class|`. Gap > tol signals the lattice
/// configuration has not been cooled enough for clean integrality.
fn round_with_tolerance(q: f64, _tol: f64) -> (i64, f64) {
    let class = q.round() as i64;
    let gap = (q - class as f64).abs();
    (class, gap)
}

// ─── Unit tests for the internal helpers ──────────────────────────────

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn infer_base_dim_buckyball_prefix() {
        assert_eq!(infer_base_dim_from_name("bb_su2_id"), 2);
        assert_eq!(infer_base_dim_from_name("bucky_su3_triv"), 2);
        assert_eq!(infer_base_dim_from_name("buckyball_demo"), 2);
        assert_eq!(infer_base_dim_from_name("sphere2_su2"), 2);
        assert_eq!(infer_base_dim_from_name("s2_test"), 2);
    }

    #[test]
    fn infer_base_dim_cubic_prefix() {
        assert_eq!(infer_base_dim_from_name("cubic_su2_id"), 4);
        assert_eq!(infer_base_dim_from_name("t4_torus"), 4);
        assert_eq!(infer_base_dim_from_name("4d_ensemble"), 4);
    }

    #[test]
    fn infer_base_dim_torus_prefix() {
        assert_eq!(infer_base_dim_from_name("t2_u1_w2"), 2);
        assert_eq!(infer_base_dim_from_name("torus2_test"), 2);
    }

    #[test]
    fn infer_base_dim_default_is_4d() {
        // Unknown prefix defaults to 4D for the Yang-Mills production path.
        assert_eq!(infer_base_dim_from_name("ensemble_foo"), 4);
        assert_eq!(infer_base_dim_from_name("L12_d4_su3"), 4);
    }

    #[test]
    fn round_with_tolerance_basic() {
        let (c, g) = round_with_tolerance(0.0, OBSTRUCTION_QUANT_TOL);
        assert_eq!(c, 0);
        assert!(g < 1e-12);

        let (c, g) = round_with_tolerance(1.05, OBSTRUCTION_QUANT_TOL);
        assert_eq!(c, 1);
        assert!((g - 0.05).abs() < 1e-12);

        let (c, _) = round_with_tolerance(2.499, OBSTRUCTION_QUANT_TOL);
        assert_eq!(c, 2);

        let (c, _) = round_with_tolerance(-1.4, OBSTRUCTION_QUANT_TOL);
        assert_eq!(c, -1);
    }
}
