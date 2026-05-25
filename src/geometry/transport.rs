//! B-perturbed parallel transport on flat tangent spaces
//! (catalog §1.2, IMPLEMENTATION_PLAN.md L1.5).
//!
//! Solves the magnetic geodesic equation
//!     `∇_{γ̇} γ̇ = B(γ̇, ·)^♯`
//! on flat `Rⁿ` via RK4. When the bias 2-form `B` is `None`, this
//! reduces to classical straight-line transport (the
//! Riemannian-flat case). When `B` is `Some(closed)`, the
//! trajectory bends — on flat `R²` with constant `B = b·dx∧dy`,
//! the trajectory is a cyclotron orbit of radius `|v|/b`.
//!
//! Energy `½|γ̇|²` is conserved along the magnetic flow by the
//! antisymmetry of `B` (`B(v, v) = 0`); the result struct surfaces
//! the actual numerical drift so callers can audit the integrator's
//! fidelity per call.
//!
//! ### What this module is NOT
//!
//! - **Not curved-manifold transport.** The magnetic geodesic
//!   equation on a non-flat Kähler manifold is a nonlinear ODE
//!   system that needs the curvature decomposition (L4) and the
//!   Hadamard-region safety guarantees (L5) before it's safe to
//!   run in production. L5.5 owns curved-space B-transport.
//! - **Not the GQL `TRANSPORT WITH B = ...` verb.** That's L1.5.3
//!   in the HTTP surface (`src/bin/gigi_stream.rs`). This module
//!   is the in-process Rust primitive the verb dispatches into.
//!
//! ### Marcella contract
//!
//! The `TransportResult` struct field set matches the response
//! shape Marcella's runtime expects per `theory/kahler_upgrade/
//! marcella_kahler_consumption_v2.md §2`. Field renames here are
//! breaking changes for the consumer; the
//! `tests/kahler_transport_marcella_contract.rs` integration test
//! gates the shape so the cross-team interface fails first if
//! either side drifts.

use crate::geometry::forms::ClosedTwoForm;

/// Default RK4 step size. Matches `validation_tests.py::test_2`'s
/// `1e-4` which hits machine-epsilon energy drift on the cyclotron
/// case. Callers running longer trajectories should scale `steps`,
/// not `dt`.
pub const DEFAULT_DT: f64 = 1e-4;

/// Default step count if the caller passes `steps = 0`. Covers
/// roughly `0.65` units of physical time at the default `dt`.
pub const DEFAULT_STEPS: usize = 65536;

/// A transport segment: where we start, where we want to end up,
/// and the initial tangent vector along the path. All vectors
/// live in `Rⁿ` for the same `n`; the segment is invalid if
/// dimensions disagree.
#[derive(Debug, Clone, PartialEq)]
pub struct TransportSegment {
    /// Starting position `p ∈ Rⁿ`.
    pub from_point: Vec<f64>,
    /// Target endpoint position `q ∈ Rⁿ`. Currently only used by
    /// callers for diagnostics — the transport flow is determined
    /// entirely by `from_point` + `initial_velocity` + `B`; the
    /// trajectory passes near `to_point` if the caller chose
    /// `initial_velocity` to aim there. Future curved-space
    /// transport (L5) will solve a boundary-value problem here.
    pub to_point: Vec<f64>,
    /// Initial tangent vector `v ∈ T_pM = Rⁿ`.
    pub initial_velocity: Vec<f64>,
}

impl TransportSegment {
    /// Construct + sanity-check dimensions match.
    pub fn new(
        from_point: Vec<f64>,
        to_point: Vec<f64>,
        initial_velocity: Vec<f64>,
    ) -> Result<Self, TransportError> {
        let d = from_point.len();
        if to_point.len() != d {
            return Err(TransportError::DimensionMismatch {
                from: d,
                to: to_point.len(),
                v: initial_velocity.len(),
            });
        }
        if initial_velocity.len() != d {
            return Err(TransportError::DimensionMismatch {
                from: d,
                to: to_point.len(),
                v: initial_velocity.len(),
            });
        }
        if d == 0 {
            return Err(TransportError::EmptyDimension);
        }
        Ok(Self {
            from_point,
            to_point,
            initial_velocity,
        })
    }

