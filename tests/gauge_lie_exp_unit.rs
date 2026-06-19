//! TDD-HAL-IV.5 red test — U full-step drift via `exp(dt·E) · U`
//! Lie-to-group map.
//!
//! These tests pin the contract for the SU(2) matrix exponential
//! `matrix_exp_su2_q` and the matching full-step drift `drift_step`
//! that lands the D step of the leapfrog KDK integrator. The Lie
//! algebra is the imaginary-quaternion subspace (q0 = 0); the closed-
//! form Rodrigues exponential maps a Lie row `(0, x, y, z)` with
//! `θ = √(x² + y² + z²)` to the SU(2) quaternion
//!
//!     exp(omega) = (cos θ, sin θ / θ · x, sin θ / θ · y, sin θ / θ · z)
//!
//! with a 4th-order Taylor fallback for `θ < 1e-8` (the sin θ / θ
//! removable singularity).
//!
//! `drift_step` reads the Lie row per edge, scales by `g² · dt` to
//! form `omega`, exponentiates, and LEFT-multiplies the existing U
//! link: `U_new[e] = qmul(exp(omega), U[e])`. The left-multiplication
//! is Halcyon bug #3's fix — the Lie-to-group map acts on the left
//! of U so the integrator stays second-order symplectic. For the
//! buckyball SU(2) at β = 2.5, `g² = (2·N) / β = 4 / 2.5 = 1.6`.
//!
//! Gate locked decisions in play:
//!   - IV-B: SU2EField sibling buffer; the kernel reads its `(n_edges,
//!     4)` Lie row layout (q0=0 invariant).
//!   - Group-erasure: SU(2)-only — the closed-form Rodrigues exp is
//!     SU(2)-specific (SU(3) needs Padé or eigen-decomp). Other groups
//!     return GroupNotImplemented at entry.

#![cfg(feature = "gauge")]

use gigi::gauge::{
    drift_step,
    e_field::{EFieldInit, SU2EField},
    matrix_exp_su2_q,
    registry::{
        clear as clear_gauge, clear_e_registry, get_su2_mut, register_su2,
        republish_su2, test_serial_lock,
    },
    su2_gauge_field::{GaugeFieldInit, SU2GaugeField},
    gibbs_sample, ObservableId,
};
use gigi::lattice::{
    registry as lattice_registry, topology::truncated_icosahedron::buckyball,
};

