//! AURORA Phase 2 — Hamiltonian action trait surface.
//!
//! Group-agnostic trait hierarchy that any Hamiltonian (in-tree
//! `KogutSusskind` or downstream `ShallowWater`) implements to plug
//! into gigi's symplectic integrator + registry + WAL.
//!
//! ── Why group-agnostic? ──
//!
//! AURORA's `ShallowWaterFactory` ships with `group_tag = "R"` (the
//! additive reals — a Hamiltonian group beside `SU(2)`). The trait
//! surface must therefore never reference `SU2GaugeField`/`SU2EField`
//! or any group-specific concrete type. State is exchanged as a
//! group-erased `&[f64]` / `Vec<f64>` buffer the concrete Hamiltonian
//! impl packs/unpacks on its own terms. The `kind_tag()` + `group_tag()`
//! pair on `HamiltonianFactory` is the rendezvous point between
//! registry name and downstream consumer.
//!
//! ── Object safety ──
//!
//! The registry stores `Box<dyn HamiltonianFactory>` and the factory's
//! `from_params` returns `Box<dyn HamiltonianHandle>` (single super-
//! trait collapsing all four sub-traits into one vtable per instance).
//! All trait methods speak in `&self`, `&[f64]`, `&mut [f64]`,
//! `Vec<f64>`, `f64`, `BTreeMap<String, f64>` — no associated types
//! leak through, no lifetime generics on the trait, no `Self` in
//! method return positions. The traits ARE object-safe by construction.
//!
//! ── Hot-path constraint ──
//!
//! Per v2 reply §11: "Trait-object dispatch lives off the integrator
//! inner loop." The registry pays one trait-object hop at factory-call
//! time; the per-substep KDK body is generic over a concrete
//! `H: HamiltonianForce + HamiltonianDrift`, not boxed. This module
//! ships the trait surface; the integrator-generic-over-H lift is a
//! separate workflow (SYMPLECTIC_FLOW stays as-is with its hardcoded
//! SU(2) Kogut-Susskind path under the IV.10 / VI bit-identity gates).
//!
//! ── Stability ──
//!
//! Every pub item in this module carries the EVOLVING marker per
//! `docs/STABILITY_GUARANTEES.md`. AURORA pins gigi by commit hash
//! and gets a stable contract surface until gigi 0.1.0 graduates.

use std::collections::{BTreeMap, HashMap};

// ─────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────

/// Factory-construction error surface.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactoryError {
    /// A required parameter was absent from the `&HashMap<String, f64>`.
    /// Never a silent default — the substrate refuses to guess.
    MissingParam { name: &'static str },
    /// A parameter was present but out of admissible range / shape.
    InvalidParam { name: &'static str, reason: String },
    /// The factory does not implement support for the requested
    /// `group_tag` (e.g. an `SU(2)` factory called against `R`).
    UnsupportedGroup { group_tag: &'static str },
}

impl std::fmt::Display for FactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FactoryError::MissingParam { name } => {
                write!(f, "hamiltonian factory: missing required param '{name}'")
            }
            FactoryError::InvalidParam { name, reason } => {
                write!(f, "hamiltonian factory: invalid param '{name}': {reason}")
            }
            FactoryError::UnsupportedGroup { group_tag } => write!(
                f,
                "hamiltonian factory: unsupported group_tag '{group_tag}'"
            ),
        }
    }
}

impl std::error::Error for FactoryError {}

/// Energy-evaluation error surface.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnergyError {
    /// The supplied state buffer's length / layout did not match what
    /// the Hamiltonian expects. `expected` is a static description
    /// (e.g. `"len = 2*n_cells + n_edges"`); `got` is the runtime
    /// description.
    StateShapeMismatch {
        expected: &'static str,
        got: String,
    },
    /// A numeric step in the energy decomposition failed (e.g.
    /// non-finite intermediate). Carries a free-form `reason`.
    NumericFailure { reason: String },
}

impl std::fmt::Display for EnergyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnergyError::StateShapeMismatch { expected, got } => write!(
                f,
                "energy decomposition: state shape mismatch (expected {expected}, got {got})"
            ),
            EnergyError::NumericFailure { reason } => {
                write!(f, "energy decomposition: numeric failure: {reason}")
            }
        }
    }
}

impl std::error::Error for EnergyError {}

/// Constraint-projection error surface.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    /// The projection solver did not converge within its iteration
    /// budget. No tunable tolerances exposed at the trait surface —
    /// each Hamiltonian impl owns its convergence criterion.
    SolverDiverged { iterations: u32 },
    /// The supplied state buffer's length / layout did not match what
    /// the projection expects.
    StateShapeMismatch {
        expected: &'static str,
        got: String,
    },
}

impl std::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectionError::SolverDiverged { iterations } => write!(
                f,
                "projection: solver diverged after {iterations} iterations"
            ),
            ProjectionError::StateShapeMismatch { expected, got } => write!(
                f,
                "projection: state shape mismatch (expected {expected}, got {got})"
            ),
        }
    }
}

impl std::error::Error for ProjectionError {}

// ─────────────────────────────────────────────────────────────────────
// Sub-traits
// ─────────────────────────────────────────────────────────────────────

