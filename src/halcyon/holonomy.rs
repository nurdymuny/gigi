//! Generalized HOLONOMY walker.
//!
//! Closes TDD-HAL-I.4 (walker on an identity connection returns
//! identity on every face). The orientation false-pass guard
//! (TDD-HAL-I.5) lands in the next commit and exercises the same
//! `walk_loop` against a non-identity FixedConnection.
//!
//! The walker is group-erased: it never names a specific group. It
//! reads each edge's `GroupElement` through `EdgeConnection`,
//! composes left-to-right per the convention pinned in the harvest
//! phase (`buckyball_gold_provenance.json` →
//! `compose_order: "left-action; for (e,s) in face[1:]: h =
//! qmul(h, U_e^s)"`).

use super::edge_connection::EdgeConnection;
use super::group_element::GroupElement;
use super::lattice::{EdgeId, EdgeOrientation, Lattice, VertexId};

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
    use super::super::truncated_icosahedron::buckyball;
    use super::*;

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
}
