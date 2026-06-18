//! Per-edge staple-sum walker (read-only helper for `GIBBS_SAMPLE`).
//!
//! Closes TDD-HAL-III.3. The "staple sum" `V_eff(e)` is the quaternion
//! the Kennedy-Pendleton heatbath conditions the single-edge update on:
//!
//! ```text
//!     V_eff(e) = Σ_{f ∋ e} (U_f with U_e removed)
//! ```
//!
//! For each face `f` incident to edge `e`, walk the face's edge cycle
//! starting one position past `e` and accumulate `n_f − 1` signed link
//! products into a quaternion `A_f`. If `e` enters `f` with forward
//! orientation (sign +1), contribute `A_f`; with reverse orientation
//! (sign −1), contribute `qconj(A_f)`. The `qconj` rewrite uses
//! `Re Tr(U_e^† · A) = Re Tr(U_e · A^†)` so `U_e` sits on the LEFT in
//! both cases — that's the form `GIBBS_SAMPLE` consumes.
//!
//! Reference: `davis-wilson-lattice/inertia_damping/buckyball_heatbath.py`
//! `_effective_staple_q` + `buckyball_action.py` `face_staple_at_edge`.
//!
//! Group-erasure note: `staple_sum_at_edge` takes `&dyn EdgeConnection`,
//! not `&dyn GaugeFieldHandle`. `EdgeFaceIncidence` is a lattice-only
//! structure (no group tag, no buffer). The returned `GroupElement`
//! variant matches whatever variant the connection returns; group
//! dispatch happens inside `GIBBS_SAMPLE` (gate III.5), not here.
//!
//! Caching: `build_edge_face_incidence` is `O(F · k)` one-shot
//! (`k` ≤ 6 face perimeter on the buckyball; ~32 · 6 = 192 entries
//! total). Caller is responsible for hoisting the result out of any
//! sweep loop. We deliberately do NOT cache on `Lattice` so the
//! `to_gql` round-trip stays byte-identical (the incidence is derived,
//! not declared).

use super::edge_connection::EdgeConnection;
use super::group_element::GroupElement;
use super::holonomy::face_edges;
use crate::lattice::{EdgeId, EdgeOrientation, Lattice};

/// Per-edge → list of `(face_idx, position_in_face)` entries.
///
/// `inc[eid]` enumerates every face that contains edge `eid`; the
/// `position_in_face` is the index `i` such that the face's vertex
/// cycle pair `(faces[face_idx][i], faces[face_idx][(i+1) % n])`
/// resolves to `eid` (one endpoint of the edge). Closed-surface
/// invariant: on the buckyball (and any closed-surface lattice every
/// `LATTICE … TOPOLOGY 'S2'` declaration commits to) every edge
/// appears in exactly 2 faces.
pub type EdgeFaceIncidence = Vec<Vec<(usize /* face_idx */, usize /* position_in_face */)>>;

/// Build the per-edge → list-of-`(face_idx, position_in_face)`
/// incidence table for `lat`. Thin re-export over
/// `Lattice::build_edge_face_incidence` so callers in the gauge crate
/// can stay in the `gauge::` namespace and get the `EdgeFaceIncidence`
/// type alias without naming `crate::lattice` directly. Iterates
/// faces in ascending `face_idx` order — load-bearing ordering for
/// the staple-sum reduction (mirrors Halcyon's
/// `_edge_face_membership`).
pub fn build_edge_face_incidence(lat: &Lattice) -> EdgeFaceIncidence {
    lat.build_edge_face_incidence()
}

