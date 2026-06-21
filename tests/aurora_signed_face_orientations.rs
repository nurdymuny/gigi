//! AURORA Phase 0 — CC-4 signed-face promotion.
//!
//! RED-first test for `Lattice::signed_face_orientations()`. This is
//! the public-API lift of the signed-face-cycle traversal logic that
//! currently lives inside the buckyball constructor
//! (`src/lattice/topology/truncated_icosahedron.rs::buckyball_with_signed_faces`).
//! AURORA's Phase-1 CUBED_SPHERE constructor needs the same surface
//! so it does not duplicate face-cycle-table machinery; promoting
//! the computation onto `Lattice` gives every topology constructor
//! a single shared call site.
//!
//! Commitment receipt: theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md
//! § CC-4 + § 5 (engine GREENLIT, bit-identity risk = 0); commit
//! ad306ec. Phase 0 commits to landing the promotion first as a
//! standalone PR (~80–100 LOC).
//!
//! Load-bearing contracts asserted here:
//!
//!   1. **Shape contract**: the public method returns a `Vec<Vec<(EdgeId,
//!      EdgeOrientation)>>` of outer length `n_faces()`, with each
//!      inner cycle the same length as the corresponding
//!      `lattice.faces[fidx]` vertex cycle. For the canonical
//!      buckyball that means outer-length 32, with 12 pentagons
//!      (len 5) emitted first, then 20 hexagons (len 6) — the order
//!      pinned by `buckyball_with_signed_faces`.
//!
//!   2. **Walker-equivalence contract**: feeding each per-face
//!      `(edge_id, orientation)` list into `walk_loop` against an
//!      identity connection must return the SU(2) identity quaternion
//!      bit-identically (no trig, pure multiply-add on `(1,0,0,0)`).
//!      This is the consumer-side bit-identity proof: the gauge
//!      walker that already passes TDD-HAL-I.4 on the buckyball must
//!      accept the new method's output as a drop-in for `face_edges()`.
//!
//!   3. **Anti-drift anchor**: face 0 (the first pentagon) must
//!      match the existing `buckyball_with_signed_faces().signed_faces[0]`
//!      cycle byte-for-byte after the `sign → EdgeOrientation`
//!      translation. If a later refactor accidentally permutes the
//!      face-storage order, this test catches it before any Halcyon
//!      gold gate has a chance to drift.
//!
//! Per the AURORA constraint sheet: no tunable tolerances (orientation
//! signs are integer ±1), no rewriting of the face-cycle machinery
//! (the promotion is a lift), and bit-identity for every existing
//! Halcyon contract that flows through face cycles (IV.10 / III.8b /
//! V.*) is preserved because `to_gql` round-trip stores no precomputed
//! signed-face table on `Lattice`.

#![cfg(feature = "halcyon")]

use gigi::gauge::edge_connection::EdgeConnection;
use gigi::gauge::group_element::GroupElement;
use gigi::gauge::holonomy::walk_loop;
use gigi::lattice::topology::truncated_icosahedron::{
    buckyball, buckyball_with_signed_faces, signed_face_to_walker,
};
use gigi::lattice::{EdgeId, EdgeOrientation};

/// Identity-everywhere connection: every edge reads back as the SU(2)
/// identity quaternion regardless of orientation. Mirrors the
/// `FixedEdgeConnection::identity_everywhere` test helper that lives
/// `#[cfg(test)] pub(crate)` inside the crate; re-stated here because
/// integration tests do not see crate-private test scaffolding.
struct IdentityConnection;

impl EdgeConnection for IdentityConnection {
    fn edge_element(&self, _edge: EdgeId, _orientation: EdgeOrientation) -> GroupElement {
        // Identity is self-inverse; orientation does not matter.
        GroupElement::su2_identity()
    }
}

