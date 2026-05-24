//! L6 e2e gate (per IMPLEMENTATION_PLAN.md L6 §"e2e validation
//! gate"): build a bundle with known Morse structure, run
//! `compress_to_morse()`, verify the compressed representation
//! reconstructs the original cohomology to within ε; storage
//! reduction reported.
//!
//! ### Known-topology fixtures
//!
//! We use two complexes with closed-form Betti numbers:
//!
//! 1. **Tetrahedron = S²** — Betti `(1, 0, 1)`, χ = 2. Built from
//!    explicit edge/face lists matching the Python reference in
//!    `validation_tests_v3.py::test_11_hodge_torus`.
//!
//! 2. **T² 6×6 triangulated grid** — Betti `(1, 2, 1)`, χ = 0.
//!    Built from periodic neighbors + the NE-SW diagonal.
//!
//! For each fixture we assert:
//! - `betti(hc, 1e-8) == expected Betti`
//! - `morse_compress(hc).cohomology_preserved() == true`
//! - `compression_ratio > 1` (we actually shrunk something)
//! - Hodge ↔ Euler identity on Betti.
//!
//! This is the cross-team math gate for L6. If the Rust code
//! drifts from the textbook Betti values, this test fires.

#![cfg(feature = "kahler")]

use gigi::discrete::{betti, morse_compress, HodgeComplex};

fn t2_grid(n: usize) -> HodgeComplex {
    let nv = n * n;
    let v = |i: usize, j: usize| (i % n) * n + (j % n);
    let mut edge_set: std::collections::BTreeSet<(usize, usize)> =
        std::collections::BTreeSet::new();
    for i in 0..n {
        for j in 0..n {
            let a = v(i, j);
            let b = v(i + 1, j);
            edge_set.insert((a.min(b), a.max(b)));
            let c = v(i, j + 1);
            edge_set.insert((a.min(c), a.max(c)));
            let d = v(i + 1, j + 1);
            edge_set.insert((a.min(d), a.max(d)));
        }
    }
    let edges: Vec<(usize, usize)> = edge_set.into_iter().collect();
    let mut face_set: std::collections::BTreeSet<(usize, usize, usize)> =
        std::collections::BTreeSet::new();
    for i in 0..n {
        for j in 0..n {
            let mut t1 = [v(i, j), v(i + 1, j), v(i + 1, j + 1)];
            let mut t2 = [v(i, j), v(i + 1, j + 1), v(i, j + 1)];
            t1.sort();
            t2.sort();
            face_set.insert((t1[0], t1[1], t1[2]));
            face_set.insert((t2[0], t2[1], t2[2]));
        }
    }
    let faces: Vec<(usize, usize, usize)> = face_set.into_iter().collect();
    HodgeComplex::new(nv, edges, faces).expect("T²")
}

fn tetrahedron() -> HodgeComplex {
    let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let faces = vec![(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
    HodgeComplex::new(4, edges, faces).expect("tet")
}

#[test]
fn tetrahedron_compresses_to_betti_1_0_1() {
    let hc = tetrahedron();
    let m = morse_compress(&hc);

    // S² Betti = (1, 0, 1).
    assert_eq!(m.betti.b0, 1);
    assert_eq!(m.betti.b1, 0);
    assert_eq!(m.betti.b2, 1);

    // Cohomology preservation = critical-cell counts match Betti.
    assert!(m.cohomology_preserved());

    // Compression: V+E+F = 14, critical = 2 ⇒ 7× compression.
    assert_eq!(m.n_original(), 14);
    assert_eq!(m.n_critical(), 2);
    assert!((m.compression_ratio() - 7.0).abs() < 1e-12);

    // Hodge ↔ Euler.
    assert_eq!(m.betti.euler_characteristic(), 2);
    assert_eq!(
        m.betti.euler_characteristic(),
        m.original_v as i64 - m.original_e as i64 + m.original_f as i64
    );
}

#[test]
fn t2_grid_compresses_to_betti_1_2_1() {
    let hc = t2_grid(6);
    let m = morse_compress(&hc);

    // T² Betti = (1, 2, 1).
    assert_eq!(m.betti.b0, 1);
    assert_eq!(m.betti.b1, 2);
    assert_eq!(m.betti.b2, 1);

    // Cohomology preservation.
    assert!(m.cohomology_preserved());

    // Total critical = 4; V+E+F much larger.
    assert_eq!(m.n_critical(), 4);
    assert!(
        m.compression_ratio() > 10.0,
        "T² compression should be > 10×; got {}",
        m.compression_ratio()
    );

    // Hodge ↔ Euler. T² has χ = 0.
    assert_eq!(m.betti.euler_characteristic(), 0);
    assert_eq!(
        m.betti.euler_characteristic(),
        m.original_v as i64 - m.original_e as i64 + m.original_f as i64
    );

    // Diagnostic: report storage reduction.
    println!(
        "L6 e2e T² 6×6: V={}, E={}, F={} ⇒ critical=(1, 2, 1), \
         compression = {:.1}×",
        m.original_v,
        m.original_e,
        m.original_f,
        m.compression_ratio()
    );
}

#[test]
fn t2_grid_betti_matches_python_reference_independently() {
    // Same Betti computation as the Python ground truth
    // (validation_tests_v3.py::test_11_hodge_torus). The Python
    // test uses square cells; our Rust uses triangulated cells.
    // Both are valid CW-complexes for T²; Betti is a topological
    // invariant. Independent agreement here ⇒ the formalism is
    // correct.
    let hc = t2_grid(6);
    let b = betti(&hc, 1e-8);
    assert_eq!(
        (b.b0, b.b1, b.b2),
        (1, 2, 1),
        "T² Betti must match Python ref (1, 2, 1); got ({}, {}, {})",
        b.b0,
        b.b1,
        b.b2
    );
}

#[test]
fn compressed_size_is_smaller_than_original() {
    // Negative-style invariant: the compressed cell count must be
    // strictly smaller than the original on non-trivial complexes.
    // Marcella consumes the compression-ratio as the routing
    // benefit; this asserts the benefit is real.
    let hc = t2_grid(6);
    let m = morse_compress(&hc);
    assert!(
        m.n_critical() < m.n_original(),
        "Morse compression must shrink: critical={}, original={}",
        m.n_critical(),
        m.n_original()
    );
}