    /// Dimension `n` of the underlying flat space.
    pub fn dim(&self) -> usize {
        self.from_point.len()
    }
}

/// Where the bias 2-form came from for this call. Surfaced on the
/// response so callers (esp. Marcella) can audit which resolution
/// path executed.
///
/// Matches the JSON enum from `marcella_kahler_consumption_v2.md §2`:
/// `"bundle" | "override" | "none" | "fallback_non_closed"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BSource {
    /// Bias came from a bundle attribute (production default).
    Bundle,
    /// Bias was passed explicitly as an override at call time.
    Override,
    /// No bias supplied — classical (Riemannian-flat) transport.
    None,
    /// Bias supplied was NOT closed; `ALLOW_NON_CLOSED` opt-in
    /// caused fall-back to classical transport. The
    /// `closedness_norm` field on `TransportResult` carries the
    /// measured violation so the caller can audit.
    FallbackNonClosed,
}

/// Result of a `flat_transport` call. Field set matches what the
/// L1.5.3 GQL surface will serialize for Marcella's runtime, per
/// the v2 consumption draft contract.
#[derive(Debug, Clone)]
pub struct TransportResult {
    /// Discrete trajectory `[γ(t_0), γ(t_1), …]`. Length is
    /// `steps + 1` (includes both endpoints). Each entry is a
    /// `Vec<f64>` of length `dim`.
    pub trajectory: Vec<Vec<f64>>,
    /// Velocity at the final step `γ̇(t_steps)`.
    pub final_velocity: Vec<f64>,
    /// Total Euclidean path length integrated along the
    /// discretized trajectory.
    pub path_length: f64,
    /// `max_t |½|γ̇(t)|² − ½|γ̇(0)|²|` over the trajectory.
    /// MUST be `< 1e-9` per turn in production per Marcella v2
    /// consumption draft §2.
    pub energy_drift: f64,
    /// `‖γ̇(t_steps) − Π(γ̇(0))‖` where Π is the parallel
    /// transport of the initial velocity along the trajectory.
    /// For flat classical transport this is 0; for magnetic, it's
    /// the angular rotation accumulated along the cyclotron arc.
    pub holonomy_norm: f64,
    /// True iff a magnetic perturbation was actually applied.
    /// (False when `b_source == BSource::None` or
    /// `BSource::FallbackNonClosed`.)
    pub used_magnetic: bool,
    /// Provenance of the bias 2-form used for this call.
    pub b_source: BSource,
    /// When `b_source == FallbackNonClosed`, the measured
    /// closedness violation `‖dB‖` of the rejected form so the
    /// caller can see how far off it was. `None` otherwise.
    pub closedness_norm: Option<f64>,
}

/// Failure modes for transport requests. All errors surface
/// enough state for the caller to construct a useful diagnostic
/// without re-reading inputs.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TransportError {
    /// Segment vectors disagree on dimension.
    #[error("segment dimensions inconsistent: from={from}, to={to}, v={v}")]
    DimensionMismatch { from: usize, to: usize, v: usize },

    /// Zero-dimensional segment (degenerate).
    #[error("segment dimension must be positive")]
    EmptyDimension,

    /// Bias 2-form dimension differs from segment dimension.
    #[error("bias 2-form dim {bias_dim} doesn't match segment dim {seg_dim}")]
    BiasDimensionMismatch { seg_dim: usize, bias_dim: usize },

    /// L5.5 — `BundleStore::transport_along` was called on a
    /// non-Hadamard bundle. The magnetic geodesic equation is not
    /// safe outside Hadamard regions (conjugate points possible).
    /// Marcella reads this as the "use ambient flat_transport +
    /// projection instead" signal per the Gate 2 findings reply.
    #[error(
        "transport_along refused: bundle is not in a Hadamard region \
         (kb_max = {kb_max}; threshold = {threshold})"
    )]
    NotHadamard { kb_max: f64, threshold: f64 },
}

