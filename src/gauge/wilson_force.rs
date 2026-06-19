//! Per-edge Wilson force + Lie-algebra momentum kick.
//!
//! Closes TDD-HAL-IV.4. The Wilson force is the gradient of the
//! Wilson action with respect to the per-edge link variable, projected
//! onto the Lie algebra. For SU(2):
//!
//! ```text
//!     F[e] = coeff * project_lie(qmul(U[e], Σ_e))
//! ```
//!
//! where `Σ_e = staple_sum_at_edge(U, lat, inc, e)` is the per-edge
//! staple sum (III.3 primitive, REUSED unchanged) and `coeff =
//! -β / (2·N²) = -β/8` for SU(2). The sign + magnitude is Halcyon's
//! bug #3 fix — the second-order symplectic accuracy of the
//! leapfrog KDK integrator (Part IV.6) hinges on this coefficient
//! being exactly `-β / (2·N²)`.
//!
//! `apply_force_kick` is the K step of KDK: `E[e] += dt_half · F[e]`
//! per edge, with `q0 = 0` restored at every row mutation (the Lie-
//! algebra invariant is enforced at the buffer write boundary by
//! `DenseLinkBuffer::write_lie_row`, so this kernel relies on that
//! contract rather than zeroing component-wise itself).
//!
//! Group-erasure note (Bee's locked decision IV-B + future SU(3)):
//! SU(2)-specific by construction — the coefficient hardcodes `N=2`
//! and `project_lie` here is the SU(2) Lie-algebra projector
//! (drop the scalar quaternion component). A future SU(3) heatbath
//! integrator ships sibling `wilson_force_su3.rs` with `-β/18` and
//! a parallel SU(3) staple. The leapfrog driver (Part IV.6) reaches
//! through `&dyn GaugeFieldHandle` and dispatches on
//! `handle.group()` at the call site; this kernel speaks raw
//! `[f64; 4]` rows for both U and E to keep the inner loop hot.
//!
//! Reference: `davis-wilson-lattice/inertia_damping/
//! buckyball_integrator.py::_force` + the matching half-step
//! `E += dt/2 · F` kick inside the leapfrog driver.

use super::e_field::SU2EField;
use super::edge_connection::EdgeConnection;
use super::error::GaugeFieldError;
use super::group::Group;
use super::group_element::GroupElement;
use super::registry::GaugeFieldHandle;
use super::staple::{staple_sum_at_edge, EdgeFaceIncidence};
use crate::lattice::{EdgeOrientation, Lattice};

/// Raw quaternion product (scalar-first, Hamilton convention).
///
/// Inlined alongside the staple consumer so the inner loop stays
/// on `[f64; 4]` rows instead of round-tripping through
/// `GroupElement::SU2 { … }`. Same algebra as `kennedy_pendleton::qmul`
/// (mirrored, not exposed — those are private to two-distinct hot
/// kernels and the symmetry is intentional).
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

/// Project a quaternion onto su(2) (the Lie algebra of SU(2)) by
/// zeroing the scalar component. The imaginary (q1, q2, q3) part is
/// the Lie-algebra coordinate vector in the canonical (σ_x, σ_y, σ_z)
/// basis (no factor of 2 — the factor lives in the integrator's β
/// coefficient).
#[inline]
fn project_lie(q: [f64; 4]) -> [f64; 4] {
    [0.0, q[1], q[2], q[3]]
}

