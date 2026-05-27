//! Kähler-geometry substrate for GIGI.
//!
//! Implements the generator 𝒢 = (M, g, J, ∇, B, Γ) from
//! `theory/kahler_upgrade/catalog.md §1` — Kähler manifold with
//! metric, complex structure, Chern connection, closed 2-form
//! perturbation, and graph discretization preserving the J/∇ split.
//!
//! Layering (see `theory/kahler_upgrade/IMPLEMENTATION_PLAN.md`):
//! - **L1** (here): foundation — `ComplexStructure` (J), `TwoForm` /
//!   `ClosedTwoForm` (B), `KahlerStructure` wrapping both. Wired into
//!   `BundleSchema` as an `Option` so non-Kähler bundles compile and
//!   run identically.
//! - **L2** (`graph::adjacency`): dual principal/auxiliary adjacency
//!   operators with the centrality-based commutativity check
//!   (catalog §1.1).
//! - **L3** (`cost::jacobi_estimator`): trajectory-ball cardinality
//!   bounds via Jacobi-field integration (catalog §1.3, §1.4, §1.5).
//! - **L4** (`bundle::KahlerCurvature`): Ricci / Weyl /
//!   holo-bisectional / holo-sectional decomposition (catalog §E.3).
//! - **L5** (`geometry::hadamard`): Hadamard substructure detection
//!   (catalog §1.4, §1.5).
//! - **L6** (`discrete::hodge_complex`): discrete Hodge theory
//!   (catalog §2.9).
//! - **L7** (`geometry::line_bundle`): prequantization + Chern-class
//!   compression (catalog §2.1, §E.1, §E.2).
//! - **L9** (`geometry::moment_map`): moment maps + Noether
//!   conservation along Hamiltonian B-flows (catalog §2.3).
//!   Retrofit of the L1.1 placeholder that didn't land first time;
//!   ships independently because it only depends on L1's
//!   `ClosedTwoForm`.
//! - **L10** (`geometry::generative_flow`): generative flow on the
//!   Kähler bundle — Sudoku-10× keystone for the brain-primitives
//!   catalog. Implements `ẋ = B⁻¹ ∇(-log p) + √(2T) dW` and
//!   parametrizes its boundary conditions to deliver SAMPLE,
//!   FORECAST, DREAM, RECONSTRUCT as one piece of infrastructure.
//!   See `theory/brain_primitives/catalog.md`.
//! - **L11** (`geometry::predictive_coding`): three more brain
//!   primitives — INPAINT (constrained Langevin), PREDICT (single
//!   Fisher-natural-gradient step), SELF-MONITOR (kernel-density
//!   confidence). Stack on top of L10's flow infrastructure.
//! - **L12** (`geometry::attention` + `geometry::memory`): closes
//!   the brain-primitives catalog. ATTEND (softmax over distance),
//!   FOCUS (top-k sub-bundle), EPISODIC (persistent-H₀ change-point
//!   detection), SEMANTIC (Morse-compressed gist).
//!
//! L1 — this module — only declares the foundation types. Subsequent
//! layers live in their own modules and depend on what we publish
//! from here.

pub mod attention;
pub mod bundle_stats;
pub mod complex_structure;
pub mod forms;
pub mod generative_flow;
pub mod hadamard;
pub mod line_bundle;
pub mod memory;
pub mod moment_map;
pub mod predictive_coding;
pub mod quantum_cohomology;
pub mod sudoku;
pub mod toeplitz;
pub mod transport;

pub use complex_structure::{ComplexStructure, ComplexStructureError};
pub use forms::{ClosedTwoForm, ClosednessError, TwoForm};
pub use generative_flow::{
    from_diagonal_gaussian, from_isotropic_gaussian, FlowConfig, GenerativeFlow,
    GenerativeFlowError,
};
pub use hadamard::{
    detect as detect_hadamard, is_hadamard_region, HadamardRegion, HadamardSubstructure,
    HADAMARD_KB_THRESHOLD, HADAMARD_TEST_RADIUS,
};
pub use line_bundle::{ChernClass, IntegralityError, LineBundle};
pub use moment_map::{
    ConservationVerdict, InfinitesimalAction, MomentMap, MomentMapError,
};
pub use attention::{attend, focus};
pub use bundle_stats::{
    compute_stats, BundleStats, CategoricalFieldStats, NumericFieldStats,
};
pub use memory::{
    episodic_events, episodic_events_with_floor, explain, semantic_gist,
    EpisodicEvent, ExplanationPath, DEFAULT_GAP_FLOOR_EPSILON,
};
pub use predictive_coding::{
    confidence_normalized, inpaint, kernel_density_confidence, predict_one_step,
    predict_one_step_natural,
};
pub use quantum_cohomology::{CohClass, HilbertPolynomial, QuantumCohomology, QuantumError};
pub use sudoku::{
    solve_constraints, Constraint, ExpandedSolution, ExpansionConfig, ExpansionResult,
    FieldOp, NearMiss, SudokuConfig, SudokuError, SudokuRequest, SudokuResponse,
    SudokuVerdict, ViolationDetail,
};
pub use toeplitz::{toeplitz_operator, ToeplitzOperator, ToeplitzError, ToeplitzSafetyGate};
pub use transport::{
    flat_transport, BSource, TransportError, TransportResult, TransportSegment,
};

/// The Kähler structure attached to a bundle's schema.
///
/// Pairs a complex structure `J: TₚM → TₚM` (`J² = -I`) with a
/// closed 2-form `B ∈ Ω²(M)` (`dB = 0`). When present on a
/// `BundleSchema`, downstream layers (L2–L7) automatically apply
/// their Kähler-aware code paths; absent, those layers fall back to
/// the existing Riemannian behavior.
///
/// This struct is intentionally minimal in L1 — it holds the
/// invariants. Operations that USE the structure (dual adjacency,
/// Jacobi cost, Hadamard detection, prequantization, etc.) live in
/// later modules.
#[derive(Debug, Clone)]
pub struct KahlerStructure {
    /// Almost-complex structure on the tangent fibers, `J² = -I`.
    pub j: ComplexStructure,
    /// Closed 2-form perturbation, `dB = 0`. Drives the magnetic
    /// trajectory equation `∇_{γ̇} γ̇ = B(γ̇, ·)^♯` (catalog §1.2).
    pub b: ClosedTwoForm,
}

impl KahlerStructure {
    /// Construct a Kähler structure from a complex structure and a
    /// closed 2-form. Both inputs have already been validated by
    /// their own constructors — this is a packaging step.
    pub fn new(j: ComplexStructure, b: ClosedTwoForm) -> Self {
        Self { j, b }
    }

    /// Dimension of the tangent fiber the J operator acts on. Must
    /// match the dimension B acts on for the pair to be coherent.
    pub fn dim(&self) -> usize {
        self.j.dim()
    }

    /// Check the dim-coherence invariant. Cheap; called at every
    /// boundary where the two halves are constructed independently
    /// (e.g. deserialization).
    pub fn dim_coherent(&self) -> bool {
        self.j.dim() == self.b.dim()
    }
}
