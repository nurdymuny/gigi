//! AURORA Phase 2 — DEC operators on `LatticeWithMetric` (RED-first).
//!
//! Receipt: AURORA Round 2 commitment (commit ad306ec, status board
//! row Q3). This file pins the contract of the new free functions
//! `gigi::lattice::dec::{d_0, delta_1, hodge_star_0, hodge_star_1,
//! hodge_star_2}` plus the `DecError` enum BEFORE any implementation
//! lands. The integration story is one suite because AURORA's
//! ShallowWater force kernel consumes grad/div/hodge as a unit, and
//! the cubed-sphere C=1 fixture is shared across operators.
//!
//! Conventions (locked):
//!
//! - `d_0`: pure combinatorics on canonical edge orientation
//!   `(tail, head)` — `(d_0 phi)[e] = phi[head] - phi[tail]`.
//! - `delta_1`: barycentric dual edge length
//!   `l_e* = (A_{v-}* + A_{v+}*) / (2 * l_e)`; no new accessor on
//!   `LatticeWithMetric`.
//! - `hodge_star_0`: `(star_0 phi)[v] = A_v* * phi[v]`.
//! - `hodge_star_1`: `(star_1 u)[e] = (l_e* / l_e) * u[e]`.
//! - `hodge_star_2`: `(star_2 omega)[c] = omega[c] / A_c`.
//! - Sign convention matches `src/discrete/hodge_complex.rs`:
//!   `d_0[e, v] = +1` if `v == canonical head`, `-1` if `v ==
//!   canonical tail`.
//!
//! Phase 2 enforces NO tunable tolerances on algebraic identities
//! (constant-input results are bit-identical `0.0`); only the
//! sum-check of `A_v*` over the sphere is allowed a 1e-10 numerical
//! envelope (sum of f64 accumulations).

#![cfg(feature = "lattice")]

use gigi::lattice::dec::{d_0, delta_1, hodge_star_0, hodge_star_1, hodge_star_2, DecError};
use gigi::lattice::topology::cubed_sphere;
use gigi::lattice::{Lattice, LatticeWithMetric};

// ── Fixtures ──────────────────────────────────────────────────────────

mod fixtures {
    use super::*;

    /// C=1 cubed sphere — 8 vertices, 12 edges, 6 quad faces.
    ///
    /// The bare constructor returns `dual_face_areas = None` (Phase 1
    /// punted on the dual mesh). For DEC tests that need the dual
    /// (`delta_1`, `hodge_star_0`, `hodge_star_1`) we wrap the
    /// constructor output in a fixture that re-bundles it through
    /// `from_lattice_and_metric` with the symmetric C=1 dual face area
    /// `A_v* = 4*pi/8 = pi/2` populated on each of the 8 corners.
    pub fn cubed_sphere_c1_with_dual() -> LatticeWithMetric {
        let bare = cubed_sphere::build(1).expect("C=1 build");
        let lat = bare.lattice().clone();
        let dual = vec![std::f64::consts::PI / 2.0; lat.n_vertices];
        LatticeWithMetric::from_lattice_and_metric(
            lat,
            bare.cell_areas().to_vec(),
            bare.edge_lengths().to_vec(),
            Some(dual),
        )
    }

    /// C=1 cubed sphere WITHOUT a dual (constructor default). Use for
    /// the `DualFaceAreasMissing` error-path tests.
    pub fn cubed_sphere_c1_no_dual() -> LatticeWithMetric {
        cubed_sphere::build(1).expect("C=1 build")
    }

    /// C=1 cubed sphere with the dual populated but `edge_lengths`
    /// blanked. Synthetic fixture for the `EdgeLengthsMissing` path.
    pub fn cubed_sphere_c1_dual_but_no_edge_lengths() -> LatticeWithMetric {
        let bare = cubed_sphere::build(1).expect("C=1 build");
        let lat = bare.lattice().clone();
        let dual = vec![std::f64::consts::PI / 2.0; lat.n_vertices];
        LatticeWithMetric::from_lattice_and_metric(
            lat,
            bare.cell_areas().to_vec(),
            Vec::new(),
            Some(dual),
        )
    }

    /// Zero-metric placeholder on a tiny quad: 4 vertices, 4 edges,
    /// 1 face, no cell areas, no edge lengths, no dual. Use for the
    /// `CellAreasMissing` / `EdgeLengthsMissing` / `DualFaceAreasMissing`
    /// error-path coverage without pulling cubed_sphere in.
    pub fn zero_metric_quad() -> LatticeWithMetric {
        let lat = Lattice::new(
            "zero_metric_quad",
            4,
            vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            vec![vec![0, 1, 2, 3]],
            None,
        );
        LatticeWithMetric::from_lattice_and_metric(lat, Vec::new(), Vec::new(), None)
    }

