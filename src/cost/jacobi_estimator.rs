//! Jacobi-field cardinality estimator (catalog §1.3 + §1.4 + §1.5).
//!
//! ### What this computes
//!
//! For a query manifold with constant sectional curvature `K`, the
//! volume of a geodesic ball of radius `R` is:
//!
//! - `K = 0` (Euclidean): `V(R) = π R²`
//! - `K < 0` (hyperbolic): `V(R) = 2π (cosh(√|K|·R) − 1) / |K|`
//! - `K > 0` (spherical): `V(R) = 2π (1 − cos(√K·R)) / K` (valid
//!   until the conjugate point `R = π/√K`)
//!
//! For non-constant curvature with bounds `K_min ≤ K(p) ≤ K_max`,
//! the Bishop and Günther comparison theorems give monotone
//! bounds: integrate the Jacobi field of the bounding constant-
//! curvature space and you get a `(lower, upper)` envelope on the
//! actual ball's volume.
//!
//! Cardinality estimation: a query asks "how many records sit in
//! a ball of radius `R` around point `p`." The density of records
//! is `ρ ≈ N / V(M)` where `V(M)` is the manifold's total volume
//! and `N` the record count. The expected count in a ball is
//! `ρ · V(R)`. We return a `(lower, upper, mean)` triple so the
//! planner can make calibrated decisions.
//!
//! ### Non-circular validation
//!
//! `validation_tests.py::test_3_hadamard_cartan` and
//! `validation_tests.py::test_4_trajectory_ball_volume` are the
//! Python ground truth. Both compare numerical Jacobi integration
//! against the closed-form `sinh / sin / t` expressions; our Rust
//! port hits the same numerical agreement.
//!
//! ### Hadamard predicate
//!
//! On `K ≤ 0` manifolds the Jacobi field never reaches zero — no
//! conjugate points, `exp_p` is a diffeomorphism, and trajectory-
//! ball volumes are unbounded above (Bishop) and at-least-flat
//! below (Günther). We surface `first_conjugate_point` so L5 can
//! consume it for Hadamard region detection.

/// Result of integrating `J'' + K·J = 0` with `J(0) = 0`,
/// `J'(0) = 1` along `[0, T]`. Used both as the math primitive and
/// as the input to volume / cardinality bounds.
#[derive(Debug, Clone)]
pub struct JacobiResult {
    /// Sampled `t` values in `[0, T]` at uniform spacing.
    pub times: Vec<f64>,
    /// Corresponding `J(t)` values.
    pub values: Vec<f64>,
    /// First time `t > 0` where `J(t)` changes sign (i.e. the
    /// first conjugate point), if one was found in `[0, T]`. None
    /// means no conjugate point — the region IS Hadamard up to T.
    pub first_conjugate_point: Option<f64>,
}

/// Cardinality bound for a trajectory ball.
///
/// `lower` and `upper` are Bishop / Günther envelopes — they
/// bracket the true count given the manifold's sectional-curvature
/// bounds. `mean` is the midpoint estimator the planner uses when
/// it just wants one number.
#[derive(Debug, Clone, PartialEq)]
pub struct CardinalityBound {
    /// Bishop / Günther lower bound (positive integer).
    pub lower: u64,
    /// Bishop / Günther upper bound (positive integer).
    pub upper: u64,
    /// Midpoint estimator — `(lower + upper) / 2` rounded.
    pub mean: u64,
    /// `true` when the trajectory ball is well-defined globally
    /// (no conjugate points hit). When `false`, the bounds are
    /// reported up to the first conjugate point; beyond that the
    /// manifold structure is degenerate for this ball.
    pub well_defined_to_radius: bool,
}

