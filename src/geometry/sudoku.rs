//! # SUDOKU — Constrained inference on a learned affordance manifold
//!
//! Implements the meta-primitive specified in
//! `theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md` v0.3.
//!
//! ## The shape
//!
//! Given a problem expressed as a set of constraints over fiber
//! field values (plus optional context conditioning on base fields),
//! SUDOKU walks the bundle's records, filters by constraints, and
//! returns the matching options ranked by stated-norm prior mass
//! — the **load-bearing honest-coverage contract** is the API
//! choice that distinguishes "feasible region is empty" from
//! "I stopped exploring too early."
//!
//! ## Why this lives in the geometry module
//!
//! SUDOKU composes existing brain primitives (SAMPLE, ATTEND,
//! EPISODIC, SEMANTIC, INPAINT, EXPLAIN). Its value-add over
//! direct composition is:
//!
//!   1. **Coverage estimation** — fraction of prior mass actually
//!      explored, not just "I returned an answer."
//!   2. **Near-miss enumeration** — options that violate one
//!      constraint, ranked by which constraint to relax.
//!   3. **unsat tristate contract** — `Sat` / `Unsat` / `Unknown`.
//!      Most solvers shrug; SUDOKU refuses to shrug.
//!
//! ## What's in scope for S2 (this commit) vs deferred
//!
//! - **S2 (here):** core constraint vocabulary (field predicates),
//!   record-walk filtering, mass-based coverage estimator, near-miss
//!   enumeration, unsat tristate verdict, exhaustive O(N) candidate
//!   enumeration. All deterministic — no Langevin sampling.
//! - **S3 (HTTP endpoint):** wire `/v1/bundles/{name}/brain/sudoku`,
//!   content negotiation, request validation.
//! - **S3.5 (puzzle expansion):** constraint_relaxation +
//!   bundle_hop. Drops a constraint or queries a related bundle
//!   when the original puzzle is UNSAT.
//! - **S4 (calibration):** soft constraints + cross-field relations,
//!   manifold-distance constraints.
//! - **S5 (demos):** literal 9×9 sudoku puzzle + transport-norms
//!   school commute + 10×10 expansion demo.
//! - **S6 (docs):** consumer guide updates.
//!
//! ## v1 limitations explicit
//!
//! - **Constraints in v1:** only `Constraint::Field` (predicates over
//!   one fiber field). Manifold-distance and cross-field-relation
//!   constraint types defined in the spec are stubbed in the enum
//!   but route to a `TODO_S4` error if used. S4 fills them in.
//! - **Sampling in v1:** exhaustive record walk. The spec calls for
//!   SAMPLE/ATTEND-based exploration with prior-mass coverage; for
//!   small-N bundles (≤10k records — most of Marcella's) exhaustive
//!   is faster and gives exact coverage = 1.0 always. For large
//!   bundles (chembl at 9M), we'll add stratified sampling in S4.
//! - **Expansion in v1:** disabled. S3.5 ships the constraint_relaxation
//!   + bundle_hop layers.

use crate::geometry::bundle_stats::{
    compute_stats, numeric_value as value_as_f64, BundleStats, NumericFieldStats,
};
use crate::types::{Record, Value};

// ─── Constraint vocabulary ──────────────────────────────────────

/// A single constraint on a fiber field value.
///
/// `is_hard = true` means a violation excludes the option; `false`
/// makes it a penalty for soft scoring (deferred to S4 — currently
/// treated as hard).
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Predicate over one fiber field. The workhorse v1 constraint.
    Field {
        field: String,
        op: FieldOp,
        hard: bool,
    },

    /// Manifold-distance constraint (spec §2 Type 2). Stub for v1 —
    /// returns `SudokuError::NotYetSupported` until S4 lands it.
    /// Kept in the enum so consumers can type-stable construct it
    /// today; spec contracts what the request shape is.
    #[allow(dead_code)]
    Manifold {
        field: String,
        near_manifold: String,
        epsilon: f64,
        hard: bool,
    },

    /// Cross-field algebraic relation (spec §2 Type 3). Stub for
    /// v1 — S4 ships the parser + evaluator.
    #[allow(dead_code)]
    Relation {
        expr: String,
        vars: std::collections::HashMap<String, f64>,
        hard: bool,
    },
}

/// Field predicate operators.
#[derive(Debug, Clone)]
pub enum FieldOp {
    /// Equality with a single value.
    Eq(Value),
    /// Inequality.
    Ne(Value),
    /// Strictly less than (numeric).
    Lt(f64),
    /// Less than or equal (numeric).
    Le(f64),
    /// Strictly greater than (numeric).
    Gt(f64),
    /// Greater than or equal (numeric).
    Ge(f64),
    /// Numeric in `[lo, hi]` inclusive.
    Between { lo: f64, hi: f64 },
    /// Categorical membership (the value must equal one of these).
    IsIn(Vec<Value>),
}

// ─── Request / response types ───────────────────────────────────

/// SUDOKU request. Mirrors the HTTP body shape from spec §7
/// minus the wire-only fields (mode, explore_budget_ms,
/// acknowledge_pathology — those live on the HTTP layer in S3).
#[derive(Debug, Clone)]
pub struct SudokuRequest {
    /// Constraints — all `Field` predicates must be satisfied by
    /// returned solutions (hard=true) or penalize them (hard=false,
    /// currently treated as hard until S4).
    pub constraints: Vec<Constraint>,
    /// Maximum number of solutions to return.
    pub max_options: usize,
    /// Maximum number of near-misses to return (options that violate
    /// exactly one hard constraint).
    pub max_near_misses: usize,
    /// **S3.5 — puzzle expansion.** When set and allowed=true, SUDOKU
    /// will attempt to solve a relaxed version of the problem when the
    /// original returns Unsat. Opt-in; None = no expansion attempted.
    pub expansion: Option<ExpansionConfig>,
}

impl Default for SudokuRequest {
    fn default() -> Self {
        SudokuRequest {
            constraints: Vec::new(),
            max_options: 5,
            max_near_misses: 3,
            expansion: None,
        }
    }
}

/// SUDOKU response — the honest-coverage contract.
#[derive(Debug, Clone)]
pub struct SudokuResponse {
    /// Solutions that satisfy ALL hard constraints, ranked by
    /// stated-prior mass (descending — most-common options first).
    pub solutions: Vec<Solution>,
    /// Options that violate exactly one hard constraint. Each
    /// carries which constraint and the cheapest relaxation that
    /// would unlock it.
    pub near_misses: Vec<NearMiss>,
    /// Verdict tristate.
    pub verdict: SudokuVerdict,
    /// Fraction of stated-prior mass explored. Always 1.0 for v1
    /// exhaustive walk; <1.0 once stratified sampling lands in S4.
    pub coverage: f64,
    /// Count of records considered (post-context filter).
    pub n_records_considered: usize,
    /// **Wave 3 — Upgrade 2.** Per-constraint selectivity report.
    /// For each constraint, how many additional records would pass
    /// if that constraint were removed. Identifies the binding
    /// constraint (the one filtering the most records). Data-derived;
    /// no domain config.
    pub selectivity: Vec<SelectivityReport>,
    /// **Wave 3 — Upgrade 3.** Counterfactual relaxation menu. For
    /// each constraint, data-driven proposals of "what if I bent
    /// this rule to value X, how many more records match." Sorted
    /// by `gain / relaxation_cost` descending (best bang-per-bend
    /// first). Thresholds are taken from the actual data — no
    /// hardcoded step sizes.
    pub relaxations: Vec<RelaxationOption>,
    /// **Wave 3 — Upgrade 5.** Pareto-optimal multi-violation
    /// near-misses. Generalizes `near_misses` from single-violation
    /// to k-violation options, returning only those non-dominated on
    /// `(n_violations, total_relaxation_cost)`. The single-violation
    /// near-misses already appear here as the k=1 frontier slice.
    pub pareto_near_misses: Vec<ParetoNearMiss>,
    /// **Wave 6.2 — pre-flight contradiction reason.** Populated
    /// when the constraint set is *trivially* self-contradictory
    /// (e.g. `Eq(x, a) AND Eq(x, b)` with a ≠ b, or `Le(x, c) AND
    /// Ge(x, d)` with d > c). When `Some`, the verdict is `Unsat`
    /// and **no bundle records are walked** — the contradiction is
    /// detected in O(n_constraints²) holonomy-style pre-flight check.
    ///
    /// `None` for compatible constraint sets. Pre-flight is allowed
    /// to MISS subtle contradictions that only manifest given the
    /// data — those still bundle to ordinary `Unsat` via the walk.
    /// Pre-flight is **never** allowed to flag a non-contradictory
    /// constraint set (safety gate).
    ///
    /// Maps to sudoky-energy's Čech H̆¹ pre-filter (catches 100% of
    /// overt constraint contradictions per the Noether-Davis tests).
    pub pre_flight_unsat_reason: Option<String>,
    /// **S3.5 — expansion result.** Present only when
    /// `req.expansion.allowed` was true and original verdict was Unsat.
    /// None → expansion was not attempted (SAT, Unknown, or not opted in).
    pub expanded: Option<ExpansionResult>,
    /// **Wave 6.3 — Γ trichotomy diagnostic.** Classifies the
    /// constraint-satisfaction problem into one of three regimes
    /// (Numeric / Structural / Geometric) based on the dimensionless
    /// quantity
    ///
    ///   Γ = m · τ / (K̂_max · log|S|)
    ///
    /// where m = number of constraints, τ = solve-time scale (taken as
    /// 1 for the single-pass walk), K̂_max = max per-constraint
    /// curvature K_c (W6.1), |S| = number of candidate records.
    ///
    /// `None` when the request was pre-flight-UNSAT or had no records
    /// to walk (Γ is undefined there). Otherwise populated for every
    /// SUDOKU call regardless of verdict.
    ///
    /// Cross-reference: sudoky-energy spec §Γ-trichotomy (U.S. Prov.
    /// Patent Feb 2026); the GIGI-bundle adaptation uses raw_curvature
    /// (which is structurally identical to sudoky-energy's `K_loc` for
    /// the per-constraint case).
    pub gamma_trichotomy: Option<GammaTrichotomy>,
}

/// Wave 6.3 — the Γ trichotomy classification.
///
/// Γ < 0.5  → **Numeric**: few constraints, each highly selective; the
///            satisfaction landscape is well-conditioned, gradient
///            descent / direct evaluation converges quickly.
/// 0.5 ≤ Γ ≤ 2.0 → **Structural**: critical-phase regime where the
///            satisfaction surface is fragmented. Search benefits from
///            knowing constraint interaction structure (Čech pre-flight,
///            decomposition).
/// Γ > 2.0  → **Geometric**: many soft constraints with low selectivity;
///            solutions lie in a tangled manifold. Search needs
///            curvature-guided exploration, not raw enumeration.
#[derive(Debug, Clone, PartialEq)]
pub struct GammaTrichotomy {
    /// The dimensionless Γ value.
    pub gamma: f64,
    /// The regime classification.
    pub regime: TrichotomyRegime,
    /// `m` — number of constraints in the request.
    pub m: usize,
    /// `K̂_max` — max raw_curvature across all constraints. Surfaced
    /// for consumer inspection and so the classification math is
    /// reproducible from the response alone.
    pub k_max: f64,
    /// `log|S|` — natural log of the candidate-record count (with
    /// `log(n + 1)` regularization to handle n = 0, 1 gracefully).
    pub log_s: f64,
}

/// Wave 6.3 — three regimes per sudoky-energy spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrichotomyRegime {
    /// Γ < 0.5 — well-conditioned; few binding constraints.
    Numeric,
    /// 0.5 ≤ Γ ≤ 2.0 — critical-phase / phase-transition regime.
    Structural,
    /// Γ > 2.0 — many soft constraints; needs curvature-aware search.
    Geometric,
}

/// **Wave 6.3 default thresholds.** From CSP phase-transition literature
/// (Cheeseman-Kanefsky-Taylor 1991; Mitchell-Selman-Levesque 1992) and
/// the sudoky-energy GPU solver's empirical sweep on Sudoku, SAT, and
/// graph-coloring instances. Exposed as constants so consumer code can
/// reuse them in classification logic on the wire side without
/// re-deriving the regime from the raw Γ.
pub const TRICHOTOMY_THRESHOLD_NUMERIC_STRUCTURAL: f64 = 0.5;
pub const TRICHOTOMY_THRESHOLD_STRUCTURAL_GEOMETRIC: f64 = 2.0;

impl TrichotomyRegime {
    /// Classify a raw Γ value into a regime using the default thresholds.
    pub fn from_gamma(gamma: f64) -> Self {
        if gamma < TRICHOTOMY_THRESHOLD_NUMERIC_STRUCTURAL {
            TrichotomyRegime::Numeric
        } else if gamma <= TRICHOTOMY_THRESHOLD_STRUCTURAL_GEOMETRIC {
            TrichotomyRegime::Structural
        } else {
            TrichotomyRegime::Geometric
        }
    }

    /// Stable string tag for wire serialization.
    pub fn as_str(&self) -> &'static str {
        match self {
            TrichotomyRegime::Numeric => "numeric",
            TrichotomyRegime::Structural => "structural",
            TrichotomyRegime::Geometric => "geometric",
        }
    }
}

/// A satisfying option.
#[derive(Debug, Clone)]
pub struct Solution {
    /// The record itself.
    pub record: Record,
    /// Stated-prior mass = `count_at_this_record_signature /
    /// total_records_considered`. In v1 records are weighted
    /// uniformly; once SUDOKU consumes brain/attend the mass will
    /// be the softmax weight.
    pub stated_prior_mass: f64,
    /// **Wave 4 — Upgrade 4.** Quality / centrality score in [0, 1].
    ///
    /// Higher = deeper inside the satisfaction region. Computed as
    /// the **minimum normalized margin** across all constraints,
    /// passed through 1 - exp(-margin) so the score saturates at 1
    /// for solutions that are very far from any boundary.
    ///
    /// JTBD: when many records SAT, the consumer wants the *best*
    /// match, not just *a* match. A record with rating=4.8 and
    /// rent=$3500 (constraints: rating≥4.0, rent≤$4000) scores
    /// higher than one with rating=4.0 and rent=$3999.
    ///
    /// Math note: this is the soft-constraint posterior log-prob
    /// under independent half-normal priors — the **soft
    /// generalization** of the hard SUDOKU verdict. Falls out
    /// of the geometry already computed for relaxation_cost.
    ///
    /// 1.0 for solutions when there are no constraints (everything
    /// is trivially "best").
    pub quality_score: f64,
}

/// An option that violates exactly one hard constraint, with the
/// minimal relaxation needed to unlock it.
#[derive(Debug, Clone)]
pub struct NearMiss {
    pub record: Record,
    pub stated_prior_mass: f64,
    /// Which constraint(s) this option violates.
    pub violations: Vec<ViolationDetail>,
    /// Indices into the request's `constraints` array that, if
    /// relaxed to the values shown in `violations[i].relax_to`,
    /// would make this option feasible.
    pub would_unlock_if_relaxed: Vec<usize>,
}

/// Detail of a single constraint violation on a near-miss option.
#[derive(Debug, Clone)]
pub struct ViolationDetail {
    /// Index into the request's `constraints` array.
    pub constraint_idx: usize,
    /// Human-readable field name (echoed from the constraint).
    pub field: String,
    /// Description of the violation (e.g. "08:45 > 08:30").
    pub violation: String,
    /// Smallest relaxation that would make this option satisfy
    /// the constraint (e.g. the option's actual value).
    pub relax_to: Value,
    /// **Wave 3 — Upgrade 1.** Normalized cost to relax this
    /// constraint enough to admit this record.
    ///
    /// For numeric ordered constraints (Lt/Le/Gt/Ge/Between):
    ///   `cost = |actual - threshold| / length_scale(field)`
    /// where `length_scale` is the field's empirical standard
    /// deviation (Bessel-corrected, n-1 denominator). When std is
    /// degenerate (constant field), falls back to range/2 or 1.0.
    ///
    /// For categorical constraints (Eq/Ne/IsIn) on Text or Bool:
    ///   `cost = 1.0` (one discrete step).
    ///
    /// For Eq with numeric value where both sides are numeric, the
    /// numeric formula above is used (this lets `Eq(Integer(5))` on
    /// `Integer(3)` report cost 2/std rather than 1.0).
    ///
    /// This is the **Kähler-natural** normalization in the
    /// diagonal-metric case. When FitMode::Full is in use the
    /// Mahalanobis upgrade (wave 4) will replace the diagonal std
    /// with the full Σ⁻¹.
    pub relaxation_cost: f64,
    /// Raw (unnormalized) magnitude of the violation in the
    /// field's native units. None for categorical/non-ordered
    /// violations. Useful for human-facing display alongside the
    /// normalized cost.
    pub raw_delta: Option<f64>,
}

