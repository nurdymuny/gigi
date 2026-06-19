//! Lie-to-group exponential map + full-step drift for the SU(2)
//! leapfrog integrator.
//!
//! Closes TDD-HAL-IV.5. `matrix_exp_su2_q` is the closed-form
//! Rodrigues exponential on the imaginary-quaternion Lie algebra
//! `su(2)`. For a Lie row `omega = (0, x, y, z)` with `θ = √(x² + y²
//! + z²)`:
//!
//! ```text
//!     exp(omega) = ( cos θ,
//!                    sin θ / θ · x,
//!                    sin θ / θ · y,
//!                    sin θ / θ · z )
//! ```
//!
//! For `θ < 1e-8` the `sin θ / θ` term has a removable singularity;
//! the small-angle path uses the 4th-order Taylor expansion
//!
//! ```text
//!     exp(omega) ≈ ( 1 - θ²/2,
//!                    (1 - θ²/6) · x,
//!                    (1 - θ²/6) · y,
//!                    (1 - θ²/6) · z )
//! ```
//!
//! which is `O(θ⁴)` accurate and keeps the q0 = 1 identity branch
//! byte-exact at `omega = 0`.
//!
//! `drift_step` is the D step of the leapfrog KDK integrator: for
//! each edge it forms `omega = g² · dt · E[e]` (the Lie row, with
//! q0=0 by E's invariant), exponentiates, and LEFT-multiplies the
//! existing U link:
//!
//! ```text
//!     U_new[e] = qmul( exp(omega), U[e] )
//! ```
//!
//! The left multiplication is Halcyon bug #3's fix — the Lie-to-
//! group map acts on the left of U so the integrator stays second-
//! order symplectic. For the buckyball SU(2) at β = 2.5, the gauge
//! coupling is `g² = (2·N) / β = 4 / 2.5 = 1.6` (Halcyon
//! `buckyball_integrator.py::_drift`).
//!
//! Group-erasure note (Bee's locked decision IV-B): SU(2)-only by
//! construction — the closed-form Rodrigues exp is SU(2)-specific
//! (SU(3) needs Padé or eigen-decomp). `drift_step` dispatches on
//! the U buffer's group tag and returns
//! `GaugeFieldError::UnsupportedGroup(_)` for non-SU(2). The Lie
//! row layout is shared by every E field (q0=0 imaginary
//! quaternion), so future U(1)/SU(3) drifts ship as
//! `drift_step_u1` / `drift_step_su3` siblings against U(1)EField /
//! SU3EField.
//!
//! Reference: Halcyon `davis-wilson-lattice/inertia_damping/
//! buckyball_integrator.py::matrix_exp_su2_q` + `_drift`.

use super::e_field::SU2EField;
use super::error::GaugeFieldError;
use super::group::Group;
use super::su2_gauge_field::SU2GaugeField;

/// Small-angle Taylor cutoff. Below this θ we use the 4th-order
/// expansion of `sin θ / θ` and `cos θ` to avoid the removable
/// singularity in the closed-form Rodrigues map.
///
/// 1e-8 is the canonical Halcyon value (mirrors `buckyball_integrator.
/// py::matrix_exp_su2_q`) — below this scale the Taylor truncation
/// error is bounded by `θ⁴ / 24 ≲ 4e-33`, well under f64 ULP at 1.0.
const TAYLOR_THETA_CUTOFF: f64 = 1e-8_f64;

/// Raw quaternion product (scalar-first, Hamilton convention).
///
/// Mirrored from `wilson_force::qmul` to keep both kernels' inner
/// loops on `[f64; 4]` rows without round-tripping through
/// `GroupElement::SU2 { … }`. The duplication is intentional — both
/// kernels are hot and the symmetry is engineering.
#[inline]
fn qmul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let (a0, a1, a2, a3) = (a[0], a[1], a[2], a[3]);
    let (b0, b1, b2, b3) = (b[0], b[1], b[2], b[3]);
    [
        a0 * b0 - a1 * b1 - a2 * b2 - a3 * b3,
        a0 * b1 + b0 * a1 + a2 * b3 - a3 * b2,
        a0 * b2 + b0 * a2 + a3 * b1 - a1 * b3,
        a0 * b3 + b0 * a3 + a1 * b2 - a2 * b1,
    ]
}