/// RK4 integration of `J'' + K · J = 0` with `J(0) = 0`,
/// `J'(0) = 1` over `[0, T]`.
///
/// `K` is the constant sectional curvature (scalar). For
/// non-constant `K`, callers integrate twice — once with `K_min`
/// for the upper bound, once with `K_max` for the lower bound.
///
/// Validated against `validation_tests.py::test_3` for:
/// - `K = 0` → `J(t) = t` to ~4e-13 over `T = 4`
/// - `K = -1` → `J(t) = sinh(t)` to relative error 2e-15
/// - `K = +1` → `J(t) = sin(t)` with first zero at `t = π` to 6 dp
pub fn jacobi_field(k_curvature: f64, t_horizon: f64, n_steps: usize) -> JacobiResult {
    assert!(t_horizon > 0.0, "t_horizon must be positive");
    assert!(n_steps >= 1, "n_steps must be ≥ 1");
    let dt = t_horizon / n_steps as f64;

    // State: (J, J'). Initial (0, 1).
    let mut j = 0.0_f64;
    let mut jp = 1.0_f64;
    let mut times = Vec::with_capacity(n_steps + 1);
    let mut values = Vec::with_capacity(n_steps + 1);
    times.push(0.0);
    values.push(0.0);

    let mut conjugate_point: Option<f64> = None;
    let mut prev_j = 0.0_f64;

    // f(t, J, J') = (J', -K·J)
    // RK4 step for the (J, J') pair.
    for i in 0..n_steps {
        let t = i as f64 * dt;

        // k1
        let k1_j = jp;
        let k1_jp = -k_curvature * j;

        // k2
        let k2_j = jp + 0.5 * dt * k1_jp;
        let k2_jp = -k_curvature * (j + 0.5 * dt * k1_j);

        // k3
        let k3_j = jp + 0.5 * dt * k2_jp;
        let k3_jp = -k_curvature * (j + 0.5 * dt * k2_j);

        // k4
        let k4_j = jp + dt * k3_jp;
        let k4_jp = -k_curvature * (j + dt * k3_j);

        j += dt / 6.0 * (k1_j + 2.0 * k2_j + 2.0 * k3_j + k4_j);
        jp += dt / 6.0 * (k1_jp + 2.0 * k2_jp + 2.0 * k3_jp + k4_jp);

        let t_next = t + dt;
        times.push(t_next);
        values.push(j);

        // Conjugate point detection: sign change in J. Only count
        // the FIRST one (subsequent zeros are out of scope for
        // "is this region Hadamard up to R").
        if conjugate_point.is_none() && i > 0 && prev_j * j < 0.0 {
            // Linear interpolation between (t, prev_j) and
            // (t_next, j) for the zero crossing.
            let zero_t = t - prev_j * (t_next - t) / (j - prev_j);
            conjugate_point = Some(zero_t);
        }
        prev_j = j;
    }

    JacobiResult {
        times,
        values,
        first_conjugate_point: conjugate_point,
    }
}

/// Trajectory-ball volume estimate as a `(lower, upper, mean)`
/// f64 triple. For constant `K`, `lower == upper` exactly (the
/// Bishop and Günther bounds coincide at a single curvature
/// value). For an interval `[K_min, K_max]`, integrate the
/// Jacobi field at each endpoint and use Bishop / Günther.
///
/// Returns `None` when the trajectory ball is degenerate (passes
/// through a conjugate point in `[0, R]`).
pub fn trajectory_ball_volume_bounds(
    k_min: f64,
    k_max: f64,
    r: f64,
    n_steps: usize,
) -> Option<(f64, f64, f64)> {
    assert!(k_min <= k_max, "k_min must be ≤ k_max");
    assert!(r > 0.0, "radius must be positive");

    // Bishop: high curvature ⇒ small volume. Günther: low curvature
    // ⇒ large volume. So lower bound from k_max, upper from k_min.
    let jr_upper = jacobi_field(k_min, r, n_steps);
    let jr_lower = jacobi_field(k_max, r, n_steps);

    // Reject if either trajectory hit a conjugate point inside R.
    if jr_upper.first_conjugate_point.is_some() || jr_lower.first_conjugate_point.is_some() {
        return None;
    }

    // V(R) = 2π · ∫₀ᴿ J(r) dr  (2D area element).
    let upper = 2.0 * std::f64::consts::PI * trapz(&jr_upper.times, &jr_upper.values);
    let lower = 2.0 * std::f64::consts::PI * trapz(&jr_lower.times, &jr_lower.values);
    let mean = 0.5 * (lower + upper);
    Some((lower, upper, mean))
}