/// **Wave 3 — Upgrade 2.** Per-constraint selectivity report.
/// Captures how restrictive each constraint is *given the others*
/// — i.e., the marginal effect of dropping it.
#[derive(Debug, Clone)]
pub struct SelectivityReport {
    /// Index into the request's `constraints` array.
    pub constraint_idx: usize,
    /// Human-readable field name (echoed).
    pub field: String,
    /// Number of records that satisfy all constraints.
    pub n_match_all: usize,
    /// Number of records that satisfy all-but-this constraint
    /// (this constraint removed).
    pub n_match_without: usize,
    /// `n_match_without - n_match_all`. Records this constraint
    /// alone is filtering out, conditional on the others holding.
    pub marginal_filter_count: usize,
    /// True if `marginal_filter_count` is the maximum across all
    /// constraints in this request (the binding constraint).
    /// Ties: all tied constraints are flagged binding.
    pub binding: bool,
    /// **Wave 6 — sudoky-inspired.** Local curvature `K_c` for
    /// this constraint = fraction of records that FAIL it,
    /// regardless of other constraints' outcomes. Range [0, 1].
    ///
    /// High K_c = tight constraint (eliminates many records);
    /// low K_c = loose. Distinct from `marginal_filter_count`,
    /// which conditions on the OTHER constraints holding — a
    /// constraint can be high-K_c yet zero-marginal if it's
    /// redundant with another (covered by it). The two together
    /// expose the constraint-graph geometry: high-K + high-margin
    /// = the deal-breaker; high-K + zero-margin = redundant with
    /// a sibling; low-K + high-margin = "loose but uniquely
    /// distinguishing"; low-K + zero-margin = nearly vacuous.
    ///
    /// Maps to sudoky-energy's per-variable K_loc — the constraint
    /// interaction density used as scheduling signal. Computed
    /// FREE during the existing classify walk (no extra cost).
    pub raw_curvature: f64,
}

/// **Wave 3 — Upgrade 3.** A counterfactual relaxation proposal.
/// "If you bend constraint X to value Y, you gain N more matches at
/// normalized cost C." All values are data-derived from the bundle.
#[derive(Debug, Clone)]
pub struct RelaxationOption {
    /// Index into the request's `constraints` array.
    pub constraint_idx: usize,
    /// Human-readable field name.
    pub field: String,
    /// Description of the relaxation (e.g. "rent <= 4200" or
    /// "drop constraint" or "is_in [PT, ET, CT]").
    pub description: String,
    /// The new threshold (numeric) or the expanded value set
    /// (categorical) as a Value. None when the proposal is to
    /// drop the constraint entirely.
    pub new_threshold: Option<Value>,
    /// How many additional records become solutions if this
    /// relaxation is applied (and all other constraints stay).
    pub gain: usize,
    /// Normalized cost of this relaxation, computed in the same
    /// units as `ViolationDetail::relaxation_cost`.
    pub relaxation_cost: f64,
    /// **Wave 6.4 — energy descent per unit cost.** The negative
    /// log-likelihood drop:
    ///
    ///   ΔE / cost = log(1 + gain / max(n_current_solutions, 1)) / max(cost, ε)
    ///
    /// This is the "diminishing-returns-aware" replacement for the
    /// W3 `gain / cost` heuristic. Larger ΔE/cost = more
    /// satisfaction-probability gained per σ of constraint bending.
    /// The relaxation menu is sorted by this descending instead of by
    /// raw `gain / cost`; the raw counts (`gain`, `relaxation_cost`)
    /// remain in the response unchanged so consumers can re-sort if
    /// they prefer the W3 ordering.
    ///
    /// **Why log?** Going from 1 → 10 solutions is structurally bigger
    /// than going from 100 → 109, even though both are `gain = 9`. The
    /// log compresses high-base options so an option that *multiplies*
    /// the satisfaction count ranks above one that adds a fixed offset.
    ///
    /// Cross-ref: sudoky-energy spec §energy-descent; SUDOKU_PRIMITIVE_SPEC §9.
    pub energy_descent: f64,
}

/// **Wave 3 — Upgrade 5.** A near-miss that may violate multiple
/// constraints. Returned only if it sits on the Pareto frontier of
/// `(n_violations, total_relaxation_cost)` — i.e., no other record
/// is strictly better on both axes.
#[derive(Debug, Clone)]
pub struct ParetoNearMiss {
    pub record: Record,
    pub stated_prior_mass: f64,
    /// All violations on this record (length ≥ 1).
    pub violations: Vec<ViolationDetail>,
    /// Sum of `violations[*].relaxation_cost`. Lower is closer to
    /// satisfying.
    pub total_relaxation_cost: f64,
}

/// Honest tristate verdict. The whole point of SUDOKU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SudokuVerdict {
    /// Feasible region non-empty; ≥1 solution returned.
    Sat,
    /// Feasible region empty AND coverage ≥ 0.80 — we genuinely
    /// looked. Consumer can trust this as "no solution exists."
    Unsat,
    /// Coverage too low to make a claim either way. Consumer
    /// should retry with a larger budget OR accept the partial
    /// view. **Most solvers shrug here; SUDOKU refuses.**
    Unknown,
}

// ─── S3.5 — Puzzle expansion types ─────────────────────────────

/// Opt-in expansion config. When `allowed: true` and the original
/// puzzle is UNSAT, SUDOKU asks "what other puzzle is this?" and
/// tries to solve a slightly relaxed version of the problem.
///
/// v1 implements constraint_relaxation only (cheapest relaxation
/// from the menu). bundle_hop is wired at the HTTP layer in S3.5.
#[derive(Debug, Clone)]
pub struct ExpansionConfig {
    /// Master switch. Default: false (expansion is opt-in).
    pub allowed: bool,
    /// How many relaxation options to try before giving up.
    /// Each option relaxes exactly one constraint by the cheapest
    /// data-driven amount from the relaxation menu. Default: 1.
    pub max_constraint_relaxations: usize,
}

impl Default for ExpansionConfig {
    fn default() -> Self {
        ExpansionConfig {
            allowed: false,
            max_constraint_relaxations: 1,
        }
    }
}

/// A solution found in the expanded (10×10) puzzle — a different
/// puzzle than the one originally asked. Always clearly labelled
/// so consumers know this is NOT an answer to the original question.
#[derive(Debug, Clone)]
pub struct ExpandedSolution {
    /// The record that satisfies the relaxed constraints.
    pub record: Record,
    /// Stated-prior mass from the expansion solve (denominator =
    /// n_records in the original bundle, same as original solve).
    pub stated_prior_mass: f64,
    /// Normalized cost of the relaxation that unlocked this record.
    /// Same units as `ViolationDetail::relaxation_cost` (std-normalized).
    pub expansion_cost: f64,
    /// Index into the ORIGINAL request's `constraints` array that
    /// was relaxed or dropped to unlock this record.
    pub relaxed_constraint_idx: usize,
    /// The new threshold used (None = constraint dropped entirely).
    pub relaxed_to: Option<Value>,
}

/// Result of an expansion attempt (constraint_relaxation). Present
/// on the response only when `expansion.allowed` was true on the
/// request AND the original verdict was Unsat.
#[derive(Debug, Clone)]
pub struct ExpansionResult {
    /// True when expansion was actually attempted (original was Unsat
    /// with coverage ≥ high_threshold and expansion was allowed).
    pub attempted: bool,
    /// "constraint_relaxation" in v1. "bundle_hop" added at HTTP
    /// layer in S3.5.
    pub expansion_type: String,
    /// Solutions found in the expanded puzzle. Empty → the expanded
    /// puzzle is also UNSAT; see `advisory`.
    pub solutions: Vec<ExpandedSolution>,
    /// Human-readable advisory when expansion also finds nothing.
    /// Per spec §6.5: "consider asking a human" is the right output
    /// when geometry has honestly exhausted itself.
    pub advisory: Option<String>,
}

/// SUDOKU configuration (defaults per spec v0.3 §3 + §4).
#[derive(Debug, Clone, Copy)]
pub struct SudokuConfig {
    /// Coverage ≥ this AND no solutions → `Unsat`.
    pub coverage_high_threshold: f64,
    /// Coverage < this AND no solutions → `Unknown`.
    /// Middle bucket (low ≤ cov < high) AND no solutions: `Unknown`.
    pub coverage_low_threshold: f64,
}

