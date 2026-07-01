//! RED-first failing tests for higher Betti numbers (β_k for k ≥ 2)
//! and the π_1 fundamental-group presentation on lattice cell complexes.
//!
//! These tests live in the CHERN/BETTI/PI_1/OBSTRUCTION topology-verb
//! cluster. They are intentionally FAILING at commit time: the target
//! module `gigi::topology` (with `betti_topological` and
//! `pi_1_presentation`) does NOT exist yet. The corresponding GREEN
//! commit will create `src/topology.rs` (or `src/topology/{homology,pi1}.rs`)
//! and make these pass.
//!
//! Test fixtures use already-shipped constructors:
//! - buckyball (truncated_icosahedron): S² with V=60, E=90, F=32, χ=2
//! - 2D cubic L=4 (flat torus T²): V=16, E=32, F=16, χ=0
//! - 4D cubic L=4 (4-torus T⁴): V=256, E=1024, F=1536
//!
//! Expected topological invariants:
//! - S² (buckyball):  β_0=1, β_1=0, β_2=1; π_1 trivial (rank 0)
//! - T² (cubic L=4 D=2):  β_0=1, β_1=2, β_2=1; π_1 = ℤ² (rank 2)
//! - T⁴ (cubic L=4 D=4):  β_0=1, β_1=4, ..., β_4=1; π_1 = ℤ⁴ (rank 4)
//!
//! Consistency invariant: Σ_k (-1)^k β_k == χ(lattice) for closed manifolds.
//!
//! Run with:
//!   `cargo test --features lattice --test betti_pi1_basic`
//!
//! Expected before GREEN commit: all 8 tests FAIL (the `gigi::topology`
//! module does not exist; the test file even fails to compile until
//! the GREEN commit lands the module).

#![cfg(feature = "lattice")]

use gigi::lattice::topology::cubic::cubic;
use gigi::lattice::topology::truncated_icosahedron::buckyball;

// The target API — does NOT exist yet at HEAD = 35a727d. The GREEN
// commit will create `src/topology.rs` exposing these symbols.
use gigi::topology::{betti_topological, pi_1_presentation, Pi1Presentation};

// ── Higher Betti tests ──────────────────────────────────────────────

/// β_2(buckyball) = 1 (the buckyball is a triangulation of S²).
/// Expected: Σ (-1)^k β_k = 1 - 0 + 1 = 2 = χ(S²).
#[test]
fn test_betti_order_2_buckyball_returns_one() {
    let lat = buckyball();
    let b2 = betti_topological(&lat, 2)
        .expect("betti_topological order=2 on buckyball should succeed");
    assert_eq!(
        b2, 1,
        "β_2(buckyball / S²) must be 1, got {b2}. \
         Buckyball has V=60, E=90, F=32, χ=2; with β_0=1 and β_1=0, \
         consistency forces β_2 = 1."
    );
}

/// β_2(T²) = 1 on the 4×4 flat torus (CUBIC L=4 D=2).
#[test]
fn test_betti_order_2_flat_torus_returns_one() {
    let lwm = cubic("t2_4", 4, 2, true, None);
    let lat = lwm.lattice();
    let b2 = betti_topological(lat, 2)
        .expect("betti_topological order=2 on T² should succeed");
    assert_eq!(
        b2, 1,
        "β_2(T² L=4) must be 1, got {b2}. \
         T² has V=16, E=32, F=16, χ=0; with β_0=1 and β_1=2, \
         consistency forces β_2 = 1."
    );
}

/// β_2(T⁴) = 6 on the 4×4×4×4 4-torus (CUBIC L=4 D=4).
/// T^D Betti pattern: β_k = C(D, k), so β_2(T⁴) = C(4,2) = 6.
#[test]
fn test_betti_order_2_4d_cubic_l4_returns_six() {
    let lwm = cubic("c4_4", 4, 4, true, None);
    let lat = lwm.lattice();
    let b2 = betti_topological(lat, 2)
        .expect("betti_topological order=2 on T⁴ L=4 should succeed");
    assert_eq!(
        b2, 6,
        "β_2(T⁴) must be C(4,2) = 6, got {b2}. \
         T⁴ Betti pattern: β_k = C(4,k) = (1, 4, 6, 4, 1) for k = 0..4."
    );
}