    /// Tiny 4-vertex single-quad fixture shared between the
    /// `lattice::dec` operators and a hand-built `HodgeComplex` for
    /// the sign-convention cross-check. The lattice has explicit
    /// edges in canonical `(min, max)` order matching the
    /// `HodgeComplex` invariant `i < j`. Metric values are dummies
    /// (1.0 everywhere) since `d_0` is pure combinatorics.
    pub fn shared_quad_for_hodge_complex_cross_check() -> LatticeWithMetric {
        let lat = Lattice::new(
            "shared_quad",
            4,
            // Edges in canonical (i, j), i < j order so HodgeComplex
            // accepts them without re-ordering.
            vec![(0, 1), (1, 2), (2, 3), (0, 3)],
            vec![vec![0, 1, 2, 3]],
            None,
        );
        let dual = vec![1.0_f64; 4];
        LatticeWithMetric::from_lattice_and_metric(
            lat,
            vec![1.0],
            vec![1.0; 4],
            Some(dual),
        )
    }
}

// ── d_0 ───────────────────────────────────────────────────────────────

mod d_0_tests {
    use super::*;

    /// Identity 1: `d_0` of the constant function on the C=1 cubed
    /// sphere is the zero 1-form, EXACTLY (no tolerance).
    #[test]
    fn d_0_of_constant_is_zero_vector() {
        let lwm = fixtures::cubed_sphere_c1_no_dual();
        let n_v = lwm.lattice().n_vertices;
        let n_e = lwm.lattice().n_edges();
        assert_eq!(n_v, 8);
        assert_eq!(n_e, 12);
        let phi = vec![1.0_f64; n_v];
        let out = d_0(&lwm, &phi).expect("d_0 ok on constant");
        assert_eq!(out, vec![0.0_f64; n_e]);
    }

    /// Identity 2: length mismatch returns a structured error with
    /// the documented surface label `"d_0::phi"`.
    #[test]
    fn d_0_length_mismatch_is_structured_error() {
        let lwm = fixtures::cubed_sphere_c1_no_dual();
        let phi = vec![0.0_f64; 7]; // wrong: should be 8
        let err = d_0(&lwm, &phi).unwrap_err();
        assert_eq!(
            err,
            DecError::LengthMismatch {
                expected: 8,
                actual: 7,
                surface: "d_0::phi",
            }
        );
    }

    /// Identity 3: sign-convention pin. `phi[0] = 1`, `phi[i] = 0`
    /// for `i != 0`. Expected: edges with canonical tail==0 carry
    /// `-1.0`; edges with canonical head==0 carry `+1.0`; all others
    /// `0.0`. Reads `edges()` directly rather than hard-coding which
    /// edge ids touch vertex 0, so the test survives any non-
    /// observable reordering inside the cubed-sphere constructor.
    #[test]
    fn d_0_sign_convention_pin_on_indicator_phi() {
        let lwm = fixtures::cubed_sphere_c1_no_dual();
        let n_v = lwm.lattice().n_vertices;
        let mut phi = vec![0.0_f64; n_v];
        phi[0] = 1.0;
        let out = d_0(&lwm, &phi).expect("d_0 ok on indicator");
        for (e, &(tail, head)) in lwm.lattice().edges.iter().enumerate() {
            let expected = if tail == 0 {
                -1.0
            } else if head == 0 {
                1.0
            } else {
                0.0
            };
            assert_eq!(
                out[e], expected,
                "edge {e} = ({tail}, {head}): expected {expected}, got {}",
                out[e]
            );
        }
    }

    /// Stability-marker compile-time check: each public DEC fn binds
    /// through `Result<Vec<f64>, DecError>`. This test is the documented
    /// contract surface — if a future patch changes a return type, the
    /// compile fails here first.
    #[test]
    fn d_0_returns_result_vec_f64_dec_error() {
        let lwm = fixtures::cubed_sphere_c1_no_dual();
        let phi = vec![0.0_f64; lwm.lattice().n_vertices];
        let _out: Result<Vec<f64>, DecError> = d_0(&lwm, &phi);
    }
}

// ── delta_1 ──────────────────────────────────────────────────────────

mod delta_1_tests {
    use super::*;

