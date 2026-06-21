//! Discrete Exterior Calculus (DEC) operators on [`LatticeWithMetric`].
//!
//! AURORA Phase 2 (Round 2 commitment, commit ad306ec, status board row
//! Q3). The module exposes the minimal set of DEC free functions
//! AURORA's ShallowWater force kernel needs:
//!
//! - [`d_0`] — exterior derivative `Form0 → Form1` (the `grad h` in
//!   AURORA's geopotential force law). Pure combinatorics; consumes
//!   only the lattice's edge incidence.
//! - [`delta_1`] — codifferential `Form1 → Form0` (the `div(hu)` mass-
//!   flux divergence). Requires `edge_lengths` + `dual_face_areas`.
//! - [`hodge_star_0`] / [`hodge_star_1`] / [`hodge_star_2`] — the
//!   primal-dual Hodge stars packaging the metric accessors as DEC
//!   operators.
//!
//! Status board note: the row is labeled `delta_0` but the correct
//! mathematical term for the codifferential going 1-form → 0-form on a
//! 2-manifold is `delta_1` (adjoint of `d_0`, takes 1-forms to
//! 0-forms). This module ships under the correct name.
//!
//! Conventions (locked, matched against
//! `src/discrete/hodge_complex.rs`):
//!
//! - For canonical edge `e = (tail, head)`:
//!   `(d_0 phi)[e] = phi[head] - phi[tail]`.
//! - Barycentric dual edge length:
//!   `l_e* = (A_{v-}* + A_{v+}*) / (2 * l_e)`. Computed inside the
//!   module — no new accessor on [`LatticeWithMetric`]. See
//!   [`codifferential`] doc-comment for the Phase 3 upgrade path.
//! - `Form0 / Form1 / Form2` are documented `Vec<f64>` (semantic
//!   labeling via doc-comments, not newtype wrappers — matches
//!   AURORA's consumption pattern).
//!
//! Phase 1/2 additivity: this module does NOT modify
//! [`LatticeWithMetric`] or `src/discrete/hodge_complex.rs`. Only the
//! single `pub mod dec;` line in `src/lattice/mod.rs` is the touch
//! outside this directory.

pub mod codifferential;
pub mod d;
pub mod hodge;

pub use codifferential::delta_1;
pub use d::d_0;
pub use hodge::{hodge_star_0, hodge_star_1, hodge_star_2};

use thiserror::Error;

/// Structured error returned by every DEC operator in this module.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
///
/// Each variant carries only `Copy` primitive payloads so the enum is
/// `Eq` for ergonomic `assert_eq!` comparisons in tests. The `surface`
/// label on `LengthMismatch` lets multi-step DEC pipelines (e.g.
/// `delta_1 ∘ d_0`) produce useful diagnostics without backtraces.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecError {
    /// An input form's length did not match the lattice cardinality the
    /// operator expects. `surface` is a `&'static str` label like
    /// `"d_0::phi"`, `"delta_1::u"`, `"hodge_star_2::omega"`.
    #[error("length mismatch on {surface}: expected {expected}, got {actual}")]
    LengthMismatch {
        expected: usize,
        actual: usize,
        surface: &'static str,
    },
    /// The operator needs `cell_areas` but the wrapper's vector is
    /// empty (Phase 1 zero-metric placeholder).
    #[error("cell_areas is empty — operator requires a metric")]
    CellAreasMissing,
    /// The operator needs `edge_lengths` but the wrapper's vector is
    /// empty.
    #[error("edge_lengths is empty — operator requires a metric")]
    EdgeLengthsMissing,
    /// The operator needs `dual_face_areas` but the wrapper holds
    /// `None` (constructor declined to commit to a dual mesh).
    #[error("dual_face_areas is None — operator requires a dual mesh")]
    DualFaceAreasMissing,
}