/// Compute the per-edge Wilson force `F[e] = coeff · project_lie(
/// qmul(U[e], Σ_e))` for every edge in `lat`.
///
/// `handle` is the SU(2) gauge field; the kernel reaches through the
/// `EdgeConnection` surface to read each link in Forward orientation
/// (the canonical `U_e`). `inc` is the cached edge-face incidence
/// from III.3; the caller hoists it out of any leapfrog sweep loop
/// (the kernel does NOT rebuild it).
///
/// Returns one `[0, F_x, F_y, F_z]` Lie row per edge, in
/// `0..lat.n_edges()` order. The q0 slot is zeroed by `project_lie`
/// before the coefficient multiplication; the result is therefore
/// `(0, coeff·Σ_imag_x, coeff·Σ_imag_y, coeff·Σ_imag_z)` for the
/// quaternion `qmul(U[e], Σ_e)` (whose imaginary part is what the
/// canonical Halcyon `_force` consumes).
///
/// SU(2)-only at launch: non-SU(2) groups return
/// `Err(GaugeFieldError::UnsupportedGroup(_))` so the parser / HTTP
/// layer can surface a typed error. Future SU(3) wilson force ships
/// as `wilson_force_per_edge_su3` in a sibling module.
pub fn wilson_force_per_edge(
    handle: &dyn GaugeFieldHandle,
    lat: &Lattice,
    inc: &EdgeFaceIncidence,
    beta: f64,
) -> Result<Vec<[f64; 4]>, GaugeFieldError> {
    match handle.group() {
        Group::SU2 => {}
        other => return Err(GaugeFieldError::UnsupportedGroup(other)),
    }
    // SU(2) Wilson-action force coefficient: -β / (2 · N²) with N=2
    // → -β / 8. The leading minus sign is the gradient direction:
    // S_W = -(β/N) Σ_f Re Tr U_f → ∂S/∂U[e] proportional to -U[e]·Σ_e,
    // then projected to the Lie algebra. Halcyon's bug #3 fix lives
    // here — the coefficient is the load-bearing constant for the
    // leapfrog's second-order symplectic accuracy.
    let coeff = -beta / 8.0_f64;
    let n_edges = lat.n_edges();
    let mut out: Vec<[f64; 4]> = Vec::with_capacity(n_edges);
    let conn: &dyn EdgeConnection = handle;
    for eid in 0..n_edges {
        // U[e] in canonical Forward orientation (the kernel never
        // needs the inverse — Σ_e already carries the orientation
        // arithmetic per III.3).
        let u_e_q = match conn.edge_element(eid, EdgeOrientation::Forward) {
            GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
            _ => unreachable!(
                "wilson_force_per_edge: handle.group() == SU2 but edge_element returned non-SU2"
            ),
        };
        let sigma = match staple_sum_at_edge(conn, lat, inc, eid) {
            GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
            _ => unreachable!(
                "wilson_force_per_edge: staple_sum_at_edge returned non-SU2"
            ),
        };
        let prod = qmul(u_e_q, sigma);
        let lie = project_lie(prod);
        out.push([
            0.0,
            coeff * lie[1],
            coeff * lie[2],
            coeff * lie[3],
        ]);
    }
    Ok(out)
}

