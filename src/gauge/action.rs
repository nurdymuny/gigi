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
//! ── Separability gap (Phase 3) ──
//!
//! Per AURORA reply 4 §8: Stormer-Verlet KDK assumes a *separable*
//! Hamiltonian `H(q, p) = T(p) + V(q)`. AURORA's `ShallowWater` is
//! non-separable (vorticity flux couples height and momentum through
//! the same vector-invariant term), and the KDK split produces a
//! 7x WORSE Casimir drift than forward Euler on that physics — failure
//! by construction, not by truncation. The structure-preserving fix is
//! a Lie-Poisson bracket integrator that advances state along the
//! skew-symmetric bracket directly.
//!
//! This trait surface admits both separable (KDK) and non-separable
//! (Lie-Poisson) integration paths via the
//! `HamiltonianFactory::capabilities()` method. A factory declares
//! `HamiltonianCapabilities { force_drift, poisson_bracket }` and
//! `SYMPLECTIC_FLOW` dispatches: the `force_drift` path consumes
//! `HamiltonianForce + HamiltonianDrift`; the `poisson_bracket` path
//! consumes `HamiltonianPoissonBracket::bracket_step`. Existing
//! factories (`KogutSusskindFactory` et al.) inherit the default
//! `{ force_drift: true, poisson_bracket: false }` and continue
//! dispatching through KDK byte-identically.
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

/// Lie-Poisson bracket-step physics-invalidity error surface.
///
/// Emitted by `HamiltonianPoissonBracket::bracket_step` when the
/// proposed advance violates a physics precondition (negative depth,
/// CFL breach, etc.). Casimir drift is NOT a `BracketPhysicsError` —
/// that is the substrate's receipt responsibility (compare
/// `evaluate()` before/after `bracket_step` against `RECEIPT_TOL`).
///
/// AURORA reply 3 (2026-06-22) established that Stormer-Verlet KDK
/// fails BY CONSTRUCTION on non-separable Hamiltonians (shallow-water
/// produces 7x worse Casimir drift than forward Euler). The
/// Lie-Poisson integrator is the structure-preserving alternative;
/// this error enum is its physics-precondition surface.
///
/// Derives `PartialEq` but NOT `Eq` — two variants carry `f64`
/// fields. This is the only deviation from `FactoryError` /
/// `EnergyError` / `ProjectionError`, which all derive `Eq` because
/// their numeric fields are `u32` / `usize` / `&'static str` / `String`.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
#[derive(Debug, Clone, PartialEq)]
pub enum BracketPhysicsError {
    /// Cell depth went non-positive at grid index `(i, j)`, last
    /// observed value `h`. Shallow-water bracket step refuses to
    /// continue (would propagate NaN through `1/h` divisions).
    NegativeDepth { i: usize, j: usize, h: f64 },
    /// CFL stability breach — the proposed step's Courant number
    /// exceeded the integrator's admissible ceiling.
    CflViolation { courant: f64, max_courant: f64 },
    /// Free-form physics-invalidity reason that does not fit either
    /// of the structured variants above. Carries a downstream-owned
    /// message (substrate refuses, does not translate).
    Other(String),
}

impl std::fmt::Display for BracketPhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BracketPhysicsError::NegativeDepth { i, j, h } => write!(
                f,
                "bracket physics: negative depth at (i, j)=({i}, {j}), h={h:e}"
            ),
            BracketPhysicsError::CflViolation {
                courant,
                max_courant,
            } => write!(
                f,
                "bracket physics: CFL violation: courant={courant} > max={max_courant}"
            ),
            BracketPhysicsError::Other(reason) => {
                write!(f, "bracket physics: {reason}")
            }
        }
    }
}

impl std::error::Error for BracketPhysicsError {}

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