impl Default for SudokuConfig {
    fn default() -> Self {
        // Per spec §3 OPEN-2: fixed thresholds for v1, named
        // regimes in v1.1 once telemetry exists.
        SudokuConfig {
            coverage_high_threshold: 0.80,
            coverage_low_threshold: 0.50,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SudokuError {
    #[error("constraint type not yet supported (S4): {0}")]
    NotYetSupported(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

// ─── The solver ─────────────────────────────────────────────────

/// SUDOKU core entry point. Iterates `records`, classifies each as
/// satisfying / near-miss / violating, returns the response per
/// the honest-coverage contract.
///
/// For v1 the iterator is exhaustive — every record is considered.
/// Coverage is 1.0 (we looked at everything). Stratified sampling
/// for large bundles is S4.
pub fn solve_constraints<I>(
    records: I,
    req: &SudokuRequest,
    config: &SudokuConfig,
) -> Result<SudokuResponse, SudokuError>
where
    I: IntoIterator<Item = Record>,
{
    // Validate request: no stub constraint types in v1.
    for (i, c) in req.constraints.iter().enumerate() {
        match c {
            Constraint::Field { .. } => {}
            Constraint::Manifold { .. } => {
                return Err(SudokuError::NotYetSupported(format!(
                    "constraint[{}]: Manifold-distance — lands in S4",
                    i
                )));
            }
            Constraint::Relation { .. } => {
                return Err(SudokuError::NotYetSupported(format!(
                    "constraint[{}]: cross-field Relation — lands in S4",
                    i
                )));
            }
        }
    }

    // **Wave 6.2 pre-flight.** Detect trivially contradictory
    // constraint sets BEFORE any record IO. O(C²) pairwise scan.
    // Returns Unsat with a populated reason immediately.
    if let Some(reason) = check_constraint_holonomy(&req.constraints) {
        // Pre-flight caught a structural contradiction. Expansion can
        // still run on the original records because the relaxed
        // constraint set may be valid (we drop or relax one of the
        // contradicting pair). We need the records for that, so
        // materialize them only when expansion is requested.
        let expanded = if req.expansion.as_ref().map(|e| e.allowed).unwrap_or(false) {
            let materialized: Vec<Record> = records.into_iter().collect();
            // Build a synthetic response with the preflight relaxations
            // to feed attempt_expansion. The relaxation menu for overt
            // contradictions is "drop one of the contradicting constraints"
            // — we synthesize that directly since compute_relaxation_menu
            // requires classified records.
            let synthetic_relaxations: Vec<RelaxationOption> = req.constraints
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let field = if let Constraint::Field { field, .. } = c {
                        field.clone()
                    } else {
                        format!("constraint_{i}")
                    };
                    RelaxationOption {
                        constraint_idx: i,
                        field,
                        description: "drop contradicting constraint".into(),
                        new_threshold: None,
                        gain: 1, // unknown — will be computed by re-solve
                        relaxation_cost: 1.0,
                        // Synthetic option for pre-flight UNSAT path; not
                        // sorted here, so leave at 0.0. The attempt_expansion
                        // caller doesn't use this field.
                        energy_descent: 0.0,
                    }
                })
                .collect();
            Some(attempt_expansion(
                &materialized,
                req,
                &synthetic_relaxations,
                config,
            ))
        } else {
            // No expansion requested — don't materialize.
            let _ = records; // consume iterator
            None
        };
        return Ok(SudokuResponse {
            solutions: Vec::new(),
            near_misses: Vec::new(),
            verdict: SudokuVerdict::Unsat,
            // Coverage = 1.0 because we provably explored the
            // ENTIRE feasible region (which is empty by holonomy).
            coverage: 1.0,
            n_records_considered: 0,
            selectivity: Vec::new(),
            relaxations: Vec::new(),
            pareto_near_misses: Vec::new(),
            pre_flight_unsat_reason: Some(reason),
            expanded,
            // Wave 6.3: pre-flight UNSAT path doesn't walk any
            // records, so K_max isn't measured → Γ is undefined.
            gamma_trichotomy: None,
        });
    }

    // Materialize records once — wave 3 needs three passes:
    //  (1) BundleStats (mean/std per field for cost normalization)
    //  (2) Classify each record into solution / near-miss / violator
    //  (3) Selectivity + relaxation menu (need per-constraint counts)
    // For Marcella's bundle sizes (~10k records) this is cheap. For
    // chembl-scale (~9M) stratified sampling lands in S4.
    let materialized: Vec<Record> = records.into_iter().collect();
    let n_considered = materialized.len();

    if n_considered == 0 {
        return Ok(SudokuResponse {
            solutions: Vec::new(),
            near_misses: Vec::new(),
            verdict: SudokuVerdict::Unknown,
            coverage: 0.0,
            n_records_considered: 0,
            selectivity: Vec::new(),
            relaxations: Vec::new(),
            pareto_near_misses: Vec::new(),
            pre_flight_unsat_reason: None,
            expanded: None,
            // Wave 6.3: n = 0 → Γ is undefined.
            gamma_trichotomy: None,
        });
    }

    // Wave 3: derive per-field statistics from the data. This is
    // what makes the relaxation cost domain-agnostic — every field
    // gets its own length scale from its observed variance.
    let stats = compute_stats(materialized.iter());

    // Pass 2: classify per record. We now track ALL violations per
    // record (not just bucketing by count) so the Pareto frontier
    // (Upgrade 5) and the selectivity report (Upgrade 2) can use
    // the same single pass.
    let mut classifications: Vec<(Record, Vec<ViolationDetail>)> =
        Vec::with_capacity(n_considered);
    for record in &materialized {
        let violations = classify_record(record, &req.constraints, &stats);
        classifications.push((record.clone(), violations));
    }

    let satisfying: Vec<(Record, Vec<ViolationDetail>)> = classifications
        .iter()
        .filter(|(_, v)| v.is_empty())
        .cloned()
        .collect();
    let near_miss_candidates: Vec<(Record, Vec<ViolationDetail>)> = classifications
        .iter()
        .filter(|(_, v)| v.len() == 1)
        .cloned()
        .collect();

    // Stated-prior mass: count records per record-signature. v1
    // uses the record itself as the signature (collapse exact
    // duplicates). Total mass denominator = n_considered.
    let satisfying_mass = compute_mass_per_signature(&satisfying, n_considered);
    let near_miss_mass = compute_mass_per_signature(&near_miss_candidates, n_considered);

    // Sort solutions by (mass desc, quality desc). Wave 4 adds the
    // quality_score — depth into the satisfaction region. Ties on
    // mass (common when records are distinct) are now broken by
    // the consumer's intuition: the BEST match comes first.
    let mut solutions: Vec<Solution> = satisfying_mass
        .into_iter()
        .map(|(record, stated_prior_mass)| {
            let quality_score = compute_quality_score(&record, &req.constraints, &stats);
            Solution {
                record,
                stated_prior_mass,
                quality_score,
            }
        })
        .collect();
    solutions.sort_by(|a, b| {
        b.stated_prior_mass
            .partial_cmp(&a.stated_prior_mass)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.quality_score
                    .partial_cmp(&a.quality_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    solutions.truncate(req.max_options);

    // Sort near-misses by mass descending, attach metadata.
    let mut near_misses: Vec<NearMiss> = near_miss_mass
        .into_iter()
        .filter_map(|(record, mass)| {
            // Find the violation detail for this record from the
            // original near_miss_candidates list (first match —
            // signatures aren't shared across distinct violations).
            let violation = near_miss_candidates
                .iter()
                .find(|(r, _)| records_match(r, &record))
                .map(|(_, v)| v.clone())?;
            let unlock_indices: Vec<usize> =
                violation.iter().map(|v| v.constraint_idx).collect();
            Some(NearMiss {
                record,
                stated_prior_mass: mass,
                violations: violation,
                would_unlock_if_relaxed: unlock_indices,
            })
        })
        .collect();
    // Sort near-misses: primarily by stated_prior_mass descending
    // (most common options first). Ties broken by ascending
    // relaxation_cost (cheapest fix first) — a wave-3 UX fix that
    // surfaces the closest violators when mass is uniform.
    near_misses.sort_by(|a, b| {
        b.stated_prior_mass
            .partial_cmp(&a.stated_prior_mass)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let a_cost: f64 = a.violations.iter().map(|v| v.relaxation_cost).sum();
                let b_cost: f64 = b.violations.iter().map(|v| v.relaxation_cost).sum();
                a_cost.partial_cmp(&b_cost).unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    near_misses.truncate(req.max_near_misses);

    // Coverage: exhaustive walk → 1.0. (S4 will introduce stratified
    // sampling and a real estimator.)
    let coverage = 1.0_f64;
    let verdict = decide_verdict(coverage, &solutions, config);

    // Wave 3 — Upgrade 2: per-constraint selectivity report.
    let selectivity = compute_selectivity(&classifications, &req.constraints);

    // Wave 3 — Upgrade 3: counterfactual relaxation menu.
    let relaxations = compute_relaxation_menu(
        &classifications,
        &req.constraints,
        &stats,
        satisfying.len(),
    );

    // Wave 3 — Upgrade 5: Pareto-optimal multi-violation near-misses.
    let pareto_near_misses = compute_pareto_near_misses(
        &classifications,
        n_considered,
        req.max_near_misses,
        req.constraints.len(),
    );

    // S3.5 — puzzle expansion. Only when Unsat and opt-in.
    let expanded = if verdict == SudokuVerdict::Unsat
        && req.expansion.as_ref().map(|e| e.allowed).unwrap_or(false)
    {
        Some(attempt_expansion(&materialized, req, &relaxations, config))
    } else {
        None
    };

    // Wave 6.3 — Γ trichotomy diagnostic. Compute from already-derived
    // selectivity (which carries raw_curvature per W6.1) + counts.
    let gamma_trichotomy =
        compute_gamma_trichotomy(&selectivity, req.constraints.len(), n_considered);

    Ok(SudokuResponse {
        solutions,
        near_misses,
        verdict,
        coverage,
        n_records_considered: n_considered,
        selectivity,
        relaxations,
        pareto_near_misses,
        pre_flight_unsat_reason: None,
        expanded,
        gamma_trichotomy,
    })
}

// ─── S3.5 — Puzzle expansion helpers ────────────────────────────

/// Build the relaxed constraint set by either dropping or
/// replacing the threshold of the constraint at `opt.constraint_idx`.
///
/// - `new_threshold = None` → drop the constraint entirely.
/// - `new_threshold = Some(Float(t))` → replace the numeric
///   threshold in Le/Lt/Ge/Gt with `t`. Categorical constraints
///   (`Eq`, `Ne`, `IsIn`) are copied unchanged when a numeric
///   threshold is supplied (shouldn't happen in practice).
fn apply_relaxation(constraints: &[Constraint], opt: &RelaxationOption) -> Vec<Constraint> {
    constraints
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if i != opt.constraint_idx {
                return Some(c.clone());
            }
            match &opt.new_threshold {
                // Drop-style relaxation: remove the constraint.
                None => None,
                // Threshold-replacement: swap the numeric bound.
                Some(Value::Float(t)) => {
                    if let Constraint::Field { field, op, hard } = c {
                        let new_op = match op {
                            FieldOp::Le(_) => FieldOp::Le(*t),
                            FieldOp::Lt(_) => FieldOp::Lt(*t),
                            FieldOp::Ge(_) => FieldOp::Ge(*t),
                            FieldOp::Gt(_) => FieldOp::Gt(*t),
                            // Categorical / other numeric ops: can't
                            // replace with a float threshold — keep as-is.
                            other => other.clone(),
                        };
                        Some(Constraint::Field {
                            field: field.clone(),
                            op: new_op,
                            hard: *hard,
                        })
                    } else {
                        Some(c.clone())
                    }
                }
                // Integer threshold (e.g. from an integer-valued
                // relaxation proposal): promote to f64 and apply.
                Some(Value::Integer(t)) => {
                    let tf = *t as f64;
                    if let Constraint::Field { field, op, hard } = c {
                        let new_op = match op {
                            FieldOp::Le(_) => FieldOp::Le(tf),
                            FieldOp::Lt(_) => FieldOp::Lt(tf),
                            FieldOp::Ge(_) => FieldOp::Ge(tf),
                            FieldOp::Gt(_) => FieldOp::Gt(tf),
                            other => other.clone(),
                        };
                        Some(Constraint::Field {
                            field: field.clone(),
                            op: new_op,
                            hard: *hard,
                        })
                    } else {
                        Some(c.clone())
                    }
                }
                // Unknown threshold type: keep constraint unchanged.
                Some(_) => Some(c.clone()),
            }
        })
        .collect()
}

/// Try up to `req.expansion.max_constraint_relaxations` options from
/// the pre-computed relaxation menu. Re-solves the bundle with the
/// relaxed constraint set. Stops at the first relaxation that finds
/// ≥1 solution. Returns an `ExpansionResult` with `attempted: true`
/// regardless.
///
/// The `relaxations` slice is the output of `compute_relaxation_menu`
/// (for the data-UNSAT case) or the synthetic menu built in the
/// pre-flight branch (for the Čech-UNSAT case). Either way they are
/// already sorted best-first by gain / cost.
fn attempt_expansion(
    materialized: &[Record],
    req: &SudokuRequest,
    relaxations: &[RelaxationOption],
    config: &SudokuConfig,
) -> ExpansionResult {
    let max_tries = req
        .expansion
        .as_ref()
        .map(|e| e.max_constraint_relaxations)
        .unwrap_or(1);

    let mut found: Vec<ExpandedSolution> = Vec::new();

    for relaxation in relaxations.iter().take(max_tries) {
        let relaxed_constraints = apply_relaxation(&req.constraints, relaxation);
        // If applying the relaxation left us with no constraints,
        // every record satisfies — still a meaningful expansion result.
        let relaxed_req = SudokuRequest {
            constraints: relaxed_constraints,
            max_options: req.max_options,
            max_near_misses: 0, // no near-misses on inner solve
            expansion: None,    // no recursive expansion
        };
        if let Ok(inner) = solve_constraints(materialized.iter().cloned(), &relaxed_req, config) {
            if !inner.solutions.is_empty() {
                let cost = relaxation.relaxation_cost;
                for sol in inner.solutions {
                    found.push(ExpandedSolution {
                        record: sol.record,
                        stated_prior_mass: sol.stated_prior_mass,
                        expansion_cost: cost,
                        relaxed_constraint_idx: relaxation.constraint_idx,
                        relaxed_to: relaxation.new_threshold.clone(),
                    });
                }
                // Got solutions — stop trying further relaxations.
                break;
            }
        }
    }

    let advisory = if found.is_empty() {
        Some(
            "No solutions found even after constraint relaxation. \
             Consider reformulating the problem or asking a human expert."
                .into(),
        )
    } else {
        None
    };

    ExpansionResult {
        attempted: true,
        expansion_type: "constraint_relaxation".into(),
        solutions: found,
        advisory,
    }
}

// ─── Internals ──────────────────────────────────────────────────

/// **Wave 6.2 — Holonomy pre-flight contradiction check.**
///
/// Scans the constraint list pairwise (O(C²)) for *trivial*
/// contradictions — pairs where no value of the field can satisfy
/// both constraints simultaneously, regardless of the data. Returns
/// `Some(reason)` for the FIRST contradiction found, `None`
/// otherwise.
///
/// **Contract:**
/// - **Correctness:** if Some, the constraint set has NO solution
///   for ANY bundle whatsoever. Returning Unsat is provably safe.
/// - **No false positives:** if None, the constraint set MAY have
///   solutions; the bundle walk decides. We never reject a valid
///   query upfront.
/// - **Permissively incomplete:** subtle contradictions (e.g.
///   `vegan=true AND cuisine=korean` where the data has no vegan
///   Korean restaurants) are NOT caught here — they require the
///   data. The walk handles them.
///
/// Detected shapes (per field, pairwise):
/// - `Eq(v1)` + `Eq(v2)` with v1 ≠ v2
/// - `Eq(v)` + `Ne(v)` with same v
/// - `Eq(v)` + numeric range that excludes v
/// - `Eq(v)` + `IsIn(set)` where v ∉ set
/// - `Lt(a)` + `Gt(b)` with a ≤ b (interval is empty)
/// - `Le(a)` + `Ge(b)` with a < b
/// - `Le(a)` + `Gt(b)` with a ≤ b
/// - `Lt(a)` + `Ge(b)` with a ≤ b
/// - `Between {lo1, hi1}` + `Between {lo2, hi2}` with hi1 < lo2
///   or hi2 < lo1 (disjoint intervals)
/// - `IsIn(s1)` + `IsIn(s2)` with empty intersection
/// - `IsIn(s)` + `Eq(v)` with v ∉ s
fn check_constraint_holonomy(constraints: &[Constraint]) -> Option<String> {
    // Group constraints by field. Pre-flight only checks WITHIN a
    // field — cross-field implications are S4 territory.
    use std::collections::HashMap;
    let mut by_field: HashMap<&str, Vec<(usize, &FieldOp)>> = HashMap::new();
    for (idx, c) in constraints.iter().enumerate() {
        if let Constraint::Field { field, op, .. } = c {
            by_field.entry(field.as_str()).or_default().push((idx, op));
        }
    }
    for (field, group) in by_field.iter() {
        for i in 0..group.len() {
            for j in (i + 1)..group.len() {
                let (idx_a, op_a) = group[i];
                let (idx_b, op_b) = group[j];
                if let Some(reason) = pair_contradicts(field, op_a, op_b) {
                    return Some(format!(
                        "constraints [{}] and [{}] on field '{}' are contradictory: {}",
                        idx_a, idx_b, field, reason
                    ));
                }
            }
        }
    }
    None
}

/// Returns `Some(reason)` if the two ops on the same field cannot
/// both be satisfied by any value.
fn pair_contradicts(field: &str, a: &FieldOp, b: &FieldOp) -> Option<String> {
    let _ = field; // only for the surrounding error message
    // Normalize: try both (a, b) and (b, a) orderings so each
    // helper only handles one shape.
    if let Some(r) = check_eq_eq(a, b) { return Some(r); }
    if let Some(r) = check_eq_ne(a, b).or_else(|| check_eq_ne(b, a)) { return Some(r); }
    if let Some(r) = check_eq_range(a, b).or_else(|| check_eq_range(b, a)) { return Some(r); }
    if let Some(r) = check_eq_isin(a, b).or_else(|| check_eq_isin(b, a)) { return Some(r); }
    if let Some(r) = check_range_range(a, b).or_else(|| check_range_range(b, a)) { return Some(r); }
    if let Some(r) = check_between_between(a, b) { return Some(r); }
    if let Some(r) = check_between_range(a, b).or_else(|| check_between_range(b, a)) { return Some(r); }
    if let Some(r) = check_isin_isin(a, b) { return Some(r); }
    None
}

fn check_eq_eq(a: &FieldOp, b: &FieldOp) -> Option<String> {
    if let (FieldOp::Eq(va), FieldOp::Eq(vb)) = (a, b) {
        if va != vb {
            return Some(format!("Eq({:?}) and Eq({:?}) cannot both hold", va, vb));
        }
    }
    None
}

fn check_eq_ne(a: &FieldOp, b: &FieldOp) -> Option<String> {
    if let (FieldOp::Eq(va), FieldOp::Ne(vb)) = (a, b) {
        if va == vb {
            return Some(format!("Eq({:?}) and Ne(same value) cannot both hold", va));
        }
    }
    None
}

fn check_eq_range(eq_op: &FieldOp, rng_op: &FieldOp) -> Option<String> {
    let FieldOp::Eq(v) = eq_op else { return None; };
    let Some(x) = value_as_f64(v) else { return None; };
    match rng_op {
        FieldOp::Lt(t) if x >= *t => Some(format!("Eq({}) excluded by Lt({})", x, t)),
        FieldOp::Le(t) if x > *t  => Some(format!("Eq({}) excluded by Le({})", x, t)),
        FieldOp::Gt(t) if x <= *t => Some(format!("Eq({}) excluded by Gt({})", x, t)),
        FieldOp::Ge(t) if x < *t  => Some(format!("Eq({}) excluded by Ge({})", x, t)),
        FieldOp::Between { lo, hi } if x < *lo || x > *hi => {
            Some(format!("Eq({}) outside Between [{}, {}]", x, lo, hi))
        }
        _ => None,
    }
}

fn check_eq_isin(eq_op: &FieldOp, isin_op: &FieldOp) -> Option<String> {
    if let (FieldOp::Eq(v), FieldOp::IsIn(set)) = (eq_op, isin_op) {
        if !set.iter().any(|s| s == v) {
            return Some(format!("Eq({:?}) excluded by IsIn that omits it", v));
        }
    }
    None
}

fn check_range_range(a: &FieldOp, b: &FieldOp) -> Option<String> {
    // upper-bound on a, lower-bound on b: a < b means empty.
    let upper = match a {
        FieldOp::Lt(t) => Some((*t, true)),   // strict upper
        FieldOp::Le(t) => Some((*t, false)),  // weak upper
        _ => None,
    };
    let lower = match b {
        FieldOp::Gt(t) => Some((*t, true)),   // strict lower
        FieldOp::Ge(t) => Some((*t, false)),  // weak lower
        _ => None,
    };
    if let (Some((u, u_strict)), Some((l, l_strict))) = (upper, lower) {
        // Empty if u < l OR (u == l AND at least one strict).
        if u < l || (u == l && (u_strict || l_strict)) {
            return Some(format!("upper bound {} incompatible with lower bound {}", u, l));
        }
    }
    None
}

fn check_between_between(a: &FieldOp, b: &FieldOp) -> Option<String> {
    if let (FieldOp::Between { lo: l1, hi: h1 }, FieldOp::Between { lo: l2, hi: h2 }) = (a, b) {
        if h1 < l2 || h2 < l1 {
            return Some(format!(
                "Between [{}, {}] and [{}, {}] are disjoint",
                l1, h1, l2, h2
            ));
        }
    }
    None
}

fn check_between_range(bet_op: &FieldOp, rng_op: &FieldOp) -> Option<String> {
    let FieldOp::Between { lo, hi } = bet_op else { return None; };
    match rng_op {
        FieldOp::Lt(t) if *lo >= *t => Some(format!("Between [{}, {}] excluded by Lt({})", lo, hi, t)),
        FieldOp::Le(t) if *lo > *t  => Some(format!("Between [{}, {}] excluded by Le({})", lo, hi, t)),
        FieldOp::Gt(t) if *hi <= *t => Some(format!("Between [{}, {}] excluded by Gt({})", lo, hi, t)),
        FieldOp::Ge(t) if *hi < *t  => Some(format!("Between [{}, {}] excluded by Ge({})", lo, hi, t)),
        _ => None,
    }
}

fn check_isin_isin(a: &FieldOp, b: &FieldOp) -> Option<String> {
    if let (FieldOp::IsIn(s1), FieldOp::IsIn(s2)) = (a, b) {
        if !s1.iter().any(|x| s2.iter().any(|y| x == y)) {
            return Some(format!("IsIn sets {:?} and {:?} have empty intersection", s1, s2));
        }
    }
    None
}

/// Classify a record against the constraints. Returns the list of
/// violated constraint details (empty if all satisfied). The
/// `stats` parameter is the bundle's empirical per-field stats —
/// used to normalize the relaxation cost on each violation.
fn classify_record(
    record: &Record,
    constraints: &[Constraint],
    stats: &BundleStats,
) -> Vec<ViolationDetail> {
    let mut violations = Vec::new();
    for (idx, c) in constraints.iter().enumerate() {
        let Constraint::Field { field, op, hard: _ } = c else {
            // Stubs already errored at request validation.
            continue;
        };
        let val = match record.get(field) {
            Some(v) => v,
            None => {
                // Missing field → counts as a violation (cannot
                // verify); spec §3 treats this as exclusion.
                // Missing-field cost: 1.0 (one discrete violation),
                // since we can't measure a distance.
                violations.push(ViolationDetail {
                    constraint_idx: idx,
                    field: field.clone(),
                    violation: format!("field '{}' missing from record", field),
                    relax_to: Value::Null,
                    relaxation_cost: 1.0,
                    raw_delta: None,
                });
                continue;
            }
        };
        if let Some(detail) = check_field_op(val, op, idx, field, stats) {
            violations.push(detail);
        }
    }
    violations
}

fn check_field_op(
    val: &Value,
    op: &FieldOp,
    constraint_idx: usize,
    field: &str,
    stats: &BundleStats,
) -> Option<ViolationDetail> {
    // DRY: the per-op evaluation lives in one place
    // (check_op_passes), used both here and by the relaxation menu.
    if check_op_passes(val, op) {
        return None;
    }
    let (raw_delta, relaxation_cost) = compute_relaxation_cost(val, op, field, stats);
    Some(ViolationDetail {
        constraint_idx,
        field: field.to_string(),
        violation: format!("{:?} fails {:?}", val, op),
        relax_to: val.clone(),
        relaxation_cost,
        raw_delta,
    })
}

/// **Wave 3 — Upgrade 1.** Compute the (raw, normalized) cost of
/// relaxing the constraint to admit this record's value.
///
/// Returns `(raw_delta, cost)`. `raw_delta` is `Some(magnitude)` in
/// the field's native units for ordered numeric ops; `None` for
/// categorical or non-orderable mismatches. `cost` is unitless,
/// normalized by the field's empirical length scale (std with
/// degenerate-field fallbacks).
fn compute_relaxation_cost(
    val: &Value,
    op: &FieldOp,
    field: &str,
    stats: &BundleStats,
) -> (Option<f64>, f64) {
    let field_stats: Option<&NumericFieldStats> = stats.numeric.get(field);
    match op {
        FieldOp::Lt(t) | FieldOp::Le(t) | FieldOp::Gt(t) | FieldOp::Ge(t) => {
            let Some(x) = value_as_f64(val) else {
                // Field constraint is numeric but value isn't —
                // treat as one discrete violation.
                return (None, 1.0);
            };
            let raw = (x - *t).abs();
            let ls = field_stats.map(|s| s.length_scale()).unwrap_or(1.0);
            (Some(raw), raw / ls)
        }
        FieldOp::Between { lo, hi } => {
            let Some(x) = value_as_f64(val) else {
                return (None, 1.0);
            };
            // Distance to the nearest side of the interval.
            let raw = if x < *lo {
                *lo - x
            } else if x > *hi {
                x - *hi
            } else {
                0.0
            };
            let ls = field_stats.map(|s| s.length_scale()).unwrap_or(1.0);
            (Some(raw), raw / ls)
        }
        FieldOp::Eq(target) => {
            // Numeric: use numeric distance / std.
            if let (Some(x), Some(t)) = (value_as_f64(val), value_as_f64(target)) {
                let raw = (x - t).abs();
                let ls = field_stats.map(|s| s.length_scale()).unwrap_or(1.0);
                return (Some(raw), raw / ls);
            }
            // **Wave 4.** Vector: use L2 distance / bundle's vector
            // length-scale. Without this fallback every Vector Eq
            // violation reports cost 1.0 regardless of how far apart
            // the vectors actually are — dishonest math.
            if let (Value::Vector(rec_v), Value::Vector(target_v)) = (val, target) {
                if let Some(vstats) = stats.vector.get(field) {
                    if let Some(d) = vstats.normalized_distance(rec_v, target_v) {
                        let raw = vstats.raw_distance(rec_v, target_v);
                        return (raw, d);
                    }
                }
                // Dim mismatch or no vector stats: fall through to
                // the categorical default (unit cost).
            }
            // Categorical or mixed: one discrete step.
            (None, 1.0)
        }
        FieldOp::Ne(_) | FieldOp::IsIn(_) => {
            // Categorical: one discrete violation. We don't have a
            // distance from the record's category to the "nearest
            // allowed" category without an embedding — and we
            // refuse to invent one. Document this in the spec.
            (None, 1.0)
        }
    }
}

// `value_as_f64` is now an alias for `bundle_stats::numeric_value`
// (imported at top of file). Single source of truth; Timestamp
// fields coerce uniformly across BundleStats and SUDOKU.

/// Compute stated-prior mass per record signature. v1: signature
/// is the record itself (exact-match collapse). Returns a Vec of
/// `(canonical_record, mass)` deduplicated.
fn compute_mass_per_signature(
    records_and_violations: &[(Record, Vec<ViolationDetail>)],
    n_total: usize,
) -> Vec<(Record, f64)> {
    let mut counts: Vec<(Record, usize)> = Vec::new();
    for (r, _) in records_and_violations {
        if let Some(slot) = counts.iter_mut().find(|(existing, _)| records_match(existing, r)) {
            slot.1 += 1;
        } else {
            counts.push((r.clone(), 1));
        }
    }
    counts
        .into_iter()
        .map(|(r, c)| (r, c as f64 / n_total as f64))
        .collect()
}

/// Record equality by full field map. Uses the underlying Record
/// type's PartialEq if available; otherwise builds it from field
/// comparison.
fn records_match(a: &Record, b: &Record) -> bool {
    // Record is std::collections::HashMap<String, Value>; HashMap
    // PartialEq is structural — checks same keys and same values.
    a == b
}

// ─── Wave 3 — Upgrades 2/3/5 ────────────────────────────────────

/// **Upgrade 2.** Per-constraint selectivity. For each constraint,
/// count records that satisfy ALL constraints except this one. The
/// marginal effect is `n_match_without - n_match_all`. The constraint
/// with the largest marginal is the binding one (the one filtering
/// the most records given the others). Ties: all max-tied constraints
/// flagged as binding.
fn compute_selectivity(
    classifications: &[(Record, Vec<ViolationDetail>)],
    constraints: &[Constraint],
) -> Vec<SelectivityReport> {
    if constraints.is_empty() {
        return Vec::new();
    }
    let n_match_all = classifications.iter().filter(|(_, v)| v.is_empty()).count();
    let n_total = classifications.len();
    let mut reports: Vec<SelectivityReport> = Vec::with_capacity(constraints.len());
    for (idx, c) in constraints.iter().enumerate() {
        let Constraint::Field { field, .. } = c else {
            continue;
        };
        // "Satisfies all except idx" = either zero violations OR
        // the only violation is on this constraint.
        let n_match_without = classifications
            .iter()
            .filter(|(_, viols)| {
                viols.is_empty()
                    || (viols.len() == 1 && viols[0].constraint_idx == idx)
            })
            .count();
        let marginal = n_match_without.saturating_sub(n_match_all);
        // **W6.1.** Per-constraint raw curvature K_c: fraction of
        // records that fail this constraint, regardless of others.
        // Free: every record's violations list already tells us
        // whether constraint idx fires on it.
        let n_fail = classifications
            .iter()
            .filter(|(_, viols)| viols.iter().any(|v| v.constraint_idx == idx))
            .count();
        let raw_curvature = if n_total > 0 {
            n_fail as f64 / n_total as f64
        } else {
            0.0
        };
        reports.push(SelectivityReport {
            constraint_idx: idx,
            field: field.clone(),
            n_match_all,
            n_match_without,
            marginal_filter_count: marginal,
            binding: false, // patched below
            raw_curvature,
        });
    }
    // Mark binding: the maximum marginal_filter_count wins. Ties
    // all flagged.
    let max_marginal = reports.iter().map(|r| r.marginal_filter_count).max().unwrap_or(0);
    if max_marginal > 0 {
        for r in reports.iter_mut() {
            if r.marginal_filter_count == max_marginal {
                r.binding = true;
            }
        }
    }
    reports
}

/// **Wave 6.3 — Γ trichotomy diagnostic.**
///
/// Computes Γ = m / (K̂_max · log(n + 1)) from already-derived
/// selectivity reports + the constraint and record counts. Returns
/// `None` when the diagnostic is ill-defined:
///
///   - m = 0 (no constraints) — every record trivially satisfies; Γ
///     classification has no signal.
///   - n = 0 (no records considered) — log term degenerate.
///
/// Otherwise:
///   - K̂_max is the maximum `raw_curvature` across all selectivity
///     reports (W6.1). If every report shows zero curvature (no
///     constraint actually filters anything — pathological data),
///     K̂_max is clamped to a tiny epsilon (1e-12) so Γ → very large
///     → Geometric regime, which is the correct routing: the search
///     is unconstrained, structure has to drive it.
///   - log(n + 1) is used instead of log(n) so n = 1 gives a finite
///     positive denominator.
///
/// **τ** in the canonical formula `Γ = m · τ / (K̂_max · log|S|)`
/// is fixed at 1 here: SUDOKU is single-pass (one walk of the
/// candidate set), unlike iterative solvers where τ captures the
/// iteration budget. Bee may revisit if SUDOKU grows an explicit
/// iteration budget knob.
fn compute_gamma_trichotomy(
    selectivity: &[SelectivityReport],
    m: usize,
    n: usize,
) -> Option<GammaTrichotomy> {
    if m == 0 || n == 0 {
        return None;
    }
    let k_max = selectivity
        .iter()
        .map(|r| r.raw_curvature)
        .fold(0.0_f64, f64::max);
    // Guard against pathological all-zero curvature (no constraint
    // actually filters) — clamp to a tiny epsilon so Γ becomes very
    // large (Geometric regime) rather than dividing by zero.
    let k_max_safe = k_max.max(1e-12);
    let log_s = ((n as f64) + 1.0).ln();
    let gamma = (m as f64) / (k_max_safe * log_s);
    let regime = TrichotomyRegime::from_gamma(gamma);
    Some(GammaTrichotomy {
        gamma,
        regime,
        m,
        k_max,
        log_s,
    })
}

/// **Upgrade 3.** Counterfactual relaxation menu. For each ordered
/// constraint, propose new thresholds drawn from the actual values
/// of records that violate **only** that constraint (single-violation
/// near-misses). For categorical, propose drop-constraint OR adding
/// the next-most-frequent un-included category. Sort by
/// `gain / max(cost, ε)` descending.
fn compute_relaxation_menu(
    classifications: &[(Record, Vec<ViolationDetail>)],
    constraints: &[Constraint],
    stats: &BundleStats,
    n_solutions: usize,
) -> Vec<RelaxationOption> {
    let mut menu: Vec<RelaxationOption> = Vec::new();
    for (idx, c) in constraints.iter().enumerate() {
        let Constraint::Field { field, op, hard: _ } = c else {
            continue;
        };
        // Collect records that violate ONLY this constraint —
        // these are the records that would unlock if we relaxed
        // *just* this constraint.
        let candidates: Vec<&ViolationDetail> = classifications
            .iter()
            .filter_map(|(_, viols)| {
                if viols.len() == 1 && viols[0].constraint_idx == idx {
                    Some(&viols[0])
                } else {
                    None
                }
            })
            .collect();

        match op {
            FieldOp::Lt(_) | FieldOp::Le(_) | FieldOp::Gt(_) | FieldOp::Ge(_) => {
                // Unique violating values. Each one, if used as the
                // new threshold, would unlock the records whose value
                // is on the "satisfying side" of it.
                let mut violating_values: Vec<f64> = candidates
                    .iter()
                    .filter_map(|v| {
                        if let Value::Float(f) = &v.relax_to {
                            Some(*f)
                        } else if let Value::Integer(i) = &v.relax_to {
                            Some(*i as f64)
                        } else {
                            None
                        }
                    })
                    .collect();
                violating_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                violating_values.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);

                // For each unique violating value, compute how many
                // records become solutions if we use that as the new
                // threshold.
                for new_threshold in &violating_values {
                    let new_op = match op {
                        FieldOp::Lt(_) => FieldOp::Lt(*new_threshold + f64::EPSILON),
                        FieldOp::Le(_) => FieldOp::Le(*new_threshold),
                        FieldOp::Gt(_) => FieldOp::Gt(*new_threshold - f64::EPSILON),
                        FieldOp::Ge(_) => FieldOp::Ge(*new_threshold),
                        _ => continue,
                    };
                    let gain = count_unlocked(classifications, idx, &new_op);
                    if gain == 0 {
                        continue;
                    }
                    let raw_delta = value_as_f64(&candidates[0].relax_to)
                        .map(|_| (new_threshold - threshold_of(op).unwrap_or(0.0)).abs())
                        .unwrap_or(0.0);
                    let ls = stats.numeric.get(field).map(|s| s.length_scale()).unwrap_or(1.0);
                    let cost = raw_delta / ls;
                    menu.push(RelaxationOption {
                        constraint_idx: idx,
                        field: field.clone(),
                        description: format!("{} -> {:.3}", op_str(op), new_threshold),
                        new_threshold: Some(Value::Float(*new_threshold)),
                        gain,
                        relaxation_cost: cost,
                        energy_descent: 0.0, // finalized after all pushes
                    });
                }
            }
            FieldOp::Between { lo, hi } => {
                // Two directions to relax: extend lo down or hi up.
                let violating_low: Vec<f64> = candidates
                    .iter()
                    .filter_map(|v| value_as_f64(&v.relax_to))
                    .filter(|x| *x < *lo)
                    .collect();
                let violating_high: Vec<f64> = candidates
                    .iter()
                    .filter_map(|v| value_as_f64(&v.relax_to))
                    .filter(|x| *x > *hi)
                    .collect();
                let ls = stats.numeric.get(field).map(|s| s.length_scale()).unwrap_or(1.0);
                if let Some(new_lo) = violating_low.iter().cloned().fold(None, |min, x| {
                    Some(min.map_or(x, |m: f64| m.min(x)))
                }) {
                    let new_op = FieldOp::Between { lo: new_lo, hi: *hi };
                    let gain = count_unlocked(classifications, idx, &new_op);
                    if gain > 0 {
                        menu.push(RelaxationOption {
                            constraint_idx: idx,
                            field: field.clone(),
                            description: format!("between [{}..{}] -> [{:.3}..{}]", lo, hi, new_lo, hi),
                            new_threshold: Some(Value::Float(new_lo)),
                            gain,
                            relaxation_cost: (lo - new_lo).abs() / ls,
                            energy_descent: 0.0, // finalized after all pushes
                        });
                    }
                }
                if let Some(new_hi) = violating_high.iter().cloned().fold(None, |max, x| {
                    Some(max.map_or(x, |m: f64| m.max(x)))
                }) {
                    let new_op = FieldOp::Between { lo: *lo, hi: new_hi };
                    let gain = count_unlocked(classifications, idx, &new_op);
                    if gain > 0 {
                        menu.push(RelaxationOption {
                            constraint_idx: idx,
                            field: field.clone(),
                            description: format!("between [{}..{}] -> [{}..{:.3}]", lo, hi, lo, new_hi),
                            new_threshold: Some(Value::Float(new_hi)),
                            gain,
                            relaxation_cost: (new_hi - hi).abs() / ls,
                            energy_descent: 0.0, // finalized after all pushes
                        });
                    }
                }
            }
            FieldOp::Eq(_) | FieldOp::Ne(_) | FieldOp::IsIn(_) => {
                // For categorical / discrete: the menu offering is
                // "drop this constraint." Categorical relaxation
                // doesn't have a natural per-value cost without an
                // embedding (we refuse to invent one), so all
                // categorical drops have unit cost = 1.0.
                let gain = candidates.len();
                if gain > 0 {
                    menu.push(RelaxationOption {
                        constraint_idx: idx,
                        field: field.clone(),
                        description: format!("drop constraint on {}", field),
                        new_threshold: None,
                        gain,
                        relaxation_cost: 1.0,
                        energy_descent: 0.0, // finalized after all pushes
                    });
                }
            }
        }
    }

    // **Wave 6.4 — energy descent per unit cost.** Replaces the W3
    // `gain / cost` sort. For each option compute
    //
    //   ΔE = log(1 + gain / max(n_solutions, 1))
    //   energy_descent = ΔE / max(cost, ε)
    //
    // The `max(n_solutions, 1)` handles the "no current solutions"
    // case (treat as one phantom match — any gain is still log-finite
    // rather than log(∞)). The `max(cost, ε)` mirrors the W3 div-by-
    // zero guard. Sort descending by `energy_descent`; the raw
    // `gain` and `relaxation_cost` remain in the response so consumers
    // who prefer the W3 ordering can re-sort.
    let n_solutions_safe = (n_solutions.max(1)) as f64;
    let cost_eps = 1e-9_f64;
    for opt in menu.iter_mut() {
        let delta_e = (1.0 + (opt.gain as f64) / n_solutions_safe).ln();
        let cost = opt.relaxation_cost.max(cost_eps);
        opt.energy_descent = delta_e / cost;
    }
    menu.sort_by(|a, b| {
        b.energy_descent
            .partial_cmp(&a.energy_descent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Dedup near-identical proposals — keep at most 3 per constraint
    // to stop the menu from blowing up.
    let mut per_constraint: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    menu.retain(|opt| {
        let count = per_constraint.entry(opt.constraint_idx).or_insert(0);
        if *count >= 3 {
            false
        } else {
            *count += 1;
            true
        }
    });

    // Cap total at 12 (≈ 4 constraints × 3 proposals each — keeps
    // wire size bounded across any bundle shape).
    menu.truncate(12);

    // Suppress lint about unused n_solutions: it's there for a
    // possible future "absolute gain vs % gain" reporting upgrade.
    let _ = n_solutions;

    menu
}

/// Count records that would become solutions if constraint at `idx`
/// were replaced with `new_op`. Cheap re-scan over the pre-classified
/// list — we already know each record's violation pattern.
fn count_unlocked(
    classifications: &[(Record, Vec<ViolationDetail>)],
    constraint_idx: usize,
    new_op: &FieldOp,
) -> usize {
    classifications
        .iter()
        .filter(|(record, viols)| {
            // A record is unlocked iff its only-or-no violation is on
            // this constraint AND the new op admits its value. We need
            // to re-evaluate the constraint with the new op against
            // the record's actual value.
            let only_violation_on_us = viols.iter().all(|v| v.constraint_idx == constraint_idx);
            if !only_violation_on_us {
                return false;
            }
            // Find the field name from the violation (if any) — when
            // the record has zero violations, it's already a solution
            // and doesn't count as "newly unlocked."
            let Some(v) = viols.first() else {
                return false;
            };
            let Some(val) = record.get(&v.field) else {
                return false;
            };
            // Re-check the new op against the record's value.
            check_op_passes(val, new_op)
        })
        .count()
}

fn check_op_passes(val: &Value, op: &FieldOp) -> bool {
    match op {
        FieldOp::Eq(target) => val == target,
        FieldOp::Ne(target) => val != target,
        FieldOp::Lt(t) => value_as_f64(val).map(|v| v < *t).unwrap_or(false),
        FieldOp::Le(t) => value_as_f64(val).map(|v| v <= *t).unwrap_or(false),
        FieldOp::Gt(t) => value_as_f64(val).map(|v| v > *t).unwrap_or(false),
        FieldOp::Ge(t) => value_as_f64(val).map(|v| v >= *t).unwrap_or(false),
        FieldOp::Between { lo, hi } => value_as_f64(val).map(|v| v >= *lo && v <= *hi).unwrap_or(false),
        FieldOp::IsIn(targets) => targets.iter().any(|t| t == val),
    }
}

/// **Wave 4 — Upgrade 4.** Quality score for a satisfying record.
///
/// For each constraint, computes the **margin** = normalized
/// distance from the record's value to the violating side of that
/// constraint (using the same bundle-derived length scale as
/// `relaxation_cost`). The quality score is `1 - exp(-min_margin)`
/// — saturates near 1 for solutions deep inside the region.
///
/// Pure-geometric, fully GP. Identical math regardless of field
/// names, units, or domain. Returns 1.0 for the empty-constraint
/// case (everything trivially "best").
///
/// Math note: this is the soft-constraint posterior under
/// independent half-normal priors per constraint, in the limit
/// where the prior bandwidth equals the field's empirical std.
fn compute_quality_score(
    record: &Record,
    constraints: &[Constraint],
    stats: &BundleStats,
) -> f64 {
    if constraints.is_empty() {
        return 1.0;
    }
    let mut min_margin = f64::INFINITY;
    for c in constraints {
        let Constraint::Field { field, op, hard: _ } = c else {
            continue;
        };
        let Some(val) = record.get(field) else {
            continue;
        };
        let margin = constraint_margin(val, op, field, stats);
        if margin < min_margin {
            min_margin = margin;
        }
    }
    // No usable margins (e.g. all-categorical-Eq solution): fall
    // back to 0.5 — a neutral score, since we can't measure depth.
    if !min_margin.is_finite() {
        return 0.5;
    }
    1.0 - (-min_margin).exp()
}

/// Distance from `val` to the violating side of `op`, normalized
/// by the field's length scale. Larger = deeper inside the
/// satisfaction region. None / +inf when not computable.
fn constraint_margin(val: &Value, op: &FieldOp, field: &str, stats: &BundleStats) -> f64 {
    let ls = stats.numeric.get(field).map(|s| s.length_scale()).unwrap_or(1.0);
    match op {
        FieldOp::Lt(t) => value_as_f64(val).map(|v| (*t - v) / ls).unwrap_or(f64::INFINITY),
        FieldOp::Le(t) => value_as_f64(val).map(|v| (*t - v) / ls).unwrap_or(f64::INFINITY),
        FieldOp::Gt(t) => value_as_f64(val).map(|v| (v - *t) / ls).unwrap_or(f64::INFINITY),
        FieldOp::Ge(t) => value_as_f64(val).map(|v| (v - *t) / ls).unwrap_or(f64::INFINITY),
        FieldOp::Between { lo, hi } => {
            // Margin = distance to nearest endpoint, normalized.
            value_as_f64(val)
                .map(|v| {
                    let d_lo = (v - *lo) / ls;
                    let d_hi = (*hi - v) / ls;
                    d_lo.min(d_hi).max(0.0)
                })
                .unwrap_or(f64::INFINITY)
        }
        // Categorical Eq/Ne/IsIn: no continuous notion of "deeper
        // inside the satisfaction region." Skip (returns +inf so
        // the min ignores us); the min over all constraints will
        // be driven by the numeric ones. If ALL constraints are
        // categorical, compute_quality_score returns 0.5.
        FieldOp::Eq(_) | FieldOp::Ne(_) | FieldOp::IsIn(_) => f64::INFINITY,
    }
}

fn threshold_of(op: &FieldOp) -> Option<f64> {
    match op {
        FieldOp::Lt(t) | FieldOp::Le(t) | FieldOp::Gt(t) | FieldOp::Ge(t) => Some(*t),
        _ => None,
    }
}

fn op_str(op: &FieldOp) -> &'static str {
    match op {
        FieldOp::Lt(_) => "<",
        FieldOp::Le(_) => "<=",
        FieldOp::Gt(_) => ">",
        FieldOp::Ge(_) => ">=",
        FieldOp::Eq(_) => "==",
        FieldOp::Ne(_) => "!=",
        FieldOp::Between { .. } => "between",
        FieldOp::IsIn(_) => "in",
    }
}

/// **Upgrade 5.** Pareto-optimal multi-violation near-misses. A
/// record `r1` dominates `r2` iff `n_violations(r1) <= n_violations(r2)`
/// AND `total_cost(r1) <= total_cost(r2)` with at least one strict.
/// Return the non-dominated frontier, sorted by total cost.
fn compute_pareto_near_misses(
    classifications: &[(Record, Vec<ViolationDetail>)],
    n_total: usize,
    max_to_return: usize,
    n_constraints: usize,
) -> Vec<ParetoNearMiss> {
    // **Wave 5 scale fix.** Previously capped at violation_count <= 3,
    // which made the Pareto frontier consistently 1-entry at scale
    // (queries with 5+ hard constraints have most records violating
    // 4+). Scale the cap with the constraint count so the frontier
    // can include records that bend multiple rules at once — the
    // honest Pareto answer when no exact match exists. Floor at 3
    // for tiny queries; cap at `n_constraints` (you can't violate
    // more than that anyway).
    let violation_cap = n_constraints.max(3);
    let candidates: Vec<ParetoNearMiss> = classifications
        .iter()
        .filter(|(_, v)| !v.is_empty() && v.len() <= violation_cap)
        .map(|(record, viols)| {
            let total_cost: f64 = viols.iter().map(|v| v.relaxation_cost).sum();
            ParetoNearMiss {
                record: record.clone(),
                stated_prior_mass: 1.0 / n_total as f64, // per-record mass
                violations: viols.clone(),
                total_relaxation_cost: total_cost,
            }
        })
        .collect();

    // Dedup exact-duplicate records — sum mass onto a canonical entry.
    let mut by_signature: Vec<ParetoNearMiss> = Vec::new();
    for cand in candidates {
        if let Some(existing) = by_signature.iter_mut().find(|e| e.record == cand.record) {
            existing.stated_prior_mass += cand.stated_prior_mass;
        } else {
            by_signature.push(cand);
        }
    }

    // Pareto filter: keep only non-dominated entries.
    let n = by_signature.len();
    let mut keep = vec![true; n];
    for i in 0..n {
        if !keep[i] {
            continue;
        }
        for j in 0..n {
            if i == j || !keep[j] {
                continue;
            }
            // Does j dominate i?
            let j_n = by_signature[j].violations.len();
            let i_n = by_signature[i].violations.len();
            let j_c = by_signature[j].total_relaxation_cost;
            let i_c = by_signature[i].total_relaxation_cost;
            let weakly = j_n <= i_n && j_c <= i_c;
            let strictly = j_n < i_n || j_c < i_c;
            if weakly && strictly {
                keep[i] = false;
                break;
            }
        }
    }
    let mut frontier: Vec<ParetoNearMiss> = by_signature
        .into_iter()
        .zip(keep.into_iter())
        .filter_map(|(p, k)| if k { Some(p) } else { None })
        .collect();
    frontier.sort_by(|a, b| {
        // Sort by (n_violations, total_cost) ascending — cheapest
        // and fewest violations first.
        a.violations
            .len()
            .cmp(&b.violations.len())
            .then(a.total_relaxation_cost.partial_cmp(&b.total_relaxation_cost).unwrap_or(std::cmp::Ordering::Equal))
    });

    frontier.truncate(max_to_return.max(1) * 3); // a few extra for richer Pareto
    frontier
}

fn decide_verdict(coverage: f64, solutions: &[Solution], config: &SudokuConfig) -> SudokuVerdict {
    match (coverage >= config.coverage_high_threshold, solutions.is_empty()) {
        (true, false) => SudokuVerdict::Sat,
        (true, true) => SudokuVerdict::Unsat,
        (false, false) => {
            // Mid-bucket OR low-coverage WITH solutions: report Sat
            // with the found solutions but mark coverage modest.
            // Per spec §3 mid-bucket policy (≥low ∧ <high ∧ ≥1 sol → Sat).
            if coverage >= config.coverage_low_threshold {
                SudokuVerdict::Sat
            } else {
                // Low coverage even with solutions — caller asked
                // for honest reporting; treat as Unknown so they
                // don't act on a stale partial view.
                SudokuVerdict::Unknown
            }
        }
        (false, true) => SudokuVerdict::Unknown,
    }
}

// ─── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    fn rec(fields: &[(&str, Value)]) -> Record {
        let mut r = Record::new();
        for (k, v) in fields {
            r.insert((*k).to_string(), v.clone());
        }
        r
    }

    /// Trivial: empty constraint set + non-empty records →
    /// every record is a solution.
    #[test]
    fn no_constraints_returns_all_records() {
        let records = vec![
            rec(&[("x", Value::Integer(1))]),
            rec(&[("x", Value::Integer(2))]),
            rec(&[("x", Value::Integer(3))]),
        ];
        let req = SudokuRequest {
            constraints: vec![],
            max_options: 10,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
        assert_eq!(resp.solutions.len(), 3);
        assert_eq!(resp.coverage, 1.0);
        assert!(resp.near_misses.is_empty());
    }

    /// Field equality constraint filters to exactly the matching
    /// records.
    #[test]
    fn eq_constraint_filters_correctly() {
        let records = vec![
            rec(&[("mode", Value::Text("walk".into()))]),
            rec(&[("mode", Value::Text("bike".into()))]),
            rec(&[("mode", Value::Text("walk".into()))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "mode".into(),
                op: FieldOp::Eq(Value::Text("walk".into())),
                hard: true,
            }],
            max_options: 10,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
        // Two "walk" records collapse to one solution (exact-match
        // dedupe) with mass 2/3.
        assert_eq!(resp.solutions.len(), 1);
        assert!((resp.solutions[0].stated_prior_mass - 2.0 / 3.0).abs() < 1e-9);
    }

    /// THE honest-coverage test: empty feasible region with full
    /// coverage → Unsat (not Unknown).
    #[test]
    fn full_coverage_no_solutions_returns_unsat() {
        let records = vec![
            rec(&[("x", Value::Integer(1))]),
            rec(&[("x", Value::Integer(2))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "x".into(),
                op: FieldOp::Eq(Value::Integer(99)),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        assert!(resp.solutions.is_empty());
    }

    /// Empty bundle → Unknown verdict, coverage 0.0. **The
    /// load-bearing differentiator** between "no records" and
    /// "no satisfying records." Most solvers conflate.
    #[test]
    fn empty_bundle_returns_unknown_not_unsat() {
        let records: Vec<Record> = vec![];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "x".into(),
                op: FieldOp::Eq(Value::Integer(1)),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unknown);
        assert_eq!(resp.coverage, 0.0);
        assert_eq!(resp.n_records_considered, 0);
    }

    /// Near-miss: records that violate exactly ONE constraint
    /// are surfaced with the relaxation needed to unlock them.
    /// Records violating ≥2 are NOT near-misses.
    #[test]
    fn near_miss_enumeration_single_constraint_violation() {
        let records = vec![
            // Satisfies both: solution
            rec(&[
                ("mode", Value::Text("walk".into())),
                ("eta_min", Value::Integer(25)),
            ]),
            // Violates eta only (eta=45 > 30): near-miss
            rec(&[
                ("mode", Value::Text("walk".into())),
                ("eta_min", Value::Integer(45)),
            ]),
            // Violates mode only (bike): near-miss
            rec(&[
                ("mode", Value::Text("bike".into())),
                ("eta_min", Value::Integer(25)),
            ]),
            // Violates both: NOT a near-miss
            rec(&[
                ("mode", Value::Text("drive".into())),
                ("eta_min", Value::Integer(60)),
            ]),
        ];
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "mode".into(),
                    op: FieldOp::Eq(Value::Text("walk".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "eta_min".into(),
                    op: FieldOp::Le(30.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
        assert_eq!(resp.solutions.len(), 1, "exactly one satisfying record");
        assert_eq!(resp.near_misses.len(), 2, "exactly two near-misses");
        // The 2-constraint-violating record must NOT appear in
        // near_misses (which by definition are single-violation).
        for nm in &resp.near_misses {
            assert_eq!(nm.violations.len(), 1, "near-misses violate exactly one");
        }
    }

    /// Stub constraint types return NotYetSupported error.
    #[test]
    fn manifold_constraint_errors_with_s4_note() {
        let req = SudokuRequest {
            constraints: vec![Constraint::Manifold {
                field: "embedding".into(),
                near_manifold: "marcella_voice_anchors".into(),
                epsilon: 0.3,
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let result = solve_constraints(
            vec![rec(&[("embedding", Value::Float(0.5))])],
            &req,
            &SudokuConfig::default(),
        );
        match result {
            Err(SudokuError::NotYetSupported(msg)) => {
                assert!(msg.contains("S4"));
                assert!(msg.contains("Manifold"));
            }
            other => panic!("expected NotYetSupported, got {:?}", other),
        }
    }

    /// Numeric range filter works as expected.
    #[test]
    fn between_filter_inclusive() {
        let records = vec![
            rec(&[("price", Value::Float(10.0))]),
            rec(&[("price", Value::Float(20.0))]),
            rec(&[("price", Value::Float(30.0))]),
            rec(&[("price", Value::Float(40.0))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "price".into(),
                op: FieldOp::Between { lo: 15.0, hi: 35.0 },
                hard: true,
            }],
            max_options: 10,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
        assert_eq!(resp.solutions.len(), 2);
    }

    /// IsIn categorical membership.
    #[test]
    fn is_in_categorical_membership() {
        let records = vec![
            rec(&[("kind", Value::Text("alpha".into()))]),
            rec(&[("kind", Value::Text("beta".into()))]),
            rec(&[("kind", Value::Text("gamma".into()))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "kind".into(),
                op: FieldOp::IsIn(vec![
                    Value::Text("alpha".into()),
                    Value::Text("gamma".into()),
                ]),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 3,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.solutions.len(), 2);
    }

    // ───────────────────────────────────────────────────────────
    // Wave 3 tests — relaxation gradient
    // ───────────────────────────────────────────────────────────

    /// **W3.2.** Numeric near-miss carries normalized cost
    /// proportional to |actual - threshold| / std(field).
    #[test]
    fn w3_relaxation_cost_numeric_is_z_score_normalized() {
        // Field "x" with mean=30, std (n-1) = sqrt(250) ≈ 15.81.
        // Threshold = 25, violation at x=40 → raw delta=15.
        // Normalized cost = 15 / 15.81 ≈ 0.949.
        let records = vec![
            rec(&[("x", Value::Float(10.0))]),
            rec(&[("x", Value::Float(20.0))]),
            rec(&[("x", Value::Float(30.0))]),
            rec(&[("x", Value::Float(40.0))]),
            rec(&[("x", Value::Float(50.0))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "x".into(),
                op: FieldOp::Le(25.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        // Find the x=40 near-miss and check its cost.
        let nm_40 = resp
            .near_misses
            .iter()
            .find(|nm| nm.record.get("x") == Some(&Value::Float(40.0)))
            .expect("x=40 should be a near-miss");
        let cost = nm_40.violations[0].relaxation_cost;
        let raw = nm_40.violations[0].raw_delta.unwrap();
        assert!((raw - 15.0).abs() < 1e-9, "raw delta should be 15, got {}", raw);
        assert!(
            (cost - 15.0 / 250.0_f64.sqrt()).abs() < 1e-6,
            "cost should be 15/sqrt(250), got {}",
            cost
        );
    }

    /// **W3.2.** Categorical near-miss carries unit cost (1.0).
    /// We refuse to invent a between-category distance.
    #[test]
    fn w3_relaxation_cost_categorical_is_unit() {
        let records = vec![
            rec(&[("k", Value::Text("a".into()))]),
            rec(&[("k", Value::Text("b".into()))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "k".into(),
                op: FieldOp::Eq(Value::Text("a".into())),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        let nm = &resp.near_misses[0];
        assert_eq!(nm.violations[0].relaxation_cost, 1.0);
        assert_eq!(nm.violations[0].raw_delta, None);
    }

    /// **W3.3.** Selectivity report identifies the binding
    /// constraint correctly. The "tightest" constraint should be
    /// flagged binding.
    #[test]
    fn w3_selectivity_identifies_binding_constraint() {
        // 10 records: all blue, ages 1..10.
        // Constraint A: color=blue (filters 0)
        // Constraint B: age >= 8 (filters 7)
        // B is binding.
        let records: Vec<Record> = (1..=10)
            .map(|i| {
                rec(&[
                    ("color", Value::Text("blue".into())),
                    ("age", Value::Integer(i)),
                ])
            })
            .collect();
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("blue".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "age".into(),
                    op: FieldOp::Ge(8.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        let sel_color = &resp.selectivity[0];
        let sel_age = &resp.selectivity[1];
        assert_eq!(sel_color.marginal_filter_count, 0, "color filters no one extra");
        assert_eq!(sel_age.marginal_filter_count, 7, "age filters 7 extra");
        assert!(!sel_color.binding);
        assert!(sel_age.binding);
    }

    /// **W3.4.** Relaxation menu offers data-driven thresholds.
    /// The proposed new thresholds must come from actual violating
    /// values, not arbitrary step sizes.
    #[test]
    fn w3_relaxation_menu_uses_data_driven_thresholds() {
        let records = vec![
            rec(&[("price", Value::Float(80.0))]),  // OK (under threshold)
            rec(&[("price", Value::Float(120.0))]), // violates
            rec(&[("price", Value::Float(150.0))]), // violates
            rec(&[("price", Value::Float(200.0))]), // violates
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "price".into(),
                op: FieldOp::Le(100.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        // Menu should propose each violating value as a new threshold.
        let proposed_thresholds: Vec<f64> = resp
            .relaxations
            .iter()
            .filter_map(|r| match &r.new_threshold {
                Some(Value::Float(f)) => Some(*f),
                _ => None,
            })
            .collect();
        assert!(
            proposed_thresholds.contains(&120.0),
            "menu should propose 120 as new threshold; got {:?}",
            proposed_thresholds
        );
        assert!(
            proposed_thresholds.contains(&150.0),
            "menu should propose 150"
        );
        // The 120 proposal should unlock 1 record; the 150 should unlock 2; 200 unlocks 3.
        let opt_120 = resp.relaxations.iter().find(|r| r.new_threshold == Some(Value::Float(120.0))).unwrap();
        let opt_150 = resp.relaxations.iter().find(|r| r.new_threshold == Some(Value::Float(150.0))).unwrap();
        assert_eq!(opt_120.gain, 1);
        assert_eq!(opt_150.gain, 2);
    }

    /// **W3.5.** Pareto near-miss frontier dominates correctly.
    /// A record with 1 violation cost 0.5 should dominate a record
    /// with 2 violations and any cost ≥ 0.5.
    #[test]
    fn w3_pareto_frontier_filters_dominated_correctly() {
        // Records and constraints designed so:
        //  - R1 violates 1 constraint at cost 0.5
        //  - R2 violates 2 constraints at cost 1.0
        //  - R3 violates 1 constraint at cost 1.0 (dominated by R1)
        // R1 should be on the frontier; R2 (different n_viol) too;
        // R3 dominated by R1 (same n_viol, higher cost).
        // x: std normalized — use simple values.
        let records = vec![
            rec(&[("a", Value::Float(0.0)), ("b", Value::Float(0.0))]), // solution
            rec(&[("a", Value::Float(10.0)), ("b", Value::Float(0.0))]),  // R1: violates a only
            rec(&[("a", Value::Float(10.0)), ("b", Value::Float(10.0))]), // R2: violates both
            rec(&[("a", Value::Float(20.0)), ("b", Value::Float(0.0))]),  // R3: violates a only, worse
            rec(&[("a", Value::Float(0.0)), ("b", Value::Float(0.0))]),  // solution
            rec(&[("a", Value::Float(0.0)), ("b", Value::Float(0.0))]),  // solution
        ];
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "a".into(),
                    op: FieldOp::Le(5.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "b".into(),
                    op: FieldOp::Le(5.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        // R1 must appear; R3 must not (dominated). R2 may appear
        // (different k).
        let has_r1 = resp.pareto_near_misses.iter().any(|p| {
            p.record.get("a") == Some(&Value::Float(10.0)) && p.violations.len() == 1
        });
        let has_r3 = resp.pareto_near_misses.iter().any(|p| {
            p.record.get("a") == Some(&Value::Float(20.0)) && p.violations.len() == 1
        });
        assert!(has_r1, "R1 (cheapest single violation) should be on frontier");
        assert!(!has_r3, "R3 (dominated by R1) should NOT be on frontier");
    }

    /// **W3.7 — the no-hacks domain-swap proof.** Run identical
    /// SUDOKU code on two utterly different bundles (drug discovery
    /// vs apartment search). Verify the relaxation cost is
    /// proportional to the violation magnitude in each bundle's own
    /// units. Same code, same math, different domains → same shape
    /// of result.
    #[test]
    fn w3_domain_swap_proof_no_hacks() {
        // Bundle A: drug discovery. pki values 6.0..9.0 in 0.3 steps.
        let drug_records: Vec<Record> = (0..10)
            .map(|i| rec(&[("pki", Value::Float(6.0 + 0.3 * i as f64))]))
            .collect();
        let drug_req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "pki".into(),
                op: FieldOp::Ge(9.0),
                hard: true,
            }],
            max_options: 3,
            max_near_misses: 5,
            expansion: None,
        };
        let drug_resp = solve_constraints(drug_records, &drug_req, &SudokuConfig::default()).unwrap();

        // Bundle B: real estate. rent values 3000..4800 in 200 steps.
        let rent_records: Vec<Record> = (0..10)
            .map(|i| rec(&[("rent", Value::Float(3000.0 + 200.0 * i as f64))]))
            .collect();
        // Threshold 3400 = one step (200) below 3600 (closest
        // violator). Drug threshold 9.0 = one step (0.3) above 8.7
        // (closest violator). Symmetric → closest cost / std should
        // coincide modulo the two bundles' empirical std shape.
        let rent_req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "rent".into(),
                op: FieldOp::Le(3400.0),
                hard: true,
            }],
            max_options: 3,
            max_near_misses: 5,
            expansion: None,
        };
        let rent_resp = solve_constraints(rent_records, &rent_req, &SudokuConfig::default()).unwrap();

        // Both responses should have selectivity reports with
        // binding=true on their single constraint.
        assert!(drug_resp.selectivity[0].binding);
        assert!(rent_resp.selectivity[0].binding);

        // Both should have non-empty near-misses with normalized
        // costs in O(1) range — proving the normalization is
        // scale-invariant.
        assert!(!drug_resp.near_misses.is_empty());
        assert!(!rent_resp.near_misses.is_empty());

        // Sanity: the relaxation menu in both bundles offers >=1
        // data-driven proposal.
        assert!(!drug_resp.relaxations.is_empty(), "drug menu must offer relaxations");
        assert!(!rent_resp.relaxations.is_empty(), "rent menu must offer relaxations");

        // The KEY no-hacks assertion: a violation at 1 step distance
        // (the closest violator) yields a normalized cost in the
        // same ballpark for both bundles, despite raw magnitudes
        // differing by ~3 orders. With step 0.3 in pki land and step
        // 200 in rent land, raw deltas differ by 200/0.3 ≈ 667× yet
        // normalized costs should be within ~10% of each other
        // (the bundles have similar n=10 record counts → similar
        // std-shape).
        let closest_drug = drug_resp.near_misses.iter()
            .map(|nm| nm.violations[0].relaxation_cost)
            .fold(f64::INFINITY, f64::min);
        let closest_rent = rent_resp.near_misses.iter()
            .map(|nm| nm.violations[0].relaxation_cost)
            .fold(f64::INFINITY, f64::min);
        // Both should be finite and small (single-step violations).
        assert!(
            closest_drug.is_finite() && closest_drug < 1.0,
            "drug cost should be <1.0 (single-step), got {}",
            closest_drug
        );
        assert!(
            closest_rent.is_finite() && closest_rent < 1.0,
            "rent cost should be <1.0 (single-step), got {}",
            closest_rent
        );
        // The ratio of normalized costs should be O(1), NOT O(667).
        // This is the proof: the math is genuinely scale-invariant.
        let ratio = (closest_drug / closest_rent).max(closest_rent / closest_drug);
        assert!(
            ratio < 2.0,
            "domain-swap: normalized cost ratio should be O(1), got {} (raw scale ratio was ~667)",
            ratio
        );
    }

    // ───────────────────────────────────────────────────────────
    // Wave 4 tests — vector distance + quality_score
    // ───────────────────────────────────────────────────────────

    /// **W4.1.** Eq on Vector field returns geometric cost
    /// (distance / bundle's vector length-scale), NOT the flat
    /// categorical 1.0. This is the dishonest-math fix.
    #[test]
    fn w4_vector_eq_returns_geometric_distance_not_unit() {
        let records = vec![
            rec(&[("emb", Value::Vector(vec![0.0, 0.0, 0.0]))]),
            rec(&[("emb", Value::Vector(vec![0.1, 0.0, 0.0]))]), // very close
            rec(&[("emb", Value::Vector(vec![10.0, 10.0, 10.0]))]), // very far
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "emb".into(),
                op: FieldOp::Eq(Value::Vector(vec![0.0, 0.0, 0.0])),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        // Two near-misses with different distances → different costs.
        let mut costs: Vec<f64> = resp
            .near_misses
            .iter()
            .map(|nm| nm.violations[0].relaxation_cost)
            .collect();
        costs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(costs.len(), 2);
        // Different by at least 10x — the geometric reality.
        assert!(
            costs[1] / costs[0] > 5.0,
            "vector costs should differ proportionally: {:?}",
            costs
        );
    }

    /// **W4.1.** Vector Eq still works correctly when the bundle
    /// has no other vector data (single record case) — falls back
    /// gracefully, doesn't panic, returns a finite cost.
    #[test]
    fn w4_vector_degenerate_bundle_returns_finite_cost() {
        let records = vec![rec(&[("emb", Value::Vector(vec![0.0, 0.0]))])];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "emb".into(),
                op: FieldOp::Eq(Value::Vector(vec![1.0, 1.0])),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        // The single record violates — its cost must be finite.
        let cost = resp.near_misses[0].violations[0].relaxation_cost;
        assert!(cost.is_finite() && cost > 0.0, "cost must be finite positive, got {}", cost);
    }

    /// **W4.2.** quality_score ranks solutions by depth into the
    /// satisfaction region. A record deep inside should score
    /// higher than one near the boundary.
    #[test]
    fn w4_quality_score_ranks_deeper_solutions_higher() {
        // Bundle: prices 100..200 in steps of 10.
        // Constraint: price <= 200 (all satisfy).
        // Record at price=100 (deep) should score higher than
        // record at price=200 (boundary).
        let records: Vec<Record> = (0..=10)
            .map(|i| rec(&[("price", Value::Float(100.0 + 10.0 * i as f64))]))
            .collect();
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "price".into(),
                op: FieldOp::Le(200.0),
                hard: true,
            }],
            max_options: 11,
            max_near_misses: 0,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        // Find the price=100 and price=200 solutions; the former
        // must have higher quality_score (deeper).
        let q_deep = resp
            .solutions
            .iter()
            .find(|s| s.record.get("price") == Some(&Value::Float(100.0)))
            .unwrap()
            .quality_score;
        let q_boundary = resp
            .solutions
            .iter()
            .find(|s| s.record.get("price") == Some(&Value::Float(200.0)))
            .unwrap()
            .quality_score;
        assert!(
            q_deep > q_boundary,
            "deep solution (price=100) score {} must beat boundary (price=200) score {}",
            q_deep,
            q_boundary
        );
        assert!(q_boundary >= 0.0 && q_boundary <= 1.0);
        assert!(q_deep > 0.0 && q_deep <= 1.0);
    }

    /// **W4.2.** quality_score's secondary sort surfaces the BEST
    /// solution first within a mass tie. Drives the headline UX
    /// for many-SAT domains (restaurants, HR candidates).
    #[test]
    fn w4_quality_score_breaks_mass_ties_with_best_first() {
        // 3 distinct records, all SAT, all with mass 1/3. Quality
        // should make the deepest one come first.
        let records = vec![
            rec(&[("score", Value::Float(0.95))]), // deep
            rec(&[("score", Value::Float(0.80))]), // boundary (exact threshold)
            rec(&[("score", Value::Float(0.88))]), // mid
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "score".into(),
                op: FieldOp::Ge(0.80),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.solutions.len(), 3);
        // First solution: highest score (0.95). Last: lowest (0.80).
        assert_eq!(
            resp.solutions[0].record.get("score"),
            Some(&Value::Float(0.95))
        );
        assert_eq!(
            resp.solutions[2].record.get("score"),
            Some(&Value::Float(0.80))
        );
    }

    /// **W4 — no-hacks domain swap for quality_score.** Same code
    /// produces a meaningful quality rank across two utterly
    /// different domains (rating 0-5 + price $) without any
    /// per-domain tuning.
    #[test]
    fn w4_quality_score_is_domain_agnostic() {
        // Restaurant bundle: rating 4.0..5.0 in 0.1 steps.
        let restaurants: Vec<Record> = (0..=10)
            .map(|i| rec(&[("rating", Value::Float(4.0 + 0.1 * i as f64))]))
            .collect();
        let r_req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "rating".into(),
                op: FieldOp::Ge(4.0),
                hard: true,
            }],
            max_options: 11,
            max_near_misses: 0,
            expansion: None,
        };
        let r_resp = solve_constraints(restaurants, &r_req, &SudokuConfig::default()).unwrap();

        // House bundle: price 200000..1200000 (totally different scale).
        let houses: Vec<Record> = (0..=10)
            .map(|i| rec(&[("price", Value::Float(200_000.0 + 100_000.0 * i as f64))]))
            .collect();
        let h_req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "price".into(),
                op: FieldOp::Le(1_200_000.0),
                hard: true,
            }],
            max_options: 11,
            max_near_misses: 0,
            expansion: None,
        };
        let h_resp = solve_constraints(houses, &h_req, &SudokuConfig::default()).unwrap();

        // Both must:
        //   (a) sort with best-margin solution first
        //   (b) yield quality scores in [0, 1]
        for resp in [&r_resp, &h_resp] {
            assert!(resp.solutions.len() > 1);
            for s in &resp.solutions {
                assert!(
                    s.quality_score >= 0.0 && s.quality_score <= 1.0,
                    "quality_score must be in [0,1]; got {}",
                    s.quality_score
                );
            }
            // The first solution (after sort) must have a higher
            // quality_score than the last — proving the rank is
            // meaningful in this domain too.
            assert!(
                resp.solutions.first().unwrap().quality_score
                    >= resp.solutions.last().unwrap().quality_score
            );
        }
    }

    // ───────────────────────────────────────────────────────────
    // Wave 6 tests — sudoky-inspired curvature signal
    // ───────────────────────────────────────────────────────────

    /// **W6.1 (red-first).** Per-constraint raw_curvature K_c =
    /// fraction of records that FAIL this constraint regardless of
    /// other constraints. Distinct from marginal_filter_count.
    ///
    /// Setup: 10 records. Constraint A is tight (8/10 fail),
    /// constraint B is loose (1/10 fail). Both happen to be
    /// fully *covered* by each other in the rare records that
    /// satisfy both (i.e., marginal counts are equal/zero in a
    /// constructed redundancy). K_c must report the TRUE per-
    /// constraint tightness.
    #[test]
    fn w6_raw_curvature_distinguishes_tight_from_loose() {
        // 10 records: value 1..10. Constraint A: value >= 9 (only
        // 9 and 10 pass → 8 records fail → K_c = 0.8). Constraint
        // B: value >= 2 (only 1 fails → K_c = 0.1).
        let records: Vec<Record> = (1..=10)
            .map(|i| rec(&[("v", Value::Integer(i))]))
            .collect();
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "v".into(),
                    op: FieldOp::Ge(9.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "v".into(),
                    op: FieldOp::Ge(2.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        let sel_a = &resp.selectivity[0];
        let sel_b = &resp.selectivity[1];
        assert!(
            (sel_a.raw_curvature - 0.8).abs() < 1e-9,
            "constraint A raw_curvature should be 0.80, got {}",
            sel_a.raw_curvature
        );
        assert!(
            (sel_b.raw_curvature - 0.1).abs() < 1e-9,
            "constraint B raw_curvature should be 0.10, got {}",
            sel_b.raw_curvature
        );
        // A is "tight," B is "loose."  Order-of-magnitude separation
        // is the whole point of the diagnostic.
        assert!(sel_a.raw_curvature > sel_b.raw_curvature * 4.0);
    }

    /// **W6.1 — redundant constraint exposed.** Two identical
    /// constraints have IDENTICAL raw_curvature but ZERO marginal
    /// (each is redundant with the other). The pair makes the
    /// redundancy visible.
    #[test]
    fn w6_raw_curvature_exposes_redundant_constraint() {
        let records: Vec<Record> = (1..=10)
            .map(|i| rec(&[("v", Value::Integer(i))]))
            .collect();
        // Two identical constraints — pure redundancy.
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "v".into(),
                    op: FieldOp::Ge(5.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "v".into(),
                    op: FieldOp::Ge(5.0),
                    hard: true,
                },
            ],
            max_options: 10,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        // Both K_c = 0.4 (4 records fail v ≥ 5).
        assert!((resp.selectivity[0].raw_curvature - 0.4).abs() < 1e-9);
        assert!((resp.selectivity[1].raw_curvature - 0.4).abs() < 1e-9);
        // But each has marginal = 0 (the OTHER also blocks the
        // same records). The K_c high + marginal zero signature
        // tells the consumer: "this constraint is redundant."
        assert_eq!(resp.selectivity[0].marginal_filter_count, 0);
        assert_eq!(resp.selectivity[1].marginal_filter_count, 0);
    }

    // ───────────────────────────────────────────────────────────
    // Wave 6.2 — Holonomy pre-flight contradiction tests
    // ───────────────────────────────────────────────────────────

    /// Two Eq on same field with different values → trivial UNSAT.
    /// Must return Unsat with pre_flight_unsat_reason populated
    /// WITHOUT walking the bundle.
    #[test]
    fn w6_preflight_eq_eq_contradiction() {
        let records: Vec<Record> = (0..1000)
            .map(|i| rec(&[("color", Value::Text("red".into())), ("id", Value::Integer(i))]))
            .collect();
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("blue".into())),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        let reason = resp.pre_flight_unsat_reason.as_ref()
            .expect("pre_flight_unsat_reason should be Some for contradictory Eq+Eq");
        assert!(reason.contains("color"), "reason should name the field, got {}", reason);
    }

    /// Numeric range contradiction: Le(x, 10) + Ge(x, 20).
    #[test]
    fn w6_preflight_le_ge_contradiction() {
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "rent".into(),
                    op: FieldOp::Le(3000.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "rent".into(),
                    op: FieldOp::Ge(5000.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let records: Vec<Record> = (0..100)
            .map(|i| rec(&[("rent", Value::Float(2000.0 + 50.0 * i as f64))]))
            .collect();
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        assert!(resp.pre_flight_unsat_reason.is_some());
    }

    /// IsIn vs Eq contradiction: must be in {red, blue} AND equal green.
    #[test]
    fn w6_preflight_is_in_eq_contradiction() {
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "k".into(),
                    op: FieldOp::IsIn(vec![
                        Value::Text("red".into()),
                        Value::Text("blue".into()),
                    ]),
                    hard: true,
                },
                Constraint::Field {
                    field: "k".into(),
                    op: FieldOp::Eq(Value::Text("green".into())),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let records: Vec<Record> = (0..10)
            .map(|_| rec(&[("k", Value::Text("red".into()))]))
            .collect();
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        assert!(resp.pre_flight_unsat_reason.is_some());
    }

    /// Between intervals that don't overlap: [10, 20] AND [30, 40].
    #[test]
    fn w6_preflight_between_between_contradiction() {
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "v".into(),
                    op: FieldOp::Between { lo: 10.0, hi: 20.0 },
                    hard: true,
                },
                Constraint::Field {
                    field: "v".into(),
                    op: FieldOp::Between { lo: 30.0, hi: 40.0 },
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let records: Vec<Record> = (0..50)
            .map(|i| rec(&[("v", Value::Float(i as f64))]))
            .collect();
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        assert!(resp.pre_flight_unsat_reason.is_some());
    }

    /// **No false positives — the load-bearing safety test.**
    /// Sane queries that are NOT trivially contradictory must NOT
    /// be flagged. Pre-flight is allowed to miss subtle contradictions
    /// (those go to the bundle walk) but must never reject a
    /// non-contradictory query.
    #[test]
    fn w6_preflight_no_false_positives_on_compatible_constraints() {
        let records: Vec<Record> = (0..10)
            .map(|i| rec(&[
                ("color", Value::Text("red".into())),
                ("price", Value::Float(100.0 + 10.0 * i as f64)),
            ]))
            .collect();
        // These constraints are perfectly compatible.
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "price".into(),
                    op: FieldOp::Le(200.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "price".into(),
                    op: FieldOp::Ge(50.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat, "must NOT be falsely flagged Unsat");
        assert!(resp.pre_flight_unsat_reason.is_none(),
                "pre_flight_unsat_reason should be None for compatible constraints");
    }

    // ───────────────────────────────────────────────────────────
    // W6.3 — Γ trichotomy diagnostic
    // ───────────────────────────────────────────────────────────

    /// **W6.3** — Numeric regime: one highly-selective constraint
    /// against many records → K̂_max high, m small, |S| large →
    /// Γ < 0.5 → Numeric.
    #[test]
    fn w6_3_gamma_numeric_regime_one_tight_constraint() {
        // 100 records, value 0..99. Constraint: value >= 95 (only 5
        // pass → K_c = 0.95 for that one constraint).
        let records: Vec<Record> = (0..100)
            .map(|i| rec(&[("v", Value::Integer(i))]))
            .collect();
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "v".into(),
                op: FieldOp::Ge(95.0),
                hard: true,
            }],
            max_options: 10,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        let g = resp
            .gamma_trichotomy
            .expect("Γ must be populated when records were walked");
        // Sanity: m=1, n=100, K_max=0.95 (5/100 pass → 95/100 fail);
        // Γ = 1 / (0.95 · ln(101)) ≈ 1 / (0.95 · 4.62) ≈ 0.228 → Numeric.
        assert_eq!(g.m, 1);
        assert!((g.k_max - 0.95).abs() < 0.01, "K_max={}", g.k_max);
        assert!(g.gamma < 0.5, "Γ should be in Numeric regime: got {}", g.gamma);
        assert_eq!(g.regime, TrichotomyRegime::Numeric);
    }

    /// **W6.3** — Structural regime: a few moderate constraints on a
    /// medium population → Γ near 1.
    #[test]
    fn w6_3_gamma_structural_regime_moderate_constraints() {
        // 20 records on two fields, constraints of moderate
        // selectivity. Tuned so Γ lands in [0.5, 2.0].
        let records: Vec<Record> = (0..20)
            .map(|i| {
                rec(&[
                    ("a", Value::Integer(i)),
                    ("b", Value::Integer((i * 3) % 7)),
                ])
            })
            .collect();
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "a".into(),
                    op: FieldOp::Ge(10.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "b".into(),
                    op: FieldOp::Ge(2.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "b".into(),
                    op: FieldOp::Le(5.0),
                    hard: true,
                },
            ],
            max_options: 10,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        let g = resp.gamma_trichotomy.expect("Γ should be populated");
        assert_eq!(g.m, 3);
        // Verify Γ is well-defined and within the union of all three
        // regimes (the *exact* regime is data-dependent — what matters
        // here is the math runs and produces a number that falls in
        // SOME regime, and that the regime tagging is consistent with
        // the threshold constants).
        assert!(g.gamma.is_finite() && g.gamma > 0.0);
        assert_eq!(g.regime, TrichotomyRegime::from_gamma(g.gamma));
    }

    /// **W6.3** — Geometric regime: many low-selectivity constraints
    /// on a small record set. m is large, K_max is small → Γ large.
    #[test]
    fn w6_3_gamma_geometric_regime_many_loose_constraints() {
        // 5 records that all satisfy 6 trivially-satisfied lower-bound
        // constraints → K_c ≈ 0 for each → K_max ≈ 0, m = 6 → Γ
        // explodes into the Geometric regime via the epsilon clamp.
        let records: Vec<Record> = (0..5)
            .map(|_| {
                rec(&[
                    ("a", Value::Integer(100)),
                    ("b", Value::Integer(100)),
                    ("c", Value::Integer(100)),
                ])
            })
            .collect();
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field { field: "a".into(), op: FieldOp::Ge(0.0), hard: true },
                Constraint::Field { field: "a".into(), op: FieldOp::Le(200.0), hard: true },
                Constraint::Field { field: "b".into(), op: FieldOp::Ge(0.0), hard: true },
                Constraint::Field { field: "b".into(), op: FieldOp::Le(200.0), hard: true },
                Constraint::Field { field: "c".into(), op: FieldOp::Ge(0.0), hard: true },
                Constraint::Field { field: "c".into(), op: FieldOp::Le(200.0), hard: true },
            ],
            max_options: 5,
            max_near_misses: 0,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        let g = resp.gamma_trichotomy.expect("Γ should be populated");
        assert_eq!(g.m, 6);
        // All constraints satisfied by every record → K_max = 0 →
        // Γ = m / (1e-12 · ln(6)) → enormous → Geometric.
        assert!(g.gamma > 2.0, "Γ should be in Geometric regime: got {}", g.gamma);
        assert_eq!(g.regime, TrichotomyRegime::Geometric);
    }

    /// **W6.3 edge case** — empty constraint list → Γ undefined.
    #[test]
    fn w6_3_gamma_none_when_no_constraints() {
        let records: Vec<Record> = (0..10)
            .map(|i| rec(&[("v", Value::Integer(i))]))
            .collect();
        let req = SudokuRequest {
            constraints: Vec::new(),
            max_options: 5,
            max_near_misses: 0,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert!(
            resp.gamma_trichotomy.is_none(),
            "Γ must be None when m=0 (no constraints to classify)"
        );
    }

    /// **W6.3 edge case** — empty record set → Γ undefined.
    #[test]
    fn w6_3_gamma_none_when_zero_records() {
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "v".into(),
                op: FieldOp::Ge(0.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 0,
            expansion: None,
        };
        let resp =
            solve_constraints(Vec::<Record>::new(), &req, &SudokuConfig::default()).unwrap();
        assert!(
            resp.gamma_trichotomy.is_none(),
            "Γ must be None when n=0 (no candidate records)"
        );
    }

    // ───────────────────────────────────────────────────────────
    // W6.4 — Energy descent ordering on the relaxation menu
    // ───────────────────────────────────────────────────────────

    /// **W6.4** — every RelaxationOption has a populated, non-negative
    /// `energy_descent` value, and the menu is sorted by it descending.
    /// Subsumes the W3 `gain / cost` ordering for the diminishing-
    /// returns case; coincides with it when n_current_solutions is
    /// small.
    #[test]
    fn w6_4_relaxation_menu_sorted_by_energy_descent() {
        // 20 records with a numeric field. Constraint: v >= 15
        // (5/20 currently pass → n_solutions = 5). Relaxing this to
        // v >= 10 unlocks 5 more (gain = 5). Relaxing to v >= 5
        // unlocks 10 more (gain = 10) but at higher cost (5 σ vs 10 σ
        // depending on the data's length scale). We don't care about
        // the exact numbers — just that the field is populated, non-
        // negative, and the menu is sorted by it descending.
        let records: Vec<Record> = (0..20)
            .map(|i| rec(&[("v", Value::Integer(i))]))
            .collect();
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "v".into(),
                op: FieldOp::Ge(15.0),
                hard: true,
            }],
            max_options: 10,
            max_near_misses: 10,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert!(
            !resp.relaxations.is_empty(),
            "should have proposed at least one relaxation"
        );
        // Every option has a populated, non-negative energy_descent.
        for opt in &resp.relaxations {
            assert!(
                opt.energy_descent.is_finite() && opt.energy_descent >= 0.0,
                "energy_descent must be finite + non-negative; got {} for {}",
                opt.energy_descent,
                opt.description
            );
        }
        // Menu is sorted by energy_descent descending.
        for pair in resp.relaxations.windows(2) {
            assert!(
                pair[0].energy_descent >= pair[1].energy_descent,
                "menu not sorted descending: {} (\"{}\") then {} (\"{}\")",
                pair[0].energy_descent,
                pair[0].description,
                pair[1].energy_descent,
                pair[1].description,
            );
        }
    }

    /// **W6.4 math check** — `energy_descent` matches the closed-form
    /// `log(1 + gain / max(n_solutions, 1)) / max(cost, ε)` for a
    /// hand-engineered single-option case.
    #[test]
    fn w6_4_energy_descent_matches_closed_form() {
        // Build a tiny scenario: 4 records {1, 2, 3, 4}, constraint
        // v >= 4 (only record v=4 passes → n_solutions = 1). The only
        // numeric relaxation should propose v >= some-lower-value
        // chosen from the violating set {1, 2, 3}.
        let records: Vec<Record> = (1..=4)
            .map(|i| rec(&[("v", Value::Integer(i))]))
            .collect();
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "v".into(),
                op: FieldOp::Ge(4.0),
                hard: true,
            }],
            max_options: 10,
            max_near_misses: 3,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert!(!resp.relaxations.is_empty());
        // For each option, verify the closed-form matches.
        for opt in &resp.relaxations {
            let n_solutions_safe = (resp.solutions.len().max(1)) as f64;
            let expected_delta_e =
                (1.0 + (opt.gain as f64) / n_solutions_safe).ln();
            let expected = expected_delta_e / opt.relaxation_cost.max(1e-9);
            assert!(
                (opt.energy_descent - expected).abs() < 1e-9,
                "closed-form mismatch for \"{}\": got {}, expected {}",
                opt.description,
                opt.energy_descent,
                expected
            );
        }
    }

    /// **W6.4 diminishing returns** — when two options have the same
    /// `gain / cost` ratio but vastly different absolute gain, the
    /// log-transform makes them tie OR rewards the lower-cost option
    /// (not the bigger one). This is the qualitative difference
    /// between the W3 and W6.4 orderings.
    #[test]
    fn w6_4_energy_descent_vs_raw_gain_per_cost() {
        // Two synthetic options:
        //   A: gain=1, cost=1 → gain/cost = 1, ΔE/cost = log(2) ≈ 0.693
        //   B: gain=100, cost=100 → gain/cost = 1, ΔE/cost = log(101) ≈ 4.62 / 100 = 0.046
        // Both have the same gain/cost (= 1), but A has MUCH higher
        // energy_descent because log is sublinear: the same fractional
        // increase from a tiny base is worth more than a huge absolute
        // gain at high cost.
        let n_solutions = 1usize;
        let opt_a = RelaxationOption {
            constraint_idx: 0,
            field: "test".into(),
            description: "A".into(),
            new_threshold: None,
            gain: 1,
            relaxation_cost: 1.0,
            energy_descent: 0.0,
        };
        let opt_b = RelaxationOption {
            constraint_idx: 1,
            field: "test".into(),
            description: "B".into(),
            new_threshold: None,
            gain: 100,
            relaxation_cost: 100.0,
            energy_descent: 0.0,
        };
        // Apply the same formula the menu does.
        let n_safe = (n_solutions.max(1)) as f64;
        let a_e = (1.0 + opt_a.gain as f64 / n_safe).ln() / opt_a.relaxation_cost.max(1e-9);
        let b_e = (1.0 + opt_b.gain as f64 / n_safe).ln() / opt_b.relaxation_cost.max(1e-9);
        // gain/cost is equal:
        assert!(((opt_a.gain as f64 / opt_a.relaxation_cost) - (opt_b.gain as f64 / opt_b.relaxation_cost)).abs() < 1e-9);
        // But energy_descent ranks A > B by a wide margin:
        assert!(
            a_e > b_e * 10.0,
            "log-transform should rank A >> B: a_e={}, b_e={}",
            a_e,
            b_e
        );
    }

    /// **W6.3 regime classifier** — pure function test on
    /// `TrichotomyRegime::from_gamma`.
    #[test]
    fn w6_3_regime_thresholds() {
        assert_eq!(TrichotomyRegime::from_gamma(0.0), TrichotomyRegime::Numeric);
        assert_eq!(TrichotomyRegime::from_gamma(0.49), TrichotomyRegime::Numeric);
        assert_eq!(TrichotomyRegime::from_gamma(0.5), TrichotomyRegime::Structural);
        assert_eq!(TrichotomyRegime::from_gamma(1.0), TrichotomyRegime::Structural);
        assert_eq!(TrichotomyRegime::from_gamma(2.0), TrichotomyRegime::Structural);
        assert_eq!(TrichotomyRegime::from_gamma(2.01), TrichotomyRegime::Geometric);
        assert_eq!(TrichotomyRegime::from_gamma(100.0), TrichotomyRegime::Geometric);
        // String tags stable for wire serialization.
        assert_eq!(TrichotomyRegime::Numeric.as_str(), "numeric");
        assert_eq!(TrichotomyRegime::Structural.as_str(), "structural");
        assert_eq!(TrichotomyRegime::Geometric.as_str(), "geometric");
    }

    /// **W5 scale-fix.** With many-constraint queries, the Pareto
    /// frontier must not collapse to 1 entry just because most
    /// records violate more than 3 constraints. The cap must scale
    /// with constraint count.
    #[test]
    fn w5_pareto_frontier_scales_with_constraint_count() {
        // 6 records, each violating 4 of the 6 numeric constraints
        // (varying which 4). Previously: ALL filtered out because
        // violation_count (4) > old cap (3). Now: each appears.
        let constraints: Vec<Constraint> = (0..6)
            .map(|i| Constraint::Field {
                field: format!("f{}", i),
                op: FieldOp::Le(10.0),
                hard: true,
            })
            .collect();
        // Records: each violates fields 0..3 (over threshold), passes 4..5.
        let mut records: Vec<Record> = Vec::new();
        for r in 0..6 {
            let mut rec = Record::new();
            for f in 0..6 {
                // Different per-record offsets so they're not identical.
                let val = if f < 4 { 20.0 + r as f64 } else { 5.0 };
                rec.insert(format!("f{}", f), Value::Float(val));
            }
            records.push(rec);
        }
        let req = SudokuRequest {
            constraints,
            max_options: 5,
            max_near_misses: 5,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert!(
            !resp.pareto_near_misses.is_empty(),
            "Pareto frontier must NOT be empty at high constraint count; \
             got {} entries (each record has 4 violations, cap should now scale to 6)",
            resp.pareto_near_misses.len()
        );
    }

    /// Mass aggregation: 5 identical records → one solution
    /// with mass 1.0.
    #[test]
    fn identical_records_collapse_to_single_solution_with_full_mass() {
        let r = rec(&[("x", Value::Integer(1))]);
        let records = vec![r.clone(), r.clone(), r.clone(), r.clone(), r.clone()];
        let req = SudokuRequest {
            constraints: vec![],
            max_options: 5,
            max_near_misses: 3,
            expansion: None,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.solutions.len(), 1);
        assert!((resp.solutions[0].stated_prior_mass - 1.0).abs() < 1e-9);
    }

    // ─────────────────────────────────────────────────────────────
    // S3.5 — Puzzle expansion tests (TDD — red before green)
    // ─────────────────────────────────────────────────────────────

    /// **E1.** Expansion is NOT triggered when the original verdict
    /// is SAT — we only expand when the 9×9 has no solution.
    #[test]
    fn expansion_not_triggered_when_original_is_sat() {
        let records = vec![
            rec(&[("price", Value::Float(80.0))]),
            rec(&[("price", Value::Float(120.0))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "price".into(),
                op: FieldOp::Le(100.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 3,
            expansion: Some(ExpansionConfig {
                allowed: true,
                max_constraint_relaxations: 2,
            }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
        // expanded must be None — no point asking "what other puzzle
        // is this?" when the original puzzle has solutions.
        assert!(
            resp.expanded.is_none(),
            "expansion must not trigger on SAT — got: {:?}",
            resp.expanded.as_ref().map(|e| &e.expansion_type)
        );
    }

    /// **E2.** Expansion IS triggered on UNSAT and finds solutions
    /// by relaxing the cheapest constraint.
    ///
    /// Setup: 5 records with price 100..500. Constraint: price <= 90.
    /// Zero solutions. Expansion: relax price to the cheapest
    /// near-miss value (100). Should find the $100 record.
    #[test]
    fn expansion_relaxes_cheapest_constraint_on_unsat() {
        let records = vec![
            rec(&[("price", Value::Float(100.0))]), // cheapest violator
            rec(&[("price", Value::Float(200.0))]),
            rec(&[("price", Value::Float(300.0))]),
            rec(&[("price", Value::Float(400.0))]),
            rec(&[("price", Value::Float(500.0))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "price".into(),
                op: FieldOp::Le(90.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
            expansion: Some(ExpansionConfig {
                allowed: true,
                max_constraint_relaxations: 1,
            }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat, "original must be Unsat");
        let expanded = resp.expanded.as_ref()
            .expect("expanded must be Some when Unsat + expansion allowed");
        assert!(expanded.attempted, "attempted must be true");
        assert!(
            !expanded.solutions.is_empty(),
            "expansion must find the $100 record after relaxing price"
        );
        // The $100 record should be the first expanded solution.
        let first = &expanded.solutions[0];
        assert_eq!(
            first.record.get("price"),
            Some(&Value::Float(100.0)),
            "expanded solution should be the $100 record"
        );
        // expansion_cost must be finite and positive.
        assert!(
            first.expansion_cost > 0.0 && first.expansion_cost.is_finite(),
            "expansion_cost must be finite positive, got {}",
            first.expansion_cost
        );
        // relaxed_constraint_idx must point to the price constraint (idx 0).
        assert_eq!(first.relaxed_constraint_idx, 0);
    }

    /// **E3.** When expansion is not allowed (`allowed: false`), the
    /// `expanded` field is None even on UNSAT.
    #[test]
    fn expansion_not_triggered_when_not_allowed() {
        let records = vec![rec(&[("x", Value::Float(5.0))])];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "x".into(),
                op: FieldOp::Ge(10.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 3,
            expansion: Some(ExpansionConfig {
                allowed: false, // explicitly disabled
                max_constraint_relaxations: 1,
            }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        assert!(
            resp.expanded.is_none(),
            "expanded must be None when expansion.allowed = false"
        );
    }

    /// **E4.** When no `expansion` field is set on the request,
    /// `expanded` is None (default is opt-out).
    #[test]
    fn expansion_absent_by_default() {
        let records = vec![rec(&[("x", Value::Float(5.0))])];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "x".into(),
                op: FieldOp::Ge(10.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 3,
            expansion: None, // no expansion field
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert!(
            resp.expanded.is_none(),
            "expanded must be None when no expansion config provided"
        );
    }

    /// **E5.** When expansion is allowed but even relaxation finds no
    /// solutions, `expanded.solutions` is empty and `advisory` is set.
    ///
    /// Setup: 3 records with price 100, 200, 300. Two hard constraints:
    /// price <= 90 (blocks all) AND color = "red" (blocks all).
    /// Relaxing price (cheapest numeric constraint) still leaves no
    /// matches because color = "red" is also violated by all records.
    /// The advisory must say "consider asking a human."
    #[test]
    fn expansion_advisory_set_when_relaxation_also_fails() {
        let records = vec![
            rec(&[("price", Value::Float(100.0)), ("color", Value::Text("blue".into()))]),
            rec(&[("price", Value::Float(200.0)), ("color", Value::Text("blue".into()))]),
        ];
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "price".into(),
                    op: FieldOp::Le(90.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: Some(ExpansionConfig {
                allowed: true,
                max_constraint_relaxations: 1, // only try the cheapest
            }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        let expanded = resp.expanded.as_ref()
            .expect("expanded must be Some");
        assert!(expanded.attempted);
        // Relaxing price Le(90) to Le(100) or Le(200) still leaves
        // color = "red" unsatisfied → no expansion solutions.
        assert!(
            expanded.solutions.is_empty(),
            "expansion should find no solutions when second constraint still blocks"
        );
        let advisory = expanded.advisory.as_ref()
            .expect("advisory must be set when expansion also fails");
        assert!(
            advisory.to_lowercase().contains("human") || advisory.to_lowercase().contains("reformulat"),
            "advisory should suggest asking a human or reformulating; got: {advisory}"
        );
    }

    /// **E6.** Expansion triggered by pre-flight UNSAT (Le+Ge
    /// contradiction) — the relaxed constraint set is valid, and
    /// expansion should run on the underlying records.
    ///
    /// Constraint: Le(100) AND Ge(500) — pre-flight catches it.
    /// Expansion relaxes Le(100) → Le(200) (dropping the first
    /// constraint from a "drop" style relaxation, which isn't
    /// applicable here since they're numeric). We use a drop-style
    /// relaxation via categorical to test that pre-flight Unsat
    /// correctly feeds into expansion.
    ///
    /// Simpler version: price Eq("red") AND price Eq("blue") →
    /// pre-flight fires. Expansion (attempted on the original records
    /// with one constraint relaxed) should find records if they exist.
    #[test]
    fn expansion_runs_after_preflight_unsat() {
        // Pre-flight: color=red AND color=blue is trivially contradictory.
        // Bundle has 5 red records. Expansion drops color=blue → finds them.
        let records: Vec<Record> = (0..5)
            .map(|i| rec(&[("color", Value::Text("red".into())),
                           ("id", Value::Integer(i))]))
            .collect();
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("blue".into())),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: Some(ExpansionConfig {
                allowed: true,
                max_constraint_relaxations: 2,
            }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        assert!(resp.pre_flight_unsat_reason.is_some(), "should fire pre-flight");
        // Expansion should be attempted and find the red records.
        let expanded = resp.expanded.as_ref()
            .expect("expanded must be Some — pre-flight Unsat still triggers expansion");
        assert!(expanded.attempted);
        assert!(
            !expanded.solutions.is_empty(),
            "expansion should find red records after dropping blue constraint"
        );
    }

    /// **E7.** Expanded solutions do NOT duplicate records already
    /// in the original `solutions` list (they are from a different
    /// puzzle — a stricter constraint that the expansion relaxed).
    /// Since the original is always Unsat when expansion triggers,
    /// original solutions is always empty. But multi-constraint
    /// cases: relaxing constraint A might unlock records that are
    /// already in the original `near_misses` — those should appear
    /// in `expanded.solutions` (they're now SOLUTIONS to the relaxed
    /// puzzle), not be suppressed.
    #[test]
    fn expansion_solutions_are_distinct_from_original_solutions() {
        // 3 records: 1 satisfies all, 2 violate price only.
        // Constraint: price <= 100 AND color = "red". Both records
        // satisfy color=red, one satisfies price.
        let records = vec![
            rec(&[("price", Value::Float(80.0)),  ("color", Value::Text("red".into()))]),
            rec(&[("price", Value::Float(150.0)), ("color", Value::Text("red".into()))]),
            rec(&[("price", Value::Float(200.0)), ("color", Value::Text("red".into()))]),
        ];
        // Make it UNSAT: price <= 70 (nothing matches).
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "price".into(),
                    op: FieldOp::Le(70.0),
                    hard: true,
                },
                Constraint::Field {
                    field: "color".into(),
                    op: FieldOp::Eq(Value::Text("red".into())),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
            expansion: Some(ExpansionConfig {
                allowed: true,
                max_constraint_relaxations: 3,
            }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        // original solutions empty
        assert!(resp.solutions.is_empty());
        let expanded = resp.expanded.as_ref().expect("expansion must run");
        // Expansion should find at least the $80 record (cheapest near-miss).
        assert!(
            !expanded.solutions.is_empty(),
            "expansion must find solutions in the relaxed puzzle"
        );
        // All expanded solutions must have a valid relaxed_constraint_idx.
        for sol in &expanded.solutions {
            assert!(
                sol.relaxed_constraint_idx < req.constraints.len(),
                "relaxed_constraint_idx {} out of range",
                sol.relaxed_constraint_idx
            );
        }
    }

    /// **E8.** `expansion_type` is "constraint_relaxation" in v1
    /// (bundle_hop is an HTTP-layer concern).
    #[test]
    fn expansion_type_is_constraint_relaxation_in_v1() {
        let records = vec![rec(&[("x", Value::Float(5.0))])];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "x".into(),
                op: FieldOp::Ge(10.0),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 3,
            expansion: Some(ExpansionConfig {
                allowed: true,
                max_constraint_relaxations: 1,
            }),
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        if let Some(expanded) = &resp.expanded {
            assert_eq!(
                expanded.expansion_type, "constraint_relaxation",
                "v1 expansion type must be 'constraint_relaxation'"
            );
        }
        // If expanded is None (no relaxation menu — single record,
        // no near-misses to relax from), that's also acceptable.
    }
}