    /// Identity 4: `delta_1 ∘ d_0 (phi=constant) = 0` exactly on the
    /// C=1 cubed sphere. This is an algebraic identity (sum of equal-
    /// magnitude opposite-sign edge contributions at every vertex),
    /// NOT a convergence claim, so the assertion is bit-identical zero
    /// rather than a tolerance.
    #[test]
    fn delta_1_of_d_0_of_constant_is_exact_zero() {
        let lwm = fixtures::cubed_sphere_c1_with_dual();
        let n_v = lwm.lattice().n_vertices;
        let phi = vec![1.0_f64; n_v];
        let one_form = d_0(&lwm, &phi).expect("d_0 ok");
        let lap = delta_1(&lwm, &one_form).expect("delta_1 ok");
        assert_eq!(lap, vec![0.0_f64; n_v]);
    }

    /// Identity 5: `dual_face_areas == None` → `DualFaceAreasMissing`.
    /// Fixture is the bare cubed-sphere constructor output.
    #[test]
    fn delta_1_missing_dual_face_areas_is_structured_error() {
        let lwm = fixtures::cubed_sphere_c1_no_dual();
        let u = vec![0.0_f64; lwm.lattice().n_edges()];
        let err = delta_1(&lwm, &u).unwrap_err();
        assert_eq!(err, DecError::DualFaceAreasMissing);
    }

    /// Identity 6: `edge_lengths` empty but `dual_face_areas` Some →
    /// `EdgeLengthsMissing`. Synthetic fixture re-bundles cubed-sphere
    /// C=1 with the edge-length vector blanked.
    #[test]
    fn delta_1_missing_edge_lengths_is_structured_error() {
        let lwm = fixtures::cubed_sphere_c1_dual_but_no_edge_lengths();
        let u = vec![0.0_f64; lwm.lattice().n_edges()];
        let err = delta_1(&lwm, &u).unwrap_err();
        assert_eq!(err, DecError::EdgeLengthsMissing);
    }

    /// Identity 7: `u.len() != n_edges` → `LengthMismatch` with the
    /// `"delta_1::u"` surface label.
    #[test]
    fn delta_1_length_mismatch_is_structured_error() {
        let lwm = fixtures::cubed_sphere_c1_with_dual();
        let u = vec![0.0_f64; 11]; // wrong: should be 12
        let err = delta_1(&lwm, &u).unwrap_err();
        assert_eq!(
            err,
            DecError::LengthMismatch {
                expected: 12,
                actual: 11,
                surface: "delta_1::u",
            }
        );
    }

    /// Stability marker for `delta_1`.
    #[test]
    fn delta_1_returns_result_vec_f64_dec_error() {
        let lwm = fixtures::cubed_sphere_c1_with_dual();
        let u = vec![0.0_f64; lwm.lattice().n_edges()];
        let _out: Result<Vec<f64>, DecError> = delta_1(&lwm, &u);
    }
}

// ── hodge_star_0 ─────────────────────────────────────────────────────

mod hodge_star_0_tests {
    use super::*;

    /// Identity 10a: `(star_0 phi)[v] = A_v* * phi[v]`. With phi=1 the
    /// output equals `dual_face_areas` elementwise.
    /// Sum-check: total dual area covers the sphere → `sum = 4*pi`
    /// within 1e-10. This is the only test in the suite that uses an
    /// f64 tolerance, and it does so because summing 8 f64 values
    /// against the analytic 4*pi is the one numerical (not algebraic)
    /// claim being made.
    #[test]
    fn hodge_star_0_of_constant_one_is_dual_face_areas() {
        let lwm = fixtures::cubed_sphere_c1_with_dual();
        let n_v = lwm.lattice().n_vertices;
        let phi = vec![1.0_f64; n_v];
        let out = hodge_star_0(&lwm, &phi).expect("hodge_star_0 ok");
        let dual = lwm.dual_face_areas().expect("fixture sets dual");
        assert_eq!(out.len(), n_v);
        for v in 0..n_v {
            assert_eq!(out[v], dual[v], "vertex {v}: star_0(1) must equal A_v*");
        }
        let sum: f64 = out.iter().sum();
        let four_pi = 4.0 * std::f64::consts::PI;
        assert!(
            (sum - four_pi).abs() < 1e-10,
            "sum of dual areas = {sum}, expected 4*pi = {four_pi}"
        );
    }

    /// Identity 11: `dual_face_areas == None` → `DualFaceAreasMissing`.
    #[test]
    fn hodge_star_0_missing_dual_is_structured_error() {
        let lwm = fixtures::zero_metric_quad();
        let phi = vec![0.0_f64; lwm.lattice().n_vertices];
        let err = hodge_star_0(&lwm, &phi).unwrap_err();
        assert_eq!(err, DecError::DualFaceAreasMissing);
    }

