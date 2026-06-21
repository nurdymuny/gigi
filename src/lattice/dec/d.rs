//! `d_0` — discrete exterior derivative `Form0 → Form1`.
//!
//! Pure combinatorics: for canonical edge `e = (tail, head)`,
//! `(d_0 phi)[e] = phi[head] - phi[tail]`. Does not read
//! `cell_areas` / `edge_lengths` / `dual_face_areas` — safe to call on
//! a Phase 1 zero-metric placeholder lattice.
//!
//! Sign convention matches `src/discrete/hodge_complex.rs`:
//! `d_0[e, v] = +1` if `v == head`, `-1` if `v == tail`. The cross-
//! module pin test `tests/aurora_dec_operators.rs::cross_module_pins`
//! verifies this equivalence on a shared 4-vertex single-quad fixture.

use crate::lattice::dec::DecError;
use crate::lattice::LatticeWithMetric;

/// Discrete exterior derivative on a 0-form.
///
/// Preconditions:
/// - `phi.len() == lwm.lattice().n_vertices`
///
/// Returns a `Vec<f64>` of length `lwm.lattice().n_edges()`,
/// semantically a `Form1`. For canonical edge `e = (tail, head)`:
/// `out[e] = phi[head] - phi[tail]`.
///
/// Errors:
/// - [`DecError::LengthMismatch`] with surface `"d_0::phi"` if input
///   length is wrong.
///
/// Pure combinatorics — does NOT read cell_areas / edge_lengths /
/// dual_face_areas. Safe to call on a zero-metric placeholder lattice.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn d_0(lwm: &LatticeWithMetric, phi: &[f64]) -> Result<Vec<f64>, DecError> {
    let lat = lwm.lattice();
    let n_v = lat.n_vertices;
    if phi.len() != n_v {
        return Err(DecError::LengthMismatch {
            expected: n_v,
            actual: phi.len(),
            surface: "d_0::phi",
        });
    }
    let mut out = Vec::with_capacity(lat.n_edges());
    for &(tail, head) in lat.edges.iter() {
        out.push(phi[head] - phi[tail]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::Lattice;

    fn tiny_quad() -> LatticeWithMetric {
        let lat = Lattice::new(
            "q",
            4,
            vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            vec![vec![0, 1, 2, 3]],
            None,
        );
        LatticeWithMetric::from_lattice_and_metric(lat, Vec::new(), Vec::new(), None)
    }

    #[test]
    fn d_0_zero_input_yields_zero_output() {
        let lwm = tiny_quad();
        let out = d_0(&lwm, &[0.0, 0.0, 0.0, 0.0]).expect("ok");
        assert_eq!(out, vec![0.0; 4]);
    }

    #[test]
    fn d_0_length_mismatch_carries_surface_label() {
        let lwm = tiny_quad();
        let err = d_0(&lwm, &[0.0, 0.0]).unwrap_err();
        assert_eq!(
            err,
            DecError::LengthMismatch {
                expected: 4,
                actual: 2,
                surface: "d_0::phi",
            }
        );
    }

    #[test]
    fn d_0_sign_is_head_minus_tail() {
        let lwm = tiny_quad();
        // phi[0]=10, phi[1]=20; edge (0,1) ⇒ out[0] = 20-10 = 10.
        let out = d_0(&lwm, &[10.0, 20.0, 0.0, 0.0]).expect("ok");
        assert_eq!(out[0], 10.0);
    }
}
