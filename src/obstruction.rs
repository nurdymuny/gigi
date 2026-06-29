//! OBSTRUCTION verb — Phase 1 (RED stub).
//!
//! Decides whether a principal G-bundle on a closed base manifold
//! admits a global section. Leading obstruction is the integrated
//! characteristic class (`c_2` for SU(N) on 4D bases, `c_1` for U(1)
//! on 2D bases). On 2D bases with SU(N) the answer is vacuously
//! trivial (every SU(N>=2) bundle on a closed surface is trivial).
//!
//! Phase 1 wires this as a thin policy layer over the Chern-Weil
//! kernel (sibling module `crate::chern_weil`, also Phase 1) — this
//! module never recomputes plaquettes.
//!
//! RED status (2026-06-29): the public API surface is declared
//! (types + signatures) but every function body is `unimplemented!()`.
//! Tests at `tests/obstruction_basic.rs` compile against this stub
//! and FAIL at runtime when they call into the unimplemented body.
//! GREEN commit fills in the bodies after CHERN_CLASS ships.

#![allow(dead_code)]

use crate::engine::Engine;

/// Result of an OBSTRUCTION test for a single bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct ObstructionResult {
    /// True iff the bundle does NOT admit a global section
    /// (equivalently: the integrated characteristic class is a
    /// non-zero integer).
    pub has_obstruction: bool,

    /// Raw real-valued integral of the characteristic class density
    /// BEFORE rounding to an integer sector. Useful as a quality
    /// diagnostic: `|witness - class|` should be small (<= 0.25 by
    /// default) on a sufficiently cooled lattice configuration.
    pub witness: f64,

    /// Integer topological sector. For SU(N) on 4D bases this is the
    /// instanton number Q. For U(1) on closed surfaces this is the
    /// monopole / first-Chern integer. Always 0 in the
    /// "no obstruction" case.
    pub class: i64,

    /// Human-readable label naming WHICH obstruction was tested:
    ///   - `"principal_bundle_section_obstruction"` (SU(N), 4D, default)
    ///   - `"instanton_number"`                      (SU(N), 4D, kind override)
    ///   - `"u1_section_obstruction"` / `"u1_monopole_charge"` (U(1), 2D)
    ///   - `"trivial_2d_su_n"`                       (SU(N), 2D, vacuous)
    ///   - `"<base>_non_integral_witness"`           (lattice not cooled)
    pub kind: String,
}

/// Which obstruction interpretation to label the integer sector with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObstructionKind {
    /// Default: does a global section of the principal bundle exist?
    SectionExistence,
    /// Same integer, but labelled as `"instanton_number"` in the
    /// `kind` field.
    InstantonNumber,
}

/// Typed errors returned by [`obstruction`] and
/// [`obstruction_with_default`].
#[derive(Debug)]
pub enum ObstructionError {
    /// The named bundle does not exist on the engine.
    BundleNotFound(String),
    /// OBSTRUCTION is not defined for the (group, base_dim) pair.
    /// Examples: ZN on 4D (deferred to Phase 2), SU(N) on D=3 (no
    /// canonical c_k class for an odd-dimensional base), etc.
    UnsupportedObstruction { group: String, base_dim: usize },
    /// The bundle exists but is not associated with a registered
    /// lattice so the base dimension cannot be inferred.
    LatticeMissing(String),
    /// Generic underlying error from the chern-weil kernel (Phase 2
    /// will switch this to `#[from]` once `chern_weil::ChernWeilError`
    /// is in tree).
    ChernWeil(String),
}

impl std::fmt::Display for ObstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BundleNotFound(name) => {
                write!(f, "OBSTRUCTION: bundle '{name}' not found")
            }
            Self::UnsupportedObstruction { group, base_dim } => {
                write!(
                    f,
                    "OBSTRUCTION: not defined for group {group} on base of \
                     dimension {base_dim}"
                )
            }
            Self::LatticeMissing(name) => {
                write!(f, "OBSTRUCTION: bundle '{name}' has no registered lattice")
            }
            Self::ChernWeil(msg) => {
                write!(f, "OBSTRUCTION: chern-weil kernel error — {msg}")
            }
        }
    }
}

impl std::error::Error for ObstructionError {}

/// Phase 1 OBSTRUCTION entry point.
///
/// Resolves the bundle's group + base dimension, dispatches to the
/// appropriate Chern-Weil order, and rounds the integral to an
/// integer sector with `0.25`-tolerance quantization.
///
/// RED stub: body is `unimplemented!()`. Replaced in the GREEN commit
/// once `crate::chern_weil::chern_class` ships.
pub fn obstruction(
    _engine: &Engine,
    _bundle_name: &str,
    _kind: ObstructionKind,
) -> Result<ObstructionResult, ObstructionError> {
    unimplemented!(
        "OBSTRUCTION verb — Phase 1 stub (RED). \
         GREEN commit wires this through crate::chern_weil::chern_class."
    )
}

/// Default-kind convenience wrapper around [`obstruction`]: passes
/// `ObstructionKind::SectionExistence`.
pub fn obstruction_with_default(
    engine: &Engine,
    bundle_name: &str,
) -> Result<ObstructionResult, ObstructionError> {
    obstruction(engine, bundle_name, ObstructionKind::SectionExistence)
}