    /// Length-mismatch path for `hodge_star_0`.
    #[test]
    fn hodge_star_0_length_mismatch_is_structured_error() {
        let lwm = fixtures::cubed_sphere_c1_with_dual();
        let phi = vec![0.0_f64; 7]; // wrong: should be 8
        let err = hodge_star_0(&lwm, &phi).unwrap_err();
        assert_eq!(
            err,
            DecError::LengthMismatch {
                expected: 8,
                actual: 7,
                surface: "hodge_star_0::phi",
            }
        );
    }
}

// ── hodge_star_1 ─────────────────────────────────────────────────────

mod hodge_star_1_tests {
    use super::*;

    /// Identity 12: on the C=1 cubed sphere with the fixture's
    /// `A_v* = pi/2` and actual primal edge length `l_e = arccos(1/3)`
    /// (great-circle arc between adjacent cube corners on the unit
    /// sphere), the barycentric dual edge length is
    /// `l_e* = (pi/2 + pi/2)/(2*l_e) = pi/(2*l_e)`, and the Hodge
    /// ratio is `l_e* / l_e = pi/(2 * l_e^2)`. By full symmetry every
    /// edge gets the same ratio, so `(star_1 u=1)[e]` is constant
    /// across all 12 edges — bit-identical equality between entries,
    /// and bit-identical equality to the formula computed against
    /// `lwm.edge_lengths()` directly (so the test is robust to the
    /// exact f64 form of `arccos(1/3)`).
    #[test]
    fn hodge_star_1_of_constant_is_one_on_c1_by_symmetry() {
        let lwm = fixtures::cubed_sphere_c1_with_dual();
        let n_e = lwm.lattice().n_edges();
        let u = vec![1.0_f64; n_e];
        let out = hodge_star_1(&lwm, &u).expect("hodge_star_1 ok");
        // Expected per edge: (l_e* / l_e) * 1.0 where
        // l_e* = (A_{v-}* + A_{v+}*)/(2 * l_e) and A_v* = pi/2.
        let dual = lwm.dual_face_areas().expect("fixture sets dual");
        let edge_lengths = lwm.edge_lengths();
        for (e, &(tail, head)) in lwm.lattice().edges.iter().enumerate() {
            let l_e = edge_lengths[e];
            let l_dual = (dual[tail] + dual[head]) / (2.0 * l_e);
            let expected = l_dual / l_e;
            assert_eq!(
                out[e], expected,
                "edge {e}: star_1(1) ratio mismatch"
            );
        }
        // Full symmetry pin: every entry equals the first entry.
        for e in 1..n_e {
            assert_eq!(out[e], out[0], "edge {e} should match edge 0 by symmetry");
        }
    }

    /// `edge_lengths` missing → `EdgeLengthsMissing`.
    #[test]
    fn hodge_star_1_missing_edge_lengths_is_structured_error() {
        let lwm = fixtures::cubed_sphere_c1_dual_but_no_edge_lengths();
        let u = vec![0.0_f64; lwm.lattice().n_edges()];
        let err = hodge_star_1(&lwm, &u).unwrap_err();
        assert_eq!(err, DecError::EdgeLengthsMissing);
    }

    /// `dual_face_areas` missing → `DualFaceAreasMissing` (the dual
    /// face areas are required to compute `l_e*`).
    #[test]
    fn hodge_star_1_missing_dual_is_structured_error() {
        let lwm = fixtures::cubed_sphere_c1_no_dual();
        let u = vec![0.0_f64; lwm.lattice().n_edges()];
        let err = hodge_star_1(&lwm, &u).unwrap_err();
        assert_eq!(err, DecError::DualFaceAreasMissing);
    }

    /// Length-mismatch path.
    #[test]
    fn hodge_star_1_length_mismatch_is_structured_error() {
        let lwm = fixtures::cubed_sphere_c1_with_dual();
        let u = vec![0.0_f64; 11]; // wrong: should be 12
        let err = hodge_star_1(&lwm, &u).unwrap_err();
        assert_eq!(
            err,
            DecError::LengthMismatch {
                expected: 12,
                actual: 11,
                surface: "hodge_star_1::u",
            }
        );
    }
}

// ── hodge_star_2 ─────────────────────────────────────────────────────

mod hodge_star_2_tests {
    use super::*;