/// Compute the effective staple `V_eff(e)` at `edge` under the
/// connection behind `handle`.
///
/// For each face `f` incident to `edge` (read off `inc[edge]`):
///
/// 1. Resolve the face's edge cycle via `face_edges(lat, fidx)`.
/// 2. Locate `edge` in the cycle at position `pos`.
/// 3. Read the face's edge orientation `σ_f(edge)` at that position
///    (Forward → +1, Reverse → −1).
/// 4. Walk the OTHER `n − 1` edges starting at `(pos + 1) % n` and
///    compose their signed link elements into a quaternion `A_f`.
/// 5. If `σ_f(edge) = +1` contribute `A_f`; if `−1` contribute
///    `qconj(A_f)` (= `A_f.inverse()` for SU(2) unit quaternions).
///
/// Sum across all incident faces. The result is the quaternion
/// `V_eff` such that `P(U_e) ∝ exp((β/N) · Re Tr(U_e · V_eff))` —
/// the Kennedy-Pendleton local Boltzmann weight `GIBBS_SAMPLE`
/// conditions on.
///
/// Convention: at the SU(2) identity every `A_f = I` and
/// `qconj(I) = I`, so `V_eff(e) = (k, 0, 0, 0)` where `k` is the
/// number of incident faces. On a closed surface this is `(2, 0, 0, 0)`
/// for every edge (closed-surface invariant).
pub fn staple_sum_at_edge(
    handle: &dyn EdgeConnection,
    lat: &Lattice,
    inc: &EdgeFaceIncidence,
    edge: EdgeId,
) -> GroupElement {
    // Zero accumulator (SU(2) sums are componentwise quaternion
    // addition; we publish the result as a `GroupElement::SU2`
    // variant carrying the running totals).
    let mut acc_q0 = 0.0_f64;
    let mut acc_q1 = 0.0_f64;
    let mut acc_q2 = 0.0_f64;
    let mut acc_q3 = 0.0_f64;

    for &(fidx, pos) in &inc[edge] {
        let edges = face_edges(lat, fidx);
        let n = edges.len();
        debug_assert!(pos < n);
        // Walk the n - 1 OTHER edges starting one step past `edge`.
        // Compose left-to-right per the convention pinned in the
        // harvest phase (mirror of Halcyon's `face_staple_at_edge`:
        // start with `face[(k+1) % n]`, then qmul with each
        // successive edge through `face[(k + n - 1) % n]`).
        let (e0, o0) = edges[(pos + 1) % n];
        let mut a = handle.edge_element(e0, o0);
        for j in 2..n {
            let (ej, oj) = edges[(pos + j) % n];
            let uj = handle.edge_element(ej, oj);
            a = a.compose(&uj);
        }
        // σ_f(edge) → contribute A_f vs qconj(A_f).
        let (_self_eid, self_orient) = edges[pos];
        let contrib = match self_orient {
            EdgeOrientation::Forward => a,
            EdgeOrientation::Reverse => a.inverse(),
        };
        match contrib {
            GroupElement::SU2 { q0, q1, q2, q3 } => {
                acc_q0 += q0;
                acc_q1 += q1;
                acc_q2 += q2;
                acc_q3 += q3;
            }
            _ => unreachable!(
                "staple_sum_at_edge: connection returned non-SU2 GroupElement; group dispatch belongs in GIBBS_SAMPLE (III.5)"
            ),
        }
    }

    GroupElement::SU2 {
        q0: acc_q0,
        q1: acc_q1,
        q2: acc_q2,
        q3: acc_q3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauge::registry as gauge_registry;
    use crate::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
    use crate::lattice::registry as lattice_registry;
    use crate::lattice::topology::truncated_icosahedron::buckyball;
    use std::sync::Arc;

    /// TDD-HAL-III.3: build_edge_face_incidence on the buckyball —
    /// every edge appears in exactly 2 faces (closed-surface
    /// invariant); the position field round-trips (the face's
    /// vertex-cycle pair at `pos_in_face` resolves to the recorded
    /// edge).
    #[test]
    fn tdd_hal_iii_3_incidence_buckyball_shape() {
        let bb = buckyball();
        let inc = build_edge_face_incidence(&bb);
        assert_eq!(inc.len(), bb.n_edges(), "incidence is per-edge");

        for (eid, entries) in inc.iter().enumerate() {
            assert_eq!(
                entries.len(),
                2,
                "edge {eid} appears in {} faces, expected 2 on closed surface",
                entries.len()
            );
            for &(fidx, pos) in entries {
                let face = &bb.faces[fidx];
                let n = face.len();
                let a = face[pos];
                let b = face[(pos + 1) % n];
                let (resolved_eid, _orient) = bb
                    .resolve_edge(a, b)
                    .unwrap_or_else(|| panic!("face pair ({a},{b}) not an edge"));
                assert_eq!(
                    resolved_eid, eid,
                    "incidence (fidx={fidx}, pos={pos}) → vertex pair ({a},{b}) → edge {resolved_eid}, expected {eid}"
                );
            }
        }
    }

    /// TDD-HAL-III.3: INIT IDENTITY → `staple_sum_at_edge` returns
    /// the sum of identity quaternions, one per incident face. On
    /// the buckyball every edge sits in exactly 2 faces, so the
    /// staple is `(2.0, 0, 0, 0)` exactly. Confirms the convention:
    /// `V_eff(e) = Σ_f A_f` with `A_f = I` when every link is
    /// identity.
    #[test]
    fn tdd_hal_iii_3_staple_at_identity_is_face_count() {
        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_iii_3_id".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init must succeed");
        gauge_registry::register(Arc::new(field));
        let handle = gauge_registry::get("U_iii_3_id").expect("registered");
        let inc = build_edge_face_incidence(&bb);

        for eid in 0..bb.n_edges() {
            let v = staple_sum_at_edge(handle.as_ref(), &bb, &inc, eid);
            match v {
                GroupElement::SU2 { q0, q1, q2, q3 } => {
                    // 2 incident faces × identity quaternion + qconj
                    // of identity is identity → (2, 0, 0, 0) byte-
                    // identical (no FP error from identity products).
                    assert_eq!(q0, 2.0, "edge {eid}: q0");
                    assert_eq!(q1, 0.0, "edge {eid}: q1");
                    assert_eq!(q2, 0.0, "edge {eid}: q2");
                    assert_eq!(q3, 0.0, "edge {eid}: q3");
                }
                _ => panic!("expected SU2"),
            }
        }
    }

    /// TDD-HAL-III.3: INIT HAAR_RANDOM SEED 20260616 → staple at
    /// edge 0 matches the Halcyon Python reference
    /// `_effective_staple_q(U, 0, graph)` quaternion. The golden
    /// constant below was harvested by feeding the same SEED-20260616
    /// quaternion buffer to Halcyon's reference impl. Tolerance
    /// 1e-12 covers the floating-point accumulation order difference
    /// between the Python `torch.zeros + accumulator` path and the
    /// Rust scalar-accumulator path (a handful of FMA-friendly
    /// reorderings; the algorithms are mathematically identical).
    #[test]
    fn tdd_hal_iii_3_staple_matches_halcyon_at_seed() {
        // Harvested once from `davis-wilson-lattice/inertia_damping/
        // buckyball_action.py::staple_sum_q(U, 0, graph)` against
        // the SEED-20260616 Haar buffer
        // (`tests/fixtures/halcyon/buckyball_haar_random_seed_20260616_gold.json`).
        const HALCYON_STAPLE_E0: [f64; 4] = [
            0.576614125169816,
            -0.9034170104474746,
            -1.1238436541744323,
            0.014849962921294355,
        ];

        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_iii_3_haar".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(20260616),
        )
        .expect("haar init must succeed");
        gauge_registry::register(Arc::new(field));
        let handle = gauge_registry::get("U_iii_3_haar").expect("registered");
        let inc = build_edge_face_incidence(&bb);

        let v = staple_sum_at_edge(handle.as_ref(), &bb, &inc, 0);
        match v {
            GroupElement::SU2 { q0, q1, q2, q3 } => {
                let tol = 1e-12;
                assert!(
                    (q0 - HALCYON_STAPLE_E0[0]).abs() < tol,
                    "q0: got {q0}, want {}, diff {}",
                    HALCYON_STAPLE_E0[0],
                    (q0 - HALCYON_STAPLE_E0[0]).abs()
                );
                assert!(
                    (q1 - HALCYON_STAPLE_E0[1]).abs() < tol,
                    "q1: got {q1}, want {}, diff {}",
                    HALCYON_STAPLE_E0[1],
                    (q1 - HALCYON_STAPLE_E0[1]).abs()
                );
                assert!(
                    (q2 - HALCYON_STAPLE_E0[2]).abs() < tol,
                    "q2: got {q2}, want {}, diff {}",
                    HALCYON_STAPLE_E0[2],
                    (q2 - HALCYON_STAPLE_E0[2]).abs()
                );
                assert!(
                    (q3 - HALCYON_STAPLE_E0[3]).abs() < tol,
                    "q3: got {q3}, want {}, diff {}",
                    HALCYON_STAPLE_E0[3],
                    (q3 - HALCYON_STAPLE_E0[3]).abs()
                );
            }
            _ => panic!("expected SU2"),
        }
    }

    /// TDD-HAL-III.3: planted-edge guard — set a non-identity element
    /// on the target edge, identity elsewhere; the staple at the
    /// target edge must be (2, 0, 0, 0) because the staple sum is
    /// the product of every OTHER link in each incident face, and
    /// "every other link" is identity. Confirms the walker skips the
    /// self edge (a pre-bug version that included the self edge would
    /// pick up the planted half-turn and return a non-identity sum).
    #[test]
    fn tdd_hal_iii_3_staple_skips_self_edge() {
        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let mut field = SU2GaugeField::new(
            "U_iii_3_skip".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        // Half-turn about z on edge 0. Every other edge stays identity.
        let e_target: EdgeId = 0;
        field.buffer.data[4 * e_target] = 0.0;
        field.buffer.data[4 * e_target + 1] = 0.0;
        field.buffer.data[4 * e_target + 2] = 0.0;
        field.buffer.data[4 * e_target + 3] = 1.0;
        gauge_registry::register(Arc::new(field));
        let handle = gauge_registry::get("U_iii_3_skip").expect("registered");
        let inc = build_edge_face_incidence(&bb);

        let v = staple_sum_at_edge(handle.as_ref(), &bb, &inc, e_target);
        match v {
            GroupElement::SU2 { q0, q1, q2, q3 } => {
                assert_eq!(q0, 2.0, "self-edge skip: q0 must be 2.0 (sum of 2 identities), got {q0}");
                assert_eq!(q1, 0.0, "self-edge skip: q1 must be 0.0, got {q1}");
                assert_eq!(q2, 0.0, "self-edge skip: q2 must be 0.0, got {q2}");
                assert_eq!(q3, 0.0, "self-edge skip: q3 must be 0.0, got {q3}");
            }
            _ => panic!("expected SU2"),
        }
    }
}