/// Closed-form Rodrigues exponential of an `su(2)` Lie-algebra
/// quaternion.
///
/// `omega` is expected to be of the form `(0, x, y, z)`. The q0 slot
/// of the input is NOT enforced to zero (the caller is responsible),
/// but the small-angle path uses `θ² = x² + y² + z²` so a nonzero
/// `omega[0]` is silently ignored by the math — consistent with the
/// q0=0 invariant the E-field write boundary already enforces.
///
/// Returns an SU(2) quaternion `(q0, q1, q2, q3)` on the unit sphere
/// S³ to f64 precision (within 2 ULP of unit norm).
///
/// Branches on `θ < 1e-8`:
///   - small-angle Taylor (4th-order): keeps `omega = 0` exactly
///     identity, no trig roundoff.
///   - large-angle Rodrigues: `cos θ` for q0, `sin θ / θ` factor on
///     the imaginary triple.
pub fn matrix_exp_su2_q(omega: [f64; 4]) -> [f64; 4] {
    let x = omega[1];
    let y = omega[2];
    let z = omega[3];
    let theta_sq = x * x + y * y + z * z;
    let theta = theta_sq.sqrt();
    if theta < TAYLOR_THETA_CUTOFF {
        // 4th-order Taylor: cos θ ≈ 1 - θ²/2, sin θ / θ ≈ 1 - θ²/6.
        let half = theta_sq * 0.5_f64;
        let sixth = theta_sq / 6.0_f64;
        let scale = 1.0_f64 - sixth;
        [1.0_f64 - half, scale * x, scale * y, scale * z]
    } else {
        let c = theta.cos();
        let s_over_theta = theta.sin() / theta;
        [c, s_over_theta * x, s_over_theta * y, s_over_theta * z]
    }
}

