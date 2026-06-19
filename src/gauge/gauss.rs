//! Covariant Gauss vertex divergence operator.
//!
//! Closes TDD-HAL-IV.2. The Gauss vertex divergence is the per-vertex
//! Lie-algebra residual the symplectic flow projects against to
//! enforce Gauss's law on the lattice. For vertex `v` with incident
//! edges `{e_i with orientation o_i}`:
//!
//! ```text
//!     G_v = Σ_{i: o_i=Forward} E[e_i].vec
//!         - Σ_{i: o_i=Reverse} Ad(U[e_i])[E[e_i].vec]
//! ```
//!
//! The Forward arm reads the raw Lie-algebra (`q1, q2, q3`) row; the
//! Reverse arm parallel-transports it back to `v` by sandwiching with
//! `U_e` (the SU(2) adjoint action on su(2)):
//!
//! ```text
//!     Ad(U) v = U · (0, v_1, v_2, v_3) · U^†
//! ```
//!
//! where the inner expression is quaternion multiplication and `U^†`
//! is the SU(2) conjugate `(q0, -q1, -q2, -q3)`. The scalar component
//! of the result is zero (Ad preserves the su(2) subspace), so we
//! discard it and return only `(v_1', v_2', v_3')`.
//!
//! The FLAT version drops the Ad action: it's the signed sum of E
//! components (the abelian divergence), used as a baseline and as the
//! zero-cost Identity-U cross-check.
//!
//! Reference: `davis-wilson-lattice/inertia_damping/buckyball_action.py`
//! `compute_gauss_residual_covariant`.
//!
//! Group-erasure note (Bee's locked decision IV-B): `compute_gauss_
//! residual_covariant` dispatches on `handle.group()`. The SU(2) arm
//! runs the Ad-sandwich algorithm; non-SU(2) groups return
//! `UnsupportedGroup`. `VertexEdgeIncidence` itself is group-agnostic
//! — it's a lattice-only structure. Future U(1) E-fields would use
//! abelian divergence (no Ad action needed) and would ship a sibling
//! `compute_gauss_residual_covariant_u1`; the FLAT compute already
//! works for them.
//!
//! Caching: `build_vertex_edge_incidence` (on `Lattice`) is `O(E)`
//! one-shot (`E = 90` on the buckyball). The caller is responsible
//! for hoisting the result out of any sweep loop. We deliberately do
//! NOT cache it on `Lattice` so `to_gql` round-trip stays byte-
//! identical (the incidence is derived, not declared).

use super::e_field::EFieldHandle;
use super::error::GaugeFieldError;
use super::group::Group;
use super::registry::GaugeFieldHandle;
use crate::lattice::{EdgeId, EdgeOrientation, Lattice};

/// Per-vertex → list of `(edge_id, orientation)` entries.
///
/// `inc[v]` enumerates every edge that touches `v`; the
/// `orientation` records whether `v` is the head (`Forward` — the
/// `b` end of `edges[i] = (a, b)`) or the tail (`Reverse` — the
/// `a` end). On a degree-k graph every vertex has exactly `k`
/// entries; on the buckyball every vertex has exactly 3.
pub type VertexEdgeIncidence = Vec<Vec<(EdgeId, EdgeOrientation)>>;

/// Build the per-vertex incidence table for `lat`. Thin re-export
/// over `Lattice::build_vertex_edge_incidence` so callers in the
/// gauge crate can stay in the `gauge::` namespace and get the
/// `VertexEdgeIncidence` type alias without naming `crate::lattice`
/// directly. Iterates edges in ascending `edge_id` order — load-
/// bearing ordering for A2 row 1 bit-identity on the IV.10
/// symplectic flow.
pub fn build_vertex_edge_incidence(lat: &Lattice) -> VertexEdgeIncidence {
    lat.build_vertex_edge_incidence()
}