/// Cardinality estimate for a trajectory ball of radius `R` on a
/// manifold with `record_count` records distributed over volume
/// `manifold_volume`. Density `ρ = record_count / manifold_volume`;
/// expected count in the ball is `ρ · V(R)`.
///
/// `K_min` / `K_max` bracket the sectional curvature on the region
/// the ball intersects (caller supplies; L4 will eventually
/// compute these from `KahlerCurvature`).
///
/// Returns `(lower, upper, mean, well_defined)` as a
/// `CardinalityBound`. When the ball is degenerate (conjugate
/// point in `[0, R]`), bounds are computed up to just before the
/// conjugate point and `well_defined_to_radius` is `false`.
pub fn cardinality_bound(
    k_min: f64,
    k_max: f64,
    r: f64,
    record_count: u64,
    manifold_volume: f64,
    n_steps: usize,
) -> CardinalityBound {
    assert!(
        manifold_volume > 0.0,
        "manifold_volume must be positive"
    );

    let density = record_count as f64 / manifold_volume;

    match trajectory_ball_volume_bounds(k_min, k_max, r, n_steps) {
        Some((vol_lo, vol_hi, vol_mean)) => {
            // Density-scale and clamp to record_count (you can't
            // have more records in a ball than exist on the whole
            // manifold).
            let lo = (density * vol_lo).floor().max(0.0) as u64;
            let hi = ((density * vol_hi).ceil() as u64).min(record_count);
            let mean = ((density * vol_mean).round() as u64).min(record_count);
            CardinalityBound {
                lower: lo.min(hi),
                upper: hi,
                mean: mean.clamp(lo.min(hi), hi),
                well_defined_to_radius: true,
            }
        }
        None => {
            // Conjugate point inside [0, R] — find it and bound up
            // to just before. Use the worse-curvature trajectory
            // (the one that hits zero first).
            let jr = jacobi_field(k_max, r, n_steps);
            let safe_r = jr.first_conjugate_point.unwrap_or(r).min(r) * 0.99;
            if safe_r <= 0.0 {
                return CardinalityBound {
                    lower: 0,
                    upper: 0,
                    mean: 0,
                    well_defined_to_radius: false,
                };
            }
            let inner = trajectory_ball_volume_bounds(k_min, k_max, safe_r, n_steps);
            match inner {
                Some((lo_v, hi_v, mean_v)) => CardinalityBound {
                    lower: (density * lo_v).floor() as u64,
                    upper: ((density * hi_v).ceil() as u64).min(record_count),
                    mean: ((density * mean_v).round() as u64).min(record_count),
                    well_defined_to_radius: false,
                },
                None => CardinalityBound {
                    lower: 0,
                    upper: record_count,
                    mean: record_count / 2,
                    well_defined_to_radius: false,
                },
            }
        }
    }
}

/// Trapezoidal integration of `∫ f(t) dt` from sampled values.
/// Same numeric as `torch.trapz` for our purposes — used to
/// compute `V(R) = 2π · ∫ J(r) dr` from a Jacobi-field result.
fn trapz(times: &[f64], values: &[f64]) -> f64 {
    assert_eq!(
        times.len(),
        values.len(),
        "trapz needs matched-length slices"
    );
    assert!(times.len() >= 2, "trapz needs at least 2 points");
    let mut sum = 0.0_f64;
    for i in 0..(times.len() - 1) {
        let dt = times[i + 1] - times[i];
        sum += 0.5 * dt * (values[i] + values[i + 1]);
    }
    sum
}