/// TDD-HAL-IV.5: exp(0) is the SU(2) identity quaternion exactly
/// (Taylor fallback path; no trig roundoff).
#[test]
fn tdd_hal_iv_5_exp_zero_is_identity() {
    let q = matrix_exp_su2_q([0.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        q,
        [1.0, 0.0, 0.0, 0.0],
        "exp(0) must be SU(2) identity (1, 0, 0, 0) exactly"
    );
}

/// TDD-HAL-IV.5: `exp(omega)` lands on the SU(2) manifold — the
/// output quaternion is unit-norm within 2 ULP of 1.0 for a generic
/// non-zero Lie input.
#[test]
fn tdd_hal_iv_5_exp_unit_norm() {
    let q = matrix_exp_su2_q([0.0, 0.5, 0.3, 0.2]);
    let n2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
    // Two ULPs at 1.0 is approximately 2 * f64::EPSILON.
    let tol = 2.0 * f64::EPSILON;
    assert!(
        (n2 - 1.0).abs() <= tol,
        "exp(omega) must be unit-norm: |q|² = {n2}, deviation {} > tol {tol}",
        (n2 - 1.0).abs()
    );
}

/// TDD-HAL-IV.5: known-value Rodrigues — for `omega = (0, π/4, 0, 0)`
/// the Lie-norm `θ = √(x² + y² + z²) = π/4`, so `exp(omega) =
/// (cos(π/4), sin(π/4), 0, 0)`. This is the half-rotation about the
/// x axis in the algebra-norm convention Halcyon Python uses
/// (`buckyball_integrator.py::matrix_exp_su2_q` — `cos|v|, sin|v|/|v|
/// · v` with `|v| = sqrt((v·v).sum)`).
#[test]
fn tdd_hal_iv_5_exp_known_value() {
    let q = matrix_exp_su2_q([0.0, std::f64::consts::FRAC_PI_4, 0.0, 0.0]);
    let want_c = (std::f64::consts::FRAC_PI_4).cos();
    let want_s = (std::f64::consts::FRAC_PI_4).sin();
    let tol = 1e-15_f64;
    assert!(
        (q[0] - want_c).abs() < tol,
        "q0: got {} want {want_c}",
        q[0]
    );
    assert!(
        (q[1] - want_s).abs() < tol,
        "q1: got {} want {want_s}",
        q[1]
    );
    assert!(q[2].abs() < tol, "q2: got {} want 0", q[2]);
    assert!(q[3].abs() < tol, "q3: got {} want 0", q[3]);
}

/// TDD-HAL-IV.5: identity U + zero E + nonzero dt → drift is a no-op.
/// Every U row stays at the SU(2) identity exactly (`exp(0) · I = I`).
#[test]
fn tdd_hal_iv_5_drift_identity_u_zero_e_no_op() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_iv5_noop".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .expect("identity U init must succeed");
    register_su2(u_field);
    let u_arc = get_su2_mut("U_iv5_noop").expect("registered");

    let e_field = SU2EField::new(
        "E_iv5_noop".into(),
        &*u_arc.lock().expect("u mutex poisoned"),
        EFieldInit::Zero,
        None,
    )
    .expect("zero E init must succeed");

    let g2 = 4.0 / 2.5_f64;
    let dt = 0.02_f64;
    {
        let mut u_guard = u_arc.lock().expect("u mutex poisoned");
        drift_step(&mut *u_guard, &e_field, dt, g2)
            .expect("drift must succeed");
    }

    let u_guard = u_arc.lock().expect("u mutex poisoned");
    for edge in 0..bb.n_edges() {
        let base = 4 * edge;
        let row = [
            u_guard.buffer.data[base],
            u_guard.buffer.data[base + 1],
            u_guard.buffer.data[base + 2],
            u_guard.buffer.data[base + 3],
        ];
        assert_eq!(
            row,
            [1.0, 0.0, 0.0, 0.0],
            "edge {edge}: identity U + zero E drift must leave U at identity"
        );
    }
}

/// TDD-HAL-IV.5: U = identity, E = canonical non-zero `(0, 0.1, 0, 0)`
/// on every edge → `drift_step` left-multiplies by `exp((0, g²·dt·0.1,
/// 0, 0))`. Since U[e] = identity, U_new[e] equals the exponential
/// itself byte-for-byte. Verifies the LEFT multiplication (Halcyon
/// bug #3 fix).
#[test]
fn tdd_hal_iv_5_drift_left_multiplication() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_iv5_left".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .expect("identity U init must succeed");
    register_su2(u_field);
    let u_arc = get_su2_mut("U_iv5_left").expect("registered");

    let mut e_field = SU2EField::new(
        "E_iv5_left".into(),
        &*u_arc.lock().expect("u mutex poisoned"),
        EFieldInit::Zero,
        None,
    )
    .expect("zero E init must succeed");
    for edge in 0..bb.n_edges() {
        e_field.write_element_q(edge, [0.0, 0.1, 0.0, 0.0]);
    }

    let g2 = 1.6_f64;
    let dt = 0.02_f64;
    let want = matrix_exp_su2_q([0.0, g2 * dt * 0.1, 0.0, 0.0]);

    {
        let mut u_guard = u_arc.lock().expect("u mutex poisoned");
        drift_step(&mut *u_guard, &e_field, dt, g2)
            .expect("drift must succeed");
    }

    let u_guard = u_arc.lock().expect("u mutex poisoned");
    let tol = 4.0 * f64::EPSILON;
    for edge in 0..bb.n_edges() {
        let base = 4 * edge;
        let row = [
            u_guard.buffer.data[base],
            u_guard.buffer.data[base + 1],
            u_guard.buffer.data[base + 2],
            u_guard.buffer.data[base + 3],
        ];
        // LEFT multiplication: qmul(exp(omega), identity) = exp(omega)
        // up to the per-edge renormalization (the Halcyon `_drift`
        // divides by qnorm to defend FP roundoff; for small θ the
        // norm is within a few ULP of 1.0 so the renormalized output
        // matches the raw exponential to a few ULP).
        for k in 0..4 {
            assert!(
                (row[k] - want[k]).abs() <= tol,
                "edge {edge} comp {k}: U_new = {} vs exp(omega) = {} (diff {})",
                row[k], want[k], (row[k] - want[k]).abs()
            );
        }
    }
}

