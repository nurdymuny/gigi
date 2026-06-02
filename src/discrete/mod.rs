//! L6 — Discrete exterior calculus + Hodge complex (catalog §2.9).
//!
//! Builds the discrete chain complex `0 → C⁰ → C¹ → C² → 0` from
//! cell incidence data and exposes the Hodge Laplacians
//! `Δ_k = d† d + d d†` with Betti numbers via eigendecomposition.
//!
//! ### What "cell" means in a GIGI bundle
//!
//! Discrete exterior calculus needs a CW or simplicial structure.
//! For a `BundleStore` we use the natural construction:
//!
//! - **0-cells (vertices)** = records (one per base point).
//! - **1-cells (edges)** = pairs of records sharing a value on any
//!   indexed field — this is the same field-index graph the L3
//!   `spectral_gap` runs on.
//! - **2-cells (faces)** = triangles (3-cliques) in that graph.
//!
//! Per catalog §2.9 the d² = 0 identity is forced by the cell
//! orientation convention — the implementation of `d_1` walks each
//! triangle's three oriented edges and the signs cancel exactly.
//! The math validation (`validation_tests_v3.py::test_11_hodge_torus`)
//! checks this on the T² grid + S² tetrahedron — our Rust must
//! reproduce Betti `(1, 2, 1)` and `(1, 0, 1)` on identical inputs.
//!
//! ### Layering
//!
//! - `hodge_complex` — d_0, d_1 operators + d² = 0 invariant.
//! - `hodge_laplacian` — Δ_k construction + Betti via eigendecomp.
//! - `morse` — Morse compression preserving cohomology (L6.4–L6.5
//!   Marcella ask).
//!
//! Gated on the `kahler` feature so the no-feature build stays
//! bit-identical to pre-upgrade GIGI.

#![cfg(feature = "kahler")]

pub mod f2_rank;
pub mod hodge_complex;
pub mod hodge_laplacian;
pub mod morse;

pub use hodge_complex::{HodgeComplex, HodgeComplexError};
pub use hodge_laplacian::{betti, BettiNumbers};
pub use morse::{morse_compress, MorseComplex};
