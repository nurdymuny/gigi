//! Observable identifiers + reduction tags consumed by the GQL parser
//! at TDD-HAL-III.6.
//!
//! Two enum surfaces:
//!
//!   - [`PlaquetteReduction`] — the per-face / mean / sum tag the
//!     parser emits when it desugars `PLAQUETTE OF U` inside SELECT
//!     projections (locked decision D7). The executor reads this tag
//!     to dispatch to `plaquette_per_face`, `plaquette_mean`, or
//!     `plaquette_sum`.
//!   - [`ObservableId`] — re-export of the same enum the III.5
//!     `gibbs_sample` API consumes for `MEASURE (…)` clauses. The
//!     parser emits a `Vec<ObservableId>` straight through into the
//!     executor's `gauge::gibbs_sample` call.
//!
//! Group-erasure note (Bee's locked decision D4): the GQL grammar
//! ships group-agnostic enum tags. The executor performs the
//! SU(2)-specific dispatch via `handle.group()` matching when it
//! lowers each tag into the concrete primitive call.

pub use super::gibbs_sample::ObservableId;

/// Reduction applied to the per-face plaquette column produced by
/// `gauge::plaquette_per_face`. Mirrors the three SELECT shapes the
/// parser at TDD-HAL-III.6 desugars:
///
/// | GQL form                       | `PlaquetteReduction` |
/// |--------------------------------|----------------------|
/// | `SELECT PLAQUETTE OF U;`       | `PerFace`            |
/// | `SELECT MEAN(PLAQUETTE OF U);` | `Mean`               |
/// | `SELECT SUM(PLAQUETTE OF U);`  | `Sum`                |
///
/// `PerFace` returns a `Vec<f64>` of length `F` (the q0 column only,
/// locked decision D7); `Mean` and `Sum` return scalar `f64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaquetteReduction {
    /// Per-face column: `Vec<f64>` of length `lat.n_faces()` in face
    /// index order. Executor dispatches to
    /// `gauge::plaquette_per_face`.
    PerFace,
    /// Mean: `(1/F) · Σ_f q0_f`. Scalar `f64`. Executor dispatches to
    /// `gauge::plaquette_mean`.
    Mean,
    /// Sum: `Σ_f q0_f`. Scalar `f64`. Executor dispatches to
    /// `gauge::plaquette_sum`.
    Sum,
}

impl PlaquetteReduction {
    /// Stable label used in the JSON wire envelope (`reduction` field).
    pub fn label(&self) -> &'static str {
        match self {
            PlaquetteReduction::PerFace => "per_face",
            PlaquetteReduction::Mean => "mean",
            PlaquetteReduction::Sum => "sum",
        }
    }
}

/// Reduction tag for `SELECT GAUSS_RESIDUAL_MAX OF (U, E)` (TDD-HAL-IV.7).
///
/// `Covariant` (default) dispatches to
/// `gauge::compute_gauss_residual_covariant`; `Flat` dispatches to
/// `gauge::compute_gauss_residual_flat`. The wire envelope carries the
/// `reduction` label so JSON consumers can disambiguate without
/// re-reading the GQL source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GaussReduction {
    /// Covariant divergence `∂_e (U_e · E_e · U_e^†)` per vertex
    /// (Halcyon production-canonical). Default for SELECT
    /// GAUSS_RESIDUAL_MAX.
    Covariant,
    /// Flat divergence `∂_e E_e` per vertex — the no-parallel-transport
    /// debug shape. Strictly P1 today; lands here so the parser surface
    /// is stable when the executor exposes it.
    Flat,
}

impl GaussReduction {
    /// Stable label used in the JSON wire envelope (`reduction` field).
    pub fn label(&self) -> &'static str {
        match self {
            GaussReduction::Covariant => "covariant",
            GaussReduction::Flat => "flat",
        }
    }
}