/// Σ_k (-1)^k β_k must equal χ(lattice) for both buckyball (χ=2)
/// and 4×4 flat torus (χ=0). On the buckyball only k ∈ {0,1,2}
/// contribute (β_k = 0 for k > 2). Same for T² L=4.
#[test]
fn test_betti_euler_characteristic_consistency() {
    // Buckyball: S², χ = 2 = β_0 - β_1 + β_2 = 1 - 0 + 1.
    let buc = buckyball();
    let b0_buc = betti_topological(&buc, 0)
        .expect("β_0(buckyball) should succeed");
    let b1_buc = betti_topological(&buc, 1)
        .expect("β_1(buckyball) should succeed");
    let b2_buc = betti_topological(&buc, 2)
        .expect("β_2(buckyball) should succeed");
    let chi_buc = buc.euler_characteristic();
    let alt_sum_buc =
        b0_buc as i64 - b1_buc as i64 + b2_buc as i64;
    assert_eq!(
        alt_sum_buc, chi_buc,
        "Σ (-1)^k β_k must equal χ on the buckyball: got (β_0, β_1, β_2) = ({b0_buc}, {b1_buc}, {b2_buc}), \
         alt-sum {alt_sum_buc}, χ {chi_buc}."
    );

    // T² L=4: χ = 0 = 1 - 2 + 1.
    let lwm = cubic("t2_4", 4, 2, true, None);
    let t2 = lwm.lattice();
    let b0_t2 = betti_topological(t2, 0)
        .expect("β_0(T²) should succeed");
    let b1_t2 = betti_topological(t2, 1)
        .expect("β_1(T²) should succeed");
    let b2_t2 = betti_topological(t2, 2)
        .expect("β_2(T²) should succeed");
    let chi_t2 = t2.euler_characteristic();
    let alt_sum_t2 = b0_t2 as i64 - b1_t2 as i64 + b2_t2 as i64;
    assert_eq!(
        alt_sum_t2, chi_t2,
        "Σ (-1)^k β_k must equal χ on T² L=4: got (β_0, β_1, β_2) = ({b0_t2}, {b1_t2}, {b2_t2}), \
         alt-sum {alt_sum_t2}, χ {chi_t2}."
    );
}

// ── π_1 fundamental-group tests ─────────────────────────────────────

/// π_1(buckyball / S²) is trivial (rank 0). All face relators must
/// kill all the non-tree generators when abelianized.
#[test]
fn test_pi_1_buckyball_is_trivial() {
    let lat = buckyball();
    let pres: Pi1Presentation = pi_1_presentation(&lat);
    assert_eq!(
        pres.rank, 0,
        "π_1(buckyball / S²) must have rank 0 (trivial group), got {}",
        pres.rank
    );
    assert_eq!(
        pres.abelianized_rank, 0,
        "abelianized π_1(buckyball) must have rank 0 (= β_1 = 0), got {}",
        pres.abelianized_rank
    );
}

/// π_1(T² L=4) is ℤ² (rank 2). The 4×4 flat torus has β_1 = 2.
#[test]
fn test_pi_1_flat_torus_2d_l4_is_z2() {
    let lwm = cubic("t2_4", 4, 2, true, None);
    let lat = lwm.lattice();
    let pres: Pi1Presentation = pi_1_presentation(lat);
    assert_eq!(
        pres.abelianized_rank, 2,
        "abelianized π_1(T² L=4) must have rank 2 (= ℤ²), got {}",
        pres.abelianized_rank
    );
    // Phase 1 reports abelianized rank as `rank`.
    assert_eq!(
        pres.rank, 2,
        "Phase 1 π_1(T²) rank must equal abelianized rank 2, got {}",
        pres.rank
    );
}

/// π_1(T⁴ L=4) is ℤ⁴ (rank 4). The 4D 4-torus has β_1 = 4.
#[test]
fn test_pi_1_4d_cubic_l4_is_z4() {
    let lwm = cubic("c4_4", 4, 4, true, None);
    let lat = lwm.lattice();
    let pres: Pi1Presentation = pi_1_presentation(lat);
    assert_eq!(
        pres.abelianized_rank, 4,
        "abelianized π_1(T⁴ L=4) must have rank 4 (= ℤ⁴), got {}",
        pres.abelianized_rank
    );
    assert_eq!(
        pres.rank, 4,
        "Phase 1 π_1(T⁴) rank must equal abelianized rank 4, got {}",
        pres.rank
    );
}

/// Massey presentation invariant: each face contributes one relator,
/// so `len(relators) == n_faces(lattice)` for any closed cell complex.
/// We verify this on all three fixtures (buckyball / T² / T⁴).
#[test]
fn test_pi_1_presentation_has_face_count_relators() {
    // Buckyball: F = 32 faces → 32 relators.
    let buc = buckyball();
    let buc_pres = pi_1_presentation(&buc);
    assert_eq!(
        buc_pres.relators.len(),
        buc.n_faces(),
        "buckyball: π_1 presentation must have one relator per face, \
         got {} relators for {} faces",
        buc_pres.relators.len(),
        buc.n_faces()
    );

    // T² L=4: F = 16 faces → 16 relators.
    let t2_lwm = cubic("t2_4", 4, 2, true, None);
    let t2 = t2_lwm.lattice();
    let t2_pres = pi_1_presentation(t2);
    assert_eq!(
        t2_pres.relators.len(),
        t2.n_faces(),
        "T² L=4: π_1 presentation must have one relator per face, \
         got {} relators for {} faces",
        t2_pres.relators.len(),
        t2.n_faces()
    );

    // T⁴ L=4: F = 4⁴ · C(4,2) = 256 · 6 = 1536 faces → 1536 relators.
    let t4_lwm = cubic("c4_4", 4, 4, true, None);
    let t4 = t4_lwm.lattice();
    let t4_pres = pi_1_presentation(t4);
    assert_eq!(
        t4_pres.relators.len(),
        t4.n_faces(),
        "T⁴ L=4: π_1 presentation must have one relator per face, \
         got {} relators for {} faces",
        t4_pres.relators.len(),
        t4.n_faces()
    );
}