/// AURORA-CC-4.a — shape contract.
///
/// The promoted method must return:
///   - outer Vec of length `n_faces()` (32 on the buckyball)
///   - inner cycles in `faces` order: 12 pentagons (len 5) first,
///     then 20 hexagons (len 6) — matches the pin in
///     `buckyball_with_signed_faces` lines 315–321.
#[test]
fn test_signed_face_orientations_buckyball_canonical_face_count() {
    let lat = buckyball();
    let sfo = lat.signed_face_orientations();

    assert_eq!(
        sfo.len(),
        32,
        "buckyball must expose 32 signed faces (12 pentagons + 20 hexagons)"
    );
    assert_eq!(
        sfo.len(),
        lat.n_faces(),
        "outer length must equal Lattice::n_faces()"
    );

    for (fidx, cycle) in sfo.iter().enumerate() {
        let declared_len = lat.faces[fidx].len();
        assert_eq!(
            cycle.len(),
            declared_len,
            "face {fidx}: signed cycle len {} != declared vertex-cycle len {}",
            cycle.len(),
            declared_len
        );
    }

    // Pentagons first, then hexagons — the buckyball emit-order pin.
    for fidx in 0..12 {
        assert_eq!(
            sfo[fidx].len(),
            5,
            "face {fidx} expected pentagon (len 5), got len {}",
            sfo[fidx].len()
        );
    }
    for fidx in 12..32 {
        assert_eq!(
            sfo[fidx].len(),
            6,
            "face {fidx} expected hexagon (len 6), got len {}",
            sfo[fidx].len()
        );
    }
}

/// AURORA-CC-4.b — walker-equivalence (bit-identity) contract.
///
/// For every face, feeding the new method's `(edge_id, orientation)`
/// list into `walk_loop` against an identity connection must return
/// the SU(2) identity quaternion exactly. This is the consumer-side
/// proof that the lift is a true drop-in for `face_edges()`: the
/// gauge walker that already passes TDD-HAL-I.4 on the buckyball
/// accepts the promoted surface without observable drift.
#[test]
fn test_signed_face_orientations_consistency_with_walk_loop() {
    let lat = buckyball();
    let sfo = lat.signed_face_orientations();
    let conn = IdentityConnection;

    let identity = GroupElement::su2_identity();

    for (fidx, cycle) in sfo.iter().enumerate() {
        let result = walk_loop(&lat, cycle, &conn);
        assert_eq!(
            result, identity,
            "face {fidx}: walk_loop on identity connection must return SU(2) \
             identity bit-identically (got {result:?})"
        );
    }
}

/// AURORA-CC-4.c — anti-drift anchor.
///
/// Face 0 must match the existing buckyball signed-face table
/// byte-for-byte after the canonical `sign → EdgeOrientation`
/// translation (`+1 → Forward`, `-1 → Reverse`). A second anchor on
/// face 12 (the first hexagon) tests the boundary between the
/// pentagon block and the hexagon block, catching any off-by-one in
/// face storage if a later refactor accidentally permutes things.
///
/// Reference: `buckyball_with_signed_faces().signed_faces[..]` is the
/// internal-but-vetted source the Halcyon gold gate already trusts;
/// the lift must agree with it on every face, including these two
/// anchors.
#[test]
fn test_signed_face_orientations_buckyball_anchor() {
    let bb = buckyball_with_signed_faces();
    let sfo = bb.lattice.signed_face_orientations();

    // Face 0 (first pentagon).
    let expected_f0 = signed_face_to_walker(&bb.signed_faces[0]);
    assert_eq!(
        sfo[0].len(),
        5,
        "face 0 expected pentagon length 5, got {}",
        sfo[0].len()
    );
    assert_eq!(
        sfo[0], expected_f0,
        "face 0 signed cycle must match buckyball_with_signed_faces().signed_faces[0] \
         after sign->orientation translation"
    );

    // Face 12 (first hexagon — boundary between the pentagon block
    // and the hexagon block).
    let expected_f12 = signed_face_to_walker(&bb.signed_faces[12]);
    assert_eq!(
        sfo[12].len(),
        6,
        "face 12 expected hexagon length 6, got {}",
        sfo[12].len()
    );
    assert_eq!(
        sfo[12], expected_f12,
        "face 12 signed cycle must match buckyball_with_signed_faces().signed_faces[12] \
         after sign->orientation translation"
    );

    // Full-table agreement: every face matches its
    // `signed_face_to_walker`-translated counterpart. The two named
    // anchors above stay as load-bearing receipts when this loop
    // grows expensive to debug.
    for fidx in 0..bb.lattice.n_faces() {
        let expected = signed_face_to_walker(&bb.signed_faces[fidx]);
        assert_eq!(
            sfo[fidx], expected,
            "face {fidx}: promoted signed_face_orientations() drifted from \
             buckyball_with_signed_faces().signed_faces[{fidx}]"
        );
    }
}
