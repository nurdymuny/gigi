//! Primal-dual Hodge stars on a 2-manifold lattice.
//!
//! Three operators in this Phase 2 surface:
//!
//! - [`hodge_star_0`]: `Form0 → Form2` weighted by dual cell area.
//!   `(star_0 phi)[v] = A_v* * phi[v]`.
//! - [`hodge_star_1`]: `Form1 → Form1` primal-to-dual edge ratio.
//!   `(star_1 u)[e] = (l_e* / l_e) * u[e]` using the barycentric dual
//!   edge length `l_e* = (A_{v-}* + A_{v+}*) / (2 * l_e)`.
//! - [`hodge_star_2`]: `Form2 → Form0` inverse cell area weighting.
//!   `(star_2 omega)[c] = omega[c] / A_c`.
//!
//! Sign convention pinned against `src/discrete/hodge_complex.rs`. The
//! dual edge length convention is documented in
//! `src/lattice/dec/codifferential.rs`; this module reuses the same
//! barycentric formula so `delta_1 = star_2 d_1 star_1` (or any of the
//! standard codifferential factorizations) composes consistently.

use crate::lattice::dec::DecError;
use crate::lattice::LatticeWithMetric;

/// Hodge star on a 0-form: `(star_0 phi)[v] = A_v* * phi[v]`.
///
/// Preconditions:
/// - `phi.len() == lwm.lattice().n_vertices`
/// - `lwm.dual_face_areas()` is `Some(_)` with length `n_vertices`.
///
/// Returns a `Vec<f64>` of length `n_vertices`, semantically a dual
/// `Form2` (dual cell mass per primal vertex).
///
/// Errors:
/// - [`DecError::LengthMismatch`] with surface `"hodge_star_0::phi"`.
/// - [`DecError::DualFaceAreasMissing`] if the lattice has no dual.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn hodge_star_0(lwm: &LatticeWithMetric, phi: &[f64]) -> Result<Vec<f64>, DecError> {
    let n_v = lwm.lattice().n_vertices;
    if phi.len() != n_v {
        return Err(DecError::LengthMismatch {
            expected: n_v,
            actual: phi.len(),
            surface: "hodge_star_0::phi",
        });
    }
    let dual = lwm.dual_face_areas().ok_or(DecError::DualFaceAreasMissing)?;
    let mut out = Vec::with_capacity(n_v);
    for v in 0..n_v {
        out.push(dual[v] * phi[v]);
    }
    Ok(out)
}

/// Hodge star on a 1-form: `(star_1 u)[e] = (l_e* / l_e) * u[e]` with
/// `l_e* = (A_{v-}* + A_{v+}*) / (2 * l_e)` (barycentric dual edge).
///
/// Preconditions:
/// - `u.len() == lwm.lattice().n_edges()`
/// - `lwm.edge_lengths()` non-empty with length `n_edges`.
/// - `lwm.dual_face_areas()` is `Some(_)` with length `n_vertices`.
///
/// Returns a `Vec<f64>` of length `n_edges`, semantically a dual
/// `Form1`.
///
/// Errors:
/// - [`DecError::LengthMismatch`] with surface `"hodge_star_1::u"`.
/// - [`DecError::EdgeLengthsMissing`] if the wrapper has no edge
///   lengths.
/// - [`DecError::DualFaceAreasMissing`] if the wrapper has no dual.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn hodge_star_1(lwm: &LatticeWithMetric, u: &[f64]) -> Result<Vec<f64>, DecError> {
    let lat = lwm.lattice();
    let n_e = lat.n_edges();
    if u.len() != n_e {
        return Err(DecError::LengthMismatch {
            expected: n_e,
            actual: u.len(),
            surface: "hodge_star_1::u",
        });
    }
    let edge_lengths = lwm.edge_lengths();
    if edge_lengths.is_empty() {
        return Err(DecError::EdgeLengthsMissing);
    }
    let dual = lwm.dual_face_areas().ok_or(DecError::DualFaceAreasMissing)?;
    let mut out = Vec::with_capacity(n_e);
    for (e, &(tail, head)) in lat.edges.iter().enumerate() {
        let l_e = edge_lengths[e];
        // Barycentric dual edge length.
        let l_dual = (dual[tail] + dual[head]) / (2.0 * l_e);
        out.push((l_dual / l_e) * u[e]);
    }
    Ok(out)
}

/// Hodge star on a 2-form: `(star_2 omega)[c] = omega[c] / A_c`.
///
/// Preconditions:
/// - `omega.len() == lwm.lattice().n_faces()`
/// - `lwm.cell_areas()` non-empty with length `n_faces`.
///
/// Returns a `Vec<f64>` of length `n_faces`, semantically a dual
/// `Form0`.
///
/// Errors:
/// - [`DecError::LengthMismatch`] with surface `"hodge_star_2::omega"`.
/// - [`DecError::CellAreasMissing`] if the cell area vector is empty.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn hodge_star_2(lwm: &LatticeWithMetric, omega: &[f64]) -> Result<Vec<f64>, DecError> {
    let n_f = lwm.lattice().n_faces();
    if omega.len() != n_f {
        return Err(DecError::LengthMismatch {
            expected: n_f,
            actual: omega.len(),
            surface: "hodge_star_2::omega",
        });
    }
    let areas = lwm.cell_areas();
    if areas.is_empty() {
        return Err(DecError::CellAreasMissing);
    }
    let mut out = Vec::with_capacity(n_f);
    for c in 0..n_f {
        out.push(omega[c] / areas[c]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::Lattice;

    fn quad_with_full_metric() -> LatticeWithMetric {
        let lat = Lattice::new(
            "q",
            4,
            vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            vec![vec![0, 1, 2, 3]],
            None,
        );
        LatticeWithMetric::from_lattice_and_metric(
            lat,
            vec![2.0],
            vec![1.0; 4],
            Some(vec![0.5; 4]),
        )
    }

    #[test]
    fn star_0_length_mismatch() {
        let lwm = quad_with_full_metric();
        let err = hodge_star_0(&lwm, &[0.0]).unwrap_err();
        assert!(matches!(err, DecError::LengthMismatch { surface: "hodge_star_0::phi", .. }));
    }

    #[test]
    fn star_1_length_mismatch() {
        let lwm = quad_with_full_metric();
        let err = hodge_star_1(&lwm, &[0.0]).unwrap_err();
        assert!(matches!(err, DecError::LengthMismatch { surface: "hodge_star_1::u", .. }));
    }

    #[test]
    fn star_2_length_mismatch() {
        let lwm = quad_with_full_metric();
        let err = hodge_star_2(&lwm, &[0.0, 0.0]).unwrap_err();
        assert!(matches!(err, DecError::LengthMismatch { surface: "hodge_star_2::omega", .. }));
    }

    #[test]
    fn star_2_happy_path_divides_by_area() {
        let lwm = quad_with_full_metric();
        let out = hodge_star_2(&lwm, &[4.0]).expect("ok");
        assert_eq!(out, vec![2.0]); // 4.0 / 2.0
    }
}
