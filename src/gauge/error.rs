//! `GaugeFieldError` — typed error surface for the GAUGE_FIELD
//! declaration + construction path.
//!
//! Lifted from the inline `unimplemented_for_group!` panic to a
//! return-value enum so the parser/executor can surface failure as a
//! normal user error and Halcyon's G2.D regex anchor `SU\(2\)` has a
//! stable `Display` impl to match against (Bee's locked decision 5).
//!
//! Source of truth for cross-binding error messages. Variants:
//!
//! - `SeedRequired` — `INIT HAAR_RANDOM` was declared but no SEED
//!   was provided. Display includes the literal "SEED" so Halcyon's
//!   `match="SEED"` substring check hits.
//! - `UnsupportedGroup(Group)` — group compiles but has no live math
//!   at launch (everything except `SU(2)`). Display includes the
//!   group's stable label (`"SU(2)"`, `"SU(3)"`, …).
//! - `LatticeNotDeclared(name)` — `GAUGE_FIELD ON L` referenced a
//!   lattice that the registry does not know.
//! - `FieldNotDeclared(name)` — `INIT FROM_FIELD X` referenced a
//!   field that the registry does not know.
//! - `BufferShapeMismatch { expected, got }` — a buffer materialized
//!   in a different shape than the lattice's `n_edges * repr_dim`
//!   demands.
//!
//! Inner math (`compose`, `inverse`, `read_element`) keeps its
//! Part-I panic — reaching it from a well-typed buffer is a
//! programming error, not a user error.

use super::action::BracketPhysicsError;
use super::group::Group;