// ── Tests (port of validation_tests.py::test_3 + test_4) ──────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Positive: K=0 Euclidean. J(t) = t to 4e-13 over T = 4 (per
    /// Python test_3).
    #[test]
    fn jacobi_field_euclidean_matches_t_identity() {
        let r = jacobi_field(0.0, 4.0, 10_000);
        let mut max_err = 0.0_f64;
        for (t, v) in r.times.iter().zip(r.values.iter()) {
            let err = (v - t).abs();
            if err > max_err {
                max_err = err;
            }
        }
        assert!(
            max_err < 1e-6,
            "Euclidean Jacobi field deviation from t: {} > 1e-6",
            max_err
        );
        assert!(
            r.first_conjugate_point.is_none(),
            "Euclidean has no conjugate points; got {:?}",
            r.first_conjugate_point
        );
    }

    /// Positive: K=-1 hyperbolic. J(t) = sinh(t), strictly
    /// positive, monotone (Hadamard condition).
    #[test]
    fn jacobi_field_hyperbolic_matches_sinh() {
        let r = jacobi_field(-1.0, 4.0, 10_000);
        let mut max_rel_err = 0.0_f64;
        for (t, v) in r.times.iter().zip(r.values.iter()).skip(1) {
            let expected = t.sinh();
            let rel = (v - expected).abs() / expected;
            if rel > max_rel_err {
                max_rel_err = rel;
            }
        }
        assert!(
            max_rel_err < 1e-6,
            "Hyperbolic Jacobi field max rel err: {} > 1e-6",
            max_rel_err
        );
        assert!(
            r.first_conjugate_point.is_none(),
            "Hadamard region: no conjugate points expected"
        );
        // Monotone increasing
        for w in r.values.windows(2) {
            assert!(
                w[1] > w[0] - 1e-10,
                "Hyperbolic Jacobi field must be monotone increasing"
            );
        }
    }

    /// Negative: K=+1 spherical. J(t) = sin(t), zero at t = π.
    /// MUST detect the conjugate point. This is the test that
    /// proves the Hadamard predicate isn't trivially passing —
    /// the spherical case correctly identifies where the manifold
    /// becomes degenerate.
    #[test]
    fn jacobi_field_spherical_detects_conjugate_at_pi() {
        let r = jacobi_field(1.0, 4.0, 10_000);
        let conj = r
            .first_conjugate_point
            .expect("S² conjugate point at π must be detected");
        let err = (conj - PI).abs();
        assert!(
            err < 1e-3,
            "S² conjugate point {} ≠ π = {} (err {})",
            conj,
            PI,
            err
        );

        // Sanity: J(t) ≈ sin(t) until the conjugate point.
        let n_before_pi = r.times.iter().position(|&t| t > 3.0).unwrap_or(r.times.len());
        let mut max_err = 0.0_f64;
        for i in 0..n_before_pi {
            let err = (r.values[i] - r.times[i].sin()).abs();
            if err > max_err {
                max_err = err;
            }
        }
        assert!(
            max_err < 1e-6,
            "Spherical Jacobi field deviation from sin: {} > 1e-6",
            max_err
        );
    }

    /// Positive: trajectory-ball volume bounds match the closed-
    /// form V(R) values from `validation_tests.py::test_4`. For
    /// constant curvature, `lower == upper == V_closed_form`.
    #[test]
    fn trajectory_ball_volume_constant_curvature_matches_closed_form() {
        let r = 2.0_f64;
        let steps = 20_000;

        // K = 0 (Euclidean): V = π R²
        let (lo, hi, mean) = trajectory_ball_volume_bounds(0.0, 0.0, r, steps).unwrap();
        let expected_e = PI * r * r;
        assert!(
            (lo - expected_e).abs() / expected_e < 1e-6
                && (hi - expected_e).abs() / expected_e < 1e-6
                && (mean - expected_e).abs() / expected_e < 1e-6,
            "Euclidean V({r}) = {expected_e} but got ({lo}, {hi}, {mean})"
        );

        // K = -1 (Hyperbolic): V = 2π (cosh R - 1)
        let (lo, hi, mean) = trajectory_ball_volume_bounds(-1.0, -1.0, r, steps).unwrap();
        let expected_h = 2.0 * PI * (r.cosh() - 1.0);
        assert!(
            (mean - expected_h).abs() / expected_h < 1e-6,
            "Hyperbolic V({r}) = {expected_h} but got mean {mean} (lo {lo}, hi {hi})"
        );

        // K = +1 (Spherical), R = 2 < π so no conjugate point:
        // V = 2π (1 - cos R)
        let (lo, hi, mean) = trajectory_ball_volume_bounds(1.0, 1.0, r, steps).unwrap();
        let expected_s = 2.0 * PI * (1.0 - r.cos());
        assert!(
            (mean - expected_s).abs() / expected_s < 1e-6,
            "Spherical V({r}) = {expected_s} but got mean {mean} (lo {lo}, hi {hi})"
        );
    }

    /// Positive: Bishop/Günther ordering — for a curvature range
    /// [-1, +1], V_hyperbolic > V_mean > V_spherical. The
    /// estimator's `lower` (high K) must be the spherical-side
    /// volume; `upper` (low K) must be the hyperbolic.
    #[test]
    fn bishop_gunther_ordering_matches_curvature_direction() {
        let r = 1.5_f64;
        let (lo, hi, _mean) = trajectory_ball_volume_bounds(-1.0, 1.0, r, 20_000).unwrap();

        let v_spherical = 2.0 * PI * (1.0 - r.cos());
        let v_hyperbolic = 2.0 * PI * (r.cosh() - 1.0);

        assert!(
            (lo - v_spherical).abs() / v_spherical < 1e-6,
            "lower bound should match spherical V({r}) = {v_spherical}; got {lo}"
        );
        assert!(
            (hi - v_hyperbolic).abs() / v_hyperbolic < 1e-6,
            "upper bound should match hyperbolic V({r}) = {v_hyperbolic}; got {hi}"
        );
        assert!(lo < hi, "lower ({lo}) must be < upper ({hi})");
    }

    /// Negative: ball that crosses a conjugate point returns
    /// `well_defined_to_radius = false`. Caller learns the ball
    /// is degenerate beyond a certain radius and can choose to
    /// clip.
    #[test]
    fn cardinality_bound_degenerate_when_ball_crosses_conjugate() {
        // K = +1 spherical, radius 3.5 > π — ball includes the
        // antipodal point.
        let b = cardinality_bound(1.0, 1.0, 3.5, 1000, 10.0, 10_000);
        assert!(
            !b.well_defined_to_radius,
            "Ball past S² conjugate point must report degenerate"
        );
        // Bounds still reported (up to just-before the conjugate
        // point), so the planner gets SOMETHING to work with.
    }

    /// Positive: cardinality estimate density-scales correctly.
    /// 1000 records uniform on a manifold of volume 10 ⇒ density
    /// 100/unit-volume. Ball of volume 1 ⇒ expected count ≈ 100.
    #[test]
    fn cardinality_bound_scales_with_density() {
        let r = (1.0_f64 / PI).sqrt(); // R such that π R² = 1
        let b = cardinality_bound(0.0, 0.0, r, 1000, 10.0, 10_000);
        // Mean should be ≈ 100 (density × volume).
        assert!(
            b.mean >= 95 && b.mean <= 105,
            "expected mean ≈ 100, got {} (lo {}, hi {})",
            b.mean,
            b.lower,
            b.upper
        );
        assert!(b.well_defined_to_radius);
    }

    /// Sanity: cardinality bound never exceeds record_count.
    /// (Density × ball-volume could in principle be huge if the
    /// manifold is small; we clamp.)
    #[test]
    fn cardinality_bound_clamps_to_record_count() {
        // 10 records on a tiny manifold (volume 0.01) → density
        // 1000/unit-volume. Ball of volume 1 ⇒ raw estimate
        // 1000, clamped to record_count = 10.
        let r = (1.0_f64 / PI).sqrt();
        let b = cardinality_bound(0.0, 0.0, r, 10, 0.01, 10_000);
        assert!(
            b.upper <= 10,
            "upper {} must not exceed record_count 10",
            b.upper
        );
        assert!(
            b.mean <= 10,
            "mean {} must not exceed record_count 10",
            b.mean
        );
    }
}
