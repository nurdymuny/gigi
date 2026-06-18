//! Generalized HOLONOMY walker.
//!
//! Closes TDD-HAL-I.4 (walker on an identity connection returns
//! identity on every face) and TDD-HAL-I.5 (orientation false-pass
//! guard — forward · backward = identity to FP64 tolerance).
//!
//! The walker is group-erased: it never names a specific group. It
//! reads each edge's `GroupElement` through `EdgeConnection`,
//! composes left-to-right per the convention pinned in the harvest
//! phase (`buckyball_gold_provenance.json` →
//! `compose_order: "left-action; for (e,s) in face[1:]: h =
//! qmul(h, U_e^s)"`).

use super::edge_connection::EdgeConnection;
use super::group_element::GroupElement;
use crate::lattice::{EdgeId, EdgeOrientation, Lattice, VertexId};

/// Walk an edge-list loop on a lattice against a connection.
/// Returns the accumulated `GroupElement` (the face holonomy when
/// the edges enumerate a face cycle).
///
/// Composition order is left-to-right: `h ← qmul(h, U_e^s)`. The
/// initial value `h` is the SU(2) identity if `edges` is empty, or
/// the first edge's `GroupElement` otherwise — equivalently, the
/// loop's product `U_{e_0}^{s_0} · U_{e_1}^{s_1} · … ·
/// U_{e_{k-1}}^{s_{k-1}}` evaluated left-associatively.
pub fn walk_loop(
    _lattice: &Lattice,
    edges: &[(EdgeId, EdgeOrientation)],
    conn: &dyn EdgeConnection,
) -> GroupElement {
    let mut h = GroupElement::su2_identity();
    for &(eid, orient) in edges {
        let u = conn.edge_element(eid, orient);
        h = h.compose(&u);
    }
    h
}

/// Resolve a face's cyclic vertex list to a list of `(EdgeId,
/// EdgeOrientation)` pairs. Convenience for callers that want to
/// walk a declared face; the gold gate uses this to thread the
/// LATTICE's stored face cycles through the walker.
pub fn face_edges(lattice: &Lattice, face_index: usize) -> Vec<(EdgeId, EdgeOrientation)> {
    let face = &lattice.faces[face_index];
    let n = face.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a: VertexId = face[i];
        let b: VertexId = face[(i + 1) % n];
        let (eid, orient) = lattice
            .resolve_edge(a, b)
            .unwrap_or_else(|| panic!("face_edges: ({a},{b}) is not an edge of the lattice"));
        out.push((eid, orient));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::edge_connection::test_support::FixedEdgeConnection;
    use crate::lattice::topology::truncated_icosahedron::buckyball;
    use super::*;

    fn assert_su2_identity(g: GroupElement, tol: f64, ctx: &str) {
        match g {
            GroupElement::SU2 { q0, q1, q2, q3 } => {
                assert!((q0 - 1.0).abs() < tol, "{ctx}: q0={q0}");
                assert!(q1.abs() < tol, "{ctx}: q1={q1}");
                assert!(q2.abs() < tol, "{ctx}: q2={q2}");
                assert!(q3.abs() < tol, "{ctx}: q3={q3}");
            }
            _ => panic!("{ctx}: expected SU2 variant"),
        }
    }

    /// TDD-HAL-I.4 — identity connection on every edge of the
    /// buckyball → walker returns identity on every face.
    #[test]
    fn tdd_hal_i_4_walker_identity_on_every_face() {
        let lat = buckyball();
        let conn = FixedEdgeConnection::identity_everywhere();
        for fidx in 0..lat.n_faces() {
            let edges = face_edges(&lat, fidx);
            let h = walk_loop(&lat, &edges, &conn);
            // FP64 identity for f64 product of identity quaternions
            // is exact: 1.0 * 1.0 = 1.0, no accumulation. So tol = 0.
            assert_eq!(h, GroupElement::su2_identity(), "face {fidx}");
        }
    }

    /// TDD-HAL-I.5 — orientation false-pass guard. Plant a non-
    /// identity element on a single edge; forward traversal of a
    /// 3-edge path containing that edge produces a non-identity
    /// quaternion, backward traversal produces its inverse, and
    /// their composition is identity within FP64 tolerance.
    ///
    /// This is the load-bearing receipt that the walker actually
    /// reads orientation off the LATTICE incidence and routes it
    /// through `EdgeConnection::edge_element`. A pre-bug version
    /// that always returns the canonical U_e (ignoring orientation)
    /// would pass TDD-HAL-I.4 and the gold gate, but fail here.
    #[test]
    fn tdd_hal_i_5_orientation_sensitivity() {
        let lat = buckyball();

        // Half-turn about z-axis: θ = π → q0 = cos(π/2) = 0,
        // q3 = sin(π/2) = 1.
        let half_turn_z = GroupElement::SU2 {
            q0: 0.0,
            q1: 0.0,
            q2: 0.0,
            q3: 1.0,
        };

        // Plant the rotation on edge index 7 (an arbitrary inner
        // pentagon edge). Every other edge stays identity.
        let conn = FixedEdgeConnection::identity_everywhere()
            .with_edge(7, half_turn_z);

        // 3-edge forward path: edges 5, 7, 9 all forward.
        // The first and third edge contribute identity, the middle
        // edge contributes the half-turn → product is the half-
        // turn, NOT identity.
        let forward_path = vec![
            (5usize, EdgeOrientation::Forward),
            (7usize, EdgeOrientation::Forward),
            (9usize, EdgeOrientation::Forward),
        ];
        let h_fwd = walk_loop(&lat, &forward_path, &conn);
        // Not identity.
        match h_fwd {
            GroupElement::SU2 { q0, .. } => {
                assert!((q0 - 1.0).abs() > 1e-6, "forward path must not be identity");
                // Specifically: half-turn → q0 = 0.
                assert!(q0.abs() < 1e-14, "expected half-turn q0 = 0, got {q0}");
            }
            _ => panic!("expected SU2"),
        }

        // Backward path: edges 9, 7, 5 all reversed (traversing
        // the same physical path in the opposite direction).
        let backward_path = vec![
            (9usize, EdgeOrientation::Reverse),
            (7usize, EdgeOrientation::Reverse),
            (5usize, EdgeOrientation::Reverse),
        ];
        let h_bwd = walk_loop(&lat, &backward_path, &conn);
        // Backward holonomy must equal forward inverse.
        let h_fwd_inv = h_fwd.inverse();
        match (h_bwd, h_fwd_inv) {
            (
                GroupElement::SU2 { q0: a0, q1: a1, q2: a2, q3: a3 },
                GroupElement::SU2 { q0: b0, q1: b1, q2: b2, q3: b3 },
            ) => {
                assert!((a0 - b0).abs() < 1e-14);
                assert!((a1 - b1).abs() < 1e-14);
                assert!((a2 - b2).abs() < 1e-14);
                assert!((a3 - b3).abs() < 1e-14);
            }
            _ => panic!("expected SU2"),
        }

        // Forward composed with backward is identity.
        let h_round = h_fwd.compose(&h_bwd);
        assert_su2_identity(h_round, 1e-14, "forward ∘ backward");
    }
}