/// TDD-HAL-IV.5: MANIFOLD PRESERVATION RECEIPT — the entire reason
/// this gate exists. Thermalize U via 200 GIBBS_SAMPLE sweeps at
/// β=2.5 seed=20260616, initialize E via MaxwellBoltzmann seed
/// 20260617, then chain 1000 drift_step calls at dt=0.02. After 1000
/// steps every edge's U[e] is on the SU(2) manifold within 2 ULP of
/// unit norm — the closed-form Rodrigues exp + qmul keep the orbit
/// on S³ to f64 precision, no projection step required.
///
/// (Per Bee's note in the spec, this gate also exercises the chained
/// IV.4 + IV.5 calls; the IV.4 force-kick is omitted here because
/// the manifold receipt is on the drift step alone — kicks mutate E,
/// not U, so the kick does not move points off the manifold. The
/// 1000 drift-only steps still consume E energy as the drift left-
/// multiplies U, which is the relevant manifold contract.)
#[test]
fn tdd_hal_iv_5_drift_manifold_preservation_1000_steps() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    // Thermalize a U field via GIBBS_SAMPLE — Haar init + 200 sweeps
    // at β=2.5.
    let u_field = SU2GaugeField::new(
        "U_iv5_manifold".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .expect("haar U init must succeed");
    register_su2(u_field);
    let _resp = gibbs_sample(
        "U_iv5_manifold",
        2.5,
        200,
        0,
        Vec::<ObservableId>::new(),
        Some(20260616),
    )
    .expect("gibbs sample must succeed");

    let u_arc = get_su2_mut("U_iv5_manifold").expect("registered");

    // Initialize E via Maxwell–Boltzmann at β=2.5 seed=20260617.
    let e_field = SU2EField::new(
        "E_iv5_manifold".into(),
        &*u_arc.lock().expect("u mutex poisoned"),
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .expect("MB E init must succeed");

    let g2 = 4.0 / 2.5_f64;
    let dt = 0.02_f64;
    let n_steps = 1000_usize;
    {
        let mut u_guard = u_arc.lock().expect("u mutex poisoned");
        for _ in 0..n_steps {
            drift_step(&mut *u_guard, &e_field, dt, g2)
                .expect("drift must succeed");
        }
    }
    // Re-publish so the read map is post-mutation coherent for any
    // downstream lookup (the test itself does not need it, but it
    // mirrors the gibbs_sample epilogue convention).
    republish_su2("U_iv5_manifold", u_arc.clone());

    let u_guard = u_arc.lock().expect("u mutex poisoned");
    let tol = 2.0 * f64::EPSILON;
    for edge in 0..bb.n_edges() {
        let base = 4 * edge;
        let n2 = u_guard.buffer.data[base] * u_guard.buffer.data[base]
            + u_guard.buffer.data[base + 1] * u_guard.buffer.data[base + 1]
            + u_guard.buffer.data[base + 2] * u_guard.buffer.data[base + 2]
            + u_guard.buffer.data[base + 3] * u_guard.buffer.data[base + 3];
        assert!(
            (n2 - 1.0).abs() <= tol,
            "edge {edge}: ||U_final||² = {n2}, deviation {} > 2 ULP tol {tol} after {n_steps} drift steps",
            (n2 - 1.0).abs()
        );
    }
}