/// Compute the covariant Gauss vertex divergence `G_cov[v]` under
/// the connection behind `u` and the E field behind `e`.
///
/// For each vertex `v` with incident entries `inc[v]`:
///
/// 1. Read `E[eid].vec = (q1, q2, q3)` for every incident edge.
/// 2. If the entry's orientation is `Forward` (v is the head),
///    contribute `+ E[eid].vec` to `G_cov[v]`.
/// 3. If the entry's orientation is `Reverse` (v is the tail),
///    contribute `- Ad(U[eid])[E[eid].vec]` — parallel-transport
///    the Lie-algebra row back from the head end to the tail end
///    by sandwiching with the SU(2) link element.
///
/// Returns a `(n_vertices, 3)` `Vec<[f64; 3]>`. Group dispatch:
/// `handle.group()` must be `Group::SU2`; otherwise returns
/// `Err(UnsupportedGroup(_))`. Future SU(3) / U(1) E-fields will
/// ship sibling computes with their own Ad-action formulas.
pub fn compute_gauss_residual_covariant(
    u: &dyn GaugeFieldHandle,
    e: &dyn EFieldHandle,
    lat: &Lattice,
    inc: &VertexEdgeIncidence,
) -> Result<Vec<[f64; 3]>, GaugeFieldError> {
    if u.group() != Group::SU2 {
        return Err(GaugeFieldError::UnsupportedGroup(u.group()));
    }
    if e.group() != Group::SU2 {
        return Err(GaugeFieldError::UnsupportedGroup(e.group()));
    }
    debug_assert_eq!(inc.len(), lat.n_vertices);

    let u_buf = u.as_dense_buffer();
    let e_buf = e.as_dense_buffer();
    let mut out: Vec<[f64; 3]> = vec![[0.0_f64; 3]; lat.n_vertices];

    for vid in 0..lat.n_vertices {
        let mut g = [0.0_f64; 3];
        for &(eid, orient) in &inc[vid] {
            // E Lie row is (0, v1, v2, v3); discard q0 (== 0).
            let er = e_buf.read_lie_row(eid);
            let ev = [er[1], er[2], er[3]];
            match orient {
                EdgeOrientation::Forward => {
                    // v is the HEAD of edges[eid] = (u, v). Forward
                    // contribution is + raw E row.
                    g[0] += ev[0];
                    g[1] += ev[1];
                    g[2] += ev[2];
                }
                EdgeOrientation::Reverse => {
                    // v is the TAIL. Subtract the Ad-rotated row:
                    // Ad(U_e) v = U_e · (0, v1, v2, v3) · U_e^†.
                    let base = u_buf.repr_dim * eid;
                    let u0 = u_buf.data[base];
                    let u1 = u_buf.data[base + 1];
                    let u2 = u_buf.data[base + 2];
                    let u3 = u_buf.data[base + 3];
                    let r = ad_action_su2([u0, u1, u2, u3], ev);
                    g[0] -= r[0];
                    g[1] -= r[1];
                    g[2] -= r[2];
                }
            }
        }
        out[vid] = g;
    }
    Ok(out)
}

/// Compute the FLAT Gauss vertex divergence `G_flat[v]` — the
/// abelian signed sum of E components, drops the Ad action.
///
/// `G_flat[v] = Σ_{Forward} E[eid].vec − Σ_{Reverse} E[eid].vec`.
///
/// Two uses: (1) baseline for the WF#2 ratio (the covariant residual
/// is much smaller than the flat one at thermalized U); (2) zero-cost
/// cross-check that `compute_gauss_residual_covariant` reduces to it
/// when `U = I` (because `Ad(I) = id`).
pub fn compute_gauss_residual_flat(
    e: &dyn EFieldHandle,
    lat: &Lattice,
    inc: &VertexEdgeIncidence,
) -> Result<Vec<[f64; 3]>, GaugeFieldError> {
    if e.group() != Group::SU2 {
        return Err(GaugeFieldError::UnsupportedGroup(e.group()));
    }
    debug_assert_eq!(inc.len(), lat.n_vertices);

    let e_buf = e.as_dense_buffer();
    let mut out: Vec<[f64; 3]> = vec![[0.0_f64; 3]; lat.n_vertices];

    for vid in 0..lat.n_vertices {
        let mut g = [0.0_f64; 3];
        for &(eid, orient) in &inc[vid] {
            let er = e_buf.read_lie_row(eid);
            let ev = [er[1], er[2], er[3]];
            match orient {
                EdgeOrientation::Forward => {
                    g[0] += ev[0];
                    g[1] += ev[1];
                    g[2] += ev[2];
                }
                EdgeOrientation::Reverse => {
                    g[0] -= ev[0];
                    g[1] -= ev[1];
                    g[2] -= ev[2];
                }
            }
        }
        out[vid] = g;
    }
    Ok(out)
}