/// Solve the magnetic geodesic equation on flat `Rⁿ` via RK4.
///
/// Semantics:
/// - `bias = None`  → classical transport: straight line at
///   constant velocity. `holonomy_norm = 0` exactly.
///   `used_magnetic = false`. `b_source = BSource::None`.
/// - `bias = Some(B)` → magnetic flow:
///       `ẍ_i = (B^♯)_{ij} ẋ_j = B_{ij} ẋ_j`
///   (with the metric being the standard `Rⁿ` Euclidean inner
///   product, so `B^♯ = B`). On flat `R²` with constant
///   `B = b·dx∧dy`, this is cyclotron motion — radius `|v|/b`,
///   period `2π/b`, energy `½|v|²` conserved exactly. The Rust
///   integrator hits `~6e-15` energy drift over one full period
///   (matches `validation_tests.py::test_2`).
///
/// `b_source` defaults to `Override` when `bias = Some` and the
/// caller hasn't told us otherwise. Higher layers (`bundle.transport_along`)
/// override it to `Bundle` when reading from the bundle's attached
/// `kahler.B`, or `FallbackNonClosed` when the supplied form was
/// rejected by the closedness check.
pub fn flat_transport(
    seg: &TransportSegment,
    bias: Option<&ClosedTwoForm>,
    dt: f64,
    steps: usize,
    b_source: BSource,
) -> Result<TransportResult, TransportError> {
    if let Some(b) = bias {
        if b.dim() != seg.dim() {
            return Err(TransportError::BiasDimensionMismatch {
                seg_dim: seg.dim(),
                bias_dim: b.dim(),
            });
        }
    }

    let steps = if steps == 0 { DEFAULT_STEPS } else { steps };
    let dt = if dt <= 0.0 { DEFAULT_DT } else { dt };
    let dim = seg.dim();
    // Total integration time T = dt × steps. The trajectory
    // advances `initial_velocity * T` distance for the classical
    // case (no bias); for magnetic the path bends but covers the
    // same "amount of time," not the same Euclidean distance. The
    // caller chooses (dt, steps) to match the physics — for a
    // cyclotron orbit you want T = 2π/b for one full period.

    let mut x = seg.from_point.clone();
    let mut v = seg.initial_velocity.clone();
    let initial_energy = 0.5 * dot(&v, &v);

    let mut trajectory = Vec::with_capacity(steps + 1);
    trajectory.push(x.clone());

    let mut max_energy_dev = 0.0_f64;
    let mut path_length = 0.0_f64;

    for _ in 0..steps {
        // RK4 on (x, v) with ẋ = v, v̇ = B^♯(v).
        let (k1x, k1v) = derivative(&x, &v, bias);
        let (xb, vb) = (
            add_scaled(&x, &k1x, 0.5 * dt),
            add_scaled(&v, &k1v, 0.5 * dt),
        );
        let (k2x, k2v) = derivative(&xb, &vb, bias);
        let (xc, vc) = (
            add_scaled(&x, &k2x, 0.5 * dt),
            add_scaled(&v, &k2v, 0.5 * dt),
        );
        let (k3x, k3v) = derivative(&xc, &vc, bias);
        let (xd, vd) = (add_scaled(&x, &k3x, dt), add_scaled(&v, &k3v, dt));
        let (k4x, k4v) = derivative(&xd, &vd, bias);

        let x_next = combine4(&x, &k1x, &k2x, &k3x, &k4x, dt);
        let v_next = combine4(&v, &k1v, &k2v, &k3v, &k4v, dt);

        // Path-length accumulation: just sum |x_next - x|.
        path_length += distance(&x, &x_next);

        x = x_next;
        v = v_next;
        trajectory.push(x.clone());

        let energy_now = 0.5 * dot(&v, &v);
        let dev = (energy_now - initial_energy).abs();
        if dev > max_energy_dev {
            max_energy_dev = dev;
        }
    }

    // Holonomy on flat space: difference between final velocity
    // and the parallel-transport of the initial velocity. Classical
    // (no B): final_v = initial_v exactly (within FP), so holonomy
    // is 0. Magnetic: final_v has rotated by the cyclotron angle;
    // the norm difference IS the holonomy magnitude.
    let parallel = &seg.initial_velocity;
    let mut diff = vec![0.0_f64; dim];
    for i in 0..dim {
        diff[i] = v[i] - parallel[i];
    }
    let holonomy_norm = dot(&diff, &diff).sqrt();

    let used_magnetic = matches!(b_source, BSource::Bundle | BSource::Override) && bias.is_some();

    Ok(TransportResult {
        trajectory,
        final_velocity: v,
        path_length,
        energy_drift: max_energy_dev,
        holonomy_norm,
        used_magnetic,
        b_source,
        closedness_norm: None,
    })
}