/// Lie-Poisson bracket integrator for non-separable Hamiltonians.
///
/// Single-step structure-preserving advance for Hamiltonians whose
/// canonical (q, p) split does not exist — e.g. shallow water, where
/// the Stormer-Verlet KDK path fails BY CONSTRUCTION (AURORA reply 3,
/// 2026-06-22: KDK produced 7x worse Casimir drift than forward Euler
/// on the ShallowWater factory).
///
/// The consumer owns the bracket-preserving integration internals
/// (skew-symmetric vorticity flux, consistent divergence, vector-
/// invariant momentum). The substrate calls `bracket_step` from the
/// `SYMPLECTIC_FLOW` loop, then independently checks Casimir drift
/// via `EnergyDecomposition::evaluate()` before/after the step —
/// same referee contract as the existing Stormer-Verlet KDK path.
///
/// `bracket_step` returns `Err(BracketPhysicsError)` ONLY for physics
/// invalidity (negative depth, CFL breach). It does NOT check Casimir
/// drift — that is the substrate's receipt responsibility, exercised
/// against `RECEIPT_TOL` outside the bracket implementation.
///
/// Object-safe: `&self` + `&mut [f64]` + `f64` -> `Result<()>` carries
/// no `Self` in return position and no associated types, so
/// `Box<dyn HamiltonianPoissonBracket>` is constructible.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub trait HamiltonianPoissonBracket: HamiltonianHandle {
    /// Advance `state` by one Lie-Poisson bracket step of duration
    /// `dt`. Returns `Ok(())` on physics-valid completion;
    /// `Err(BracketPhysicsError)` on negative depth / CFL breach /
    /// other physics invalidity. Casimir drift is checked by the
    /// substrate outside this call.
    fn bracket_step(&self, state: &mut [f64], dt: f64) -> Result<(), BracketPhysicsError>;
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
// Capabilities
// ─────────────────────────────────────────────────────────────────────

/// Declarative summary of which integration paths a `HamiltonianFactory`
/// supports. Read by `SYMPLECTIC_FLOW` at dispatch time to pick between
/// the Stormer-Verlet KDK path (separable Hamiltonians) and the
/// Lie-Poisson `bracket_step` path (non-separable Hamiltonians).
///
/// Both flags MAY be `true` simultaneously: a factory that supports both
/// integration paths exposes `force_drift: true` to keep comparative
/// receipt tests + diagnostic gates (e.g. AURORA A17/A18) operational,
/// and `poisson_bracket: true` to opt-in to the structure-preserving
/// integrator for non-separable physics.
///
/// At least one flag MUST be `true` for a factory to be usable by
/// `SYMPLECTIC_FLOW`; the dispatcher errors if a factory declares neither
/// integration path.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HamiltonianCapabilities {
    /// Factory's `HamiltonianHandle` supports the Stormer-Verlet
    /// kick-drift-kick path via `HamiltonianForce` + `HamiltonianDrift`.
    /// Required for A17/A18 KDK diagnostic gates.
    pub force_drift: bool,
    /// Factory's `HamiltonianHandle` additionally implements
    /// `HamiltonianPoissonBracket` and prefers the Lie-Poisson
    /// `bracket_step` path for non-separable Hamiltonians (e.g.
    /// shallow-water vorticity flux).
    pub poisson_bracket: bool,
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
    /// Downcast to a Lie-Poisson bracket integrator if this Hamiltonian
    /// supplies one. Default returns `None`, so every existing handle
    /// (Kogut-Susskind et al.) keeps working unmodified — only handles
    /// that implement `HamiltonianPoissonBracket` override this method
    /// to return `Some(self)`.
    ///
    /// `SYMPLECTIC_FLOW` uses this together with
    /// `HamiltonianFactory::capabilities()` to pick the integrator path:
    /// the factory declares a capability, the handle proves the impl,
    /// and the dispatcher checks BOTH before taking the bracket path
    /// (if a factory lies about capabilities, the dispatcher falls back
    /// to the KDK path rather than panicking).
    ///
    /// Stability: EVOLVING until gigi 0.1.0 tag.
    fn as_poisson_bracket(&self) -> Option<&dyn HamiltonianPoissonBracket> {
        None
    }
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

    /// Declare which integration paths this factory's handles support.
    ///
    /// Default returns `{ force_drift: true, poisson_bracket: false }`
    /// — the Stormer-Verlet KDK path that every pre-Phase-3 factory
    /// (e.g. `KogutSusskindFactory`) already implements via
    /// `HamiltonianForce` + `HamiltonianDrift`. Existing factories
    /// inherit this default and continue to dispatch through the KDK
    /// path with byte-identical behaviour.
    ///
    /// Factories whose handles implement `HamiltonianPoissonBracket`
    /// (the non-separable Lie-Poisson path, e.g. AURORA's
    /// `ShallowWaterFactory` after Phase 3) override this method to
    /// return `{ force_drift: true, poisson_bracket: true }`. Keeping
    /// `force_drift: true` preserves the A17/A18 KDK diagnostic gates
    /// and permits comparative receipt tests (KDK Casimir drift vs
    /// bracket Casimir drift) on the same handle.
    fn capabilities(&self) -> HamiltonianCapabilities {
        HamiltonianCapabilities {
            force_drift: true,
            poisson_bracket: false,
        }
    }
}