/// Scalar reduction: maximum over vertices of the L∞ norm of the
/// per-vertex Lie-algebra residual. Used as the GaussResidualMax
/// observable + as the symplectic-flow Gauss-projection convergence
/// readout. Returns `0.0` on an empty input (max-on-non-negative
/// identity).
pub fn max_inf_norm(residual: &[[f64; 3]]) -> f64 {
    let mut m = 0.0_f64;
    for row in residual {
        for &c in row.iter() {
            let a = c.abs();
            if a > m {
                m = a;
            }
        }
    }
    m
}

/// SU(2) adjoint action on the Lie algebra: `Ad(U) v = U · (0, v) ·
/// U^†`. Returns the imaginary-component triple `(w_1, w_2, w_3)`;
/// the scalar component is zero by construction (Ad preserves the
/// su(2) subspace).
///
/// Quaternion convention: scalar-first `(q0, q1, q2, q3)` with the
/// Hamilton product
/// ```text
/// (a · b).0 = a0 b0 − a · b
/// (a · b).vec = a0 b_vec + b0 a_vec − a × b
/// ```
/// (matches `GroupElement::compose` and the rest of the gauge crate).
#[inline]
fn ad_action_su2(u: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let u0 = u[0];
    let u1 = u[1];
    let u2 = u[2];
    let u3 = u[3];
    // Step 1: u · (0, v1, v2, v3)
    //   q0 = u0*0 − u·v = −(u1 v1 + u2 v2 + u3 v3)
    //   q_vec = u0*v + 0*u − u × v
    let s = u1 * v[0] + u2 * v[1] + u3 * v[2];
    let cx = u2 * v[2] - u3 * v[1];
    let cy = u3 * v[0] - u1 * v[2];
    let cz = u1 * v[1] - u2 * v[0];
    let a0 = -s;
    let a1 = u0 * v[0] - cx;
    let a2 = u0 * v[1] - cy;
    let a3 = u0 * v[2] - cz;
    // Step 2: (a0, a1, a2, a3) · u^† where u^† = (u0, -u1, -u2, -u3)
    //   c_vec = a0 * (-u_vec) + u0 * a_vec − a × (-u_vec)
    //         = -a0 u_vec + u0 a_vec + a × u_vec
    let bx = a2 * u3 - a3 * u2;
    let by = a3 * u1 - a1 * u3;
    let bz = a1 * u2 - a2 * u1;
    let c1 = -a0 * u1 + u0 * a1 + bx;
    let c2 = -a0 * u2 + u0 * a2 + by;
    let c3 = -a0 * u3 + u0 * a3 + bz;
    [c1, c2, c3]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ad_action_su2` at U = identity reduces to the identity map
    /// on the Lie algebra: `Ad(I) v = v`.
    #[test]
    fn ad_identity_is_identity() {
        let v = [0.3, -0.7, 1.2];
        let r = ad_action_su2([1.0, 0.0, 0.0, 0.0], v);
        assert!((r[0] - v[0]).abs() < 1e-15);
        assert!((r[1] - v[1]).abs() < 1e-15);
        assert!((r[2] - v[2]).abs() < 1e-15);
    }

    /// `ad_action_su2` for a unit quaternion preserves the Lie-algebra
    /// L2 norm (Ad is an SO(3) rotation).
    #[test]
    fn ad_preserves_norm() {
        // Rotation by θ = π/3 about z-axis: u = (cos π/6, 0, 0, sin π/6).
        let c = (std::f64::consts::PI / 6.0).cos();
        let s = (std::f64::consts::PI / 6.0).sin();
        let u = [c, 0.0, 0.0, s];
        let v = [0.3, -0.7, 1.2];
        let r = ad_action_su2(u, v);
        let nv2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        let nr2 = r[0] * r[0] + r[1] * r[1] + r[2] * r[2];
        assert!(
            (nv2 - nr2).abs() < 1e-12,
            "|v|^2 = {nv2}, |Ad(u) v|^2 = {nr2}"
        );
    }

    /// `max_inf_norm` smoke test (the dedicated integration-test
    /// path lives in `tests/gauge_gauss_unit.rs`).
    #[test]
    fn max_inf_norm_smoke() {
        let r = [[0.0, 0.0, 0.0], [3.5, -1.0, 0.25], [0.0, 2.7, 0.0]];
        assert_eq!(max_inf_norm(&r), 3.5);
        let empty: [[f64; 3]; 0] = [];
        assert_eq!(max_inf_norm(&empty), 0.0);
    }
}