/// Hamiltonian force: ∂H/∂q evaluated against a group-erased state
/// buffer, returning a per-edge (or per-cell) force vector with the
/// same flat layout the caller supplied.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub trait HamiltonianForce {
    /// Evaluate force at `state`. Returned `Vec<f64>` is conventionally
    /// the same layout as `state` (the integrator KDK kick step adds
    /// `force * dt` to the momentum slice of state).
    fn force(&self, state: &[f64]) -> Vec<f64>;
}

/// Hamiltonian drift: ∂H/∂p advance of the position slice of state by
/// a step `dt`, returning the new state.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub trait HamiltonianDrift {
    /// Advance state by `dt` along the drift vector field. Returned
    /// `Vec<f64>` has the same layout as `state`.
    fn drift(&self, state: &[f64], dt: f64) -> Vec<f64>;
}

/// Constraint projection: in-place projection of `state` back onto the
/// Hamiltonian's constraint surface (e.g. Gauss's law for SU(2)
/// Kogut-Susskind, or the height-positivity / divergence-free
/// constraints for ShallowWater).
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub trait ProjectionOperator {
    /// Project `state` onto the constraint surface in place. Returns
    /// `Ok(())` on convergence; `Err(ProjectionError)` on solver
    /// divergence or state-shape mismatch — never a silent no-op.
    fn project_constraint(&self, state: &mut [f64]) -> Result<(), ProjectionError>;
}

/// Energy decomposition: declarative `(name → value)` map for
/// diagnostic envelopes and WAL-adjacent receipts.
///
/// AURORA's ShallowWater publishes 7 keys (`casimir_energy`,
/// `casimir_mass`, `casimir_pv_l1`, `casimir_pv_l2`, `kelvin_eq`,
/// `kelvin_n30`, `kelvin_s30`). gigi's future `KogutSusskind` impl
/// publishes its own slice (`htotal`, `edge_kinetic`, `vertex_gauss`,
/// `mean_plaquette`, `q_surrogate`, …). Each Hamiltonian owns its
/// canonical key list.
///
/// `BTreeMap` (not `HashMap`) for deterministic key iteration — the
/// diagnostic envelopes that consume this surface must be reproducible
/// byte-for-byte across runs.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub trait EnergyDecomposition {
    /// Stable slice of energy-component names this Hamiltonian
    /// publishes. The order is the canonical reporting order for
    /// diagnostic envelopes; it is NOT necessarily lexicographic.
    fn energy_keys(&self) -> &'static [&'static str];

    /// Evaluate every energy component at `state` and return a
    /// deterministically-iterated map. Implementations should populate
    /// exactly the keys returned by `energy_keys()`.
    fn evaluate(&self, state: &[f64]) -> Result<BTreeMap<String, f64>, EnergyError>;
}

// ─────────────────────────────────────────────────────────────────────
// Super-trait + factory
// ─────────────────────────────────────────────────────────────────────

/// Composite trait every Hamiltonian implements — collapses the four
/// sub-traits into a single trait object so the registry can store
/// `Box<dyn HamiltonianHandle>` with one vtable per instance.
///
/// Implementors typically write:
///
/// ```ignore
/// impl HamiltonianForce for MyHam { ... }
/// impl HamiltonianDrift for MyHam { ... }
/// impl ProjectionOperator for MyHam { ... }
/// impl EnergyDecomposition for MyHam { ... }
/// impl HamiltonianHandle for MyHam {}
/// ```
///
/// `Send + Sync` so the registry can safely share handles across the
/// host binary's startup-then-read-only lifecycle.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub trait HamiltonianHandle:
    HamiltonianForce
    + HamiltonianDrift
    + ProjectionOperator
    + EnergyDecomposition
    + Send
    + Sync
    + std::fmt::Debug
{
}

/// Factory that constructs a concrete `HamiltonianHandle` from a
/// parameter map. Registered by name in `hamiltonian_registry` and
/// looked up at runtime.
///
/// AURORA's `ShallowWaterFactory` implements this trait with
/// `kind_tag = "SHALLOW_WATER"`, `group_tag = "R"`, and
/// `from_params(&HashMap<String, f64>)` reading `g`, `omega`, `a`.
///
/// `Send + Sync` so the registry can hand out factory references
/// across threads.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub trait HamiltonianFactory: Send + Sync {
    /// Canonical kind tag (e.g. `"SHALLOW_WATER"`, `"KOGUT_SUSSKIND"`).
    /// Echoed into the WAL `HamiltonianDeclare` event so replay /
    /// inspection tools can group registrations by physics kind.
    fn kind_tag(&self) -> &'static str;

    /// Canonical group tag (e.g. `"R"` for additive reals,
    /// `"SU2"` for SU(2)). Echoed into the WAL `HamiltonianDeclare`
    /// event so downstream consumers can refuse registrations whose
    /// group is incompatible.
    fn group_tag(&self) -> &'static str;

    /// Construct a fresh `HamiltonianHandle` from `params`. Errors
    /// loudly on missing/invalid params; never returns a silent
    /// default.
    fn from_params(
        &self,
        params: &HashMap<String, f64>,
    ) -> Result<Box<dyn HamiltonianHandle>, FactoryError>;
}