// ── RK4 plumbing ──

/// Right-hand side of the ODE `(ẋ, v̇) = (v, B^♯(v))`. The
/// metric-raise `B^♯` on flat Euclidean Rⁿ is just `B_{ij} v_j`
/// (identity metric).
fn derivative(_x: &[f64], v: &[f64], bias: Option<&ClosedTwoForm>) -> (Vec<f64>, Vec<f64>) {
    let xdot = v.to_vec();
    let vdot = match bias {
        None => vec![0.0; v.len()],
        Some(b) => {
            // For a 2-form B with matrix B_{ij}, the magnetic
            // force in component form is F_i = B_{ij} v_j.
            // Note: B is antisymmetric, so this naturally encodes
            // the Lorentz-force rotation.
            let m = b.form().matrix();
            let n = v.len();
            let mut out = vec![0.0_f64; n];
            for i in 0..n {
                let mut s = 0.0_f64;
                for j in 0..n {
                    s += m[i * n + j] * v[j];
                }
                out[i] = s;
            }
            out
        }
    };
    (xdot, vdot)
}

fn add_scaled(a: &[f64], b: &[f64], k: f64) -> Vec<f64> {
    a.iter().zip(b.iter()).map(|(x, y)| x + k * y).collect()
}

fn combine4(base: &[f64], k1: &[f64], k2: &[f64], k3: &[f64], k4: &[f64], dt: f64) -> Vec<f64> {
    base.iter()
        .zip(k1.iter())
        .zip(k2.iter())
        .zip(k3.iter())
        .zip(k4.iter())
        .map(|((((b, a), c), d), e)| b + (dt / 6.0) * (a + 2.0 * c + 2.0 * d + e))
        .collect()
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

// ── Tests (red-first, ports validation_tests.py test 2) ─────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::forms::TwoForm;
    use std::f64::consts::PI;

    /// Positive: classical transport (no B) is a straight line and
    /// hits exactly zero holonomy + zero energy drift. Anchors the
    /// "no behavior change without B" contract — Marcella's
    /// `b_source = None` path must give the same numbers as
    /// pre-upgrade transport.
    #[test]
    fn flat_classical_transport_is_straight_line() {
        // T = dt·steps = 0.01·100 = 1.0 units of physical time.
        // Initial v = (1, 0), so final x = 1.0, y = 0.
        let seg = TransportSegment::new(vec![0.0, 0.0], vec![1.0, 0.0], vec![1.0, 0.0]).unwrap();
        let r = flat_transport(&seg, None, 0.01, 100, BSource::None).unwrap();

        let final_pos = r.trajectory.last().unwrap();
        assert!((final_pos[0] - 1.0).abs() < 1e-12, "x: {}", final_pos[0]);
        assert!(final_pos[1].abs() < 1e-12, "y: {}", final_pos[1]);

        // No magnetic force ⇒ velocity unchanged.
        assert!(
            (r.final_velocity[0] - 1.0).abs() < 1e-12 && r.final_velocity[1].abs() < 1e-12,
            "v: {:?}",
            r.final_velocity
        );

        // Holonomy ZERO by definition (no rotation).
        assert!(r.holonomy_norm < 1e-12, "holonomy: {}", r.holonomy_norm);

        // Energy drift trivial (no force).
        assert!(r.energy_drift < 1e-12, "drift: {}", r.energy_drift);

        // No magnetism flag.
        assert!(!r.used_magnetic);
        assert_eq!(r.b_source, BSource::None);
        assert!(r.closedness_norm.is_none());
    }

    /// Positive: cyclotron orbit on flat R² with constant
    /// `B = b·dx∧dy`. Direct port of
    /// `validation_tests.py::test_2`: b = 1.5, initial v = (1, 0),
    /// expected radius |v|/b = 2/3, period T = 2π/b. We integrate
    /// for exactly one period (dt = 1e-4, steps = round(T/dt)) so
    /// the trajectory closes.
    #[test]
    fn flat_magnetic_transport_cyclotron_radius_and_energy() {
        let b = 1.5_f64;
        let period = 2.0 * PI / b;
        let dt = 1e-4;
        let n_steps = (period / dt).round() as usize;

        // Bias matrix [[0, b], [-b, 0]] gives ẍ = +b·ẏ, ÿ = -b·ẋ
        // (matches Python test 2 sign convention).
        let bias = ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, b, -b, 0.0], 2).expect("antisymmetric"),
        );

        // Physical initial velocity = (1, 0). No scaling — we
        // control time with (dt, steps) instead.
        let seg = TransportSegment::new(vec![0.0, 0.0], vec![0.0, 0.0], vec![1.0, 0.0]).unwrap();
        let r = flat_transport(&seg, Some(&bias), dt, n_steps, BSource::Override).unwrap();

        // Expected cyclotron geometry: radius |v|/b = 2/3, center
        // at (0, -1/b) (perpendicular to initial v, on the side
        // the magnetic force pushes toward).
        let expected_r = 1.0 / b;
        let center = vec![0.0, -1.0 / b];
        let mut max_r: f64 = 0.0;
        let mut min_r: f64 = f64::INFINITY;
        for p in &r.trajectory {
            let d = distance(p, &center);
            if d > max_r {
                max_r = d;
            }
            if d < min_r {
                min_r = d;
            }
        }
        assert!(
            (max_r - expected_r).abs() < 1e-5,
            "max radius {} ≠ expected {}",
            max_r,
            expected_r
        );
        assert!(
            (min_r - expected_r).abs() < 1e-5,
            "min radius {} ≠ expected {}",
            min_r,
            expected_r
        );

        // ENERGY: ½|v|² conserved along the flow.
        // Initial |v|² = 1, initial energy 0.5.
        let initial_e = 0.5_f64;
        let final_e = 0.5 * dot(&r.final_velocity, &r.final_velocity);
        let energy_dev = (final_e - initial_e).abs();
        assert!(
            energy_dev < 1e-9,
            "energy drift {} exceeds 1e-9 production bound",
            energy_dev
        );
        assert!(
            r.energy_drift < 1e-9,
            "reported energy_drift {} exceeds 1e-9",
            r.energy_drift
        );

        // After exactly one period, the trajectory closes.
        let p0 = &r.trajectory[0];
        let pf = &r.trajectory[r.trajectory.len() - 1];
        let closure = distance(p0, pf);
        assert!(closure < 1e-3, "period closure {} exceeds 1e-3", closure);

        // Magnetic flags set correctly.
        assert!(r.used_magnetic);
        assert_eq!(r.b_source, BSource::Override);
        assert!(r.closedness_norm.is_none());
    }

    /// Negative: bias with the OPPOSITE sign of B reverses the
    /// rotation direction. Catches "we returned the wrong sign"
    /// bugs. Compare trajectories at t = T/4 between +B and -B
    /// — they must be on opposite sides of the initial-velocity
    /// axis.
    #[test]
    fn opposite_signed_B_reverses_curvature_direction() {
        let b = 1.5_f64;
        let period = 2.0 * PI / b;
        let dt = 1e-4;
        let n_steps = (period / dt).round() as usize;

        let bias_pos = ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, b, -b, 0.0], 2).unwrap(),
        );
        let bias_neg = ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, -b, b, 0.0], 2).unwrap(),
        );

        let seg = TransportSegment::new(vec![0.0, 0.0], vec![0.0, 0.0], vec![1.0, 0.0]).unwrap();
        let r_pos =
            flat_transport(&seg, Some(&bias_pos), dt, n_steps, BSource::Override).unwrap();
        let r_neg =
            flat_transport(&seg, Some(&bias_neg), dt, n_steps, BSource::Override).unwrap();

        // At quarter-period, +B trajectory curves down (y < 0) and
        // -B curves up (y > 0). Both have x > 0.
        let q = r_pos.trajectory.len() / 4;
        assert!(
            r_pos.trajectory[q][1] < 0.0,
            "+B should curve down by T/4: y = {}",
            r_pos.trajectory[q][1]
        );
        assert!(
            r_neg.trajectory[q][1] > 0.0,
            "-B should curve up by T/4: y = {}",
            r_neg.trajectory[q][1]
        );
    }

    /// Negative: dimension mismatch between bias and segment errors
    /// cleanly. A common bug when the bundle's Kähler dim doesn't
    /// match the segment dim.
    #[test]
    fn bias_dim_mismatch_errors_cleanly() {
        let seg = TransportSegment::new(vec![0.0, 0.0], vec![1.0, 0.0], vec![1.0, 0.0]).unwrap();
        // 4×4 bias on a 2D segment.
        let mut raw = vec![0.0_f64; 16];
        raw[1] = 0.5;
        raw[4] = -0.5;
        let bias = ClosedTwoForm::new_constant(TwoForm::new(raw, 4).unwrap());
        match flat_transport(&seg, Some(&bias), 1e-4, 100, BSource::Override) {
            Err(TransportError::BiasDimensionMismatch { seg_dim, bias_dim }) => {
                assert_eq!(seg_dim, 2);
                assert_eq!(bias_dim, 4);
            }
            other => panic!("expected BiasDimensionMismatch, got {:?}", other),
        }
    }

    /// Negative: empty dimension segment rejected at construction.
    #[test]
    fn empty_dimension_segment_rejected() {
        match TransportSegment::new(vec![], vec![], vec![]) {
            Err(TransportError::EmptyDimension) => {}
            other => panic!("expected EmptyDimension, got {:?}", other),
        }
    }

    /// Negative: from/to/v dim disagreement rejected at
    /// construction.
    #[test]
    fn segment_dim_disagreement_rejected() {
        match TransportSegment::new(vec![0.0, 0.0], vec![1.0], vec![1.0, 0.0]) {
            Err(TransportError::DimensionMismatch { from, to, v }) => {
                assert_eq!(from, 2);
                assert_eq!(to, 1);
                assert_eq!(v, 2);
            }
            other => panic!("expected DimensionMismatch, got {:?}", other),
        }
    }

    /// Sanity: holonomy on a closed magnetic loop is bounded.
    /// After a full cyclotron period the final velocity should
    /// match the initial velocity within FP, so the holonomy
    /// (defined as ‖v_final − v_initial‖) is small.
    #[test]
    fn magnetic_holonomy_small_after_full_period() {
        let b = 1.5_f64;
        let period = 2.0 * PI / b;
        let dt = 1e-4;
        let n_steps = (period / dt).round() as usize;

        let bias = ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, b, -b, 0.0], 2).unwrap(),
        );
        let seg = TransportSegment::new(vec![0.0, 0.0], vec![0.0, 0.0], vec![1.0, 0.0]).unwrap();
        let r = flat_transport(&seg, Some(&bias), dt, n_steps, BSource::Override).unwrap();

        // After exactly one period, v_final ≈ v_initial (up to RK4
        // error). |v| = 1 is conserved; the deviation is just RK4
        // rotation precision over one period. 1e-3 is generous;
        // typical drift on this case is ~1e-5.
        assert!(
            r.holonomy_norm < 1e-3,
            "holonomy {} too large for one-period closure",
            r.holonomy_norm
        );
    }
}