/// K step of the KDK leapfrog: `E[e] += dt_half · F[e]` per edge.
///
/// `dt_half` is the integrator's half-step Δt/2 (the full leapfrog
/// step is K-D-K, so each K kick lands a Δt/2 contribution). `force`
/// is the result of `wilson_force_per_edge` (one `[0, F_x, F_y, F_z]`
/// row per edge); the kernel reads `force[edge][1..4]`, adds it
/// scaled by `dt_half` to `e.buffer.row[edge][1..4]`, and writes
/// the row back through `e.write_element_q` so the q0=0 invariant
/// is restored at the buffer boundary (defends against FP roundoff).
///
/// Returns `Err(GaugeFieldError::BufferShapeMismatch { … })` if
/// `force.len() != e.buffer.n_edges` so the leapfrog driver can
/// surface a typed error before mutating state.
pub fn apply_force_kick(
    e: &mut SU2EField,
    force: &[[f64; 4]],
    dt_half: f64,
) -> Result<(), GaugeFieldError> {
    let n_edges = e.buffer.n_edges;
    if force.len() != n_edges {
        return Err(GaugeFieldError::BufferShapeMismatch {
            expected: n_edges,
            got: force.len(),
        });
    }
    for edge in 0..n_edges {
        let prev = e.read_element_q(edge);
        let f = force[edge];
        // The q0 slot is zeroed by `write_element_q` regardless of
        // input; compute the imaginary part directly and let the
        // write boundary restore the invariant.
        let next = [
            0.0,
            prev[1] + dt_half * f[1],
            prev[2] + dt_half * f[2],
            prev[3] + dt_half * f[3],
        ];
        e.write_element_q(edge, next);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauge::e_field::{EFieldInit, SU2EField};
    use crate::gauge::registry as gauge_registry;
    use crate::gauge::staple::build_edge_face_incidence;
    use crate::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
    use crate::lattice::registry as lattice_registry;
    use crate::lattice::topology::truncated_icosahedron::buckyball;
    use std::sync::Arc;

    /// TDD-HAL-IV.4 unit: identity U → force is zero on every edge.
    /// Companion to the integration test in
    /// `tests/gauge_wilson_force_unit.rs`; the in-lib version guards
    /// the private `qmul` + `project_lie` helpers from regression
    /// without round-tripping through the registry.
    #[test]
    fn tdd_hal_iv_4_unit_identity_force_is_zero() {
        let _serial = gauge_registry::test_serial_lock();
        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_iv4_unit_id".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        gauge_registry::register(Arc::new(field));
        let handle = gauge_registry::get("U_iv4_unit_id").expect("registered");
        let inc = build_edge_face_incidence(&bb);

        let f =
            wilson_force_per_edge(handle.as_ref(), &bb, &inc, 2.5).unwrap();
        assert_eq!(f.len(), bb.n_edges());
        for (eid, row) in f.iter().enumerate() {
            assert_eq!(
                row,
                &[0.0, 0.0, 0.0, 0.0],
                "edge {eid}: identity force not zero"
            );
        }
    }

    /// TDD-HAL-IV.4 unit: `apply_force_kick` linearity and q0=0
    /// invariant — kick on zero E with arbitrary force yields
    /// `dt_half · F` exactly, q0 stays 0.0.
    #[test]
    fn tdd_hal_iv_4_unit_kick_writes_dt_times_force() {
        let _serial = gauge_registry::test_serial_lock();
        gauge_registry::clear();
        gauge_registry::clear_e_registry();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let u = SU2GaugeField::new(
            "U_iv4_unit_kick".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        let mut e =
            SU2EField::new("E_iv4_unit".into(), &u, EFieldInit::Zero, None)
                .unwrap();

        // Synthetic force: row[edge] = [0, edge as f64, 2*edge, 3*edge].
        let n = bb.n_edges();
        let force: Vec<[f64; 4]> = (0..n)
            .map(|e| [0.0, e as f64, 2.0 * e as f64, 3.0 * e as f64])
            .collect();
        apply_force_kick(&mut e, &force, 0.5).unwrap();

        for edge in 0..n {
            let row = e.read_element_q(edge);
            assert_eq!(row[0], 0.0, "edge {edge}: q0 not zeroed");
            assert_eq!(row[1], 0.5 * edge as f64);
            assert_eq!(row[2], 0.5 * 2.0 * edge as f64);
            assert_eq!(row[3], 0.5 * 3.0 * edge as f64);
        }
    }

    /// TDD-HAL-IV.4 unit: shape mismatch surfaces as typed error
    /// before any buffer mutation.
    #[test]
    fn tdd_hal_iv_4_unit_kick_shape_mismatch() {
        let _serial = gauge_registry::test_serial_lock();
        gauge_registry::clear();
        gauge_registry::clear_e_registry();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let u = SU2GaugeField::new(
            "U_iv4_unit_mismatch".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        let mut e =
            SU2EField::new("E_iv4_mm".into(), &u, EFieldInit::Zero, None)
                .unwrap();
        let force_wrong = vec![[0.0, 1.0, 2.0, 3.0]; 5];
        let err = apply_force_kick(&mut e, &force_wrong, 0.1).unwrap_err();
        match err {
            GaugeFieldError::BufferShapeMismatch { expected, got } => {
                assert_eq!(expected, bb.n_edges());
                assert_eq!(got, 5);
            }
            other => panic!("expected BufferShapeMismatch, got {other:?}"),
        }
        // Confirm the buffer is unchanged after the error path.
        for edge in 0..bb.n_edges() {
            assert_eq!(e.read_element_q(edge), [0.0, 0.0, 0.0, 0.0]);
        }
    }
}