    /// Identity 8: `(star_2 omega=1)[c] = 1.0 / cell_areas[c]`,
    /// elementwise. Asserted against `lwm.cell_areas()` directly
    /// (NOT against the analytic `6 / (4*pi)`) so the test is robust
    /// to spherical-excess corrections inside the constructor.
    #[test]
    fn hodge_star_2_of_constant_one_is_inverse_cell_areas() {
        let lwm = fixtures::cubed_sphere_c1_no_dual();
        let n_f = lwm.lattice().n_faces();
        assert_eq!(n_f, 6);
        let omega = vec![1.0_f64; n_f];
        let out = hodge_star_2(&lwm, &omega).expect("hodge_star_2 ok");
        let areas = lwm.cell_areas();
        assert_eq!(out.len(), n_f);
        for c in 0..n_f {
            assert_eq!(out[c], 1.0 / areas[c], "face {c}: star_2(1) = 1/A_c");
        }
    }

    /// Identity 9: `cell_areas` empty → `CellAreasMissing`.
    #[test]
    fn hodge_star_2_missing_cell_areas_is_structured_error() {
        let lwm = fixtures::zero_metric_quad();
        let omega = vec![0.0_f64; lwm.lattice().n_faces()];
        let err = hodge_star_2(&lwm, &omega).unwrap_err();
        assert_eq!(err, DecError::CellAreasMissing);
    }

    /// Length-mismatch path with `"hodge_star_2::omega"` label.
    #[test]
    fn hodge_star_2_length_mismatch_is_structured_error() {
        let lwm = fixtures::cubed_sphere_c1_no_dual();
        let omega = vec![0.0_f64; 5]; // wrong: should be 6
        let err = hodge_star_2(&lwm, &omega).unwrap_err();
        assert_eq!(
            err,
            DecError::LengthMismatch {
                expected: 6,
                actual: 5,
                surface: "hodge_star_2::omega",
            }
        );
    }
}

// ── Cross-module pin ─────────────────────────────────────────────────
//
// The bundle-side `HodgeComplex` lives behind the `kahler` feature
// (see `src/lib.rs`), so the sign-convention cross-check only compiles
// when both `lattice` and `kahler` are enabled. Run with
// `--features halcyon,kahler` to exercise it; the halcyon-only test
// run still gets the other 17 tests and the primary contract.

#[cfg(feature = "kahler")]
mod cross_module_pins {
    use super::*;
    use gigi::discrete::hodge_complex::HodgeComplex;

    /// Identity 13: sign-convention cross-check against the
    /// `src/discrete/hodge_complex.rs` peer. Both modules are asked
    /// to compute `d_0 phi` on the same 4-vertex single-quad
    /// fixture with a non-constant `phi`; the two outputs must agree
    /// elementwise. Pins that `lattice::dec` did not accidentally
    /// invert the convention relative to the already-validated
    /// bundle-side `HodgeComplex`.
    ///
    /// Note: the bundle-side `HodgeComplex` consumes faces as
    /// 3-cliques `(i, j, k)`, so the cross-check is performed at the
    /// `d_0` level (vertices → edges) only — the lattice-side quad
    /// face is irrelevant for this comparison and the HodgeComplex
    /// side is built with an empty face list.
    #[test]
    fn d_0_sign_matches_discrete_hodge_complex() {
        let lwm = fixtures::shared_quad_for_hodge_complex_cross_check();
        let n_v = lwm.lattice().n_vertices;
        // Non-constant phi so the comparison is non-trivial.
        let phi = vec![0.0_f64, 1.0, 2.5, -3.0];
        assert_eq!(phi.len(), n_v);

        // Lattice-side d_0.
        let out_lat = d_0(&lwm, &phi).expect("lattice d_0 ok");

        // HodgeComplex side — same vertex count, same edges in
        // canonical (i, j) order, empty face list (we only need d_0).
        let edges = lwm
            .lattice()
            .edges
            .iter()
            .copied()
            .collect::<Vec<(usize, usize)>>();
        let hc = HodgeComplex::new(n_v, edges, Vec::new())
            .expect("HodgeComplex builds on canonical-edge fixture");

        // hc.d0 has shape |E| x |V|; multiply by phi as a column.
        let phi_col = nalgebra::DVector::from_column_slice(&phi);
        let out_hc = &hc.d0 * &phi_col;

        assert_eq!(out_lat.len(), out_hc.len());
        for e in 0..out_lat.len() {
            assert_eq!(
                out_lat[e], out_hc[e],
                "edge {e}: lattice::dec::d_0 = {} vs HodgeComplex.d0*phi = {}",
                out_lat[e], out_hc[e]
            );
        }
    }
}