/// Typed error surface for GAUGE_FIELD construction.
#[derive(Debug, Clone, PartialEq)]
pub enum GaugeFieldError {
    /// `INIT HAAR_RANDOM` declared without a SEED clause.
    SeedRequired,
    /// Group variant compiles but has no live math at launch
    /// (everything except `SU(2)`).
    UnsupportedGroup(Group),
    /// `GAUGE_FIELD ON L` references an unknown lattice.
    LatticeNotDeclared(String),
    /// `INIT FROM_FIELD X` references an unknown source field.
    FieldNotDeclared(String),
    /// A materialized buffer's flat-length does not match
    /// `n_edges * repr_dim` for the bound lattice.
    BufferShapeMismatch { expected: usize, got: usize },
    /// `GIBBS_SAMPLE … MEASURE (…)` requested an observable whose
    /// implementation needs an E field that Part IV will introduce
    /// (`HTotal`, `GaussResidualMax`, `EdgeKinetic`, `VertexGauss`,
    /// `Energy`). Display contains both "Part IV" and "E field" so
    /// the III.5 red-test and the upstream parser / HTTP layer can
    /// match either token.
    PartIvObservableNotReady(&'static str),
    /// `E_FIELD ON U` referenced a U field that the registry does
    /// not know. Distinct from `FieldNotDeclared` so the parser can
    /// surface a Part-IV-shaped error to the user.
    EFieldNotDeclared(String),
    /// `INIT FROM_FIELD other_e` tried to clone an E field that is
    /// bound to a different lattice than the U field this new E is
    /// being attached to. `e_lattice` is the lattice the source E
    /// lives on; `u_lattice` is the lattice the target U lives on.
    EFieldSourceMismatch {
        e_lattice: String,
        u_lattice: String,
    },
    /// AURORA Phase 3 — a `HamiltonianFactory` resolved by name
    /// declares neither `force_drift` nor `poisson_bracket`. The
    /// `SYMPLECTIC_FLOW` dispatcher has no admissible path and
    /// refuses to advance.
    NoIntegrationPath {
        factory: String,
        force_drift: bool,
        poisson_bracket: bool,
    },
    /// AURORA Phase 3 — `HamiltonianPoissonBracket::bracket_step`
    /// returned a physics-invalidity error (negative depth, CFL
    /// breach, …). Distinct from a Casimir-drift Refusal: drift is
    /// the substrate's receipt responsibility post-flow.
    BracketPhysics(BracketPhysicsError),
    /// AURORA Phase 3 — a `HamiltonianFactory` registered name
    /// could not be resolved at dispatch time. Distinct from
    /// `FieldNotDeclared` so consumers can surface a Phase-3-shaped
    /// error to the user.
    HamiltonianFactoryNotRegistered(String),
    /// `INIT FLUX …` routed to a non-U(1) construction path
    /// (2026-07-16). Flux is a per-edge U(1) phase materialized as a
    /// theta bundle; SU(2)/SU(3)/Z(N) links are not scalar phases.
    FluxInitRequiresU1(Group),
    /// `INIT FROM BUNDLE <b>` referenced a bundle the engine does not
    /// know (2026-07-18). Distinct from `FieldNotDeclared` (a gauge
    /// field) so the executor surfaces a bundle-shaped error.
    BundleNotFound(String),
    /// `INIT FROM BUNDLE <b>` found a bundle with zero edge records —
    /// an injection needs at least one edge to write. Almost always an
    /// emitter bug, so it refuses rather than silently registering an
    /// all-identity field.
    BundleEmpty(String),
    /// `INIT FROM BUNDLE <b>` bundle is missing a required column: a
    /// base endpoint (`vertex_a` / `vertex_b`) or a fiber component the
    /// target group's representation needs.
    BundleFieldMissing { bundle: String, column: String },
    /// `INIT FROM BUNDLE <b>`: the bundle's fiber columns do not carry
    /// the target group's representation dimension (SU(2) needs the four
    /// canonical `q0..q3` columns). `got` is how many of the group's
    /// canonical fiber columns were actually present.
    FiberArityMismatch {
        bundle: String,
        expected: usize,
        got: usize,
    },
    /// `INIT FROM BUNDLE`: a record's `(vertex_a, vertex_b)` pair is not
    /// an edge of the bound lattice in either orientation.
    NonLatticeEdge { vertex_a: i64, vertex_b: i64 },
    /// `INIT FROM BUNDLE`: a chosen SU(2) quaternion is not unit-norm
    /// (`|q|²` deviates from 1 by more than 1e-6). Rejected rather than
    /// silently normalized — `inverse == conjugate` (hence the
    /// round-trip and the order estimate) only holds for `|q| = 1`, so a
    /// silent renorm would hide emitter bugs and flip the reverse-edge
    /// read.
    NonNormalizedQuaternion { edge: usize, norm: f64 },
}

impl From<BracketPhysicsError> for GaugeFieldError {
    fn from(err: BracketPhysicsError) -> Self {
        GaugeFieldError::BracketPhysics(err)
    }
}

impl std::fmt::Display for GaugeFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GaugeFieldError::SeedRequired => write!(
                f,
                "gauge: INIT HAAR_RANDOM requires a SEED clause \
                 (intra-binding bit-identity is the contract — declare \
                 INIT HAAR_RANDOM SEED <u64>)"
            ),
            GaugeFieldError::UnsupportedGroup(g) => write!(
                f,
                "gauge: group {} is not implemented (Part II ships SU(2) math only; \
                 future groups land as separate EdgeConnection impls per the group-erasure plan)",
                g.label()
            ),
            GaugeFieldError::LatticeNotDeclared(name) => write!(
                f,
                "gauge: lattice '{name}' is not declared (DECLARE the LATTICE before attaching a GAUGE_FIELD)"
            ),
            GaugeFieldError::FieldNotDeclared(name) => write!(
                f,
                "gauge: source field '{name}' is not declared (INIT FROM_FIELD needs an existing GAUGE_FIELD)"
            ),
            GaugeFieldError::BufferShapeMismatch { expected, got } => write!(
                f,
                "gauge: buffer shape mismatch (expected {expected} f64s, got {got})"
            ),
            GaugeFieldError::PartIvObservableNotReady(name) => write!(
                f,
                "gauge: observable {name} needs an E field that Part IV will introduce \
                 (GIBBS_SAMPLE Part III ships SU(2)-substrate observables only — \
                 MeanPlaquette and QSurrogate). Part IV adds the E-field tangent-space \
                 machinery this verb composes against."
            ),
            GaugeFieldError::EFieldNotDeclared(name) => write!(
                f,
                "gauge: E field '{name}' is not declared (DECLARE the E_FIELD before referencing it)"
            ),
            GaugeFieldError::EFieldSourceMismatch {
                e_lattice,
                u_lattice,
            } => write!(
                f,
                "gauge: E source-lattice mismatch (source E lives on lattice '{e_lattice}', \
                 target U lives on lattice '{u_lattice}' — INIT FROM_FIELD requires the \
                 source E's bound lattice to match the target U's bound lattice)"
            ),
            GaugeFieldError::NoIntegrationPath {
                factory,
                force_drift,
                poisson_bracket,
            } => write!(
                f,
                "gauge: hamiltonian factory '{factory}' declares no integration path \
                 (capabilities: force_drift={force_drift}, poisson_bracket={poisson_bracket}) \
                 — SYMPLECTIC_FLOW has no admissible advance"
            ),
            GaugeFieldError::BracketPhysics(inner) => write!(
                f,
                "gauge: Lie-Poisson bracket step refused: {inner}"
            ),
            GaugeFieldError::HamiltonianFactoryNotRegistered(name) => write!(
                f,
                "gauge: hamiltonian factory '{name}' is not registered \
                 (call gauge::hamiltonian_registry::register before SYMPLECTIC_FLOW)"
            ),
            GaugeFieldError::FluxInitRequiresU1(g) => write!(
                f,
                "gauge: INIT FLUX requires GROUP U(1) this phase (got {}) — \
                 flux is a per-edge U(1) phase materialized as a theta bundle; \
                 SU(2)/SU(3)/Z(N) links are not scalar phases",
                g.label()
            ),
            GaugeFieldError::BundleNotFound(name) => write!(
                f,
                "gauge: INIT FROM BUNDLE bundle '{name}' is not found \
                 (materialize the bundle — INGEST / INIT FLUX / CREATE BUNDLE — \
                 before injecting it into a GAUGE_FIELD)"
            ),
            GaugeFieldError::BundleEmpty(name) => write!(
                f,
                "gauge: INIT FROM BUNDLE bundle '{name}' has no edge records \
                 (an injection needs at least one edge to write — check the emitter)"
            ),
            GaugeFieldError::BundleFieldMissing { bundle, column } => write!(
                f,
                "gauge: INIT FROM BUNDLE bundle '{bundle}' is missing the required \
                 column '{column}' (edge records need base 'vertex_a'/'vertex_b' \
                 plus the group's canonical fiber columns)"
            ),
            GaugeFieldError::FiberArityMismatch {
                bundle,
                expected,
                got,
            } => write!(
                f,
                "gauge: INIT FROM BUNDLE bundle '{bundle}' fiber arity mismatch \
                 (target group needs {expected} canonical fiber columns, found {got}) \
                 — an SU(2) injection needs q0, q1, q2, q3"
            ),
            GaugeFieldError::NonLatticeEdge { vertex_a, vertex_b } => write!(
                f,
                "gauge: INIT FROM BUNDLE record edge ({vertex_a} -> {vertex_b}) is not \
                 an edge of the bound lattice in either orientation"
            ),
            GaugeFieldError::NonNormalizedQuaternion { edge, norm } => write!(
                f,
                "gauge: INIT FROM BUNDLE edge {edge} carries a non-normalized SU(2) \
                 quaternion (|q| = {norm:.6}, must be 1 to within 1e-6) — rejected \
                 rather than auto-normalized so the round-trip and order estimate stay honest"
            ),
        }
    }
}

impl std::error::Error for GaugeFieldError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SeedRequired` Display contains the literal "SEED" so Halcyon's
    /// `match="SEED"` substring check hits.
    #[test]
    fn seed_required_display_contains_seed() {
        let err = GaugeFieldError::SeedRequired;
        assert!(err.to_string().contains("SEED"));
    }

    /// `UnsupportedGroup(Group::U1)` Display contains the literal
    /// "SU(2)" so Halcyon's `match="SU\\(2\\)"` regex anchor hits.
    #[test]
    fn unsupported_group_display_contains_su2() {
        let err = GaugeFieldError::UnsupportedGroup(Group::U1);
        assert!(err.to_string().contains("SU(2)"));
    }

    /// Every variant has a non-empty Display.
    #[test]
    fn every_variant_displays() {
        let variants = [
            GaugeFieldError::SeedRequired,
            GaugeFieldError::UnsupportedGroup(Group::SU3),
            GaugeFieldError::LatticeNotDeclared("L".into()),
            GaugeFieldError::FieldNotDeclared("U".into()),
            GaugeFieldError::BufferShapeMismatch { expected: 360, got: 100 },
        ];
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }
}
