//! GIGI Encrypt v0.4 — Sprint O.A: I_Aff falsification harness.
//!
//! The Sprint H invariant evaluator (`crate::invariant::evaluate`)
//! provides membership in I_Aff *by construction*: the parser only
//! admits compositions of whitelisted invariant operators, so any
//! query that parses cannot reach into fiber-decrypt paths. This module
//! provides the *complementary runtime falsification harness*:
//! `is_in_iaff_harness(f, values, gauges, tolerance)` numerically
//! checks whether a candidate scalar function `f: &[f64] → f64` is
//! invariant under a sample of affine gauges.
//!
//! **The harness is necessary but not sufficient** for I_Aff
//! membership: a function that happens to agree on the sampled gauges
//! but breaks on others would pass. Membership proof comes from the
//! parser — see `theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md`
//! §Sprint O for the precise statement and the generating-set question
//! (open as of v0.4).
//!
//! Used by:
//!  - **v0.4 credential issuance** (`crate::credentials`) — operators
//!    issuing a `QueryCredential` for a custom query callback can run
//!    the harness as a sanity gate before issuing.
//!  - **Sprint O TDD** (`tests/credentials_v0_4.rs::o1`, `o2`) — proves
//!    K, τ, K + K² pass the harness; mean, sum, std fail; the
//!    adversarial K_fake = mean/std² is caught.
//!
//! **Generating set** (parser-admitted vocabulary):
//!
//!   G_IAff = { K, λ_1, ⟨Hol⟩, τ, β_0, β_1 } + polynomial closure
//!
//! Containment in I_Aff is Theorem 4.X of the encryption paper;
//! whether G_IAff *generates* I_Aff is the v0.5 open problem.

/// Davis dispersion ratio K = Var(v) / range(v)², computed on a raw
/// value slice. Gauge-invariant under Aff(ℝ)ᵏ by Theorem 3.5 of the
/// encryption paper. Returns 0.0 if the range is zero (degenerate
/// bundle — all values identical).
///
/// This is the value-level companion to
/// `crate::curvature::scalar_curvature` (which operates on a
/// `BundleStore`). Both compute the same quantity; this slice version
/// is what the IAff harness uses to evaluate candidate queries.
pub fn compute_k(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut max = f64::NEG_INFINITY;
    let mut min = f64::INFINITY;
    let mut sum = 0.0_f64;
    for &x in v {
        if x > max {
            max = x;
        }
        if x < min {
            min = x;
        }
        sum += x;
    }
    let range = max - min;
    if range.abs() < 1e-12 {
        return 0.0;
    }
    let mean = sum / (v.len() as f64);
    let mut var = 0.0_f64;
    for &x in v {
        let d = x - mean;
        var += d * d;
    }
    var /= v.len() as f64;
    var / (range * range)
}

/// Record count τ. Trivially gauge-invariant — record count is unchanged
/// by any deterministic encryption.
pub fn compute_tau(v: &[f64]) -> f64 {
    v.len() as f64
}

/// **I_Aff falsification harness**: numerically checks whether `f` is
/// gauge-invariant on `values` under each tested affine gauge `(a, b)`.
///
/// Returns `(passes, max_relative_error)`:
///   - `passes = true` iff for every (a, b) in `gauges`,
///     `|f(values) − f(a·values + b)| / (|f(values)| + 1e-12) < tolerance`.
///   - `max_relative_error` is the worst case across all tested gauges
///     (useful for diagnostics — a function with rel_err = 1e-3 might
///     be an *approximate* invariant, e.g. with numerical drift).
///
/// **Necessary but not sufficient** for I_Aff membership. Pass = the
/// function survived a sample of gauges; it does NOT mean the function
/// is provably invariant under every g ∈ Aff(ℝ)ᵏ.
///
/// Typical tolerance: `1e-6` (matches the value used in the Python
/// validation oracle and spec §Sprint O TDD).
pub fn is_in_iaff_harness<F>(
    f: &F,
    values: &[f64],
    gauges: &[(f64, f64)],
    tolerance: f64,
) -> (bool, f64)
where
    F: Fn(&[f64]) -> f64,
{
    let f_plain = f(values);
    let denom = f_plain.abs() + 1e-12;
    let mut max_rel_err = 0.0_f64;
    for &(a, b) in gauges {
        let encrypted: Vec<f64> = values.iter().map(|v| a * v + b).collect();
        let f_enc = f(&encrypted);
        let rel = (f_plain - f_enc).abs() / denom;
        if rel > max_rel_err {
            max_rel_err = rel;
        }
    }
    (max_rel_err < tolerance, max_rel_err)
}

// ───────────────────────────────────────────────────────────────────
// Unit tests
// ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<f64> {
        (0..100).map(|i| (i as f64) * 0.7 + 1.3).collect()
    }

    #[test]
    fn compute_k_is_zero_for_constant_bundle() {
        assert_eq!(compute_k(&[5.0; 10]), 0.0);
    }

    #[test]
    fn compute_k_is_positive_for_varying_bundle() {
        assert!(compute_k(&sample()) > 0.0);
    }

    #[test]
    fn compute_tau_returns_record_count() {
        assert_eq!(compute_tau(&sample()), 100.0);
    }

    #[test]
    fn k_passes_harness_under_random_gauges() {
        let v = sample();
        let gauges = vec![(2.5, 100.0), (-1.3, 7.0), (0.001, -500.0)];
        let (ok, err) = is_in_iaff_harness(&|v: &[f64]| compute_k(v), &v, &gauges, 1e-6);
        assert!(ok, "K should pass harness, max_rel_err = {}", err);
    }

    #[test]
    fn mean_fails_harness() {
        let v = sample();
        let mean = |v: &[f64]| v.iter().sum::<f64>() / (v.len() as f64);
        let (ok, _) = is_in_iaff_harness(&mean, &v, &[(2.5, 100.0)], 1e-6);
        assert!(!ok, "mean must fail the harness under non-trivial gauge");
    }
}