/// D step of the KDK leapfrog: `U[e] ← exp(g²·dt·E[e]) · U[e]` per
/// edge.
///
/// `u` is mutated in place. `e` is read-only — the drift step does
/// not touch E. `dt` is the full leapfrog step (NOT a half-step; the
/// half-step ½Δt convention is reserved for `apply_force_kick`'s
/// `dt_half`). `g2` is the gauge coupling `g² = (2·N) / β`; for
/// SU(2) at β = 2.5 the canonical value is `g² = 1.6`.
///
/// Returns `Err(GaugeFieldError::UnsupportedGroup(_))` for non-SU(2)
/// U buffers (the closed-form Rodrigues exp is SU(2)-only) and
/// `Err(GaugeFieldError::BufferShapeMismatch { … })` if the U and E
/// edge counts disagree so the leapfrog driver can surface a typed
/// error before any mutation lands.
///
/// SU(2)-only by construction. Future SU(3) drift ships as
/// `drift_step_su3` in a sibling module against an SU3EField; the
/// dispatch lives in the leapfrog driver (Part IV.6) which reaches
/// through `&dyn GaugeFieldHandle` and switches on `handle.group()`.
pub fn drift_step(
    u: &mut SU2GaugeField,
    e: &SU2EField,
    dt: f64,
    g2: f64,
) -> Result<(), GaugeFieldError> {
    match u.buffer.group {
        Group::SU2 => {}
        other => return Err(GaugeFieldError::UnsupportedGroup(other)),
    }
    let n_edges = u.buffer.n_edges;
    if e.buffer.n_edges != n_edges {
        return Err(GaugeFieldError::BufferShapeMismatch {
            expected: n_edges,
            got: e.buffer.n_edges,
        });
    }
    let coef = g2 * dt;
    for edge in 0..n_edges {
        let e_row = e.buffer.read_lie_row(edge);
        let omega = [
            0.0_f64,
            coef * e_row[1],
            coef * e_row[2],
            coef * e_row[3],
        ];
        let exp_q = matrix_exp_su2_q(omega);
        let base = 4 * edge;
        let u_row = [
            u.buffer.data[base],
            u.buffer.data[base + 1],
            u.buffer.data[base + 2],
            u.buffer.data[base + 3],
        ];
        // LEFT multiplication — Halcyon bug #3 fix.
        let mut u_new = qmul(exp_q, u_row);
        // Per-edge renormalization (Halcyon `_drift`
        // `U_new[e] = U_new[e] / qnorm(U_new[e])` — defends FP
        // roundoff so ||U[e]|| stays within 2 ULP of 1.0 over the
        // 1000-step drift orbit. Skip when the (numerically
        // impossible) zero-quaternion case crops up so the buffer
        // never sees a NaN.
        let n2 = u_new[0] * u_new[0]
            + u_new[1] * u_new[1]
            + u_new[2] * u_new[2]
            + u_new[3] * u_new[3];
        if n2 > 0.0_f64 {
            let n = n2.sqrt();
            u_new[0] /= n;
            u_new[1] /= n;
            u_new[2] /= n;
            u_new[3] /= n;
        }
        u.buffer.data[base] = u_new[0];
        u.buffer.data[base + 1] = u_new[1];
        u.buffer.data[base + 2] = u_new[2];
        u.buffer.data[base + 3] = u_new[3];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauge::e_field::{EFieldInit, SU2EField};
    use crate::gauge::registry as gauge_registry;
    use crate::gauge::su2_gauge_field::GaugeFieldInit;
    use crate::lattice::registry as lattice_registry;
    use crate::lattice::topology::truncated_icosahedron::buckyball;

    /// TDD-HAL-IV.5 unit: exp(0) is the SU(2) identity quaternion
    /// exactly. Companion to the integration test in
    /// `tests/gauge_lie_exp_unit.rs`; the in-lib version guards the
    /// `TAYLOR_THETA_CUTOFF` branch from regression.
    #[test]
    fn tdd_hal_iv_5_unit_exp_zero_is_identity() {
        let q = matrix_exp_su2_q([0.0, 0.0, 0.0, 0.0]);
        assert_eq!(q, [1.0, 0.0, 0.0, 0.0]);
    }

    /// TDD-HAL-IV.5 unit: small-angle Taylor path lands on a unit-
    /// norm quaternion within 2 ULP. Picks `θ = 1e-9 < cutoff`.
    #[test]
    fn tdd_hal_iv_5_unit_exp_small_angle_taylor_unit_norm() {
        let q = matrix_exp_su2_q([0.0, 1e-9, 0.0, 0.0]);
        let n2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
        assert!((n2 - 1.0).abs() <= 2.0 * f64::EPSILON);
    }

    /// TDD-HAL-IV.5 unit: shape mismatch surfaces as typed error
    /// before any buffer mutation.
    #[test]
    fn tdd_hal_iv_5_unit_drift_shape_mismatch() {
        let _serial = gauge_registry::test_serial_lock();
        gauge_registry::clear();
        gauge_registry::clear_e_registry();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let mut u = SU2GaugeField::new(
            "U_iv5_unit_mm".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        // Build an E field with a deliberately wrong shape by
        // round-tripping a smaller U → using FromField is the
        // wrong path here; instead build the E directly with a
        // smaller n_edges via the dense buffer constructor.
        let mut bad = SU2EField::new(
            "E_iv5_unit_mm".into(),
            &u,
            EFieldInit::Zero,
            None,
        )
        .unwrap();
        // Manually truncate the E buffer to simulate a shape mismatch.
        bad.buffer.n_edges = 5;
        bad.buffer.data.truncate(20);

        let err = drift_step(&mut u, &bad, 0.02, 1.6).unwrap_err();
        match err {
            GaugeFieldError::BufferShapeMismatch { expected, got } => {
                assert_eq!(expected, bb.n_edges());
                assert_eq!(got, 5);
            }
            other => panic!("expected BufferShapeMismatch, got {other:?}"),
        }
    }
}
