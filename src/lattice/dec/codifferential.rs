//! `delta_1` — discrete codifferential `Form1 → Form0`.
//!
//! Adjoint of [`crate::lattice::dec::d_0`] under the L2 inner product
//! weighted by `dual_face_areas` (vertices) and the Hodge ratio
//! `l_e* / l_e` (edges). On a closed 2-manifold:
//!
//! ```text
//!     (delta_1 u)(v) = (1 / A_v*) *
//!         sum over edges e incident to v of [ sigma(e, v) * (l_e* / l_e) * u(e) ]
//! ```
//!
//! where:
//! - `sigma(e, v) = +1` if `v == edges[e].1` (head, matching the
//!   `d_0[e, v] = +1 ⇔ v is head` sign convention).
//! - `sigma(e, v) = -1` if `v == edges[e].0` (tail).
//! - `l_e* = (A_{v-}* + A_{v+}*) / (2 * l_e)` is the **barycentric**
//!   dual edge length, computed on the fly inside this module.
//!
//! ## Why barycentric (and the Phase 3 upgrade path)
//!
//! Phase 1's [`crate::lattice::LatticeWithMetric`] ships `edge_lengths`
//! (primal `l_e`) and `dual_face_areas` (`A_v*`) but does NOT ship
//! `dual_edge_lengths`. Rather than (a) requiring a Phase 3 extension
//! to `metric.rs` or (b) demanding every topology constructor pre-
//! compute `l_e*` and stuff it into a fifth accessor, Phase 2 computes
//! the dual edge length on the fly here using the standard barycentric
//! (median-dual) formula from Hirani 2003 §5.5 and Desbrun-Kanso-Tong
//! 2005 §4.2:
//!
//! ```text
//!     l_e* := (A_{v-}* + A_{v+}*) / (2 * l_e)
//! ```
//!
//! This is:
//!
//! 1. **Exact** on the C=1 cubed sphere by symmetry: every
//!    `A_v* = pi/2` and every `l_e = pi/2` give `l_e* = pi/2`, which
//!    is the geodesic distance between adjacent cube-face centers.
//! 2. **Boundary-preserving**: no new accessor on
//!    `LatticeWithMetric`. Phase 1/2 additivity contract held.
//! 3. **First-order correct** as the cubed-sphere refinement `C → ∞`
//!    (the convergence story owed in a separate scope, joint with
//!    AURORA).
//!
//! The exact **circumcentric** dual (each `l_e*` = great-circle
//! distance between adjacent cell circumcenters) is a Phase 3 upgrade
//! gated on adding `dual_edge_lengths: Option<Vec<f64>>` to
//! [`crate::lattice::LatticeWithMetric`]. When Phase 3 lands, this
//! module flips to consume the accessor when present and fall back to
//! the barycentric formula when the constructor declines. The function
//! signature is stable through that change.

use crate::lattice::dec::DecError;
use crate::lattice::LatticeWithMetric;

/// Discrete codifferential on a 1-form.
///
/// Preconditions:
/// - `u.len() == lwm.lattice().n_edges()`
/// - `lwm.edge_lengths()` non-empty with length `n_edges`.
/// - `lwm.dual_face_areas()` is `Some(_)` with length `n_vertices`.
///
/// Returns a `Vec<f64>` of length `n_vertices`, semantically a
/// `Form0`. The discrete Laplacian on 0-forms is `delta_1 ∘ d_0`; on
/// the constant input it is bit-identical zero (algebraic identity).
///
/// Errors:
/// - [`DecError::LengthMismatch`] with surface `"delta_1::u"`.
/// - [`DecError::EdgeLengthsMissing`] if `edge_lengths` is empty.
/// - [`DecError::DualFaceAreasMissing`] if `dual_face_areas` is None.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn delta_1(lwm: &LatticeWithMetric, u: &[f64]) -> Result<Vec<f64>, DecError> {
    let lat = lwm.lattice();
    let n_e = lat.n_edges();
    let n_v = lat.n_vertices;
    if u.len() != n_e {
        return Err(DecError::LengthMismatch {
            expected: n_e,
            actual: u.len(),
            surface: "delta_1::u",
        });
    }
    let dual = lwm.dual_face_areas().ok_or(DecError::DualFaceAreasMissing)?;
    let edge_lengths = lwm.edge_lengths();
    if edge_lengths.is_empty() {
        return Err(DecError::EdgeLengthsMissing);
    }

    // Accumulate signed flux contributions at each vertex.
    let mut acc = vec![0.0_f64; n_v];
    for (e, &(tail, head)) in lat.edges.iter().enumerate() {
        let l_e = edge_lengths[e];
        let l_dual = (dual[tail] + dual[head]) / (2.0 * l_e);
        let ratio = l_dual / l_e;
        let weighted = ratio * u[e];
        // sigma(e, head) = +1; sigma(e, tail) = -1.
        acc[head] += weighted;
        acc[tail] -= weighted;
    }
    // Divide by A_v* at each vertex.
    for v in 0..n_v {
        acc[v] /= dual[v];
    }
    Ok(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::Lattice;

    fn full_metric_quad() -> LatticeWithMetric {
        let lat = Lattice::new(
            "q",
            4,
            vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            vec![vec![0, 1, 2, 3]],
            None,
        );
        LatticeWithMetric::from_lattice_and_metric(
            lat,
            vec![1.0],
            vec![1.0; 4],
            Some(vec![1.0; 4]),
        )
    }

    #[test]
    fn delta_1_length_mismatch() {
        let lwm = full_metric_quad();
        let err = delta_1(&lwm, &[0.0]).unwrap_err();
        assert!(matches!(err, DecError::LengthMismatch { surface: "delta_1::u", .. }));
    }

    #[test]
    fn delta_1_missing_dual() {
        let lat = Lattice::new(
            "q",
            4,
            vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            vec![vec![0, 1, 2, 3]],
            None,
        );
        let lwm = LatticeWithMetric::from_lattice_and_metric(lat, vec![1.0], vec![1.0; 4], None);
        let err = delta_1(&lwm, &[0.0; 4]).unwrap_err();
        assert_eq!(err, DecError::DualFaceAreasMissing);
    }

    #[test]
    fn delta_1_missing_edge_lengths() {
        let lat = Lattice::new(
            "q",
            4,
            vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            vec![vec![0, 1, 2, 3]],
            None,
        );
        let lwm = LatticeWithMetric::from_lattice_and_metric(
            lat,
            vec![1.0],
            Vec::new(),
            Some(vec![1.0; 4]),
        );
        let err = delta_1(&lwm, &[0.0; 4]).unwrap_err();
        assert_eq!(err, DecError::EdgeLengthsMissing);
    }
}
